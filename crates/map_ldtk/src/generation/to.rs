use std::rc::Rc;

use bevy::math::{IVec2, Vec2};
use bevy_ecs_ldtk::{
    ldtk::{FieldInstance, FieldValue, LdtkJson, Level, NeighbourLevel, RealEditorValue},
    EntityInstance,
};
use serde_json::Value;
use uuid::Uuid;

use map::{game::entity::map::player_spawn::PlayerSpawnConfig, generation::{
    entity::{door::DoorConfig, location::EntityLocation, window::WindowConfig},
    room::{Room, RoomConnection},
    IMapGenerator,
}};

use crate::map_const::{self, LAYER_ENTITY};

#[derive(Debug, Clone)]
pub struct GeneratedRoom {
    level: Level,
    ldtk: Rc<LdtkJson>,
}

impl GeneratedRoom {
    pub fn create(ldtk_json: Rc<LdtkJson>, room: &Room) -> Self {
        let mut level = ldtk_json
            .levels
            .iter()
            .find(|item| item.identifier == room.level_def.level_id)
            .expect("failed to find level from original")
            .clone();

        level.iid = room.level_iid.clone();
        level.identifier = room.level_iid.clone();
        level.world_x = room.position.0;
        level.world_y = room.position.1;
        level.neighbours.clear();

        if !room.properties.is_empty() {
            for field in level.field_instances.iter_mut() {
                if let Some(value) = room.properties.get(field.identifier.as_str()) {
                    field.value = match value {
                        Value::Bool(b_value) => FieldValue::Bool(*b_value),
                        _ => FieldValue::String(None),
                    };
                    field.real_editor_values = vec![Some(RealEditorValue {
                        // TODO: do the correct mapping
                        id: "V_Bool".to_string(),
                        params: vec![value.clone()],
                    })]
                }
            }
        }
        level
            .layer_instances
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|x| x.identifier == LAYER_ENTITY)
            .unwrap()
            .entity_instances
            .clear();

        GeneratedRoom {
            level,
            ldtk: ldtk_json,
        }
    }
}

pub struct GeneratedMap {
    pub ldtk_json: Rc<LdtkJson>,
    pub generated_rooms: Vec<GeneratedRoom>,
}

pub fn _add_property_entity() {}

pub fn get_new_entity(
    room: &GeneratedRoom,
    original_entity_identifier: &str,
    location: &EntityLocation,
    tile_size: (i32, i32),
    // identifier and value
    fields: Vec<(&str, FieldValue)>,
) -> EntityInstance {
    let entity = room
        .ldtk
        .defs
        .entities
        .iter()
        .find(|x| x.identifier == original_entity_identifier)
        .unwrap();

    let px = (
        location.position.0 * tile_size.0,
        location.position.1 * tile_size.1,
    );
    let world_px = (px.0 + room.level.world_x, px.1 + room.level.world_y);

    let identifiers = fields
        .iter()
        .map(|x| {
            let field = entity
                .field_defs
                .iter()
                .find(|fd| fd.identifier == x.0)
                .unwrap_or_else(|| panic!("failed to get field for entity {}", x.0));

            let real_editor_value = match x.1.clone() {
                FieldValue::Int(v) => Some(("V_Int", serde_json::to_value(v).unwrap())),
                FieldValue::Bool(v) => Some(("V_Bool", serde_json::to_value(v).unwrap())),
                _ => None,
            };

            let real_editor_value = real_editor_value.map(|v| RealEditorValue {
                id: v.0.to_string(),
                params: vec![v.1],
            });

            FieldInstance {
                identifier: field.identifier.clone(),
                def_uid: field.uid,
                field_instance_type: field.field_definition_type.clone(),
                value: x.1.clone(),
                tile: None,
                real_editor_values: vec![real_editor_value],
            }
        })
        .collect();

    EntityInstance {
        identifier: original_entity_identifier.into(),
        def_uid: entity.uid,
        grid: IVec2::new(location.position.0, location.position.1),
        pivot: Vec2::new(entity.pivot_x, entity.pivot_y),
        tags: vec![],
        tile: entity.tile_rect,
        smart_color: entity.color,
        iid: Uuid::new_v4().to_string(),
        width: location.size.0 * tile_size.0,
        height: location.size.1 * tile_size.1,
        field_instances: identifiers,
        px: IVec2::new(px.0, px.1),
        world_x: Some(world_px.0),
        world_y: Some(world_px.1),
    }
}

