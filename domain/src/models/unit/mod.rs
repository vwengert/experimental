pub mod length_unit;

pub use length_unit::LengthUnit;

pub trait UnitConvertible: Copy {
    fn unit_factor(self) -> f64;

    fn convert_between(value: f64, from: Self, to: Self) -> f64 {
        value * from.unit_factor() / to.unit_factor()
    }
}
