use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use crate::models::model::ItemLine;

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
    result_sender: Sender<LineCalculationResult>,
) -> Sender<LineCalculationRequest> {
    let (tx, rx) = mpsc::channel::<LineCalculationRequest>();

    thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            eprintln!(
                "[domain-calc] start list=#{} '{}' line=#{} title='{}'",
                request.list_index, request.list_name, request.line_index, request.line.title
            );

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

            if let Err(error) = result_sender.send(result) {
                eprintln!("[domain-calc] failed to send result: {error}");
            }
        }
    });

    tx
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
    use super::collect_numeric_values;
    use crate::models::model::{ItemLine, ItemSet};

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
}


