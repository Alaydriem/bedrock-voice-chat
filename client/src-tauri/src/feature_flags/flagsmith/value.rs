use std::collections::HashMap;

use open_feature::{StructValue, Value};

#[derive(Debug, Clone)]
pub(crate) enum FlagsmithFlagValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Json(serde_json::Value),
}

impl FlagsmithFlagValue {
    pub fn from_json(val: &serde_json::Value) -> Self {
        match val {
            serde_json::Value::Bool(b) => Self::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Self::Float(f)
                } else {
                    Self::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => Self::String(s.clone()),
            other => Self::Json(other.clone()),
        }
    }

    pub fn to_of_value(&self) -> Value {
        match self {
            Self::Bool(b) => Value::Bool(*b),
            Self::Int(i) => Value::Int(*i),
            Self::Float(f) => Value::Float(*f),
            Self::String(s) => Value::String(s.clone()),
            Self::Json(v) => Self::json_to_of_value(v),
        }
    }

    fn json_to_of_value(val: &serde_json::Value) -> Value {
        match val {
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.iter().map(Self::json_to_of_value).collect())
            }
            serde_json::Value::Object(obj) => {
                let fields: HashMap<String, Value> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::json_to_of_value(v)))
                    .collect();
                Value::Struct(StructValue { fields })
            }
            serde_json::Value::Null => Value::String(String::new()),
        }
    }
}
