use animation::{FacingDirection, SpriteSheetConfig};
use bevy::{log::{tracing::span, Level}, platform::collections::HashMap, prelude::*};
use bevy_fixed::fixed_math;
use bevy_ggrs::{AddRollbackCommandExtension, Rollback};
use ggrs::PlayerHandle;
use serde::{Deserialize, Serialize};
use utils::{net_id::{GgrsNetId, GgrsNetIdFactory}, order_iter};

use crate::{
    character::{
        health::{DamageAccumulator, Health, HitBy},
        movement::Velocity,
        enemy::Enemy,
        player::{input::INPUT_MELEE_ATTACK, jjrs::PeerConfig, Player},
    },
    collider::{is_colliding, Collider, ColliderShape, CollisionLayer, CollisionSettings},
    global_asset::GlobalAsset,
};
use utils::frame::FrameCount;

// MELEE ATTACK PATTERN
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MeleeAttackPattern {
    // Single quick strike
    SingleStrike,
    // Sweeping arc attack (hits multiple targets in front)
    Sweep {
        arc_angle: fixed_math::Fixed,
    },
    // Thrust forward (longer range, narrower)
    Thrust,
    // Combo chain (multiple strikes in sequence)
    Combo {
        strikes_in_combo: u32,
    },
}

// MELEE WEAPON CONFIG
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeleeWeaponConfig {
    pub name: String,
    pub damage: fixed_math::Fixed,
    pub range: fixed_math::Fixed,
    pub attack_pattern: MeleeAttackPattern,
    pub attack_duration_frames: u32,
    pub cooldown_frames: u32,
    pub knockback_force: fixed_math::Fixed,
    pub stamina_cost: fixed_math::Fixed,
}

// MELEE WEAPON SPRITE CONFIG
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeleeWeaponSpriteConfig {
    pub name: String,
    pub index: usize,
    pub weapon_offset: fixed_math::FixedVec2,
}

// MELEE WEAPON ASSET
#[derive(Serialize, Deserialize, Clone)]
pub struct MeleeWeaponAsset {
    pub config: MeleeWeaponConfig,
    pub sprite_config: MeleeWeaponSpriteConfig,
}

// MELEE WEAPON COMPONENT
#[derive(Component, Debug, Clone)]
pub struct MeleeWeapon {
    pub config: MeleeWeaponConfig,
    pub sprite_config: MeleeWeaponSpriteConfig,
}

impl From<MeleeWeaponAsset> for MeleeWeapon {
    fn from(value: MeleeWeaponAsset) -> Self {
        Self {
            config: value.config,
            sprite_config: value.sprite_config,
        }
    }
}

// MELEE ATTACK STATE
#[derive(Component, Reflect, Default, Clone, Debug, Serialize, Deserialize)]
pub struct MeleeAttackState {
    pub is_attacking: bool,
    pub attack_started_frame: Option<u32>,
    pub attack_ended_frame: Option<u32>,
    pub last_attack_frame: u32,
    pub combo_count: u32,
    pub entities_hit_this_attack: Vec<GgrsNetId>,
}

impl MeleeAttackState {
    pub fn can_attack(&self, current_frame: u32, cooldown_frames: u32) -> bool {
        !self.is_attacking && (current_frame - self.last_attack_frame >= cooldown_frames)
    }

    pub fn start_attack(&mut self, current_frame: u32) {
        self.is_attacking = true;
        self.attack_started_frame = Some(current_frame);
        self.attack_ended_frame = None;
        self.entities_hit_this_attack.clear();
    }

    pub fn end_attack(&mut self, current_frame: u32) {
        self.is_attacking = false;
        self.attack_ended_frame = Some(current_frame);
        self.last_attack_frame = current_frame;
        self.combo_count = 0;
        self.entities_hit_this_attack.clear();
    }

    pub fn has_hit_entity(&self, net_id: &GgrsNetId) -> bool {
        self.entities_hit_this_attack.contains(net_id)
    }

