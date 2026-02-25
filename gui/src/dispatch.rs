use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use domain::domain::{ItemData, ItemLine, ItemList, ItemSet};
use domain::schema::{KeySpec, Schemas};

use crate::{Action, ActionType, AppWindow, FileEntry, KeyValuePair, LineItem};

// ─────────────────────────────────────────────────────────────────────────────

/// All shared application state passed to dispatch handlers.
pub struct AppState {
    pub schemas: Schemas,
    pub list_models: Rc<RefCell<Vec<Rc<VecModel<LineItem>>>>>,
    pub all_pairs_models: Rc<RefCell<Vec<Rc<RefCell<Vec<Rc<VecModel<KeyValuePair>>>>>>>>,
    pub active_list_idx: Rc<RefCell<usize>>,
    pub list_names: Rc<VecModel<SharedString>>,
    pub app_weak: slint::Weak<AppWindow>,
}

pub fn make_pair(
    key: &str,
    spec: &KeySpec,
    units: &std::collections::HashMap<String, Vec<String>>,
) -> KeyValuePair {
    let (unit, unit_options) = match &spec.unit {
        None => (SharedString::new(), ModelRc::from(Rc::new(VecModel::<SharedString>::default()))),
        Some(unit_type) => {
            let unit_values: &[String] = units
                .get(unit_type.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let first =
                unit_values.first().map(|s| SharedString::from(s.as_str())).unwrap_or_default();
            let model: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(
                unit_values.iter().map(|s| SharedString::from(s.as_str())).collect::<Vec<_>>(),
            ));
            (first, ModelRc::from(model))
        }
    };
    KeyValuePair { key: SharedString::from(key), value: SharedString::new(), unit, unit_options }
}

// ── Individual action handlers ────────────────────────────────────────────────

pub fn handle_add_line(state: &AppState, action: &Action) {
    let active = *state.active_list_idx.borrow();
    let lines_model = state.list_models.borrow()[active].clone();
    let pairs_models = state.all_pairs_models.borrow()[active].clone();

    let name = action.schema_name.as_str();
    if name.is_empty() {
        return;
    }
    if let Some(schema) = state.schemas.schema_for(name) {
        let mut pairs: Vec<KeyValuePair> = schema
            .0
            .iter()
            .map(|(k, spec)| make_pair(k.as_str(), spec, &state.schemas.units))
            .collect();
        pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));

        let pairs_vec = Rc::new(VecModel::from(pairs));
        let pairs_model_rc = ModelRc::from(pairs_vec.clone());
        pairs_models.borrow_mut().push(pairs_vec);

        lines_model.push(LineItem { title: action.schema_name.clone(), pairs: pairs_model_rc });
    }
}

pub fn handle_value_changed(state: &AppState, action: &Action) {
    let active = *state.active_list_idx.borrow();
    let pairs_models = state.all_pairs_models.borrow()[active].clone();

    let li = action.line_index as usize;
    let pi = action.pair_index as usize;
    let borrowed = pairs_models.borrow();
    if let Some(pairs_model) = borrowed.get(li) {
        if let Some(mut pair) = pairs_model.row_data(pi) {
            pair.value = action.new_value.clone();
            pairs_model.set_row_data(pi, pair);
        }
    }
}

pub fn handle_unit_changed(state: &AppState, action: &Action) {
    let active = *state.active_list_idx.borrow();
    let pairs_models = state.all_pairs_models.borrow()[active].clone();

    let li = action.line_index as usize;
    let pi = action.pair_index as usize;
    let borrowed = pairs_models.borrow();
    if let Some(pairs_model) = borrowed.get(li) {
        if let Some(mut pair) = pairs_model.row_data(pi) {
            pair.unit = action.new_value.clone();
            pairs_model.set_row_data(pi, pair);
        }
    }
}

pub fn handle_line_type_changed(state: &AppState, action: &Action) {
    let active = *state.active_list_idx.borrow();
    let lines_model = state.list_models.borrow()[active].clone();
    let pairs_models = state.all_pairs_models.borrow()[active].clone();

    let li = action.line_index as usize;
    let name = action.schema_name.as_str();
    if let Some(schema) = state.schemas.schema_for(name) {
        let mut pairs: Vec<KeyValuePair> = schema
            .0
            .iter()
            .map(|(k, spec)| make_pair(k.as_str(), spec, &state.schemas.units))
            .collect();
        pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));

        let borrowed = pairs_models.borrow();
        if let Some(pairs_model) = borrowed.get(li) {
            pairs_model.set_vec(pairs);
        }
        drop(borrowed);

        if let Some(mut line) = lines_model.row_data(li) {
            line.title = action.schema_name.clone();
            lines_model.set_row_data(li, line);
        }
    }
}

