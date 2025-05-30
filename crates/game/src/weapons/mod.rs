pub mod ui;

use animation::{create_child_sprite, AnimationBundle, FacingDirection, SpriteSheetConfig};
use bevy::{log::{tracing::span, Level}, platform::collections::{HashMap, HashSet}, prelude::*};
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_fixed::{fixed_math, rng::RollbackRng};
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule, PlayerInputs, Rollback, RollbackApp};
use ggrs::PlayerHandle;

use serde::{Deserialize, Serialize};
use utils::{
    bmap, net_id::{GgrsNetId, GgrsNetIdFactory}, order_iter, order_mut_iter
};

use crate::{
    character::{
        dash::DashState,
        health::{DamageAccumulator, Health, HitBy},
        movement::SprintState,
        player::{
            input::{
                CursorPosition, INPUT_DASH, INPUT_RELOAD, INPUT_SPRINT, INPUT_SWITCH_WEAPON_MODE,
            },
            jjrs::PeerConfig,
            Player,
        },
    },
    collider::{is_colliding, Collider, ColliderShape, CollisionLayer, CollisionSettings, Wall},
    global_asset::GlobalAsset,
    system_set::RollbackSystemSet,
    GAME_SPEED,
};
use std::fmt;
use utils::frame::FrameCount;

