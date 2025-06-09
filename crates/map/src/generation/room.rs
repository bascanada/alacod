use std::{collections::HashMap, rc::Rc};

use bevy_fixed::rng::RollbackRng;
use serde_json::Value;

use super::{
    config::MapGenerationConfig,
    context::{AvailableLevel, Connection, Side},
    entity::location::EntityLocations,
    position::Position,
};

#[derive(Debug, Clone, Default)]
pub enum ConnectionTo {
    Room((String, usize)),
    #[default]
    DeadEnd,
    OutSide,
}

#[derive(Debug, Clone)]
pub struct RoomConnection {
    pub index: usize,
    pub level_iid: String,
    pub level_id: String,
    pub side: Side,
    pub to: Option<ConnectionTo>,
}

/*
 * Room is an instance of a level in a map that is being generated
 */
#[derive(Debug, Clone)]
pub struct Room {
    pub level_iid: String,
    pub position: Position,
    pub connections: Vec<RoomConnection>,
    pub entity_locations: EntityLocations,

    pub level_def: Rc<AvailableLevel>,

    pub properties: HashMap<String, Value>,
}

impl Room {
    pub fn create(
        rng: &mut RollbackRng,
        level: Rc<AvailableLevel>,
        position: Position,
        properties: HashMap<String, Value>,
    ) -> Self {
        let level_iid: String = rng.next_uuid();

        let connections: Vec<_> = level
            .connections
            .iter()
            .map(|x| RoomConnection {
                index: x.index,
                to: None,
                level_id: x.level_id.clone(),
                level_iid: level_iid.clone(),
                side: x.side,
            })
            .collect();

        Self {
            level_iid,
            position,
            entity_locations: level.entity_locations.clone(),
            connections,
            properties,
            level_def: level,
        }
    }

    fn set_connection(
        &mut self,
        my_connection_index: usize,
        their_room: &mut Room,
        their_connection_index: usize,
    ) {
        let my_connection = self.connections.get_mut(my_connection_index).unwrap();
        if my_connection.to.is_some() {
            panic!("connection is already used");
        }
        my_connection.to = Some(ConnectionTo::Room((
            their_room.level_iid.clone(),
            their_connection_index,
        )));
    }

    pub fn set_connection_between(
        &mut self,
        my_connection_index: usize,
        their_room: &mut Room,
        their_connection_index: usize,
    ) {
        // throw if one is already link
        self.set_connection(my_connection_index, their_room, their_connection_index);
        their_room.set_connection(their_connection_index, self, my_connection_index);
    }

    pub fn is_overlapping(&self, other: &Room) -> bool {
        // find if we are overlapping
        let left_of_other = self.position.0 + self.level_def.level_size_p.0 < other.position.0;
        let left_of_self = other.position.0 + other.level_def.level_size_p.0 < self.position.0;

        // Check if one square is above the other
        let above_other = self.position.1 + self.level_def.level_size_p.1 < other.position.1;
        let above_self = other.position.1 + other.level_def.level_size_p.1 < self.position.1;

        // If neither square is to the left or above the other, they overlap
        !(left_of_other || left_of_self || above_other || above_self)
    }

    // check if top-left corner is outside or not
    pub fn is_outside(&self, config: &MapGenerationConfig) -> bool {
        let position = &self.position;

        (position.0 > config.max_width || position.0 < -config.max_width)
            || (position.1 > config.max_heigth || position.1 < -config.max_heigth)
    }

    pub fn get_connecting_room_position(
        &self,
        my_connection: &Connection,
        their_level: &AvailableLevel,
        their_connection: usize,
        tile_size: &(i32, i32),
    ) -> Position {
        let my_position = &self.position;

        let their_connection = their_level.connections.get(their_connection).unwrap();

        let offset = my_connection.starting_at - their_connection.starting_at;

        // calculate the pixel offset
        match my_connection.side {
            Side::N | Side::S => {
                let offset_pixel = (offset as i32) * tile_size.0;

                Position(
                    my_position.0 + (their_connection.side.get_factor() * (offset_pixel)),
                    my_position.1
                        + (-their_connection.side.get_factor() * their_level.level_size_p.1),
                )
            }
            Side::W | Side::E => {
                let offset_pixel = (offset as i32) * tile_size.1;

                Position(
                    my_position.0
                        + (-their_connection.side.get_factor() * their_level.level_size_p.0),
                    my_position.1 + (their_connection.side.get_factor() * (offset_pixel)),
                )
            }
        }
    }
}
