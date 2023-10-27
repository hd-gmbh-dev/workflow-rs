use crate::db::deserialize_entry;

use wasm_bindgen::prelude::*;
use wfrs_model::deserialize;
use wfrs_model::WorkflowDefinition;
use wfrs_engine::Runtime;

use crate::db::{load, store};
use crate::db::{DbEntry, IndexedDb};
use crate::instance::JsWorkflowInstance;

#[derive(Clone)]
#[wasm_bindgen]
pub struct JsWorkflowDefinition(&'static WorkflowDefinition);

#[wasm_bindgen]
pub fn create(data: &[u8]) -> Result<JsWorkflowDefinition, String> {
    let definition = deserialize(data).map_err(|e| format!("{e:#?}"))?;
    let js_workflow = Box::new(definition);
    Ok(JsWorkflowDefinition(Box::leak(js_workflow)))
}

#[wasm_bindgen]
impl JsWorkflowDefinition {
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<JsWorkflowDefinition, String> {
        create(data)
    }

    pub fn id(&self) -> String {
        format!("{}:{}", self.0.id, self.0.version)
    }

    pub fn version(&self) -> String {
        self.0.version.to_string()
    }

    pub fn has_autostart(&self) -> bool {
        self.0
            .options
            .as_ref()
            .map(|options| options.autostart)
            .unwrap_or(false)
    }

    pub async fn exist(&self, instance_id: String) -> Result<bool, String> {
        let db = IndexedDb::new().await.map_err(|err| format!("{err:#?}"))?;
        Ok(
            crate::db::read_entry(db.as_ref(), &self.0.format_id(&instance_id))
                .await
                .map_err(|err| format!("{err:#?}"))?
                .is_some(),
        )
    }

    pub async fn restore(
        &self,
        entity_id: String,
        remote_id: String,
        remote_version: i64,
        state: &[u8],
    ) -> Result<JsWorkflowInstance, String> {
        let state = deserialize_entry(state)
            .await
            .map_err(|err| format!("{err:#?}"))?;
        let runtime = Runtime::new(self.0, self.0.format_id(&entity_id)).with_state(state);
        runtime
            .instance
            .set_remote_id(remote_id, remote_version)
            .await;
        let js_runtime = Box::new(runtime);
        store(DbEntry::new(
            js_runtime.entity_id.clone(),
            js_runtime.instance.clone(),
        ))
        .await?;
        let result = JsWorkflowInstance::new(Box::leak(js_runtime));
        // result.print().await;
        Ok(result)
    }

    pub async fn start(&self, entity_id: String) -> Result<JsWorkflowInstance, String> {
        let js_runtime = Box::new(Runtime::new(self.0, self.0.format_id(&entity_id)));
        js_runtime.run().await;
        js_runtime.simulate().await;
        js_runtime.set_default_active_task().await;
        store(DbEntry::new(
            js_runtime.entity_id.clone(),
            js_runtime.instance.clone(),
        ))
        .await?;
        let result = JsWorkflowInstance::new(Box::leak(js_runtime));
        // result.print().await;
        Ok(result)
    }

    pub async fn load(&self, entity_id: String) -> Result<JsWorkflowInstance, String> {
        let mut js_runtime = Box::new(Runtime::new(self.0, self.0.format_id(&entity_id)));
        let entry = load(&js_runtime.entity_id).await?;
        if let Some(entry) = entry {
            js_runtime.instance = entry.state;
        }
        let result = JsWorkflowInstance::new(Box::leak(js_runtime));
        // result.print().await;
        Ok(result)
    }

    pub fn user_tasks(&self) -> Vec<i32> {
        self.0.user_tasks()
    }

    pub fn task_ids(&self) -> String {
        self.0.task_ids.join(",")
    }

    pub fn print(&self) {
        log::info!("{:#?}", self.0);
    }
}
