use crate::db::deserialize_entry;
use crate::db::remove;
use crate::db::store;
use crate::db::DbEntry;
use crate::variables::JsRuntimeVariables;
use js_sys::Object;
use log::info;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::ser::Serializer;
use wasm_bindgen::prelude::*;
use wfrs_model::json::JsonValue;
use wfrs_engine::Runtime;

#[wasm_bindgen]
pub struct JsWorkflowInstanceVersion {
    id: String,
    pub ts: i64,
}

#[wasm_bindgen]
impl JsWorkflowInstanceVersion {
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

#[wasm_bindgen]
pub struct JsWorkflowInstance {
    rt: &'static mut Runtime<'static>,
}

impl JsWorkflowInstance {
    pub fn new(runtime: &'static mut Runtime<'static>) -> Self {
        Self { rt: runtime }
    }
}

#[wasm_bindgen]
impl JsWorkflowInstance {
    pub async fn print(&self) {
        let state = self.rt.instance.state().await;
        info!("current_tasks: {:#?}", state.inner.current_tasks);
        info!("current_flows: {:#?}", state.inner.current_flows);
        info!("visited_tasks: {:#?}", state.inner.visited_tasks);
        info!("visited_flows: {:#?}", state.inner.visited_flows);
        info!("pending_tasks: {:#?}", state.inner.pending_tasks);
        info!("maybe_future_tasks: {:#?}", state.inner.maybe_future_tasks);
        info!("maybe_future_flows: {:#?}", state.inner.maybe_future_flows);
        info!(
            "maybe_visited_tasks: {:#?}",
            state.inner.maybe_visited_tasks
        );
        info!("variables: {:#?}", state.inner.variables);
    }

    pub async fn state(&self) -> js_sys::Uint8Array {
        let s = self.rt.instance.state().await;
        let mut serializer = AllocSerializer::<0>::default();
        serializer.serialize_value(&s.inner).unwrap();
        let result = serializer.into_serializer().into_inner().to_vec();
        let buf = js_sys::Uint8Array::new_with_length(result.len() as u32);
        buf.copy_from(&result);
        buf
    }

    pub async fn set_state(&self, state: Vec<u8>) -> Result<(), String> {
        let state = deserialize_entry(&state)
            .await
            .map_err(|err| format!("{err:#?}"))?;
        self.rt.replace(state).await;
        Ok(())
    }

    pub async fn maybe_visited_tasks(&self) -> js_sys::Int32Array {
        let state = self.rt.instance.state().await;
        unsafe { js_sys::Int32Array::view(&state.inner.maybe_visited_tasks) }
    }

    pub async fn visited_tasks(&self) -> js_sys::Int32Array {
        let state = self.rt.instance.state().await;
        unsafe { js_sys::Int32Array::view(&state.inner.visited_tasks) }
    }

    pub async fn pending_tasks(&self) -> js_sys::Int32Array {
        let state = self.rt.instance.state().await;
        unsafe { js_sys::Int32Array::view(&state.inner.pending_tasks) }
    }

    pub async fn get_active(&self) -> i32 {
        self.rt.instance.state().await.inner.active
    }

    pub async fn get_remote_id(&self) -> Option<JsWorkflowInstanceVersion> {
        let state = self.rt.instance.state().await;
        state
            .inner
            .remote_id
            .clone()
            .zip(state.inner.remote_version)
            .map(|(id, ts)| JsWorkflowInstanceVersion { id, ts })
    }

    pub async fn set_remote_id(
        &self,
        remote_id: String,
        remote_version: i64,
    ) -> Result<(), String> {
        self.rt
            .instance
            .set_remote_id(remote_id, remote_version)
            .await;
        store(DbEntry::new(
            self.rt.entity_id.clone(),
            self.rt.instance.clone(),
        ))
        .await?;
        Ok(())
    }

    pub async fn complete(&self, task_id: i32) -> Result<(), String> {
        self.rt.complete(task_id).await?;
        self.rt.simulate().await;
        let pending_tasks = self.pending_tasks().await;
        if let Some(active) = pending_tasks.at(0) {
            self.rt.instance.set_active(active).await;
        }
        store(DbEntry::new(
            self.rt.entity_id.clone(),
            self.rt.instance.clone(),
        ))
        .await?;
        Ok(())
    }

