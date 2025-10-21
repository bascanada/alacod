use bevy::prelude::*;

use crate::generation::entity::door::DoorConfig;

#[derive(Default, Component, Reflect)]
pub struct DoorComponent {
    pub config: DoorConfig,
}

/// Component to store the grid position and level identifier of a door
/// This is used to match paired doors during interaction
#[derive(Component, Clone, Debug)]
pub struct DoorGridPosition {
    pub level_iid: String,
    pub grid_x: i32,
    pub grid_y: i32,
}
