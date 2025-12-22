pub mod control;
pub mod create;
pub mod input;
pub mod jjrs;

use bevy::prelude::*;
use ggrs::PlayerHandle;

#[derive(Component, Reflect, Default, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct LocalPlayer {}

#[derive(Component, Reflect, Default, Debug, Clone)]
#[reflect(Component)]
pub struct Player {
    pub handle: PlayerHandle,
    pub color: Color,
    pub name: String,
    pub pubkey: String,
}
