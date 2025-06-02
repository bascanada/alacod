use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use map::{game::entity::map::{map_rollback::MapRollbackMarker, window::WindowComponent}, generation::entity::window::WindowConfig};

pub fn window_from_field(_: &EntityInstance) -> WindowComponent {
    WindowComponent {
        config: WindowConfig {},
    }
}

#[derive(Default, Bundle, LdtkEntity)]
pub struct WindowBundle {
    #[with(window_from_field)]
    door: WindowComponent,
    rollback_marker: MapRollbackMarker,
    #[sprite_sheet]
    sprite_sheet: Sprite,
}
