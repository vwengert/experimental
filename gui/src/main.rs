use jsonsss::domain::{ElementSchemas, UnitSpec, ValueType};
use jsonsss::io::{self, json_read_string};
use slint::{Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

slint::include_modules!();

fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn make_pair(key: &str, spec: &jsonsss::domain::KeySpec) -> KeyValuePair {
    let (unit, unit_options) = match &spec.units {
        None => (SharedString::new(), ModelRc::from(Rc::new(VecModel::<SharedString>::default()))),
        Some(UnitSpec::Fixed(s)) => (SharedString::from(s.as_str()), ModelRc::from(Rc::new(VecModel::<SharedString>::default()))),
        Some(UnitSpec::List(opts)) => {
            let first = opts.first().map(|s| SharedString::from(s.as_str())).unwrap_or_default();
            let model: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(
                opts.iter().map(|s| SharedString::from(s.as_str())).collect::<Vec<_>>(),
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

fn main() {
    let schemas_str = include_str!("../../cli/src/schemas.json");
    let schemas: ElementSchemas =
        json_read_string(schemas_str).expect("Failed to load schemas.json");

    let app = AppWindow::new().unwrap();

    // Populate schema names (sorted for deterministic ordering)
    let mut schema_names: Vec<SharedString> = schemas
        .0
        .keys()
        .map(|k| SharedString::from(k.as_str()))
        .collect();
    schema_names.sort();
    app.set_schema_names(ModelRc::from(Rc::new(VecModel::from(schema_names))));

    // Set up the lines model
    let lines_model = Rc::new(VecModel::<LineItem>::default());
    app.set_lines(ModelRc::from(lines_model.clone()));

    // Keep references to the inner pair models so we can update them from callbacks
    let pairs_models: Rc<RefCell<Vec<Rc<VecModel<KeyValuePair>>>>> =
        Rc::new(RefCell::new(Vec::new()));

    let schemas_clone = schemas.clone();
    let lines_model_clone = lines_model.clone();
    let pairs_models_clone = pairs_models.clone();

    // Single dispatch callback that handles all actions
    app.on_dispatch(move |action| {
        match action.action_type {
            ActionType::AddLine => {
                let name = action.schema_name.as_str();
                if name.is_empty() {
                    return;
                }
                if let Some(schema) = schemas_clone.schema_for(name) {
                    let mut pairs: Vec<KeyValuePair> = schema
                        .allowed
                        .iter()
                        .map(|(k, spec)| make_pair(k.as_str(), spec))
                        .collect();
                    // Sort keys for consistent display order
                    pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));

                    let pairs_vec = Rc::new(VecModel::from(pairs));
                    let pairs_model_rc = ModelRc::from(pairs_vec.clone());
                    pairs_models_clone.borrow_mut().push(pairs_vec);

                    lines_model_clone.push(LineItem {
                        title: action.schema_name.clone(),
                        pairs: pairs_model_rc,
                    });
                }
            }
            ActionType::ValueChanged => {
                let li = action.line_index as usize;
                let pi = action.pair_index as usize;
                let borrowed = pairs_models_clone.borrow();
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
                let borrowed = pairs_models_clone.borrow();
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
                        .allowed
                        .iter()
                        .map(|(k, spec)| make_pair(k.as_str(), spec))
                        .collect();
                    pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));

                    // Replace the contents of the existing pairs model in-place
                    let borrowed = pairs_models_clone.borrow();
                    if let Some(pairs_model) = borrowed.get(li) {
                        pairs_model.set_vec(pairs);
                    }
                    drop(borrowed);

                    // Update the line title
                    if let Some(mut line) = lines_model_clone.row_data(li) {
                        line.title = action.schema_name;
                        lines_model_clone.set_row_data(li, line);
                    }
                }
            }
            ActionType::Save => {
                let mut saved: Vec<serde_json::Value> = Vec::new();
                let borrowed = pairs_models_clone.borrow();
                for i in 0..lines_model_clone.row_count() {
                    let Some(line) = lines_model_clone.row_data(i) else { continue };
                    let title = line.title.to_string();
                    let mut params: HashMap<String, serde_json::Value> = HashMap::new();
                    let mut units: HashMap<String, String> = HashMap::new();
                    if let Some(schema) = schemas_clone.schema_for(&title) {
                        if let Some(pairs_model) = borrowed.get(i) {
                            for j in 0..pairs_model.row_count() {
                                let Some(pair) = pairs_model.row_data(j) else { continue };
                                let key = pair.key.to_string();
                                let val_str = pair.value.to_string();
                                if !val_str.is_empty() {
                                    if let Some(key_spec) = schema.allowed.get(&key) {
                                        let json_val = match key_spec.ty {
                                            ValueType::Str => serde_json::Value::String(val_str),
                                            ValueType::Int => val_str
                                                .parse::<i64>()
                                                .map(|n| serde_json::json!(n))
                                                .unwrap_or_else(|_| serde_json::Value::String(val_str)),
                                            ValueType::Float => val_str
                                                .parse::<f64>()
                                                .map(|n| serde_json::json!(n))
                                                .unwrap_or_else(|_| serde_json::Value::String(val_str)),
                                            ValueType::Bool => val_str
                                                .parse::<bool>()
                                                .map(|b| serde_json::json!(b))
                                                .unwrap_or_else(|_| serde_json::Value::String(val_str)),
                                        };
                                        params.insert(key.clone(), json_val);
                                    }
                                }
                                let unit_str = pair.unit.to_string();
                                if !unit_str.is_empty() {
                                    units.insert(key, unit_str);
                                }
                            }
                        }
                    }
                    let mut entry = serde_json::json!({
                        "name": title,
                        "params": params,
                    });
                    if !units.is_empty() {
                        entry["units"] = serde_json::json!(units);
                    }
                    saved.push(entry);
                }
                drop(borrowed);
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    let _ = io::save(&path, &saved);
                }
            }
            ActionType::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    if let Ok(data) = io::load::<Vec<serde_json::Value>>(&path) {
                        lines_model_clone.set_vec(vec![]);
                        pairs_models_clone.borrow_mut().clear();
                        for entry in data {
                            let Some(name) = entry["name"].as_str() else { continue };
                            if let Some(schema) = schemas_clone.schema_for(name) {
                                let saved_params = entry["params"].as_object();
                                let saved_units = entry["units"].as_object();
                                let mut pairs: Vec<KeyValuePair> = schema
                                    .allowed
                                    .iter()
                                    .map(|(k, spec)| {
                                        let mut pair = make_pair(k.as_str(), spec);
                                        if let Some(params) = saved_params {
                                            if let Some(val) = params.get(k) {
                                                pair.value = SharedString::from(
                                                    json_value_to_string(val).as_str(),
                                                );
                                            }
                                        }
                                        if let Some(units) = saved_units {
                                            if let Some(u) = units.get(k).and_then(|v| v.as_str()) {
                                                pair.unit = SharedString::from(u);
                                            }
                                        }
                                        pair
                                    })
                                    .collect();
                                pairs.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));
                                let pairs_vec = Rc::new(VecModel::from(pairs));
                                let pairs_model_rc = ModelRc::from(pairs_vec.clone());
                                pairs_models_clone.borrow_mut().push(pairs_vec);
                                lines_model_clone.push(LineItem {
                                    title: SharedString::from(name),
                                    pairs: pairs_model_rc,
                                });
                            }
                        }
                    }
                }
            }
        }
    });

    app.run().unwrap();
}
