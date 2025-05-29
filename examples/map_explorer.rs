use bevy::{asset::AssetMetaCheck, prelude::*, window::WindowResolution};
use bevy_ecs_ldtk::prelude::*;

use map::generation::config::MapGenerationConfig;
use map_ldtk::{
    loader::{get_asset_loader_generation, setup_generated_map},
    plugins::LdtkRoguePlugin,
};

use utils::web::WebPlugin;

use std::env;

fn main() {
    let level_loader = get_asset_loader_generation();

    let map_generation_config = get_config();
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "zrl-map-explorer".to_string(),
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
        .add_systems(Startup, setup_generated_map)
        .add_plugins(WebPlugin {})
        .add_plugins(LdtkRoguePlugin)
        .add_systems(Update, load_levels_if_not_present)
        .insert_resource(map_generation_config)
        .register_asset_loader(level_loader)
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
    let id = ids.first().unwrap();

    let ldtk_project = ldtk_project.get(*id).unwrap();
    let level_iids: Vec<_> = ldtk_project
        .data()
        .iter_raw_levels()
        .map(|l| l.iid.clone())
        .collect();

    let mut level_set = level_set.iter_mut().last().unwrap();
    if !level_set.iids.is_empty() {
        let mut clear = false;
        for iid in level_set.iids.iter() {
            if !level_iids.iter().any(|x| iid.to_string() == *x) {
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
