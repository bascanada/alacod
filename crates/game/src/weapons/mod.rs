pub mod ui;

use animation::{create_child_sprite, AnimationBundle, FacingDirection, SpriteSheetConfig};
use bevy::{math::VectorSpace, prelude::*, utils::{HashMap, HashSet}};
use bevy_ggrs::{AddRollbackCommandExtension, PlayerInputs, Rollback};
use ggrs::PlayerHandle;
use serde::{Deserialize, Serialize};
use utils::{bmap, rng::RollbackRng, fixed_math};

use crate::{character::{dash::DashState, health::{self, DamageAccumulator, Health}, movement::SprintState, player::{input::{CursorPosition, INPUT_DASH, INPUT_RELOAD, INPUT_SPRINT, INPUT_SWITCH_WEAPON_MODE}, jjrs::PeerConfig, Player}}, collider::{is_colliding, Collider, ColliderShape, CollisionLayer, CollisionSettings, Wall}, frame::FrameCount, global_asset::GlobalAsset, GAME_SPEED};


// COMPONENTSo
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FiringMode {
    Automatic{},  // Hold trigger to continuously fire
    Manual{},     // One shot per trigger pull
    Burst{pellets_per_shot: u32, cooldown_frames: u32},      // Fire a fixed number of shots per trigger pull
    Shotgun{pellet_count: u32, spread_angle: fixed_math::Fixed},
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MagBulletConfig {
    Mag {
        mag_size: u32,
        mag_limit: u32,
    },
    Magless {
        bullet_limit: u32,
    },
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
        Self { config: value.config, sprite_config: value.sprite_config }
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
#[derive(Component, Debug, Clone)]
pub struct WeaponInventory {
    pub active_weapon_index: usize,
    pub frame_switched: u32,
    pub frame_switched_mode: u32,
    pub weapons: Vec<(Entity, Weapon)>,  // Store entity handles and weapon data

    pub reloading_ending_frame: Option<u32>,
}

impl Default for WeaponInventory {
    fn default() -> Self {
        Self {
            active_weapon_index: 0,
            frame_switched: 0,
            frame_switched_mode: 0,
            reloading_ending_frame: None,
            weapons: Vec::new(),
        }
    }
}

impl WeaponInventory {
    pub fn active_weapon(&self) -> &(Entity, Weapon) {
        return self.weapons.get(self.active_weapon_index).unwrap();
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

// Rollback state for bullets
#[derive(Component, Clone)]
pub struct BulletRollbackState {
    spawn_frame: u32,
    initial_position: fixed_math::FixedVec2,
    direction: fixed_math::FixedVec2,
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
        self.reloading_ending_frame.map_or_else(|| true, |f| current_frame >= f)
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
                let frames_to_reload = (reload_time_seconds * utils::fixed_math::new(60.)).ceil() ;
                if frames_to_reload == 0 { // Ensure at least one frame for very short reload times
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
) -> Entity {

    let map_layers = global_assets.spritesheets.get(&weapon.sprite_config.name).unwrap().clone();
    let animation_handle = global_assets.animations.get(&weapon.sprite_config.name).unwrap().clone();

    let animation_bundle =
        AnimationBundle::new(map_layers.clone(), animation_handle.clone(), weapon.sprite_config.index, bmap!("body" => String::new()));

    let mut weapon_state = WeaponState::default();
    let mut weapon_modes_state = WeaponModesState::default();
    weapon_state.active_mode = weapon.config.default_firing_mode.clone();
    for (k, v) in weapon.config.firing_modes.iter() {
        let mut weapon_mode_state = WeaponModeState::default();
        match v.mag {
            MagBulletConfig::Mag { mag_size, mag_limit } => {
                weapon_mode_state.mag_ammo = mag_size;
                weapon_mode_state.mag_quantity = mag_limit;
                weapon_mode_state.mag_size = mag_size;
            },
            MagBulletConfig::Magless {  bullet_limit } => {
                weapon_mode_state.mag_ammo = bullet_limit;
            },
        };

        weapon_modes_state.modes.insert(k.clone(), weapon_mode_state);
    }

    let weapon: Weapon = weapon.into();

    let transform = Transform::from_translation(fixed_math::fixed_to_vec2(weapon.sprite_config.weapon_offset).extend(0.)).with_rotation(Quat::IDENTITY);
    let ggrs_transform = fixed_math::FixedTransform3D::from_bevy_transform(&transform);

    let entity = commands.spawn((
        transform,
        ggrs_transform,
        weapon_state,
        weapon_modes_state,
        weapon.clone(),
        animation_bundle,
    )).add_rollback().id();


    let spritesheet_config = sprint_sheet_assets.get(map_layers.get("body").unwrap()).unwrap();
    create_child_sprite(
        commands,
        &asset_server,
        texture_atlas_layouts,
        entity.clone(), &spritesheet_config, 0);

    inventory.weapons.push((entity.clone(), weapon));

    if active {
        commands.entity(entity).insert((ActiveWeapon{}, Visibility::Inherited));
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
    collision_settings: &Res<CollisionSettings>,
    parent_layer: &CollisionLayer,
) -> Entity {
    let (velocity, damage, range, radius) = match &bullet_type {
        BulletType::Standard { speed, damage: damage_bullet } => {
            (direction * (*speed / *GAME_SPEED ), *damage_bullet, range, fixed_math::new(5.0))
        },
        BulletType::Explosive { speed, damage: damage_bullet, blast_radius, explosive_damage_multiplier } => {
            (direction * (*speed / *GAME_SPEED ), *damage_bullet, range, fixed_math::new(8.0))
        },
        BulletType::Piercing { speed, damage: damage_bullet, penetration } => {
            (direction * (*speed / *GAME_SPEED ), *damage_bullet, range, fixed_math::new(5.0))
        }
    };

    let color = match &bullet_type {
        BulletType::Standard { .. } => Color::BLACK,
        BulletType::Explosive { .. } => Color::WHITE,
        BulletType::Piercing { .. } => Color::BLACK,
    };

    let firing_position_v2 = if matches!(facing_direction, FacingDirection::Right) {
        weapon.sprite_config.bullet_offset_right
    } else {
        weapon.sprite_config.bullet_offset_left
    };

    let fixed_weapon_translation: fixed_math::FixedVec3 = weapon_transform.translation;
    let fixed_weapon_rotation_mat3: fixed_math::FixedMat3 = weapon_transform.rotation.clone();
    // And the local firing position as a FixedVec2:
    // let fixed_firing_position_v2_local = FixedVec2::from_f32(bevy_firing_position_v2.x, bevy_firing_position_v2.y);
    // Or directly:
    let fixed_firing_position_v2_local = player_transform.translation.truncate() + weapon_transform.translation.truncate() + firing_position_v2;

    // 1. Extend local firing position_v2 to a 3D local offset:
    let fixed_local_offset_3d = fixed_math::FixedVec3 {
        x: fixed_firing_position_v2_local.x,
        y: fixed_firing_position_v2_local.y,
        z: fixed_math::Fixed::ZERO, // Or whatever fixed-point representation of 0.0 you use
    };

    // 2. Calculate world firing position (equivalent to transform_point):
    // world_point = (rotation * local_point) + translation
    let rotated_offset_fixed = fixed_weapon_rotation_mat3.mul_vec3(fixed_local_offset_3d);

    // Ensure FixedVec3 has operator overloading for addition
    // If FixedVec3 doesn't have `+` overloaded, you'd do:
    // let world_firing_position_fixed = FixedVec3 {
    //     x: rotated_offset_fixed.x.saturating_add(fixed_weapon_translation.x),
    //     y: rotated_offset_fixed.y.saturating_add(fixed_weapon_translation.y),
    //     z: rotated_offset_fixed.z.saturating_add(fixed_weapon_translation.z),
    // };
    // Assuming it does (based on FixedVec2):
    let world_firing_position_fixed = rotated_offset_fixed + fixed_weapon_translation;
    // `world_firing_position_fixed` is your fixed-point equivalent of `firing_position`


    // 3. Get weapon's world rotation:
    // This is already `fixed_weapon_rotation_mat3`.
    // Bevy's `weapon_world_rotation` is a Quat. `fixed_weapon_rotation_mat3` is its matrix form.
    let projectile_rotation_fixed_mat3 = fixed_weapon_rotation_mat3;


    // 4. Create a new "transform" concept for the projectile using these fixed-point values.
    // If you're using the FixedTransform3D struct:
    let new_projectile_fixed_transform = fixed_math::FixedTransform3D::new(
        world_firing_position_fixed,
        projectile_rotation_fixed_mat3,
        fixed_math::FixedVec3::ONE, 
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
            created_at: current_frame
        },
        BulletRollbackState {
            spawn_frame: current_frame,
            initial_position: firing_position_v2,
            direction,
        },
        Collider {
            offset: fixed_math::FixedVec3::ZERO,
            shape: ColliderShape::Circle { radius },
        },
        CollisionLayer(parent_layer.0),
        new_projectile_fixed_transform.to_bevy_transform(),
        new_projectile_fixed_transform,
    ));

    match bullet_type {
        BulletType::Explosive { .. } =>  { entity_commands.insert(ExplosiveTag); },
        BulletType::Piercing { .. } => { entity_commands.insert(PiercingTag); },
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
    for (childs, cursor_position, direction) in query.iter() {
        for child in childs.iter() {
            if let Ok(mut transform) = query_weapon.get_mut(*child) {
                let cursor_game_world_pos = fixed_math::FixedVec3::new(fixed_math::new(cursor_position.x as f32), fixed_math::new(cursor_position.y as f32), fixed_math::new(0.0));
                let direction_to_target_fixed = (cursor_game_world_pos - transform.translation).normalize();
                let angle_radians_fixed = fixed_math::atan2_fixed(direction_to_target_fixed.y, direction_to_target_fixed.x);
                        
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

    mut inventory_query: Query<(Entity, &mut WeaponInventory, &SprintState, &DashState, &CollisionLayer, &fixed_math::FixedTransform3D, &Player)>,
    mut weapon_query: Query<(&mut Weapon, &mut WeaponState, &mut WeaponModesState, &fixed_math::FixedTransform3D, &Parent)>,

    player_query: Query<(&fixed_math::FixedTransform3D, &FacingDirection, &Player)>,

    collision_settings: Res<CollisionSettings>,
) {
    // Process weapon firing for all players
    for (entity,  mut inventory, sprint_state, dash_state , collision_layer, transform, player) in inventory_query.iter_mut() {
        let (input, _input_status) = inputs[player.handle];

        // Do nothing if no weapons
        if inventory.weapons.is_empty() {
            continue;
        }

        if sprint_state.is_sprinting || dash_state.is_dashing || input.buttons & INPUT_SPRINT != 0 || input.buttons & INPUT_DASH != 0 {
            continue;
        }


        // Nothing to do for weapon if we are sprinting
        
        // Get active weapon
        let (weapon_entity, _) = inventory.weapons[inventory.active_weapon_index];


        // Get the entity for the active weapon
        if let Ok((mut weapon, mut weapon_state, mut weapon_modes_state, weapon_transform, parent)) = weapon_query.get_mut(weapon_entity) {
            let active_mode = weapon_state.active_mode.clone();
            let weapon_config = weapon.config.firing_modes.get(&active_mode).unwrap();

            if input.buttons & INPUT_SWITCH_WEAPON_MODE != 0 {
                if let Some(new_mode) = weapon_modes_state.modes.keys().find(|&x| *x != weapon_state.active_mode) {
                    if inventory.frame_switched_mode + 20 < frame.frame && inventory.frame_switched + 20 < frame.frame {
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
            } else if input.buttons & INPUT_RELOAD != 0  && !weapon_mode_state.is_mag_full() {
                inventory.start_reload(frame.frame, weapon_config.reload_time_seconds);
                continue;
            }

            // Handle switching of weapons, will start firing on the next frame
            if input.switch_weapon && !inventory.weapons.is_empty(){
                let new_index = (inventory.active_weapon_index + 1) % inventory.weapons.len();

                if new_index != inventory.active_weapon_index &&
                    inventory.frame_switched + 20 < frame.frame &&
                    inventory.frame_switched_mode + 20 < frame.frame  {

                    inventory.active_weapon_index = new_index;
                    inventory.frame_switched = frame.frame;

                    continue;
                }
            }

            // TODO: fix only support two mode, take the first that is not the current
            if input.fire {
                // Calculate fire rate in frames (60 FPS assumed) , need to be configure via ressource instead
                let frame_per_shot = ((utils::fixed_math::new(60.) / weapon_config.firing_rate)).to_num::<u32>();
                let current_frame = frame.frame;
                let frames_since_last_shot = current_frame - weapon_state.last_fire_frame;

               let (can_fire, empty) = match weapon_config.firing_mode {
                    FiringMode::Automatic { .. } => {
                        (frames_since_last_shot >= frame_per_shot, weapon_mode_state.mag_ammo == 0)
                    },
                    
                    FiringMode::Manual { .. } => {
                        (!weapon_state.is_firing && frames_since_last_shot >= frame_per_shot,
                        weapon_mode_state.mag_ammo == 0)
                    },
                    
                    FiringMode::Burst { pellets_per_shot, cooldown_frames } => {
                        if weapon_mode_state.burst_shots_left > 0 && frames_since_last_shot >= frame_per_shot {
                            // Continue ongoing burst
                            (true, weapon_mode_state.mag_ammo == 0)
                        } else if weapon_mode_state.burst_shots_left == 0 {
                            if !weapon_state.is_firing && !weapon_mode_state.burst_cooldown && frames_since_last_shot >= cooldown_frames {
                                // Start new burst when trigger is pulled
                                weapon_mode_state.burst_shots_left = pellets_per_shot;
                                (true, weapon_mode_state.mag_ammo == 0)
                            } else if weapon_mode_state.burst_cooldown && frames_since_last_shot >= cooldown_frames {
                                // Reset cooldown
                                weapon_mode_state.burst_cooldown = false;
                                (false, false)
                            } else {
                                (false, false)
                            }
                        } else {
                            (false, false)
                        }
                    },
                    
                    FiringMode::Shotgun { pellet_count, spread_angle } => {
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
                    if let Ok((_, facing_direction, _)) = player_query.get(**parent) {
                        let mut aim_dir = fixed_math::FixedVec2::new(
                            fixed_math::Fixed::from_num(input.pan_x),
                            fixed_math::Fixed::from_num(input.pan_y),
                        );
                        aim_dir.x = aim_dir.x / fixed_math::new(127.0);
                        aim_dir.y = aim_dir.y / fixed_math::new(127.0);
                        aim_dir = aim_dir.normalize();

                               match weapon_config.firing_mode {
                                    FiringMode::Shotgun { pellet_count, spread_angle } => {
                                        // Fire multiple pellets in a spread pattern
                                        for _ in 0..pellet_count {
                                            // Calculate a random angle within the spread range
                                            let random_fixed_val = rng.next_fixed();
                                            let offset_from_center = random_fixed_val.saturating_sub(fixed_math::FIXED_HALF);
                                            let pellet_angle_fixed = offset_from_center.saturating_mul(spread_angle);

                                            // Create the fixed-point 2D rotation matrix
                                            let fixed_spread_rotation = fixed_math::FixedMat2::from_angle(pellet_angle_fixed);

                                            // Apply the rotation to the fixed-point aim direction
                                            let direction = fixed_spread_rotation.mul_vec2(aim_dir);

                                            spawn_bullet_rollback(
                                                &mut commands,
                                                &weapon,
                                                transform,
                                                weapon_transform,
                                                facing_direction,
                                                direction,
                                                weapon_config.bullet_type.clone(),
                                                weapon_config.range,
                                                player.handle,
                                                frame.frame,
                                                &collision_settings,
                                                collision_layer,
                                            );
                                        }
                                        weapon_mode_state.mag_ammo -= 1; // Shotgun uses one ammo for all pellets
                                        inventory.start_reload(frame.frame, weapon_config.reload_time_seconds);
                                    },
                                    _ => {
                                        let random_fixed_val = rng.next_fixed();
                                        let offset_from_center = random_fixed_val.saturating_sub(fixed_math::FIXED_HALF);
                                        let pellet_angle_fixed = offset_from_center.saturating_mul(fixed_math::FIXED_ONE);

                                        let fixed_spread_rotation = fixed_math::FixedMat2::from_angle(pellet_angle_fixed);
                                        let direction = fixed_spread_rotation.mul_vec2(aim_dir);

                                        spawn_bullet_rollback(
                                            &mut commands,
                                            &weapon,
                                            transform,
                                            weapon_transform,
                                            facing_direction,
                                            direction,
                                            weapon_config.bullet_type.clone(),
                                            weapon_config.range,
                                            player.handle,
                                            frame.frame,
                                            &collision_settings,
                                            collision_layer,
                                        );
                                        weapon_mode_state.mag_ammo -= 1;

                                        if matches!(weapon_config.firing_mode, FiringMode::Burst { .. }) && weapon_mode_state.burst_shots_left > 0 {
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
    mut bullet_query: Query<(Entity, &mut fixed_math::FixedTransform3D, &mut Bullet, &BulletRollbackState)>,
) {
    for (entity, mut transform, mut bullet, bullet_state) in bullet_query.iter_mut() {
        // Move bullet based on velocity (fixed timestep)
        let delta = bullet.velocity; // Assume bullet.velocity is already deterministic for this frame

        // Apply movement
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;


        bullet.distance_traveled += delta.length();

        if bullet.distance_traveled >= bullet.range {
            commands.entity(entity).despawn();
        }
    }
}


fn apply_bullet_dommage(
    commands: &mut Commands,
    target_entity: Entity,
    bullet: &Bullet,
    
    mut opt_dmg_accumulator: Option<Mut<'_, DamageAccumulator, >>
) {
    if let Some(accumulator) = opt_dmg_accumulator.as_mut() {
        // Update existing accumulator
        accumulator.total_damage += bullet.damage;
        accumulator.hit_count += 1;
        accumulator.last_hit_by = Some(health::HitBy::Player(bullet.player_handle))
    } else {
        commands.entity(target_entity).insert(DamageAccumulator{
            hit_count: 1,
            total_damage: bullet.damage,
            last_hit_by: Some(health::HitBy::Player(bullet.player_handle)),
        });
    }
}


pub fn bullet_rollback_collision_system(
    mut commands: Commands,
    settings: Res<CollisionSettings>,
    bullet_query: Query<(Entity, &fixed_math::FixedTransform3D, &Bullet, &Collider, &CollisionLayer), With<Rollback>>,
    mut collider_query: Query<(Entity, &fixed_math::FixedTransform3D, &Collider, &CollisionLayer, Option<&Wall>, Option<&Health>, Option<&mut DamageAccumulator>), (Without<Bullet>, With<Rollback>)>,
) {
    let mut bullets_to_despawn_set = HashSet::new(); // Use HashSet for efficient duplicate avoidance and checks

    for (bullet_entity, bullet_transform, bullet, bullet_collider, bullet_layer) in bullet_query.iter() {
        // Skip already processed bullets that are marked for despawn
        if bullets_to_despawn_set.contains(&bullet_entity) {
            continue;
        }

        // Phase 1: Identify all entities this bullet is colliding with
        for (target_entity, target_transform, target_collider, target_layer, opt_wall, opt_health, opt_accumulator_mut) in collider_query.iter_mut() { // Note: iter() not iter_mut() for the broad phase
            if !settings.layer_matrix[bullet_layer.0 as usize][target_layer.0 as usize] {
                continue;
            }

            // Check for collision using our new helper function
            if is_colliding(&bullet_transform.translation, bullet_collider, &target_transform.translation, target_collider) { 

                if opt_health.is_some() {
                    // Apply damage (using the refactored logic from your apply_bullet_dommage function)
                    if let Some(mut accumulator) = opt_accumulator_mut {
                        // Update existing accumulator
                        accumulator.total_damage += bullet.damage;
                        accumulator.hit_count += 1;
                        accumulator.last_hit_by = Some(health::HitBy::Player(bullet.player_handle));
                    } else {
                        // Insert new accumulator if it doesn't exist
                        commands.entity(target_entity).insert(DamageAccumulator {
                            hit_count: 1,
                            total_damage: bullet.damage,
                            last_hit_by: Some(health::HitBy::Player(bullet.player_handle)),
                        });
                    }
                }

                let mut should_bullet_despawn_now = false;
                match bullet.bullet_type {
                    BulletType::Standard { .. } => {
                        should_bullet_despawn_now = true;
                    },
                    BulletType::Explosive { blast_radius, .. } => {
                        should_bullet_despawn_now = true;
                    },
                    BulletType::Piercing { .. } => {
                        if opt_wall.is_some() {
                            should_bullet_despawn_now = true;
                        }
                    },
                }

                if should_bullet_despawn_now {
                    bullets_to_despawn_set.insert(bullet_entity);
                    break; // Bullet is destroyed, stop processing more targets for this bullet
                }
            }
        }
    }

    // Convert HashSet to Vec and sort by entity ID for deterministic despawning
    let mut bullets_to_despawn_vec: Vec<Entity> = bullets_to_despawn_set.into_iter().collect();
    bullets_to_despawn_vec.sort_by_key(|entity| entity.index());
    
    // Despawn bullets
    for entity in bullets_to_despawn_vec {
        commands.entity(entity).despawn(); // Despawn if the entity still exists
    }
}

// Non rollback system to display the weapon correct sprite
pub fn weapon_inventory_system(
    mut commands: Commands,
    query: Query<(Entity, &mut WeaponInventory)>,
    mut weapon_entities: Query<(Entity, &mut Visibility),  With<Weapon>>,
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
                    commands.entity(*weapon_entity)
                        .insert(ActiveWeapon);
                    *visibility = Visibility::Visible;
                } else {
                    commands.entity(*weapon_entity)
                        .remove::<ActiveWeapon>();
                    *visibility = Visibility::Hidden;
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