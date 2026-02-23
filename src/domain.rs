use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub params: HashMap<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Str,
    Int,
    Float,
    Bool,
}

#[derive(Debug, Clone)]
pub struct KeySpec {
    pub ty: ValueType,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct ElementSchema {
    pub allowed: HashMap<String, KeySpec>,
    pub allow_unknown_keys: bool,
}

#[derive(Debug, Clone)]
pub struct ElementSchemas {
    by_element_name: HashMap<String, ElementSchema>,
}

impl ElementSchemas {
    pub fn new(by_element_name: HashMap<String, ElementSchema>) -> Self {
        Self { by_element_name }
    }

    pub fn schema_for(&self, element_name: &str) -> Option<&ElementSchema> {
        self.by_element_name.get(element_name)
    }
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub key: Option<String>,
    pub message: String,
}

/// High-level validator that composes the individual steps.
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

/// Step 1: validate that the element name exists and return the schema if it does.
/// Since a missing schema makes further validation meaningless, this returns a single error.
pub fn validate_element_name<'a>(
    element_schemas: &'a ElementSchemas,
    element_name: &str,
) -> Result<&'a ElementSchema, ValidationError> {
    match element_schemas.schema_for(element_name) {
        Some(schema) => Ok(schema),
        None => Err(ValidationError {
            key: None,
            message: format!("Unknown element name '{}', element_name),
        }),
    }
}

/// Step 2: validate that all provided keys are allowed (if unknown keys are disallowed),
/// and that all required keys are present.
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

/// Step 3: validate types of values for keys that are present in both params and schema.
pub fn validate_values(
    element_schema: &ElementSchema,
    element: &Element,
    errors: &mut Vec<ValidationError>,
) {
    for (key, key_spec) in &element_schema.allowed {
        let Some(value) = element.params.get(key) else { continue };

        let type_matches = match (key_spec.ty, value) {
            (ValueType::Str, Value::Str(_)) => true,
            (ValueType::Int, Value::Int(_)) => true,
            (ValueType::Float, Value::Float(_)) => true,
            (ValueType::Bool, Value::Bool(_)) => true,
            _ => false,
        };

        if !type_matches {
            errors.push(ValidationError {
                key: Some(key.clone()),
                message: format!("Wrong type (expected {:?})", key_spec.ty),
            });
        }
    }
}