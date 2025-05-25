use animation::SpriteSheetConfig;
use bevy::{prelude::*};
use map::game::entity::map::enemy_spawn::EnemySpawnerComponent;
use utils::{fixed_math, rng::RollbackRng};

use crate::{character::{config::CharacterConfig, player::Player}, collider::{Collider, CollisionSettings, Wall}, frame::FrameCount, global_asset::GlobalAsset, weapons::WeaponsConfig};

use super::{create::spawn_enemy, Enemy};

#[derive(Component, Debug, Reflect, Clone)]
#[reflect]
pub struct EnemySpawnerState {
    pub cooldown_remaining: u32,
    pub last_spawn_frame: u32,
    pub active: bool,
}


impl Default for EnemySpawnerState {
    fn default() -> Self {
        Self {
            cooldown_remaining: 0,
            last_spawn_frame: 0,
            active: true,
        }
    } 
}


pub fn enemy_spawn_from_spawners_system(
    mut commands: Commands,
    frame: Res<FrameCount>,
    mut rng: ResMut<RollbackRng>, // Make sure RollbackRng's f32 generation is deterministic
    mut spawner_query: Query<(Entity, &EnemySpawnerComponent, &mut EnemySpawnerState, &fixed_math::FixedTransform3D)>,
    enemy_query: Query<&fixed_math::FixedTransform3D, With<Enemy>>,
    player_query: Query<&fixed_math::FixedTransform3D, With<Player>>,

    global_assets: Res<GlobalAsset>,
    collision_settings: Res<CollisionSettings>,
    weapons_asset: Res<Assets<WeaponsConfig>>,
    characters_asset: Res<Assets<CharacterConfig>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: Res<Assets<SpriteSheetConfig>>,
) {
    // Get player positions for checking distance
    let player_positions: Vec<fixed_math::FixedVec2> = player_query
        .iter()
        .map(|transform| transform.translation.truncate()) // This is already FixedVec2
        .collect();


    if player_positions.is_empty() {
         println!("Frame {}: No players found, skipping spawn", frame.frame); // Keep for debugging if needed
        return; // No players, don't spawn
    }

    // Count current enemies (global count)
    let current_enemies = enemy_query.iter().count();
    let global_max_enemies = 20; // This could be a global config (integer, no change needed)

    if current_enemies >= global_max_enemies {
        println!("Frame {}: Global max enemies reached", frame.frame); // Keep for debugging if needed
        return; // Already at global max enemies
    }

    // Process each spawner
    for (_, config, mut state, spawner_fixed_transform) in spawner_query.iter_mut() {
        // Skip inactive spawners or those on cooldown
        if !state.active || state.cooldown_remaining > 0 {
            // Decrease cooldown
            if state.cooldown_remaining > 0 {
                state.cooldown_remaining -= 1; // Integer, no change
            }
            continue;
        }

        let spawner_pos_v2 = spawner_fixed_transform.translation.truncate(); // FixedVec2

        // Check minimum distance to players
        // Note: Ensure EnemySpawnerComponent.min_spawn_distance is of type fixed_math::Fixed
        let min_distance_to_player = player_positions.iter()
            .map(|player_pos_v2| spawner_pos_v2.distance(player_pos_v2)) // Uses FixedVec2::distance -> Fixed
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) // Fixed should impl PartialOrd
            .unwrap_or(fixed_math::FIXED_32_MAX); // Use the specific fixed-point max value

        // config.min_spawn_distance must be fixed_math::Fixed
        if min_distance_to_player < config.min_spawn_distance {
            println!("Min distance to player {} but config {} position player {:?}  me {:?}", min_distance_to_player, config.min_spawn_distance, player_positions, spawner_pos_v2);
            continue;
        }

        // Calculate final spawn position (with optional small random offset)
        // Note: Ensure EnemySpawnerComponent.spawn_radius is of type fixed_math::Fixed
        let final_spawn_pos_v3 = if config.spawn_radius > fixed_math::FIXED_ZERO {
            // Create deterministic offset using RNG
            // Convert f32 from RNG to Fixed. This is a critical point for determinism.
            // Ensure rng.next_f32() is suitable for your rollback needs.
            let angle_rand_fixed = rng.next_fixed(); // Fixed num in [0, 1) approx.
            let angle_fixed = angle_rand_fixed * fixed_math::FIXED_TAU;

            let distance_rand_fixed = rng.next_fixed(); // Fixed num in [0, 1) approx.
            let distance_fixed = distance_rand_fixed * config.spawn_radius;

            let offset_v2 = fixed_math::FixedVec2::new(
                fixed_math::cos_fixed(angle_fixed),
                fixed_math::sin_fixed(angle_fixed)
            ) * distance_fixed; // FixedVec2 * Fixed scalar multiplication

            // Apply the offset to the spawner's 2D position and extend to 3D
            fixed_math::FixedVec3::new(
                spawner_pos_v2.x.saturating_add(offset_v2.x),
                spawner_pos_v2.y.saturating_add(offset_v2.y),
                fixed_math::FIXED_ZERO // Assuming Z is zero for your game's spawning logic
            )
        } else {
            // Use exact spawner position (which is already FixedVec3)
            spawner_fixed_transform.translation
        };

        // Select enemy type deterministically
        let type_index = (rng.next_u32() as usize) % config.enemy_types.len();
        let enemy_type_name = config.enemy_types[type_index].clone();

        // Spawn the enemy
        // The `spawn_enemy` function's signature must be updated to accept FixedVec3 for position
        spawn_enemy(
            enemy_type_name,
            final_spawn_pos_v3, // This is now FixedVec3
            &mut commands,
            &weapons_asset,
            &characters_asset,
            &asset_server,
            &mut texture_atlas_layouts,
            &sprint_sheet_assets,
            &global_assets,
            &collision_settings,
        );

        // Update state
        state.cooldown_remaining = config.max_cooldown; // Assuming max_cooldown is integer
        state.last_spawn_frame = frame.frame;
    }
}