// COMPONENTS
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FiringMode {
    Automatic {}, // Hold trigger to continuously fire
    Manual {},    // One shot per trigger pull
    Burst {
        pellets_per_shot: u32,
        cooldown_frames: u32,
    }, // Fire a fixed number of shots per trigger pull
    Shotgun {
        pellet_count: u32,
        spread_angle: fixed_math::Fixed,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MagBulletConfig {
    Mag { mag_size: u32, mag_limit: u32 },
    Magless { bullet_limit: u32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BulletType {
    Standard {
        damage: fixed_math::Fixed,
        speed: fixed_math::Fixed,
    },
    Explosive {
        damage: fixed_math::Fixed,
        speed: fixed_math::Fixed,
        blast_radius: fixed_math::Fixed,
        explosive_damage_multiplier: fixed_math::Fixed,
    },
    Piercing {
        damage: fixed_math::Fixed,
        speed: fixed_math::Fixed,
        penetration: u8,
    },
}

impl fmt::Display for BulletType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BulletType::Standard { .. } => write!(f, "Standard"),
            BulletType::Explosive { .. } => write!(f, "Explosive"),
            BulletType::Piercing { .. } => write!(f, "Piercing"),
        }
    }
}

#[derive(Component)]
pub struct ExplosiveTag;

#[derive(Component)]
pub struct PiercingTag;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FiringModeConfig {
    pub firing_rate: fixed_math::Fixed,
    pub firing_mode: FiringMode,
    pub spread: fixed_math::Fixed,
    pub recoil: fixed_math::Fixed,
    pub bullet_type: BulletType,
    pub range: fixed_math::Fixed,

    pub reload_time_seconds: fixed_math::Fixed,
    pub mag: MagBulletConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeaponConfig {
    pub name: String,
    pub default_firing_mode: String,
    pub firing_modes: HashMap<String, FiringModeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeaponSpriteConfig {
    pub name: String,
    pub index: usize,

    pub weapon_offset: fixed_math::FixedVec2,

    pub bullet_offset_left: fixed_math::FixedVec2,
    pub bullet_offset_right: fixed_math::FixedVec2,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WeaponAsset {
    pub config: WeaponConfig,
    pub sprite_config: WeaponSpriteConfig,
}

// Component for a weapon
#[derive(Component, Debug, Clone)]
pub struct Weapon {
    pub config: WeaponConfig,
    pub sprite_config: WeaponSpriteConfig,
}

impl From<WeaponAsset> for Weapon {
    fn from(value: WeaponAsset) -> Self {
        Self {
            config: value.config,
            sprite_config: value.sprite_config,
        }
    }
}

#[derive(Component, Clone, Serialize, Deserialize)]
pub struct HitMarker {
    pub target: Entity,
    pub damage: fixed_math::Fixed,
}

#[derive(Component)]
pub struct VisualEffectRequest {
    pub effect_type: EffectType,
    pub position: fixed_math::FixedVec2,
    pub scale: fixed_math::Fixed,
}

#[derive(Clone)]
pub enum EffectType {
    BulletHit,
    Explosion,
    Piercing,
}

#[derive(Component, Clone, Serialize, Deserialize)]
pub struct ExplosionMarker {
    pub radius: fixed_math::Fixed,
    pub damage: fixed_math::Fixed,
    pub player_handle: PlayerHandle,
    pub processed: bool, // Flag to ensure one-time processing
}

/// Component to mark an entity as the active weapon
#[derive(Component)]
pub struct ActiveWeapon;

/// Component for bullets
#[derive(Component, Clone)]
pub struct Bullet {
    pub velocity: fixed_math::FixedVec2,
    pub bullet_type: BulletType,
    pub damage: fixed_math::Fixed,
    pub range: fixed_math::Fixed,
    pub distance_traveled: fixed_math::Fixed,
    pub player_handle: PlayerHandle,
    pub created_at: u32,
}

/// Component to track the player's weapon inventory
#[derive(Component, Debug, Clone, Default)]
pub struct WeaponInventory {
    pub active_weapon_index: usize,
    pub frame_switched: u32,
    pub frame_switched_mode: u32,
    pub weapons: Vec<(Entity, Weapon)>, // Store entity handles and weapon data

    pub reloading_ending_frame: Option<u32>,
}

impl WeaponInventory {
    pub fn active_weapon(&self) -> &(Entity, Weapon) {
        self.weapons.get(self.active_weapon_index).unwrap()
    }
}

#[derive(Reflect, Default, Clone)]
pub struct WeaponModeState {
    pub mag_ammo: u32,
    pub mag_quantity: u32,

    pub burst_shots_left: u32,

    pub mag_size: u32,
    pub burst_cooldown: bool,
}

#[derive(Component, Reflect, Default, Clone)]
pub struct WeaponModesState {
    pub modes: HashMap<String, WeaponModeState>,
}

// Component to track rollbackable state for weapons
#[derive(Component, Reflect, Default, Clone)]
pub struct WeaponState {
    pub last_fire_frame: u32,
    pub is_firing: bool,
    pub active_mode: String,
}

#[derive(Event)]
pub struct FireWeaponEvent {
    pub player_entity: Entity,
}

// ASSETS

#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct WeaponsConfig(pub HashMap<String, WeaponAsset>);

// UTILITY FUNCTION

impl WeaponModeState {
    // Do the reloading of the ammo when the reloading process is over or some other event
    pub fn reload(&mut self) {
        if self.mag_quantity > 0 {
            self.mag_quantity -= 1;
            self.mag_ammo = self.mag_size;
        }
    }

    pub fn is_mag_full(&self) -> bool {
        self.mag_ammo == self.mag_size
    }
}

impl WeaponModesState {
    pub fn reload(&mut self, mode: &String) {
        if let Some(mode) = self.modes.get_mut(mode) {
            mode.reload();
        }
    }
}

impl WeaponInventory {
    pub fn is_reloading(&self) -> bool {
        self.reloading_ending_frame.is_some()
    }

    pub fn is_reloading_over(&self, current_frame: u32) -> bool {
        self.reloading_ending_frame
            .map_or_else(|| true, |f| current_frame >= f)
    }

    pub fn clear_reloading(&mut self) {
        self.reloading_ending_frame = None;
    }

    pub fn start_reload(
        &mut self,
        current_game_frame: u32,
        reload_time_seconds: fixed_math::Fixed,
    ) {
        self.reloading_ending_frame = {
            if reload_time_seconds <= fixed_math::new(0.0) {
                None
            } else {
                let frames_to_reload =
                    (reload_time_seconds * bevy_fixed::fixed_math::new(60.)).ceil();
                if frames_to_reload == 0 {
                    // Ensure at least one frame for very short reload times
                    Some(current_game_frame + 1)
                } else {
                    Some(current_game_frame + frames_to_reload.to_num::<u32>())
                }
            }
        };
    }
}

// start the reload process

// Function to spawn weapon , all weapon should be spawn on the user when they got them
pub fn spawn_weapon_for_player(
    commands: &mut Commands,
    global_assets: &Res<GlobalAsset>,

    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: &Res<Assets<SpriteSheetConfig>>,

    active: bool,

    player_entity: Entity,
    weapon: WeaponAsset,
    inventory: &mut WeaponInventory,

    id_factory: &mut ResMut<GgrsNetIdFactory>,
) -> Entity {
    let map_layers = global_assets
        .spritesheets
        .get(&weapon.sprite_config.name)
        .unwrap()
        .clone();
    let animation_handle = global_assets
        .animations
        .get(&weapon.sprite_config.name)
        .unwrap()
        .clone();

    let animation_bundle = AnimationBundle::new(
        map_layers.clone(),
        animation_handle.clone(),
        weapon.sprite_config.index,
        bmap!("body" => String::new()),
    );

    let mut weapon_state = WeaponState::default();
    let mut weapon_modes_state = WeaponModesState::default();
    weapon_state.active_mode = weapon.config.default_firing_mode.clone();
    for (k, v) in weapon.config.firing_modes.iter() {
        let mut weapon_mode_state = WeaponModeState::default();
        match v.mag {
            MagBulletConfig::Mag {
                mag_size,
                mag_limit,
            } => {
                weapon_mode_state.mag_ammo = mag_size;
                weapon_mode_state.mag_quantity = mag_limit;
                weapon_mode_state.mag_size = mag_size;
            }
            MagBulletConfig::Magless { bullet_limit } => {
                weapon_mode_state.mag_ammo = bullet_limit;
            }
        };

        weapon_modes_state
            .modes
            .insert(k.clone(), weapon_mode_state);
    }

    let weapon: Weapon = weapon.into();

    let transform = Transform::from_translation(
        fixed_math::fixed_to_vec2(weapon.sprite_config.weapon_offset).extend(0.),
    )
    .with_rotation(Quat::IDENTITY);
    let ggrs_transform = fixed_math::FixedTransform3D::from_bevy_transform(&transform);

    let entity = commands
        .spawn((
            transform,
            ggrs_transform,
            weapon_state,
            weapon_modes_state,
            weapon.clone(),
            animation_bundle,
            id_factory.next(weapon.config.name.clone()),
        ))
        .add_rollback()
        .id();

    let spritesheet_config = sprint_sheet_assets
        .get(map_layers.get("body").unwrap())
        .unwrap();
    create_child_sprite(
        commands,
        asset_server,
        texture_atlas_layouts,
        entity,
        spritesheet_config,
        0,
    );

    inventory.weapons.push((entity, weapon));

    if active {
        commands
            .entity(entity)
            .insert((ActiveWeapon {}, Visibility::Inherited));
        inventory.active_weapon_index = inventory.weapons.len() - 1;
    } else {
        commands.entity(entity).insert(Visibility::Hidden);
    }

    commands.entity(player_entity).add_child(entity);

    entity
}

fn spawn_bullet_rollback(
    commands: &mut Commands,
    weapon: &Weapon,
    player_transform: &fixed_math::FixedTransform3D,
    weapon_transform: &fixed_math::FixedTransform3D,
    facing_direction: &FacingDirection,
    direction: fixed_math::FixedVec2,
    bullet_type: BulletType,
    range: fixed_math::Fixed,
    player_handle: PlayerHandle,
    current_frame: u32,
    _collision_settings: &Res<CollisionSettings>,
    parent_layer: &CollisionLayer,
    id_factory: &mut ResMut<GgrsNetIdFactory>,
) -> Entity {
    let (velocity, damage, range, radius) = match &bullet_type {
        BulletType::Standard {
            speed,
            damage: damage_bullet,
        } => (
            direction * (*speed / *GAME_SPEED),
            *damage_bullet,
            range,
            fixed_math::new(5.0),
        ),
        BulletType::Explosive {
            speed,
            damage: damage_bullet,
            ..
        } => (
            direction * (*speed / *GAME_SPEED),
            *damage_bullet,
            range,
            fixed_math::new(8.0),
        ),
        BulletType::Piercing {
            speed,
            damage: damage_bullet,
            ..
        } => (
            direction * (*speed / *GAME_SPEED),
            *damage_bullet,
            range,
            fixed_math::new(5.0),
        ),
    };

    let color = match &bullet_type {
        BulletType::Standard { .. } => Color::BLACK,
        BulletType::Explosive { .. } => Color::WHITE,
        BulletType::Piercing { .. } => Color::BLACK,
    };

    let local_muzzle_offset_v2 = if matches!(facing_direction, FacingDirection::Right) {
        weapon.sprite_config.bullet_offset_right
    } else {
        weapon.sprite_config.bullet_offset_left
    };

    // 1. Muzzle offset in weapon's local 3D space
    let local_muzzle_offset_v3 = fixed_math::FixedVec3 {
        x: local_muzzle_offset_v2.x,
        y: local_muzzle_offset_v2.y,
        z: fixed_math::Fixed::ZERO,
    };

    // 2. Transform muzzle offset by weapon's local rotation (relative to player)
    //    and add weapon's local translation (relative to player)
    //    to get muzzle position in player's local coordinate system.
    let weapon_local_rotation_mat3: fixed_math::FixedMat3 = weapon_transform.rotation.clone();
    let weapon_local_translation_v3: fixed_math::FixedVec3 = weapon_transform.translation;

    let muzzle_pos_in_player_space =
        weapon_local_rotation_mat3.mul_vec3(local_muzzle_offset_v3) + weapon_local_translation_v3;

    // 3. Transform muzzle position from player's local space to world space.
    let player_world_rotation_mat3: fixed_math::FixedMat3 = player_transform.rotation.clone();
    let player_world_translation_v3: fixed_math::FixedVec3 = player_transform.translation;

    let world_firing_position = player_world_rotation_mat3.mul_vec3(muzzle_pos_in_player_space)
        + player_world_translation_v3;

    // 4. Calculate projectile's world rotation.
    // This is player's world rotation combined with weapon's local rotation.
    let projectile_world_rotation =
        player_world_rotation_mat3.mul_mat3(&weapon_local_rotation_mat3); // Ensure mul_mat3 is the correct operation

    // 5. Create the projectile's transform.
    let new_projectile_fixed_transform = fixed_math::FixedTransform3D::new(
        world_firing_position,
        projectile_world_rotation,
        fixed_math::FixedVec3::ONE,
    );

    let g_id = id_factory.next(format!("{}", bullet_type));

    info!(
        "{} spawn at {} by {}",
        g_id, new_projectile_fixed_transform.translation, player_handle
    );

    let mut entity_commands = commands.spawn((
        Sprite::from_color(color, Vec2::new(10.0, 10.0)),
        Bullet {
            velocity,
            bullet_type,
            damage,
            range,
            distance_traveled: fixed_math::Fixed::ZERO,
            player_handle,
            created_at: current_frame,
        },
        Collider {
            offset: fixed_math::FixedVec3::ZERO,
            shape: ColliderShape::Circle { radius },
        },
        CollisionLayer(parent_layer.0),
        new_projectile_fixed_transform.to_bevy_transform(),
        new_projectile_fixed_transform,
        g_id,
    ));

    match bullet_type {
        BulletType::Explosive { .. } => {
            entity_commands.insert(ExplosiveTag);
        }
        BulletType::Piercing { .. } => {
            entity_commands.insert(PiercingTag);
        }
        _ => {}
    };

    entity_commands.add_rollback().id()
}

// SYSTEMS

// Rollback system to correctly transform the weapon based on the position
pub fn system_weapon_position(
    query: Query<(&Children, &CursorPosition, &FacingDirection), With<Rollback>>,
    mut query_weapon: Query<&mut fixed_math::FixedTransform3D, With<ActiveWeapon>>,
) {
    for (childs, cursor_position, _direction) in query.iter() {
        for child in childs.iter() {
            if let Ok(mut transform) = query_weapon.get_mut(child.clone()) {
                let cursor_game_world_pos = fixed_math::FixedVec3::new(
                    fixed_math::new(cursor_position.x as f32),
                    fixed_math::new(cursor_position.y as f32),
                    fixed_math::new(0.0),
                );
                let direction_to_target_fixed =
                    (cursor_game_world_pos - transform.translation).normalize();
                let angle_radians_fixed = fixed_math::atan2_fixed(
                    direction_to_target_fixed.y,
                    direction_to_target_fixed.x,
                );

                transform.rotation = fixed_math::FixedMat3::from_rotation_z(angle_radians_fixed);
            }
        }
    }
}

// rollback system for weapon action , firing and all
pub fn weapon_rollback_system(
    mut commands: Commands,
    mut rng: ResMut<RollbackRng>,
    inputs: Res<PlayerInputs<PeerConfig>>,
    frame: Res<FrameCount>,

    mut inventory_query: Query<(
        Entity,
        &mut WeaponInventory,
        &SprintState,
        &DashState,
        &CollisionLayer,
        &fixed_math::FixedTransform3D,
        &Player,
    )>,
    mut weapon_query: Query<(
        &mut Weapon,
        &mut WeaponState,
        &mut WeaponModesState,
        &fixed_math::FixedTransform3D,
        &ChildOf,
    )>,

    player_query: Query<(&fixed_math::FixedTransform3D, &FacingDirection, &Player)>,

    collision_settings: Res<CollisionSettings>,

    mut id_factory: ResMut<GgrsNetIdFactory>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "weapon");
    let _enter = system_span.enter(); // Enter the span

    // Process weapon firing for all players
    for (_entity, mut inventory, sprint_state, dash_state, collision_layer, transform, player) in
        inventory_query.iter_mut()
    {
        let (input, _input_status) = inputs[player.handle];

        // Do nothing if no weapons
        if inventory.weapons.is_empty() {
            continue;
        }

        if sprint_state.is_sprinting
            || dash_state.is_dashing
            || input.buttons & INPUT_SPRINT != 0
            || input.buttons & INPUT_DASH != 0
        {
            continue;
        }

        // Nothing to do for weapon if we are sprinting

        // Get active weapon
        let (weapon_entity, _) = inventory.weapons[inventory.active_weapon_index];

        // Get the entity for the active weapon
        if let Ok((weapon, mut weapon_state, mut weapon_modes_state, weapon_transform, child_of)) =
            weapon_query.get_mut(weapon_entity)
        {
            let active_mode = weapon_state.active_mode.clone();
            let weapon_config = weapon.config.firing_modes.get(&active_mode).unwrap();

            if input.buttons & INPUT_SWITCH_WEAPON_MODE != 0 {
                if let Some(new_mode) = weapon_modes_state
                    .modes
                    .keys()
                    .find(|&x| *x != weapon_state.active_mode)
                {
                    if inventory.frame_switched_mode + 20 < frame.frame
                        && inventory.frame_switched + 20 < frame.frame
                    {
                        inventory.frame_switched_mode = frame.frame;
                        weapon_state.active_mode = new_mode.clone();

                        continue;
                    }
                }
            }

            let weapon_mode_state = weapon_modes_state.modes.get_mut(&active_mode).unwrap();

            // Check if reloading and update progress,
            if inventory.is_reloading() {
                if inventory.is_reloading_over(frame.frame) {
                    weapon_mode_state.reload();
                    inventory.clear_reloading();
                } else {
                    continue;
                }
            } else if input.buttons & INPUT_RELOAD != 0 && !weapon_mode_state.is_mag_full() {
                inventory.start_reload(frame.frame, weapon_config.reload_time_seconds);
                continue;
            }

            // Handle switching of weapons, will start firing on the next frame
            if input.switch_weapon && !inventory.weapons.is_empty() {
                let new_index = (inventory.active_weapon_index + 1) % inventory.weapons.len();

                if new_index != inventory.active_weapon_index
                    && inventory.frame_switched + 20 < frame.frame
                    && inventory.frame_switched_mode + 20 < frame.frame
                {
                    inventory.active_weapon_index = new_index;
                    inventory.frame_switched = frame.frame;

                    continue;
                }
            }

            // TODO: fix only support two mode, take the first that is not the current
            if input.fire {
                // Calculate fire rate in frames (60 FPS assumed) , need to be configure via ressource instead
                let frame_per_shot =
                    (bevy_fixed::fixed_math::new(60.) / weapon_config.firing_rate).to_num::<u32>();
                let current_frame = frame.frame;
                let frames_since_last_shot = current_frame - weapon_state.last_fire_frame;

                let (can_fire, empty) = match weapon_config.firing_mode {
                    FiringMode::Automatic { .. } => (
                        frames_since_last_shot >= frame_per_shot,
                        weapon_mode_state.mag_ammo == 0,
                    ),

                    FiringMode::Manual { .. } => (
                        !weapon_state.is_firing && frames_since_last_shot >= frame_per_shot,
                        weapon_mode_state.mag_ammo == 0,
                    ),

                    FiringMode::Burst {
                        pellets_per_shot,
                        cooldown_frames,
                    } => {
                        if weapon_mode_state.burst_shots_left > 0
                            && frames_since_last_shot >= frame_per_shot
                        {
                            // Continue ongoing burst
                            (true, weapon_mode_state.mag_ammo == 0)
                        } else if weapon_mode_state.burst_shots_left == 0 {
                            if !weapon_state.is_firing
                                && !weapon_mode_state.burst_cooldown
                                && frames_since_last_shot >= cooldown_frames
                            {
                                // Start new burst when trigger is pulled
                                weapon_mode_state.burst_shots_left = pellets_per_shot;
                                (true, weapon_mode_state.mag_ammo == 0)
                            } else if weapon_mode_state.burst_cooldown
                                && frames_since_last_shot >= cooldown_frames
                            {
                                // Reset cooldown
                                weapon_mode_state.burst_cooldown = false;
                                (false, false)
                            } else {
                                (false, false)
                            }
                        } else {
                            (false, false)
                        }
                    }

                    FiringMode::Shotgun { .. } => {
                        if !weapon_state.is_firing && frames_since_last_shot >= frame_per_shot {
                            // Shotgun fires all pellets at once, so we don't need burst_shots_left
                            (true, weapon_mode_state.mag_ammo == 0)
                        } else {
                            (false, false)
                        }
                    }
                };

                if empty {
                    inventory.start_reload(frame.frame, weapon_config.reload_time_seconds);
                    continue;
                }

                weapon_state.is_firing = true;

                if can_fire {
                    if let Ok((_, facing_direction, _)) = player_query.get(child_of.parent()) {
                        let mut aim_dir = fixed_math::FixedVec2::new(
                            fixed_math::Fixed::from_num(input.pan_x),
                            fixed_math::Fixed::from_num(input.pan_y),
                        );
                        aim_dir.x /= fixed_math::new(127.0);
                        aim_dir.y /= fixed_math::new(127.0);
                        aim_dir = aim_dir.normalize();

                        match weapon_config.firing_mode {
                            FiringMode::Shotgun {
                                pellet_count,
                                spread_angle,
                            } => {
                                // Fire multiple pellets in a spread pattern
                                for _ in 0..pellet_count {
                                    // Calculate a random angle within the spread range
                                    let random_fixed_val = rng.next_fixed();
                                    let offset_from_center =
                                        random_fixed_val.saturating_sub(fixed_math::FIXED_HALF);
                                    let pellet_angle_fixed =
                                        offset_from_center.saturating_mul(spread_angle);

                                    // Create the fixed-point 2D rotation matrix
                                    let fixed_spread_rotation =
                                        fixed_math::FixedMat2::from_angle(pellet_angle_fixed);

                                    // Apply the rotation to the fixed-point aim direction
                                    let direction = fixed_spread_rotation.mul_vec2(aim_dir);

                                    spawn_bullet_rollback(
                                        &mut commands,
                                        &weapon,
                                        transform,
                                        weapon_transform,
                                        facing_direction,
                                        direction,
                                        weapon_config.bullet_type,
                                        weapon_config.range,
                                        player.handle,
                                        frame.frame,
                                        &collision_settings,
                                        collision_layer,
                                        &mut id_factory,
                                    );
                                }
                                weapon_mode_state.mag_ammo -= 1; // Shotgun uses one ammo for all pellets
                                inventory
                                    .start_reload(frame.frame, weapon_config.reload_time_seconds);
                            }
                            _ => {
                                let random_fixed_val = rng.next_fixed();
                                let offset_from_center =
                                    random_fixed_val.saturating_sub(fixed_math::FIXED_HALF);
                                let pellet_angle_fixed =
                                    offset_from_center.saturating_mul(fixed_math::FIXED_ONE);

                                let fixed_spread_rotation =
                                    fixed_math::FixedMat2::from_angle(pellet_angle_fixed);
                                let direction = fixed_spread_rotation.mul_vec2(aim_dir);

                                spawn_bullet_rollback(
                                    &mut commands,
                                    &weapon,
                                    transform,
                                    weapon_transform,
                                    facing_direction,
                                    direction,
                                    weapon_config.bullet_type,
                                    weapon_config.range,
                                    player.handle,
                                    frame.frame,
                                    &collision_settings,
                                    collision_layer,
                                    &mut id_factory,
                                );
                                weapon_mode_state.mag_ammo -= 1;

                                if matches!(weapon_config.firing_mode, FiringMode::Burst { .. })
                                    && weapon_mode_state.burst_shots_left > 0
                                {
                                    weapon_mode_state.burst_shots_left -= 1;

                                    // Set cooldown when burst finishes
                                    if weapon_mode_state.burst_shots_left == 0 {
                                        weapon_mode_state.burst_cooldown = true;
                                    }
                                }
                            }
                        }
                        weapon_state.last_fire_frame = frame.frame;
                    }
                }
            } else {
                weapon_state.is_firing = false;
            }
        }
    }
}

pub fn bullet_rollback_system(
    mut commands: Commands,
    frame: Res<FrameCount>,
    mut bullet_query: Query<(
        &GgrsNetId,
        Entity,
        &mut fixed_math::FixedTransform3D,
        &mut Bullet,
    )>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "bullet_movement");
    let _enter = system_span.enter();

    for (g_id, entity, mut transform, mut bullet) in order_mut_iter!(bullet_query) {
        // Move bullet based on velocity (fixed timestep)
        let delta = bullet.velocity;

        // Apply movement
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;

        bullet.distance_traveled += delta.length();

        if bullet.distance_traveled >= bullet.range {
            info!(
                "{} despawn after travelleing {}",
                g_id, bullet.distance_traveled
            );
            commands.entity(entity).despawn();
        }
    }
}
pub fn bullet_rollback_collision_system(
    frame: Res<FrameCount>,
    mut commands: Commands,
    settings: Res<CollisionSettings>,
    bullet_query: Query<
        (
            &GgrsNetId,
            Entity,
            &fixed_math::FixedTransform3D,
            &Bullet,
            &Collider,
            &CollisionLayer,
        ),
        With<Rollback>,
    >,
    // Query for colliders. We'll need mutable access to DamageAccumulator later.
    mut collider_query: Query<
        (
            Entity,
            &fixed_math::FixedTransform3D,
            &Collider,
            &CollisionLayer,
            &GgrsNetId,
            Option<&Wall>,
            Option<&Health>,
            Option<&mut DamageAccumulator>,
        ),
        (Without<Bullet>, With<Rollback>),
    >,
) {
    let system_span = span!(
        Level::INFO,
        "ggrs",
        f = frame.frame,
        s = "bullet_collissions"
    );
    let _enter = system_span.enter();

    let mut bullets_to_despawn_set = HashSet::new();

    for (ggrs_net_id, bullet_entity, bullet_transform, bullet, bullet_collider, bullet_layer) in
       order_iter!(bullet_query) 
    {
        if bullets_to_despawn_set.contains(&bullet_entity) {
            continue;
        }

        let mut actual_collided_target_entities: Vec<(Entity, GgrsNetId)> = Vec::new();

        // Phase 1: Identify ALL entities this bullet is colliding with (immutable pass first)
        for (
            target_entity,
            target_transform,
            target_collider,
            target_layer,
            collider_net_id,
            _opt_wall,
            _opt_health,
            /* no mut here */ _,
        ) in collider_query.iter()
        {
            if !settings.layer_matrix[bullet_layer.0][target_layer.0] {
                continue;
            }
            if is_colliding(
                &bullet_transform.translation,
                bullet_collider,
                &target_transform.translation,
                target_collider,
            ) {
                actual_collided_target_entities.push((target_entity, collider_net_id.clone()));
            }
        }

        if actual_collided_target_entities.is_empty() {
            continue; // No collision for this bullet
        }

        // Sort the collided entities to pick the "first" one deterministically
        actual_collided_target_entities.sort_unstable_by_key(|(_, g_id)| g_id.0);

        // The bullet will interact with the first entity in this sorted list.
        let (deterministic_target_entity, deterministic_target_g_id) =
            actual_collided_target_entities[0].clone();

        info!(
            "bullet {} collissions with {:?}",
            ggrs_net_id, deterministic_target_g_id
        );
        // Now get mutable components for this specific, deterministically chosen target
        if let Ok((
            _target_entity_refetch,
            _target_transform,
            _target_collider,
            _target_layer,
            _,
            opt_wall,
            opt_health,
            opt_accumulator_mut,
        )) = collider_query.get_mut(deterministic_target_entity)
        {
            if opt_health.is_some() {
                let last_hit_by = Some(vec![
                    HitBy::Player(bullet.player_handle),
                    HitBy::Entity(ggrs_net_id.clone()),
                ]);
                if let Some(mut accumulator) = opt_accumulator_mut {
                    accumulator.total_damage =
                        accumulator.total_damage.saturating_add(bullet.damage);
                    accumulator.hit_count += 1;
                    accumulator.last_hit_by = last_hit_by;
                } else {
                    commands
                        .entity(deterministic_target_entity)
                        .insert(DamageAccumulator {
                            hit_count: 1,
                            total_damage: bullet.damage,
                            last_hit_by,
                        });
                }
            }

            let mut should_bullet_despawn_now = false;
            match bullet.bullet_type {
                BulletType::Standard { .. } => {
                    should_bullet_despawn_now = true;
                }
                BulletType::Explosive { .. } => {
                    should_bullet_despawn_now = true;
                }
                BulletType::Piercing { .. } => {
                    if opt_wall.is_some() {
                        // Piercing bullets despawn on walls
                        should_bullet_despawn_now = true;
                    }
                    // If piercing bullets should continue through enemies, this logic is fine.
                    // They would only despawn if they hit a wall (or run out of pierce, etc.)
                }
            }

            if should_bullet_despawn_now {
                bullets_to_despawn_set.insert(bullet_entity);
                // Since the original code had a `break` here, we effectively stop processing
                // more targets for this bullet after this first deterministic interaction.
            }
        }
    }

    // Deterministic despawning of bullets (already good)
    let mut bullets_to_despawn_vec: Vec<Entity> = bullets_to_despawn_set.into_iter().collect();
    bullets_to_despawn_vec.sort_by_key(|entity| entity.index()); // Or .to_bits()
    for entity in bullets_to_despawn_vec {
        commands.entity(entity).despawn();
    }
}

// Non rollback system to display the weapon correct sprite
pub fn weapon_inventory_system(
    mut commands: Commands,
    query: Query<(Entity, &mut WeaponInventory)>,
    mut weapon_entities: Query<(Entity, &mut Visibility), With<Weapon>>,
) {
    for (_player_entity, inventory) in query.iter() {
        if inventory.weapons.is_empty() {
            continue;
        }

        // Update active/inactive weapon visibility
        for (i, (weapon_entity, _)) in inventory.weapons.iter().enumerate() {
            let is_active = i == inventory.active_weapon_index;

            // For simplicity, we're using commands to add/remove components
            // In a real implementation, you might want to use a Visibility component
            if let Ok((_, mut visibility)) = weapon_entities.get_mut(*weapon_entity) {
                if is_active {
                    commands.entity(*weapon_entity).insert(ActiveWeapon);
                    *visibility = Visibility::Visible;
                } else {
                    commands.entity(*weapon_entity).remove::<ActiveWeapon>();
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

pub fn update_weapon_sprite_direction(
    mut query_sprite: Query<&mut Sprite>,
    query_players: Query<(&Children, &FacingDirection)>,
    query_weapons: Query<&Children, With<ActiveWeapon>>,
) {
    for (childs, direction) in query_players.iter() {
        for child in childs.iter() {
            if let Ok(childs) = query_weapons.get(child.clone()) {
                for child in childs.iter() {
                    if let Ok(mut sprite) = query_sprite.get_mut(child.clone()) {
                        match direction {
                            FacingDirection::Left => {
                                sprite.flip_y = true;
                            }
                            FacingDirection::Right => {
                                sprite.flip_y = false;
                            }
                        };
                    }
                }
            }
        }
    }
}

pub fn weapons_config_update_system(
    _asset_server: Res<AssetServer>,

    weapons_config: Res<Assets<WeaponsConfig>>,

    mut ev_asset: EventReader<AssetEvent<WeaponsConfig>>,

    mut query_weapons: Query<(&Children, Entity, &mut Weapon)>,
) {
    for event in ev_asset.read() {
        if let AssetEvent::Modified { id } = event {
            if let Some(weapons_config) = weapons_config.get(*id) {
                for (_childs, _entity, mut weapon) in query_weapons.iter_mut() {
                    if let Some(config) = weapons_config.0.get(&weapon.config.name) {
                        weapon.config = config.config.clone();
                        weapon.sprite_config = config.sprite_config.clone();
                    }
                }
            }
        }
    }
}

pub struct BaseWeaponGamePlugin {}

impl Plugin for BaseWeaponGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(self::ui::WeaponDebugUIPlugin);

        app.add_plugins(RonAssetPlugin::<WeaponsConfig>::new(&["ron"]));

        app.rollback_component_with_clone::<WeaponInventory>()
            .rollback_component_with_clone::<WeaponModesState>()
            .rollback_component_with_clone::<WeaponState>()
            .rollback_component_with_clone::<Bullet>();

        app.add_systems(
            Update,
            (
                update_weapon_sprite_direction,
                weapon_inventory_system,
                weapons_config_update_system,
            ),
        );

        app.add_systems(
            GgrsSchedule,
            (
                system_weapon_position,
                weapon_rollback_system.after(system_weapon_position),
                bullet_rollback_system.after(weapon_rollback_system),
                bullet_rollback_collision_system.after(bullet_rollback_system),
            )
                .in_set(RollbackSystemSet::Weapon),
        );
    }
}
