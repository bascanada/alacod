use bevy::prelude::*;
use bevy_fixed::fixed_math;
use bevy_ggrs::Rollback;
use serde::{Deserialize, Serialize};
use utils::{frame::FrameCount, net_id::GgrsNetId, order_mut_iter};

use crate::{
    character::{
        enemy::Enemy,
        health::DamageAccumulator,
        player::Player,
    },
    collider::Window,
};

/// Zombie behavior state
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ZombieState {
    /// Moving toward a target (player or window)
    Pathing,
    /// Attacking a window to break through
    AttackingWindow {
        target_window: GgrsNetId,
        last_attack_frame: u32,
    },
    /// Attacking a nearby player
    AttackingPlayer {
        target_player: GgrsNetId,
        last_attack_frame: u32,
    },
    /// Breaching through a broken window
    Breaching {
        breach_start_frame: u32,
    },
}

impl Default for ZombieState {
    fn default() -> Self {
        ZombieState::Pathing
    }
}

/// Target information for zombies
#[derive(Component, Clone, Debug, Serialize, Deserialize, Default)]
pub struct ZombieTarget {
    /// The target net ID (could be a player or window)
    pub target: Option<GgrsNetId>,
    /// Type of target
    pub target_type: TargetType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TargetType {
    #[default]
    None,
    Player,
    Window,
}

/// Configuration for zombie combat behavior
#[derive(Resource, Clone, Debug)]
pub struct ZombieCombatConfig {
    /// Range at which zombies can attack
    pub attack_range: fixed_math::Fixed,
    /// Frames between attacks
    pub attack_cooldown_frames: u32,
    /// Damage dealt to windows per attack
    pub window_damage: u8,
    /// Damage dealt to players per attack
    pub player_damage: fixed_math::Fixed,
    /// How long breaching through a window takes (frames)
    pub breach_duration_frames: u32,
    /// Range to detect windows
    pub window_detection_range: fixed_math::Fixed,
}

impl Default for ZombieCombatConfig {
    fn default() -> Self {
        Self {
            attack_range: fixed_math::new(40.0),
            attack_cooldown_frames: 60, // 1 second at 60 FPS
            window_damage: 1,
            player_damage: fixed_math::new(10.0),
            breach_duration_frames: 60, // 1 second to breach through
            window_detection_range: fixed_math::new(100.0),
        }
    }
}

/// Event for zombie attacking a window
#[derive(Event, Clone, Debug)]
pub struct ZombieWindowAttackEvent {
    pub zombie_net_id: GgrsNetId,
    pub window_net_id: GgrsNetId,
    pub damage: u8,
}

/// System to determine zombie targets (windows or players)
pub fn zombie_target_selection(
    frame: Res<FrameCount>,
    mut zombie_query: Query<
        (
            &GgrsNetId,
            &fixed_math::FixedTransform3D,
            &mut ZombieTarget,
            &mut ZombieState,
        ),
        With<Enemy>,
    >,
    player_query: Query<(&GgrsNetId, &fixed_math::FixedTransform3D), (With<Player>, Without<Enemy>)>,
    window_query: Query<
        (
            &GgrsNetId,
            &fixed_math::FixedTransform3D,
            &map::game::entity::map::window::WindowHealth,
        ),
        (With<Window>, With<Rollback>),
    >,
    config: Res<ZombieCombatConfig>,
) {
    for (zombie_net_id, zombie_transform, mut target, mut state) in order_mut_iter!(zombie_query) {
        let zombie_pos = zombie_transform.translation.truncate();

        // Check current state - don't retarget if already attacking or breaching
        match *state {
            ZombieState::AttackingWindow { .. } | ZombieState::AttackingPlayer { .. } | ZombieState::Breaching { .. } => {
                continue;
            }
            _ => {}
        }

        // Find closest player
        let mut closest_player: Option<(GgrsNetId, fixed_math::Fixed)> = None;
        for (player_net_id, player_transform) in player_query.iter() {
            let player_pos = player_transform.translation.truncate();
            let distance = zombie_pos.distance(&player_pos);
            
            match closest_player {
                None => closest_player = Some((player_net_id.clone(), distance)),
                Some((_, closest_dist)) if distance < closest_dist => {
                    closest_player = Some((player_net_id.clone(), distance));
                }
                _ => {}
            }
        }

        // Find closest intact window (health > 0)
        let mut closest_window: Option<(GgrsNetId, fixed_math::Fixed)> = None;
        for (window_net_id, window_transform, window_health) in window_query.iter() {
            // Only target windows with health > 0
            if window_health.current > 0 {
                let window_pos = window_transform.translation.truncate();
                let distance = zombie_pos.distance(&window_pos);
                
                if distance < config.window_detection_range {
                    match closest_window {
                        None => closest_window = Some((window_net_id.clone(), distance)),
                        Some((_, closest_dist)) if distance < closest_dist => {
                            closest_window = Some((window_net_id.clone(), distance));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Decision logic: Prioritize players if close, otherwise go for windows
        if let Some((player_net_id, player_distance)) = closest_player.clone() {
            // If player is very close (within attack range), prioritize player
            if player_distance < config.attack_range * fixed_math::new(2.0) {
                target.target = Some(player_net_id);
                target.target_type = TargetType::Player;
                continue;
            }
        }

        // Check if there's a window blocking the path to player
        if let Some((window_net_id, window_distance)) = closest_window.clone() {
            // If window is closer than player, or no player exists, target window
            let should_target_window = match closest_player {
                Some((_, player_distance)) => window_distance < player_distance,
                None => true,
            };

            if should_target_window {
                target.target = Some(window_net_id);
                target.target_type = TargetType::Window;
                continue;
            }
        }

        // Default to player if available
        if let Some((player_net_id, _)) = closest_player {
            target.target = Some(player_net_id);
            target.target_type = TargetType::Player;
        } else {
            target.target = None;
            target.target_type = TargetType::None;
        }
    }
}

/// System to handle zombie attacks on windows and players
pub fn zombie_attack_system(
    frame: Res<FrameCount>,
    mut zombie_query: Query<
        (
            &GgrsNetId,
            Entity,
            &fixed_math::FixedTransform3D,
            &mut ZombieTarget,
            &mut ZombieState,
        ),
        With<Enemy>,
    >,
    player_query: Query<(&GgrsNetId, Entity, &fixed_math::FixedTransform3D), (With<Player>, Without<Enemy>)>,
    window_query: Query<
        (
            &GgrsNetId,
            Entity,
            &fixed_math::FixedTransform3D,
            &map::game::entity::map::window::WindowHealth,
        ),
        (With<Window>, With<Rollback>),
    >,
    mut window_damage_events: EventWriter<ZombieWindowAttackEvent>,
    mut player_damage_query: Query<&mut DamageAccumulator>,
    config: Res<ZombieCombatConfig>,
) {
    for (zombie_net_id, zombie_entity, zombie_transform, mut target, mut state) in order_mut_iter!(zombie_query) {
        let zombie_pos = zombie_transform.translation.truncate();

        // Update state based on target
        match target.target_type {
            TargetType::Window => {
                if let Some(ref target_net_id) = target.target {
                    // Find window by net ID
                    let mut window_found = false;
                    for (window_net_id, window_entity, window_transform, window_health) in window_query.iter() {
                        if window_net_id == target_net_id {
                            window_found = true;
                            let window_pos = window_transform.translation.truncate();
                            let distance = zombie_pos.distance(&window_pos);

                            // If window is destroyed (health = 0), try to breach through
                            if window_health.current == 0 {
                                if distance < config.attack_range {
                                    *state = ZombieState::Breaching {
                                        breach_start_frame: frame.frame,
                                    };
                                } else {
                                    // Window is destroyed but not in breach range - clear target and find new one
                                    target.target = None;
                                    target.target_type = TargetType::None;
                                    *state = ZombieState::Pathing;
                                }
                            }
                            // If in range of window, attack it
                            else if distance < config.attack_range {
                                let should_attack = match &*state {
                                    ZombieState::AttackingWindow { target_window, last_attack_frame } => {
                                        // Check if cooldown has passed and still same target
                                        target_window == window_net_id &&
                                        frame.frame >= last_attack_frame + config.attack_cooldown_frames
                                    }
                                    _ => {
                                        // Not currently attacking, start attacking
                                        true
                                    }
                                };

                                if should_attack {
                                    // Send attack event
                                    window_damage_events.write(ZombieWindowAttackEvent {
                                        zombie_net_id: zombie_net_id.clone(),
                                        window_net_id: window_net_id.clone(),
                                        damage: config.window_damage,
                                    });

                                    *state = ZombieState::AttackingWindow {
                                        target_window: window_net_id.clone(),
                                        last_attack_frame: frame.frame,
                                    };

                                    info!(
                                        "{} Zombie {} attacking window {} at distance {:?}",
                                        frame.as_ref(),
                                        zombie_net_id,
                                        window_net_id,
                                        distance.to_num::<f32>()
                                    );
                                }
                            } else {
                                // Moving toward window
                                *state = ZombieState::Pathing;
                            }
                            break;
                        }
                    }
                    
                    if !window_found {
                        // Window no longer exists - clear target
                        target.target = None;
                        target.target_type = TargetType::None;
                        *state = ZombieState::Pathing;
                    }
                }
            }
            TargetType::Player => {
                if let Some(ref target_net_id) = target.target {
                    // Find player by net ID
                    let mut player_found = false;
                    for (player_net_id, player_entity, player_transform) in player_query.iter() {
                        if player_net_id == target_net_id {
                            player_found = true;
                            let player_pos = player_transform.translation.truncate();
                            let distance = zombie_pos.distance(&player_pos);

                            // If in range, attack player
                            if distance < config.attack_range {
                                let should_attack = match &*state {
                                    ZombieState::AttackingPlayer { target_player, last_attack_frame } => {
                                        target_player == player_net_id &&
                                        frame.frame >= last_attack_frame + config.attack_cooldown_frames
                                    }
                                    _ => true,
                                };

                                if should_attack {
                                    // Apply damage to player
                                    if let Ok(mut damage_accumulator) = player_damage_query.get_mut(player_entity) {
                                        damage_accumulator.total_damage += config.player_damage;
                                    }

                                    *state = ZombieState::AttackingPlayer {
                                        target_player: player_net_id.clone(),
                                        last_attack_frame: frame.frame,
                                    };

                                    info!(
                                        "{} Zombie {} attacking player {} at distance {:?}",
                                        frame.as_ref(),
                                        zombie_net_id,
                                        player_net_id,
                                        distance.to_num::<f32>()
                                    );
                                }
                            } else {
                                *state = ZombieState::Pathing;
                            }
                            break;
                        }
                    }
                    
                    if !player_found {
                        *state = ZombieState::Pathing;
                    }
                }
            }
            TargetType::None => {
                *state = ZombieState::Pathing;
            }
        }

        // Handle breaching state
        if let ZombieState::Breaching { breach_start_frame } = *state {
            if frame.frame >= breach_start_frame + config.breach_duration_frames {
                // Finished breaching - clear window target so zombie can find new target
                target.target = None;
                target.target_type = TargetType::None;
                *state = ZombieState::Pathing;
                info!(
                    "{} Zombie {} finished breaching",
                    frame.as_ref(),
                    zombie_net_id
                );
            }
        }
    }
}
