use bevy::{prelude::*, reflect::TypePath, utils::HashMap};
use serde::Deserialize;
use utils::fixed_math;

use crate::{character::movement::MovementConfig, collider::{Collider, ColliderConfig}};

use super::health::HealthConfig;


#[derive(Debug, Deserialize, Clone)]
pub struct CharacterSkin {
    pub layers: HashMap<String, String>
}

#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
pub struct CharacterConfig {
    pub movement: MovementConfig,

    pub asset_name_ref: String,

    pub base_health: HealthConfig,

    pub collider: ColliderConfig,

    pub scale: fixed_math::Fixed,

    pub starting_skin: String,
    pub skins: HashMap<String, CharacterSkin>
}

#[derive(Component)]
pub struct CharacterConfigHandles {
    pub config: Handle<CharacterConfig>
}
