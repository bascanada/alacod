use bevy::{asset::AssetMetaCheck, prelude::*};
use bevy_ggrs::{prelude::*, GgrsSchedule};
use bevy_kira_audio::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;
use utils::{fixed_math::{self, sync_bevy_transforms_from_fixed}, rng::RollbackRng};
use std::hash::Hash;
use bevy_common_assets::ron::RonAssetPlugin;

use animation::{set_sprite_flip, D2AnimationPlugin};
use bevy_ggrs::GgrsPlugin;


use crate::{
    audio::ZAudioPlugin,
    camera::CameraControlPlugin,
    character::{
        config::CharacterConfig,
        dash::DashState,
        enemy::{
            /*
            ai::pathing::{
                calculate_paths,
                check_direct_paths,
                move_enemies,
                update_enemy_targets,
                EnemyPath,
                PathfindingConfig
            },
            spawning::{
                enemy_spawn_from_spawners_system, EnemySpawnerState
            },*/
            Enemy
        },
        health::{
            rollback_apply_accumulated_damage,
            rollback_apply_death,
            ui::update_health_bars,
            DamageAccumulator, Death, Health}, movement::{SprintState, Velocity}, player::{control::PlayerAction, input::{apply_friction, apply_inputs, move_characters, read_local_inputs, update_animation_state, PointerWorldPosition}, jjrs::PeerConfig, Player}}, collider::{Collider, CollisionLayer, CollisionSettings, Wall}, debug::SpriteDebugOverlayPlugin, frame::{increase_frame_system, FrameCount}, global_asset::{add_global_asset, loading_asset_system}, jjrs::{log_ggrs_events, setup_ggrs_local, start_matchbox_socket, wait_for_players, GggrsSessionConfiguration}, weapons::{bullet_rollback_collision_system, bullet_rollback_system, system_weapon_position, ui::WeaponDebugUIPlugin, weapon_inventory_system, weapon_rollback_system, weapons_config_update_system, Bullet, BulletRollbackState, WeaponInventory, WeaponModesState, WeaponState, WeaponsConfig}};

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Loading,
    Lobby,
    InGame,
}

#[derive(Debug, Clone, Resource)]
pub struct GameInfo {
    pub version: String,
}

impl Default for GameInfo {
    fn default() -> Self {
        Self { version: env!("APP_VERSION").into() }
    } 
}

pub struct BaseZombieGamePlugin { online: bool }

impl BaseZombieGamePlugin {
    pub fn new(online: bool) -> Self {
        Self { online: online }
    }
}

impl Plugin for BaseZombieGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameInfo>();
        app.add_plugins(SpriteDebugOverlayPlugin{});

        app.add_plugins(ZAudioPlugin {});

        app.add_plugins(D2AnimationPlugin);
        app.add_plugins(WeaponDebugUIPlugin);
        app.add_plugins(CameraControlPlugin);

        app.add_plugins((
            RonAssetPlugin::<CharacterConfig>::new(&["ron"]),
            RonAssetPlugin::<WeaponsConfig>::new(&["ron"]),
        ));

        app.add_plugins(InputManagerPlugin::<PlayerAction>::default());
        app.init_resource::<PointerWorldPosition>();


        app.init_resource::<CollisionSettings>();

        app.init_state::<AppState>();

        //app.init_resource::<PathfindingConfig>();
        //app.init_resource::<EnemySpawnSystemHolder>();
        //app.init_resource::<EnemySpawnState>();

        app.set_rollback_schedule_fps(60);
        app.add_plugins(GgrsPlugin::<PeerConfig>::default())
            .rollback_resource_with_copy::<RollbackRng>()
            //.rollback_resource_with_reflect::<PathfindingConfig>()
            //.rollback_resource_with_reflect::<EnemySpawnState>()
            //.rollback_component_with_clone::<EnemySpawnerComponent>()
            //.rollback_component_with_reflect::<EnemyPath>()
            .rollback_resource_with_copy::<PointerWorldPosition>()
            .rollback_resource_with_copy::<FrameCount>()
            .rollback_component_with_clone::<fixed_math::FixedTransform3D>()
            .rollback_component_with_clone::<Health>()
            .rollback_component_with_clone::<DamageAccumulator>()
            .rollback_component_with_clone::<WeaponInventory>()
            .rollback_component_with_clone::<WeaponModesState>()
            .rollback_component_with_clone::<WeaponState>()
            .rollback_component_with_clone::<Bullet>()
            .rollback_component_with_clone::<BulletRollbackState>()
            .rollback_component_with_clone::<Collider>()
            .rollback_component_with_clone::<Wall>()
            .rollback_component_with_clone::<CollisionLayer>()
            .rollback_component_with_clone::<DashState>()
            .rollback_component_with_clone::<SprintState>()
            .rollback_component_with_clone::<Velocity>()
            .rollback_component_with_clone::<Death>()
            .rollback_component_with_reflect::<Player>()
            .rollback_component_with_reflect::<Enemy>();

        app.add_systems(Startup, (add_global_asset));
        app.add_systems(Update, loading_asset_system.run_if(in_state(AppState::Loading)));
        

        if self.online {
            app.add_systems(Startup, start_matchbox_socket.after(add_global_asset));
            app.add_systems(Update, wait_for_players.run_if(in_state(AppState::Lobby)));
            app.add_systems(Update, log_ggrs_events.run_if(in_state(AppState::InGame)));
        } else {
            app.add_systems(OnEnter(AppState::Lobby), setup_ggrs_local.after(add_global_asset));
        }


        app.add_systems(ReadInputs, read_local_inputs);
        app.insert_resource(FrameCount { frame: 0 });
        app.add_systems(
            GgrsSchedule, (
                // HANDLE ALL PLAYERS INPUT
                apply_inputs,
                // MOVEMENT CHARACTERS
                apply_friction.after(apply_inputs),
                move_characters.after(apply_friction),
                // WEAPON
                system_weapon_position.after(move_characters),
                weapon_rollback_system.after(system_weapon_position),
                bullet_rollback_system.after(weapon_rollback_system),
                bullet_rollback_collision_system.after(bullet_rollback_system),
                rollback_apply_accumulated_damage.after(bullet_rollback_collision_system),
                rollback_apply_death.after(rollback_apply_accumulated_damage),
                // ANIMATION CRATE
                set_sprite_flip.after(bullet_rollback_collision_system),
                update_animation_state.after(set_sprite_flip),
                // SPAWING
                //enemy_spawn_from_spawners_system.after(update_animation_state),
                // LOGIC OF ENEMY
                //update_enemy_targets.after(enemy_spawn_from_spawners_system),
                //check_direct_paths.after(update_enemy_targets),
                //calculate_paths.after(check_direct_paths),
                //move_enemies.after(calculate_paths),
                
                increase_frame_system.after(update_animation_state)
            ));
        app.add_systems(Update, (
            sync_bevy_transforms_from_fixed,
            weapon_inventory_system,
            weapons_config_update_system,

            update_health_bars,
        ));
    }
}
