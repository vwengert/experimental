use unit_enum_derive::ItemLineStruct;

use crate::models::elements::ValueType;
pub use crate::models::error::item_line_conversion_error::ItemLineConversionError;
pub use crate::models::unit::length_unit::LengthUnit;
use crate::models::unit::UnitConvertible;
use crate::utility::parse::{
    parse_float_value, parse_int_value, parse_length_unit, parse_string_value, validate_field,
    validate_field_without_unit,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ValueWithUnit<T, U> {
    pub value: T,
    pub unit: U,
}

impl<U: UnitConvertible> ValueWithUnit<f64, U> {
    pub fn convert_to(self, to: U) -> Self {
        Self {
            value: U::convert_between(self.value, self.unit, to),
            unit: to,
        }
    }
}

impl<U: UnitConvertible> ValueWithUnit<i64, U> {
    pub fn convert_to(self, to: U) -> ValueWithUnit<f64, U> {
        ValueWithUnit {
            value: U::convert_between(self.value as f64, self.unit, to),
            unit: to,
        }
    }
}

#[derive(Debug, Clone, PartialEq, ItemLineStruct)]
#[item_line(element = "Container")]
pub struct ContainerLine {
    #[item_field(ty = "Float", unit = "length")]
    pub width: ValueWithUnit<f64, LengthUnit>,
    #[item_field(ty = "Float", unit = "length")]
    pub height: ValueWithUnit<f64, LengthUnit>,
    #[item_field(ty = "Int", unit = "length")]
    pub padding: ValueWithUnit<i64, LengthUnit>,
}

impl ContainerLine {
    pub fn calculate(&self) -> Result<(), String> {
        eprintln!(
            "[domain-calc][Container] width: {}",
            render_all_length_units(self.width.value, self.width.unit)
        );
        eprintln!(
            "[domain-calc][Container] height: {}",
            render_all_length_units(self.height.value, self.height.unit)
        );
        eprintln!(
            "[domain-calc][Container] padding: {}",
            render_all_length_units(self.padding.value as f64, self.padding.unit)
        );
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, ItemLineStruct)]
#[item_line(element = "Button")]
pub struct ButtonLine {
    #[item_field(ty = "Str")]
    pub label: String,
}

impl ButtonLine {
    pub fn calculate(&self) -> Result<(), String> {
        eprintln!("[domain-calc][Button] label='{}'", self.label);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, ItemLineStruct)]
#[item_line(element = "TextField")]
pub struct TextFieldLine {
    #[item_field(ty = "Str")]
    pub placeholder: String,
    #[item_field(name = "maxLength", ty = "Int")]
    pub max_length: i64,
    #[item_field(ty = "Str")]
    pub value: String,
}

impl TextFieldLine {
    pub fn calculate(&self) -> Result<(), String> {
        eprintln!(
            "[domain-calc][TextField] placeholder='{}' maxLength={} value='{}'",
            self.placeholder, self.max_length, self.value
        );
        Ok(())
    }
}

fn render_all_length_units(value: f64, from: LengthUnit) -> String {
    all_length_units()
        .iter()
        .map(|to| {
            let converted = LengthUnit::convert_value(value, from, *to);
            format!("{converted:.4} {:?}", to)
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn all_length_units() -> [LengthUnit; 4] {
    [
        LengthUnit::Px,
        LengthUnit::Em,
        LengthUnit::Rem,
        LengthUnit::Percent,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::elements::Schemas;
    use crate::models::model::{ItemLine, ItemSet};

    fn valid_container_line() -> ItemLine {
        ItemLine {
            title: "Container".into(),
            data: vec![
                ItemSet {
                    key: "width".into(),
                    value: "100.5".into(),
                    unit: "px".into(),
                },
                ItemSet {
                    key: "height".into(),
                    value: "42.0".into(),
                    unit: "em".into(),
                },
                ItemSet {
                    key: "padding".into(),
                    value: "8".into(),
                    unit: "rem".into(),
                },
            ],
        }
    }

    #[test]
    fn converts_container_line_to_typed_struct() {
        let schemas = Schemas::load_default();
        let line = valid_container_line();

        let container = ContainerLine::try_from_item_line(&line, &schemas).unwrap();

        assert_eq!(container.width.value, 100.5);
        assert_eq!(container.width.unit, LengthUnit::Px);
        assert_eq!(container.height.unit, LengthUnit::Em);
        assert_eq!(container.padding.value, 8);
        assert_eq!(container.padding.unit, LengthUnit::Rem);
    }

    #[test]
    fn rejects_unit_not_in_length_group() {
        let schemas = Schemas::load_default();
        let mut line = valid_container_line();
        line.data[0].unit = "miles".into();

        let error = ContainerLine::try_from_item_line(&line, &schemas).unwrap_err();
        assert_eq!(
            error,
            ItemLineConversionError::UnitNotAllowedForLength {
                unit: "miles".into()
            }
        );
    }

    #[test]
    fn rejects_non_container_line() {
        let schemas = Schemas::load_default();
        let mut line = valid_container_line();
        line.title = "Button".into();

        let error = ContainerLine::try_from_item_line(&line, &schemas).unwrap_err();
        assert_eq!(
            error,
            ItemLineConversionError::WrongElementType {
                expected: "Container",
                found: "Button".into()
            }
        );
    }

    #[test]
    fn parses_percent_unit() {
        assert_eq!(LengthUnit::try_from("%").unwrap(), LengthUnit::Percent);
    }

    #[test]
    fn provides_variant_factors_from_markers() {
        assert_eq!(LengthUnit::Px.factor(), 1.0);
        assert_eq!(LengthUnit::Em.factor(), 3.0);
        assert_eq!(LengthUnit::Rem.factor(), 2.0);
        assert_eq!(LengthUnit::Percent.factor(), 0.5);
    }

    #[test]
    fn converts_values_between_units_using_factors() {
        let value_in_em = LengthUnit::convert_value(9.0, LengthUnit::Px, LengthUnit::Em);
        let value_in_px = LengthUnit::convert_value(3.0, LengthUnit::Em, LengthUnit::Px);

        assert!((value_in_em - 3.0).abs() < f64::EPSILON);
        assert!((value_in_px - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn converts_value_with_unit_via_instance_method() {
        let value_in = ValueWithUnit {
            value: 9.0,
            unit: LengthUnit::Px,
        };

        let converted = value_in.convert_to(LengthUnit::Em);
        assert!((converted.value - 3.0).abs() < f64::EPSILON);
        assert_eq!(converted.unit, LengthUnit::Em);
    }

    #[test]
    fn converts_button_line_to_typed_struct() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "Button".into(),
            data: vec![ItemSet {
                key: "label".into(),
                value: "Submit".into(),
                unit: String::new(),
            }],
        };

        let button = ButtonLine::try_from_item_line(&line, &schemas).unwrap();
        assert_eq!(button.label, "Submit");
    }

    #[test]
    fn converts_text_field_line_to_typed_struct() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "TextField".into(),
            data: vec![
                ItemSet {
                    key: "placeholder".into(),
                    value: "Your name".into(),
                    unit: String::new(),
                },
                ItemSet {
                    key: "maxLength".into(),
                    value: "120".into(),
                    unit: String::new(),
                },
                ItemSet {
                    key: "value".into(),
                    value: "Alice".into(),
                    unit: String::new(),
                },
            ],
        };

        let text_field = TextFieldLine::try_from_item_line(&line, &schemas).unwrap();
        assert_eq!(text_field.placeholder, "Your name");
        assert_eq!(text_field.max_length, 120);
        assert_eq!(text_field.value, "Alice");
    }
}
