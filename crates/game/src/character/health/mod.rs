
pub mod ui;

use bevy::{log::{tracing::span, Level}, prelude::*};
use bevy_fixed::fixed_math;
use bevy_ggrs::Rollback;
use ggrs::PlayerHandle;
use serde::{Deserialize, Serialize};
use std::fmt;
use utils::{frame::FrameCount, net_id::GgrsNetId, order_iter, order_mut_iter};

#[derive(Component, Reflect, Debug, Clone, Serialize, Deserialize)]
pub enum HitBy {
    Entity(GgrsNetId),
    Player(PlayerHandle),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthConfig {
    pub max: fixed_math::Fixed,
    #[serde(default)]
    pub regen_rate: Option<fixed_math::Fixed>, // Health per second
    #[serde(default)]
    pub regen_delay_frames: Option<u32>, // Frames to wait after taking damage before regen starts
}

#[derive(Component, Clone, Debug, Serialize, Default, Deserialize)]
pub struct Health {
    pub current: fixed_math::Fixed,
    pub max: fixed_math::Fixed,
    pub invulnerable_until_frame: Option<u32>, // Optional invulnerability window
}

#[derive(Component, Clone, Debug, Serialize, Default, Deserialize)]
pub struct HealthRegen {
    pub last_damage_frame: u32,
    pub regen_rate: fixed_math::Fixed,
    pub regen_delay_frames: u32,
}

#[derive(Component, Clone, Debug, Serialize, Deserialize, Default)]
pub struct Death {
    pub last_hit_by: Option<Vec<HitBy>>,
}

#[derive(Component, Clone, Serialize, Deserialize, Default)]
pub struct DamageAccumulator {
    pub total_damage: fixed_math::Fixed,
    pub hit_count: u32,
    pub last_hit_by: Option<Vec<HitBy>>,
}

impl fmt::Display for HitBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HitBy::Entity(net_id) => write!(f, "NetId({})", net_id.0),
            HitBy::Player(player_handle) => write!(f, "Player({})", player_handle),
        }
    }
}

impl fmt::Display for Health {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HP: {}/{}", self.current, self.max)?;
        if let Some(frame) = self.invulnerable_until_frame {
            write!(f, " (Invulnerable until frame {})", frame)?;
        }
        Ok(())
    }
}

impl fmt::Display for Death {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.last_hit_by {
            Some(hits) if !hits.is_empty() => {
                for (i, hit_by) in hits.iter().enumerate() {

 
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", hit_by)?;
                }
                Ok(())
            }
            Some(_) | None => write!(f, "Died (cause unknown or no direct hit)"),
        }
    }
}

impl From<HealthConfig> for Health {
    fn from(value: HealthConfig) -> Self {
        Self {
            current: value.max,
            max: value.max,
            invulnerable_until_frame: None,
        }
    }
}

#[derive(Component, Clone, Debug, Default)]
pub struct DeathAnimation {
    pub timer: u32, // frames elapsed
    pub total_frames: u32, // total duration of the animation
    pub frame_duration: u32, // how many frames each animation frame lasts
    pub animation_frame_count: u32, // number of sprite frames in the animation
}

// Small struct describing a pending visual death spawn. Kept as a non-rollback
// resource so visuals are spawned in the non-rollback world (visible in solo).
#[derive(Clone, Debug)]
pub struct DeathSpawn {
    pub asset_name: String,
    pub translation: fixed_math::FixedVec3,
    pub seed: u64,
}

#[derive(Resource, Default)]
pub struct DeathSpawnQueue(pub Vec<DeathSpawn>);

impl DeathSpawnQueue {
    pub fn push(&mut self, item: DeathSpawn) {
        self.0.push(item);
    }

    pub fn drain_all(&mut self) -> Vec<DeathSpawn> {
        self.0.drain(..).collect()
    }
}

// Death animation prefix constant
const DEATH_ANIMATION_PREFIX: &str = "Dying";

