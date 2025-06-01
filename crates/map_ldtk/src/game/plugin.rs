use bevy::prelude::*;
use game::core::AppState;

use crate::{game::utility::load_levels_if_not_present, loader::{get_asset_loader_generation, setup_generated_map}};



pub struct LdtkMapLoadingPlugin;


impl Plugin for LdtkMapLoadingPlugin {
    fn build(&self, app: &mut App) {
        let level_loader = get_asset_loader_generation();

        app.register_asset_loader(level_loader);

        app.add_systems(OnEnter(AppState::GameLoading), (setup_generated_map));
        app.add_systems(Update, (load_levels_if_not_present));

    }
    
}