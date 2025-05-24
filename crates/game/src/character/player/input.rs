
use animation::{ActiveLayers, FacingDirection};
use animation::{AnimationState, CharacterAnimationHandles};
use bevy::window::PrimaryWindow;
use bevy::{prelude::*, time::Time, utils::HashMap};
use leafwing_input_manager::prelude::*;
use bevy_ggrs::prelude::*;
use bevy_ggrs::LocalInputs;
use serde::{Serialize, Deserialize};
use utils::fixed_math; 

use crate::character::config::{CharacterConfig, CharacterConfigHandles};
use crate::character::dash::DashState;
use crate::character::movement::{MovementConfig, SprintState, Velocity};
use crate::character::player::{control::PlayerAction, Player};
use crate::collider::{is_colliding, Collider, CollisionLayer, CollisionSettings, Wall};
use crate::frame::FrameCount;
use crate::weapons::WeaponInventory;

use super::jjrs::PeerConfig;
use super::LocalPlayer;

pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0; // 60 FPS fixed timestep


const INPUT_UP: u16 = 1 << 0;
const INPUT_DOWN: u16 = 1 << 1;
const INPUT_LEFT: u16 = 1 << 2;
const INPUT_RIGHT: u16 = 1 << 3;
pub const INPUT_RELOAD: u16 = 1 << 4;
pub const INPUT_SWITCH_WEAPON_MODE: u16 = 1 << 5;
pub const INPUT_SPRINT: u16 = 1 << 6;
pub const INPUT_DASH: u16 = 1 << 7;
pub const INPUT_MODIFIER: u16 = 1 << 8;

const PAN_FACING_THRESHOLD: i16 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BoxInput{
    pub buttons: u16,
    pub pan_x: i16,
    pub pan_y: i16,

    pub fire: bool,
    pub switch_weapon: bool,
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct PointerWorldPosition(pub Vec2);

/// Component for the weapon sprite's position relative to player
#[derive(Component, Clone, Copy, Default)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32
}


fn get_facing_direction(input: &BoxInput) -> FacingDirection {

    if input.pan_x > PAN_FACING_THRESHOLD {
        FacingDirection::Right
    } else if input.pan_x < -PAN_FACING_THRESHOLD {
        FacingDirection::Left
    } else {
        if input.buttons & INPUT_RIGHT != 0 {
            FacingDirection::Right
        } else if input.buttons & INPUT_LEFT != 0 {
            FacingDirection::Left
        } else {
            FacingDirection::Right
        }
    }
}

pub fn read_local_inputs(
    mut commands: Commands,
    players: Query<(&ActionState<PlayerAction>, &Transform, &Player), With<LocalPlayer>>,
    
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) {

    let mut local_inputs = HashMap::new();

    for (action_state, transform, player) in players.iter() {
        let mut input = BoxInput::default();

         if action_state.pressed(&PlayerAction::MoveUp) {
            input.buttons |= INPUT_UP;
         }
         if action_state.pressed(&PlayerAction::MoveDown) {
            input.buttons |= INPUT_DOWN;
         }
         if action_state.pressed(&PlayerAction::MoveLeft) {
            input.buttons |= INPUT_LEFT;
         }
         if action_state.pressed(&PlayerAction::MoveRight) {
            input.buttons |= INPUT_RIGHT;
         }

         if action_state.pressed(&PlayerAction::PointerClick) {
            input.fire = true;
         }

         if action_state.pressed(&PlayerAction::SwitchWeapon) {
            input.switch_weapon = true;
         }
         if action_state.pressed(&PlayerAction::SwitchWeaponMode) {
            input.buttons |= INPUT_SWITCH_WEAPON_MODE;
         }

         if action_state.pressed(&PlayerAction::Reload) {
            input.buttons |= INPUT_RELOAD;
         }

         if action_state.pressed(&PlayerAction::Sprint) {
            input.buttons |= INPUT_SPRINT;
         }

        if action_state.pressed(&PlayerAction::Dash) {
            input.buttons |= INPUT_DASH;
        }

        if action_state.pressed(&PlayerAction::Modifier) {
            input.buttons |= INPUT_MODIFIER;
        }


        if let Ok(window) = q_window.get_single() {
            if let Ok((camera, camera_transform)) = q_camera.get_single() {
                if let Some(cursor_position) = window.cursor_position() {
                    if let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
                        let player_position = transform.translation.truncate();
                        let pointer_distance = world_position - player_position;

                        input.pan_x = (pointer_distance.x).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                        input.pan_y = (pointer_distance.y).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    }
                }
            }
        }

        local_inputs.insert(player.handle, input);
    }

    commands.insert_resource(LocalInputs::<PeerConfig>(local_inputs));
}