    pub fn add_hit_entity(&mut self, net_id: GgrsNetId) {
        if !self.entities_hit_this_attack.contains(&net_id) {
            self.entities_hit_this_attack.push(net_id);
        }
    }
}

// MELEE ATTACK HITBOX
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct MeleeHitbox {
    pub damage: fixed_math::Fixed,
    pub knockback_force: fixed_math::Fixed,
    pub owner_entity: Entity,
    pub owner_net_id: GgrsNetId,
    pub owner_handle: Option<PlayerHandle>,
    pub created_frame: u32,
    pub duration_frames: u32,
}

// SLASH VISUAL EFFECT
#[derive(Component)]
pub struct SlashEffect {
    pub start_frame: u32,
    pub duration_frames: u32,
}

// MELEE WEAPONS CONFIG ASSET
#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct MeleeWeaponsConfig(pub HashMap<String, MeleeWeaponAsset>);

// SPAWN MELEE WEAPON
pub fn spawn_melee_weapon_for_character(
    commands: &mut Commands,
    _global_assets: &Res<GlobalAsset>,
    _asset_server: &Res<AssetServer>,
    _texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    _sprite_sheet_assets: &Res<Assets<SpriteSheetConfig>>,
    _active: bool,
    character_entity: Entity,
    weapon: MeleeWeaponAsset,
    id_factory: &mut ResMut<GgrsNetIdFactory>,
) -> Entity {
    // For now, melee weapons might not have visible sprites or use simple sprite representations
    // This can be expanded later with actual weapon sprites
    
    let weapon_component: MeleeWeapon = weapon.into();
    
    let transform = Transform::from_translation(
        fixed_math::fixed_to_vec2(weapon_component.sprite_config.weapon_offset).extend(0.),
    )
    .with_rotation(Quat::IDENTITY);
    let ggrs_transform = fixed_math::FixedTransform3D::from_bevy_transform(&transform);

    let entity = commands
        .spawn((
            transform,
            ggrs_transform,
            weapon_component.clone(),
            id_factory.next(weapon_component.config.name.clone()),
            Visibility::Hidden, // Melee weapons can be invisible or shown only during attack
        ))
        .add_rollback()
        .id();

    commands.entity(character_entity).add_child(entity);

    entity
}

