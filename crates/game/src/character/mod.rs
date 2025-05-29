pub mod config;
pub mod create;
pub mod dash;
pub mod enemy;
pub mod health;
pub mod movement;
pub mod player;

use bevy::prelude::*;

#[derive(Component, Clone, Copy, Default)]
pub struct Character;
