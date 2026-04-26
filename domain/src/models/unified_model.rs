use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedModel {
    pub own: UnifiedObject,
    pub objects: Vec<UnifiedObject>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnifiedObject {
    pub name: String,
    pub positions: Vec<Position>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub timestamp: u32,
}
