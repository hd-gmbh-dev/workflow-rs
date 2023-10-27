use crate::jsep::JsepNode;
use rkyv::{Archive, Deserialize, Serialize};
use std::sync::Arc;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct StartEventDef {
    pub outgoing: Arc<[i32]>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct EndEventDef {
    pub incoming: Arc<[i32]>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct UserTaskDef {
    pub incoming: Arc<[i32]>,
    pub outgoing: Arc<[i32]>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct ExclusiveGatewayDef {
    pub incoming: Arc<[i32]>,
    pub outgoing: Arc<[i32]>,
    pub default: i32,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum TaskDef {
    StartEvent(StartEventDef),
    EndEvent(EndEventDef),
    UserTask(UserTaskDef),
    ExclusiveGateway(ExclusiveGatewayDef),
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct Task {
    pub id: i32,
    pub def: TaskDef,
}

impl Task {
    pub fn is_user_task(&self) -> bool {
        match &self.def {
            TaskDef::UserTask(_) => true,
            _ => false,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum ConditionExpression {
    Jsep(JsepNode),
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct Flow {
    pub id: i32,
    pub source_ref: i32,
    pub target_ref: i32,
    pub condition_expression: Option<ConditionExpression>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct WorkflowProperties {
    pub autostart: bool,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(
    bound(
        serialize = "__S: rkyv::ser::ScratchSpace + rkyv::ser::SharedSerializeRegistry + rkyv::ser::Serializer",
        deserialize = "__D: rkyv::de::SharedDeserializeRegistry"
    ),
    compare(PartialEq)
)]
#[archive_attr(derive(Debug))]
pub struct WorkflowDefinition {
    pub version: Arc<str>,
    pub id: Arc<str>,
    pub start_event: i32,
    #[omit_bounds]
    pub parent: Option<Arc<WorkflowDefinition>>,
    pub flows: Arc<[Flow]>,
    pub flow_ids: Arc<[Arc<str>]>,
    pub tasks: Arc<[Task]>,
    pub task_ids: Arc<[Arc<str>]>,
    #[omit_bounds]
    pub children: Option<Arc<[WorkflowDefinition]>>,
    pub options: Option<WorkflowProperties>,
}

impl WorkflowDefinition {
    pub fn root_start_event(&self) -> i32 {
        if let Some(parent) = self.parent.as_ref() {
            parent.root_start_event()
        } else {
            self.start_event
        }
    }

    pub fn format_id(&self, id: &str) -> String {
        format!("{}_{id}", self.id.as_ref())
    }

    pub fn user_tasks(&self) -> Vec<i32> {
        self.tasks
            .iter()
            .enumerate()
            .filter_map(|(idx, task)| {
                if task.is_user_task() {
                    Some(idx as i32)
                } else {
                    None
                }
            })
            .collect()
    }
}
