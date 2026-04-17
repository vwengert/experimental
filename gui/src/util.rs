use std::collections::HashMap;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use domain::schema::{ElementSchema, FieldSpec, ValueType};

use crate::{FileEntry, KeyData};

pub fn build_unit_options(
    spec: &FieldSpec,
    units: &HashMap<String, Vec<String>>,
) -> ModelRc<SharedString> {
    match &spec.unit {
        None => ModelRc::from(Rc::new(VecModel::<SharedString>::default())),
        Some(unit_type) => {
            let unit_values: &[String] =
                units.get(unit_type.as_str()).map(|v| v.as_slice()).unwrap_or(&[]);
            ModelRc::from(Rc::new(VecModel::from(
                unit_values.iter().map(|s| SharedString::from(s.as_str())).collect::<Vec<_>>(),
            )))
        }
    }
}

pub fn make_key_data(key: &str, spec: &FieldSpec, units: &HashMap<String, Vec<String>>) -> KeyData {
    let unit_options = build_unit_options(spec, units);
    let unit = match &spec.unit {
        None => SharedString::new(),
        Some(unit_type) => units
            .get(unit_type.as_str())
            .and_then(|v| v.first())
            .map(|s| SharedString::from(s.as_str()))
            .unwrap_or_default(),
    };
    KeyData { key: SharedString::from(key), value: SharedString::new(), unit, unit_options, is_valid: false }
}

pub fn build_key_data_for_schema(
    schema: &ElementSchema,
    units: &HashMap<String, Vec<String>>,
) -> Vec<KeyData> {
    schema
        .iter_fields()
        .map(|(name, spec)| make_key_data(name, spec, units))
        .collect()
}

pub fn read_dir_entries(path: &std::path::Path) -> Vec<FileEntry> {
    let mut entries: Vec<FileEntry> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(FileEntry { name: SharedString::from(name.as_str()), is_dir });
        }
        // Directories first, then files, both sorted alphabetically
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.as_str().cmp(b.name.as_str())));
    }
    entries
}

/// Returns true if the string `value` is non-empty and matches the expected `ty`.
pub fn validate_value_str(value: &str, ty: ValueType) -> bool {
    if value.is_empty() {
        return false;
    }
    match ty {
        ValueType::Str => true,
        ValueType::Int => value.parse::<i64>().is_ok(),
        ValueType::Float => value.parse::<f64>().is_ok(),
        ValueType::Bool => matches!(value.to_lowercase().as_str(), "true" | "false"),
    }
}
