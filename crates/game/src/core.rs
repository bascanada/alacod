
use animation::D2AnimationPlugin;
use bevy::{
    app::PluginGroupBuilder, asset::AssetMetaCheck, log::LogPlugin, prelude::*,
    window::WindowResolution,
};
use bevy_fixed::{
    fixed_math::{self, sync_bevy_transforms_from_fixed},
    rng::RollbackRng,
};
use bevy_ggrs::{GgrsApp, GgrsPlugin, GgrsSchedule, RollbackApp};
use serde::{Deserialize, Serialize};
use utils::{
    frame::FrameCount,
    net_id::{GgrsNetId, GgrsNetIdFactory},
    web::WebPlugin,
};

use crate::{
    audio::ZAudioPlugin, camera::CameraControlPlugin, character::{player::jjrs::PeerConfig, BaseCharacterGamePlugin}, collider::BaseColliderGamePlugin, frame::{increase_frame_system, FrameDebugUIPlugin}, global_asset::{add_global_asset, loading_asset_system}, jjrs::{log_ggrs_events, setup_ggrs_local, start_matchbox_socket, wait_for_players}, light::ZLightPlugin, system_set::RollbackSystemSet, weapons::BaseWeaponGamePlugin
};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CoreSetupConfig {
    pub app_name: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Loading,
    LobbyLocal,
    LobbyOnline,
    InGame,
}

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OnlineState {
    #[default]
    Unset,
    Online,
    Offline,
}

#[derive(Debug, Clone, Resource)]
pub struct GameInfo {
    pub version: String,
}

impl Default for GameInfo {
    fn default() -> Self {
        Self {
            version: env!("APP_VERSION").into(),
        }
    }
}

#[derive(Default)]
pub struct CoreSetupPlugin(pub CoreSetupConfig);

impl Plugin for CoreSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ZLightPlugin);
        app.add_plugins(ZAudioPlugin);
        app.add_plugins(WebPlugin);
        app.add_plugins(FrameDebugUIPlugin);
        app.add_plugins(D2AnimationPlugin);
        app.add_plugins(CameraControlPlugin);
        app.add_plugins(GgrsPlugin::<PeerConfig>::default());

        app.add_plugins(BaseWeaponGamePlugin {});
        app.add_plugins(BaseColliderGamePlugin {});
        app.add_plugins(BaseCharacterGamePlugin {});

        app.init_resource::<GameInfo>();
        app.init_resource::<GgrsNetIdFactory>();
        app.init_resource::<FrameCount>();

        app.init_state::<AppState>();
        app.set_rollback_schedule_fps(60);

        app.rollback_resource_with_copy::<RollbackRng>()
            .rollback_resource_with_clone::<GgrsNetIdFactory>()
            .rollback_resource_with_copy::<FrameCount>()
            .rollback_component_with_clone::<fixed_math::FixedTransform3D>()
            .rollback_component_with_clone::<GgrsNetId>();

        app.configure_sets(
            GgrsSchedule,
            (
                RollbackSystemSet::Input,
                RollbackSystemSet::Movement,
                RollbackSystemSet::Weapon,
                RollbackSystemSet::CollisionDamage,
                RollbackSystemSet::DeathManagement,
                RollbackSystemSet::AnimationUpdates,
                RollbackSystemSet::EnemySpawning,
                RollbackSystemSet::EnemyAI,
                RollbackSystemSet::FrameCounter,
            )
                .chain(),
        );

        app.add_systems(Startup, add_global_asset);

        app.add_systems(
            Update,
            (
                sync_bevy_transforms_from_fixed.run_if(in_state(AppState::InGame)),
                loading_asset_system.run_if(in_state(AppState::Loading)),
            ),
        );

        app.add_systems(OnEnter(AppState::LobbyOnline), start_matchbox_socket);
        app.add_systems(
            Update,
            wait_for_players.run_if(in_state(AppState::LobbyOnline)),
        );

        app.add_systems(OnEnter(AppState::LobbyLocal), setup_ggrs_local);

        app.add_systems(Update, log_ggrs_events.run_if(in_state(AppState::InGame)));

        app.add_systems(
            GgrsSchedule,
            (increase_frame_system,).in_set(RollbackSystemSet::FrameCounter),
        );
    }
}

impl CoreSetupPlugin {
    pub fn get_default_plugin(&self) -> PluginGroupBuilder {
        let window_plugin = WindowPlugin {
            primary_window: Some(Window {
                title: self.0.app_name.to_string(),
                resolution: WindowResolution::new(800., 600.),

                resizable: true,
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#bevy-canvas".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                #[cfg(target_arch = "wasm32")]
                file_path: format!("{}/assets", env!("APP_VERSION")),
                ..Default::default()
            })
            .disable::<LogPlugin>()
            .set(window_plugin)
    }
}
