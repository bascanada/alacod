use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use map::game::entity::map::door::DoorComponent;
use map::generation::entity::door::DoorConfig;
use crate::map_const;

pub fn door_component_from_field(entity_instance: &EntityInstance) -> DoorComponent {
    println!("FROM FIELD {:?}", entity_instance);
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

#[derive(Default, Bundle, LdtkEntity)]
pub struct DoorBundle {
    #[with(door_component_from_field)]
    door: DoorComponent,
    #[sprite_sheet]
    sprite_sheet: Sprite,

    visibility: Visibility,
}
