use std::rc::Rc;

use crate::{game::entity::map::player_spawn::PlayerSpawnConfig, generation::{
    context::{AvailableLevel, LevelType, MapGenerationContext, MapGenerationData},
    entity::{door::DoorConfig, location::EntityLocation, window::WindowConfig},
    position::Position,
    room::{ConnectionTo, RoomConnection},
    IMapGeneration, Room, LEVEL_PROPERTIES_SPAWN_NAME,
}};

use bevy_fixed::rng::RollbackRng;
use serde_json::Value;
use utils::map;

// private struct to store data during the map generation
struct Map {
    // index of the last room that we iterate on
    last_generated_room_index: Option<usize>,
    // list of the room i have generated
    rooms: Vec<Room>,
    // index of all the item in the rooms vector that still possively have open connection
    rooms_possible: Vec<usize>,
    // Track the depth/distance from spawn for each room (by index)
    room_depths: Vec<usize>,
}

pub struct BasicMapGeneration {
    context: MapGenerationContext,
    data: MapGenerationData,
    map: Map,
}

impl BasicMapGeneration {
    pub fn create(context: MapGenerationContext) -> Self {
        BasicMapGeneration {
            data: MapGenerationData::default(),
            context,
            map: Map {
                last_generated_room_index: None,
                rooms: vec![],
                rooms_possible: vec![],
                room_depths: vec![],
            },
        }
    }
}

impl BasicMapGeneration {
    fn get_next_room_recursize(&mut self, rng: &mut RollbackRng,) -> Option<(Room, RoomConnection, RoomConnection)> {
        if self.context.config.max_room > 0 && self.map.rooms.len() >= self.context.config.max_room
        {
            println!(
                "max room stopping generation {} {}",
                self.map.rooms.len(),
                self.context.config.max_room
            );
            return None;
        }

        loop {
            if self.map.last_generated_room_index.is_none() {
                println!("no room mark to continue generation");
                return None;
            }

            let previous_room_index = self.map.last_generated_room_index.unwrap();

            let previous_room = self.map.rooms.get_mut(previous_room_index).unwrap();
            let previous_room_def = previous_room.level_def.clone();

            let free_connection_len = previous_room
                .connections
                .iter()
                .filter(|i| i.to.is_none())
                .count();

            if free_connection_len == 0 {
                if let Some(index) = self
                    .map
                    .rooms_possible
                    .iter()
                    .position(|&x| x == previous_room_index)
                {
                    self.map.rooms_possible.remove(index);
                }

                if self.map.rooms_possible.is_empty() {
                    println!("no more room_possible stopping generation");
                    return None;
                }

                self.map.last_generated_room_index = self
                    .map
                    .rooms_possible
                    .get(
                            rng
                            .next_u32_range(0, self.map.rooms_possible.len() as u32)
                            as usize,
                    )
                    .copied();

                continue;
            } else {
                // get the connection def
                let connection_def = {
                    let connection = previous_room
                        .connections
                        .iter()
                        .filter(|i| i.to.is_none())
                        .skip(rng.next_u32_range(0, free_connection_len as u32) as usize)
                        .last()
                        .unwrap();

                    previous_room_def.connections.get(connection.index).unwrap()
                };

                if connection_def.compatiable_levels.is_empty() {
                    previous_room
                        .connections
                        .get_mut(connection_def.index)
                        .unwrap()
                        .to = Some(ConnectionTo::DeadEnd);
                    println!("no compatible levels marking as DeadEnd");
                    continue;
                } else {
                    let compatible_level = connection_def
                        .compatiable_levels
                        .iter()
                        .skip(
                                rng
                                .next_u32_range(0, connection_def.compatiable_levels.len() as u32)
                                as usize,
                        )
                        .last()
                        .unwrap();

                    let compatible_level_def = self
                        .context
                        .available_levels
                        .iter()
                        .find(|l| l.level_id == compatible_level.0)
                        .unwrap();

                    let level_connection = compatible_level_def
                        .connections
                        .get(compatible_level.1)
                        .unwrap();

                    let my_position = previous_room.get_connecting_room_position(
                        connection_def,
                        compatible_level_def,
                        compatible_level.1,
                        &self.context.tile_size,
                    );

                    let mut new_room = Room::create(
                        rng,
                        compatible_level_def.clone(),
                        my_position,
                        map!(LEVEL_PROPERTIES_SPAWN_NAME => Value::Bool(false)),
                    );

                    if new_room.is_outside(&self.context.config) {
                        previous_room
                            .connections
                            .get_mut(connection_def.index)
                            .unwrap()
                            .to = Some(ConnectionTo::OutSide);

                        continue;
                    } else {
                        new_room.set_connection_between(
                            level_connection.index,
                            previous_room,
                            connection_def.index,
                        );

                        let new_room_level_connection = new_room
                            .connections
                            .get(level_connection.index)
                            .unwrap()
                            .clone();

                        return Some((
                            new_room,
                            new_room_level_connection,
                            previous_room
                                .connections
                                .get(connection_def.index)
                                .unwrap()
                                .clone(),
                        ));
                    }
                }
            }
        }
    }
}

