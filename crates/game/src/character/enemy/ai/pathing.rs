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
use utils::{frame::FrameCount, net_id::GgrsNetId, order_iter, order_mut_iter};

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
            recalculation_interval: 10, // Recalculate every ~166ms (at 60 FPS) for fresher target tracking
            max_iterations: 1000,
            max_path_length: 50,
            direct_path_threshold: fixed_math::new(200.0),
            node_size: fixed_math::new(20.0),
            movement_speed: fixed_math::new(20.0),
            waypoint_reach_distance: fixed_math::new(10.0),
            optimal_attack_distance: fixed_math::new(30.0), // Melee range - get close to players
            slow_down_distance: fixed_math::new(50.0),      // Start slowing down very close
            enemy_separation_force: fixed_math::new(2.0),    // Much stronger separation force
            enemy_separation_distance: fixed_math::new(80.0), // Larger separation distance
        }
    }
}

// System to find closest player and set as target
// Now also considers zombie targets (windows or players) from the combat system
pub fn update_enemy_targets(
    player_query: Query<(&GgrsNetId, &fixed_math::FixedTransform3D, &Player)>,
    mut enemy_query: Query<
        (
            &fixed_math::FixedTransform3D,
            &mut EnemyPath,
            Option<&super::combat::ZombieTarget>,
        ),
        With<Enemy>,
    >,
    window_query: Query<(&GgrsNetId, &fixed_math::FixedTransform3D), With<crate::collider::Window>>,
    frame: Res<FrameCount>,
    config: Res<PathfindingConfig>,
) {
    // Get all player positions with their net IDs
    // GGRS CRITICAL: Sort by net_id for deterministic tie-breaking when multiple players at equal distance
    let mut player_positions: Vec<(GgrsNetId, fixed_math::FixedVec2)> = player_query
        .iter()
        .map(|(net_id, fixed_transform, _)| (net_id.clone(), fixed_transform.translation.truncate()))
        .collect();
    player_positions.sort_unstable_by_key(|(net_id, _)| net_id.0);

    // Update each enemy's target
    for (enemy_fixed_transform, mut path, zombie_target_opt) in enemy_query.iter_mut() {
        // Only update periodically to save performance
        if frame.frame % config.recalculation_interval != 0 {
            continue;
        }

        let enemy_pos_v2 = enemy_fixed_transform.translation.truncate();

        // If zombie has a specific target (window or player), use that
        if let Some(zombie_target) = zombie_target_opt {
            if let Some(ref target_net_id) = zombie_target.target {
                // Try to get target position based on type
                let target_pos_opt = match zombie_target.target_type {
                    super::combat::TargetType::Player => {
                        // Find player by net ID
                        player_positions
                            .iter()
                            .find(|(net_id, _)| net_id == target_net_id)
                            .map(|(_, pos)| *pos)
                    }
                    super::combat::TargetType::Window => {
                        // Get window position by net ID
                        window_query
                            .iter()
                            .find(|(net_id, _)| *net_id == target_net_id)
                            .map(|(_, transform)| transform.translation.truncate())
                    }
                    super::combat::TargetType::None => None,
                };

                if let Some(target_pos) = target_pos_opt {
                    path.target_position = target_pos;
                    path.recalculate_ticks = frame.frame;
                    continue;
                }
            }
        }

        // Fallback: find closest player if no zombie target or target is invalid
        if !player_positions.is_empty() {
            // Initialize with the first player
            let mut closest_player_pos_v2 = player_positions[0].1;
            let mut closest_distance_sq = enemy_pos_v2.distance_squared(&closest_player_pos_v2);

            for (_, player_pos_v2) in player_positions.iter().skip(1) {
                let distance_sq = enemy_pos_v2.distance_squared(player_pos_v2);
                if distance_sq < closest_distance_sq {
                    closest_distance_sq = distance_sq;
                    closest_player_pos_v2 = *player_pos_v2;
                }
            }

            path.target_position = closest_player_pos_v2;
            path.recalculate_ticks = frame.frame;
        }
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
    mut enemy_query: Query<(&GgrsNetId, Entity, &fixed_math::FixedTransform3D, &mut EnemyPath), With<Enemy>>,
    wall_query: Query<(&fixed_math::FixedTransform3D, &Collider), With<Wall>>,
    mut rng: ResMut<RollbackRng>,
    config: Res<PathfindingConfig>,
) {
    // --- Step 1: Collect entities and categorize them ---
    // GGRS CRITICAL: Use net_id for sorting, NOT entity.to_bits() (entity IDs can differ across clients)
    let mut entities_needing_path_calculation_data: Vec<(
        usize, // net_id for sorting
        Entity,
        fixed_math::FixedVec2,
        fixed_math::FixedVec2,
    )> = Vec::new();
    let mut entities_to_set_direct: Vec<Entity> = Vec::new();

    // Initial immutable iteration to categorize
    for (net_id, entity, enemy_fixed_transform, path_component) in enemy_query.iter() {
        if path_component.path_status == PathStatus::CalculatingPath {
            let enemy_pos_v2 = enemy_fixed_transform.translation.truncate();
            let target_v2 = path_component.target_position;

            // Check if already at target (or very close) using squared length for efficiency
            if (target_v2 - enemy_pos_v2).length_squared() > fixed_math::FixedWide::ZERO {
                entities_needing_path_calculation_data.push((net_id.0, entity, enemy_pos_v2, target_v2));
            } else {
                // This entity is in CalculatingPath but already at its target.
                entities_to_set_direct.push(entity);
            }
        }
    }

    // --- Step 2: Update status for entities already at their target ---
    // These entities are no longer part of the main path calculation logic using RNG for this frame.
    for entity_id in entities_to_set_direct {
        if let Ok((_, _, _, mut path_mut)) = enemy_query.get_mut(entity_id) {
            path_mut.waypoints.clear();
            path_mut.path_status = PathStatus::DirectPath;
        }
    }

    // --- Step 3: Sort entities by net_id for deterministic RNG use ---
    entities_needing_path_calculation_data.sort_unstable_by_key(|(net_id, _, _, _)| *net_id);

    // --- Step 4: Iterate sorted entities and perform path calculation logic (including RNG) ---
    for (_net_id, entity_id, enemy_pos_v2, target_v2) in entities_needing_path_calculation_data {
        // Get mutable access to the path component for the current entity
        if let Ok((_, _fetched_entity, _fetched_transform, mut path)) = enemy_query.get_mut(entity_id)
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

            if waypoints.len() < config.max_path_length
                && (waypoints.is_empty() || waypoints.back() != Some(&target_v2))
            {
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
    frame: Res<FrameCount>,
    mut enemy_query: Query<
        (
            &GgrsNetId,
            Entity,
            &mut fixed_math::FixedTransform3D,
            &mut Velocity,
            &mut EnemyPath,
            &mut FacingDirection,
            &CharacterConfigHandles,
            &Collider,
            &crate::collider::CollisionLayer,
        ),
        With<Enemy>,
    >,
    player_query: Query<&fixed_math::FixedTransform3D, (With<Player>, Without<Enemy>)>,
    character_configs: Res<Assets<CharacterConfig>>,
    config: Res<PathfindingConfig>,
    collision_settings: Res<crate::collider::CollisionSettings>,
    wall_collider_query: Query<
        (&fixed_math::FixedTransform3D, &Collider, &crate::collider::CollisionLayer),
        (With<Wall>, Without<Enemy>, Without<Player>),
    >,
    flow_field_cache: Res<super::navigation::FlowFieldCache>,
) {
    // First pass - collect all enemy positions for separation calculation
    // GGRS CRITICAL: Use order_iter! for deterministic iteration order
    let enemy_positions: Vec<(Entity, fixed_math::FixedVec2)> = order_iter!(enemy_query)
        .iter()
        .map(|(_, entity, transform, ..)| (*entity, transform.translation.truncate()))
        .collect();

    // Second pass - calculate and apply movement in deterministic order
    for (
        net_id,
        entity,
        mut fixed_transform,
        mut velocity_component,
        mut path,
        mut facing_direction,
        config_handles,
        enemy_collider,
        enemy_collision_layer,
    ) in order_mut_iter!(enemy_query)
    {
        let enemy_pos_v2 = fixed_transform.translation.truncate();

        // Get character movement config
        let movement_speed =
            if let Some(char_config) = character_configs.get(&config_handles.config) {
                char_config.movement.max_speed // This should be fixed_math::Fixed
            } else {
                config.movement_speed // Fallback is also Fixed
            };

        // Get the current player position from flow field cache (most up-to-date)
        let flow_field_target = flow_field_cache.target_pos.to_fixed();

        // Use flow field for navigation (pathfinds around walls)
        let direction_to_target_v2 = if let Some(flow_field) = flow_field_cache.get_flow_field(super::navigation::NavProfile::Ground) {
            // Get direction from flow field
            match flow_field.get_direction_vector(enemy_pos_v2) {
                Some(dir) => dir,
                None => {
                    // Not in flow field range, move directly toward player
                    (flow_field_target - enemy_pos_v2).normalize_or_zero()
                }
            }
        } else {
            // No flow field, fall back to direct path toward player
            (flow_field_target - enemy_pos_v2).normalize_or_zero()
        };

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

        // Calculate base velocity using flow field direction
        let base_velocity_v2 = direction_to_target_v2 * movement_speed;

        // Slow down when near player (for attack positioning)
        let speed_factor_fixed =
            if distance_to_nearest_player < config.optimal_attack_distance {
                fixed_math::FIXED_ZERO // Stop when in melee range
            } else if distance_to_nearest_player < config.slow_down_distance {
                let range = config.slow_down_distance - config.optimal_attack_distance;
                if range > fixed_math::FIXED_ZERO {
                    let t = (distance_to_nearest_player - config.optimal_attack_distance) / range;
                    t.clamp(fixed_math::FIXED_ZERO, fixed_math::FIXED_ONE)
                } else {
                    fixed_math::FIXED_ONE
                }
            } else {
                fixed_math::FIXED_ONE // Full speed
            };

        let desired_move_velocity_v2 = base_velocity_v2 * speed_factor_fixed;

        // Combine movement and separation for AI intent
        let final_movement_v2 = desired_move_velocity_v2 + separation_v2;
        velocity_component.main = final_movement_v2;

        // GGRS debug trace - log enemy movement decisions for desync debugging
        trace!(
            "[Frame {}] Enemy {:?} (net_id={:?}): pos=({}, {}), flow_dir=({}, {}), vel=({}, {}), sep_count={}",
            frame.frame,
            entity,
            net_id.0,
            enemy_pos_v2.x.to_num::<i32>(),
            enemy_pos_v2.y.to_num::<i32>(),
            direction_to_target_v2.x.to_num::<i32>(),
            direction_to_target_v2.y.to_num::<i32>(),
            final_movement_v2.x.to_num::<i32>(),
            final_movement_v2.y.to_num::<i32>(),
            separation_count,
        );

        // Apply movement using both main and knockback velocities
        let total_velocity = velocity_component.main + velocity_component.knockback;
        if total_velocity.length_squared() > fixed_math::new(0.01) {
            let delta_x = total_velocity.x * fixed_math::new(FIXED_TIMESTEP);
            let delta_y = total_velocity.y * fixed_math::new(FIXED_TIMESTEP);

            // Helper to check collision at a position
            let check_wall_collision = |pos: &fixed_math::FixedVec3| -> bool {
                for (wall_transform, wall_collider, wall_layer) in wall_collider_query.iter() {
                    if !collision_settings.layer_matrix[enemy_collision_layer.0][wall_layer.0] {
                        continue;
                    }
                    if is_colliding(pos, enemy_collider, &wall_transform.translation, wall_collider) {
                        return true;
                    }
                }
                false
            };

            // Try full movement (X + Y)
            let full_pos = fixed_math::FixedVec3::new(
                fixed_transform.translation.x.saturating_add(delta_x),
                fixed_transform.translation.y.saturating_add(delta_y),
                fixed_transform.translation.z,
            );

            if !check_wall_collision(&full_pos) {
                fixed_transform.translation = full_pos;
            } else {
                // Full movement blocked - try sliding along walls
                let mut moved_x = false;
                let mut moved_y = false;

                // Try X only
                if delta_x != fixed_math::FIXED_ZERO {
                    let x_only_pos = fixed_math::FixedVec3::new(
                        fixed_transform.translation.x.saturating_add(delta_x),
                        fixed_transform.translation.y,
                        fixed_transform.translation.z,
                    );
                    if !check_wall_collision(&x_only_pos) {
                        fixed_transform.translation.x = x_only_pos.x;
                        moved_x = true;
                    }
                }

                // Try Y only
                if delta_y != fixed_math::FIXED_ZERO {
                    let y_only_pos = fixed_math::FixedVec3::new(
                        fixed_transform.translation.x, // Use potentially updated X
                        fixed_transform.translation.y.saturating_add(delta_y),
                        fixed_transform.translation.z,
                    );
                    if !check_wall_collision(&y_only_pos) {
                        fixed_transform.translation.y = y_only_pos.y;
                        moved_y = true;
                    }
                }

                // If completely stuck, zero velocity
                if !moved_x && !moved_y {
                    velocity_component.main = fixed_math::FixedVec2::ZERO;
                    trace!(
                        "[Frame {}] Enemy {:?} STUCK at ({}, {})",
                        frame.frame,
                        entity,
                        fixed_transform.translation.x.to_num::<i32>(),
                        fixed_transform.translation.y.to_num::<i32>(),
                    );
                }
            }

            // Update facing direction based on main velocity (not knockback)
            if velocity_component.main.length_squared() > fixed_math::new(0.01) {
                *facing_direction = FacingDirection::from_fixed_vector(velocity_component.main);
            }
        }
    }
}
