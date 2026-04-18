use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::dto::lists_config::ListsConfigDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    Str,
    Int,
    Float,
    Bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldSpec {
    pub ty: ValueType,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ElementField {
    pub name: String,
    pub spec: FieldSpec,
}

impl ElementField {
    pub fn new(name: impl Into<String>, spec: FieldSpec) -> Self {
        Self { name: name.into(), spec }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementSchema {
    #[serde(default)]
    pub allow_init: bool,
    #[serde(default)]
    fields: Vec<ElementField>,
}

impl ElementSchema {
    pub fn new(allow_init: bool, fields: Vec<ElementField>) -> Self {
        Self { allow_init, fields }
    }

    pub fn field(&self, name: &str) -> Option<&FieldSpec> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.spec)
    }

    pub fn fields(&self) -> &[ElementField] {
        &self.fields
    }

    pub fn iter_fields(&self) -> impl Iterator<Item = (&str, &FieldSpec)> {
        self.fields.iter().map(|field| (field.name.as_str(), &field.spec))
    }

    pub fn contains_field(&self, name: &str) -> bool {
        self.field(name).is_some()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Schemas {
    pub units: HashMap<String, Vec<String>>,
    pub elements: HashMap<String, ElementSchema>,
}

impl Schemas {
    pub fn schema_for(&self, element_name: &str) -> Option<&ElementSchema> {
        self.elements.get(element_name)
    }


    pub fn init_element_names(&self) -> Vec<&str> {
        let names: Vec<&str> = self
            .elements
            .iter()
            .filter(|(_, schema)| schema.allow_init)
            .map(|(name, _)| name.as_str())
            .collect();
        names
    }

    pub fn load_default() -> Self {
        let text = include_str!("../../assets/lists.config.json");
        let config: ListsConfigDto = serde_json::from_str(text)
            .expect("Built-in lists.config.json is invalid JSON");
        config.properties.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_init_field() {
        let schemas = Schemas::load_default();

        // Button and Container should have allow_init = true
        let button = schemas.schema_for("Button").unwrap();
        assert!(button.allow_init, "Button should have allow_init = true");
        assert!(button.contains_field("label"), "Button should have 'label' field");
        assert_eq!(
            button.iter_fields().map(|(name, _)| name).collect::<Vec<_>>(),
            vec!["label", "onClick", "disabled"]
        );

        let container = schemas.schema_for("Container").unwrap();
        assert!(container.allow_init, "Container should have allow_init = true");

        // TextField should not have allow_init
        let text_field = schemas.schema_for("TextField").unwrap();
        assert!(!text_field.allow_init, "TextField should have allow_init = false");

        // init_element_names should return only init elements, sorted
        let init_names = schemas.init_element_names();
        assert!(init_names.contains(&"Button"));
        assert!(init_names.contains(&"Container"));
        assert!(!init_names.contains(&"TextField"));
    }

    #[test]
    fn test_lists_config_deserializes() {
        let text = include_str!("../../assets/lists.config.json");
        let config: ListsConfigDto = serde_json::from_str(text).unwrap();

        assert_eq!(config.title, "Lists Config");
        assert!(config.description.contains("Default configuration"));

        let schemas: Schemas = config.properties.into();
        assert!(schemas.units.contains_key("length"));
        assert!(schemas.elements.contains_key("Button"));
    }

    #[test]
    fn test_element_schema_rejects_legacy_flat_fields() {
        let json = r#"{
            "allow_init": true,
            "height": { "ty": "Float", "unit": "length" },
            "padding": { "ty": "Int", "unit": "length" },
            "width": { "ty": "Float", "unit": "length" }
        }"#;

        assert!(serde_json::from_str::<ElementSchema>(json).is_err());
    }

    #[test]
    fn test_element_schema_reads_field_list_and_serializes_as_list() {
        let json = r#"{
            "fields": [
                { "name": "label", "spec": { "ty": "Str" } },
                { "name": "disabled", "spec": { "ty": "Bool" } }
            ]
        }"#;

        let schema: ElementSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.fields()[0].name, "label");
    }
}


