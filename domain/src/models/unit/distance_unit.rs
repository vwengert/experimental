use unit_enum_derive::UnitEnum;

use crate::models::unit::UnitConvertible;

#[derive(Debug, Clone, Copy, PartialEq, Eq, UnitEnum)]
pub enum DistanceUnit {
    #[unit(rename = "m", factor = 1.0)]
    Meter,
    #[unit(rename = "km", factor = 1000.0)]
    Kilometer,
    #[unit(rename = "miles", factor = 1609.34)]
    Miles,
}

#[cfg(test)]
mod tests {
    use super::DistanceUnit;

    #[test]
    fn parses_distance_units() {
        assert_eq!(DistanceUnit::try_from("m").unwrap(), DistanceUnit::Meter);
        assert_eq!(
            DistanceUnit::try_from("km").unwrap(),
            DistanceUnit::Kilometer
        );
        assert_eq!(
            DistanceUnit::try_from("miles").unwrap(),
            DistanceUnit::Miles
        );
    }

    #[test]
    fn converts_distance_units() {
        let km = DistanceUnit::convert_value(1000.0, DistanceUnit::Meter, DistanceUnit::Kilometer);
        let miles = DistanceUnit::convert_value(1609.34, DistanceUnit::Meter, DistanceUnit::Miles);

        assert!((km - 1.0).abs() < f64::EPSILON);
        assert!((miles - 1.0).abs() < 1e-10);
    }
}