// SYSTEM: MELEE ATTACK HITBOX SPAWNING
pub fn spawn_melee_hitbox(
    commands: &mut Commands,
    attacker_entity: Entity,
    attacker_net_id: &GgrsNetId,
    attacker_transform: &fixed_math::FixedTransform3D,
    facing_direction: &FacingDirection,
    melee_weapon: &MeleeWeapon,
    current_frame: u32,
    collision_settings: &Res<CollisionSettings>,
    owner_handle: Option<PlayerHandle>,
    id_factory: &mut ResMut<GgrsNetIdFactory>,
    global_assets: &Res<GlobalAsset>,
    spritesheet_assets: &Res<Assets<SpriteSheetConfig>>,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let config = &melee_weapon.config;
    
    // Calculate hitbox position based on facing direction and range
    // Use the direction's vector to determine offset
    let direction_vec = facing_direction.to_vector();
    let direction_fixed = fixed_math::FixedVec2::new(
        fixed_math::Fixed::from_num(direction_vec.x),
        fixed_math::Fixed::from_num(direction_vec.y),
    );
    
    let offset = direction_fixed * config.range * fixed_math::FIXED_HALF;
    let hitbox_position = fixed_math::FixedVec3::new(
        attacker_transform.translation.x + offset.x,
        attacker_transform.translation.y + offset.y,
        attacker_transform.translation.z,
    );
    
    let hitbox_transform = fixed_math::FixedTransform3D::new(
        hitbox_position,
        attacker_transform.rotation.clone(),
        fixed_math::FixedVec3::ONE,
    );
    
    // Determine hitbox shape based on attack pattern
    let hitbox_collider = match config.attack_pattern {
        MeleeAttackPattern::SingleStrike | MeleeAttackPattern::Combo { .. } => {
            Collider {
                shape: ColliderShape::Circle {
                    radius: config.range * fixed_math::new(0.6),
                },
                offset: fixed_math::FixedVec3::ZERO,
            }
        }
        MeleeAttackPattern::Sweep { .. } => {
            Collider {
                shape: ColliderShape::Circle {
                    radius: config.range * fixed_math::new(0.8),
                },
                offset: fixed_math::FixedVec3::ZERO,
            }
        }
        MeleeAttackPattern::Thrust => {
            Collider {
                shape: ColliderShape::Rectangle {
                    width: config.range,
                    height: config.range * fixed_math::new(0.5),
                },
                offset: fixed_math::FixedVec3::ZERO,
            }
        }
    };
    
    let g_id = id_factory.next(format!("melee_hitbox_{}", config.name));
    
    info!(
        "{} melee hitbox spawned at {} by {:?}",
        g_id, hitbox_position, owner_handle
    );
    
    let layer = if owner_handle.is_some() {
        collision_settings.bullet_layer // Reuse bullet layer for player melee attacks
    } else {
        collision_settings.enemy_layer // Enemy melee attacks
    };
    
    let hitbox_entity = commands
        .spawn((
            MeleeHitbox {
                damage: config.damage,
                knockback_force: config.knockback_force,
                owner_entity: attacker_entity,
                owner_net_id: attacker_net_id.clone(),
                owner_handle,
                created_frame: current_frame,
                duration_frames: config.attack_duration_frames,
            },
            hitbox_collider,
            CollisionLayer(layer),
            hitbox_transform.to_bevy_transform(),
            hitbox_transform.clone(),
            g_id,
        ))
        .add_rollback()
        .id();
    
    // Spawn slash visual effect
    let slash_spritesheet = spritesheet_assets.get(&global_assets.slash_effect_spritesheet);
    if let Some(slash_config) = slash_spritesheet {
        let texture_handle: Handle<Image> = asset_server.load(&slash_config.path);
        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(slash_config.tile_size.0, slash_config.tile_size.1),
            slash_config.columns,
            slash_config.rows,
            None,
            None,
        );
        let layout_handle = texture_atlas_layouts.add(layout);
        
        // Position effect at hitbox location
        let mut effect_transform = hitbox_transform.to_bevy_transform();
        effect_transform.translation.z = 5.0; // Place above everything
        
        // For 8-directional slashes, we need to handle flipping carefully
        // The sprite is designed for right-facing attacks (0 degrees)
        // For left-facing, we flip and use the opposite angle
        let (flip_x, flip_y, rotation_angle) = match facing_direction {
            FacingDirection::Right => (false, false, 0.0),
            FacingDirection::UpRight => (false, false, std::f32::consts::PI / 4.0),
            FacingDirection::Up => (false, false, std::f32::consts::PI / 2.0),
            FacingDirection::UpLeft => (true, false, -std::f32::consts::PI / 4.0),  // Flip + negative angle for upper left
            FacingDirection::Left => (true, false, 0.0),  // Flip + 0° for left
            FacingDirection::DownLeft => (true, false, std::f32::consts::PI / 4.0),  // Flip + positive angle for lower left
            FacingDirection::Down => (false, false, -std::f32::consts::PI / 2.0),
            FacingDirection::DownRight => (false, false, -std::f32::consts::PI / 4.0),
        };
        
        effect_transform.rotation = Quat::from_rotation_z(rotation_angle);
        
        commands.spawn((
            SlashEffect {
                start_frame: current_frame,
                duration_frames: 9, // 3 frames * 3 ticks each = 9 frames total
            },
            Sprite {
                image: texture_handle,
                texture_atlas: Some(TextureAtlas {
                    layout: layout_handle,
                    index: 0,
                }),
                flip_x,
                flip_y,
                ..default()
            },
            effect_transform,
        ));
    }
    
    hitbox_entity
}

