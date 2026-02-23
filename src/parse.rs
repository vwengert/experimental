use crate::domain::{Value, ValueType};

pub fn parse_value(s: &str, ty: ValueType) -> Result<Value, String> {
    match ty {
        ValueType::Str => Ok(Value::Str(s.to_owned())),
        ValueType::Int => s.parse::<i64>().map(Value::Int).map_err(|e| e.to_string()),
        ValueType::Float => s.parse::<f64>().map(Value::Float).map_err(|e| e.to_string()),
        ValueType::Bool => s.parse::<bool>().map(Value::Bool).map_err(|e| e.to_string()),
    }
}

pub fn value_to_string(v: &Value) -> String {
    match v {
        Value::Str(x) => x.clone(),
        Value::Int(x) => x.to_string(),
        Value::Float(x) => x.to_string(),
        Value::Bool(x) => x.to_string(),
    }
}