pub fn apply_inputs(
    mut commands: Commands,
    inputs: Res<PlayerInputs<PeerConfig>>,
    character_configs: Res<Assets<CharacterConfig>>,
    mut query: Query<(Entity, &WeaponInventory, &mut fixed_math::FixedTransform3D, &mut DashState, &mut Velocity, &mut ActiveLayers, &mut FacingDirection, &mut CursorPosition, &mut SprintState, &CharacterConfigHandles, &Player), With<Rollback>>,
) {
    for (entity, inventory, mut transform, mut dash_state, mut velocity, mut active_layers, mut facing_direction, mut cursor_position, mut sprint_state, config_handles, player) in query.iter_mut() {
        if let Some(config) = character_configs.get(&config_handles.config) {
            let (input, _input_status) = inputs[player.handle];
            
            dash_state.update();
            
            // If currently dashing, directly update position
            if dash_state.is_dashing {
                // Calculate position based on remaining frames and distance
                let completed_fraction = fixed_math::FIXED_ONE - (fixed_math::new(dash_state.dash_frames_remaining as f32) / 
                                              fixed_math::new((config.movement.dash_duration_frames as f32)));
                
                let dash_offset = dash_state.dash_direction * dash_state.dash_total_distance * completed_fraction;
                transform.translation = dash_state.dash_start_position + fixed_math::FixedVec3::new(dash_offset.x, dash_offset.y, fixed_math::new(0.0));
                
                // Zero out velocity while dashing to prevent normal movement physics
                velocity.0 = fixed_math::FixedVec2::ZERO;
                continue;
            }
            
            // Check if player is trying to dash
            if (input.buttons & INPUT_DASH != 0) && dash_state.can_dash() {
                // Get looking direction for dash
                let look_direction = fixed_math::FixedVec2::new(fixed_math::new(input.pan_x as f32), fixed_math::new(input.pan_y as f32));

                let is_reverse_dash = (input.buttons & INPUT_MODIFIER) != 0;
                
                // If the player isn't aiming, use facing direction
                let mut dash_direction = if look_direction.length_squared() > 1.0 {
                    look_direction.normalize()
                } else {
                    fixed_math::FixedVec2::new(fixed_math::new(facing_direction.to_int() as f32), fixed_math::new(0.0))
                };

                if is_reverse_dash {
                    dash_direction = -dash_direction;
                }
                
                // Start dash with current position
                dash_state.start_dash(
                    dash_direction, 
                    transform.translation, 
                    config.movement.dash_distance,
                    config.movement.dash_duration_frames
                );
                dash_state.set_cooldown(config.movement.dash_cooldown_frames);
                
                // Zero out velocity to prevent normal movement physics
                velocity.0 = fixed_math::FixedVec2::ZERO;
                continue;
            }

            let is_sprinting = input.buttons & INPUT_SPRINT != 0;
            sprint_state.is_sprinting = is_sprinting;
            
            if is_sprinting {
                sprint_state.sprint_factor += config.movement.sprint_acceleration_per_frame;
                sprint_state.sprint_factor = sprint_state.sprint_factor.min(fixed_math::FIXED_ZERO);
            } else {
                sprint_state.sprint_factor -= config.movement.sprint_deceleration_per_frame;
                sprint_state.sprint_factor = sprint_state.sprint_factor.max(fixed_math::FIXED_ZERO);
            }

            let mut direction = fixed_math::FixedVec2::ZERO;
            if input.buttons & INPUT_UP != 0    { direction.y += fixed_math::FIXED_ONE; }
            if input.buttons & INPUT_DOWN != 0  { direction.y -= fixed_math::FIXED_ONE; }
            if input.buttons & INPUT_LEFT != 0  { direction.x -= fixed_math::FIXED_ONE; }
            if input.buttons & INPUT_RIGHT != 0 { direction.x += fixed_math::FIXED_ONE; }

            *facing_direction = get_facing_direction(&input);

            cursor_position.x = input.pan_x as i32;
            cursor_position.y = input.pan_y as i32;

            if direction != fixed_math::FixedVec2::ZERO {
                let sprint_multiplier = fixed_math::FIXED_ONE + (config.movement.sprint_multiplier - fixed_math::FIXED_ONE) * sprint_state.sprint_factor;
                // Using FIXED_TIMESTEP instead of time.delta()
                let move_delta = direction.normalize() * config.movement.acceleration * sprint_multiplier * fixed_math::new(FIXED_TIMESTEP);
                velocity.0 += move_delta;
                
                let max_speed = config.movement.max_speed * sprint_multiplier;
                velocity.0 = velocity.0.clamp_length_max(max_speed);
            }
        }
    }
}

