use bevy::prelude::*;
use bevy_fixed::fixed_math;
use bevy_ggrs::{GgrsSchedule, Rollback, RollbackApp};
use serde::{Deserialize, Serialize};
use utils::net_id::GgrsNetId;

use crate::{
    collider::Collider,
    system_set::RollbackSystemSet,
};

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
    /// The entity being interacted with
    pub interactable: Entity,
    /// The type of interaction
    pub interaction_type: InteractionType,
    /// The GGRS net ID of the interactable (for deterministic lookup)
    pub interactable_net_id: GgrsNetId,
}

/// System that detects interactions within the GGRS schedule
pub fn interaction_detection_system(
    mut event_writer: EventWriter<InteractionEvent>,
    interactors: Query<
        (Entity, &fixed_math::FixedTransform3D, &crate::character::player::input::InteractionInput),
        (With<Interactor>, With<Rollback>),
    >,
    interactables: Query<
        (
            Entity,
            &fixed_math::FixedTransform3D,
            &Interactable,
            &GgrsNetId,
        ),
        With<Rollback>,
    >,
) {
    for (interactor_entity, interactor_transform, interaction_input) in interactors.iter() {
        // Only process if the interaction button is being held
        if !interaction_input.is_holding {
            continue;
        }

        // Check each interactable
        for (interactable_entity, interactable_transform, interactable, net_id) in
            interactables.iter()
        {
            // Calculate distance between interactor and interactable
            let distance_vec = interactable_transform.translation - interactor_transform.translation;
            let distance_sq: fixed_math::FixedWide = distance_vec.length_squared();
            
            // Convert range to FixedWide for comparison
            let range_fixed = interactable.interaction_range;
            let range_fw = fixed_math::FixedWide::from_num(range_fixed.to_num::<f32>());
            let range_sq_fw = range_fw.saturating_mul(range_fw);

            // If within range, send interaction event
            if distance_sq <= range_sq_fw {
                event_writer.write(InteractionEvent {
                    interactor: interactor_entity,
                    interactable: interactable_entity,
                    interaction_type: interactable.interaction_type,
                    interactable_net_id: net_id.clone(),
                });
            }
        }
    }
}

/// System that handles door interactions
pub fn handle_door_interaction(
    mut event_reader: EventReader<InteractionEvent>,
    mut commands: Commands,
    door_query: Query<(Entity, &map::game::entity::map::door::DoorComponent)>,
) {
    for event in event_reader.read() {
        // Only handle door interactions
        if event.interaction_type != InteractionType::Door {
            continue;
        }

        // Verify the interactable is actually a door
        if let Ok((door_entity, _door_component)) = door_query.get(event.interactable) {
            info!(
                "Door interaction triggered by {:?} on door {:?}",
                event.interactor, door_entity
            );

            // Remove the collider from the door entity (in GGRS schedule)
            // This makes the door passable
            commands.entity(door_entity).remove::<Collider>();
            
            // Note: Visual feedback (sprite changes) would be handled outside GGRS
            // in a separate system that responds to the door's changed state
        }
    }
}

/// Plugin for the interaction system
pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        // Register event
        app.add_event::<InteractionEvent>();

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

        // Add visual feedback systems (outside GGRS schedule)
        app.add_systems(
            Update,
            (
                update_door_visuals,
                display_interaction_prompts,
            ),
        );
    }
}

/// Component to track the state of a door for visual feedback
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorState {
    Closed,
    Open,
}

impl Default for DoorState {
    fn default() -> Self {
        Self::Closed
    }
}

/// System that updates door visual state based on collider presence
/// This runs outside the GGRS schedule for visual feedback only
pub fn update_door_visuals(
    mut door_query: Query<
        (Entity, &mut Sprite, &mut DoorState, Option<&Collider>),
        With<map::game::entity::map::door::DoorComponent>,
    >,
) {
    for (entity, mut sprite, mut door_state, collider) in door_query.iter_mut() {
        // Determine current state based on collider presence
        let new_state = if collider.is_some() {
            DoorState::Closed
        } else {
            DoorState::Open
        };

        // Only update visuals if state changed
        if *door_state != new_state {
            *door_state = new_state;
            
            match new_state {
                DoorState::Open => {
                    // Make the door semi-transparent when open
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.3);
                    info!("Door {:?} opened - visual updated", entity);
                }
                DoorState::Closed => {
                    // Full opacity when closed
                    sprite.color = Color::WHITE;
                }
            }
        }
    }
}

/// System that displays interaction prompts when player is near interactable
pub fn display_interaction_prompts(
    mut gizmos: Gizmos,
    interactors: Query<&fixed_math::FixedTransform3D, With<Interactor>>,
    interactables: Query<
        (Entity, &fixed_math::FixedTransform3D, &Interactable),
        (Without<Interactor>, With<Rollback>),
    >,
) {
    for interactor_transform in interactors.iter() {
        for (_interactable_entity, interactable_transform, interactable) in interactables.iter() {
            // Calculate distance
            let distance_vec = interactable_transform.translation - interactor_transform.translation;
            let distance_sq: fixed_math::FixedWide = distance_vec.length_squared();
            
            // Convert range to FixedWide for comparison
            let range_fixed = interactable.interaction_range;
            let range_fw = fixed_math::FixedWide::from_num(range_fixed.to_num::<f32>());
            let range_sq_fw = range_fw.saturating_mul(range_fw);

            // Draw the interaction range circle so players can see the radius
            let range_pos = Vec3::new(
                fixed_math::to_f32(interactable_transform.translation.x),
                fixed_math::to_f32(interactable_transform.translation.y),
                fixed_math::to_f32(interactable_transform.translation.z),
            );
            
            // Draw range circle in yellow with low opacity
            gizmos.circle(
                Isometry3d::from_translation(range_pos),
                fixed_math::to_f32(interactable.interaction_range),
                Color::srgba(1.0, 1.0, 0.0, 0.3),
            );

            // If within range, draw visual indicator
            if distance_sq <= range_sq_fw {
                let pos = Vec3::new(
                    fixed_math::to_f32(interactable_transform.translation.x),
                    fixed_math::to_f32(interactable_transform.translation.y),
                    fixed_math::to_f32(interactable_transform.translation.z),
                );
                
                // Draw a bright circle above the interactable when in range
                gizmos.circle(
                    Isometry3d::from_translation(pos),
                    10.0,
                    Color::srgb(1.0, 1.0, 0.0),
                );
            }
        }
    }
}
