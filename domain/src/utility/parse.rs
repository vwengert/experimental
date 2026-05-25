use crate::models::elements::{FieldSpec, Schemas, ValueType};
use crate::models::error::item_line_conversion_error::ItemLineConversionError;
use crate::models::model::ItemLine;
use crate::models::unit::distance_unit::DistanceUnit;
use crate::models::unit::length_unit::LengthUnit;

pub(crate) trait ParseFieldValue: Sized {
    fn parse_field(field: &'static str, raw: &str) -> Result<Self, ItemLineConversionError>;
}

impl ParseFieldValue for f64 {
    fn parse_field(field: &'static str, raw: &str) -> Result<Self, ItemLineConversionError> {
        raw.parse::<f64>()
            .map_err(|_| ItemLineConversionError::InvalidFloatValue {
                field,
                value: raw.to_string(),
            })
    }
}

impl ParseFieldValue for i64 {
    fn parse_field(field: &'static str, raw: &str) -> Result<Self, ItemLineConversionError> {
        raw.parse::<i64>()
            .map_err(|_| ItemLineConversionError::InvalidIntValue {
                field,
                value: raw.to_string(),
            })
    }
}

impl ParseFieldValue for String {
    fn parse_field(_field: &'static str, raw: &str) -> Result<Self, ItemLineConversionError> {
        Ok(raw.to_string())
    }
}

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

pub fn validate_field_without_unit(
    field: Option<&FieldSpec>,
    field_name: &'static str,
    expected_type: ValueType,
) -> Result<(), ItemLineConversionError> {
    let field = field.ok_or(ItemLineConversionError::SchemaFieldMissing { field: field_name })?;

    if field.ty != expected_type {
        return Err(ItemLineConversionError::SchemaFieldTypeMismatch {
            field: field_name,
            expected: expected_type,
            found: field.ty,
        });
    }

    if field.unit.is_some() {
        return Err(ItemLineConversionError::SchemaFieldUnitMismatch {
            field: field_name,
            expected: "none",
            found: field.unit.clone(),
        });
    }

    Ok(())
}

pub(crate) fn parse_value<T: ParseFieldValue>(
    line: &ItemLine,
    field: &'static str,
) -> Result<T, ItemLineConversionError> {
    let value = find_value(line, field)?;
    T::parse_field(field, value)
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

pub fn parse_distance_unit(
    line: &ItemLine,
    schemas: &Schemas,
    field: &'static str,
) -> Result<DistanceUnit, ItemLineConversionError> {
    let item = line
        .data
        .iter()
        .find(|item| item.key == field)
        .ok_or(ItemLineConversionError::MissingValue { field })?;

    let allowed_units = schemas
        .units
        .get("distance")
        .ok_or(ItemLineConversionError::MissingDistanceUnitGroup)?;
    if !allowed_units.iter().any(|unit| unit == &item.unit) {
        return Err(ItemLineConversionError::UnitNotAllowedForDistance {
            unit: item.unit.clone(),
        });
    }

    DistanceUnit::try_from(item.unit.as_str()).map_err(|_| {
        ItemLineConversionError::UnsupportedDistanceUnit {
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