pub fn apply_friction(
    inputs: Res<PlayerInputs<PeerConfig>>,
    movement_configs: Res<Assets<CharacterConfig>>,
    mut query: Query<(&mut Velocity, &CharacterConfigHandles, &Player), With<Rollback>>,
) {
    for (mut velocity, config_handles, player) in query.iter_mut() {
        if let Some(config) = movement_configs.get(&config_handles.config) {
            let (input, _input_status) = inputs[player.handle];

            let moving = input.buttons & INPUT_RIGHT != 0 || input.buttons & INPUT_LEFT != 0 || input.buttons & INPUT_UP != 0 || input.buttons & INPUT_DOWN != 0;

            if !moving && velocity.length_squared() > 0.1 {
                velocity.0 = velocity.0 * (fixed_math::FIXED_ONE - config.movement.friction * fixed_math::new(FIXED_TIMESTEP)).max(fixed_math::FIXED_ZERO);
                if velocity.length_squared() < 1.0 {
                    velocity.0 = fixed_math::FixedVec2::ZERO;
                }
            }
        }
    }
}

pub fn move_characters(
    mut query: Query<(&mut fixed_math::FixedTransform3D, &mut Velocity, &Collider, &CollisionLayer), (With<Rollback>, With<Player>)>,
    settings: Res<CollisionSettings>,
    collider_query: Query<(Entity, &fixed_math::FixedTransform3D, &Collider, &CollisionLayer), (With<Wall>, Without<Player>)>,
) {
    'mainloop: for (mut transform, mut velocity, player_collider, collision_layer) in query.iter_mut() {
        let mut new_transform = transform.clone();
        new_transform.translation.x += velocity.x * fixed_math::new(FIXED_TIMESTEP);
        new_transform.translation.y += velocity.y * fixed_math::new(FIXED_TIMESTEP);
        
        for (target_entity, target_transform, target_collider, target_layer) in collider_query.iter() {
            if !settings.layer_matrix[collision_layer.0 as usize][target_layer.0 as usize] {
                continue;
            }
            if is_colliding(&new_transform.translation, player_collider, &target_transform.translation, target_collider) {
                velocity.0 = fixed_math::FixedVec2::ZERO;
                continue 'mainloop;
            }
        }
        *transform = new_transform;
    }
}

pub fn update_animation_state(mut query: Query<(&Velocity, &mut AnimationState), With<Rollback>>) {
    for (velocity, mut state) in query.iter_mut() {
        let current_state_name = state.0.clone();
        let new_state_name = if velocity.length_squared() > 0.5 { "Run" } else { "Idle" };
        if current_state_name != new_state_name { state.0 = new_state_name.to_string(); }
    }
}