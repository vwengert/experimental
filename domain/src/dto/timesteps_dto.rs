use crate::models::unified_model::{Position, UnifiedModel, UnifiedObject};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Timestep {
    pub timestep: u32,
    pub num_objects: usize,
    pub own: Own,
    pub objects: Vec<Object>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Own {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Object {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl From<Vec<Timestep>> for UnifiedModel {
    fn from(timesteps: Vec<Timestep>) -> Self {
        let max_objects = timesteps
            .iter()
            .map(|timestep| timestep.num_objects)
            .max()
            .unwrap_or(0) as usize;

        let own_positions: Vec<Position> = timesteps
            .iter()
            .map(|timestep| Position {
                x: timestep.own.x,
                y: timestep.own.y,
                z: timestep.own.z,
                timestamp: timestep.timestep,
            })
            .collect();

        let mut objects: Vec<UnifiedObject> = Vec::with_capacity(max_objects);

        for i in 0..max_objects {
            let positions: Vec<Position> = timesteps
                .iter()
                .filter_map(|timestep| {
                    if i < timestep.objects.len() {
                        Some(Position {
                            x: timestep.objects[i].x,
                            y: timestep.objects[i].y,
                            z: timestep.objects[i].z,
                            timestamp: timestep.timestep,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let name = timesteps
                .iter()
                .find_map(|timestep| timestep.objects.get(i).map(|obj| obj.name.clone()))
                .unwrap_or_else(|| format!("object_{}", i + 1));

            objects.push(UnifiedObject { name, positions });
        }

        UnifiedModel {
            own: UnifiedObject {
                name: timesteps[0].own.name.clone(),
                positions: own_positions,
            },
            objects,
        }
    }
}

impl From<UnifiedModel> for Vec<Timestep> {
    fn from(model: UnifiedModel) -> Self {
        let mut timesteps: Vec<Timestep> = Vec::new();

        for (i, position) in model.own.positions.iter().enumerate() {
            let objects: Vec<Object> = model
                .objects
                .iter()
                .map(|unified_object| Object {
                    name: unified_object.name.clone(),
                    x: unified_object.positions[i].x,
                    y: unified_object.positions[i].y,
                    z: unified_object.positions[i].z,
                })
                .collect();

            timesteps.push(Timestep {
                timestep: position.timestamp,
                num_objects: model.objects.len() as usize,
                own: Own {
                    name: model.own.name.clone(),
                    x: position.x,
                    y: position.y,
                    z: position.z,
                },
                objects,
            });
        }

        timesteps
    }
}
