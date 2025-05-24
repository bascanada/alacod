use bevy::{asset::AssetMetaCheck, prelude::*, window::WindowResolution};
use bevy_ecs_ldtk::prelude::*;

use map::{
    generation::config::MapGenerationConfig,
    ldtk::{
        loader::{get_asset_loader_generation, reload_map, setup_generated_map},
        plugins::{LdtkRoguePlugin, MyWorldInspectorPlugin},
    },
};
use utils::{
    camera::tod::{move_camera, setup_camera},
    web::WebPlugin,
};

use std::time::{SystemTime, UNIX_EPOCH};
use std::env;

fn main() {
    let level_loader = get_asset_loader_generation();

    let map_generation_config = get_config();
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "zrl-map-preview".to_string(),
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
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    #[cfg(target_arch = "wasm32")]
                    file_path: format!("{}/assets", env!("APP_VERSION")),
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest())
                .set(window_plugin),
        )
        .add_plugins(MyWorldInspectorPlugin)
        .add_systems(Startup, (setup_generated_map, setup_camera))
        .add_plugins(LdtkRoguePlugin)
        .insert_resource(map_generation_config)
        .register_asset_loader(level_loader)
        .add_systems(Update, (load_levels_if_not_present, move_camera, keyinput))
        .add_plugins(WebPlugin {})
        .run();
}

// should be a step before the game part
fn load_levels_if_not_present(
    ldtk_project: Res<Assets<LdtkProject>>,
    mut level_set: Query<&mut LevelSet>,
) {
    if ldtk_project.is_empty() {
        return;
    }
    let ids: Vec<_> = ldtk_project.ids().collect();
    let id = ids.get(0).unwrap();

    let ldtk_project = ldtk_project.get(*id).unwrap();
    let level_iids: Vec<_> = ldtk_project
        .data()
        .iter_raw_levels()
        .map(|l| l.iid.clone())
        .collect();

    let mut level_set = level_set.iter_mut().last().unwrap();
    if level_set.iids.len() > 0 {
        let mut clear = false;
        for iid in level_set.iids.iter() {
            if level_iids.iter().find(|x| iid.to_string() == **x).is_none() {
                clear = true;
                break;
            }
        }
        if clear {
            level_set.iids.clear();
        }
    }

    level_iids.iter().for_each(|id| {
        level_set.iids.insert(LevelIid::new(id));
    });
}

fn keyinput(
    input: Res<ButtonInput<KeyCode>>,
    level_query: Query<Entity, With<LevelIid>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut config: ResMut<MapGenerationConfig>,
    mut level_set: Query<&mut LevelSet>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        for level_entity in &level_query {
            commands.entity(level_entity).despawn_recursive()
        }
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        config.seed = since_the_epoch.as_secs() as i32;
        reload_map(&asset_server, config.as_ref());
        for mut level_set in level_set.iter_mut() {
            level_set.iids.clear();
        }
    }
}

fn get_config() -> MapGenerationConfig {
    let args: Vec<String> = env::args().collect();

    let map_path = if args.len() < 2 {
        "exemples/test_map.ldtk".to_string()
    } else {
        args.get(1).unwrap().clone()
    };

    let seed = if args.len() < 3 {
        12345
    } else {
        args.get(2).unwrap().parse::<i32>().unwrap()
    };

    MapGenerationConfig {
        seed,
        map_path,
        max_width: 1000,
        max_heigth: 1000,
        ..Default::default()
    }
}
