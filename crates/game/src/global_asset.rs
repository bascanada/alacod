use animation::{AnimationMapConfig, SpriteSheetConfig};
use bevy::{prelude::*, utils::HashMap};
use utils::bmap;

use crate::{
    camera::CameraSettingsAsset, character::config::CharacterConfig, plugins::AppState,
    weapons::WeaponsConfig,
};

const PLAYER_SPRITESHEET_CONFIG_PATH: &str = "ZombieShooter/Sprites/Character/player_sheet.ron";
const PLAYER_SHIRT_SPRITESHEET_CONFIG_PATH: &str =
    "ZombieShooter/Sprites/Character/shirt_1_sheet.ron";
const PLAYER_HAIR_SPRITESHEET_CONFIG_PATH: &str =
    "ZombieShooter/Sprites/Character/hair_1_sheet.ron";
const PLAYER_ANIMATIONS_CONFIG_PATH: &str = "ZombieShooter/Sprites/Character/player_animation.ron";
const PLAYER_CONFIG_PATH: &str = "ZombieShooter/Sprites/Character/player_config.ron";

#[derive(Resource)]
pub struct GlobalAsset {
    pub spritesheets: HashMap<String, HashMap<String, Handle<SpriteSheetConfig>>>,
    pub animations: HashMap<String, Handle<AnimationMapConfig>>,
    pub character_configs: HashMap<String, Handle<CharacterConfig>>,
    pub weapons: Handle<WeaponsConfig>,
    pub camera: Handle<CameraSettingsAsset>,
}

impl GlobalAsset {
    pub fn create(asset_server: &AssetServer) -> Self {
        Self {
            spritesheets: bmap!(
                "player" => bmap!(
                    "body" => asset_server.load(PLAYER_SPRITESHEET_CONFIG_PATH),
                    "shirt" => asset_server.load(PLAYER_SHIRT_SPRITESHEET_CONFIG_PATH),
                    "hair" => asset_server.load(PLAYER_HAIR_SPRITESHEET_CONFIG_PATH),
                    "shadow" => asset_server.load("ZombieShooter/Sprites/Character/shadow_sheet.ron")
                ),
                "shotgun" => bmap!(
                    "body" => asset_server.load("ZombieShooter/Sprites/Character/shotgun_sheet.ron")
                ),
                "pistol" => bmap!(
                    "body" => asset_server.load("ZombieShooter/Sprites/Character/pistol_sheet.ron")
                ),
                "machine_gun" => bmap!(
                    "body" => asset_server.load("ZombieShooter/Sprites/Character/machine_gun_sheet.ron")
                ),
                "zombie_1" => bmap!(
                    "body" => asset_server.load("ZombieShooter/Sprites/Zombie/zombie_sheet.ron"),
                    "shadow" => asset_server.load("ZombieShooter/Sprites/Character/shadow_sheet.ron")
                ),
                "zombie_2" => bmap!(
                    "body" => asset_server.load("ZombieShooter/Sprites/Zombie/zombie_hard_sheet.ron"),
                    "shadow" => asset_server.load("ZombieShooter/Sprites/Character/shadow_sheet.ron")
                )
            ),
            animations: bmap!(
                "player" => asset_server.load(PLAYER_ANIMATIONS_CONFIG_PATH),
                "machine_gun" => asset_server.load(PLAYER_ANIMATIONS_CONFIG_PATH),
                "pistol" => asset_server.load(PLAYER_ANIMATIONS_CONFIG_PATH),
                "shotgun" => asset_server.load(PLAYER_ANIMATIONS_CONFIG_PATH),
                "zombie_1" => asset_server.load("ZombieShooter/Sprites/Zombie/zombie_animation.ron"),
                "zombie_2" => asset_server.load("ZombieShooter/Sprites/Zombie/zombie_animation.ron")
            ),
            character_configs: bmap!(
                "player" => asset_server.load(PLAYER_CONFIG_PATH),
                "zombie_1" => asset_server.load("ZombieShooter/Sprites/Zombie/zombie_config.ron"),
                "zombie_2" => asset_server.load("ZombieShooter/Sprites/Zombie/zombie_hard_config.ron")
            ),
            weapons: asset_server.load("ZombieShooter/Sprites/Character/weapons.ron"),
            camera: asset_server.load("camera.ron"),
        }
    }
}

pub fn add_global_asset(mut commands: Commands, asset_server: Res<AssetServer>) {
    let global_asset = GlobalAsset::create(&asset_server);

    commands.insert_resource(global_asset);
}

pub fn loading_asset_system(
    mut app_state: ResMut<NextState<AppState>>,
    global_assets: Res<GlobalAsset>,
    asset_server: Res<AssetServer>,
) {
    for (_, v) in global_assets.spritesheets.iter() {
        for (_, handle) in v.iter() {
            if !asset_server.load_state(handle).is_loaded() {
                return;
            }
        }
    }

    for (_, handle) in global_assets.animations.iter() {
        if !asset_server.load_state(handle).is_loaded() {
            return;
        }
    }

    for (_, handle) in global_assets.character_configs.iter() {
        if !asset_server.load_state(handle).is_loaded() {
            return;
        }
    }

    if !asset_server.load_state(&global_assets.weapons).is_loaded() {
        return;
    }
    if !asset_server.load_state(&global_assets.camera).is_loaded() {
        return;
    }

    app_state.set(AppState::Lobby);
    info!("loading of asset is done , now entering lobby");
}
