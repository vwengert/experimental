use crate::models::elements::{FieldSpec, Schemas, ValueType};
use crate::models::error::item_line_conversion_error::ItemLineConversionError;
use crate::models::model::ItemLine;
use crate::models::unit::length_unit::LengthUnit;

pub fn validate_field(
    field: Option<&FieldSpec>,
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

pub fn parse_float_value(
    line: &ItemLine,
    field: &'static str,
) -> Result<f64, ItemLineConversionError> {
    let value = find_value(line, field)?;
    value
        .parse::<f64>()
        .map_err(|_| ItemLineConversionError::InvalidFloatValue {
            field,
            value: value.to_string(),
        })
}

pub fn parse_int_value(
    line: &ItemLine,
    field: &'static str,
) -> Result<i64, ItemLineConversionError> {
    let value = find_value(line, field)?;
    value
        .parse::<i64>()
        .map_err(|_| ItemLineConversionError::InvalidIntValue {
            field,
            value: value.to_string(),
        })
}

pub fn parse_length_unit(
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

pub fn find_value<'a>(
    line: &'a ItemLine,
    field: &'static str,
) -> Result<&'a str, ItemLineConversionError> {
    line.data
        .iter()
        .find(|item| item.key == field)
        .map(|item| item.value.as_str())
        .ok_or(ItemLineConversionError::MissingValue { field })
}
