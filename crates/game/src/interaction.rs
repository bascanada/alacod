use bevy::{log::{tracing::span, Level}, prelude::*};
use bevy_fixed::fixed_math;
use bevy_ggrs::{GgrsSchedule, Rollback, RollbackApp};
use serde::{Deserialize, Serialize};
use utils::{frame::FrameCount, net_id::GgrsNetId, order_iter};

use crate::{
    collider::{Collider, CollisionLayer},
    core::AppState,
    system_set::RollbackSystemSet,
};

/// Component marker for the interaction prompt text UI
#[derive(Component)]
pub struct InteractionPromptText;

/// Component that marks an entity as interactable
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct Interactable {
    /// Range within which interaction is possible
    pub interaction_range: fixed_math::Fixed,
    /// Type of interaction
    pub interaction_type: InteractionType,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            interaction_range: fixed_math::new(50.0),
            interaction_type: InteractionType::Door,
        }
    }
}

/// Types of interactions available
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Reflect, PartialEq, Eq)]
pub enum InteractionType {
    Door,
    // Future: Crate, Weapon, Soda, etc.
}

/// Component that marks an entity as capable of interacting
#[derive(Component, Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Interactor;

/// Event sent when an interaction is triggered
#[derive(Event, Clone, Debug)]
pub struct InteractionEvent {
    /// The entity performing the interaction
    pub interactor: Entity,
    /// The GGRS net ID of the interactor (for deterministic logging)
    pub interactor_net_id: GgrsNetId,
    /// The entity being interacted with
    pub interactable: Entity,
    /// The type of interaction
    pub interaction_type: InteractionType,
    /// The GGRS net ID of the interactable (for deterministic lookup)
    pub interactable_net_id: GgrsNetId,
}

/// Event sent when a door is opened (for visual feedback)
/// This event is sent from GGRS schedule to Update schedule
#[derive(Event, Clone, Debug)]
pub struct DoorOpenedEvent {
    /// The GGRS net ID of the door that was opened
    pub door_net_id: GgrsNetId,
    /// The visual entity (LDTK entity) that needs to be hidden
    pub visual_entity: Entity,
}

/// System that detects interactions within the GGRS schedule
pub fn interaction_detection_system(
    frame: Res<FrameCount>,
    mut event_writer: EventWriter<InteractionEvent>,
    interactors: Query<
        (&GgrsNetId, Entity, &fixed_math::FixedTransform3D, &crate::character::player::input::InteractionInput),
        (With<Interactor>, With<Rollback>),
    >,
    interactables: Query<
        (
            &GgrsNetId,
            Entity,
            &fixed_math::FixedTransform3D,
            &Interactable,
            Option<&crate::collider::Collider>,
        ),
        With<Rollback>,
    >,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "interaction_detection");
    let _enter = system_span.enter();

    for (interactor_net_id, interactor_entity, interactor_transform, interaction_input) in order_iter!(interactors) {
        // Only process if the interaction button is being held
        if !interaction_input.is_holding {
            continue;
        }

        let interactor_pos = interactor_transform.translation;

        // Track the closest interactable within range
        let mut closest_interactable: Option<(fixed_math::FixedWide, GgrsNetId, Entity, InteractionType)> = None;

        // Check each interactable to find the closest one
        for (net_id, interactable_entity, interactable_transform, interactable, collider_opt) in
            order_iter!(interactables)
        {
            // Compute squared distance from the interactor to the interactable.
            // If the interactable has a collider, measure distance to the collider surface;
            // otherwise fall back to entity-center distance.
            let distance_sq: fixed_math::FixedWide = if let Some(collider) = collider_opt {
                point_to_collider_surface_distance_sq(
                    interactor_pos,
                    interactable_transform.translation,
                    collider,
                )
            } else {
                (interactable_transform.translation - interactor_pos).length_squared()
            };

            // Convert range to FixedWide for comparison
            // Direct conversion from Fixed to FixedWide to maintain precision
            let range_fw = fixed_math::FixedWide::from_num(interactable.interaction_range.to_num::<i64>());
            let range_sq_fw = range_fw.saturating_mul(range_fw);

            // If within range, check if this is the closest one
            if distance_sq <= range_sq_fw {
                match &closest_interactable {
                    None => {
                        // First interactable found
                        closest_interactable = Some((distance_sq, net_id.clone(), interactable_entity, interactable.interaction_type));
                    }
                    Some((closest_dist_sq, _, _, _)) => {
                        // Compare distances; if this one is closer, use it
                        if distance_sq < *closest_dist_sq {
                            closest_interactable = Some((distance_sq, net_id.clone(), interactable_entity, interactable.interaction_type));
                        }
                    }
                }
            }
        }

        // Only send interaction event for the closest interactable
        if let Some((distance_sq, net_id, interactable_entity, interaction_type)) = closest_interactable {
            info!("{} interaction detected: interactor {} with {} at distance_sq {:?}", 
                  frame.as_ref(), interactor_net_id, net_id, 
                  fixed_math::to_f32(fixed_math::Fixed::from_num(distance_sq.to_num::<f32>())));
            event_writer.write(InteractionEvent {
                interactor: interactor_entity,
                interactor_net_id: interactor_net_id.clone(),
                interactable: interactable_entity,
                interaction_type,
                interactable_net_id: net_id,
            });
        }
    }
}

