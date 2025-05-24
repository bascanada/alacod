use std::rc::Rc;

use crate::generation::{
    context::{AvailableLevel, LevelType, MapGenerationContext, MapGenerationData},
    entity::{door::DoorConfig, location::EntityLocation, window::WindowConfig},
    position::Position,
    room::{ConnectionTo, RoomConnection},
    IMapGeneration, Room, LEVEL_PROPERTIES_SPAWN_NAME,
};

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
}

pub struct BasicMapGeneration {
    context: MapGenerationContext,
    data: MapGenerationData,

    map: Map,
}

impl BasicMapGeneration {
    pub fn create(context: MapGenerationContext) -> Self {
        BasicMapGeneration {
            data: MapGenerationData::from_context(&context),
            context,
            map: Map {
                last_generated_room_index: None,
                rooms: vec![],
                rooms_possible: vec![],
            },
        }
    }
}

impl BasicMapGeneration {
    fn get_next_room_recursize(&mut self) -> Option<(Room, RoomConnection, RoomConnection)> {
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

                if self.map.rooms_possible.len() == 0 {
                    println!("no more room_possible stopping generation");
                    return None;
                }

                self.map.last_generated_room_index = self
                    .map
                    .rooms_possible
                    .get(
                        self.data
                            .rng
                            .next_u32_range(0, self.map.rooms_possible.len() as u32) as usize,
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
                        .skip(self.data.rng.next_u32_range(0, free_connection_len as u32) as usize)
                        .last()
                        .unwrap();

                    previous_room_def.connections.get(connection.index).unwrap()
                };

                if connection_def.compatiable_levels.len() == 0 {
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
                            self.data.rng.next_u32_range(0, connection_def.compatiable_levels.len() as u32) as usize
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
                        &connection_def,
                        &compatible_level_def,
                        compatible_level.1,
                        &self.context.tile_size,
                    );

                    let mut new_room = Room::create(
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
    fn get_spawning_room(&mut self) -> Room {
        let spawning_levels: Vec<&Rc<AvailableLevel>> = self
            .context
            .available_levels
            .iter()
            .filter(|i| i.level_type == LevelType::Spawn)
            .collect();

        let spawning_room_def = spawning_levels
            .iter()
            .skip(self.data.rng.next_u32_range_inclusive(0, (spawning_levels.len() -1) as u32) as usize)
            .last();

        if spawning_room_def.is_none() {
            panic!("no spawning room found");
        }

        let spawning_room_def = (*spawning_room_def.unwrap()).clone();

        let x: i32 = self.data.rng.next_i32_range_inclusive(-self.context.config.max_width, self.context.config.max_width - spawning_room_def.level_size_p.0);
        let y: i32 = self.data.rng.next_i32_range_inclusive(-self.context.config.max_heigth, self.context.config.max_heigth - spawning_room_def.level_size_p.1);

        let spawning_room_def = Room::create(
            spawning_room_def.clone(),
            Position(x, y),
            map!(LEVEL_PROPERTIES_SPAWN_NAME => Value::Bool(true)),
        );
        self.map.rooms.push(spawning_room_def.clone());
        self.map.last_generated_room_index = Some(0);

        spawning_room_def
    }

    fn get_next_room(&mut self) -> Option<(Room, RoomConnection, RoomConnection)> {
        let room = self.get_next_room_recursize();
        if let Some(room) = room.as_ref() {
            self.map.rooms.push(room.0.clone());
            let index = self.map.rooms.len() - 1;
            self.map.rooms_possible.push(index);
        }

        room
    }

    fn get_doors(&mut self) -> Vec<(EntityLocation, crate::generation::entity::door::DoorConfig)> {
        // get all my level , get all the doors in each level
        self.map
            .rooms
            .iter()
            .flat_map(|x| {
                x.entity_locations
                    .doors
                    .iter()
                    .map(|y| {
                        (
                            EntityLocation {
                                position: y.position,
                                size: y.size,
                                level_iid: x.level_iid.clone(),
                            },
                            DoorConfig {
                                // TODO: create the algo to fix the price of door
                                cost: 1000,
                                // TODO: create the algo to found the electrify rule
                                electrify: true,
                            },
                        )
                    })
                    .collect::<Vec<(EntityLocation, crate::generation::entity::door::DoorConfig)>>()
            })
            .collect()
    }

    fn get_windows(
        &mut self,
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

    fn get_player_spawn(&mut self) -> Vec<(EntityLocation, ())> {
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
                return false;
            })
            .flat_map(|roor| {
                roor.entity_locations
                    .player_spawns
                    .iter()
                    .map(|y| {
                        (
                            EntityLocation {
                                position: y.position,
                                size: y.size,
                                level_iid: roor.level_iid.clone(),
                            },
                            (),
                        )
                    })
                    .collect::<Vec<(EntityLocation, ())>>()
            })
            .collect()
    }
}
