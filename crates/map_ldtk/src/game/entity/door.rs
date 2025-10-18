use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use map::game::entity::map::map_rollback::MapRollbackMarker;

use crate::map_const;
use map::game::entity::map::door::DoorComponent;
use map::generation::entity::door::DoorConfig;

#[derive(Component, Clone, Copy)]
pub struct LdtkEntitySize {
    pub width: f32,
    pub height: f32,
}

pub fn ldtk_entity_size_from_instance(entity_instance: &EntityInstance) -> LdtkEntitySize {
    LdtkEntitySize {
        width: entity_instance.width as f32,
        height: entity_instance.height as f32,
    }
}

pub fn door_component_from_field(entity_instance: &EntityInstance) -> DoorComponent {
    DoorComponent {
        config: DoorConfig {
            electrify: *entity_instance
                .get_bool_field(map_const::FIELD_ELECTRIFY_NAME)
                .unwrap(),
            cost: *entity_instance
                .get_int_field(map_const::FIELD_PRICE_NAME)
                .unwrap(),
        },
    }
}

#[derive(Bundle, LdtkEntity)]
pub struct DoorBundle {
    #[with(door_component_from_field)]
    door: DoorComponent,
    #[sprite_sheet]
    sprite_sheet: Sprite,
    rollback_marker: MapRollbackMarker,
    visibility: Visibility,
    #[with(ldtk_entity_size_from_instance)]
    ldtk_size: LdtkEntitySize,
}

impl Default for DoorBundle {
    fn default() -> Self {
        Self { 
            rollback_marker: MapRollbackMarker("door".into()), 
            door: DoorComponent::default(), 
            sprite_sheet: Sprite::default(), 
            visibility: Visibility::default(),
            ldtk_size: LdtkEntitySize { width: 64.0, height: 32.0 },
        }
    } 
}
