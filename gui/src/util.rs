use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use domain::models::elements::{ElementSchema, FieldSpec, ValueType};
use domain::utility::calculation::LineCalculationResult;

use crate::{FileEntry, GraphAxis, GraphPath, GraphPoint, KeyData};

#[derive(Clone)]
struct ProjectedGraphPoint {
    x: f32,
    y: f32,
    size: f32,
    depth: f32,
    series: i32,
    line_index: usize,
    world_x: f32,
    world_y: f32,
    world_z: f32,
}

#[derive(Clone)]
struct ProjectedPosition {
    x: f32,
    y: f32,
    depth: f32,
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
    yaw_degrees: f32,
    pitch_degrees: f32,
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

    let origin_x = 370.0f32;
    let origin_y = 240.0f32;
    let yaw = yaw_degrees.to_radians();
    let pitch = pitch_degrees.to_radians();
    let sin_yaw = yaw.sin();
    let cos_yaw = yaw.cos();
    let sin_pitch = pitch.sin();
    let cos_pitch = pitch.cos();

    let project_position = |world_x: f32, world_y: f32, world_z: f32| {
        let rotated_x = world_x * cos_yaw + world_z * sin_yaw;
        let rotated_z = -world_x * sin_yaw + world_z * cos_yaw;
        let rotated_y = world_y * cos_pitch - rotated_z * sin_pitch;
        let camera_z = world_y * sin_pitch + rotated_z * cos_pitch;
        let perspective = 1.0 / (camera_z + 3.2);
        ProjectedPosition {
            x: origin_x + rotated_x * 265.0 * perspective,
            y: origin_y - rotated_y * 265.0 * perspective,
            depth: camera_z + 1.5,
        }
    };

    let mut points: Vec<ProjectedGraphPoint> = flattened
        .drain(..)
        .map(|(list_idx, result)| {
            let world_x = result.x as f32;
            let world_y = result.y as f32;
            let world_z = result.z as f32;

            let projected = project_position(world_x, world_y, world_z);

            ProjectedGraphPoint {
                x: projected.x,
                y: projected.y,
                size: 4.0,
                depth: projected.depth,
                series: list_idx as i32,
                line_index: result.line_index,
                world_x,
                world_y,
                world_z,
            }
        })
        .collect();

    points.sort_by(|left, right| left.depth.total_cmp(&right.depth));
    points
}

pub fn project_graph_axes(yaw_degrees: f32, pitch_degrees: f32) -> Vec<GraphAxis> {
    let origin_x = 370.0f32;
    let origin_y = 240.0f32;
    let yaw = yaw_degrees.to_radians();
    let pitch = pitch_degrees.to_radians();
    let sin_yaw = yaw.sin();
    let cos_yaw = yaw.cos();
    let sin_pitch = pitch.sin();
    let cos_pitch = pitch.cos();

    let project_position = |world_x: f32, world_y: f32, world_z: f32| {
        let rotated_x = world_x * cos_yaw + world_z * sin_yaw;
        let rotated_z = -world_x * sin_yaw + world_z * cos_yaw;
        let rotated_y = world_y * cos_pitch - rotated_z * sin_pitch;
        let camera_z = world_y * sin_pitch + rotated_z * cos_pitch;
        let perspective = 1.0 / (camera_z + 3.2);

        ProjectedPosition {
            x: origin_x + rotated_x * 265.0 * perspective,
            y: origin_y - rotated_y * 265.0 * perspective,
            depth: camera_z + 1.5,
        }
    };

    let axes = [
        (0, "X", (0.0f32, 0.0f32, 0.0f32), (1.15f32, 0.0f32, 0.0f32)),
        (1, "Y", (0.0f32, 0.0f32, 0.0f32), (0.0f32, 1.15f32, 0.0f32)),
        (2, "Z", (0.0f32, 0.0f32, 0.0f32), (0.0f32, 0.0f32, 1.15f32)),
    ];

    let mut projected_axes: Vec<GraphAxis> = axes
        .into_iter()
        .map(|(axis, label, start, end)| {
            let start = project_position(start.0, start.1, start.2);
            let end = project_position(end.0, end.1, end.2);
            let mut commands = String::new();
            let _ = write!(
                &mut commands,
                "M {:.2} {:.2} L {:.2} {:.2}",
                start.x, start.y, end.x, end.y
            );

            GraphAxis {
                commands: commands.into(),
                label: label.into(),
                label_x: end.x + 8.0,
                label_y: end.y - 8.0,
                depth: (start.depth + end.depth) / 2.0,
                axis,
            }
        })
        .collect();

    projected_axes.sort_by(|left, right| left.depth.total_cmp(&right.depth));
    projected_axes
}

pub fn project_graph_points(
    results: &[Vec<Option<LineCalculationResult>>],
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Vec<GraphPoint> {
    let projected_points = project_graph_geometry(results, yaw_degrees, pitch_degrees);

    if !projected_points.is_empty() {
        eprintln!(
            "[gui-graph] displayed points (count={} yaw={:.1} pitch={:.1})",
            projected_points.len(),
            yaw_degrees,
            pitch_degrees
        );
        for point in &projected_points {
            eprintln!(
                "[gui-graph] series={} line={} world=({:.3}, {:.3}, {:.3}) screen=({:.2}, {:.2}) depth={:.3}",
                point.series,
                point.line_index,
                point.world_x,
                point.world_y,
                point.world_z,
                point.x,
                point.y,
                point.depth
            );
        }
    }

    projected_points
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

pub fn project_graph_paths(
    results: &[Vec<Option<LineCalculationResult>>],
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Vec<GraphPath> {
    let projected = project_graph_geometry(results, yaw_degrees, pitch_degrees);
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
