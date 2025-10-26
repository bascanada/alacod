
use bevy::prelude::*;
use bevy_fixed::fixed_math;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct MovementConfig {
    pub acceleration: fixed_math::Fixed,
    pub max_speed: fixed_math::Fixed,
    pub friction: fixed_math::Fixed,
    pub sprint_multiplier: fixed_math::Fixed, // How much faster sprint is (e.g. 2.0 for double speed)
    pub sprint_acceleration_per_frame: fixed_math::Fixed, // How much sprint increases each frame (0-1)
    pub sprint_deceleration_per_frame: fixed_math::Fixed,

    pub dash_distance: fixed_math::Fixed, // Total distance to dash
    pub dash_duration_frames: u32,        // How many frames the dash takes
    pub dash_cooldown_frames: u32,        // Frames before dash can be used again
}

#[derive(Component, Default, Clone)]
pub struct SprintState {
    pub is_sprinting: bool,
    pub sprint_factor: fixed_math::Fixed, // Ranges from 0.0 to 1.0 for gradual acceleration
}

#[derive(Component, Default, Clone)]
pub struct Velocity {
    pub main: fixed_math::FixedVec2,
    pub knockback: fixed_math::FixedVec2,
}


/// Resource for configuring knockback damping
#[derive(Resource, Clone)]
pub struct KnockbackDampingConfig {
    pub damping: f32, // e.g., 0.85 means 15% decay per frame
}

impl Default for KnockbackDampingConfig {
    fn default() -> Self {
        Self { damping: 0.85 }
    }
}

/// System to apply damping to knockback velocity each frame
pub fn apply_knockback_damping(
    mut query: Query<&mut Velocity>,
    config: Res<KnockbackDampingConfig>,
) {
    let damping = fixed_math::new(config.damping);
    for mut velocity in query.iter_mut() {
        velocity.knockback = velocity.knockback * damping;
        // If knockback is very small, zero it out
        if velocity.knockback.length_squared() < fixed_math::new(0.01) {
            velocity.knockback = fixed_math::FixedVec2::ZERO;
        }
    }
}
