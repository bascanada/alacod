use crate::character::config::{CharacterConfig, CharacterConfigHandles};
use crate::character::enemy::Enemy;
use crate::character::movement::Velocity;
use crate::character::player::input::FIXED_TIMESTEP;
use crate::character::player::Player;
use crate::collider::{is_colliding, Collider, Wall};
use animation::FacingDirection;
use bevy::prelude::*;
use bevy_fixed::fixed_math;
use bevy_fixed::rng::RollbackRng;
use std::collections::VecDeque;
use utils::frame::FrameCount;

#[derive(Component, Debug, Clone, Default)]
pub struct EnemyPath {
    // Target to move toward
    pub target_position: fixed_math::FixedVec2,
    // Queue of waypoints (if using pathfinding)
    pub waypoints: VecDeque<fixed_math::FixedVec2>,
    // Path recalculation timer
    pub recalculate_ticks: u32,
    // Path status
    pub path_status: PathStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PathStatus {
    #[default]
    Idle,
    DirectPath,
    CalculatingPath,
    FollowingPath,
    Blocked,
}

#[derive(Resource, Clone)]
pub struct PathfindingConfig {
    // How often to recalculate paths (in frames)
    pub recalculation_interval: u32,
    // Maximum pathfinding iterations
    pub max_iterations: u32,
    // Maximum path length
    pub max_path_length: usize,
    // Direct path threshold (distance at which to use direct path)
    pub direct_path_threshold: fixed_math::Fixed,
    // Node size for discretization (if using grid-based approach)
    pub node_size: fixed_math::Fixed,
    // Movement speed fallback
    pub movement_speed: fixed_math::Fixed,
    // Waypoint reach distance
    pub waypoint_reach_distance: fixed_math::Fixed,
    // Minimum distance to maintain from player (attack range)
    pub optimal_attack_distance: fixed_math::Fixed,
    // Distance at which to start slowing down
    pub slow_down_distance: fixed_math::Fixed,
    // Separation force between enemies
    pub enemy_separation_force: fixed_math::Fixed,
    // Separation distance between enemies
    pub enemy_separation_distance: fixed_math::Fixed,
}

impl Default for PathfindingConfig {
    fn default() -> Self {
        Self {
            recalculation_interval: 30, // Recalculate every half second (at 60 FPS)
            max_iterations: 1000,
            max_path_length: 50,
            direct_path_threshold: fixed_math::new(200.0),
            node_size: fixed_math::new(20.0),
            movement_speed: fixed_math::new(20.0),
            waypoint_reach_distance: fixed_math::new(10.0),
            optimal_attack_distance: fixed_math::new(100.0), // Keep this distance from players
            slow_down_distance: fixed_math::new(150.0),      // Start slowing down at this distance
            enemy_separation_force: fixed_math::new(2.0),    // Much stronger separation force
            enemy_separation_distance: fixed_math::new(80.0), // Larger separation distance
        }
    }
}

// System to find closest player and set as target
pub fn update_enemy_targets(
    player_query: Query<(&fixed_math::FixedTransform3D, &Player)>, // Assuming Player uses FixedTransform3D
    mut enemy_query: Query<(&fixed_math::FixedTransform3D, &mut EnemyPath), With<Enemy>>,
    frame: Res<FrameCount>,
    config: Res<PathfindingConfig>,
) {
    // Get all player positions
    let player_positions: Vec<fixed_math::FixedVec2> = player_query // Changed Vec<Vec2>
        .iter()
        .map(|(fixed_transform, _)| fixed_transform.translation.truncate()) // .truncate() is correct
        .collect();

    if player_positions.is_empty() {
        return;
    }

    // Update each enemy's target
    for (enemy_fixed_transform, mut path) in enemy_query.iter_mut() {
        // Only update periodically to save performance
        if frame.frame % config.recalculation_interval != 0 {
            continue;
        }

        // Find the closest player
        let enemy_pos_v2 = enemy_fixed_transform.translation.truncate();
        // Initialize with the first player, assuming player_positions is not empty (checked above)
        let mut closest_player_pos_v2 = player_positions[0];
        let mut closest_distance_sq = enemy_pos_v2.distance_squared(&closest_player_pos_v2); // Use distance_squared for comparison

        for player_pos_v2 in player_positions.iter().skip(1) {
            // Iterate from the second element
            let distance_sq = enemy_pos_v2.distance_squared(player_pos_v2);
            if distance_sq < closest_distance_sq {
                closest_distance_sq = distance_sq;
                closest_player_pos_v2 = *player_pos_v2;
            }
        }

        // Set the target
        path.target_position = closest_player_pos_v2;

        // Mark for path recalculation
        path.recalculate_ticks = frame.frame; // This seems to be setting it to current frame, not a timer
                                              // Consider if you want path.path_status = PathStatus::CalculatingPath; here
                                              // or if recalculate_ticks is used differently.
                                              // For now, matching original logic.
    }
}

// System to check if direct path is clear
pub fn check_direct_paths(
    wall_query: Query<(&fixed_math::FixedTransform3D, &Collider), With<Wall>>,
    mut enemy_query: Query<(&fixed_math::FixedTransform3D, &mut EnemyPath), With<Enemy>>,
    config: Res<PathfindingConfig>,
) {
    for (enemy_fixed_transform, mut path) in enemy_query.iter_mut() {
        let enemy_pos_v2 = enemy_fixed_transform.translation.truncate();
        let target_v2 = path.target_position;

        let distance = enemy_pos_v2.distance(&target_v2);
        if distance < config.direct_path_threshold {
            let direction_v2 = (target_v2 - enemy_pos_v2).normalize_or_zero();

            // If direction is zero (enemy is at target), path is clear
            if direction_v2 == fixed_math::FixedVec2::ZERO {
                path.waypoints.clear();
                path.path_status = PathStatus::DirectPath;
                continue;
            }

            // Define a step distance for checking collisions along the path
            let check_step_distance = fixed_math::new(10.0); // How far each step check is (e.g., half collider radius)
            let num_steps = (distance / check_step_distance)
                .ceil()
                .to_num::<i32>()
                .max(1) as usize; // Fixed.ceil() then convert

            let mut path_is_blocked = false;

            // Virtual collider for checking along the path
            // Ensure Collider struct and ColliderShape are updated for fixed-point
            let test_collider = Collider {
                shape: crate::collider::ColliderShape::Circle {
                    radius: fixed_math::new(15.0),
                }, // Example radius
                offset: fixed_math::FixedVec3::ZERO,
            };

            for i in 1..=num_steps {
                // Iterate up to and including the target (or num_steps)
                let step_dist = check_step_distance * fixed_math::Fixed::from_num(i);
                let current_check_dist = if step_dist > distance {
                    distance
                } else {
                    step_dist
                };
                let test_pos_v2 = enemy_pos_v2 + direction_v2 * current_check_dist;

                // Create a FixedTransform3D for the test position
                // Assuming Z is 0 for 2D path checking
                let test_fixed_transform = fixed_math::FixedTransform3D {
                    translation: test_pos_v2.extend(), // Converts FixedVec2 to FixedVec3 with z=0
                    rotation: fixed_math::FixedMat3::IDENTITY, // Assuming no rotation for point check
                    scale: fixed_math::FixedVec3::ONE,
                };

                for (wall_fixed_transform, wall_collider) in wall_query.iter() {
                    // is_colliding must take (&FixedTransform3D, &Collider, &FixedTransform3D, &Collider)
                    if is_colliding(
                        &test_fixed_transform.translation,
                        &test_collider,
                        &wall_fixed_transform.translation,
                        wall_collider,
                    ) {
                        path_is_blocked = true;
                        break;
                    }
                }
                if path_is_blocked {
                    break;
                }
            }

            if !path_is_blocked {
                path.waypoints.clear();
                path.path_status = PathStatus::DirectPath;
            } else {
                // Path is blocked
                // If not already following a calculated path, mark for calculation
                if path.path_status != PathStatus::FollowingPath
                    && path.path_status != PathStatus::CalculatingPath
                {
                    path.path_status = PathStatus::CalculatingPath;
                }
            }
        } else {
            // Target is far, needs pathfinding
            // If not already following a calculated path, mark for calculation
            if path.path_status != PathStatus::FollowingPath
                && path.path_status != PathStatus::CalculatingPath
            {
                path.path_status = PathStatus::CalculatingPath;
            }
        }
    }
}

// System to calculate paths around obstacles when needed
pub fn calculate_paths(
    mut enemy_query: Query<(Entity, &fixed_math::FixedTransform3D, &mut EnemyPath), With<Enemy>>,
    wall_query: Query<(&fixed_math::FixedTransform3D, &Collider), With<Wall>>,
    mut rng: ResMut<RollbackRng>,
    config: Res<PathfindingConfig>,
) {
    // --- Step 1: Collect entities and categorize them ---
    let mut entities_needing_path_calculation_data: Vec<(
        Entity,
        fixed_math::FixedVec2,
        fixed_math::FixedVec2,
    )> = Vec::new();
    let mut entities_to_set_direct: Vec<Entity> = Vec::new();

    // Initial immutable iteration to categorize
    for (entity, enemy_fixed_transform, path_component) in enemy_query.iter() {
        if path_component.path_status == PathStatus::CalculatingPath {
            let enemy_pos_v2 = enemy_fixed_transform.translation.truncate();
            let target_v2 = path_component.target_position;

            // Check if already at target (or very close) using squared length for efficiency
            if (target_v2 - enemy_pos_v2).length_squared() > fixed_math::FixedWide::ZERO {
                entities_needing_path_calculation_data.push((entity, enemy_pos_v2, target_v2));
            } else {
                // This entity is in CalculatingPath but already at its target.
                entities_to_set_direct.push(entity);
            }
        }
    }

    // --- Step 2: Update status for entities already at their target ---
    // These entities are no longer part of the main path calculation logic using RNG for this frame.
    for entity_id in entities_to_set_direct {
        if let Ok((_, _, mut path_mut)) = enemy_query.get_mut(entity_id) {
            path_mut.waypoints.clear();
            path_mut.path_status = PathStatus::DirectPath;
        }
    }

    // --- Step 3: Sort entities that genuinely need path calculation for deterministic RNG use ---
    entities_needing_path_calculation_data.sort_unstable_by_key(|(e, _, _)| e.to_bits());

    // --- Step 4: Iterate sorted entities and perform path calculation logic (including RNG) ---
    for (entity_id, enemy_pos_v2, target_v2) in entities_needing_path_calculation_data {
        // Get mutable access to the path component for the current entity
        if let Ok((_fetched_entity, _fetched_transform, mut path)) = enemy_query.get_mut(entity_id)
        {
            // The entity should still be in PathStatus::CalculatingPath because we filtered
            // and processed the "already at target" cases separately. A defensive check can be added if necessary.
            // if path.path_status != PathStatus::CalculatingPath { continue; }

            let mut waypoints = VecDeque::new();
            // Note: enemy_pos_v2 and target_v2 are passed from the collected data
            let direct_dir_v2 = (target_v2 - enemy_pos_v2).normalize_or_zero();

            // If, after all, direct_dir_v2 is zero (e.g. due to precision after collection),
            // handle it here to avoid issues in path logic.
            if direct_dir_v2 == fixed_math::FixedVec2::ZERO {
                path.waypoints.clear();
                path.path_status = PathStatus::DirectPath;
                continue;
            }

            let base_angle_offsets: [fixed_math::Fixed; 7] = [
                fixed_math::FIXED_ZERO,
                fixed_math::new(0.5),
                -fixed_math::new(0.5),
                fixed_math::FIXED_ONE,
                -fixed_math::FIXED_ONE,
                fixed_math::new(1.5),
                -fixed_math::new(1.5),
            ];

            let mut best_angle_fixed = fixed_math::FIXED_ZERO;
            // Initialize with a value that any valid score can beat.
            // Using Fixed::MIN if scores can be negative, or a very small fixed number otherwise.
            let mut best_clearance_score = fixed_math::Fixed::MIN; // Or Fixed::MIN if NEG_INFINITY not defined

            let initial_angle_fixed = fixed_math::atan2_fixed(direct_dir_v2.y, direct_dir_v2.x);

            for angle_offset_fixed in base_angle_offsets.iter() {
                let current_angle_fixed = initial_angle_fixed + *angle_offset_fixed;
                let test_dir_v2 = fixed_math::FixedVec2::new(
                    fixed_math::cos_fixed(current_angle_fixed),
                    fixed_math::sin_fixed(current_angle_fixed),
                );

                let step_check_distance = config.node_size;
                let max_check_steps = 10;

                let test_collider = Collider {
                    shape: crate::collider::ColliderShape::Circle {
                        radius: fixed_math::new(15.0),
                    },
                    offset: fixed_math::FixedVec3::ZERO, // Assuming Collider.offset is FixedVec2
                };

                let mut max_clear_distance = fixed_math::FIXED_ZERO;
                for i in 1..=max_check_steps {
                    let test_dist_along_dir = fixed_math::Fixed::from_num(i) * step_check_distance;
                    let test_pos_v2 = enemy_pos_v2 + test_dir_v2 * test_dist_along_dir;

                    let test_fixed_transform = fixed_math::FixedTransform3D {
                        translation: test_pos_v2.extend(),
                        rotation: fixed_math::FixedMat3::IDENTITY,
                        scale: fixed_math::FixedVec3::ONE,
                    };

                    let mut collides = false;
                    for (wall_fixed_transform, wall_collider) in wall_query.iter() {
                        if is_colliding(
                            &test_fixed_transform.translation,
                            &test_collider,
                            &wall_fixed_transform.translation,
                            wall_collider,
                        ) {
                            collides = true;
                            break;
                        }
                    }

                    if collides {
                        max_clear_distance = test_dist_along_dir - step_check_distance;
                        break;
                    } else {
                        max_clear_distance = test_dist_along_dir;
                    }
                }
                max_clear_distance = max_clear_distance.max(fixed_math::FIXED_ZERO);

                let alignment_factor = fixed_math::FIXED_ONE + test_dir_v2.dot(&direct_dir_v2);
                let current_score = max_clear_distance * alignment_factor;

                if current_score > best_clearance_score {
                    best_clearance_score = current_score;
                    best_angle_fixed = current_angle_fixed;
                }
            }

            if best_clearance_score > fixed_math::Fixed::MIN {
                // Check against initial value
                let waypoint_distance = config.node_size * fixed_math::new(2.0);
                // Calculate remaining distance to target to avoid overshooting massively with waypoint
                let distance_to_target = enemy_pos_v2.distance(&target_v2);
                let waypoint_dist_clamped =
                    waypoint_distance.min(distance_to_target * fixed_math::FIXED_HALF);

                // *** RNG consumed in deterministic order ***
                let jitter_angle_offset = rng.next_fixed_symmetric() * fixed_math::new(0.1);
                let final_waypoint_angle = best_angle_fixed + jitter_angle_offset;
                let final_waypoint_dir = fixed_math::FixedVec2::new(
                    fixed_math::cos_fixed(final_waypoint_angle),
                    fixed_math::sin_fixed(final_waypoint_angle),
                );

                // Ensure waypoint_dist_clamped is positive before creating waypoint
                if waypoint_dist_clamped > fixed_math::FIXED_ZERO {
                    let waypoint_v2 = enemy_pos_v2 + final_waypoint_dir * waypoint_dist_clamped;
                    waypoints.push_back(waypoint_v2);
                }
            }

            if waypoints.len() < config.max_path_length && (waypoints.is_empty() || waypoints.back() != Some(&target_v2)) {
                waypoints.push_back(target_v2);
            }

            if !waypoints.is_empty() {
                path.waypoints = waypoints;
                path.path_status = PathStatus::FollowingPath;
            } else {
                // If no waypoints generated (e.g. couldn't find a clear path, or already very close)
                // Revert to direct path, or mark as blocked if direct path isn't viable.
                // For simplicity here, let's assume if no waypoints, it becomes blocked,
                // allowing check_direct_paths to potentially resolve it next frame or it remains blocked.
                path.path_status = PathStatus::Blocked;
            }
        }
    }
}

pub fn move_enemies(
    mut enemy_query: Query<
        (
            Entity,
            &mut fixed_math::FixedTransform3D,
            &mut Velocity, // Assuming Velocity.0 is FixedVec2
            &mut EnemyPath,
            &mut FacingDirection,
            &CharacterConfigHandles,
        ),
        With<Enemy>,
    >,
    // Assuming Player also uses FixedTransform3D for its logical position
    player_query: Query<&fixed_math::FixedTransform3D, (With<Player>, Without<Enemy>)>,
    character_configs: Res<Assets<CharacterConfig>>, // Assuming max_speed is Fixed
    config: Res<PathfindingConfig>,
) {
    // First pass - collect all enemy positions for separation calculation
    let enemy_positions: Vec<(Entity, fixed_math::FixedVec2)> = enemy_query
        .iter()
        .map(|(entity, fixed_transform, ..)| (entity, fixed_transform.translation.truncate()))
        .collect();

    // Second pass - calculate and apply movement
    for (
        entity,
        mut fixed_transform,
        mut velocity_component,
        mut path,
        mut facing_direction,
        config_handles,
    ) in enemy_query.iter_mut()
    {
        let enemy_pos_v2 = fixed_transform.translation.truncate();

        // Get character movement config
        let movement_speed =
            if let Some(char_config) = character_configs.get(&config_handles.config) {
                char_config.movement.max_speed // This should be fixed_math::Fixed
            } else {
                config.movement_speed // Fallback is also Fixed
            };

        let current_target_v2 = if let Some(waypoint_v2) = path.waypoints.front() {
            *waypoint_v2
        } else {
            path.target_position
        };

        let direction_to_target_v2 = (current_target_v2 - enemy_pos_v2).normalize_or_zero();

        // Calculate distance to nearest player (for attack range check)
        // Initialize with FixedWide::MAX because distance_squared now returns FixedWide
        let mut min_dist_sq_to_player_fw = fixed_math::FixedWide::MAX;

        for player_fixed_transform in player_query.iter() {
            let player_pos_v2 = player_fixed_transform.translation.truncate();
            // enemy_pos_v2.distance_squared(&player_pos_v2) returns FixedWide
            let current_dist_sq_fw = enemy_pos_v2.distance_squared(&player_pos_v2);

            // Compare FixedWide with FixedWide
            if current_dist_sq_fw < min_dist_sq_to_player_fw {
                min_dist_sq_to_player_fw = current_dist_sq_fw;
            }
        }

        // Now min_dist_sq_to_player_fw holds the minimum squared distance as FixedWide.
        // Calculate the actual distance (as Fixed) if a player was found.
        let distance_to_nearest_player: fixed_math::Fixed;
        if min_dist_sq_to_player_fw < fixed_math::FixedWide::MAX {
            // Check against FixedWide::MAX
            // .sqrt() on FixedWide returns FixedWide (assuming FixedSqrt is implemented for FixedWide)
            let dist_fw = min_dist_sq_to_player_fw.sqrt();

            // Convert the FixedWide result of sqrt back to Fixed for use in subsequent game logic
            // This relies on your Fixed::from_num and FixedWide::to_num methods.
            distance_to_nearest_player = fixed_math::Fixed::from_num(dist_fw.to_num::<f32>());
        } else {
            // No players found, or distance was effectively infinite.
            // Set to a very large Fixed value that your game logic can handle.
            distance_to_nearest_player = fixed_math::Fixed::MAX;
        }

        // Calculate separation force (avoid other enemies)
        let mut separation_v2 = fixed_math::FixedVec2::ZERO;
        let mut separation_count: u32 = 0; // Use u32 for count

        for (other_entity, other_pos_v2) in &enemy_positions {
            if *other_entity == entity {
                continue;
            }

            let dist_to_other = enemy_pos_v2.distance(other_pos_v2);
            // Use small epsilon for distance > 0 check
            if dist_to_other < config.enemy_separation_distance
                && dist_to_other > fixed_math::new(0.1)
            {
                let repulsion_v2 = (enemy_pos_v2 - *other_pos_v2).normalize_or_zero()
                    / dist_to_other.max(fixed_math::FIXED_ONE);
                separation_v2 += repulsion_v2;
                separation_count += 1;
            }
        }

        if separation_count > 0 {
            // Convert count to Fixed for division
            separation_v2 = (separation_v2 / fixed_math::Fixed::from_num(separation_count))
                * config.enemy_separation_force;
        }

        let mut desired_move_velocity_v2 = fixed_math::FixedVec2::ZERO;

        match path.path_status {
            PathStatus::DirectPath | PathStatus::FollowingPath => {
                let base_velocity_v2 = direction_to_target_v2 * movement_speed;

                let speed_factor_fixed =
                    if distance_to_nearest_player < config.optimal_attack_distance {
                        fixed_math::new(-0.3) // Back up slightly
                    } else if distance_to_nearest_player < config.slow_down_distance {
                        let range = config.slow_down_distance - config.optimal_attack_distance;
                        if range > fixed_math::FIXED_ZERO {
                            // Avoid division by zero
                            let t = (distance_to_nearest_player - config.optimal_attack_distance)
                                / range;
                            t.clamp(fixed_math::FIXED_ZERO, fixed_math::FIXED_ONE)
                        } else {
                            fixed_math::FIXED_ONE // At optimal or closer than slow_down, but range is zero
                        }
                    } else {
                        fixed_math::FIXED_ONE // Full speed
                    };

                desired_move_velocity_v2 = base_velocity_v2 * speed_factor_fixed;

                if let PathStatus::FollowingPath = path.path_status {
                    if let Some(waypoint_v2) = path.waypoints.front() {
                        if enemy_pos_v2.distance(waypoint_v2) < config.waypoint_reach_distance {
                            path.waypoints.pop_front();
                            if path.waypoints.is_empty() {
                                path.path_status = PathStatus::DirectPath; // Reached end of waypoint list
                            }
                        }
                    } else {
                        // Should not happen if FollowingPath, means waypoints became empty
                        path.path_status = PathStatus::DirectPath;
                    }
                }
            }
            _ => { /* Idle, CalculatingPath, Blocked - no target-based movement */ }
        }

        // Combine movement and separation
        let final_movement_v2 = desired_move_velocity_v2 + separation_v2;
        velocity_component.0 = final_movement_v2; // Assuming Velocity.0 is FixedVec2

        // Apply movement using FIXED_TIMESTEP (must be Fixed)
        // Check against a small epsilon for length_squared
        if velocity_component.0.length_squared() > fixed_math::new(0.01) {
            fixed_transform.translation.x = fixed_transform
                .translation
                .x
                .saturating_add(velocity_component.0.x * fixed_math::new(FIXED_TIMESTEP));
            fixed_transform.translation.y = fixed_transform
                .translation
                .y
                .saturating_add(velocity_component.0.y * fixed_math::new(FIXED_TIMESTEP));
            // Z remains unchanged for 2D movement

            // Update facing direction
            let threshold = fixed_math::new(0.1);
            if velocity_component.0.x > threshold {
                *facing_direction = FacingDirection::Right;
            } else if velocity_component.0.x < -threshold {
                *facing_direction = FacingDirection::Left;
            }
        }
    }
}