// SYSTEM: UPDATE MELEE HITBOXES (despawn when duration expires)
pub fn update_melee_hitboxes(
    mut commands: Commands,
    frame: Res<FrameCount>,
    hitbox_query: Query<(&GgrsNetId, Entity, &MeleeHitbox), With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "melee_hitbox_update");
    let _enter = system_span.enter();
    
    for (g_id, entity, hitbox) in order_iter!(hitbox_query) {
        let frames_alive = frame.frame - hitbox.created_frame;
        
        if frames_alive >= hitbox.duration_frames {
            info!("{} melee hitbox despawned after {} frames", g_id, frames_alive);
            commands.entity(entity).despawn();
        }
    }
}

// SYSTEM: UPDATE SLASH VISUAL EFFECTS
pub fn update_slash_effects(
    mut commands: Commands,
    frame: Res<FrameCount>,
    mut slash_query: Query<(Entity, &SlashEffect, &mut Sprite)>,
) {
    for (entity, slash_effect, mut sprite) in slash_query.iter_mut() {
        let frames_alive = frame.frame - slash_effect.start_frame;
        
        // Update animation frame (3 frames, 3 ticks each)
        let animation_frame = (frames_alive / 3).min(2) as usize;
        
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = animation_frame;
        }
        
        // Despawn when animation is complete
        if frames_alive >= slash_effect.duration_frames {
            commands.entity(entity).despawn();
        }
    }
}

// SYSTEM: MELEE HITBOX COLLISION DETECTION
pub fn melee_hitbox_collision_system(
    frame: Res<FrameCount>,
    mut commands: Commands,
    settings: Res<CollisionSettings>,
    hitbox_query: Query<
        (
            &GgrsNetId,
            Entity,
            &fixed_math::FixedTransform3D,
            &MeleeHitbox,
            &Collider,
            &CollisionLayer,
        ),
        With<Rollback>,
    >,
    mut target_query: Query<
        (
            Entity,
            &fixed_math::FixedTransform3D,
            &Collider,
            &CollisionLayer,
            &GgrsNetId,
            Option<&Health>,
            Option<&mut DamageAccumulator>,
            Option<&mut Velocity>,
            Option<&Player>,
            Option<&Enemy>,
        ),
        (Without<MeleeHitbox>, With<Rollback>),
    >,
    mut attacker_query: Query<(&GgrsNetId, &mut MeleeAttackState, &fixed_math::FixedTransform3D), With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "melee_collisions");
    let _enter = system_span.enter();
    
    for (hitbox_g_id, _hitbox_entity, hitbox_transform, hitbox, hitbox_collider, hitbox_layer) in
        order_iter!(hitbox_query)
    {
        // Find the attacker by their GgrsNetId
        let mut attacker_data = None;
        for (attacker_net_id, attack_state, attacker_transform) in attacker_query.iter_mut() {
            if attacker_net_id == &hitbox.owner_net_id {
                attacker_data = Some((attack_state, attacker_transform));
                break;
            }
        }
        
        let Some((mut attacker_state, attacker_transform)) = attacker_data else {
            continue;
        };
        
        for (
            target_entity,
            target_transform,
            target_collider,
            target_layer,
            target_g_id,
            opt_health,
            opt_accumulator_mut,
            opt_velocity_mut,
            opt_player,
            opt_enemy,
        ) in target_query.iter_mut()
        {
            // Skip if this is the attacker
            if target_entity == hitbox.owner_entity {
                continue;
            }
            
            // Skip if already hit this entity in this attack
            if attacker_state.has_hit_entity(target_g_id) {
                continue;
            }
            
            // Check layer collision compatibility
            if !settings.layer_matrix[hitbox_layer.0][target_layer.0] {
                continue;
            }
            
            // Skip if player attacking player or enemy attacking enemy
            if hitbox.owner_handle.is_some() && opt_player.is_some() {
                continue;
            }
            if hitbox.owner_handle.is_none() && opt_enemy.is_some() {
                continue;
            }
            
            // Check collision
            if is_colliding(
                &hitbox_transform.translation,
                hitbox_collider,
                &target_transform.translation,
                target_collider,
            ) {
                info!(
                    "Melee hitbox {} hit target {} for {} damage",
                    hitbox_g_id, target_g_id, hitbox.damage
                );
                
                // Apply damage
                if opt_health.is_some() {
                    let hit_by = if let Some(handle) = hitbox.owner_handle {
                        vec![HitBy::Player(handle), HitBy::Entity(hitbox_g_id.clone())]
                    } else {
                        vec![HitBy::Entity(hitbox_g_id.clone())]
                    };
                    
                    if let Some(mut accumulator) = opt_accumulator_mut {
                        accumulator.total_damage = accumulator.total_damage.saturating_add(hitbox.damage);
                        accumulator.hit_count += 1;
                        accumulator.last_hit_by = Some(hit_by);
                    } else {
                        commands.entity(target_entity).insert(DamageAccumulator {
                            hit_count: 1,
                            total_damage: hitbox.damage,
                            last_hit_by: Some(hit_by),
                        });
                    }
                    
                    // Apply knockback
                    if let Some(mut velocity) = opt_velocity_mut {
                        // Calculate direction from attacker to target
                        let attacker_pos = attacker_transform.translation.truncate();
                        let target_pos = target_transform.translation.truncate();
                        let knockback_direction = (target_pos - attacker_pos).normalize_or_zero();
                        
                        // Apply knockback force
                        let knockback_velocity = knockback_direction * hitbox.knockback_force;
                        velocity.0 = velocity.0 + knockback_velocity;
                        
                        info!(
                            "Applied knockback force {} in direction {:?} to target {}",
                            hitbox.knockback_force, knockback_direction, target_g_id
                        );
                    }
                    
                    // Mark entity as hit
                    attacker_state.add_hit_entity(target_g_id.clone());
                }
            }
        }
    }
}

