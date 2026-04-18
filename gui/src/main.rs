use domain::models::elements::Schemas;
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

mod app_state;
mod util;
use app_state::AppState;
use util::read_dir_entries;

slint::include_modules!();

const LIST_COUNT: usize = 1;

fn main() {
    let schemas = Schemas::load_default();

    let app = AppWindow::new().unwrap();
    app.set_file_browser_file_name("lists.json".into());

    let cwd = std::env::current_dir().unwrap_or_default();
    app.set_file_browser_dir(SharedString::from(cwd.to_string_lossy().as_ref()));
    let initial_entries = read_dir_entries(&cwd);
    app.set_file_browser_entries(ModelRc::from(Rc::new(VecModel::from(initial_entries))));

    let mut schema_names: Vec<SharedString> = schemas
        .elements
        .keys()
        .map(|k| SharedString::from(k.as_str()))
        .collect();
    schema_names.sort();
    app.set_schema_names(ModelRc::from(Rc::new(VecModel::from(schema_names))));

    let init_schema_names: Vec<SharedString> = schemas
        .init_element_names()
        .iter()
        .map(|k| SharedString::from(*k))
        .collect();
    app.set_init_schema_names(ModelRc::from(Rc::new(VecModel::from(init_schema_names))));

    let list_names_model: Rc<VecModel<SharedString>> =
        Rc::new(VecModel::from(vec![SharedString::from("own")]));
    app.set_list_names(ModelRc::from(list_names_model.clone()));

    let list_models: Rc<RefCell<Vec<Rc<VecModel<LineItem>>>>> = Rc::new(RefCell::new(
        (0..LIST_COUNT).map(|_| Rc::new(VecModel::<LineItem>::default())).collect(),
    ));

    let all_key_data_models: Rc<RefCell<Vec<Rc<RefCell<Vec<Rc<VecModel<KeyData>>>>>>>> =
        Rc::new(RefCell::new(
            (0..LIST_COUNT)
                .map(|_| Rc::new(RefCell::new(Vec::<Rc<VecModel<KeyData>>>::new())))
                .collect(),
        ));

    let active_list_idx: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    app.set_lines(ModelRc::from(list_models.borrow()[0].clone()));

    let state = Rc::new(AppState {
        schemas,
        list_models,
        all_key_data_models,
        active_list_idx,
        list_names: list_names_model,
        app_weak: app.as_weak(),
    });

    app.on_dispatch(move |action| {
        state.handle_dispatch(action);
    });

    app.on_openDataView(|| {
        let dialog = DataView::new().unwrap();
        dialog.show().unwrap();
        let dialog_weak = dialog.as_weak();
        dialog.on_closeDataView(move || {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.hide().unwrap();
            }
        })
    });

    app.run().unwrap();
}