pub fn handle_remove_line(state: &AppState, action: &Action) {
    let active = *state.active_list_idx.borrow();
    let lines_model = state.list_models.borrow()[active].clone();
    let pairs_models = state.all_pairs_models.borrow()[active].clone();

    let li = action.line_index as usize;
    if li < lines_model.row_count() {
        lines_model.remove(li);
        let mut borrowed = pairs_models.borrow_mut();
        if li < borrowed.len() {
            borrowed.remove(li);
        }
    }
}

pub fn handle_switch_list(state: &AppState, action: &Action) {
    let new_idx = action.line_index as usize;
    let list_count = state.list_models.borrow().len();
    if new_idx < list_count {
        *state.active_list_idx.borrow_mut() = new_idx;
        let new_model = state.list_models.borrow()[new_idx].clone();
        if let Some(app) = state.app_weak.upgrade() {
            app.set_lines(ModelRc::from(new_model));
        }
    }
}

pub fn handle_add_list(state: &AppState) {
    let count = state.list_models.borrow().len();
    state.list_models.borrow_mut().push(Rc::new(VecModel::<LineItem>::default()));
    state.all_pairs_models.borrow_mut().push(Rc::new(RefCell::new(Vec::new())));
    state.list_names.push(SharedString::from(format!("list {count}").as_str()));
    let new_idx = count;
    *state.active_list_idx.borrow_mut() = new_idx;
    if let Some(app) = state.app_weak.upgrade() {
        app.set_active_list_index(new_idx as i32);
        let new_model = state.list_models.borrow()[new_idx].clone();
        app.set_lines(ModelRc::from(new_model));
    }
}

pub fn handle_remove_list(state: &AppState, action: &Action) {
    let idx = action.line_index as usize;
    let count = state.list_models.borrow().len();
    if idx == 0 || idx >= count {
        return;
    }
    state.list_models.borrow_mut().remove(idx);
    state.all_pairs_models.borrow_mut().remove(idx);
    state.list_names.remove(idx);
    let current = *state.active_list_idx.borrow();
    let new_active = if current >= idx && current > 0 { current - 1 } else { current };
    *state.active_list_idx.borrow_mut() = new_active;
    if let Some(app) = state.app_weak.upgrade() {
        app.set_active_list_index(new_active as i32);
        let new_model = state.list_models.borrow()[new_active].clone();
        app.set_lines(ModelRc::from(new_model));
    }
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

pub fn handle_navigate_dir(state: &AppState, action: &Action) {
    let path_str = action.new_value.as_str();
    if path_str.is_empty() {
        return;
    }
    let canonical = match std::path::Path::new(path_str).canonicalize() {
        Ok(p) if p.is_dir() => p,
        _ => return,
    };
    let entries = read_dir_entries(&canonical);
    if let Some(app) = state.app_weak.upgrade() {
        app.set_file_browser_dir(SharedString::from(canonical.to_string_lossy().as_ref()));
        app.set_file_browser_entries(ModelRc::from(Rc::new(VecModel::from(entries))));
    }
}

pub fn handle_save_list(state: &AppState, action: &Action) {
    let path = action.new_value.as_str();
    if path.is_empty() {
        return;
    }
    let list_models_ref = state.list_models.borrow();
    let all_pairs_ref = state.all_pairs_models.borrow();
    let mut item_lists: Vec<ItemList> = Vec::new();
    for li in 0..list_models_ref.len() {
        let name = state
            .list_names
            .row_data(li)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let line_model = &list_models_ref[li];
        let pairs_for_list = all_pairs_ref[li].borrow();
        let mut item_lines: Vec<ItemLine> = Vec::new();
        for (line_idx, pairs_model) in pairs_for_list.iter().enumerate() {
            let title = line_model
                .row_data(line_idx)
                .map(|l| l.title.to_string())
                .unwrap_or_default();
            let item_sets: Vec<ItemSet> = (0..pairs_model.row_count())
                .filter_map(|pi| pairs_model.row_data(pi))
                .map(|p| ItemSet {
                    key: p.key.to_string(),
                    value: p.value.to_string(),
                    unit: if p.unit.is_empty() { None } else { Some(p.unit.to_string()) },
                })
                .collect();
            item_lines.push(ItemLine { title, sets: item_sets });
        }
        item_lists.push(ItemList { name, lines: item_lines });
    }
    let data = ItemData { lists: item_lists };
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write(path, json);
    }

    if let Some(app) = state.app_weak.upgrade() {
        app.set_is_dirty(false);
    }
}

