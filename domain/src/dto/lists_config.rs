use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::elements::{ElementField, ElementSchema, FieldSpec, Schemas, ValueType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListsConfigDto {
    pub title: String,
    pub description: String,
    pub properties: SchemasDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemasDto {
    pub units: HashMap<String, Vec<String>>,
    pub elements: HashMap<String, ElementSchemaDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSchemaDto {
    #[serde(default)]
    pub allow_init: bool,
    #[serde(default)]
    pub fields: Vec<ElementFieldDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementFieldDto {
    pub name: String,
    pub spec: FieldSpecDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpecDto {
    pub ty: ValueType,
    #[serde(default)]
    pub unit: Option<String>,
}

impl From<SchemasDto> for Schemas {
    fn from(value: SchemasDto) -> Self {
        Self {
            units: value.units,
            elements: value
                .elements
                .into_iter()
                .map(|(name, schema)| (name, schema.into()))
                .collect(),
        }
    }
}

impl From<&Schemas> for SchemasDto {
    fn from(value: &Schemas) -> Self {
        Self {
            units: value.units.clone(),
            elements: value
                .elements
                .iter()
                .map(|(name, schema)| (name.clone(), schema.into()))
                .collect(),
        }
    }
}

impl From<ElementSchemaDto> for ElementSchema {
    fn from(value: ElementSchemaDto) -> Self {
        ElementSchema::new(
            value.allow_init,
            value.fields.into_iter().map(Into::into).collect(),
        )
    }
}

impl From<&ElementSchema> for ElementSchemaDto {
    fn from(value: &ElementSchema) -> Self {
        Self {
            allow_init: value.allow_init,
            fields: value.fields().iter().map(Into::into).collect(),
        }
    }
}

impl From<ElementFieldDto> for ElementField {
    fn from(value: ElementFieldDto) -> Self {
        ElementField::new(value.name, value.spec.into())
    }
}

impl From<&ElementField> for ElementFieldDto {
    fn from(value: &ElementField) -> Self {
        Self {
            name: value.name.clone(),
            spec: (&value.spec).into(),
        }
    }
}

impl From<FieldSpecDto> for FieldSpec {
    fn from(value: FieldSpecDto) -> Self {
        Self {
            ty: value.ty,
            unit: value.unit,
        }
    }
}

impl From<&FieldSpec> for FieldSpecDto {
    fn from(value: &FieldSpec) -> Self {
        Self {
            ty: value.ty,
            unit: value.unit.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_lists_config_dto_to_domain() {
        let dto = ListsConfigDto {
            title: "Lists Config".into(),
            description: "Config description".into(),
            properties: SchemasDto {
                units: HashMap::from([("length".into(), vec!["px".into()])]),
                elements: HashMap::from([(
                    "Button".into(),
                    ElementSchemaDto {
                        allow_init: true,
                        fields: vec![ElementFieldDto {
                            name: "label".into(),
                            spec: FieldSpecDto {
                                ty: ValueType::Str,
                                unit: None,
                            },
                        }],
                    },
                )]),
            },
        };

        assert_eq!(dto.title, "Lists Config");
        let schemas: Schemas = dto.properties.into();
        assert!(schemas.elements.contains_key("Button"));
        assert!(schemas.schema_for("Button").unwrap().contains_field("label"));
    }
}

