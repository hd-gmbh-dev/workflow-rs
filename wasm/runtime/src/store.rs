use futures_locks::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

// use crate::client::proto::WorkflowInfo;
use crate::definition::JsWorkflowDefinition;
// use crate::instance::JsWorkflowInstance;

#[derive(Default)]
struct Store {
    // active: RwLock<Vec<ActiveWorkflow>>,
    definitions: RwLock<HashMap<String, JsWorkflowDefinition>>,
}

// #[wasm_bindgen(js_name = ActiveWorkflow)]
// pub struct ActiveWorkflow {
//     // info: Arc<WorkflowInfo>,
//     loaded: Arc<RwLock<Option<JsWorkflowInstance>>>,
// }

#[wasm_bindgen(js_name = WorkflowStore)]
pub struct WorkflowStore {
    inner: Arc<Store>,
}

#[wasm_bindgen(js_class = WorkflowStore)]
impl WorkflowStore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Default::default()),
        }
    }

    pub async fn register(&self, data: &[u8]) -> Result<JsWorkflowDefinition, String> {
        let definiton = JsWorkflowDefinition::new(data)?;
        self.inner
            .definitions
            .write()
            .await
            .insert(definiton.id(), definiton.clone());
        Ok(definiton)
    }
}
