use futures_locks::RwLock;
use futures_locks::RwLockReadGuard;
use futures_locks::RwLockWriteGuard;
use rkyv::{Archive, Deserialize, Serialize};
use std::sync::Arc;

#[derive(Archive, Debug, Deserialize, Serialize)]
#[archive(bound(
    serialize = "__S: rkyv::ser::ScratchSpace + rkyv::ser::SharedSerializeRegistry + rkyv::ser::Serializer",
    deserialize = "__D: rkyv::de::SharedDeserializeRegistry"
))]
pub struct State {
    pub active: i32,
    pub current_tasks: Vec<i32>,
    pub current_flows: Vec<i32>,
    pub visited_tasks: Vec<i32>,
    pub visited_flows: Vec<i32>,
    pub pending_tasks: Vec<i32>,
    pub maybe_future_tasks: Vec<i32>,
    pub maybe_future_flows: Vec<i32>,
    pub maybe_visited_tasks: Vec<i32>,
    pub variables: wfrs_model::json::JsonValue,
    pub completed: bool,
    pub remote_id: Option<String>,
    pub remote_version: Option<i64>,
}

pub struct LockedState {
    pub inner: State,
}

#[derive(Clone)]
pub struct WorkflowState {
    inner: Arc<RwLock<LockedState>>,
}

impl WorkflowState {
    pub fn new(start_event: i32) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LockedState {
                inner: State {
                    active: -1,
                    current_tasks: vec![start_event],
                    current_flows: vec![],
                    visited_tasks: vec![],
                    visited_flows: vec![],
                    pending_tasks: vec![],
                    maybe_future_tasks: vec![],
                    maybe_future_flows: vec![],
                    maybe_visited_tasks: vec![],
                    variables: wfrs_model::json::JsonValue::map(),
                    completed: false,
                    remote_id: None,
                    remote_version: None,
                },
            })),
        }
    }

    pub fn from_state(state: State) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LockedState { inner: state })),
        }
    }

    pub async fn replace(&self, state: State) {
        self.inner.write().await.inner = state;
    }

    pub async fn set_remote_id(&self, remote_id: String, remote_version: i64) {
        let mut state = self.inner.write().await;
        state.inner.remote_id = Some(remote_id);
        state.inner.remote_version = Some(remote_version);
    }

    pub async fn get_remote_id(&self) -> Option<String> {
        self.inner.read().await.inner.remote_id.clone()
    }

    pub async fn has_visited(&self, user_task: i32) -> bool {
        let state = self.inner.read().await;
        state.inner.visited_tasks.contains(&user_task)
    }

    pub async fn has_maybe_visited(&self, user_task: i32) -> bool {
        let state = self.inner.read().await;
        state.inner.maybe_visited_tasks.contains(&user_task)
    }

    pub async fn set_usertask(&self, user_task: i32) {
        let mut state = self.inner.write().await;
        state.inner.current_tasks.clear();
        state.inner.current_flows.clear();
        state.inner.pending_tasks.clear();
        state.inner.pending_tasks.push(user_task);
        state.inner.active = user_task;
    }

    pub async fn clear_future(&self, start_event: i32) {
        let mut state = self.inner.write().await;
        state.inner.maybe_future_tasks.clear();
        state.inner.maybe_future_flows.clear();
        state.inner.maybe_visited_tasks.clear();
        state.inner.maybe_future_tasks.push(start_event);
    }

    pub async fn set_active(&self, active: i32) {
        self.inner.write().await.inner.active = active;
    }

    pub async fn get_active(&self) -> i32 {
        self.inner.read().await.inner.active
    }

    pub async fn set_completed(&self) {
        self.inner.write().await.inner.completed = true;
    }

    pub async fn pop_current_task(&self) -> Option<i32> {
        self.inner.write().await.inner.current_tasks.pop()
    }

    pub async fn push_current_task(&self, task: i32) {
        self.inner.write().await.inner.current_tasks.push(task);
    }

    pub async fn pop_visited_task(&self) -> Option<i32> {
        self.inner.write().await.inner.visited_tasks.pop()
    }

    pub async fn push_visited_task(&self, task: i32) {
        let mut state = self.inner.write().await;
        let exist = state.inner.visited_tasks.contains(&task);
        if !exist {
            state.inner.visited_tasks.push(task);
        }
    }

    pub async fn pop_current_flow(&self) -> Option<i32> {
        self.inner.write().await.inner.current_flows.pop()
    }

    pub async fn push_current_flow(&self, flow: i32) {
        self.inner.write().await.inner.current_flows.push(flow);
    }

    pub async fn pop_visited_flow(&self) -> Option<i32> {
        self.inner.write().await.inner.visited_flows.pop()
    }

    pub async fn push_visited_flow(&self, flow: i32) {
        self.inner.write().await.inner.visited_flows.push(flow);
    }

    pub async fn pop_pending_task(&self) -> Option<i32> {
        self.inner.write().await.inner.pending_tasks.pop()
    }

    pub async fn push_pending_task(&self, task: i32) {
        self.inner.write().await.inner.pending_tasks.push(task);
    }

    pub async fn pop_maybe_future_task(&self) -> Option<i32> {
        self.inner.write().await.inner.maybe_future_tasks.pop()
    }

    pub async fn push_maybe_future_task(&self, task: i32) {
        self.inner.write().await.inner.maybe_future_tasks.push(task);
    }

    pub async fn push_maybe_visited_task(&self, task: i32) {
        self.inner
            .write()
            .await
            .inner
            .maybe_visited_tasks
            .push(task);
    }

    pub async fn pop_maybe_future_flow(&self) -> Option<i32> {
        self.inner.write().await.inner.maybe_future_flows.pop()
    }

    pub async fn push_maybe_future_flow(&self, flow: i32) {
        self.inner.write().await.inner.maybe_future_flows.push(flow);
    }

    pub async fn pending_task_by_index(&self, idx: usize) -> i32 {
        self.inner.write().await.inner.pending_tasks.remove(idx)
    }

    pub async fn state(&self) -> RwLockReadGuard<LockedState> {
        self.inner.read().await
    }
    pub async fn mut_state(&self) -> RwLockWriteGuard<LockedState> {
        self.inner.write().await
    }
}
