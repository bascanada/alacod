//! Generic Obstacle System
//!
//! Replaces hardcoded Window/Wall logic with a flexible obstacle system
//! that supports different types of blocking entities with various properties.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Type of obstacle - determines default behavior and appearance
/// GGRS: PartialOrd + Ord required for BTreeMap in FlowFieldCache
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect, Serialize, Deserialize, Default)]
pub enum ObstacleType {
    /// Solid wall - never breakable, always blocks
    #[default]
    Wall,
    /// Window - breakable, allows attack through, blocks movement
    Window,
    /// Barricade - breakable, blocks attack, blocks movement
    Barricade,
    /// Water - blocks ground units, not flying
    Water,
    /// Pit/hole - blocks ground units, not flying
    Pit,
    /// Low cover - can shoot over, blocks movement for ground
    LowCover,
}

impl ObstacleType {
    /// Returns true if this obstacle type is typically breakable
    pub fn is_breakable(&self) -> bool {
        matches!(self, ObstacleType::Window | ObstacleType::Barricade)
    }

    /// Returns true if attacks can pass through this obstacle
    pub fn allows_attack_through(&self) -> bool {
        matches!(self, ObstacleType::Window | ObstacleType::LowCover)
    }

    /// Returns true if this blocks ground movement by default
    pub fn blocks_ground(&self) -> bool {
        !matches!(self, ObstacleType::LowCover)
    }

    /// Returns true if this blocks flying movement
    pub fn blocks_flying(&self) -> bool {
        matches!(self, ObstacleType::Wall)
    }
}

/// Generic obstacle component that replaces hardcoded Window behavior
#[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize)]
pub struct Obstacle {
    /// Type of obstacle (determines default behaviors)
    pub obstacle_type: ObstacleType,
    /// Whether this obstacle blocks movement
    pub blocks_movement: bool,
    /// Whether attacks can pass through this obstacle
    pub allows_attack_through: bool,
    /// Whether this obstacle can be destroyed
    pub breakable: bool,
    /// Current health (if breakable)
    pub health: Option<u32>,
    /// Maximum health (if breakable)
    pub max_health: Option<u32>,
}

impl Default for Obstacle {
    fn default() -> Self {
        Self {
            obstacle_type: ObstacleType::Wall,
            blocks_movement: true,
            allows_attack_through: false,
            breakable: false,
            health: None,
            max_health: None,
        }
    }
}

impl Obstacle {
    /// Create a new obstacle with default properties for the given type
    pub fn new(obstacle_type: ObstacleType) -> Self {
        match obstacle_type {
            ObstacleType::Wall => Self {
                obstacle_type,
                blocks_movement: true,
                allows_attack_through: false,
                breakable: false,
                health: None,
                max_health: None,
            },
            ObstacleType::Window => Self {
                obstacle_type,
                blocks_movement: true,
                allows_attack_through: true,
                breakable: true,
                health: Some(3),
                max_health: Some(3),
            },
            ObstacleType::Barricade => Self {
                obstacle_type,
                blocks_movement: true,
                allows_attack_through: false,
                breakable: true,
                health: Some(5),
                max_health: Some(5),
            },
            ObstacleType::Water => Self {
                obstacle_type,
                blocks_movement: true,
                allows_attack_through: true,
                breakable: false,
                health: None,
                max_health: None,
            },
            ObstacleType::Pit => Self {
                obstacle_type,
                blocks_movement: true,
                allows_attack_through: true,
                breakable: false,
                health: None,
                max_health: None,
            },
            ObstacleType::LowCover => Self {
                obstacle_type,
                blocks_movement: true,
                allows_attack_through: true,
                breakable: true,
                health: Some(2),
                max_health: Some(2),
            },
        }
    }

    /// Create a window obstacle
    pub fn window() -> Self {
        Self::new(ObstacleType::Window)
    }

    /// Create a barricade obstacle
    pub fn barricade() -> Self {
        Self::new(ObstacleType::Barricade)
    }

    /// Create with custom health
    pub fn with_health(mut self, health: u32) -> Self {
        self.health = Some(health);
        self.max_health = Some(health);
        self.breakable = true;
        self
    }

    /// Apply damage to this obstacle, returns true if destroyed
    pub fn take_damage(&mut self, damage: u32) -> bool {
        if let Some(ref mut health) = self.health {
            *health = health.saturating_sub(damage);
            if *health == 0 {
                self.blocks_movement = false;
                return true;
            }
        }
        false
    }

    /// Check if this obstacle is destroyed
    pub fn is_destroyed(&self) -> bool {
        self.health == Some(0)
    }

    /// Check if this obstacle is intact (has health or is not breakable)
    pub fn is_intact(&self) -> bool {
        match self.health {
            Some(h) => h > 0,
            None => true, // Non-breakable obstacles are always "intact"
        }
    }
}

/// RON configuration for obstacle definitions
#[derive(Clone, Debug, Serialize, Deserialize, Asset, TypePath)]
pub struct ObstacleConfig {
    pub obstacle_type: ObstacleType,
    pub blocks_movement: Option<bool>,
    pub allows_attack_through: Option<bool>,
    pub breakable: Option<bool>,
    pub health: Option<u32>,
}

impl From<&ObstacleConfig> for Obstacle {
    fn from(config: &ObstacleConfig) -> Self {
        let mut obstacle = Obstacle::new(config.obstacle_type);

        if let Some(blocks) = config.blocks_movement {
            obstacle.blocks_movement = blocks;
        }
        if let Some(allows) = config.allows_attack_through {
            obstacle.allows_attack_through = allows;
        }
        if let Some(breakable) = config.breakable {
            obstacle.breakable = breakable;
        }
        if let Some(health) = config.health {
            obstacle.health = Some(health);
            obstacle.max_health = Some(health);
        }

        obstacle
    }
}

/// Event fired when an obstacle is attacked
#[derive(Event, Message, Clone, Debug)]
pub struct ObstacleAttackEvent {
    pub attacker: Entity,
    pub obstacle: Entity,
    pub damage: u32,
}

/// Event fired when an obstacle is destroyed
#[derive(Event, Message, Clone, Debug)]
pub struct ObstacleDestroyedEvent {
    pub obstacle: Entity,
    pub obstacle_type: ObstacleType,
    pub destroyed_by: Option<Entity>,
}

/// System to process obstacle damage
pub fn process_obstacle_damage(
    mut attack_events: MessageReader<ObstacleAttackEvent>,
    mut obstacle_query: Query<(Entity, &mut Obstacle)>,
    mut destroyed_events: MessageWriter<ObstacleDestroyedEvent>,
) {
    for event in attack_events.read() {
        if let Ok((entity, mut obstacle)) = obstacle_query.get_mut(event.obstacle) {
            let destroyed = obstacle.take_damage(event.damage);
            if destroyed {
                destroyed_events.write(ObstacleDestroyedEvent {
                    obstacle: entity,
                    obstacle_type: obstacle.obstacle_type,
                    destroyed_by: Some(event.attacker),
                });
            }
        }
    }
}
