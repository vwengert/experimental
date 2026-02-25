use jsonsss::domain::Schemas;
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

const LIST_COUNT: usize = 5;

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

    // Populate list names: "own" for index 0, "list N" for the rest
    let list_names: Vec<SharedString> = (0..LIST_COUNT)
        .map(|i| {
            if i == 0 {
                SharedString::from("own")
            } else {
                SharedString::from(format!("list {i}").as_str())
            }
        })
        .collect();
    app.set_list_names(ModelRc::from(Rc::new(VecModel::from(list_names))));

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
                if new_idx < LIST_COUNT {
                    *active_list_idx_clone.borrow_mut() = new_idx;
                    let new_model = list_models_clone.borrow()[new_idx].clone();
                    if let Some(app) = app_weak.upgrade() {
                        app.set_lines(ModelRc::from(new_model));
                    }
                }
            }
        }
    });

    app.run().unwrap();
}
