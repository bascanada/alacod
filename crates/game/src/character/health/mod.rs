pub mod ui;

use bevy::{log::{tracing::span, Level}, prelude::*};
use bevy_fixed::fixed_math;
use bevy_ggrs::Rollback;
use ggrs::PlayerHandle;
use serde::{Deserialize, Serialize};
use std::fmt;
use utils::{frame::FrameCount, net_id::GgrsNetId};

#[derive(Component, Reflect, Debug, Clone, Serialize, Deserialize)]
pub enum HitBy {
    Entity(GgrsNetId),
    Player(PlayerHandle),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthConfig {
    pub max: fixed_math::Fixed,
}

#[derive(Component, Clone, Debug, Serialize, Default, Deserialize)]
pub struct Health {
    pub current: fixed_math::Fixed,
    pub max: fixed_math::Fixed,
    pub invulnerable_until_frame: Option<u32>, // Optional invulnerability window
}

#[derive(Component, Clone, Debug, Serialize, Deserialize, Default)]
pub struct Death {
    pub last_hit_by: Option<Vec<HitBy>>,
}

#[derive(Component, Clone, Serialize, Deserialize, Default)]
pub struct DamageAccumulator {
    pub total_damage: fixed_math::Fixed,
    pub hit_count: u32,
    pub last_hit_by: Option<Vec<HitBy>>,
}

impl fmt::Display for HitBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HitBy::Entity(net_id) => write!(f, "NetId({})", net_id.0),
            HitBy::Player(player_handle) => write!(f, "Player({})", player_handle),
        }
    }
}

impl fmt::Display for Health {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HP: {}/{}", self.current, self.max)?;
        if let Some(frame) = self.invulnerable_until_frame {
            write!(f, " (Invulnerable until frame {})", frame)?;
        }
        Ok(())
    }
}

impl fmt::Display for Death {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.last_hit_by {
            Some(hits) if !hits.is_empty() => {
                for (i, hit_by) in hits.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", hit_by)?;
                }
                Ok(())
            }
            Some(_) | None => write!(f, "Died (cause unknown or no direct hit)"),
        }
    }
}

impl From<HealthConfig> for Health {
    fn from(value: HealthConfig) -> Self {
        Self {
            current: value.max,
            max: value.max,
            invulnerable_until_frame: None,
        }
    }
}

pub fn rollback_apply_accumulated_damage(
    frame: Res<FrameCount>,
    mut commands: Commands,
    mut query: Query<(Entity, &DamageAccumulator, &GgrsNetId, &mut Health), With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "apply_damage");
    let _enter = system_span.enter();

    for (entity, accumulator, g_id, mut health) in query.iter_mut() {
        if accumulator.total_damage > fixed_math::FIXED_ZERO {
            health.current = health.current.saturating_sub(accumulator.total_damage);

            info!(
                "{} receive {} dmg health is {}",
                g_id, accumulator.total_damage, health.current
            );

            commands.entity(entity).remove::<DamageAccumulator>();

            if health.current <= fixed_math::FIXED_ZERO {
                commands.entity(entity).insert(Death {
                    last_hit_by: accumulator.last_hit_by.clone(),
                });
            }
        }
    }
}

pub fn rollback_apply_death(
    frame: Res<FrameCount>,
    mut commands: Commands,
    query: Query<(Entity, &GgrsNetId, &Death), With<Rollback>>,
) {
    let system_span = span!(Level::INFO, "ggrs", f = frame.frame, s = "apply_death");
    let _enter = system_span.enter();

    let mut entities_to_despawn: Vec<(Entity, GgrsNetId, Death)> = query
        .iter()
        .map(|(entity, id, death_component)| (entity, id.clone(), death_component.clone())) // Clone Death if needed for logging
        .collect();

    // Sort by a stable identifier, like Entity's bits.
    // This step is optional but adds robustness against non-deterministic iteration order if it were to affect anything.
    entities_to_despawn.sort_unstable_by_key(|(e, _, _)| e.to_bits());

    for (entity, id, death_info) in entities_to_despawn {
        info!("{} entity {} killed by {}", frame.as_ref(), id, death_info); // Use cloned death_info
        commands.entity(entity).despawn();
    }
}
