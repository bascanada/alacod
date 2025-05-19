use animation::SpriteSheetConfig;
use bevy::prelude::*;

use crate::{character::{config::{CharacterConfig, CharacterConfigHandles}, create::create_character, movement::Velocity, player::input::CursorPosition}, collider::{Collider, ColliderShape, CollisionLayer, CollisionSettings}, global_asset::GlobalAsset, weapons::{WeaponInventory, WeaponsConfig}};

use super::Enemy;

pub fn spawn_enemy(
    enemy_type_name: String,
    position: Vec3,
    commands: &mut Commands,
    weapons_asset: &Res<Assets<WeaponsConfig>>,
    characters_asset: &Res<Assets<CharacterConfig>>,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: &Res<Assets<SpriteSheetConfig>>,

    global_assets: &Res<GlobalAsset>,
    collision_settings: &Res<CollisionSettings>,
) {

    let entity = create_character(
        commands, global_assets, characters_asset, asset_server, texture_atlas_layouts, sprint_sheet_assets,
        enemy_type_name, None,
        position, CollisionLayer(collision_settings.enemy_layer)
    );

    let inventory = WeaponInventory::default();

    commands.entity(entity)
        .insert((
            inventory,
            Enemy::default(),
        ));

}