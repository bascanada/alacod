
use animation::{create_child_sprite, AnimationBundle, SpriteSheetConfig};
use bevy::{prelude::*};
use bevy_kira_audio::prelude::*;
use utils::fixed_math;

use crate::{character::{config::CharacterConfigHandles, movement::Velocity}, collider::{Collider, ColliderShape, CollisionLayer, CollisionSettings}, global_asset::GlobalAsset, weapons::{spawn_weapon_for_player, FiringMode, Weapon, WeaponInventory, WeaponsConfig}};

use bevy_ggrs::AddRollbackCommandExtension;

use super::{config::CharacterConfig, dash::DashState, health::{ui::HealthBar, DamageAccumulator, Health}, movement::SprintState, Character};

pub fn create_character(
    commands: &mut Commands,
    global_assets: &Res<GlobalAsset>,
    character_asset: &Res<Assets<CharacterConfig>>,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: &Res<Assets<SpriteSheetConfig>>,

    config_name: String,

    skin: Option<String>,
    color_health_bar: Color,
    translation: fixed_math::FixedVec3,

    collision_layer: CollisionLayer,
) -> Entity {
    let handle = global_assets.character_configs.get(&config_name).unwrap();
    let config = character_asset.get(handle).unwrap();

    let map_layers = global_assets.spritesheets.get(&config.asset_name_ref).unwrap().clone();
    let animation_handle = global_assets.animations.get(&config.asset_name_ref).unwrap().clone();
    let player_config_handle = global_assets.character_configs.get(&config.asset_name_ref).unwrap().clone();

    let starting_layer = config.skins.get(skin.as_ref().unwrap_or(&config.starting_skin)).unwrap()
        .layers.clone();

    let animation_bundle =
        AnimationBundle::new(map_layers.clone(), animation_handle.clone(),0, starting_layer.clone());


    let transform_fixed = fixed_math::FixedTransform3D::new(
        translation,
        fixed_math::FixedMat3::IDENTITY,
        fixed_math::FixedVec3::splat(config.scale)
    );

    let collider: Collider = (&config.collider).into();
    let health: Health = config.base_health.clone().into();
    let mut entity = commands.spawn((
        transform_fixed.to_bevy_transform(),
        transform_fixed,
        Visibility::default(),
        SpatialAudioEmitter {instances: vec![]},
        Velocity(fixed_math::FixedVec2::ZERO),
        SprintState::default(),
        DashState::default(),
        collider,
        health,
        collision_layer,
        Character::default(),
        CharacterConfigHandles {
            config: player_config_handle.clone(),
        },
        animation_bundle,
    ));

    let entity = entity.with_children(|parent| {
        parent.spawn((
            HealthBar,
            Sprite {
                color: color_health_bar,
                custom_size: Some(Vec2::new(30.0, 3.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 10.0, 0.1)),
        )).add_rollback();
    });

    let entity = entity.add_rollback().id();


    for k in starting_layer.keys() {
        let spritesheet_config = sprint_sheet_assets.get(map_layers.get(k).unwrap()).unwrap();
        create_child_sprite(
            commands,
            &asset_server,
            texture_atlas_layouts,
            entity.clone(), &spritesheet_config, 0);
    }

    entity


}