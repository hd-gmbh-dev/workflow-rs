use crate::state::WorkflowState;
use async_recursion::async_recursion;
use state::State;
use wfrs_model::{Flow, Task, WorkflowDefinition};
use wfrs_validator::ExclusiveGateway;
pub mod state;

pub struct Runtime<'a> {
    pub entity_id: String,
    pub definition: &'a WorkflowDefinition,
    pub instance: WorkflowState,
}

impl<'a> Runtime<'a> {
    pub fn new(definition: &'a WorkflowDefinition, entity_id: String) -> Self {
        let instance = WorkflowState::new(definition.root_start_event());
        Self {
            entity_id,
            definition,
            instance,
        }
    }

    pub fn with_state(mut self, state: State) -> Self {
        self.instance = WorkflowState::from_state(state);
        self
    }

    pub async fn replace(&self, state: State) {
        self.instance.replace(state).await;
    }

    pub async fn complete(&self, task_id: i32) -> Result<(), String> {
        let pending_task_idx = async {
            let state = self.instance.state().await;
            state
                .inner
                .pending_tasks
                .iter()
                .position(|pidx| pidx == &task_id)
        }
        .await;
        if let Some(pending_task_idx) = pending_task_idx {
            if let Some(usertask) = self.fetch_pending_task(pending_task_idx).await {
                match &usertask.def {
                    wfrs_model::TaskDef::UserTask(ev) => {
                        self.visit_outgoing(&ev.outgoing).await;
                        self.run().await;
                        Ok(())
                    }
                    _ => Err(format!("task with id {pending_task_idx} is not a usertask")),
                }
            } else {
                Err(format!("no pending usertask with id {task_id}"))
            }
        } else {
            Err(format!("task with id {task_id} not found"))
        }
    }

    async fn fetch_pending_task(&self, idx: usize) -> Option<&Task> {
        self.definition
            .tasks
            .get(self.instance.pending_task_by_index(idx).await as usize)
    }

    async fn fetch_current_task(&self) -> Option<&Task> {
        if let Some(current_task) = self.instance.pop_current_task().await {
            self.definition.tasks.get(current_task as usize)
        } else {
            None
        }
    }

    async fn fetch_current_flow(&self) -> Option<&Flow> {
        if let Some(current_flow) = self.instance.pop_current_flow().await {
            self.definition.flows.get(current_flow as usize)
        } else {
            None
        }
    }

    async fn fetch_future_task(&self) -> Option<&Task> {
        if let Some(future_task) = self.instance.pop_maybe_future_task().await {
            self.definition.tasks.get(future_task as usize)
        } else {
            None
        }
    }

    async fn fetch_future_flow(&self) -> Option<&Flow> {
        if let Some(future_flow) = self.instance.pop_maybe_future_flow().await {
            self.definition.flows.get(future_flow as usize)
        } else {
            None
        }
    }

    async fn visit_outgoing(&self, outgoing: &[i32]) {
        for outgoing in outgoing {
            self.instance.push_current_flow(*outgoing).await;
        }
    }

    async fn visit_future_outgoing(&self, outgoing: &[i32]) {
        for outgoing in outgoing {
            self.instance.push_maybe_future_flow(*outgoing).await;
        }
    }

    #[async_recursion]
    pub async fn run(&self) {
        if let Some(current_task) = self.fetch_current_task().await {
            match &current_task.def {
                wfrs_model::TaskDef::StartEvent(ev) => {
                    self.visit_outgoing(&ev.outgoing).await;
                    self.run().await;
                }
                wfrs_model::TaskDef::UserTask(_) => {
                    self.instance.push_pending_task(current_task.id).await;
                    self.instance.push_visited_task(current_task.id).await;
                }
                wfrs_model::TaskDef::ExclusiveGateway(ev) => {
                    let out = async {
                        let state = self.instance.state().await;
                        ExclusiveGateway(ev).evaluate(self.definition, &state.inner.variables)
                    }
                    .await;
                    self.visit_outgoing(&out).await;
                    self.run().await;
                }
                wfrs_model::TaskDef::EndEvent(_) => {
                    self.instance.set_completed().await;
                }
            }
        }

        if let Some(current_flow) = self.fetch_current_flow().await {
            self.instance.push_visited_flow(current_flow.id).await;
            self.instance
                .push_current_task(current_flow.target_ref)
                .await;
            self.instance
                .push_visited_task(current_flow.source_ref)
                .await;
            self.run().await;
        }
    }

