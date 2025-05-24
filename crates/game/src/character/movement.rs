use animation::AnimationState;
use bevy::{prelude::*};
use bevy_ggrs::Rollback;
use serde::Deserialize;
use utils::fixed_math;

use crate::collider::{is_colliding, Collider, CollisionLayer, CollisionSettings, Wall};

use super::{config::{CharacterConfig, CharacterConfigHandles}, Character};

#[derive(Deserialize, Debug, Clone)]
pub struct MovementConfig {
    pub acceleration: fixed_math::Fixed,
    pub max_speed: fixed_math::Fixed,
    pub friction: fixed_math::Fixed,
    pub sprint_multiplier: fixed_math::Fixed,         // How much faster sprint is (e.g. 2.0 for double speed)
    pub sprint_acceleration_per_frame: fixed_math::Fixed, // How much sprint increases each frame (0-1)
    pub sprint_deceleration_per_frame: fixed_math::Fixed,

    pub dash_distance: fixed_math::Fixed,         // Total distance to dash
    pub dash_duration_frames: u32,  // How many frames the dash takes
    pub dash_cooldown_frames: u32,  // Frames before dash can be used again
}

#[derive(Component, Default, Clone)]
pub struct SprintState {
    pub is_sprinting: bool,
    pub sprint_factor: fixed_math::Fixed,  // Ranges from 0.0 to 1.0 for gradual acceleration
}

#[derive(Component, Default, Deref, DerefMut, Clone)]
pub struct Velocity(pub fixed_math::FixedVec2);
