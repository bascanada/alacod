use bevy::prelude::*;

#[derive(Default, Component, Reflect)]
pub struct PlayerSpawnComponent {
    pub index: usize,
}
