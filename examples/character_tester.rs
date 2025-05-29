mod args;

use args::get_args;
use bevy::{
    asset::AssetMetaCheck,
    log::LogPlugin,
    prelude::*,
    window::WindowResolution,
};
use game::{
    frame::FrameDebugUIPlugin,
    jjrs::{GggrsConnectionConfiguration, GggrsSessionConfiguration},
    plugins::BaseZombieGamePlugin,
};

use utils::{self, web::WebPlugin};

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let (local_port, mut nbr_player, players, _, matchbox, lobby, cid) = get_args();

    #[cfg(not(target_arch = "wasm32"))]
    let _logging_guard = utils::logs::setup_logging(Some(cid.clone())).unwrap();

    if nbr_player == 0 {
        nbr_player = players.len()
    }

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

    let default_plugings = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            #[cfg(target_arch = "wasm32")]
            file_path: format!("{}/assets", env!("APP_VERSION")),
            ..Default::default()
        })
        .disable::<LogPlugin>()
        .set(window_plugin);

    App::new()
        .add_plugins(default_plugings)
        .add_plugins(WebPlugin {})
        .add_plugins(FrameDebugUIPlugin)
        .add_plugins(BaseZombieGamePlugin::new(!matchbox.is_empty()))
        .insert_resource(GggrsSessionConfiguration {
            cid,
            matchbox: !matchbox.is_empty(),
            lobby: lobby.clone(),
            matchbox_url: matchbox.clone(),
            connection: GggrsConnectionConfiguration {
                input_delay: 5,
                max_player: nbr_player,
                desync_interval: 10,
                socket: players.len() > 1,
                udp_port: local_port,
            },
            players,
        })
        .run();
}
