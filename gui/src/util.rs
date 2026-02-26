use std::collections::HashMap;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use domain::schema::{ElementSchema, KeySpec};

use crate::{FileEntry, KeyValuePair};
use crate::dispatch::AppState;

pub fn set_lines_model(state: &AppState, idx: usize) {
    *state.active_list_idx.borrow_mut() = idx;
    if let Some(app) = state.app_weak.upgrade() {
        app.set_active_list_index(idx as i32);
        let model = state.list_models.borrow()[idx].clone();
        app.set_lines(ModelRc::from(model));
    }
}
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

pub fn make_pair(key: &str, spec: &KeySpec, units: &HashMap<String, Vec<String>>) -> KeyValuePair {
    let unit_options = build_unit_options(spec, units);
    let unit = match &spec.unit {
        None => SharedString::new(),
        Some(unit_type) => units
            .get(unit_type.as_str())
            .and_then(|v| v.first())
            .map(|s| SharedString::from(s.as_str()))
            .unwrap_or_default(),
    };
    KeyValuePair { key: SharedString::from(key), value: SharedString::new(), unit, unit_options, is_valid: false }
}

pub fn build_pairs_for_schema(
    schema: &ElementSchema,
    units: &HashMap<String, Vec<String>>,
) -> Vec<KeyValuePair> {
    let mut pairs: Vec<KeyValuePair> =
        schema.fields.iter().map(|(k, spec)| make_pair(k.as_str(), spec, units)).collect();
    pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));
    pairs
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