    pub async fn set_default_active_task(&self) {
        let mut state = self.instance.mut_state().await;
        if let Some(idx) = state.inner.pending_tasks.first() {
            state.inner.active = *idx;
        } else {
            state.inner.active = -1;
        }
    }

    fn is_usertask(&self, task_id: i32) -> bool {
        self.definition
            .tasks
            .get(task_id as usize)
            .map(|ev| match ev.def {
                wfrs_model::TaskDef::UserTask(_) => true,
                _ => false,
            })
            .unwrap_or(false)
    }

    #[async_recursion]
    async fn sim_run(&self) {
        if let Some(future_task) = self.fetch_future_task().await {
            match &future_task.def {
                wfrs_model::TaskDef::StartEvent(ev) => {
                    self.visit_future_outgoing(&ev.outgoing).await;
                    self.sim_run().await;
                }
                wfrs_model::TaskDef::UserTask(ev) => {
                    self.visit_future_outgoing(&ev.outgoing).await;
                    self.sim_run().await;
                }
                wfrs_model::TaskDef::ExclusiveGateway(ev) => {
                    let out = async {
                        let state = self.instance.state().await;
                        ExclusiveGateway(ev).evaluate(self.definition, &state.inner.variables)
                    }
                    .await;
                    self.visit_future_outgoing(&out).await;
                    self.sim_run().await;
                }
                wfrs_model::TaskDef::EndEvent(_) => {}
            }
        }

        if let Some(future_flow) = self.fetch_future_flow().await {
            self.instance
                .push_maybe_future_task(future_flow.target_ref)
                .await;
            if self.is_usertask(future_flow.source_ref) {
                self.instance
                    .push_maybe_visited_task(future_flow.source_ref)
                    .await;
            }
            self.sim_run().await;
        }
    }

    pub async fn simulate(&self) {
        self.instance
            .clear_future(self.definition.root_start_event())
            .await;
        self.sim_run().await;
    }

    pub async fn navigate_to(&self, task_id: i32) {
        if let Some(task) = self.definition.tasks.get(task_id as usize) {
            let visited = self.instance.has_visited(task_id).await
                && self.instance.has_maybe_visited(task_id).await;
            if task.is_user_task() && visited {
                self.instance.set_usertask(task_id).await;
                self.simulate().await;
            } else {
                let next_user_task = self.get_previous_user_task(task_id).await;
                if let Some(task_id) = next_user_task {
                    let visited = self.instance.has_visited(task_id).await;
                    if visited {
                        self.instance.set_usertask(task_id).await;
                        self.simulate().await;
                    }
                }
            }
        }
    }

    pub async fn get_previous_user_task(&self, task_id: i32) -> Option<i32> {
        let mut task_id = task_id;
        let mut result = None;
        let state = self.instance.state().await;
        loop {
            let pos = state
                .inner
                .visited_tasks
                .iter()
                .position(|t| t == &task_id)
                .and_then(|pos| {
                    if pos > 0 {
                        state.inner.visited_tasks.get(pos - 1)
                    } else {
                        None
                    }
                });
            if let Some(pos) = pos {
                if let Some(task) = self.definition.tasks.get(*pos as usize) {
                    if task.is_user_task() {
                        result = Some(*pos);
                        break;
                    } else {
                        task_id = *pos;
                    }
                }
            } else {
                break;
            }
        }
        result
    }
}
