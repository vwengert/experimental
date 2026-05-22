use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemSet {
    pub key: String,
    pub value: String,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemLine {
    pub title: String,
    pub data: Vec<ItemSet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemList {
    pub name: String,
    pub lines: Vec<ItemLine>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemData {
    pub lists: Vec<ItemList>,
}
