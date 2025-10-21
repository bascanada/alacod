use bevy::prelude::*;

#[derive(Debug, Clone, Reflect)]
pub struct DoorConfig {
    // cost to open the door
    pub cost: i32,
    // if the door need electricity to be open
    pub electrify: bool,
    // Whether this door is interactable (false for dead-ends)
    pub interactable: bool,
    // If this door is paired with another (at a room connection), store the paired door's level_iid and position
    pub paired_door: Option<(String, (i32, i32))>, // (level_iid, position)
}

impl Default for DoorConfig {
    fn default() -> Self {
        Self {
            cost: 0,
            electrify: false,
            interactable: true, // Doors are interactable by default
            paired_door: None,
        }
    }
}
