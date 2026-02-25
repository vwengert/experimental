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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySpec {
    pub ty: ValueType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ElementSchema(pub HashMap<String, KeySpec>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schemas {
    pub units: HashMap<String, Vec<String>>,
    pub elements: HashMap<String, ElementSchema>,
}

impl Schemas {
    pub fn schema_for(&self, element_name: &str) -> Option<&ElementSchema> {
        self.elements.get(element_name)
    }

    pub fn units_for(&self, unit_type: &str) -> Option<&Vec<String>> {
        self.units.get(unit_type)
    }

    pub fn load_default() -> Self {
        let text = include_str!("./schemas.json");
        serde_json::from_str(text).expect("Built-in schemas.json is invalid")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub key: Option<String>,
    pub message: String,
}

pub fn validate_element(
    schemas: &Schemas,
    element: &Element,
) -> Result<(), Vec<ValidationError>> {
    let element_schema = match validate_element_name(schemas, &element.name) {
        Ok(schema) => schema,
        Err(error) => return Err(vec![error]),
    };

    let mut errors = Vec::<ValidationError>::new();
    validate_keys_names(element_schema, element, &mut errors);
    validate_values(element_schema, element, &mut errors);

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn validate_element_name<'a>(
    schemas: &'a Schemas,
    element_name: &str,
) -> Result<&'a ElementSchema, ValidationError> {
    match schemas.schema_for(element_name) {
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
    for key in element.params.keys() {
        if !element_schema.0.contains_key(key) {
            errors.push(ValidationError {
                key: Some(key.clone()),
                message: "Key not allowed for this element type".into(),
            });
        }
    }

    for key in element_schema.0.keys() {
        if !element.params.contains_key(key) {
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
    for (key, key_spec) in &element_schema.0 {
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

// ── Serialisable save-file structures ─────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct SavedPair {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedLine {
    pub title: String,
    pub pairs: Vec<SavedPair>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedList {
    pub name: String,
    pub lines: Vec<SavedLine>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedData {
    pub lists: Vec<SavedList>,
}
