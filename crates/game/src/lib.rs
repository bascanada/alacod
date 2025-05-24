pub mod plugins;
pub mod character;
pub mod jjrs;
pub mod camera;
pub mod frame;
pub mod audio;
pub mod global_asset;
pub mod weapons;
pub mod collider;
pub mod debug;


use lazy_static::lazy_static;

lazy_static! {
    pub static ref GAME_SPEED: utils::fixed_math::Fixed = utils::fixed_math::new(60.);
}