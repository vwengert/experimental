use serde::{Deserialize, Serialize};

use crate::models::elements::{Schemas, ValueType};
pub use crate::models::error::item_line_conversion_error::ItemLineConversionError;
use crate::models::model::ItemLine;
pub use crate::models::unit::length_unit::LengthUnit;
use crate::models::unit::UnitConvertible;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerLine {
    pub width: ValueWithUnit<f64, LengthUnit>,
    pub height: ValueWithUnit<f64, LengthUnit>,
    pub padding: ValueWithUnit<i64, LengthUnit>,
}

impl ContainerLine {
    pub fn try_from_item_line(
        line: &ItemLine,
        schemas: &Schemas,
    ) -> Result<Self, ItemLineConversionError> {
        validate_container_schema(schemas)?;

        if line.title != "Container" {
            return Err(ItemLineConversionError::WrongElementType {
                expected: "Container",
                found: line.title.clone(),
            });
        }

        Ok(Self {
            width: ValueWithUnit {
                value: parse_float_value(line, "width")?,
                unit: parse_length_unit(line, schemas, "width")?,
            },
            height: ValueWithUnit {
                value: parse_float_value(line, "height")?,
                unit: parse_length_unit(line, schemas, "height")?,
            },
            padding: ValueWithUnit {
                value: parse_int_value(line, "padding")?,
                unit: parse_length_unit(line, schemas, "padding")?,
            },
        })
    }
}

fn validate_container_schema(schemas: &Schemas) -> Result<(), ItemLineConversionError> {
    let schema = schemas
        .schema_for("Container")
        .ok_or(ItemLineConversionError::MissingContainerSchema)?;

    validate_field(schema.field("width"), "width", ValueType::Float, "length")?;
    validate_field(schema.field("height"), "height", ValueType::Float, "length")?;
    validate_field(schema.field("padding"), "padding", ValueType::Int, "length")?;

    if !schemas.units.contains_key("length") {
        return Err(ItemLineConversionError::MissingLengthUnitGroup);
    }

    Ok(())
}

fn validate_field(
    field: Option<&crate::models::elements::FieldSpec>,
    field_name: &'static str,
    expected_type: ValueType,
    expected_unit: &'static str,
) -> Result<(), ItemLineConversionError> {
    let field = field.ok_or(ItemLineConversionError::SchemaFieldMissing { field: field_name })?;

    if field.ty != expected_type {
        return Err(ItemLineConversionError::SchemaFieldTypeMismatch {
            field: field_name,
            expected: expected_type,
            found: field.ty,
        });
    }

    if field.unit.as_deref() != Some(expected_unit) {
        return Err(ItemLineConversionError::SchemaFieldUnitMismatch {
            field: field_name,
            expected: expected_unit,
            found: field.unit.clone(),
        });
    }

    Ok(())
}

fn parse_float_value(line: &ItemLine, field: &'static str) -> Result<f64, ItemLineConversionError> {
    let value = find_value(line, field)?;
    value
        .parse::<f64>()
        .map_err(|_| ItemLineConversionError::InvalidFloatValue {
            field,
            value: value.to_string(),
        })
}

fn parse_int_value(line: &ItemLine, field: &'static str) -> Result<i64, ItemLineConversionError> {
    let value = find_value(line, field)?;
    value
        .parse::<i64>()
        .map_err(|_| ItemLineConversionError::InvalidIntValue {
            field,
            value: value.to_string(),
        })
}

fn parse_length_unit(
    line: &ItemLine,
    schemas: &Schemas,
    field: &'static str,
) -> Result<LengthUnit, ItemLineConversionError> {
    let item = line
        .data
        .iter()
        .find(|item| item.key == field)
        .ok_or(ItemLineConversionError::MissingValue { field })?;

    let allowed_units = schemas
        .units
        .get("length")
        .ok_or(ItemLineConversionError::MissingLengthUnitGroup)?;
    if !allowed_units.iter().any(|unit| unit == &item.unit) {
        return Err(ItemLineConversionError::UnitNotAllowedForLength {
            unit: item.unit.clone(),
        });
    }

    LengthUnit::try_from(item.unit.as_str()).map_err(|_| {
        ItemLineConversionError::UnsupportedLengthUnit {
            unit: item.unit.clone(),
        }
    })
}

fn find_value<'a>(
    line: &'a ItemLine,
    field: &'static str,
) -> Result<&'a str, ItemLineConversionError> {
    line.data
        .iter()
        .find(|item| item.key == field)
        .map(|item| item.value.as_str())
        .ok_or(ItemLineConversionError::MissingValue { field })
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
}
