use serde::{Deserialize, Serialize};

// ── Serialisable item structures ──────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct ItemSet {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
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
    pub lists: Vec<ItemList>,
}
