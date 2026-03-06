use serde::{Deserialize, Serialize};
use crate::schema::{Schemas, ElementSchema};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct ItemSet {
    pub key: String,
    pub value: String,
    pub unit: String,
}

#[derive(Serialize, Deserialize)]
pub struct ItemLine {
    pub title: String,
    pub sets: Vec<ItemSet>,
}

#[derive(Serialize, Deserialize)]
pub struct ItemList {
    pub name: String,
    pub lines: Vec<ItemLine>,
}

#[derive(Serialize, Deserialize)]
pub struct ItemData {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub units: HashMap<String, Vec<String>>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub elements: HashMap<String, ElementSchema>,

    pub lists: Vec<ItemList>,
}

impl ItemData {
    /// Get the schemas (units and elements) from this data, or use defaults if not present
    pub fn get_schemas(&self) -> Schemas {
        if self.units.is_empty() && self.elements.is_empty() {
            Schemas::load_default()
        } else {
            Schemas {
                units: self.units.clone(),
                elements: self.elements.clone(),
            }
        }
    }
}

