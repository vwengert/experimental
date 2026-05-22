use unit_enum_derive::UnitEnum;

use crate::models::unit::UnitConvertible;

#[derive(Debug, Clone, Copy, PartialEq, Eq, UnitEnum)]
pub enum LengthUnit {
    #[unit(rename = "px", factor = 1.0)]
    Px,
    #[unit(rename = "em", factor = 3.0)]
    Em,
    #[unit(rename = "rem", factor = 2.0)]
    Rem,
    #[unit(rename = "%", factor = 0.5)]
    Percent,
}
