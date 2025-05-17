
use animation::AnimationBundle;
use bevy::{math::VectorSpace, prelude::*, utils:: HashMap};
use leafwing_input_manager::{prelude::ActionState, InputManagerBundle};
use utils::bmap;
use bevy_kira_audio::prelude::*;

use crate::{character::{config::{CharacterConfig, CharacterConfigHandles, CharacterSkin}, create::create_character, movement::Velocity}, collider::{Collider, ColliderShape, CollisionLayer, CollisionSettings}, global_asset::GlobalAsset, weapons::{spawn_weapon_for_player, FiringMode, Weapon, WeaponInventory, WeaponsConfig}};

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

    local: bool,
    handle: usize,
) {


    let entity = create_character(
        commands, global_assets, character_asset, 
        "player".into(), Some(if local { "1" } else { "2" }.into()),
        Vec3::new(-50.0 * handle as f32, 0.0, 0.0),
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
            spawn_weapon_for_player(commands, global_assets, i == 0, entity, weapons_config.0.get(*k).unwrap().clone(), &mut inventory);
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