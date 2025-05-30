use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use map::{
    game::entity::map::room::RoomComponent,
    generation::{entity::room::RoomConfig, LEVEL_PROPERTIES_SPAWN_NAME},
};

pub fn add_room_component_to_ldtk_level(
    mut level_events: EventReader<LevelEvent>,
    levels: Query<(Entity, &LevelIid)>,
    projects: Query<&LdtkProjectHandle>,
    project_assets: Res<Assets<LdtkProject>>,
    mut commands: Commands,
) {
    for level_event in level_events.read() {
        if matches!(level_event, LevelEvent::Spawned(_)) {
            for (entity, level_iid) in levels.iter() {
                let level_data = project_assets
                    .get(projects.single())
                    .expect("project asset should be loaded if levels are spawned")
                    .get_raw_level_by_iid(&level_iid.to_string())
                    .expect("spawned level should exist in the loaded project");

                let is_spawn = level_data
                    .get_bool_field(LEVEL_PROPERTIES_SPAWN_NAME)
                    .expect("level should have non-nullable title string field");

                let room_config = RoomConfig { spawn: *is_spawn };

                commands.entity(entity).insert(RoomComponent {
                    config: room_config,
                });

                if *is_spawn {
                    println!("found a spawn level: {}", level_iid);
                }
            }
        }
    }
}
