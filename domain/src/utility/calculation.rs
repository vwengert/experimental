use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use crate::models::elements::Schemas;
use crate::models::model::ItemLine;
use crate::models::typed_lines::{ButtonLine, ContainerLine, TextFieldLine};

pub enum LineGenerator {
    Container(ContainerLine),
    Button(ButtonLine),
    TextField(TextFieldLine),
    Unknown(UnknownLineData),
}

pub struct UnknownLineData {
    pub title: String,
    pub numeric_count: usize,
    pub numeric_sum: f64,
}

impl LineGenerator {
    fn from_line(line: &ItemLine, schemas: &Schemas) -> Result<Self, String> {
        match line.title.as_str() {
            "Container" => ContainerLine::try_from_item_line(line, schemas)
                .map(Self::Container)
                .map_err(|error| error.to_string()),
            "Button" => ButtonLine::try_from_item_line(line, schemas)
                .map(Self::Button)
                .map_err(|error| error.to_string()),
            "TextField" => TextFieldLine::try_from_item_line(line, schemas)
                .map(Self::TextField)
                .map_err(|error| error.to_string()),
            _ => {
                let (numeric_count, numeric_sum) = collect_numeric_values(line);
                Ok(Self::Unknown(UnknownLineData {
                    title: line.title.clone(),
                    numeric_count,
                    numeric_sum,
                }))
            }
        }
    }

    fn title(&self) -> &str {
        match self {
            Self::Container(_) => "Container",
            Self::Button(_) => "Button",
            Self::TextField(_) => "TextField",
            Self::Unknown(data) => data.title.as_str(),
        }
    }

    fn collect_numeric_values(&self) -> (f64, f64, f64) {
        match self {
            Self::Container(container) => {
                let width = container.width.value;
                let height = container.height.value;
                let padding = container.padding.value as f64;
                (width, height, padding)
            }
            Self::Button(_) => (0.0, 0.0, 1.0),
            Self::TextField(_text_field) => (1.0, 1.0, 0.0),
            Self::Unknown(_data) => (1.0, 1.0, 1.0),
        }
    }
}

pub struct LineCalculationRequest {
    pub list_index: usize,
    pub list_name: String,
    pub line_index: usize,
    pub request_revision: u64,
    pub line: ItemLine,
}

impl LineCalculationRequest {
    pub fn new(
        list_index: usize,
        list_name: String,
        line_index: usize,
        request_revision: u64,
        line: ItemLine,
    ) -> Self {
        Self {
            list_index,
            list_name,
            line_index,
            request_revision,
            line,
        }
    }
}

#[derive(Clone)]
pub struct LineCalculationResult {
    pub list_index: usize,
    pub line_index: usize,
    pub request_revision: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub fn spawn_line_calculation_worker(
    result_sender: Sender<Result<LineCalculationResult, String>>,
) -> Sender<LineCalculationRequest> {
    let (tx, rx) = mpsc::channel::<LineCalculationRequest>();
    let schemas = Schemas::load_default();

    thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            let generator = match LineGenerator::from_line(&request.line, &schemas) {
                Ok(generator) => generator,
                Err(error) => {
                    let message = format!(
                        "failed to build line generator title='{}': {error}",
                        request.line.title
                    );
                    eprintln!("[domain-calc] {message}");
                    let _ = result_sender.send(Err(message));
                    continue;
                }
            };

            eprintln!(
                "[domain-calc] start list=#{} '{}' line=#{} title='{}'",
                request.list_index,
                request.list_name,
                request.line_index,
                generator.title()
            );

            if let Err(error) = calculate_line_by_generator(&generator) {
                let message = format!(
                    "failed to calculate line title='{}': {error}",
                    generator.title()
                );
                eprintln!("[domain-calc] {message}");
                let _ = result_sender.send(Err(message));
                continue;
            }

            // Simulate an expensive calculation job.
            thread::sleep(Duration::from_secs(5));

            let (x, y, z) = generator.collect_numeric_values();
            eprintln!(
                "[domain-calc] done list=#{} '{}' line=#{} title='{}' x={} y={} z={}",
                request.list_index,
                request.list_name,
                request.line_index,
                generator.title(),
                x,
                y,
                z,
            );

            let result = LineCalculationResult {
                list_index: request.list_index,
                line_index: request.line_index,
                request_revision: request.request_revision,
                x,
                y,
                z,
            };

            if let Err(error) = result_sender.send(Ok(result)) {
                eprintln!("[domain-calc] failed to send result: {error}");
            }
        }
    });

    tx
}

