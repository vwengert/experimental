use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use domain::models::elements::{ElementSchema, FieldSpec, ValueType};
use domain::utility::calculation::LineCalculationResult;

use crate::{FileEntry, GraphPath, GraphPoint, KeyData};

#[derive(Clone)]
struct ProjectedGraphPoint {
    x: f32,
    y: f32,
    size: f32,
    depth: f32,
    series: i32,
    line_index: usize,
}

pub fn build_unit_options(
    spec: &FieldSpec,
    units: &HashMap<String, Vec<String>>,
) -> ModelRc<SharedString> {
    match &spec.unit {
        None => ModelRc::from(Rc::new(VecModel::<SharedString>::default())),
        Some(unit_type) => {
            let unit_values: &[String] = units
                .get(unit_type.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            ModelRc::from(Rc::new(VecModel::from(
                unit_values
                    .iter()
                    .map(|s| SharedString::from(s.as_str()))
                    .collect::<Vec<_>>(),
            )))
        }
    }
}

pub fn make_key_data(key: &str, spec: &FieldSpec, units: &HashMap<String, Vec<String>>) -> KeyData {
    let unit_options = build_unit_options(spec, units);
    let unit = match &spec.unit {
        None => SharedString::new(),
        Some(unit_type) => units
            .get(unit_type.as_str())
            .and_then(|v| v.first())
            .map(|s| SharedString::from(s.as_str()))
            .unwrap_or_default(),
    };
    KeyData {
        key: SharedString::from(key),
        value: SharedString::new(),
        unit,
        unit_options,
        is_valid: false,
    }
}

pub fn build_key_data_for_schema(
    schema: &ElementSchema,
    units: &HashMap<String, Vec<String>>,
) -> Vec<KeyData> {
    schema
        .iter_fields()
        .map(|(name, spec)| make_key_data(name, spec, units))
        .collect()
}

pub fn read_dir_entries(path: &std::path::Path) -> Vec<FileEntry> {
    let mut entries: Vec<FileEntry> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(FileEntry {
                name: SharedString::from(name.as_str()),
                is_dir,
            });
        }
        // Directories first, then files, both sorted alphabetically
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then(a.name.as_str().cmp(b.name.as_str()))
        });
    }
    entries
}

/// Returns true if the string `value` is non-empty and matches the expected `ty`.
pub fn validate_value_str(value: &str, ty: ValueType) -> bool {
    if value.is_empty() {
        return false;
    }
    match ty {
        ValueType::Str => true,
        ValueType::Int => value.parse::<i64>().is_ok(),
        ValueType::Float => value.parse::<f64>().is_ok(),
        ValueType::Bool => matches!(value.to_lowercase().as_str(), "true" | "false"),
    }
}

fn project_graph_geometry(
    results: &[Vec<Option<LineCalculationResult>>],
) -> Vec<ProjectedGraphPoint> {
    let mut flattened: Vec<(usize, &LineCalculationResult)> = results
        .iter()
        .enumerate()
        .flat_map(|(list_idx, entries)| {
            entries
                .iter()
                .filter_map(move |entry| entry.as_ref().map(|result| (list_idx, result)))
        })
        .collect();

    if flattened.is_empty() {
        return Vec::new();
    }

    let max_line_index = flattened
        .iter()
        .map(|(_, result)| result.line_index)
        .max()
        .unwrap_or(0) as f32;
    let max_numeric_count = flattened
        .iter()
        .map(|(_, result)| result.numeric_count)
        .max()
        .unwrap_or(0) as f32;
    let min_numeric_sum = flattened
        .iter()
        .map(|(_, result)| result.numeric_sum)
        .fold(f64::INFINITY, f64::min);
    let max_numeric_sum = flattened
        .iter()
        .map(|(_, result)| result.numeric_sum)
        .fold(f64::NEG_INFINITY, f64::max);
    let numeric_sum_range = (max_numeric_sum - min_numeric_sum).max(f64::EPSILON) as f32;

    let origin_x = 176.0f32;
    let origin_y = 340.0f32;
    let x_basis = (220.0f32, -84.0f32);
    let y_basis = (-140.0f32, -84.0f32);
    let z_basis = (0.0f32, -210.0f32);

    let mut points: Vec<ProjectedGraphPoint> = flattened
        .drain(..)
        .map(|(list_idx, result)| {
            let nx = if max_line_index > 0.0 {
                result.line_index as f32 / max_line_index
            } else {
                0.5
            };
            let ny = if max_numeric_count > 0.0 {
                result.numeric_count as f32 / max_numeric_count
            } else {
                0.5
            };
            let nz = if numeric_sum_range > 0.0 {
                ((result.numeric_sum - min_numeric_sum) as f32 / numeric_sum_range).clamp(0.0, 1.0)
            } else {
                0.5
            };

            let x = origin_x + nx * x_basis.0 + ny * y_basis.0 + nz * z_basis.0;
            let y = origin_y + nx * x_basis.1 + ny * y_basis.1 + nz * z_basis.1;
            let depth = nx + ny + nz;

            ProjectedGraphPoint {
                x,
                y,
                size: 10.0 + ny * 8.0 + nz * 4.0,
                depth,
                series: list_idx as i32,
                line_index: result.line_index,
            }
        })
        .collect();

    points.sort_by(|left, right| left.depth.total_cmp(&right.depth));
    points
}

pub fn project_graph_points(results: &[Vec<Option<LineCalculationResult>>]) -> Vec<GraphPoint> {
    project_graph_geometry(results)
        .into_iter()
        .map(|point| GraphPoint {
            x: point.x,
            y: point.y,
            size: point.size,
            depth: point.depth,
            series: point.series,
        })
        .collect()
}

pub fn project_graph_paths(results: &[Vec<Option<LineCalculationResult>>]) -> Vec<GraphPath> {
    let projected = project_graph_geometry(results);
    let series_count = projected
        .iter()
        .map(|point| point.series)
        .max()
        .map(|value| value + 1)
        .unwrap_or(0);

    let mut grouped: Vec<Vec<ProjectedGraphPoint>> =
        (0..series_count).map(|_| Vec::new()).collect();
    for point in projected {
        if let Some(points) = grouped.get_mut(point.series as usize) {
            points.push(point);
        }
    }

    let mut paths = Vec::new();
    for points in &mut grouped {
        points.sort_by_key(|point| point.line_index);

        if points.len() < 2 {
            continue;
        }

        let mut commands = String::new();
        let first = &points[0];
        let _ = write!(&mut commands, "M {:.2} {:.2}", first.x, first.y);

        let mut depth_sum = first.depth;
        for point in points.iter().skip(1) {
            let _ = write!(&mut commands, " L {:.2} {:.2}", point.x, point.y);
            depth_sum += point.depth;
        }

        paths.push(GraphPath {
            commands: commands.into(),
            depth: depth_sum / points.len() as f32,
            series: first.series,
        });
    }

    paths.sort_by(|left, right| left.depth.total_cmp(&right.depth));
    paths
}