/// Selects an animation from available animations with the given prefix.
/// 
/// This is useful when you have multiple variants of an animation (e.g., "Dying_1", "Dying_2", "Dying_3")
/// and want to select one based on different criteria.
/// 
/// # Examples
/// 
/// ```ignore
/// // For death animations - use deterministic random based on entity ID
/// let death_anim = select_animation(
///     anim_config,
///     "Dying",
///     AnimationSelector::DeterministicRandom(entity_id.0)
/// );
/// 
/// // For specific attack - always use the first one
/// let attack_anim = select_animation(
///     anim_config,
///     "Attack",
///     AnimationSelector::Index(0)
/// );
/// 
/// // For combo attacks - cycle through variants
/// let combo_anim = select_animation(
///     anim_config,
///     "Attack",
///     AnimationSelector::Index(combo_count as usize)
/// );
/// ```
/// 
/// # Arguments
/// * `anim_config` - The animation configuration to search
/// * `prefix` - The prefix to match (e.g., "Dying", "Attack")
/// * `selector` - How to select from multiple matches
/// 
/// # Returns
/// The selected animation name, or None if no matches found
pub fn select_animation(
    anim_config: &animation::AnimationMapConfig,
    prefix: &str,
    selector: AnimationSelector,
) -> Option<String> {
    // Collect all animation names that start with the prefix
    let mut matching_anims: Vec<String> = anim_config
        .animations
        .keys()
        .filter(|name| name.starts_with(prefix))
        .cloned()
        .collect();
    
    if matching_anims.is_empty() {
        return None;
    }
    
    // Sort for determinism
    matching_anims.sort();
    
    match selector {
        AnimationSelector::First => Some(matching_anims[0].clone()),
        AnimationSelector::Last => Some(matching_anims[matching_anims.len() - 1].clone()),
        AnimationSelector::Index(idx) => {
            if idx < matching_anims.len() {
                Some(matching_anims[idx].clone())
            } else {
                Some(matching_anims[0].clone())
            }
        }
        AnimationSelector::DeterministicRandom(seed) => {
            let index = (seed as usize) % matching_anims.len();
            Some(matching_anims[index].clone())
        }
    }
}

/// How to select an animation when multiple variants exist
#[derive(Debug, Clone, Copy)]
pub enum AnimationSelector {
    /// Select the first animation (alphabetically)
    First,
    /// Select the last animation (alphabetically)
    Last,
    /// Select by specific index (wraps if out of bounds)
    Index(usize),
    /// Select deterministically based on a seed (for rollback consistency)
    DeterministicRandom(u64),
}


pub fn rollback_apply_accumulated_damage(
    frame: Res<FrameCount>,
    mut commands: Commands,
    mut query: Query<(&GgrsNetId, Entity, &DamageAccumulator, &mut Health, Option<&mut HealthRegen>), With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "apply_damage");
    let _enter = system_span.enter();

    for (g_id, entity, accumulator, mut health, opt_regen) in order_mut_iter!(query) {
        if accumulator.total_damage > fixed_math::FIXED_ZERO {
            health.current = health.current.saturating_sub(accumulator.total_damage);

            info!(
                "{} receive {} dmg health is {}",
                g_id, accumulator.total_damage, health.current
            );

            // Update last damage frame for regen
            if let Some(mut regen) = opt_regen {
                regen.last_damage_frame = frame.frame;
            }

            commands.entity(entity).remove::<DamageAccumulator>();

            if health.current <= fixed_math::FIXED_ZERO {
                commands.entity(entity).insert(Death {
                    last_hit_by: accumulator.last_hit_by.clone(),
                });
            }
        }
    }
}

pub fn rollback_apply_death(
    frame: Res<FrameCount>,
    mut commands: Commands,
    global_assets: Res<crate::global_asset::GlobalAsset>,
    character_asset: Res<Assets<crate::character::config::CharacterConfig>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: Res<Assets<animation::SpriteSheetConfig>>,
    animation_configs: Res<Assets<animation::AnimationMapConfig>>,
    mut death_queue: ResMut<DeathSpawnQueue>,
    mut query: Query<(&GgrsNetId, Entity, &Death, &fixed_math::FixedTransform3D, &crate::character::config::CharacterConfigHandles), With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "apply_death");
    let _enter = system_span.enter();

    for (id, entity, death_info, transform, config_handle) in order_iter!(query) {
        info!("{} entity {} killed by {}", frame.as_ref(), id, death_info);
        
        // Get the character config to find the asset_name_ref
        if let Some(config) = character_asset.get(&config_handle.config) {
            // Use asset_name_ref to look up the correct spritesheets and animations
            let asset_name = config.asset_name_ref.clone();

            // Instead of spawning the visual entity directly from the rollback system
            // (which runs inside the GGRS rollback and would be rolled back), queue
            // an instruction into the non-rollback death spawn queue. A separate
            // non-rollback system will drain this queue and spawn the visual entity
            // so it persists locally (works for solo play as well).
            death_queue.push(DeathSpawn {
                asset_name,
                translation: transform.translation,
                seed: id.0 as u64,
            });
        }

        // Despawn the rollback entity
        commands.entity(entity).despawn();
    }
}