fn calculate_line_by_generator(generator: &LineGenerator) -> Result<(), String> {
    match generator {
        LineGenerator::Container(container) => container.calculate(),
        LineGenerator::Button(button) => button.calculate(),
        LineGenerator::TextField(text_field) => text_field.calculate(),
        _ => Ok(()),
    }
}

fn collect_numeric_values(line: &ItemLine) -> (usize, f64) {
    line.data
        .iter()
        .filter_map(|item| item.value.parse::<f64>().ok())
        .fold((0usize, 0.0f64), |(count, sum), value| {
            (count + 1, sum + value)
        })
}

#[cfg(test)]
mod tests {
    use super::{calculate_line_by_generator, collect_numeric_values, LineGenerator};
    use crate::models::elements::Schemas;
    use crate::models::model::{ItemLine, ItemSet};
    use crate::models::typed_lines::{ButtonLine, ContainerLine, TextFieldLine};

    #[test]
    fn collects_only_numeric_values() {
        let line = ItemLine {
            title: "example".to_string(),
            data: vec![
                ItemSet {
                    key: "a".to_string(),
                    value: "10".to_string(),
                    unit: "m".to_string(),
                },
                ItemSet {
                    key: "b".to_string(),
                    value: "x".to_string(),
                    unit: "m".to_string(),
                },
                ItemSet {
                    key: "c".to_string(),
                    value: "2.5".to_string(),
                    unit: "m".to_string(),
                },
            ],
        };

        let (count, sum) = collect_numeric_values(&line);
        assert_eq!(count, 2);
        assert!((sum - 12.5).abs() < f64::EPSILON);
    }

    #[test]
    fn calculates_container_line_via_generator_dispatch() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "Container".to_string(),
            data: vec![
                ItemSet {
                    key: "width".to_string(),
                    value: "100.5".to_string(),
                    unit: "px".to_string(),
                },
                ItemSet {
                    key: "height".to_string(),
                    value: "42.0".to_string(),
                    unit: "em".to_string(),
                },
                ItemSet {
                    key: "padding".to_string(),
                    value: "8".to_string(),
                    unit: "rem".to_string(),
                },
            ],
        };

        let generator = LineGenerator::from_line(&line, &schemas).unwrap();
        let result = calculate_line_by_generator(&generator);
        assert!(result.is_ok());
    }

    #[test]
    fn container_with_zero_values_produces_zero_calculation_parameters() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "Container".to_string(),
            data: vec![
                ItemSet {
                    key: "width".to_string(),
                    value: "0".to_string(),
                    unit: "px".to_string(),
                },
                ItemSet {
                    key: "height".to_string(),
                    value: "0".to_string(),
                    unit: "px".to_string(),
                },
                ItemSet {
                    key: "padding".to_string(),
                    value: "0".to_string(),
                    unit: "px".to_string(),
                },
            ],
        };

        let _generator = LineGenerator::from_line(&line, &schemas).unwrap();
    }

    #[test]
    fn calculates_container_item_line_via_struct_impl() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "Container".to_string(),
            data: vec![
                ItemSet {
                    key: "width".to_string(),
                    value: "100.5".to_string(),
                    unit: "px".to_string(),
                },
                ItemSet {
                    key: "height".to_string(),
                    value: "42.0".to_string(),
                    unit: "em".to_string(),
                },
                ItemSet {
                    key: "padding".to_string(),
                    value: "8".to_string(),
                    unit: "rem".to_string(),
                },
            ],
        };

        let container = ContainerLine::try_from_item_line(&line, &schemas).unwrap();
        let result = container.calculate();
        assert!(result.is_ok());
    }

    #[test]
    fn calculates_button_item_line_via_struct_impl() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "Button".to_string(),
            data: vec![ItemSet {
                key: "label".to_string(),
                value: "Save".to_string(),
                unit: String::new(),
            }],
        };

        let button = ButtonLine::try_from_item_line(&line, &schemas).unwrap();
        let result = button.calculate();
        assert!(result.is_ok());
    }

    #[test]
    fn calculates_text_field_item_line_via_struct_impl() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "TextField".to_string(),
            data: vec![
                ItemSet {
                    key: "placeholder".to_string(),
                    value: "Search...".to_string(),
                    unit: String::new(),
                },
                ItemSet {
                    key: "maxLength".to_string(),
                    value: "100".to_string(),
                    unit: "m".to_string(),
                },
                ItemSet {
                    key: "value".to_string(),
                    value: "abc".to_string(),
                    unit: String::new(),
                },
            ],
        };

        let text_field = TextFieldLine::try_from_item_line(&line, &schemas).unwrap();
        let result = text_field.calculate();
        assert!(result.is_ok());
    }
}
