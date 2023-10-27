use rkyv::{Archive, Deserialize, Serialize};
use std::{collections::HashMap, fmt, hash::Hash};

#[derive(Archive, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JsonKey(String);

impl AsRef<str> for JsonKey {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Archive, Debug, Deserialize, Serialize, PartialEq)]
#[archive(
    bound(
        serialize = "__S: rkyv::ser::ScratchSpace + rkyv::ser::SharedSerializeRegistry + rkyv::ser::Serializer",
        deserialize = "__D: rkyv::de::SharedDeserializeRegistry"
    ),
    check_bytes
)]
#[archive_attr(
    check_bytes(
        bound = "__C: rkyv::validation::ArchiveContext, <__C as rkyv::Fallible>::Error: rkyv::bytecheck::Error"
    ),
    derive(Debug)
)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(
        #[omit_bounds]
        #[archive_attr(omit_bounds)]
        Vec<JsonValue>,
    ),
    Object(
        #[omit_bounds]
        #[archive_attr(omit_bounds)]
        HashMap<String, JsonValue>,
    ),
}

impl PartialEq<JsonValue> for ArchivedJsonValue {
    fn eq(&self, other: &JsonValue) -> bool {
        match self {
            ArchivedJsonValue::Null => other.is_null(),
            ArchivedJsonValue::Bool(v) => Some(*v) == other.as_bool(),
            ArchivedJsonValue::Number(v) => match v {
                ArchivedJsonNumber::Float(v) => Some(*v) == other.as_f64(),
                ArchivedJsonNumber::PosInt(v) => Some(*v) == other.as_u64(),
                ArchivedJsonNumber::NegInt(v) => Some(*v) == other.as_i64(),
            },
            ArchivedJsonValue::String(v) => match other {
                JsonValue::String(s) => s.as_str() == v.as_str(),
                _ => false,
            },
            ArchivedJsonValue::Array(v) => {
                if other.is_array() {
                    let other = other.as_array().unwrap();
                    let o = other.len();
                    let l = v.len();
                    if o != l {
                        return false;
                    }
                    for i in 0..l {
                        if !v[i].eq(&other[i]) {
                            return false;
                        }
                    }
                    return true;
                }
                false
            }
            ArchivedJsonValue::Object(v) => {
                if other.is_object() {
                    let other = other.as_object().unwrap();
                    let o = other.len();
                    let l = v.len();
                    if o != l {
                        return false;
                    }
                    for (key, value) in other.iter() {
                        if !v.contains_key(key.as_str()) {
                            return false;
                        }
                        if !v.get(key.as_str()).unwrap().eq(value) {
                            return false;
                        }
                    }
                    return true;
                }
                false
            }
        }
    }
}

impl JsonValue {
    pub fn map() -> Self {
        Self::Object(HashMap::default())
    }

    pub fn is_null(&self) -> bool {
        match self {
            JsonValue::Null => true,
            _ => false,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            JsonValue::Object(_) => true,
            _ => false,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            JsonValue::Array(_) => true,
            _ => false,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&JsonNumber> {
        match self {
            JsonValue::Number(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => match n {
                JsonNumber::Float(v) => Some(*v),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => match n {
                JsonNumber::NegInt(v) => Some(*v),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            JsonValue::Number(n) => match n {
                JsonNumber::PosInt(v) => Some(*v),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }
}

impl fmt::Display for ArchivedJsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null")?,
            Self::Bool(b) => write!(f, "{}", b)?,
            Self::Number(n) => write!(f, "{}", n)?,
            Self::String(s) => write!(f, "{}", s)?,
            Self::Array(a) => {
                write!(f, "[")?;
                for (i, value) in a.iter().enumerate() {
                    write!(f, "{}", value)?;
                    if i < a.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")?;
            }
            Self::Object(h) => {
                write!(f, "{{")?;
                for (i, (key, value)) in h.iter().enumerate() {
                    write!(f, "\"{}\": {}", key, value)?;
                    if i < h.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "}}")?;
            }
        }
        Ok(())
    }
}

#[derive(Archive, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
pub enum JsonNumber {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

impl JsonNumber {
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::PosInt(n) => *n as f64,
            Self::NegInt(n) => *n as f64,
            Self::Float(n) => *n,
        }
    }
}

impl fmt::Display for ArchivedJsonNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PosInt(n) => write!(f, "{}", n),
            Self::NegInt(n) => write!(f, "{}", n),
            Self::Float(n) => write!(f, "{}", n),
        }
    }
}

impl PartialEq<JsonNumber> for ArchivedJsonNumber {
    fn eq(&self, other: &JsonNumber) -> bool {
        match self {
            ArchivedJsonNumber::PosInt(v) => JsonNumber::PosInt(*v).eq(other),
            ArchivedJsonNumber::NegInt(v) => JsonNumber::NegInt(*v).eq(other),
            ArchivedJsonNumber::Float(v) => JsonNumber::Float(*v).eq(other),
        }
    }
}

impl PartialOrd<JsonNumber> for ArchivedJsonNumber {
    fn partial_cmp(&self, other: &JsonNumber) -> Option<std::cmp::Ordering> {
        match self {
            ArchivedJsonNumber::PosInt(v) => JsonNumber::PosInt(*v).partial_cmp(other),
            ArchivedJsonNumber::NegInt(v) => JsonNumber::NegInt(*v).partial_cmp(other),
            ArchivedJsonNumber::Float(v) => JsonNumber::Float(*v).partial_cmp(other),
        }
    }
}
