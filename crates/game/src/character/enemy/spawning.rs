use animation::SpriteSheetConfig;
use bevy::prelude::*;
use bevy_fixed::{fixed_math, rng::RollbackRng};
use map::game::entity::map::enemy_spawn::EnemySpawnerComponent;

use crate::{
    character::{config::CharacterConfig, player::Player},
    collider::CollisionSettings,
    global_asset::GlobalAsset,
    weapons::{melee::MeleeWeaponsConfig, WeaponsConfig},
};
use utils::{frame::FrameCount, net_id::{GgrsNetId, GgrsNetIdFactory}};

use super::{create::spawn_enemy, Enemy};

/// Maximum distance from player for spawners to activate
/// Should be within flow field range (50 cells * 16 = 800 units)
const MAX_SPAWN_DISTANCE: f32 = 700.0;

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
    mut rng: ResMut<RollbackRng>,
    mut spawner_query: Query<(
        &GgrsNetId,
        Entity,
        &EnemySpawnerComponent,
        &mut EnemySpawnerState,
        &fixed_math::FixedTransform3D,
    )>,
    enemy_query: Query<&fixed_math::FixedTransform3D, With<Enemy>>,
    player_query: Query<&fixed_math::FixedTransform3D, With<Player>>,
    global_assets: Res<GlobalAsset>,
    collision_settings: Res<CollisionSettings>,
    weapons_asset: Res<Assets<WeaponsConfig>>,
    melee_weapons_asset: Res<Assets<MeleeWeaponsConfig>>,
    characters_asset: Res<Assets<CharacterConfig>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: Res<Assets<SpriteSheetConfig>>,

    mut id_factory: ResMut<GgrsNetIdFactory>,
) {
    let player_positions: Vec<fixed_math::FixedVec2> = player_query
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();

    if player_positions.is_empty() {
        // println!("Frame {}: No players found, skipping spawn", frame.frame);
        return;
    }

    let mut current_enemies_count = enemy_query.iter().count(); // Make mutable for local tracking
    let global_max_enemies = 10; // Reduced from 20 to keep fewer zombies on screen

    if current_enemies_count >= global_max_enemies {
        // println!("Frame {}: Global max enemies reached at start", frame.frame);
        return;
    }

    // --- Step 1: Collect candidate spawner data (net_id for sorting, entity for access) ---
    let mut candidate_spawners: Vec<(usize, Entity)> = Vec::new();
    let max_spawn_dist = fixed_math::new(MAX_SPAWN_DISTANCE);

    for (net_id, entity, config, mut state, spawner_fixed_transform) in spawner_query.iter_mut() {
        if !state.active || state.cooldown_remaining > 0 {
            if state.cooldown_remaining > 0 {
                state.cooldown_remaining -= 1;
            }
            continue;
        }

        let spawner_pos_v2 = spawner_fixed_transform.translation.truncate();
        let min_distance_to_player = player_positions
            .iter()
            .map(|player_pos_v2| spawner_pos_v2.distance(player_pos_v2))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(fixed_math::Fixed::MAX);

        // Skip if player is too close (min distance check)
        if min_distance_to_player < config.min_spawn_distance {
            continue;
        }

        // Skip if player is too far (outside flow field range)
        // This prevents spawning zombies that can't navigate to the player
        if min_distance_to_player > max_spawn_dist {
            continue;
        }

        candidate_spawners.push((net_id.0, entity));
    }

    // --- Step 2: Sort by net_id for deterministic processing ---
    candidate_spawners.sort_unstable_by_key(|(net_id, _)| *net_id);

    // --- Step 3: Process sorted entities, consuming RNG deterministically ---
    for (_net_id, entity_id) in candidate_spawners {
        if current_enemies_count >= global_max_enemies {
            break; // Stop spawning if global limit is hit
        }

        // Get mutable access to the components for the current entity
        // This re-fetches, which is necessary after collecting IDs if we need mutable access.
        if let Ok((_, _entity_refetch, config, mut state, spawner_fixed_transform)) =
            spawner_query.get_mut(entity_id)
        {
            // The checks for active/cooldown/distance have already passed for this entity
            // to be in candidate_spawner_entities. state.cooldown_remaining was already decremented if needed.

            let spawner_pos_v2 = spawner_fixed_transform.translation.truncate(); // Already FixedVec2

            let final_spawn_pos_v3 = if config.spawn_radius > fixed_math::FIXED_ZERO {
                // RNG is now consumed in a deterministic order due to sorted entity iteration
                let angle_rand_fixed = rng.next_fixed();
                let angle_fixed = angle_rand_fixed * fixed_math::FIXED_TAU;

                let distance_rand_fixed = rng.next_fixed();
                let distance_fixed = distance_rand_fixed * config.spawn_radius;

                let offset_v2 = fixed_math::FixedVec2::new(
                    fixed_math::cos_fixed(angle_fixed),
                    fixed_math::sin_fixed(angle_fixed),
                ) * distance_fixed;

                fixed_math::FixedVec3::new(
                    spawner_pos_v2.x.saturating_add(offset_v2.x),
                    spawner_pos_v2.y.saturating_add(offset_v2.y),
                    fixed_math::FIXED_ZERO,
                )
            } else {
                spawner_fixed_transform.translation
            };

            let type_index = (rng.next_u32() as usize) % config.enemy_types.len();
            let enemy_type_name = config.enemy_types[type_index].clone();

            let _ = spawn_enemy(
                enemy_type_name,
                final_spawn_pos_v3,
                &mut commands,
                &weapons_asset,
                &melee_weapons_asset,
                &characters_asset,
                &asset_server,
                &mut texture_atlas_layouts,
                &sprint_sheet_assets,
                &global_assets,
                &collision_settings,
                &mut id_factory,
            );

            state.cooldown_remaining = config.max_cooldown;
            state.last_spawn_frame = frame.frame;
            current_enemies_count += 1; // Increment local count for this frame's spawns

            // If your design is to spawn ONLY ONE enemy per system call,
            // even if multiple spawners are ready, you would put a `return;` here.
            // As written, it will try to spawn from all ready & sorted spawners up to global_max_enemies.
        }
    }
}
