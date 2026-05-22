use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use crate::models::elements::Schemas;
use crate::models::model::ItemLine;
use crate::models::typed_lines::{ButtonLine, ContainerLine, TextFieldLine};
use crate::models::unit::length_unit::LengthUnit;

pub struct LineCalculationRequest {
    pub list_index: usize,
    pub list_name: String,
    pub line_index: usize,
    pub line: ItemLine,
}

pub struct LineCalculationResult {
    pub list_index: usize,
    pub line_index: usize,
    pub numeric_count: usize,
    pub numeric_sum: f64,
}

pub fn spawn_line_calculation_worker(
    result_sender: Sender<Result<LineCalculationResult, String>>,
) -> Sender<LineCalculationRequest> {
    let (tx, rx) = mpsc::channel::<LineCalculationRequest>();
    let schemas = Schemas::load_default();

    thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            eprintln!(
                "[domain-calc] start list=#{} '{}' line=#{} title='{}'",
                request.list_index, request.list_name, request.line_index, request.line.title
            );

            if let Err(error) = calculate_line_by_title(&request.line, &schemas) {
                let message = format!(
                    "failed to calculate line title='{}': {error}",
                    request.line.title
                );
                eprintln!("[domain-calc] {message}");
                let _ = result_sender.send(Err(message));
                continue;
            }

            // Simulate an expensive calculation job.
            thread::sleep(Duration::from_secs(5));

            let (numeric_count, numeric_sum) = collect_numeric_values(&request.line);
            eprintln!(
                "[domain-calc] done list=#{} '{}' line=#{} title='{}' numeric_values={} numeric_sum={}",
                request.list_index,
                request.list_name,
                request.line_index,
                request.line.title,
                numeric_count,
                numeric_sum
            );

            let result = LineCalculationResult {
                list_index: request.list_index,
                line_index: request.line_index,
                numeric_count,
                numeric_sum,
            };

            if let Err(error) = result_sender.send(Ok(result)) {
                eprintln!("[domain-calc] failed to send result: {error}");
            }
        }
    });

    tx
}

fn calculate_line_by_title(line: &ItemLine, schemas: &Schemas) -> Result<(), String> {
    match line.title.as_str() {
        "Container" => calculate_container_line(line, schemas),
        "Button" => calculate_button_line(line, schemas),
        "TextField" => calculate_text_field_line(line, schemas),
        _ => Ok(()),
    }
}

fn calculate_container_line(line: &ItemLine, schemas: &Schemas) -> Result<(), String> {
    let container =
        ContainerLine::try_from_item_line(line, schemas).map_err(|error| error.to_string())?;

    print_container_data_all_units(&container);
    Ok(())
}

fn calculate_button_line(line: &ItemLine, schemas: &Schemas) -> Result<(), String> {
    let button = ButtonLine::try_from_item_line(line, schemas).map_err(|error| error.to_string())?;
    eprintln!("[domain-calc][Button] label='{}'", button.label);
    Ok(())
}

fn calculate_text_field_line(line: &ItemLine, schemas: &Schemas) -> Result<(), String> {
    let text_field = TextFieldLine::try_from_item_line(line, schemas)
        .map_err(|error| error.to_string())?;
    eprintln!(
        "[domain-calc][TextField] placeholder='{}' maxLength={} value='{}'",
        text_field.placeholder, text_field.max_length, text_field.value
    );
    Ok(())
}

fn print_container_data_all_units(container: &ContainerLine) {
    eprintln!(
        "[domain-calc][Container] width: {}",
        render_all_length_units(container.width.value, container.width.unit)
    );
    eprintln!(
        "[domain-calc][Container] height: {}",
        render_all_length_units(container.height.value, container.height.unit)
    );
    eprintln!(
        "[domain-calc][Container] padding: {}",
        render_all_length_units(container.padding.value as f64, container.padding.unit)
    );
}

fn render_all_length_units(value: f64, from: LengthUnit) -> String {
    all_length_units()
        .iter()
        .map(|to| {
            let converted = LengthUnit::convert_value(value, from, *to);
            format!("{converted:.4} {}", to.as_str())
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn all_length_units() -> [LengthUnit; 4] {
    [
        LengthUnit::Px,
        LengthUnit::Em,
        LengthUnit::Rem,
        LengthUnit::Percent,
    ]
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
    use super::{
        calculate_button_line, calculate_container_line, calculate_line_by_title,
        calculate_text_field_line, collect_numeric_values, render_all_length_units,
    };
    use crate::models::elements::Schemas;
    use crate::models::model::{ItemLine, ItemSet};
    use crate::models::unit::length_unit::LengthUnit;

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
    fn calculates_container_line_via_title_dispatch() {
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

        let result = calculate_line_by_title(&line, &schemas);
        assert!(result.is_ok());
    }

    #[test]
    fn calculates_container_item_line_in_dedicated_function() {
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

        let result = calculate_container_line(&line, &schemas);
        assert!(result.is_ok());
    }

    #[test]
    fn renders_all_length_units() {
        let rendered = render_all_length_units(10.0, LengthUnit::Px);
        assert!(rendered.contains("px"));
        assert!(rendered.contains("em"));
        assert!(rendered.contains("rem"));
        assert!(rendered.contains("%"));
    }

    #[test]
    fn calculates_button_item_line_in_dedicated_function() {
        let schemas = Schemas::load_default();
        let line = ItemLine {
            title: "Button".to_string(),
            data: vec![ItemSet {
                key: "label".to_string(),
                value: "Save".to_string(),
                unit: String::new(),
            }],
        };

        let result = calculate_button_line(&line, &schemas);
        assert!(result.is_ok());
    }

    #[test]
    fn calculates_text_field_item_line_in_dedicated_function() {
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
                    unit: String::new(),
                },
                ItemSet {
                    key: "value".to_string(),
                    value: "abc".to_string(),
                    unit: String::new(),
                },
            ],
        };

        let result = calculate_text_field_line(&line, &schemas);
        assert!(result.is_ok());
    }
}
