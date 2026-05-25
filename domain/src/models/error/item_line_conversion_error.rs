use thiserror::Error;

use crate::models::elements::ValueType;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ItemLineConversionError {
    #[error("Container schema was not found in lists config")]
    MissingContainerSchema,
    #[error("Length unit group was not found in lists config")]
    MissingLengthUnitGroup,
    #[error("Distance unit group was not found in lists config")]
    MissingDistanceUnitGroup,
    #[error("Schema field '{field}' is missing for Container")]
    SchemaFieldMissing { field: &'static str },
    #[error("Schema field '{field}' type mismatch. expected={expected:?}, found={found:?}")]
    SchemaFieldTypeMismatch {
        field: &'static str,
        expected: ValueType,
        found: ValueType,
    },
    #[error("Schema field '{field}' unit mismatch. expected={expected}, found={found:?}")]
    SchemaFieldUnitMismatch {
        field: &'static str,
        expected: &'static str,
        found: Option<String>,
    },
    #[error("ItemLine element mismatch. expected='{expected}', found='{found}'")]
    WrongElementType {
        expected: &'static str,
        found: String,
    },
    #[error("Missing value for field '{field}'")]
    MissingValue { field: &'static str },
    #[error("Field '{field}' must be a float, got '{value}'")]
    InvalidFloatValue { field: &'static str, value: String },
    #[error("Field '{field}' must be an int, got '{value}'")]
    InvalidIntValue { field: &'static str, value: String },
    #[error("Unsupported length unit '{unit}'")]
    UnsupportedLengthUnit { unit: String },
    #[error("Unsupported distance unit '{unit}'")]
    UnsupportedDistanceUnit { unit: String },
    #[error("Unit '{unit}' is not allowed by 'length' config group")]
    UnitNotAllowedForLength { unit: String },
    #[error("Unit '{unit}' is not allowed by 'distance' config group")]
    UnitNotAllowedForDistance { unit: String },
}
