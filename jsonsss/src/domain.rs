use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub name: String,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    Str,
    Int,
    Float,
    Bool,
}

fn default_required() -> bool {
    true
}
fn is_true(value: &bool) -> bool {
    *value
}

fn default_allow_unknown_keys() -> bool {
    false
}
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySpec {
    pub ty: ValueType,

    #[serde(default = "default_required", skip_serializing_if = "is_true")]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSchema {
    #[serde(rename = "values")]
    pub allowed: HashMap<String, KeySpec>,

    #[serde(default = "default_allow_unknown_keys", skip_serializing_if = "is_false")]
    pub allow_unknown_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ElementSchemas(pub HashMap<String, ElementSchema>);

impl ElementSchemas {
    pub fn new(map: HashMap<String, ElementSchema>) -> Self {
        Self(map)
    }

    pub fn schema_for(&self, element_name: &str) -> Option<&ElementSchema> {
        self.0.get(element_name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub key: Option<String>,
    pub message: String,
}

pub fn validate_element(
    element_schemas: &ElementSchemas,
    element: &Element,
) -> Result<(), Vec<ValidationError>> {
    let element_schema = match validate_element_name(element_schemas, &element.name) {
        Ok(schema) => schema,
        Err(error) => return Err(vec![error]),
    };

    let mut errors = Vec::<ValidationError>::new();
    validate_keys_names(element_schema, element, &mut errors);
    validate_values(element_schema, element, &mut errors);

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn validate_element_name<'a>(
    element_schemas: &'a ElementSchemas,
    element_name: &str,
) -> Result<&'a ElementSchema, ValidationError> {
    match element_schemas.schema_for(element_name) {
        Some(schema) => Ok(schema),
        None => Err(ValidationError {
            key: None,
            message: format!("Unknown element name '{}'", element_name),
        }),
    }
}

pub fn validate_keys_names(
    element_schema: &ElementSchema,
    element: &Element,
    errors: &mut Vec<ValidationError>,
) {
    if !element_schema.allow_unknown_keys {
        for key in element.params.keys() {
            if !element_schema.allowed.contains_key(key) {
                errors.push(ValidationError {
                    key: Some(key.clone()),
                    message: "Key not allowed for this element type".into(),
                });
            }
        }
    }

    for (key, key_spec) in &element_schema.allowed {
        if key_spec.required && !element.params.contains_key(key) {
            errors.push(ValidationError {
                key: Some(key.clone()),
                message: "Missing required key".into(),
            });
        }
    }
}

pub fn validate_values(
    element_schema: &ElementSchema,
    element: &Element,
    errors: &mut Vec<ValidationError>,
) {
    for (key, key_spec) in &element_schema.allowed {
        let Some(value) = element.params.get(key) else { continue };

        if !json_value_matches_type(value, key_spec.ty) {
            errors.push(ValidationError {
                key: Some(key.clone()),
                message: format!("Wrong type (expected {:?})", key_spec.ty),
            });
        }
    }
}

pub fn json_value_matches_type(value: &serde_json::Value, expected: ValueType) -> bool {
    match expected {
        ValueType::Str => value.is_string(),
        ValueType::Bool => value.is_boolean(),
        ValueType::Int => value.as_i64().is_some(),
        ValueType::Float => value.is_number(),
    }
}