impl GeneratedMap {
    pub fn create(ldtk_json: LdtkJson) -> Self {
        GeneratedMap {
            ldtk_json: Rc::new(ldtk_json),
            generated_rooms: vec![],
        }
    }

    pub fn get_generated_map(&self) -> LdtkJson {
        let mut new_map: LdtkJson = (*self.ldtk_json).clone();

        new_map.levels = self
            .generated_rooms
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let mut r = x.level.clone();
                r.identifier = format!("Level_{}", i);
                r
            })
            .collect();

        new_map
    }

    fn add_entity_to_level(
        &mut self,
        location: &EntityLocation,
        entity_type: &str,
        fields: Vec<(&str, FieldValue)>,
    ) {
        let level = self
            .generated_rooms
            .iter_mut()
            .find(|x| x.level.iid == location.level_iid)
            .unwrap();

        let new_entity = get_new_entity(
            level,
            entity_type,
            location,
            (
                self.ldtk_json.default_entity_width,
                self.ldtk_json.default_entity_height,
            ),
            fields,
        );

        level
            .level
            .layer_instances
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|x| x.identifier == map_const::LAYER_ENTITY)
            .unwrap()
            .entity_instances
            .push(new_entity);
    }
}

impl IMapGenerator for GeneratedMap {
    fn add_room(
        &mut self,
        room: &Room,
        connection_used: Option<&RoomConnection>,
        connected_to: Option<&RoomConnection>,
    ) {
        let mut generated_room = GeneratedRoom::create(self.ldtk_json.clone(), room);

        println!(
            "adding room id={} type={:?} from_level={} position={} \n property={:?}",
            room.level_iid,
            room.level_def.level_type,
            room.level_def.level_id,
            room.position,
            room.properties
        );

        if let Some(connected_to) = connected_to {
            let connection_used = connection_used.unwrap();

            generated_room.level.neighbours.push(NeighbourLevel {
                level_iid: connected_to.level_iid.clone(),
                dir: connection_used.side.to_dir_str().into(),
                ..Default::default()
            });

            // find the other room and me as it's neighbours
            let linked_room = self
                .generated_rooms
                .iter_mut()
                .find(|r| r.level.iid == connected_to.level_iid)
                .unwrap();

            println!("  connecting my side={:?} index={} with side={:?} index={} of room id={} from_level={} position={}x{}",
               connection_used.side, connection_used.index, connected_to.side, connected_to.index, connected_to.level_iid,
               connected_to.level_id, linked_room.level.world_x, linked_room.level.world_y,
            );

            linked_room.level.neighbours.push(NeighbourLevel {
                dir: connected_to.side.to_dir_str().into(),
                level_iid: room.level_iid.clone(),
                ..Default::default()
            })
        }

        println!();

        self.generated_rooms.push(generated_room);
    }

    fn add_doors(&mut self, doors: &Vec<(EntityLocation, DoorConfig)>) {
        for (location, door) in doors.iter() {
            self.add_entity_to_level(
                location,
                map_const::ENTITY_DOOR_LOCATION,
                vec![
                    (
                        map_const::FIELD_PRICE_NAME,
                        FieldValue::Int(Some(door.cost)),
                    ),
                    (
                        map_const::FIELD_ELECTRIFY_NAME,
                        FieldValue::Bool(door.electrify),
                    ),
                ],
            );
        }
    }

    fn add_windows(&mut self, windows: &Vec<(EntityLocation, WindowConfig)>) {
        for (location, _) in windows.iter() {
            self.add_entity_to_level(location, map_const::ENTITY_WINDOW_LOCATION, vec![]);
        }
    }

    fn add_player_spawns(&mut self, player_spawns: &Vec<(EntityLocation, PlayerSpawnConfig)>) {
        for (location, spawn) in player_spawns.iter() {
            self.add_entity_to_level(location, map_const::ENTITY_PLAYER_SPAWN_LOCATION, vec![
                (
                    map_const::FIELD_PLAYER_SPAWN_INDEX_NAME,
                    FieldValue::Int(Some(spawn.index as i32))
                )
            ]);
        }
    }
}