impl IMapGeneration for BasicMapGeneration {
    fn get_spawning_room(&mut self, rng: &mut RollbackRng,) -> Room {
        let spawning_levels: Vec<&Rc<AvailableLevel>> = self
            .context
            .available_levels
            .iter()
            .filter(|i| i.level_type == LevelType::Spawn)
            .collect();

        let spawning_room_def = spawning_levels
            .iter()
            .skip(
                    rng
                    .next_u32_range_inclusive(0, (spawning_levels.len() - 1) as u32)
                    as usize,
            )
            .last();

        if spawning_room_def.is_none() {
            panic!("no spawning room found");
        }

        let spawning_room_def = (*spawning_room_def.unwrap()).clone();

        let x: i32 = rng.next_i32_range_inclusive(
            -self.context.config.max_width,
            self.context.config.max_width - spawning_room_def.level_size_p.0,
        );
        let y: i32 = rng.next_i32_range_inclusive(
            -self.context.config.max_heigth,
            self.context.config.max_heigth - spawning_room_def.level_size_p.1,
        );

        let spawning_room_def = Room::create(
                rng,
            spawning_room_def.clone(),
            Position(x, y),
            map!(LEVEL_PROPERTIES_SPAWN_NAME => Value::Bool(true)),
        );
        self.map.rooms.push(spawning_room_def.clone());
        self.map.room_depths.push(0); // Spawn room is at depth 0
        self.map.last_generated_room_index = Some(0);

        spawning_room_def
    }

    fn get_next_room(&mut self, rng: &mut RollbackRng,) -> Option<(Room, RoomConnection, RoomConnection)> {
        let room = self.get_next_room_recursize(rng);
        if let Some(room) = room.as_ref() {
            // Calculate depth: parent room's depth + 1
            let parent_depth = self.map.room_depths[self.map.last_generated_room_index.unwrap()];
            let new_depth = parent_depth + 1;
            
            self.map.rooms.push(room.0.clone());
            self.map.room_depths.push(new_depth);
            let index = self.map.rooms.len() - 1;
            self.map.rooms_possible.push(index);
        }

        room
    }

    fn get_doors(&mut self, _rng: &mut RollbackRng,) -> Vec<(EntityLocation, crate::generation::entity::door::DoorConfig)> {
        // get all my level , get all the doors in each level
        self.map
            .rooms
            .iter()
            .enumerate() // Add enumerate to get the room index
            .flat_map(|(room_index, room)| {
                let room_depth = self.map.room_depths[room_index];
                // Base cost is 750, increases by 250 per depth level
                // Depth 0 (spawn) = 750, Depth 1 = 1000, Depth 2 = 1250, etc.
                let base_cost = 750;
                let cost_per_depth = 250;
                let door_cost = base_cost + (room_depth * cost_per_depth) as i32;
                
                room.entity_locations
                    .doors
                    .iter()
                    .map(move |door_location| {
                        (
                            EntityLocation {
                                position: door_location.position,
                                size: door_location.size,
                                level_iid: room.level_iid.clone(),
                            },
                            DoorConfig {
                                cost: door_cost,
                                // Doors in deeper rooms could be electrified
                                electrify: room_depth > 0,
                            },
                        )
                    })
                    .collect::<Vec<(EntityLocation, crate::generation::entity::door::DoorConfig)>>()
            })
            .collect()
    }

    fn get_windows(
        &mut self,
        rng: &mut RollbackRng,
    ) -> Vec<(
        EntityLocation,
        crate::generation::entity::window::WindowConfig,
    )> {
        self.map
            .rooms
            .iter()
            .flat_map(|x| {
                x.entity_locations
                    .windows
                    .iter()
                    .map(|y| {
                        (
                            EntityLocation {
                                position: y.position,
                                size: y.size,
                                level_iid: x.level_iid.clone(),
                            },
                            WindowConfig {},
                        )
                    })
                    .collect::<Vec<(
                        EntityLocation,
                        crate::generation::entity::window::WindowConfig,
                    )>>()
            })
            .collect()
    }

    fn get_player_spawn(&mut self, rng: &mut RollbackRng,) -> Vec<(EntityLocation, PlayerSpawnConfig)> {
        self.map
            .rooms
            .iter()
            .filter(|x| {
                let property = x.properties.get(LEVEL_PROPERTIES_SPAWN_NAME);
                if let Some(property) = property {
                    if let Value::Bool(b) = property {
                        return *b;
                    }
                }
                false
            })
            .flat_map(|roor| {
                roor.entity_locations
                    .player_spawns
                    .iter()
                    .enumerate()
                    .map(|(i, y)| {
                        (
                            EntityLocation {
                                position: y.position,
                                size: y.size,
                                level_iid: roor.level_iid.clone(),
                            },
                            PlayerSpawnConfig {
                                index: i,
                            },
                        )
                    })
                    .collect::<Vec<(EntityLocation, PlayerSpawnConfig)>>()
            })
            .collect()
    }
}
