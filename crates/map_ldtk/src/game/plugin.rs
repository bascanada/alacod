use bevy::prelude::*;
use bevy_fixed::fixed_math;
use game::{collider::{Collider, CollisionLayer, CollisionSettings, Wall, Window}, core::AppState};
use map::game::entity::{map::map_rollback::MapRollbackMarker, MapRollbackItem};
use bevy_ggrs::AddRollbackCommandExtension;
use utils::net_id::GgrsNetIdFactory;

use crate::{game::{collider::create_wall_colliders_from_ldtk, entity::door::LdtkEntitySize, utility::load_levels_if_not_present}, loader::{get_asset_loader_generation, setup_generated_map}};

pub struct LdtkMapLoadingPlugin;


#[derive(Clone)]
pub struct LdtkMapEntityLoading {
    pub id: String,
    pub kind: String,
    pub global_transform: GlobalTransform,
    pub entity: Entity,
    pub sprite_size: Option<Vec2>,
}

#[derive(Resource, Clone)]
pub struct LdtkMapEntityLoadingRegistry {
    pub entities: Vec<LdtkMapEntityLoading>,
    pub registered_entities: std::collections::HashSet<Entity>,
    pub last_update_time: f32,
    pub timeout_duration: f32,
    pub loading_complete: bool,
}

impl Default for LdtkMapEntityLoadingRegistry {
    fn default() -> Self {
        Self { 
            entities: vec![], 
            registered_entities: std::collections::HashSet::new(),
            last_update_time: 0.0, 
            timeout_duration: 0.5, 
            loading_complete: false 
        }
    }
}

#[derive(Event, Default, Debug)]
pub struct LdtkMapLoadingEvent;

impl Plugin for LdtkMapLoadingPlugin {
    fn build(&self, app: &mut App) {
        let level_loader = get_asset_loader_generation();

        app.register_asset_loader(level_loader);

        app.init_resource::<LdtkMapEntityLoadingRegistry>();
        app.add_event::<LdtkMapLoadingEvent>();

        app.add_systems(OnEnter(AppState::GameLoading), setup_generated_map);
        app.add_systems(Update, (
            load_levels_if_not_present,
            wait_for_all_map_rollback_entity,
        ).run_if(in_state(AppState::GameLoading)));

        app.add_systems(Update, create_wall_colliders_from_ldtk.run_if(on_event::<LdtkMapLoadingEvent>));
    }
}

fn wait_for_all_map_rollback_entity(
    mut commands: Commands,
    mut entity_registery: ResMut<LdtkMapEntityLoadingRegistry>,
    mut ev_loading_map: EventWriter<LdtkMapLoadingEvent>,

    query_map_entity: Query<(Entity, &GlobalTransform, &MapRollbackMarker, Option<&LdtkEntitySize>), With<MapRollbackMarker>>,

    collision_settings: Res<CollisionSettings>,

    mut id_factory: ResMut<GgrsNetIdFactory>,

    time: Res<Time>,
) {

    if entity_registery.loading_complete {
        return;
    }

    let current_time = time.elapsed_secs();
    let previous_size = entity_registery.entities.len();

    for (e, global_transform, rollback_marker, ldtk_size) in query_map_entity.iter() {
        // Skip if already registered
        if entity_registery.registered_entities.contains(&e) {
            continue;
        }
        
        let translation = global_transform.translation();
        
        // Only register entities with valid (non-zero) global transforms
        // GlobalTransform gets updated by Bevy's transform propagation system
        if translation.x != 0.0 || translation.y != 0.0 {
            let sprite_size = ldtk_size.map(|s| Vec2::new(s.width, s.height));
            info!("Found {} entity {:?} at position {} with LDTK size {:?}", 
                  rollback_marker.0, e, translation, sprite_size);
            
            entity_registery.entities.push(LdtkMapEntityLoading { 
                id: rollback_marker.0.clone(), 
                kind: rollback_marker.0.clone(), 
                entity: e.clone(), 
                global_transform: *global_transform,
                sprite_size,
            });
            entity_registery.registered_entities.insert(e);
        }
    }

    let new_size = entity_registery.entities.len();

    // If new entities were added, update the last update time
    if new_size > previous_size {
        entity_registery.last_update_time = current_time;
        info!("Added {} new map entities, total: {}", new_size - previous_size, new_size);
        return;
    }

    // If we have entities and enough time has passed since the last update, consider loading complete
    if !entity_registery.entities.is_empty() && 
       (current_time - entity_registery.last_update_time) >= entity_registery.timeout_duration {

        for item in entity_registery.entities.iter() {
            let rollback_item = 
                MapRollbackItem::new(item.entity.clone(), item.kind.clone());
            let id = 
                id_factory.next(item.id.clone());

            // Convert GlobalTransform to Transform for positioning
            let world_position = item.global_transform.translation();
            let transform = Transform::from_translation(world_position);

            info!("spawning rollback map item {} at {} for parent {}", id, world_position, item.entity);
            let mut cmd = commands.spawn((
                fixed_math::FixedTransform3D::from_bevy_transform(&transform),
                rollback_item,
                id,
            ));


            match item.kind.as_str() {
                "door" => {
                    // Use sprite size if available, otherwise fall back to default size
                    let (width, height) = if let Some(size) = item.sprite_size {
                        (size.x, size.y)
                    } else {
                        info!("No sprite size for door, using default 64x32");
                        (64.0, 32.0)
                    };
                    
                    cmd.insert((
                        Wall,
                        Collider {
                            shape: game::collider::ColliderShape::Rectangle {
                                width: fixed_math::Fixed::from_num(width), 
                                height: fixed_math::Fixed::from_num(height),
                            },
                            offset: fixed_math::FixedVec3::ZERO,
                        },
                        CollisionLayer(collision_settings.wall_layer),
                    ));
                    info!("adding collider to door entity with size {}x{}", width, height);
                },
                "window" => {
                    // Use sprite size if available, otherwise fall back to default size
                    let (width, height) = if let Some(size) = item.sprite_size {
                        (size.x, size.y)
                    } else {
                        info!("No sprite size for window, using default 16x16");
                        (16.0, 16.0)
                    };
                    
                    cmd.insert((
                        Window,
                        Collider {
                            shape: game::collider::ColliderShape::Rectangle {
                                width: fixed_math::Fixed::from_num(width), 
                                height: fixed_math::Fixed::from_num(height),
                            },
                            offset: fixed_math::FixedVec3::ZERO,
                        },
                        CollisionLayer(collision_settings.window_layer),
                    ));
                    info!("adding collider to window entity with size {}x{}", width, height);
                },
                _ => {}
            }

            let _ = cmd.add_rollback().id();
        }

        ev_loading_map.write_default();
        entity_registery.loading_complete = true;

    }
}


