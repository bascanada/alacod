use animation::SpriteSheetConfig;
use bevy::prelude::*;
use bevy_fixed::fixed_math;
use utils::net_id::GgrsNetIdFactory;

use crate::{
    character::{
        config::{CharacterConfig, CharacterConfigHandles},
        create::create_character,
        movement::Velocity,
        player::input::CursorPosition,
    },
    collider::{Collider, ColliderShape, CollisionLayer, CollisionSettings},
    global_asset::GlobalAsset,
    weapons::{WeaponInventory, WeaponsConfig},
};

use super::{ai::pathing::EnemyPath, Enemy};

pub fn spawn_enemy(
    enemy_type_name: String,
    position: fixed_math::FixedVec3,
    commands: &mut Commands,
    weapons_asset: &Res<Assets<WeaponsConfig>>,
    characters_asset: &Res<Assets<CharacterConfig>>,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: &Res<Assets<SpriteSheetConfig>>,

    global_assets: &Res<GlobalAsset>,
    collision_settings: &Res<CollisionSettings>,

    id_factory: &mut ResMut<GgrsNetIdFactory>,
) {
    let entity = create_character(
        commands,
        global_assets,
        characters_asset,
        asset_server,
        texture_atlas_layouts,
        sprint_sheet_assets,
        enemy_type_name,
        None,
        (LinearRgba::RED).into(),
        position,
        CollisionLayer(collision_settings.enemy_layer),
        id_factory,
    );

    let inventory = WeaponInventory::default();

    commands
        .entity(entity)
        .insert((inventory, EnemyPath::default(), Enemy::default()));
}
