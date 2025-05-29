pub mod ai;
pub mod create;
pub mod spawning;
use bevy::prelude::*;

#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub struct Enemy {}
