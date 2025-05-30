pub mod config;
pub mod create;
pub mod dash;
pub mod enemy;
pub mod health;
pub mod movement;
pub mod player;

use animation::set_sprite_flip;
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_ggrs::{RollbackApp, GgrsSchedule, ReadInputs};
use leafwing_input_manager::plugin::InputManagerPlugin;
use map::game::entity::map::enemy_spawn::EnemySpawnerComponent;

use crate::{
    character::{
        config::CharacterConfig,
        dash::DashState,
        enemy::{
            ai::pathing::{
                calculate_paths, check_direct_paths, move_enemies, update_enemy_targets, EnemyPath,
                PathfindingConfig,
            },
            spawning::{enemy_spawn_from_spawners_system, EnemySpawnerState},
            Enemy,
        },
        health::{
            rollback_apply_accumulated_damage, rollback_apply_death, ui::update_health_bars,
            DamageAccumulator, Death, Health,
        },
        movement::{SprintState, Velocity},
        player::{
            control::PlayerAction,
            input::{
                apply_friction, apply_inputs, move_characters, read_local_inputs,
                update_animation_state, PointerWorldPosition,
            },
            Player,
        },
    },
    system_set::RollbackSystemSet,
};

#[derive(Component, Clone, Copy, Default)]
pub struct Character;

pub struct BaseCharacterGamePlugin {}

impl Plugin for BaseCharacterGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RonAssetPlugin::<CharacterConfig>::new(&["ron"]),));

        app.add_plugins(InputManagerPlugin::<PlayerAction>::default());
        app.init_resource::<PointerWorldPosition>();

        app.init_resource::<PathfindingConfig>();

        app.rollback_resource_with_clone::<PathfindingConfig>()
            .rollback_component_with_clone::<EnemySpawnerComponent>()
            .rollback_component_with_clone::<EnemySpawnerState>()
            .rollback_component_with_clone::<EnemyPath>()
            .rollback_resource_with_copy::<PointerWorldPosition>()
            .rollback_component_with_clone::<Health>()
            .rollback_component_with_clone::<DamageAccumulator>()
            .rollback_component_with_clone::<DashState>()
            .rollback_component_with_clone::<SprintState>()
            .rollback_component_with_clone::<Velocity>()
            .rollback_component_with_clone::<Death>()
            .rollback_component_with_reflect::<Player>()
            .rollback_component_with_reflect::<Enemy>();

        app.add_systems(ReadInputs, read_local_inputs);

        app.add_systems(Update, (set_sprite_flip, update_health_bars));

        app.add_systems(
            GgrsSchedule,
            (
                // HANDLE ALL PLAYERS INPUT
                (apply_inputs,).in_set(RollbackSystemSet::Input),
                // MOVEMENT CHARACTERS
                (apply_friction, move_characters.after(apply_friction))
                    .in_set(RollbackSystemSet::Movement),
                // HEALTH
                (
                    rollback_apply_accumulated_damage,
                    rollback_apply_death.after(rollback_apply_accumulated_damage),
                )
                    .in_set(RollbackSystemSet::DeathManagement),
                // ANIMATION CRATE
                (update_animation_state,).in_set(RollbackSystemSet::AnimationUpdates),
                // SPAWING
                (enemy_spawn_from_spawners_system,).in_set(RollbackSystemSet::EnemySpawning),
                // LOGIC OF ENEMY
                (
                    update_enemy_targets,
                    check_direct_paths.after(update_enemy_targets),
                    calculate_paths.after(check_direct_paths),
                    move_enemies.after(calculate_paths),
                )
                    .in_set(RollbackSystemSet::EnemyAI),
            ),
        );
    }
}