// SYSTEM: PLAYER MELEE ATTACK HANDLING
pub fn player_melee_attack_system(
    mut commands: Commands,
    frame: Res<FrameCount>,
    inputs: Res<bevy_ggrs::PlayerInputs<PeerConfig>>,
    collision_settings: Res<CollisionSettings>,
    mut id_factory: ResMut<GgrsNetIdFactory>,
    global_assets: Res<GlobalAsset>,
    spritesheet_assets: Res<Assets<SpriteSheetConfig>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut player_query: Query<
        (
            Entity,
            &GgrsNetId,
            &Player,
            &fixed_math::FixedTransform3D,
            &FacingDirection,
            &Children,
            &mut MeleeAttackState,
        ),
        With<Rollback>,
    >,
    melee_weapon_query: Query<&MeleeWeapon, With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "player_melee_attack");
    let _enter = system_span.enter();
    
    for (entity, net_id, player, transform, facing_direction, children, mut attack_state) in
        player_query.iter_mut()
    {
        let (input, _status) = inputs[player.handle];
        
        // Check if melee attack button is pressed
        let wants_melee_attack = input.buttons & INPUT_MELEE_ATTACK != 0;
        
        // Find melee weapon in children
        let mut melee_weapon_opt: Option<&MeleeWeapon> = None;
        for child in children.iter() {
            if let Ok(weapon) = melee_weapon_query.get(child) {
                melee_weapon_opt = Some(weapon);
                break;
            }
        }
        
        if let Some(melee_weapon) = melee_weapon_opt {
            let config = &melee_weapon.config;
            
            // Update attack state
            if attack_state.is_attacking {
                // Check if attack duration has expired
                if let Some(start_frame) = attack_state.attack_started_frame {
                    let frames_since_start = frame.frame - start_frame;
                    if frames_since_start >= config.attack_duration_frames {
                        attack_state.end_attack(frame.frame);
                    }
                }
            } else if wants_melee_attack && attack_state.can_attack(frame.frame, config.cooldown_frames) {
                // Start new attack
                attack_state.start_attack(frame.frame);
                
                // Spawn hitbox
                spawn_melee_hitbox(
                    &mut commands,
                    entity,
                    net_id,
                    transform,
                    facing_direction,
                    melee_weapon,
                    frame.frame,
                    &collision_settings,
                    Some(player.handle),
                    &mut id_factory,
                    &global_assets,
                    &spritesheet_assets,
                    &asset_server,
                    &mut texture_atlas_layouts,
                );
                
                info!(
                    "Player {} started melee attack with {} at frame {}",
                    player.handle, config.name, frame.frame
                );
            }
        }
    }
}

