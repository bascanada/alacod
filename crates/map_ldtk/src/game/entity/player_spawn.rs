use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use map::game::entity::map::player_spawn::PlayerSpawnComponent;

use crate::map_const;


pub fn player_spawn_component_from_field(entity_instance: &EntityInstance) -> PlayerSpawnComponent {
    PlayerSpawnComponent {
        index: *entity_instance
                .get_int_field(map_const::FIELD_PLAYER_SPAWN_INDEX_NAME)
                .unwrap() as usize,
    }
}

#[derive(Default, Bundle, LdtkEntity)]
pub struct PlayerSpawnBundle {
    player_spawn: PlayerSpawnComponent,
    #[sprite_sheet]
    sprite_sheet: Sprite,
}