// SYSTEM: HEALTH REGENERATION
pub fn rollback_health_regeneration(
    frame: Res<FrameCount>,
    mut query: Query<(&GgrsNetId, &mut Health, &HealthRegen), With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "health_regen");
    let _enter = system_span.enter();

    for (g_id, mut health, regen) in order_mut_iter!(query) {
        // Check if enough time has passed since last damage
        let frames_since_damage = frame.frame.saturating_sub(regen.last_damage_frame);
        
        if frames_since_damage >= regen.regen_delay_frames && health.current < health.max {
            let health_before = health.current;
            // Regenerate health (60 frames per second)
            let regen_per_frame = regen.regen_rate / fixed_math::new(60.0);
            health.current = (health.current + regen_per_frame).min(health.max);
            
            // Log every 60 frames (once per second) or when reaching max health
            if frame.frame % 60 == 0 || health.current >= health.max {
                info!(
                    "{} regen {} -> {} (+{}/s, {}f since dmg)",
                    g_id, health_before, health.current, regen.regen_rate, frames_since_damage
                );
            }
        }
    }
}




pub fn advance_death_animation_system(
    mut commands: Commands,
    animation_configs: Res<Assets<animation::AnimationMapConfig>>,
    spritesheet_configs: Res<Assets<animation::SpriteSheetConfig>>,
    mut query: Query<(
        Entity, 
        &mut DeathAnimation, 
        &animation::AnimationState,
        &Children,
        &animation::CharacterAnimationHandles,
    )>,
    mut sprite_query: Query<&mut Sprite, With<animation::AnimatedLayer>>,
) {
    for (entity, mut death_anim, anim_state, children, handles) in query.iter_mut() {
        // Advance timer
        death_anim.timer += 1;
        
        // Calculate which animation frame we should be on
        let current_anim_frame = if death_anim.frame_duration > 0 {
            (death_anim.timer / death_anim.frame_duration).min(death_anim.animation_frame_count - 1)
        } else {
            0
        };
        
        // Update sprite indices to match the current animation frame
        if let Some(anim_config) = animation_configs.get(&handles.animations) {
            if let Some(animation) = anim_config.animations.get(&anim_state.0) {
                // Get columns from config or spritesheet
                let columns = anim_config.columns.or_else(|| {
                    handles.spritesheets.values().next().and_then(|handle| {
                        spritesheet_configs.get(handle).map(|config| config.columns)
                    })
                }).unwrap_or(1);
                
                let (start_index, _end_index) = animation.to_absolute(columns);
                let target_index = start_index + current_anim_frame as usize;
                
                // Update all child sprites
                for child in children.iter() {
                    if let Ok(mut sprite) = sprite_query.get_mut(child) {
                        if let Some(atlas) = &mut sprite.texture_atlas {
                            atlas.index = target_index;
                        }
                    }
                }
            }
        }
        
        // If animation is done, despawn
        if death_anim.timer >= death_anim.total_frames {
            commands.entity(entity).despawn_recursive();
        }
    }
}


/// Drains the non-rollback death spawn queue and spawns visual-only entities.
/// This runs in the normal Update schedule so visuals persist in solo play.
pub fn process_death_spawn_queue(
    mut commands: Commands,
    mut queue: ResMut<DeathSpawnQueue>,
    global_assets: Res<crate::global_asset::GlobalAsset>,
    character_asset: Res<Assets<crate::character::config::CharacterConfig>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: Res<Assets<animation::SpriteSheetConfig>>,
    animation_configs: Res<Assets<animation::AnimationMapConfig>>,
) {
    let items = queue.drain_all();
    if !items.is_empty() {
        info!("Processing {} death spawns", items.len());
    }
    for item in items {
        info!("Spawning death animation for {} at {:?}", item.asset_name, item.translation);
        spawn_death_animation_entity(
            &mut commands,
            &global_assets,
            &character_asset,
            &asset_server,
            &mut texture_atlas_layouts,
            &sprint_sheet_assets,
            &animation_configs,
            item.asset_name,
            item.translation,
            item.seed,
        );
    }
}


