use jsonsss::domain::Schemas;
use serde::{Deserialize, Serialize};
use slint::{Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn make_pair(key: &str, spec: &jsonsss::domain::KeySpec, units: &std::collections::HashMap<String, Vec<String>>) -> KeyValuePair {
    let (unit, unit_options) = match &spec.unit {
        None => (SharedString::new(), ModelRc::from(Rc::new(VecModel::<SharedString>::default()))),
        Some(unit_type) => {
            let unit_values: &[String] = units
                .get(unit_type.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let first = unit_values.first().map(|s| SharedString::from(s.as_str())).unwrap_or_default();
            let model: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(
                unit_values.iter().map(|s| SharedString::from(s.as_str())).collect::<Vec<_>>(),
            ));
            (first, ModelRc::from(model))
        }
    };
    KeyValuePair {
        key: SharedString::from(key),
        value: SharedString::new(),
        unit,
        unit_options,
    }
}

// ── Serialisable save-file structures ────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct SavedPair {
    key: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SavedLine {
    title: String,
    pairs: Vec<SavedPair>,
}

#[derive(Serialize, Deserialize)]
struct SavedList {
    name: String,
    lines: Vec<SavedLine>,
}

#[derive(Serialize, Deserialize)]
struct SavedData {
    lists: Vec<SavedList>,
}

// ─────────────────────────────────────────────────────────────────────────────

const LIST_COUNT: usize = 1;

fn main() {
    let schemas = Schemas::load_default();

    let app = AppWindow::new().unwrap();

    // Populate schema names (sorted for deterministic ordering)
    let mut schema_names: Vec<SharedString> = schemas
        .elements
        .keys()
        .map(|k| SharedString::from(k.as_str()))
        .collect();
    schema_names.sort();
    app.set_schema_names(ModelRc::from(Rc::new(VecModel::from(schema_names))));

    // Populate list names: start with just "own"
    let list_names_model: Rc<VecModel<SharedString>> =
        Rc::new(VecModel::from(vec![SharedString::from("own")]));
    app.set_list_names(ModelRc::from(list_names_model.clone()));

    // Create one LineItem model per list
    let list_models: Rc<RefCell<Vec<Rc<VecModel<LineItem>>>>> = Rc::new(RefCell::new(
        (0..LIST_COUNT).map(|_| Rc::new(VecModel::<LineItem>::default())).collect(),
    ));

    // Create one pairs-models Vec per list
    let all_pairs_models: Rc<RefCell<Vec<Rc<RefCell<Vec<Rc<VecModel<KeyValuePair>>>>>>>> =
        Rc::new(RefCell::new(
            (0..LIST_COUNT)
                .map(|_| Rc::new(RefCell::new(Vec::<Rc<VecModel<KeyValuePair>>>::new())))
                .collect(),
        ));

    // Track active list index
    let active_list_idx: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    // Bind the first list model to the UI
    app.set_lines(ModelRc::from(list_models.borrow()[0].clone()));

    let schemas_clone = schemas.clone();
    let list_models_clone = list_models.clone();
    let all_pairs_models_clone = all_pairs_models.clone();
    let active_list_idx_clone = active_list_idx.clone();
    let list_names_clone = list_names_model.clone();
    let app_weak = app.as_weak();

    // Single dispatch callback that handles all actions
    app.on_dispatch(move |action| {
        let active = *active_list_idx_clone.borrow();
        let lines_model = list_models_clone.borrow()[active].clone();
        let pairs_models = all_pairs_models_clone.borrow()[active].clone();

        match action.action_type {
            ActionType::AddLine => {
                let name = action.schema_name.as_str();
                if name.is_empty() {
                    return;
                }
                if let Some(schema) = schemas_clone.schema_for(name) {
                    let mut pairs: Vec<KeyValuePair> = schema
                        .0
                        .iter()
                        .map(|(k, spec)| make_pair(k.as_str(), spec, &schemas_clone.units))
                        .collect();
                    // Sort keys for consistent display order
                    pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));

                    let pairs_vec = Rc::new(VecModel::from(pairs));
                    let pairs_model_rc = ModelRc::from(pairs_vec.clone());
                    pairs_models.borrow_mut().push(pairs_vec);

                    lines_model.push(LineItem {
                        title: action.schema_name.clone(),
                        pairs: pairs_model_rc,
                    });
                }
            }
            ActionType::ValueChanged => {
                let li = action.line_index as usize;
                let pi = action.pair_index as usize;
                let borrowed = pairs_models.borrow();
                if let Some(pairs_model) = borrowed.get(li) {
                    if let Some(mut pair) = pairs_model.row_data(pi) {
                        pair.value = action.new_value;
                        pairs_model.set_row_data(pi, pair);
                    }
                }
            }
            ActionType::UnitChanged => {
                let li = action.line_index as usize;
                let pi = action.pair_index as usize;
                let borrowed = pairs_models.borrow();
                if let Some(pairs_model) = borrowed.get(li) {
                    if let Some(mut pair) = pairs_model.row_data(pi) {
                        pair.unit = action.new_value;
                        pairs_model.set_row_data(pi, pair);
                    }
                }
            }
            ActionType::LineTypeChanged => {
                let li = action.line_index as usize;
                let name = action.schema_name.as_str();
                if let Some(schema) = schemas_clone.schema_for(name) {
                    let mut pairs: Vec<KeyValuePair> = schema
                        .0
                        .iter()
                        .map(|(k, spec)| make_pair(k.as_str(), spec, &schemas_clone.units))
                        .collect();
                    pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));

                    // Replace the contents of the existing pairs model in-place
                    let borrowed = pairs_models.borrow();
                    if let Some(pairs_model) = borrowed.get(li) {
                        pairs_model.set_vec(pairs);
                    }
                    drop(borrowed);

                    // Update the line title
                    if let Some(mut line) = lines_model.row_data(li) {
                        line.title = action.schema_name;
                        lines_model.set_row_data(li, line);
                    }
                }
            }
            ActionType::RemoveLine => {
                let li = action.line_index as usize;
                if li < lines_model.row_count() {
                    lines_model.remove(li);
                    let mut borrowed = pairs_models.borrow_mut();
                    if li < borrowed.len() {
                        borrowed.remove(li);
                    }
                }
            }
            ActionType::SwitchList => {
                let new_idx = action.line_index as usize;
                let list_count = list_models_clone.borrow().len();
                if new_idx < list_count {
                    *active_list_idx_clone.borrow_mut() = new_idx;
                    let new_model = list_models_clone.borrow()[new_idx].clone();
                    if let Some(app) = app_weak.upgrade() {
                        app.set_lines(ModelRc::from(new_model));
                    }
                }
            }
            ActionType::AddList => {
                let count = list_models_clone.borrow().len();
                list_models_clone.borrow_mut().push(Rc::new(VecModel::<LineItem>::default()));
                all_pairs_models_clone
                    .borrow_mut()
                    .push(Rc::new(RefCell::new(Vec::new())));
                list_names_clone.push(SharedString::from(format!("list {count}").as_str()));
                let new_idx = count;
                *active_list_idx_clone.borrow_mut() = new_idx;
                if let Some(app) = app_weak.upgrade() {
                    app.set_active_list_index(new_idx as i32);
                    let new_model = list_models_clone.borrow()[new_idx].clone();
                    app.set_lines(ModelRc::from(new_model));
                }
            }
            ActionType::RemoveList => {
                let idx = action.line_index as usize;
                let count = list_models_clone.borrow().len();
                if idx == 0 || idx >= count {
                    return;
                }
                list_models_clone.borrow_mut().remove(idx);
                all_pairs_models_clone.borrow_mut().remove(idx);
                list_names_clone.remove(idx);
                let current = *active_list_idx_clone.borrow();
                let new_active = if current >= idx && current > 0 { current - 1 } else { current };
                *active_list_idx_clone.borrow_mut() = new_active;
                if let Some(app) = app_weak.upgrade() {
                    app.set_active_list_index(new_active as i32);
                    let new_model = list_models_clone.borrow()[new_active].clone();
                    app.set_lines(ModelRc::from(new_model));
                }
            }
            ActionType::SaveList => {
                let path = action.new_value.as_str();
                if path.is_empty() {
                    return;
                }
                let list_models_ref = list_models_clone.borrow();
                let all_pairs_ref = all_pairs_models_clone.borrow();
                let mut saved_lists: Vec<SavedList> = Vec::new();
                for li in 0..list_models_ref.len() {
                    let name = list_names_clone
                        .row_data(li)
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let line_model = &list_models_ref[li];
                    let pairs_for_list = all_pairs_ref[li].borrow();
                    let mut saved_lines: Vec<SavedLine> = Vec::new();
                    for (line_idx, pairs_model) in pairs_for_list.iter().enumerate() {
                        let title = line_model
                            .row_data(line_idx)
                            .map(|l| l.title.to_string())
                            .unwrap_or_default();
                        let saved_pairs: Vec<SavedPair> = (0..pairs_model.row_count())
                            .filter_map(|pi| pairs_model.row_data(pi))
                            .map(|p| SavedPair {
                                key: p.key.to_string(),
                                value: p.value.to_string(),
                                unit: if p.unit.is_empty() { None } else { Some(p.unit.to_string()) },
                            })
                            .collect();
                        saved_lines.push(SavedLine { title, pairs: saved_pairs });
                    }
                    saved_lists.push(SavedList { name, lines: saved_lines });
                }
                let data = SavedData { lists: saved_lists };
                if let Ok(json) = serde_json::to_string_pretty(&data) {
                    let _ = std::fs::write(path, json);
                }
            }
            ActionType::LoadList => {
                let path = action.new_value.as_str();
                if path.is_empty() {
                    return;
                }
                let json = match std::fs::read_to_string(path) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let saved_data: SavedData = match serde_json::from_str(&json) {
                    Ok(d) => d,
                    Err(_) => return,
                };
                if saved_data.lists.is_empty() {
                    return;
                }

                // Rebuild models from loaded data
                let mut new_list_models: Vec<Rc<VecModel<LineItem>>> = Vec::new();
                let mut new_pairs_models: Vec<Rc<RefCell<Vec<Rc<VecModel<KeyValuePair>>>>>> =
                    Vec::new();

                for saved_list in &saved_data.lists {
                    let line_model: Rc<VecModel<LineItem>> =
                        Rc::new(VecModel::<LineItem>::default());
                    let pairs_for_list: Rc<RefCell<Vec<Rc<VecModel<KeyValuePair>>>>> =
                        Rc::new(RefCell::new(Vec::new()));

                    for saved_line in &saved_list.lines {
                        let pairs: Vec<KeyValuePair> = saved_line
                            .pairs
                            .iter()
                            .map(|p| {
                                // Reconstruct unit_options from schema
                                let unit_options =
                                    if let Some(schema) =
                                        schemas_clone.schema_for(&saved_line.title)
                                    {
                                        if let Some(key_spec) = schema.0.get(&p.key) {
                                            match &key_spec.unit {
                                                None => ModelRc::from(Rc::new(
                                                    VecModel::<SharedString>::default(),
                                                )),
                                                Some(unit_type) => {
                                                    let unit_values: &[String] = schemas_clone
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
                                            ModelRc::from(Rc::new(
                                                VecModel::<SharedString>::default(),
                                            ))
                                        }
                                    } else {
                                        ModelRc::from(Rc::new(
                                            VecModel::<SharedString>::default(),
                                        ))
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
                            title: SharedString::from(saved_line.title.as_str()),
                            pairs: ModelRc::from(pairs_vec),
                        });
                    }

                    new_list_models.push(line_model);
                    new_pairs_models.push(pairs_for_list);
                }

                // Replace all existing state
                *list_models_clone.borrow_mut() = new_list_models;
                *all_pairs_models_clone.borrow_mut() = new_pairs_models;

                // Replace list names
                while list_names_clone.row_count() > 0 {
                    list_names_clone.remove(0);
                }
                for saved_list in &saved_data.lists {
                    list_names_clone.push(SharedString::from(saved_list.name.as_str()));
                }

                // Switch to first list
                *active_list_idx_clone.borrow_mut() = 0;
                if let Some(app) = app_weak.upgrade() {
                    app.set_active_list_index(0);
                    let first_model = list_models_clone.borrow()[0].clone();
                    app.set_lines(ModelRc::from(first_model));
                }
            }
        }
    });

    app.run().unwrap();
}
