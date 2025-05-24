mod args;

use args::get_args;
use bevy::{asset::AssetMetaCheck, prelude::*, utils::hashbrown::HashMap, window::WindowResolution};
use game::{character::{ movement::Velocity, player::{ control::{get_input_map, PlayerAction}, LocalPlayer, Player}}, collider::{spawn_test_wall, CollisionSettings}, frame::FrameDebugUIPlugin, global_asset::GlobalAsset, jjrs::{GggrsConnectionConfiguration, GggrsSessionConfiguration}, plugins::{AppState, BaseZombieGamePlugin}, weapons::WeaponsConfig};

use utils::{web::WebPlugin};

fn main() {
    
    let (local_port,mut nbr_player, players, _, matchbox, lobby) = get_args();

    if nbr_player == 0 { nbr_player = players.len() }

    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "zrl-character_tester".to_string(),
            resolution: WindowResolution::new(800., 600.),

            resizable: true,
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#bevy-canvas".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    #[cfg(target_arch = "wasm32")]
                    file_path: format!("{}/assets", env!("APP_VERSION")),
                    ..Default::default()
                })
                .set(window_plugin),
        )
        .add_plugins(WebPlugin{})
        .add_plugins(FrameDebugUIPlugin)
        .add_plugins(BaseZombieGamePlugin::new(matchbox != ""))
        .insert_resource(GggrsSessionConfiguration { matchbox: matchbox != "", lobby: lobby.clone(), matchbox_url: matchbox.clone(), connection: GggrsConnectionConfiguration { input_delay: 5, max_player: nbr_player, desync_interval: 10, socket: players.len() > 1, udp_port: local_port}, players: players })
        .run();
}