/// Spawns a visual-only entity to play the death animation at the given position.
fn spawn_death_animation_entity(
    commands: &mut Commands,
    global_assets: &crate::global_asset::GlobalAsset,
    character_asset: &Res<Assets<crate::character::config::CharacterConfig>>,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: &Res<Assets<animation::SpriteSheetConfig>>,
    animation_configs: &Res<Assets<animation::AnimationMapConfig>>,
    config_name: String,
    translation: fixed_math::FixedVec3,
    entity_id_seed: u64, // For deterministic animation selection
) {
    // Use the same logic as create_character, but do NOT add Rollback or gameplay components
    // Only spawn the animation bundle and sprite
    let handle = global_assets.character_configs.get(&config_name).unwrap();
    let config = character_asset.get(handle).unwrap();
    let map_layers = global_assets.spritesheets.get(&config.asset_name_ref).unwrap().clone();
    let animation_handle = global_assets.animations.get(&config.asset_name_ref).unwrap().clone();
    let starting_layer = config
        .skins
        .get(&config.starting_skin)
        .unwrap()
        .layers
        .clone();
    let animation_bundle = animation::AnimationBundle::new(
        map_layers.clone(),
        animation_handle.clone(),
        0,
        starting_layer.clone(),
    );
    let transform_fixed = fixed_math::FixedTransform3D::new(
        translation,
        fixed_math::FixedMat3::IDENTITY,
        fixed_math::FixedVec3::splat(config.scale),
    );
    
    // Select a death animation deterministically
    let anim_config = animation_configs.get(&animation_handle).unwrap();
    let death_anim_name = select_animation(
        anim_config,
        DEATH_ANIMATION_PREFIX,
        AnimationSelector::DeterministicRandom(entity_id_seed),
    ).unwrap_or_else(|| "Dying_1".to_string()); // Fallback to Dying_1 if no death animations found
    
    info!("Selected death animation: {} for config: {}", death_anim_name, config_name);
    
    let death_anim = anim_config.animations.get(&death_anim_name);
    
    // Target duration for death animation (in frames at 60 FPS)
    const DEATH_ANIMATION_DURATION_FRAMES: u32 = 60; // 1 second total
    
    // Calculate total frames and per-frame duration
    let (total_frames, custom_frame_duration, animation_frame_count) = if let Some(anim) = death_anim {
        // Get the sprite sheet to know the columns
        let spritesheet = sprint_sheet_assets.get(map_layers.values().next().unwrap()).unwrap();
        let columns = spritesheet.columns;
        let (start, end) = anim.to_absolute(columns);
        let frame_count = (end - start + 1) as u32;
        
        // Calculate how many frames each animation frame should last
        // to spread the animation over DEATH_ANIMATION_DURATION_FRAMES
        let frames_per_anim_frame = DEATH_ANIMATION_DURATION_FRAMES / frame_count.max(1);
        
        (DEATH_ANIMATION_DURATION_FRAMES, frames_per_anim_frame, frame_count)
    } else {
        // Fallback
        (DEATH_ANIMATION_DURATION_FRAMES, 10, 6)
    };
    
    let entity = commands.spawn((
        transform_fixed.to_bevy_transform(),
        Visibility::default(),
        animation_bundle,
        DeathAnimation { 
            timer: 0, 
            total_frames,
            frame_duration: custom_frame_duration,
            animation_frame_count,
        },
    ));
    let entity = entity.id();
    info!("Spawned death animation entity {:?} with {} total frames, {} frame duration, {} anim frames", 
          entity, total_frames, custom_frame_duration, animation_frame_count);
    for k in starting_layer.keys() {
        let spritesheet_config = sprint_sheet_assets.get(map_layers.get(k).unwrap()).unwrap();
        animation::create_child_sprite(
            commands,
            asset_server,
            texture_atlas_layouts,
            entity,
            spritesheet_config,
            0,
        );
    }
    // Set animation state to selected death animation
    commands.entity(entity).insert(animation::AnimationState(death_anim_name));
}
