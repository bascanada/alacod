use bevy::prelude::*;
use game::core::AppState;
use map::game::entity::map::map_rollback::MapRollbackMarker;

use crate::{game::utility::load_levels_if_not_present, loader::{get_asset_loader_generation, setup_generated_map}};

pub struct LdtkMapLoadingPlugin;


#[derive(Clone)]
pub struct LdtkMapEntityLoading {
    pub id: String,
    pub entity: Entity,
}

#[derive(Resource, Default, Clone)]
pub struct LdtkMapEntityLoadingRegistry {
    pub entities: Vec<LdtkMapEntityLoading>,
    pub last_update: u64,
}

#[derive(Event, Default, Debug)]
pub struct LdtkMapLoadingEvent;

impl Plugin for LdtkMapLoadingPlugin {
    fn build(&self, app: &mut App) {
        let level_loader = get_asset_loader_generation();

        app.register_asset_loader(level_loader);

        app.init_resource::<LdtkMapEntityLoadingRegistry>();
        app.add_event::<LdtkMapLoadingEvent>();

        app.add_systems(OnEnter(AppState::GameLoading), (setup_generated_map));
        app.add_systems(Update, (
            load_levels_if_not_present,
            wait_for_all_map_rollback_entity,
        ).run_if(in_state(AppState::GameLoading)));
    }
}

fn wait_for_all_map_rollback_entity(
    mut entity_registery: ResMut<LdtkMapEntityLoadingRegistry>,
    mut ev_loading_map: EventWriter<LdtkMapLoadingEvent>,

    query_map_entity: Query<Entity, Added<MapRollbackMarker>>,

    time: Res<Time>,
) {

    let previous_size = entity_registery.entities.len();


    for (e) in query_map_entity.iter() {
        entity_registery.entities.push(LdtkMapEntityLoading { id: "".into(), entity: e.clone() });
    }


    if previous_size < entity_registery.entities.len() {
        ev_loading_map.write_default();
    }

}