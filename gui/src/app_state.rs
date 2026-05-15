use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use domain::models::elements::Schemas;
use domain::models::model::{ItemData, ItemLine, ItemList, ItemSet};
use domain::utility::calculation::{LineCalculationRequest, LineCalculationResult};

use crate::util::{
    build_key_data_for_schema, build_unit_options, read_dir_entries, validate_value_str,
};
use crate::{Action, ActionType, AppWindow, KeyData, LineItem, LineState};

// ─────────────────────────────────────────────────────────────────────────────

pub type LineModel = Rc<VecModel<LineItem>>;
pub type ListModels = Rc<RefCell<Vec<LineModel>>>;

pub type KeyDataModel = Rc<VecModel<KeyData>>;
pub type KeyDataModelsForList = Rc<RefCell<Vec<KeyDataModel>>>;
pub type AllKeyDataModels = Rc<RefCell<Vec<KeyDataModelsForList>>>;

/// All shared application state passed to dispatch handlers.
pub struct AppState {
    pub schemas: Schemas,
    pub list_models: ListModels,
    pub all_key_data_models: AllKeyDataModels,
    pub active_list_idx: Rc<RefCell<usize>>,
    pub list_names: Rc<VecModel<SharedString>>,
    pub calc_sender: Sender<LineCalculationRequest>,
    pub calc_result_receiver: RefCell<Receiver<LineCalculationResult>>,
    pub app_weak: slint::Weak<AppWindow>,
}

// ─────────────────────────────────────────────────────────────────────────────

// ── AppState implementation ───────────────────────────────────────────────────

