use std::collections::HashMap;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use domain::schema::{ElementSchema, KeySpec};

use crate::{FileEntry, KeyData};

pub fn build_unit_options(
    spec: &KeySpec,
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

pub fn make_key_data(key: &str, spec: &KeySpec, units: &HashMap<String, Vec<String>>) -> KeyData {
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
    let mut key_data: Vec<KeyData> =
        schema.fields.iter().map(|(k, spec)| make_key_data(k.as_str(), spec, units)).collect();
    key_data.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));
    key_data
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
