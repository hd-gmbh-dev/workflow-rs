use rkyv::{Archive, Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

use crate::json::JsonValue;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum Operator {
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Lower,
    LowerOrEqual,
    And,
    Or,
}

impl FromStr for Operator {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "==" => Ok(Operator::Equal),
            "!=" => Ok(Operator::NotEqual),
            ">" => Ok(Operator::Greater),
            ">=" => Ok(Operator::GreaterOrEqual),
            "<" => Ok(Operator::Lower),
            "<=" => Ok(Operator::LowerOrEqual),
            "&&" => Ok(Operator::And),
            "||" => Ok(Operator::Or),
            _ => Err(format!("Invalid operator '{s}'")),
        }
    }
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
pub struct BinaryExpression {
    pub operator: Operator,
    #[omit_bounds]
    pub left: Box<JsepNode>,
    #[omit_bounds]
    pub right: Box<JsepNode>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct ExpressionIdentifier {
    pub name: Arc<str>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct ExpressionLiteral {
    pub value: JsonValue,
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
pub struct ConditionalExpression {
    #[omit_bounds]
    pub test: Box<JsepNode>,
    #[omit_bounds]
    pub consequent: Box<JsepNode>,
    #[omit_bounds]
    pub alternate: Box<JsepNode>,
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
pub struct MemberExpression {
    pub computed: bool,
    pub optional: bool,
    #[omit_bounds]
    pub object: Box<JsepNode>,
    #[omit_bounds]
    pub property: Box<JsepNode>,
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
pub enum JsepNode {
    ConditionalExpression(ConditionalExpression),
    BinaryExpression(BinaryExpression),
    Identifier(ExpressionIdentifier),
    Literal(ExpressionLiteral),
    MemberExpression(MemberExpression),
}