impl AppState {
    fn ordered_sets_for_schema<'a>(
        schema: Option<&'a domain::models::elements::ElementSchema>,
        data: &'a [ItemSet],
    ) -> Vec<&'a ItemSet> {
        let Some(schema) = schema else {
            return data.iter().collect();
        };

        let mut ordered_sets = Vec::with_capacity(data.len());
        for field in schema.fields() {
            if let Some(item_set) = data.iter().find(|set| set.key == field.name) {
                ordered_sets.push(item_set);
            }
        }
        ordered_sets.extend(data.iter().filter(|set| !schema.contains_field(&set.key)));
        ordered_sets
    }

    /// Sets the active list index and updates the UI lines model accordingly.
    pub fn set_lines_model(&self, idx: usize) {
        *self.active_list_idx.borrow_mut() = idx;
        if let Some(app) = self.app_weak.upgrade() {
            app.set_active_list_index(idx as i32);
            let model = self.list_models.borrow()[idx].clone();
            app.set_lines(ModelRc::from(model));
        }
    }

    /// Validates every key_data in every list. Updates each key_data's `is_valid` flag and
    /// each line's `is_valid` flag accordingly. Sets the app focus properties to the first
    /// invalid field (optionally switching the active list if needed), and returns `true`
    /// only when all fields are valid.
    fn validate_all_and_focus(&self, allow_list_switch: bool) -> bool {
        // Collect (list_idx, line_idx, key_data_idx, is_valid) results while borrowing models.
        struct InvalidField {
            list_idx: usize,
            line_idx: i32,
            key_data_idx: i32,
        }
        let mut first_invalid: Option<InvalidField> = None;

        {
            let list_models = self.list_models.borrow();
            let all_key_data = self.all_key_data_models.borrow();

            for list_idx in 0..list_models.len() {
                let lines_model = &list_models[list_idx];
                let key_data_for_list = all_key_data[list_idx].borrow();

                for li in 0..lines_model.row_count() {
                    if let (Some(mut line), Some(key_data_model)) =
                        (lines_model.row_data(li), key_data_for_list.get(li))
                    {
                        let schema = self.schemas.schema_for(line.title.as_str());
                        let mut line_is_valid = true;
                        for pi in 0..key_data_model.row_count() {
                            if let Some(mut key_data) = key_data_model.row_data(pi) {
                                let is_valid = schema
                                    .and_then(|s| s.field(key_data.key.as_str()))
                                    .map(|spec| {
                                        validate_value_str(key_data.value.as_str(), spec.ty)
                                    })
                                    .unwrap_or(!key_data.value.is_empty());
                                if is_valid != key_data.is_valid {
                                    key_data.is_valid = is_valid;
                                    key_data_model.set_row_data(pi, key_data);
                                }
                                if !is_valid {
                                    line_is_valid = false;
                                    if first_invalid.is_none() {
                                        first_invalid = Some(InvalidField {
                                            list_idx,
                                            line_idx: li as i32,
                                            key_data_idx: pi as i32,
                                        });
                                    }
                                }
                            }
                        }
                        // Update line state: Invalid if any field is invalid, Valid otherwise
                        // But only update if currently Invalid (to allow Running/Done to persist)
                        if line.state == LineState::Invalid {
                            let new_state = if line_is_valid {
                                LineState::Valid
                            } else {
                                LineState::Invalid
                            };
                            if new_state != line.state {
                                line.state = new_state;
                                lines_model.set_row_data(li, line);
                            }
                        }
                    }
                }
            }
        } // list_models and all_key_data borrows released here

        let all_valid = first_invalid.is_none();

        if let Some(app) = self.app_weak.upgrade() {
            let (invalid_line, invalid_key_data) = match &first_invalid {
                Some(f) => (f.line_idx, f.key_data_idx),
                None => (-1, -1),
            };

            // If the first invalid field is in a different list, switch to it.
            if allow_list_switch {
                if let Some(f) = &first_invalid {
                    let current = *self.active_list_idx.borrow();
                    if current != f.list_idx {
                        *self.active_list_idx.borrow_mut() = f.list_idx;
                        let model = self.list_models.borrow()[f.list_idx].clone();
                        app.set_active_list_index(f.list_idx as i32);
                        app.set_lines(ModelRc::from(model));
                    }
                }
            }

            app.set_first_invalid_line(invalid_line);
            app.set_first_invalid_key_data(invalid_key_data);
            // Incrementing the epoch triggers the `changed` handler in LineRow to focus
            // the first invalid LineEdit.
            app.set_validate_epoch(app.get_validate_epoch() + 1);
        }

        all_valid
    }

    // ── Individual action handlers ────────────────────────────────────────────

    pub fn handle_add_line(&self, action: &Action) {
        let active = *self.active_list_idx.borrow();
        let lines_model = self.list_models.borrow()[active].clone();
        let key_data_models = self.all_key_data_models.borrow()[active].clone();

        let name = action.schema_name.as_str();
        if name.is_empty() {
            return;
        }
        if let Some(schema) = self.schemas.schema_for(name) {
            let key_data = build_key_data_for_schema(schema, &self.schemas.units);

            let key_data_vec = Rc::new(VecModel::from(key_data));
            let key_data_model_rc = ModelRc::from(key_data_vec.clone());
            key_data_models.borrow_mut().push(key_data_vec);

            lines_model.push(LineItem {
                title: action.schema_name.clone(),
                state: LineState::Invalid,
                data: key_data_model_rc,
            });
        }
        self.validate_all_and_focus(false);
    }

    pub fn handle_value_changed(&self, action: &Action) {
        let active = *self.active_list_idx.borrow();
        let lines_model = self.list_models.borrow()[active].clone();
        let key_data_models = self.all_key_data_models.borrow()[active].clone();

        let li = action.line_index as usize;
        let pi = action.key_data_index as usize;
        let borrowed = key_data_models.borrow();
        if let Some(key_data_model) = borrowed.get(li) {
            if let Some(mut key_data) = key_data_model.row_data(pi) {
                let new_valid = lines_model
                    .row_data(li)
                    .and_then(|line| self.schemas.schema_for(line.title.as_str()))
                    .and_then(|schema| schema.field(key_data.key.as_str()))
                    .map(|spec| validate_value_str(action.new_value.as_str(), spec.ty))
                    .unwrap_or(!action.new_value.is_empty());
                key_data.value = action.new_value.clone();
                // Only update the model when something actually changed to avoid unnecessary redraws
                if new_valid != key_data.is_valid {
                    key_data.is_valid = new_valid;
                }
                key_data_model.set_row_data(pi, key_data);
            }
        }

        // Check if all fields in this line are now valid after the change
        let line_is_valid = key_data_models
            .borrow()
            .get(li)
            .map(|model| {
                (0..model.row_count()).all(|i| {
                    model.row_data(i).map(|kd| kd.is_valid).unwrap_or(false)
                })
            })
            .unwrap_or(false);

        // Update state: Valid if all fields are valid, Invalid otherwise
        if let Some(mut line) = lines_model.row_data(li) {
            let new_state = if line_is_valid {
                LineState::Valid
            } else {
                LineState::Invalid
            };
            if new_state != line.state {
                line.state = new_state;
                lines_model.set_row_data(li, line);
            }
        }
    }

    pub fn handle_unit_changed(&self, action: &Action) {
        let active = *self.active_list_idx.borrow();
        let key_data_models = self.all_key_data_models.borrow()[active].clone();

        let li = action.line_index as usize;
        let pi = action.key_data_index as usize;
        let borrowed = key_data_models.borrow();
        if let Some(key_data_model) = borrowed.get(li) {
            if let Some(mut key_data) = key_data_model.row_data(pi) {
                key_data.unit = action.new_value.clone();
                key_data_model.set_row_data(pi, key_data);
            }
        }
    }

    pub fn handle_debounced_field_check(&self, action: &Action) {
        let active = *self.active_list_idx.borrow();
        let line_idx = action.line_index as usize;

        if !self.line_is_fully_valid(active, line_idx) {
            return;
        }

        self.set_line_calc_state(active, line_idx, 1);
        self.notify_domain_line_valid(active, line_idx);
    }

    pub fn handle_line_type_changed(&self, action: &Action) {
        let active = *self.active_list_idx.borrow();
        let lines_model = self.list_models.borrow()[active].clone();
        let key_data_models = self.all_key_data_models.borrow()[active].clone();

        let li = action.line_index as usize;
        let name = action.schema_name.as_str();
        if let Some(schema) = self.schemas.schema_for(name) {
            let key_data = build_key_data_for_schema(schema, &self.schemas.units);

            let borrowed = key_data_models.borrow();
            if let Some(key_data_model) = borrowed.get(li) {
                key_data_model.set_vec(key_data);
            }
            drop(borrowed);

            if let Some(mut line) = lines_model.row_data(li) {
                line.title = action.schema_name.clone();
                line.state = LineState::Invalid;
                lines_model.set_row_data(li, line);
            }
        }
    }

    pub fn handle_remove_line(&self, action: &Action) {
        let active = *self.active_list_idx.borrow();
        let lines_model = self.list_models.borrow()[active].clone();
        let key_data_models = self.all_key_data_models.borrow()[active].clone();

        let li = action.line_index as usize;
        if li < lines_model.row_count() {
            lines_model.remove(li);
            let mut borrowed = key_data_models.borrow_mut();
            if li < borrowed.len() {
                borrowed.remove(li);
            }
        }
    }

    pub fn handle_switch_list(&self, action: &Action) {
        let new_idx = action.line_index as usize;
        let list_count = self.list_models.borrow().len();
        if new_idx < list_count {
            *self.active_list_idx.borrow_mut() = new_idx;
            let new_model = self.list_models.borrow()[new_idx].clone();
            if let Some(app) = self.app_weak.upgrade() {
                app.set_lines(ModelRc::from(new_model));
            }
        }
    }

    pub fn handle_add_list(&self) {
        let count = self.list_models.borrow().len();
        self.list_models
            .borrow_mut()
            .push(Rc::new(VecModel::<LineItem>::default()));
        self.all_key_data_models
            .borrow_mut()
            .push(Rc::new(RefCell::new(Vec::new())));
        self.list_names
            .push(SharedString::from(format!("list {count}").as_str()));
        let new_idx = count;
        self.set_lines_model(new_idx);
        if let Some(app) = self.app_weak.upgrade() {
            app.set_focus_toolbar_epoch(app.get_focus_toolbar_epoch() + 1);
        }
    }

    pub fn handle_remove_list(&self, action: &Action) {
        let idx = action.line_index as usize;
        let count = self.list_models.borrow().len();
        if idx == 0 || idx >= count {
            return;
        }
        self.list_models.borrow_mut().remove(idx);
        self.all_key_data_models.borrow_mut().remove(idx);
        self.list_names.remove(idx);
        let current = *self.active_list_idx.borrow();
        let new_active = if current >= idx && current > 0 {
            current - 1
        } else {
            current
        };
        self.set_lines_model(new_active);
    }

    pub fn handle_navigate_dir(&self, action: &Action) {
        let path_str = action.new_value.as_str();
        if path_str.is_empty() {
            return;
        }
        let canonical = match std::path::Path::new(path_str).canonicalize() {
            Ok(p) if p.is_dir() => p,
            _ => return,
        };
        let entries = read_dir_entries(&canonical);
        if let Some(app) = self.app_weak.upgrade() {
            app.set_file_browser_dir(SharedString::from(canonical.to_string_lossy().as_ref()));
            app.set_file_browser_entries(ModelRc::from(Rc::new(VecModel::from(entries))));
        }
    }

    pub fn handle_validate_before_save(&self) {
        if self.validate_all_and_focus(true) {
            if let Some(app) = self.app_weak.upgrade() {
                app.set_open_save_dialog(true);
            }
        } else if let Some(app) = self.app_weak.upgrade() {
            app.set_validation_error_epoch(app.get_validation_error_epoch() + 1);
        }
    }

    pub fn handle_save_list(&self, action: &Action) {
        let path = action.new_value.as_str();
        if path.is_empty() {
            return;
        }
        let list_models_ref = self.list_models.borrow();
        let all_key_data_ref = self.all_key_data_models.borrow();
        let mut item_lists: Vec<ItemList> = Vec::new();
        for li in 0..list_models_ref.len() {
            let name = self
                .list_names
                .row_data(li)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let line_model = &list_models_ref[li];
            let key_data_for_list = all_key_data_ref[li].borrow();
            let mut item_lines: Vec<ItemLine> = Vec::new();
            for (line_idx, key_data_model) in key_data_for_list.iter().enumerate() {
                let title = line_model
                    .row_data(line_idx)
                    .map(|l| l.title.to_string())
                    .unwrap_or_default();
                let item_sets: Vec<ItemSet> = (0..key_data_model.row_count())
                    .filter_map(|pi| key_data_model.row_data(pi))
                    .map(|p| ItemSet {
                        key: p.key.to_string(),
                        value: p.value.to_string(),
                        unit: p.unit.to_string(),
                    })
                    .collect();
                item_lines.push(ItemLine {
                    title,
                    data: item_sets,
                });
            }
            item_lists.push(ItemList {
                name,
                lines: item_lines,
            });
        }

        let data = ItemData { lists: item_lists };
        let _ = domain::utility::persistence::save(path, &data);

        if let Some(app) = self.app_weak.upgrade() {
            app.set_is_dirty(false);
        }
    }

    pub fn handle_load_list(&self, action: &Action) {
        let path = action.new_value.as_str();
        if path.is_empty() {
            return;
        }
        let item_data: ItemData = match domain::utility::persistence::load_validated(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to load list: {}", e);
                if let Some(app) = self.app_weak.upgrade() {
                    // You could set an error message in the UI here
                    app.set_validation_error_epoch(app.get_validation_error_epoch() + 1);
                }
                return;
            }
        };
        if item_data.lists.is_empty() {
            return;
        }

        let mut new_list_models: Vec<Rc<VecModel<LineItem>>> = Vec::new();
        let mut new_key_data_models: Vec<Rc<RefCell<Vec<Rc<VecModel<KeyData>>>>>> = Vec::new();

        for item_list in &item_data.lists {
            let line_model: Rc<VecModel<LineItem>> = Rc::new(VecModel::<LineItem>::default());
            let key_data_for_list: Rc<RefCell<Vec<Rc<VecModel<KeyData>>>>> =
                Rc::new(RefCell::new(Vec::new()));

            for item_line in &item_list.lines {
                let schema = self.schemas.schema_for(&item_line.title);
                let key_data: Vec<KeyData> = Self::ordered_sets_for_schema(schema, &item_line.data)
                    .into_iter()
                    .map(|p| {
                        let unit_options = self
                            .schemas
                            .schema_for(&item_line.title)
                            .and_then(|schema| schema.field(&p.key))
                            .map(|key_spec| build_unit_options(key_spec, &self.schemas.units))
                            .unwrap_or_else(|| {
                                ModelRc::from(Rc::new(VecModel::<SharedString>::default()))
                            });
                        KeyData {
                            key: SharedString::from(p.key.as_str()),
                            value: SharedString::from(p.value.as_str()),
                            unit: SharedString::from(p.unit.as_str()),
                            unit_options,
                            is_valid: true,
                        }
                    })
                    .collect();

                let key_data_vec = Rc::new(VecModel::from(key_data));
                key_data_for_list.borrow_mut().push(key_data_vec.clone());
                line_model.push(LineItem {
                    title: SharedString::from(item_line.title.as_str()),
                    state: LineState::Valid,
                    data: ModelRc::from(key_data_vec),
                });
            }

            new_list_models.push(line_model);
            new_key_data_models.push(key_data_for_list);
        }

        *self.list_models.borrow_mut() = new_list_models;
        *self.all_key_data_models.borrow_mut() = new_key_data_models;

        while self.list_names.row_count() > 0 {
            self.list_names.remove(0);
        }
        for item_list in &item_data.lists {
            self.list_names
                .push(SharedString::from(item_list.name.as_str()));
        }

        *self.active_list_idx.borrow_mut() = 0;
        if let Some(app) = self.app_weak.upgrade() {
            app.set_active_list_index(0);
            let first_model = self.list_models.borrow()[0].clone();
            app.set_lines(ModelRc::from(first_model));
            app.set_is_dirty(false);
        }
    }

    pub fn handle_exit(&self) {
        if let Some(app) = self.app_weak.upgrade() {
            let _ = app.hide();
        }
    }

    fn focus_relative_field(&self, action: &Action, step: i32) {
        let active = *self.active_list_idx.borrow();
        let target = {
            let all_key_data = self.all_key_data_models.borrow();
            let key_data_for_list = all_key_data[active].borrow();
            let mut positions: Vec<(i32, i32)> = Vec::new();

            for (line_idx, key_data_model) in key_data_for_list.iter().enumerate() {
                for key_idx in 0..key_data_model.row_count() {
                    positions.push((line_idx as i32, key_idx as i32));
                }
            }

            if positions.is_empty() {
                None
            } else {
                let current = (action.line_index, action.key_data_index);
                let current_idx = positions.iter().position(|p| *p == current).unwrap_or(0);

                if step >= 0 {
                    if current_idx + 1 >= positions.len() {
                        self.focus_toolbar_controls();
                        return;
                    }
                    Some(positions[current_idx + 1])
                } else if current_idx == 0 {
                    Some(positions[positions.len() - 1])
                } else {
                    Some(positions[current_idx - 1])
                }
            }
        };

        let Some((target_line, target_key)) = target else {
            return;
        };

        if let Some(app) = self.app_weak.upgrade() {

            app.set_first_invalid_line(target_line);
            app.set_first_invalid_key_data(target_key);
            app.set_validate_epoch(app.get_validate_epoch() + 1);
        }
    }

    fn focus_toolbar_controls(&self) {
        if let Some(app) = self.app_weak.upgrade() {
            app.set_focus_toolbar_epoch(app.get_focus_toolbar_epoch() + 1);
        }
    }

    fn cycle_list(&self, step: i32) {
        let count = self.list_models.borrow().len();
        if count == 0 {
            return;
        }

        let current = *self.active_list_idx.borrow();
        let next = if step >= 0 {
            (current + 1) % count
        } else if current == 0 {
            count - 1
        } else {
            current - 1
        };

        self.set_lines_model(next);
        self.focus_last_field_in_list(next);
    }

    fn focus_last_field_in_list(&self, list_idx: usize) {
        let key_data_models = self.all_key_data_models.borrow();
        let Some(key_data_for_list) = key_data_models.get(list_idx) else {
            self.focus_add_list_button();
            return;
        };

        let key_data_for_list = key_data_for_list.borrow();
        let Some((line_idx, key_idx)) = key_data_for_list
            .iter()
            .enumerate()
            .rev()
            .find_map(|(li, key_data_model)| {
                let row_count = key_data_model.row_count();
                if row_count > 0 {
                    Some((li as i32, (row_count - 1) as i32))
                } else {
                    None
                }
            })
        else {
            self.focus_add_list_button();
            return;
        };

        if let Some(app) = self.app_weak.upgrade() {
            app.set_first_invalid_line(line_idx);
            app.set_first_invalid_key_data(key_idx);
            app.set_validate_epoch(app.get_validate_epoch() + 1);
        }
    }

    fn focus_add_list_button(&self) {
        if let Some(app) = self.app_weak.upgrade() {
            app.set_focus_add_list_epoch(app.get_focus_add_list_epoch() + 1);
        }
    }

    fn line_is_fully_valid(&self, list_idx: usize, line_idx: usize) -> bool {
        let all_key_data = self.all_key_data_models.borrow();
        let Some(key_data_for_list) = all_key_data.get(list_idx) else {
            return false;
        };

        let key_data_for_list = key_data_for_list.borrow();
        let Some(key_data_model) = key_data_for_list.get(line_idx) else {
            return false;
        };

        let field_count = key_data_model.row_count();
        if field_count == 0 {
            return false;
        }

        (0..field_count).all(|pi| {
            key_data_model
                .row_data(pi)
                .map(|key_data| key_data.is_valid)
                .unwrap_or(false)
        })
    }

    fn snapshot_line_for_calc(&self, list_idx: usize, line_idx: usize) -> Option<ItemLine> {
        let list_models = self.list_models.borrow();
        let all_key_data = self.all_key_data_models.borrow();

        let line_model = list_models.get(list_idx)?.row_data(line_idx)?;
        let key_data_for_list = all_key_data.get(list_idx)?.borrow();
        let key_data_model = key_data_for_list.get(line_idx)?;

        let data = (0..key_data_model.row_count())
            .filter_map(|pi| key_data_model.row_data(pi))
            .map(|key_data| ItemSet {
                key: key_data.key.to_string(),
                value: key_data.value.to_string(),
                unit: key_data.unit.to_string(),
            })
            .collect();

        Some(ItemLine {
            title: line_model.title.to_string(),
            data,
        })
    }

    fn notify_domain_line_valid(&self, list_idx: usize, line_idx: usize) {
        let Some(line) = self.snapshot_line_for_calc(list_idx, line_idx) else {
            return;
        };

        let list_name = self
            .list_names
            .row_data(list_idx)
            .map(|name| name.to_string())
            .unwrap_or_default();

        let request = LineCalculationRequest {
            list_index: list_idx,
            list_name,
            line_index: line_idx,
            line,
        };

        if let Err(error) = self.calc_sender.send(request) {
            eprintln!("failed to send line calculation request to domain: {error}");
            self.set_line_calc_state(list_idx, line_idx, 0);
        }
    }

    pub fn poll_calculation_results(&self) {
        let receiver = self.calc_result_receiver.borrow_mut();
        while let Ok(result) = receiver.try_recv() {
            eprintln!(
                "[gui] calc result list=#{} line=#{} numeric_values={} numeric_sum={}",
                result.list_index, result.line_index, result.numeric_count, result.numeric_sum
            );
            self.set_line_calc_state(result.list_index, result.line_index, 2);
        }
    }

    fn set_line_calc_state(&self, list_idx: usize, line_idx: usize, calc_state: i32) {
        let list_models = self.list_models.borrow();
        let Some(lines_model) = list_models.get(list_idx) else {
            return;
        };

        let Some(mut line) = lines_model.row_data(line_idx) else {
            return;
        };

        let new_state = match calc_state {
            1 => LineState::Running,
            2 => LineState::Done,
            _ => return, // Only handle Running and Done states
        };

        if new_state != line.state {
            line.state = new_state;
            lines_model.set_row_data(line_idx, line);
        }
    }

    // ── Main dispatch entry point ─────────────────────────────────────────────

    pub fn handle_dispatch(&self, action: Action) {
        // Determine dirty-flag impact before consuming action fields
        let marks_dirty = !matches!(
            action.action_type,
            ActionType::TabNextField
                | ActionType::TabPrevField
                | ActionType::DebouncedFieldCheck
                | ActionType::SwitchList
                | ActionType::CycleListNext
                | ActionType::CycleListPrev
                | ActionType::SaveList
                | ActionType::LoadList
                | ActionType::NavigateDir
                | ActionType::ValidateBeforeSave
                | ActionType::Exit
        );

        match action.action_type {
            ActionType::AddLine => self.handle_add_line(&action),
            ActionType::ValueChanged => self.handle_value_changed(&action),
            ActionType::TabNextField => self.focus_relative_field(&action, 1),
            ActionType::TabPrevField => self.focus_relative_field(&action, -1),
            ActionType::DebouncedFieldCheck => self.handle_debounced_field_check(&action),
            ActionType::UnitChanged => self.handle_unit_changed(&action),
            ActionType::LineTypeChanged => self.handle_line_type_changed(&action),
            ActionType::RemoveLine => self.handle_remove_line(&action),
            ActionType::SwitchList => self.handle_switch_list(&action),
            ActionType::CycleListNext => self.cycle_list(1),
            ActionType::CycleListPrev => self.cycle_list(-1),
            ActionType::AddList => self.handle_add_list(),
            ActionType::RemoveList => self.handle_remove_list(&action),
            ActionType::SaveList => self.handle_save_list(&action),
            ActionType::LoadList => self.handle_load_list(&action),
            ActionType::NavigateDir => self.handle_navigate_dir(&action),
            ActionType::ValidateBeforeSave => self.handle_validate_before_save(),
            ActionType::Exit => self.handle_exit(),
        }

        // Update the is-dirty property on the UI
        if marks_dirty {
            if let Some(app) = self.app_weak.upgrade() {
                app.set_is_dirty(true);
            }
        }
    }
}
