use bevy::prelude::*;

#[derive(Default, Component, Reflect)]
pub struct PlayerSpawnConfig {
    pub index: usize,
}