// Helper: compute squared distance (FixedWide) from a point to the surface of a collider
fn point_to_collider_surface_distance_sq(
    point: fixed_math::FixedVec3,
    collider_pos: fixed_math::FixedVec3,
    collider: &crate::collider::Collider,
) -> fixed_math::FixedWide {
    use crate::collider::ColliderShape;

    // Apply collider offset to get the collider's actual world-center
    let collider_center = collider_pos + collider.offset;

    match &collider.shape {
        ColliderShape::Rectangle { width, height } => {
            let two = fixed_math::new(2.0);
            let half_w = width.saturating_div(two);
            let half_h = height.saturating_div(two);
            let closest_x = point.x.max(collider_center.x - half_w).min(collider_center.x + half_w);
            let closest_y = point.y.max(collider_center.y - half_h).min(collider_center.y + half_h);

            let diff = fixed_math::FixedVec2::new(point.x - closest_x, point.y - closest_y);
            diff.length_squared()
        }
        ColliderShape::Circle { radius } => {
            // Distance from point to circle center
            let diff = fixed_math::FixedVec2::new(point.x - collider_center.x, point.y - collider_center.y);
            let dist_sq_fw: fixed_math::FixedWide = diff.length_squared();

            // Convert radius to FixedWide
            let radius_fw = fixed_math::FixedWide::from_num(radius.to_num::<f32>());
            let radius_sq_fw = radius_fw.saturating_mul(radius_fw);

            if dist_sq_fw <= radius_sq_fw {
                // Inside the circle: distance to surface is zero
                fixed_math::FixedWide::from_num(0.0)
            } else {
                // distance_to_surface = sqrt(dist_sq) - radius
                let dist_fw = dist_sq_fw.sqrt();
                let d_surface = dist_fw.saturating_sub(radius_fw);
                d_surface.saturating_mul(d_surface)
            }
        }
    }

}

/// System that handles door interactions
pub fn handle_door_interaction(
    frame: Res<FrameCount>,
    mut event_reader: EventReader<InteractionEvent>,
    mut door_opened_writer: EventWriter<DoorOpenedEvent>,
    mut commands: Commands,
    door_query: Query<(Entity, &map::game::entity::MapRollbackItem), (With<Interactable>, With<Rollback>)>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "handle_door_interaction");
    let _enter = system_span.enter();

    for event in event_reader.read() {
        // Only handle door interactions
        if event.interaction_type != InteractionType::Door {
            continue;
        }

        // Verify the interactable entity exists and is a rollback entity
        if let Ok((door_entity, rollback_item)) = door_query.get(event.interactable) {
            info!(
                "{} door interaction triggered: interactor {} on door {}",
                frame.as_ref(), event.interactor_net_id, event.interactable_net_id
            );

            // Remove the collider from the door entity (in GGRS schedule)
            // This makes the door passable
            commands.entity(door_entity)
                .remove::<Collider>()
                .remove::<CollisionLayer>()
                .remove::<Interactable>();

            info!(
                "{} door {} components removed (Collider, CollisionLayer, Interactable)",
                frame.as_ref(), event.interactable_net_id
            );

            // Send event for visual feedback system with the parent (visual) entity
            door_opened_writer.write(DoorOpenedEvent {
                door_net_id: event.interactable_net_id.clone(),
                visual_entity: rollback_item.parent,
            });
            
            // Note: Visual feedback (sprite changes) would be handled outside GGRS
            // in a separate system that responds to the door's changed state
        } else {
            warn!(
                "InteractionEvent received for entity {:?} that is not a rollback interactable",
                event.interactable
            );
        }
    }
}

