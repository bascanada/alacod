use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use map::{game::entity::map::{map_rollback::MapRollbackMarker, window::WindowComponent}, generation::entity::window::WindowConfig};

pub fn window_from_field(_: &EntityInstance) -> WindowComponent {
    WindowComponent {
        config: WindowConfig {},
    }
}

#[derive(Bundle, LdtkEntity)]
pub struct WindowBundle {
    #[with(window_from_field)]
    window: WindowComponent,
    rollback_marker: MapRollbackMarker,
    #[sprite_sheet]
    sprite_sheet: Sprite,
}


impl Default for WindowBundle {
    fn default() -> Self {
        Self { rollback_marker: MapRollbackMarker("window".into()), window: WindowComponent::default(), sprite_sheet: Sprite::default()}
    } 
}
