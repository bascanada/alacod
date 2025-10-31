use bevy::prelude::*;
use bevy_fixed::fixed_math;

#[derive(Component, Clone)]
pub struct EnemySpawnerComponent {
    pub spawn_radius: fixed_math::Fixed,
    pub min_spawn_distance: fixed_math::Fixed,
    pub max_cooldown: u32,
    pub max_enemies: u32,
    pub enemy_types: Vec<String>,
}

impl Default for EnemySpawnerComponent {
    fn default() -> Self {
        Self {
            spawn_radius: fixed_math::new(50.0),
            min_spawn_distance: fixed_math::new(200.0),
            max_cooldown: 600, // 10 seconds at 60fps (slower spawn rate)
            max_enemies: 3,    // Per spawner
            enemy_types: vec!["zombie_full".to_string()],
        }
    }
}
