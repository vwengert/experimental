use domain::schema::Schemas;
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

mod dispatch;
mod util;
use dispatch::{handle_dispatch, AppState};
use util::read_dir_entries;

slint::include_modules!();

const LIST_COUNT: usize = 1;

fn main() {
    let schemas = Schemas::load_default();

    let app = AppWindow::new().unwrap();
    app.set_file_browser_file_name("lists.json".into());

    // Initialise the file browser to the current working directory
    let cwd = std::env::current_dir().unwrap_or_default();
    app.set_file_browser_dir(SharedString::from(cwd.to_string_lossy().as_ref()));
    let initial_entries = read_dir_entries(&cwd);
    app.set_file_browser_entries(ModelRc::from(Rc::new(VecModel::from(initial_entries))));

    // Populate schema names (sorted for deterministic ordering)
    let mut schema_names: Vec<SharedString> = schemas
        .elements
        .keys()
        .map(|k| SharedString::from(k.as_str()))
        .collect();
    schema_names.sort();
    app.set_schema_names(ModelRc::from(Rc::new(VecModel::from(schema_names))));

    // Populate init schema names (only elements allowed as first/init entry)
    let init_schema_names: Vec<SharedString> = schemas
        .init_element_names()
        .iter()
        .map(|k| SharedString::from(*k))
        .collect();
    app.set_init_schema_names(ModelRc::from(Rc::new(VecModel::from(init_schema_names))));

    // Populate list names: start with just "own"
    let list_names_model: Rc<VecModel<SharedString>> =
        Rc::new(VecModel::from(vec![SharedString::from("own")]));
    app.set_list_names(ModelRc::from(list_names_model.clone()));

    // Create one LineItem model per list
    let list_models: Rc<RefCell<Vec<Rc<VecModel<LineItem>>>>> = Rc::new(RefCell::new(
        (0..LIST_COUNT).map(|_| Rc::new(VecModel::<LineItem>::default())).collect(),
    ));

    // Create one KeyData-models Vec per list
    let all_key_data_models: Rc<RefCell<Vec<Rc<RefCell<Vec<Rc<VecModel<KeyData>>>>>>>> =
        Rc::new(RefCell::new(
            (0..LIST_COUNT)
                .map(|_| Rc::new(RefCell::new(Vec::<Rc<VecModel<KeyData>>>::new())))
                .collect(),
        ));

    // Track active list index
    let active_list_idx: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    // Bind the first list model to the UI
    app.set_lines(ModelRc::from(list_models.borrow()[0].clone()));

    let state = Rc::new(AppState {
        schemas,
        list_models,
        all_key_data_models,
        active_list_idx,
        list_names: list_names_model,
        app_weak: app.as_weak(),
    });

    // Single dispatch callback that handles all actions
    app.on_dispatch(move |action| {
        handle_dispatch(&state, action);
    });

    app.run().unwrap();
}

