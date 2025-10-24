use animation::SpriteSheetConfig;
use bevy::prelude::*;
use bevy_light_2d::light::{AmbientLight2d, PointLight2d};
use leafwing_input_manager::{prelude::ActionState, InputManagerBundle};
use utils::net_id::GgrsNetIdFactory;

use crate::{
    character::{config::CharacterConfig, create::create_character},
    collider::{CollisionLayer, CollisionSettings},
    global_asset::GlobalAsset,
    weapons::{melee::{spawn_melee_weapon_for_character, MeleeWeaponsConfig}, spawn_weapon_for_player, WeaponInventory, WeaponsConfig},
};

use super::{
    control::{get_input_map, PlayerAction},
    input::CursorPosition,
    LocalPlayer, Player,
};
use bevy_fixed::fixed_math::{self, FixedVec3};

const PLAYER_COLORS: &[LinearRgba] = &[
    LinearRgba::RED,
    LinearRgba::BLUE,
    LinearRgba::GREEN,
    LinearRgba::BLACK,
];

pub fn create_player(
    commands: &mut Commands,
    global_assets: &Res<GlobalAsset>,
    weapons_asset: &Res<Assets<WeaponsConfig>>,
    melee_weapons_asset: &Res<Assets<MeleeWeaponsConfig>>,
    character_asset: &Res<Assets<CharacterConfig>>,
    collision_settings: &Res<CollisionSettings>,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: &Res<Assets<SpriteSheetConfig>>,

    position: FixedVec3,

    local: bool,
    handle: usize,

    id_factory: &mut ResMut<GgrsNetIdFactory>,
) {
    let entity = create_character(
        commands,
        global_assets,
        character_asset,
        asset_server,
        texture_atlas_layouts,
        sprint_sheet_assets,
        "player".into(),
        Some(if handle == 0 { "1" } else { "2" }.into()),
        (LinearRgba::GREEN).into(),
       position, 
        CollisionLayer(collision_settings.player_layer),
        id_factory,
    );
    if local {
        commands.entity(entity).insert((
            LocalPlayer {},
            InputManagerBundle::<PlayerAction> {
                action_state: ActionState::default(),
                input_map: get_input_map(),
            },
        ));
    }

    let mut inventory = WeaponInventory::default();

    if let Some(weapons_config) = weapons_asset.get(&global_assets.weapons) {
        let mut keys: Vec<&String> = weapons_config.0.keys().collect();
        keys.sort();
        for (i, k) in keys.iter().enumerate() {
            spawn_weapon_for_player(
                commands,
                global_assets,
                asset_server,
                texture_atlas_layouts,
                sprint_sheet_assets,
                i == 0,
                entity,
                weapons_config.0.get(*k).unwrap().clone(),
                &mut inventory,
                id_factory,
            );
        }
    }

    // Add a default melee weapon (bare hands) to all players
    if let Some(melee_weapons_config) = melee_weapons_asset.get(&global_assets.melee_weapons) {
        if let Some(bare_hands) = melee_weapons_config.0.get("bare_hands") {
            spawn_melee_weapon_for_character(
                commands,
                global_assets,
                asset_server,
                texture_atlas_layouts,
                sprint_sheet_assets,
                false,
                entity,
                bare_hands.clone(),
                id_factory,
            );
        }
    }

    commands.entity(entity).insert((
        inventory,
        CursorPosition::default(),
        super::input::InteractionInput::default(),
        crate::interaction::Interactor,
        Player {
            handle,
            color: PLAYER_COLORS[handle].into(),
        },
        PointLight2d {
            radius: 200.,
            cast_shadows: false,
            falloff: 4.,
            ..default()
        },
    ));
}