    pub async fn navigate_to(&self, task_id: i32) -> Result<(), String> {
        self.rt.navigate_to(task_id).await;
        self.rt.run().await;
        self.rt.simulate().await;
        store(DbEntry::new(
            self.rt.entity_id.clone(),
            self.rt.instance.clone(),
        ))
        .await?;
        Ok(())
    }

    pub async fn get_variables(&self, task_id: i32) -> JsValue {
        let state = self.rt.instance.state().await;
        let is_current_task = state.inner.pending_tasks.contains(&task_id);
        if is_current_task {
            let key = self.rt.definition.task_ids[task_id as usize].as_ref();
            if let Some(variables) = state
                .inner
                .variables
                .as_object()
                .and_then(|obj| obj.get(key))
            {
                return JsRuntimeVariables(variables).into();
            }
        }
        JsValue::null()
    }

    pub async fn set_variables(&self, task_id: i32, variables: Object) -> Result<(), JsValue> {
        async {
            let mut state = self.rt.instance.mut_state().await;
            let is_current_task = state.inner.pending_tasks.contains(&task_id);
            if is_current_task {
                let key = self.rt.definition.task_ids[task_id as usize].as_ref();
                if let Some(obj) = state.inner.variables.as_object_mut() {
                    let current_variables = if !obj.contains_key(key) {
                        obj.insert(key.to_string(), JsonValue::map());
                        obj.get_mut(key).unwrap().as_object_mut()
                    } else {
                        obj.get_mut(key).unwrap().as_object_mut()
                    }
                    .unwrap();
                    for js_key in js_sys::Reflect::own_keys(&variables)?.iter() {
                        if js_key.is_string() {
                            let key = js_key.as_string().unwrap();
                            let value = js_sys::Reflect::get(&variables, &js_key)?;
                            if value.has_type::<js_sys::JsString>() {
                                current_variables
                                    .insert(key, JsonValue::String(value.as_string().unwrap()));
                            } else if value.has_type::<js_sys::Number>() {
                                let f = value.as_f64().unwrap();
                                if f.fract() == 0.0 {
                                    if f.is_sign_positive() {
                                        current_variables.insert(
                                            key,
                                            JsonValue::Number(
                                                wfrs_model::json::JsonNumber::PosInt(unsafe {
                                                    f.to_int_unchecked::<u64>()
                                                }),
                                            ),
                                        );
                                    } else {
                                        current_variables.insert(
                                            key,
                                            JsonValue::Number(
                                                wfrs_model::json::JsonNumber::NegInt(unsafe {
                                                    f.to_int_unchecked::<i64>()
                                                }),
                                            ),
                                        );
                                    }
                                } else {
                                    current_variables.insert(
                                        key,
                                        JsonValue::Number(wfrs_model::json::JsonNumber::Float(f)),
                                    );
                                }
                            } else if value.has_type::<js_sys::Boolean>() {
                                current_variables
                                    .insert(key, JsonValue::Bool(value.as_bool().unwrap()));
                            }
                        }
                    }
                }
            }
            Ok::<(), JsValue>(())
        }
        .await?;
        self.rt.simulate().await;
        store(DbEntry::new(
            self.rt.entity_id.clone(),
            self.rt.instance.clone(),
        ))
        .await?;
        Ok(())
    }

    pub async fn is_completed(&self) -> bool {
        self.rt.instance.state().await.inner.completed
    }

    pub async fn back(&self) -> i32 {
        self.rt
            .get_previous_user_task(self.get_active().await)
            .await
            .unwrap_or(-1)
    }

    pub async fn destroy(self) -> Result<(), String> {
        remove(&self.rt.entity_id).await?;
        let Self { rt } = self;
        unsafe {
            let ptr = rt as *mut Runtime<'static>;
            let _ = Box::from_raw(ptr);
        }
        Ok(())
    }
}
