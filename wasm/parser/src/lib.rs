mod error;
mod utils;

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::io::Seek;

use crate::error::XmlError;
use js_sys::Uint8Array;
use quick_xml::de::Deserializer;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use wfrs_model::jsep::Operator;
use wfrs_model::json::JsonValue;
use wfrs_model::{
    serialize, ConditionExpression, EndEventDef, ExclusiveGatewayDef, Flow, StartEventDef, Task,
    TaskDef, UserTaskDef, WorkflowDefinition, WorkflowProperties,
};
use std::io::SeekFrom;
#[wasm_bindgen(module = "@wfrs/vite-plugin-helper")]
extern "C" {
    #[wasm_bindgen(js_name = "parseJsepExpression")]
    fn parse_jsep_expression(s: String) -> String;
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn init() {
    utils::set_panic_hook();
}

#[derive(Debug, Deserialize)]
pub struct BinaryExpression {
    pub operator: String,
    pub left: Box<JsepNode>,
    pub right: Box<JsepNode>,
}

impl<'a> From<&'a BinaryExpression> for wfrs_model::jsep::BinaryExpression {
    fn from(val: &'a BinaryExpression) -> Self {
        let operator = match Operator::from_str(val.operator.as_str()) {
            Ok(operator) => operator,
            Err(err) => {
                panic!("unable to parse condition expression {err}")
            }
        };
        wfrs_model::jsep::BinaryExpression {
            operator,
            left: Box::new(val.left.as_ref().into()),
            right: Box::new(val.right.as_ref().into()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ExpressionIdentifier {
    pub name: String,
}

impl<'a> From<&'a ExpressionIdentifier> for wfrs_model::jsep::ExpressionIdentifier {
    fn from(val: &'a ExpressionIdentifier) -> Self {
        wfrs_model::jsep::ExpressionIdentifier {
            name: Arc::from(val.name.clone()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ExpressionLiteral {
    pub value: serde_json::Value,
}

fn to_json_value(value: &serde_json::Value) -> JsonValue {
    match value {
        serde_json::Value::Null => JsonValue::Null,
        serde_json::Value::Bool(b) => JsonValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                JsonValue::Number(wfrs_model::json::JsonNumber::NegInt(n.as_i64().unwrap()))
            } else if n.is_u64() {
                return JsonValue::Number(wfrs_model::json::JsonNumber::PosInt(
                    n.as_u64().unwrap(),
                ));
            } else if n.is_f64() {
                return JsonValue::Number(wfrs_model::json::JsonNumber::Float(n.as_f64().unwrap()));
            } else {
                return JsonValue::Number(wfrs_model::json::JsonNumber::PosInt(0));
            }
        }
        serde_json::Value::String(s) => JsonValue::String(s.to_owned()),
        serde_json::Value::Array(a) => JsonValue::Array(a.iter().map(to_json_value).collect()),
        serde_json::Value::Object(o) => {
            let mut obj = HashMap::default();
            for (k, v) in o {
                obj.insert(k.to_string(), to_json_value(v));
            }
            JsonValue::Object(obj)
        }
    }
}

impl<'a> From<&'a ExpressionLiteral> for wfrs_model::jsep::ExpressionLiteral {
    fn from(val: &'a ExpressionLiteral) -> Self {
        wfrs_model::jsep::ExpressionLiteral {
            value: to_json_value(&val.value),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ConditionalExpression {
    pub test: Box<JsepNode>,
    pub consequent: Box<JsepNode>,
    pub alternate: Box<JsepNode>,
}

impl<'a> From<&'a ConditionalExpression> for wfrs_model::jsep::ConditionalExpression {
    fn from(val: &'a ConditionalExpression) -> Self {
        wfrs_model::jsep::ConditionalExpression {
            test: Box::new(val.test.as_ref().into()),
            consequent: Box::new(val.consequent.as_ref().into()),
            alternate: Box::new(val.alternate.as_ref().into()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MemberExpression {
    #[serde(default)]
    pub computed: bool,
    #[serde(default)]
    pub optional: bool,
    pub object: Box<JsepNode>,
    pub property: Box<JsepNode>,
}

impl<'a> From<&'a MemberExpression> for wfrs_model::jsep::MemberExpression {
    fn from(val: &'a MemberExpression) -> Self {
        wfrs_model::jsep::MemberExpression {
            computed: val.computed,
            optional: val.optional,
            object: Box::new(val.object.as_ref().into()),
            property: Box::new(val.property.as_ref().into()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum JsepNode {
    ConditionalExpression(ConditionalExpression),
    BinaryExpression(BinaryExpression),
    Identifier(ExpressionIdentifier),
    Literal(ExpressionLiteral),
    MemberExpression(MemberExpression),
}

impl<'a> From<&'a JsepNode> for wfrs_model::jsep::JsepNode {
    fn from(val: &'a JsepNode) -> Self {
        match val {
            JsepNode::ConditionalExpression(n) => {
                wfrs_model::jsep::JsepNode::ConditionalExpression(n.into())
            }
            JsepNode::BinaryExpression(n) => wfrs_model::jsep::JsepNode::BinaryExpression(n.into()),
            JsepNode::Identifier(n) => wfrs_model::jsep::JsepNode::Identifier(n.into()),
            JsepNode::Literal(n) => wfrs_model::jsep::JsepNode::Literal(n.into()),
            JsepNode::MemberExpression(n) => wfrs_model::jsep::JsepNode::MemberExpression(n.into()),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Connection {
    #[serde(rename = "$text")]
    id: Arc<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct StartEvent {
    #[serde(rename = "@id")]
    id: Arc<str>,
    outgoing: Arc<[Connection]>,
}

#[derive(Debug, serde::Deserialize)]
pub struct EndEvent {
    #[serde(rename = "@id")]
    id: Arc<str>,
    incoming: Arc<[Connection]>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UserTask {
    #[serde(rename = "@id")]
    id: Arc<str>,
    incoming: Arc<[Connection]>,
    outgoing: Arc<[Connection]>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ExclusiveGateway {
    #[serde(rename = "@id")]
    id: Arc<str>,
    #[serde(rename = "@default")]
    default: Option<Arc<str>>,
    incoming: Arc<[Connection]>,
    outgoing: Arc<[Connection]>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BpmnExpression {
    #[serde(rename = "@language")]
    language: Arc<str>,
    #[serde(rename = "$text")]
    expr: Arc<str>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceFlow {
    #[serde(rename = "@id")]
    id: Arc<str>,
    #[serde(rename = "@sourceRef")]
    source_ref: Arc<str>,
    #[serde(rename = "@targetRef")]
    target_ref: Arc<str>,
    condition_expression: Option<BpmnExpression>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Text {
    #[serde(rename = "$text")]
    _content: Arc<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TextAnnotation {
    #[serde(rename = "@id")]
    id: Arc<str>,
    #[serde(rename = "text")]
    _text: Text,
}

#[derive(Debug, serde::Deserialize)]
pub struct Association {
    #[serde(rename = "@id")]
    id: Arc<str>,
    #[serde(rename = "@sourceRef")]
    _source_ref: Arc<str>,
    #[serde(rename = "@targetRef")]
    _target_ref: Arc<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Property {
    #[serde(rename = "@name")]
    name: Option<Arc<str>>,
    #[serde(rename = "@value")]
    _value: Option<Arc<str>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Properties {
    property: Arc<[Property]>,
}

impl Properties {
    fn has_flag(&self, name: &str) -> bool {
        self.property
            .iter()
            .any(|p| p.name.as_deref() == Some(name))
    }
}

impl From<Properties> for WorkflowProperties {
    fn from(val: Properties) -> Self {
        WorkflowProperties {
            autostart: val.has_flag("autostart"),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ExtensionElements {
    properties: Option<Properties>,
}

pub enum BpmnEvent {
    StartEvent(StartEvent),
    EndEvent(EndEvent),
    UserTask(UserTask),
    ExclusiveGateway(ExclusiveGateway),
    SequenceFlow(SequenceFlow),
    TextAnnotation(TextAnnotation),
    Association(Association),
}

impl BpmnEvent {
    pub fn id(&self) -> Arc<str> {
        match self {
            BpmnEvent::StartEvent(e) => e.id.clone(),
            BpmnEvent::EndEvent(e) => e.id.clone(),
            BpmnEvent::UserTask(e) => e.id.clone(),
            BpmnEvent::ExclusiveGateway(e) => e.id.clone(),
            BpmnEvent::SequenceFlow(e) => e.id.clone(),
            BpmnEvent::TextAnnotation(e) => e.id.clone(),
            BpmnEvent::Association(e) => e.id.clone(),
        }
    }
}

pub struct ProcessDefinition {
    id: String,
    version: String,
    events: Vec<BpmnEvent>,
    options: Option<WorkflowProperties>,
}

fn read_to_end_into_buffer<R: BufRead>(
    reader: &mut Reader<R>,
    start_tag: &BytesStart,
    junk_buf: &mut Vec<u8>,
) -> Result<Vec<u8>, quick_xml::Error> {
    let mut depth = 0;
    let mut output_buf: Vec<u8> = Vec::new();
    let mut w = Writer::new(&mut output_buf);
    let tag_name = start_tag.name();
    w.write_event(Event::Start(start_tag.clone()))?;
    loop {
        junk_buf.clear();
        let event = reader.read_event_into(junk_buf)?;
        w.write_event(&event)?;

        match event {
            Event::Start(e) if e.name() == tag_name => depth += 1,
            Event::End(e) if e.name() == tag_name => {
                if depth == 0 {
                    return Ok(output_buf);
                }
                depth -= 1;
            }
            Event::Eof => {
                panic!("oh no")
            }
            _ => {}
        }
    }
}

fn read_element<T, R: BufRead>(
    reader: &mut Reader<R>,
    start_tag: &BytesStart,
    junk_buf: &mut Vec<u8>,
) -> Result<T, quick_xml::de::DeError>
where
    T: DeserializeOwned,
{
    let release_bytes = read_to_end_into_buffer(reader, start_tag, junk_buf).unwrap();
    let str = std::str::from_utf8(&release_bytes).unwrap();
    // deserialize from buffer
    let mut deserializer = Deserializer::from_str(str);
    T::deserialize(&mut deserializer)
}

fn read_process_definition<R: BufRead + Seek>(
    reader: &mut Reader<R>,
    start_tag: &BytesStart,
    buf: &mut Vec<u8>,
) -> Result<ProcessDefinition, XmlError> {
    let mut options = None;
    let tag_name = start_tag.name();
    let mut junk_buf: Vec<u8> = Vec::new();
    let id = start_tag
        .try_get_attribute("id")?
        .unwrap()
        .unescape_value()?
        .to_string();
    let version = start_tag
        .try_get_attribute("camunda:versionTag")?
        .unwrap()
        .unescape_value()?
        .to_string();
    let mut events = Vec::new();
    loop {
        junk_buf.clear();
        let event = reader.read_event_into(buf)?;
        match event {
            Event::Start(e) => {
                let name = e.name();
                let element = unsafe { std::str::from_utf8_unchecked(name.as_ref()) };
                let mut s = element.split(':');
                let a = s.next();
                let n = s.next();
                if let Some(name) = n {
                    if a == Some("bpmn") {
                        match name {
                            "extensionElements" => {
                                let ExtensionElements { properties } = read_element::<
                                    ExtensionElements,
                                    _,
                                >(
                                    reader, &e, &mut junk_buf
                                )?;
                                if let Some(properties) = properties {
                                    options = Some(properties.into());
                                }
                            }
                            "textAnnotation" => events.push(BpmnEvent::TextAnnotation(
                                read_element(reader, &e, &mut junk_buf)?,
                            )),
                            "association" => events.push(BpmnEvent::Association(read_element(
                                reader,
                                &e,
                                &mut junk_buf,
                            )?)),
                            "startEvent" => events.push(BpmnEvent::StartEvent(read_element(
                                reader,
                                &e,
                                &mut junk_buf,
                            )?)),
                            "endEvent" => events.push(BpmnEvent::EndEvent(read_element(
                                reader,
                                &e,
                                &mut junk_buf,
                            )?)),
                            "userTask" => events.push(BpmnEvent::UserTask(read_element(
                                reader,
                                &e,
                                &mut junk_buf,
                            )?)),
                            "exclusiveGateway" => events.push(BpmnEvent::ExclusiveGateway(
                                read_element(reader, &e, &mut junk_buf)?,
                            )),
                            "sequenceFlow" => events.push(BpmnEvent::SequenceFlow(read_element(
                                reader,
                                &e,
                                &mut junk_buf,
                            )?)),
                            _ => {
                                let pos = reader.buffer_position();
                                let reader = reader.get_mut();
                                reader.seek(SeekFrom::Start(0)).unwrap();
                                let mut buf = String::new();
                                let mut row = 0;
                                let mut r = 0;
                                while let Some(line) = reader.read_line(&mut buf).ok() {
                                    row += 1;
                                    r += line;
                                    if r > pos {
                                        break;
                                    }
                                    if line == 0 {
                                        break;
                                    }
                                }
                                unimplemented!("Element '{}' at line {row} is not implemented yet", element)
                            }
                        }
                    }
                }
            }
            Event::End(e) if e.name() == tag_name => {
                return Ok(ProcessDefinition {
                    id,
                    version,
                    events,
                    options,
                });
            }
            Event::Eof => {
                panic!("oh no")
            }
            _ => {}
        }
    }
}

fn parse_xml(xml: &str) -> Result<Option<ProcessDefinition>, XmlError> {
    let mut reader = Reader::from_reader(BufReader::new(std::io::Cursor::new(xml)));
    let mut buf = Vec::new();
    reader.trim_text(true);
    reader.expand_empty_elements(true);
    let mut result = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                let name = e.name();
                let element = unsafe { std::str::from_utf8_unchecked(name.as_ref()) };
                let mut s = element.split(':');
                let a = s.next();
                let n = s.next();
                if let Some(name) = n {
                    if a == Some("bpmn") {
                        match name {
                            "definitions" => (),
                            "process" => {
                                let mut buf = Vec::new();
                                result = Some(read_process_definition(&mut reader, &e, &mut buf)?);
                                break;
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => (),
        }
    }
    Ok(result)
}

fn find_index(id: &Arc<str>, list: &[(Arc<str>, BpmnEvent)]) -> i32 {
    list.iter()
        .position(|(tid, _)| tid.as_ref() == id.as_ref())
        .map(|i| i as i32)
        .unwrap_or(-1)
}

fn find_connections(connections: &[Connection], list: &[(Arc<str>, BpmnEvent)]) -> Arc<[i32]> {
    Arc::from(
        connections
            .iter()
            .map(|c| find_index(&c.id, list))
            .collect::<Vec<i32>>(),
    )
}

fn parse_expression(expression: &BpmnExpression) -> Option<ConditionExpression> {
    match expression.language.as_ref() {
        "jsep" => {
            let jsep_expression = parse_jsep_expression(expression.expr.as_ref().to_string());
            match serde_json::from_str::<JsepNode>(&jsep_expression) {
                Ok(jsep) => Some(ConditionExpression::Jsep((&jsep).into())),
                Err(e) => {
                    panic!("{e:#?}");
                }
            }
        }
        _ => None,
    }
}

struct WorkflowDefinitionBuilder {
    id: String,
    version: String,
    tasks: Vec<(Arc<str>, BpmnEvent)>,
    flows: Vec<(Arc<str>, BpmnEvent)>,
    options: Option<WorkflowProperties>,
}

impl WorkflowDefinitionBuilder {
    fn new(process_definition: ProcessDefinition) -> Self {
        let mut tasks: BTreeMap<Arc<str>, BpmnEvent> = BTreeMap::new();
        let mut flows: BTreeMap<Arc<str>, BpmnEvent> = BTreeMap::new();
        for event in process_definition.events {
            match event {
                BpmnEvent::SequenceFlow(f) => {
                    let event = BpmnEvent::SequenceFlow(f);
                    flows.insert(event.id(), event);
                }
                BpmnEvent::TextAnnotation(_) => {}
                BpmnEvent::Association(_) => {}
                event => {
                    tasks.insert(event.id(), event);
                }
            }
        }
        Self {
            id: process_definition.id,
            version: process_definition.version,
            options: process_definition.options,
            tasks: Vec::from_iter(tasks),
            flows: Vec::from_iter(flows),
        }
    }

    fn build(self) -> WorkflowDefinition {
        let Self {
            tasks,
            flows,
            id,
            version,
            options,
        } = self;
        let mut start_event = -1;
        let mut result_flows = Vec::new();
        let mut result_flow_ids = Vec::new();
        let mut result_tasks = Vec::new();
        let mut result_task_ids = Vec::new();
        for (id, (tid, event)) in tasks.iter().enumerate() {
            match event {
                BpmnEvent::StartEvent(e) => {
                    start_event = id as i32;
                    result_tasks.push(Task {
                        id: id as i32,
                        def: TaskDef::StartEvent(StartEventDef {
                            outgoing: find_connections(&e.outgoing, &flows),
                        }),
                    });
                    result_task_ids.push(tid.clone());
                }
                BpmnEvent::EndEvent(e) => {
                    result_tasks.push(Task {
                        id: id as i32,
                        def: TaskDef::EndEvent(EndEventDef {
                            incoming: find_connections(&e.incoming, &flows),
                        }),
                    });
                    result_task_ids.push(tid.clone());
                }
                BpmnEvent::UserTask(e) => {
                    result_tasks.push(Task {
                        id: id as i32,
                        def: TaskDef::UserTask(UserTaskDef {
                            incoming: find_connections(&e.incoming, &flows),
                            outgoing: find_connections(&e.outgoing, &flows),
                        }),
                    });
                    result_task_ids.push(tid.clone());
                }
                BpmnEvent::ExclusiveGateway(e) => {
                    let outgoing = find_connections(&e.outgoing, &flows);
                    let default = if let Some(default_flow) = e.default.as_ref() {
                        find_index(default_flow, &flows)
                    } else {
                        *outgoing.first().unwrap_or(&-1)
                    };
                    result_tasks.push(Task {
                        id: id as i32,
                        def: TaskDef::ExclusiveGateway(ExclusiveGatewayDef {
                            incoming: find_connections(&e.incoming, &flows),
                            outgoing,
                            default,
                        }),
                    });
                    result_task_ids.push(tid.clone());
                }
                _ => {}
            }
        }
        for (id, (fid, event)) in flows.iter().enumerate() {
            match event {
                BpmnEvent::SequenceFlow(f) => {
                    result_flows.push(Flow {
                        id: id as i32,
                        source_ref: find_index(&f.source_ref, &tasks),
                        target_ref: find_index(&f.target_ref, &tasks),
                        condition_expression: f
                            .condition_expression
                            .as_ref()
                            .and_then(parse_expression),
                    });
                    result_flow_ids.push(fid.clone());
                }
                _ => (),
            }
        }
        WorkflowDefinition {
            id: Arc::from(id),
            start_event,
            version: Arc::from(version),
            flows: Arc::from(result_flows),
            flow_ids: Arc::from(result_flow_ids),
            tasks: Arc::from(result_tasks),
            task_ids: Arc::from(result_task_ids),
            parent: None, // TODO: implement sub processes
            children: None,
            options,
        }
    }
}

impl From<ProcessDefinition> for WorkflowDefinition {
    fn from(val: ProcessDefinition) -> Self {
        WorkflowDefinitionBuilder::new(val).build()
    }
}

fn parse_workflow(xml: &str) -> Result<WorkflowDefinition, XmlError> {
    Ok(parse_xml(xml)?.ok_or(XmlError::NoProcessDefinition)?.into())
}

#[wasm_bindgen]
pub fn parse(xml: &str) -> Result<Uint8Array, String> {
    let result = parse_workflow(xml).map_err(|e| format!("{e:#?}"))?;
    Ok(serialize(result).as_slice().into())
}
