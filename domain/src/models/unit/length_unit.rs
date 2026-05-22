use serde::{Deserialize, Serialize};
use unit_enum_derive::UnitEnum;

use crate::models::unit::UnitConvertible;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, UnitEnum)]
pub enum LengthUnit {
    #[unit(factor = 1.0)]
    #[serde(rename = "px")]
    Px,
    #[unit(factor = 3.0)]
    #[serde(rename = "em")]
    Em,
    #[unit(factor = 2.0)]
    #[serde(rename = "rem")]
    Rem,
    #[unit(factor = 0.5)]
    #[serde(rename = "%")]
    Percent,
}
