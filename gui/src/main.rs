use jsonsss::io::json_read_string;
use jsonsss::domain::ElementSchemas;
use slint::{Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

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
                        .keys()
                        .map(|k| KeyValuePair {
                            key: SharedString::from(k.as_str()),
                            value: SharedString::new(),
                        })
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
            ActionType::LineTypeChanged => {
                let li = action.line_index as usize;
                let name = action.schema_name.as_str();
                if let Some(schema) = schemas_clone.schema_for(name) {
                    let mut pairs: Vec<KeyValuePair> = schema
                        .allowed
                        .keys()
                        .map(|k| KeyValuePair {
                            key: SharedString::from(k.as_str()),
                            value: SharedString::new(),
                        })
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
        }
    });

    app.run().unwrap();
}
