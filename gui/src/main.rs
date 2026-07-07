use domain::models::elements::Schemas;
use domain::utility::calculation::LineCalculationResult;
use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

mod app_state;
mod components;
mod util;
use app_state::{
    AllKeyDataModels, AppState, CalculationResults, CalculationRevisions, GraphAxisModel,
    GraphPathModel, GraphPointModel, KeyDataModel, KeyDataModelsForList, LineModel, ListModels,
};
use util::read_dir_entries;

use crate::components::{HoldAppWeak, Notifier};

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

    let list_models: ListModels = Rc::new(RefCell::new(
        (0..LIST_COUNT)
            .map(|_| Rc::new(VecModel::<LineItem>::default()) as LineModel)
            .collect(),
    ));

    let all_key_data_models: AllKeyDataModels = Rc::new(RefCell::new(
        (0..LIST_COUNT)
            .map(|_| Rc::new(RefCell::new(Vec::<KeyDataModel>::new())) as KeyDataModelsForList)
            .collect(),
    ));
    let graph_points: GraphPointModel = Rc::new(VecModel::default());
    let graph_paths: GraphPathModel = Rc::new(VecModel::default());
    let graph_axes: GraphAxisModel = Rc::new(VecModel::default());
    let calc_results: CalculationResults =
        Rc::new(RefCell::new((0..LIST_COUNT).map(|_| Vec::new()).collect()));
    let calc_revisions: CalculationRevisions =
        Rc::new(RefCell::new((0..LIST_COUNT).map(|_| Vec::new()).collect()));

    let active_list_idx: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let (calc_result_sender, calc_result_receiver) =
        std::sync::mpsc::channel::<Result<LineCalculationResult, String>>();
    let calc_sender =
        domain::utility::calculation::spawn_line_calculation_worker(calc_result_sender);

    app.set_lines(ModelRc::from(list_models.borrow()[0].clone()));

    let state = Rc::new(AppState {
        schemas,
        list_models,
        all_key_data_models,
        active_list_idx,
        list_names: list_names_model,
        calc_sender,
        calc_result_receiver: RefCell::new(calc_result_receiver),
        graph_points: graph_points.clone(),
        graph_paths: graph_paths.clone(),
        graph_axes: graph_axes.clone(),
        calc_results,
        calc_revisions,
        graph_yaw_degrees: RefCell::new(32.0),
        graph_pitch_degrees: RefCell::new(22.0),
        app_weak: app.as_weak(),
    });

    let poll_state = state.clone();
    let calc_result_timer = Timer::default();
    let weak = HoldAppWeak::new(app.as_weak());
    calc_result_timer.start(
        TimerMode::Repeated,
        Duration::from_millis(10000),
        move || {
            poll_state.poll_calculation_results();
            weak.notify();
        },
    );

    let dispatch_state = state.clone();
    app.on_dispatch(move |action| {
        dispatch_state.handle_dispatch(action);
    });

    state.set_graph_rotation(state.graph_yaw_degrees(), state.graph_pitch_degrees());

    let graph_window = GraphWindow::new().unwrap();
    graph_window.set_graph_points(ModelRc::from(graph_points.clone()));
    graph_window.set_graph_paths(ModelRc::from(graph_paths.clone()));
    graph_window.set_graph_axes(ModelRc::from(graph_axes.clone()));
    graph_window.set_yaw_degrees(state.graph_yaw_degrees());
    graph_window.set_pitch_degrees(state.graph_pitch_degrees());

    let graph_state = state.clone();
    graph_window.on_rotationChanged(move |yaw_degrees, pitch_degrees| {
        graph_state.set_graph_rotation(yaw_degrees, pitch_degrees);
    });

    let graph_window_weak = graph_window.as_weak();
    app.on_openGraphWindow(move || {
        if let Some(window) = graph_window_weak.upgrade() {
            window.show().unwrap();
        }
    });

    app.run().unwrap();
    drop(calc_result_timer);
}