// SYSTEM: ENEMY MELEE ATTACK HANDLING
pub fn enemy_melee_attack_system(
    mut commands: Commands,
    frame: Res<FrameCount>,
    collision_settings: Res<CollisionSettings>,
    mut id_factory: ResMut<GgrsNetIdFactory>,
    global_assets: Res<GlobalAsset>,
    spritesheet_assets: Res<Assets<SpriteSheetConfig>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut enemy_query: Query<
        (
            Entity,
            &GgrsNetId,
            &fixed_math::FixedTransform3D,
            &FacingDirection,
            &Children,
            &mut MeleeAttackState,
        ),
        (With<Enemy>, With<Rollback>),
    >,
    player_query: Query<&fixed_math::FixedTransform3D, (With<Player>, Without<Enemy>)>,
    melee_weapon_query: Query<&MeleeWeapon, With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "enemy_melee_attack");
    let _enter = system_span.enter();
    
    for (entity, net_id, transform, facing_direction, children, mut attack_state) in enemy_query.iter_mut()
    {
        // Find melee weapon in children
        let mut melee_weapon_opt: Option<&MeleeWeapon> = None;
        for child in children.iter() {
            if let Ok(weapon) = melee_weapon_query.get(child) {
                melee_weapon_opt = Some(weapon);
                break;
            }
        }
        
        if let Some(melee_weapon) = melee_weapon_opt {
            let config = &melee_weapon.config;
            let enemy_pos = transform.translation.truncate();
            
            // Update attack state
            if attack_state.is_attacking {
                // Check if attack duration has expired
                if let Some(start_frame) = attack_state.attack_started_frame {
                    let frames_since_start = frame.frame - start_frame;
                    if frames_since_start >= config.attack_duration_frames {
                        attack_state.end_attack(frame.frame);
                    }
                }
            } else if attack_state.can_attack(frame.frame, config.cooldown_frames) {
                // Check if any player is in range
                let mut player_in_range = false;
                
                for player_transform in player_query.iter() {
                    let player_pos = player_transform.translation.truncate();
                    let distance = enemy_pos.distance(&player_pos);
                    
                    // Attack if player is within range
                    if distance <= config.range * fixed_math::new(1.2) {
                        player_in_range = true;
                        break;
                    }
                }
                
                if player_in_range {
                    // Start new attack
                    attack_state.start_attack(frame.frame);
                    
                    // Spawn hitbox
                    spawn_melee_hitbox(
                        &mut commands,
                        entity,
                        net_id,
                        transform,
                        facing_direction,
                        melee_weapon,
                        frame.frame,
                        &collision_settings,
                        None, // No player handle for enemies
                        &mut id_factory,
                        &global_assets,
                        &spritesheet_assets,
                        &asset_server,
                        &mut texture_atlas_layouts,
                    );
                    
                    info!(
                        "Enemy entity {:?} started melee attack with {} at frame {}",
                        entity, config.name, frame.frame
                    );
                }
            }
        }
    }
}