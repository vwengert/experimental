use crate::models::unified_model::{Position, UnifiedModel, UnifiedObject};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Timestep {
    pub timestep: f64,
    pub num_objects: usize,
    pub own: Own,
    pub objects: Vec<Object>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
        if timesteps.is_empty() {
            return UnifiedModel {
                own: UnifiedObject {
                    name: "own".to_string(),
                    positions: Vec::new(),
                },
                objects: Vec::new(),
            };
        }

        let max_objects = timesteps
            .iter()
            .map(|timestep| timestep.num_objects)
            .max()
            .unwrap_or(0);

        let own_name = timesteps[0].own.name.clone();

        let own_positions: Vec<Position> = timesteps
            .iter()
            .map(|timestep| Position {
                // Keep the ego axis fixed at the world origin for every timestep.
                x: 0.0,
                y: 0.0,
                z: 0.0,
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
                            x: timestep.objects[i].x - timestep.own.x,
                            y: timestep.objects[i].y - timestep.own.y,
                            z: timestep.objects[i].z - timestep.own.z,
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
                name: own_name,
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
                num_objects: model.objects.len(),
                own: Own {
                    name: model.own.name.clone(),
                    // Keep own fixed at origin in exported timesteps as well.
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                objects,
            });
        }

        timesteps
    }
}

#[cfg(test)]
mod tests {
    use super::{Object, Own, Position, Timestep};
    use crate::models::unified_model::{UnifiedModel, UnifiedObject};

    #[test]
    fn keeps_own_on_origin_and_offsets_objects() {
        let timesteps = vec![Timestep {
            timestep: 1.0,
            num_objects: 1,
            own: Own {
                name: "own".to_string(),
                x: 10.0,
                y: -3.0,
                z: 2.0,
            },
            objects: vec![Object {
                name: "obj".to_string(),
                x: 13.5,
                y: -8.0,
                z: 7.0,
            }],
        }];

        let unified: UnifiedModel = timesteps.into();

        assert_eq!(unified.own.positions.len(), 1);
        assert_eq!(unified.own.positions[0].x, 0.0);
        assert_eq!(unified.own.positions[0].y, 0.0);
        assert_eq!(unified.own.positions[0].z, 0.0);

        assert_eq!(unified.objects.len(), 1);
        assert_eq!(unified.objects[0].positions[0].x, 3.5);
        assert_eq!(unified.objects[0].positions[0].y, -5.0);
        assert_eq!(unified.objects[0].positions[0].z, 5.0);
    }

    #[test]
    fn exports_timesteps_with_own_on_origin() {
        let unified = UnifiedModel {
            own: UnifiedObject {
                name: "own".to_string(),
                positions: vec![Position {
                    x: 12.0,
                    y: 7.0,
                    z: -4.0,
                    timestamp: 2.5,
                }],
            },
            objects: vec![UnifiedObject {
                name: "obj".to_string(),
                positions: vec![Position {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    timestamp: 2.5,
                }],
            }],
        };

        let timesteps: Vec<Timestep> = unified.into();
        assert_eq!(timesteps.len(), 1);
        assert_eq!(timesteps[0].own.x, 0.0);
        assert_eq!(timesteps[0].own.y, 0.0);
        assert_eq!(timesteps[0].own.z, 0.0);
    }
}