pub fn handle_load_list(state: &AppState, action: &Action) {
    let path = action.new_value.as_str();
    if path.is_empty() {
        return;
    }
    let json = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let item_data: ItemData = match serde_json::from_str(&json) {
        Ok(d) => d,
        Err(_) => return,
    };
    if item_data.lists.is_empty() {
        return;
    }

    let mut new_list_models: Vec<Rc<VecModel<LineItem>>> = Vec::new();
    let mut new_pairs_models: Vec<Rc<RefCell<Vec<Rc<VecModel<KeyValuePair>>>>>> = Vec::new();

    for item_list in &item_data.lists {
        let line_model: Rc<VecModel<LineItem>> = Rc::new(VecModel::<LineItem>::default());
        let pairs_for_list: Rc<RefCell<Vec<Rc<VecModel<KeyValuePair>>>>> =
            Rc::new(RefCell::new(Vec::new()));

        for item_line in &item_list.lines {
            let pairs: Vec<KeyValuePair> = item_line
                .sets
                .iter()
                .map(|p| {
                    let unit_options = if let Some(schema) =
                        state.schemas.schema_for(&item_line.title)
                    {
                        if let Some(key_spec) = schema.0.get(&p.key) {
                            match &key_spec.unit {
                                None => ModelRc::from(Rc::new(
                                    VecModel::<SharedString>::default(),
                                )),
                                Some(unit_type) => {
                                    let unit_values: &[String] = state
                                        .schemas
                                        .units
                                        .get(unit_type.as_str())
                                        .map(|v| v.as_slice())
                                        .unwrap_or(&[]);
                                    ModelRc::from(Rc::new(VecModel::from(
                                        unit_values
                                            .iter()
                                            .map(|s| SharedString::from(s.as_str()))
                                            .collect::<Vec<_>>(),
                                    )))
                                }
                            }
                        } else {
                            ModelRc::from(Rc::new(VecModel::<SharedString>::default()))
                        }
                    } else {
                        ModelRc::from(Rc::new(VecModel::<SharedString>::default()))
                    };
                    KeyValuePair {
                        key: SharedString::from(p.key.as_str()),
                        value: SharedString::from(p.value.as_str()),
                        unit: SharedString::from(p.unit.as_deref().unwrap_or("")),
                        unit_options,
                    }
                })
                .collect();

            let pairs_vec = Rc::new(VecModel::from(pairs));
            pairs_for_list.borrow_mut().push(pairs_vec.clone());
            line_model.push(LineItem {
                title: SharedString::from(item_line.title.as_str()),
                pairs: ModelRc::from(pairs_vec),
            });
        }

        new_list_models.push(line_model);
        new_pairs_models.push(pairs_for_list);
    }

    *state.list_models.borrow_mut() = new_list_models;
    *state.all_pairs_models.borrow_mut() = new_pairs_models;

    while state.list_names.row_count() > 0 {
        state.list_names.remove(0);
    }
    for item_list in &item_data.lists {
        state.list_names.push(SharedString::from(item_list.name.as_str()));
    }

    *state.active_list_idx.borrow_mut() = 0;
    if let Some(app) = state.app_weak.upgrade() {
        app.set_active_list_index(0);
        let first_model = state.list_models.borrow()[0].clone();
        app.set_lines(ModelRc::from(first_model));
        app.set_is_dirty(false);
    }
}

pub fn handle_exit(state: &AppState) {
    if let Some(app) = state.app_weak.upgrade() {
        let _ = app.hide();
    }
}

// ── Main dispatch entry point ─────────────────────────────────────────────────

pub fn handle_dispatch(state: &AppState, action: Action) {
    // Determine dirty-flag impact before consuming action fields
    let marks_dirty = !matches!(
        action.action_type,
        ActionType::SwitchList
            | ActionType::SaveList
            | ActionType::LoadList
            | ActionType::NavigateDir
            | ActionType::Exit
    );

    match action.action_type {
        ActionType::AddLine => handle_add_line(state, &action),
        ActionType::ValueChanged => handle_value_changed(state, &action),
        ActionType::UnitChanged => handle_unit_changed(state, &action),
        ActionType::LineTypeChanged => handle_line_type_changed(state, &action),
        ActionType::RemoveLine => handle_remove_line(state, &action),
        ActionType::SwitchList => handle_switch_list(state, &action),
        ActionType::AddList => handle_add_list(state),
        ActionType::RemoveList => handle_remove_list(state, &action),
        ActionType::SaveList => handle_save_list(state, &action),
        ActionType::LoadList => handle_load_list(state, &action),
        ActionType::NavigateDir => handle_navigate_dir(state, &action),
        ActionType::Exit => handle_exit(state),
    }

    // Update the is-dirty property on the UI
    if marks_dirty {
        if let Some(app) = state.app_weak.upgrade() {
            app.set_is_dirty(true);
        }
    }
}
