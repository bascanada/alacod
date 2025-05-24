
use animation::{AnimationBundle, SpriteSheetConfig};
use bevy::{math::VectorSpace, prelude::*, utils:: HashMap};
use leafwing_input_manager::{prelude::ActionState, InputManagerBundle};
use utils::{bmap, fixed_math};
use bevy_kira_audio::prelude::*;

use crate::{character::{config::CharacterConfig, create::create_character, dash::DashState, movement::{SprintState, Velocity}}, collider::{Collider, ColliderShape, CollisionLayer, CollisionSettings}, global_asset::GlobalAsset, weapons::{spawn_weapon_for_player, FiringMode, Weapon, WeaponInventory, WeaponsConfig}};

use bevy_ggrs::AddRollbackCommandExtension;
use super::{control::{get_input_map, PlayerAction}, input::CursorPosition, LocalPlayer, Player};

const PLAYER_COLORS: &'static [LinearRgba] = &[
    LinearRgba::RED,
    LinearRgba::BLUE,
    LinearRgba::GREEN,
    LinearRgba::BLACK,
];

pub fn create_player(
    commands: &mut Commands,
    global_assets: &Res<GlobalAsset>,
    weapons_asset: &Res<Assets<WeaponsConfig>>,
    character_asset: &Res<Assets<CharacterConfig>>,
    collision_settings: &Res<CollisionSettings>,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: &Res<Assets<SpriteSheetConfig>>,


    local: bool,
    handle: usize,
) {


    let entity = create_character(
        commands, global_assets, character_asset, asset_server, texture_atlas_layouts, sprint_sheet_assets,
        "player".into(), Some(if handle == 0 { "1" } else { "2" }.into()),
         (LinearRgba::GREEN).into(), fixed_math::FixedVec3::new(fixed_math::new(-50.0 * handle as f32), fixed_math::new(0.0), fixed_math::new(0.0)),
        CollisionLayer(collision_settings.player_layer),
    );
    if local {
        commands.entity(entity)
            .insert((
                LocalPlayer{},
                InputManagerBundle::<PlayerAction> {
                    action_state: ActionState::default(),
                    input_map: get_input_map(),
                }
            ));
    }
    
    let mut inventory = WeaponInventory::default();

    if let Some(weapons_config) = weapons_asset.get(&global_assets.weapons) {
        let mut keys: Vec<&String> = weapons_config.0.keys().collect();
        keys.sort();
        for (i, k) in keys.iter().enumerate() {
            spawn_weapon_for_player(commands, global_assets, asset_server, texture_atlas_layouts, sprint_sheet_assets, i == 0, entity, weapons_config.0.get(*k).unwrap().clone(), &mut inventory);
        }
    }
    
    commands.entity(entity)
        .insert((
            inventory,
            CursorPosition::default(),
            Player {
                handle,
                color: PLAYER_COLORS[handle].into(),
            }
        ));

}