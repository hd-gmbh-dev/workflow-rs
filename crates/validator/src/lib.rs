use wfrs_model::{
    jsep::{BinaryExpression, JsepNode, MemberExpression},
    json::JsonValue,
    ConditionExpression,
};
use wfrs_model::{ExclusiveGatewayDef, WorkflowDefinition};

pub struct Member<'a>(&'a MemberExpression);

impl<'a> Member<'a> {
    fn resolve<'v>(&self, mut variables: &'v JsonValue) -> &'v JsonValue {
        if let JsepNode::Identifier(property) = self.0.property.as_ref() {
            if let JsepNode::MemberExpression(member) = self.0.object.as_ref() {
                variables = Member(member).resolve(variables);
                if let Some(variables) = variables.as_object() {
                    if let Some(variables) = variables.get(property.name.as_ref()) {
                        return variables;
                    }
                }
            }
            if let JsepNode::Identifier(object) = self.0.object.as_ref() {
                if object.name.as_ref() == "$steps" {
                    if let Some(variables) = variables.as_object() {
                        if let Some(variables) = variables.get(property.name.as_ref()) {
                            return variables;
                        }
                    }
                }
            }
        }
        &JsonValue::Null
    }
}

pub struct Value<'a>(&'a JsepNode);

impl<'a> Value<'a> {
    fn resolve<'v>(&self, variables: &'v JsonValue) -> &'v JsonValue
    where
        'a: 'v,
    {
        match self.0 {
            JsepNode::MemberExpression(member) => Member(member).resolve(variables),
            JsepNode::Literal(lit) => &lit.value,
            _ => &JsonValue::Null,
        }
    }
}

pub struct Binary<'a>(&'a BinaryExpression);

impl<'a> Binary<'a> {
    pub fn validate(&self, variables: &JsonValue) -> bool {
        let left = Value(&self.0.left).resolve(variables);
        let right = Value(&self.0.right).resolve(variables);
        match self.0.operator {
            wfrs_model::jsep::Operator::Equal => left == right,
            wfrs_model::jsep::Operator::NotEqual => left != right,
            wfrs_model::jsep::Operator::Greater => {
                if let Some((l, r)) = left.as_number().zip(right.as_number()) {
                    return l > r;
                } else if let Some((l, r)) = left.as_str().zip(right.as_str()) {
                    return l > r;
                } else if let Some((l, r)) = left.as_bool().zip(right.as_bool()) {
                    return l & !r;
                }
                false
            }
            wfrs_model::jsep::Operator::GreaterOrEqual => {
                if let Some((l, r)) = left.as_number().zip(right.as_number()) {
                    return l >= r;
                } else if let Some((l, r)) = left.as_str().zip(right.as_str()) {
                    return l >= r;
                } else if let Some((l, r)) = left.as_bool().zip(right.as_bool()) {
                    return l >= r;
                }
                false
            }
            wfrs_model::jsep::Operator::Lower => {
                if let Some((l, r)) = left.as_number().zip(right.as_number()) {
                    return l < r;
                } else if let Some((l, r)) = left.as_str().zip(right.as_str()) {
                    return l < r;
                } else if let Some((l, r)) = left.as_bool().zip(right.as_bool()) {
                    return !l & r;
                }
                false
            }
            wfrs_model::jsep::Operator::LowerOrEqual => {
                if let Some((l, r)) = left.as_number().zip(right.as_number()) {
                    return l <= r;
                } else if let Some((l, r)) = left.as_str().zip(right.as_str()) {
                    return l <= r;
                } else if let Some((l, r)) = left.as_bool().zip(right.as_bool()) {
                    return l <= r;
                }
                false
            }
            wfrs_model::jsep::Operator::And => {
                if let Some((l, r)) = left.as_bool().zip(right.as_bool()) {
                    if l && r {
                        return true;
                    }
                }
                false
            }
            wfrs_model::jsep::Operator::Or => {
                if let Some(l) = left.as_bool() {
                    if l {
                        return true;
                    }
                }
                if let Some(r) = right.as_bool() {
                    if r {
                        return true;
                    }
                }
                false
            }
        }
    }
}

pub struct Condition<'a>(pub &'a ConditionExpression);

impl<'a> Condition<'a> {
    pub fn validate(&self, variables: &JsonValue) -> bool {
        match self.0 {
            ConditionExpression::Jsep(node) => match node {
                wfrs_model::jsep::JsepNode::BinaryExpression(binary_expr) => {
                    Binary(binary_expr).validate(variables)
                }
                _ => false,
            },
        }
    }
}

pub struct ExclusiveGateway<'a>(pub &'a ExclusiveGatewayDef);

impl<'a> ExclusiveGateway<'a> {
    pub fn evaluate(&self, definition: &WorkflowDefinition, variables: &JsonValue) -> [i32; 1] {
        let mut out = [self.0.default];
        for outgoing in self.0.outgoing.iter() {
            if let Some(expr) = definition
                .flows
                .get(*outgoing as usize)
                .and_then(|f| f.condition_expression.as_ref())
            {
                if Condition(expr).validate(variables) {
                    out = [*outgoing];
                    break;
                }
            }
        }
        out
    }
}