/// Plugin for the interaction system
pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        // Register events
        app.add_event::<InteractionEvent>();
        app.add_event::<DoorOpenedEvent>();

        // Register rollback components
        app.rollback_component_with_clone::<Interactable>()
            .rollback_component_with_clone::<Interactor>()
            .rollback_component_with_clone::<crate::character::player::input::InteractionInput>();

        // Add interaction detection to GGRS schedule
        // This runs after input processing but before movement
        app.add_systems(
            GgrsSchedule,
            (
                interaction_detection_system,
                handle_door_interaction,
            )
                .chain()
                .after(RollbackSystemSet::Input)
                .before(RollbackSystemSet::Movement)
                .in_set(RollbackSystemSet::Interaction),
        );

        // Setup UI on entering InGame state
        app.add_systems(OnEnter(AppState::InGame), setup_interaction_ui);

        // Add visual feedback systems (outside GGRS schedule)
        app.add_systems(
            Update,
            (
                update_door_visuals,
                display_interaction_prompts,
            ).run_if(in_state(AppState::InGame)),
        );
    }
}

/// System that updates door visual state based on door opened events
/// This runs outside the GGRS schedule for visual feedback only
/// Uses the visual entity (parent) from MapRollbackItem to directly update the LDTK entity
pub fn update_door_visuals(
    mut door_opened_events: EventReader<DoorOpenedEvent>,
    mut door_query: Query<&mut Visibility>,
) {
    for event in door_opened_events.read() {
        // Directly access the visual entity using the parent from MapRollbackItem
        if let Ok(mut visibility) = door_query.get_mut(event.visual_entity) {
            *visibility = Visibility::Hidden;
            info!("Door {:?} (visual entity {:?}) visibility updated to Hidden", 
                  event.door_net_id, event.visual_entity);
        } else {
            warn!("Could not find visual entity {:?} for door {:?}", 
                  event.visual_entity, event.door_net_id);
        }
    }
}

/// Setup the interaction prompt UI text
fn setup_interaction_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands.spawn((
        InteractionPromptText,
        Text::new(""),
        TextFont {
            font,
            font_size: 12.0,
            ..Default::default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)),
        TextLayout::new_with_justify(JustifyText::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(40.0), // Position in the upper-middle of the screen
            left: Val::Percent(50.0),
            ..default()
        },
    ));
}

/// System that displays interaction prompts when player is near interactable
pub fn display_interaction_prompts(
    mut gizmos: Gizmos,
    mut text_query: Query<&mut Text, With<InteractionPromptText>>,
    interactors: Query<&fixed_math::FixedTransform3D, (With<Interactor>, With<Rollback>)>,
    interactables: Query<
        (Entity, &fixed_math::FixedTransform3D, &Interactable, Option<&map::game::entity::map::door::DoorComponent>),
        (Without<Interactor>, With<Rollback>),
    >,
) {
    // Track the closest door across all players
    // Store: (distance, cost, position, range)
    let mut closest_door_info: Option<(f32, i32, Vec3, f32)> = None;
    
    for interactor_transform in interactors.iter() {
        for (_interactable_entity, interactable_transform, interactable, door_component_opt) in interactables.iter() {
            // Calculate distance
            let distance_vec = interactable_transform.translation - interactor_transform.translation;
            let distance_sq: fixed_math::FixedWide = distance_vec.length_squared();
            
            // Convert range to FixedWide for comparison
            // Direct conversion from Fixed to FixedWide to maintain precision
            let range_fw = fixed_math::FixedWide::from_num(interactable.interaction_range.to_num::<i64>());
            let range_sq_fw = range_fw.saturating_mul(range_fw);

            // If within range and it's a door, track the closest one
            if distance_sq <= range_sq_fw {
                if let Some(door_component) = door_component_opt {
                    let distance = fixed_math::to_f32(fixed_math::Fixed::from_num(distance_sq.to_num::<f32>().sqrt()));
                    let door_pos = Vec3::new(
                        fixed_math::to_f32(interactable_transform.translation.x),
                        fixed_math::to_f32(interactable_transform.translation.y),
                        fixed_math::to_f32(interactable_transform.translation.z),
                    );
                    let interaction_range = fixed_math::to_f32(interactable.interaction_range);
                    
                    match &closest_door_info {
                        None => {
                            closest_door_info = Some((distance, door_component.config.cost, door_pos, interaction_range));
                        }
                        Some((closest_dist, _, _, _)) => {
                            if distance < *closest_dist {
                                closest_door_info = Some((distance, door_component.config.cost, door_pos, interaction_range));
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw the range circle only for the closest door (if any player is in range)
    if let Some((_distance, cost, door_pos, interaction_range)) = closest_door_info {
        // Draw outer range circle in yellow with low opacity
        gizmos.circle(
            Isometry3d::from_translation(door_pos),
            interaction_range,
            Color::srgba(1.0, 1.0, 0.0, 0.3),
        );
        
        // Update the text UI
        if let Ok(mut text) = text_query.single_mut() {
            text.0 = format!("Press H to open door (Cost: {})", cost);
        }
    } else {
        // Clear the text if no door is in range
        if let Ok(mut text) = text_query.single_mut() {
            text.0.clear();
        }
    }
}
