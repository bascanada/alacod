pub mod audio;
pub mod camera;
pub mod character;
pub mod collider;
pub mod debug;
pub mod frame;
pub mod global_asset;
pub mod jjrs;
pub mod plugins;
pub mod weapons;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref GAME_SPEED: bevy_fixed::fixed_math::Fixed = bevy_fixed::fixed_math::new(60.);
}
