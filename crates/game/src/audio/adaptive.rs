//! Adaptive music system that adjusts harmonium parameters based on game state.
//!
//! This system runs OUTSIDE the GGRS rollback loop (PostUpdate schedule) since
//! music generation is a visual/audio presentation concern, not game state.

use bevy::prelude::*;
use harmonium_bevy::components::{HarmoniumSource, GenerativeConfig};
use crate::character::enemy::Enemy;
use crate::character::player::Player;
use crate::character::health::Health;
use crate::waves::state::{WaveState, WavePhase};

/// Configuration for how game state maps to music parameters
#[derive(Resource, Clone)]
pub struct AdaptiveMusicConfig {
    /// Enable/disable adaptive music (for debugging)
    pub enabled: bool,

    /// How quickly parameters interpolate to target values (0.0-1.0)
    /// Lower = smoother transitions, Higher = more reactive
    pub smoothing_factor: f32,

    /// Enemy count thresholds for intensity scaling
    pub low_enemy_threshold: usize,
    pub high_enemy_threshold: usize,
}

impl Default for AdaptiveMusicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            smoothing_factor: 0.05, // Smooth transitions over ~20 frames
            low_enemy_threshold: 5,
            high_enemy_threshold: 20,
        }
    }
}

/// System that updates harmonium parameters based on gameplay state
pub fn update_adaptive_music(
    config: Res<AdaptiveMusicConfig>,
    wave_state: Res<WaveState>,
    mut harmonium_query: Query<&mut HarmoniumSource>,
    enemy_query: Query<&Enemy>,
    player_query: Query<&Health, With<Player>>,
) {
    if !config.enabled {
        return;
    }

    let Ok(mut harmonium) = harmonium_query.single_mut() else {
        return;
    };

    // === GATHER GAME STATE ===

    let enemy_count = enemy_query.iter().count();
    let wave_number = wave_state.current_wave;
    let wave_phase = wave_state.phase;

    // Calculate average player health percentage
    let player_health_pct = if player_query.is_empty() {
        0.5 // Default if no players
    } else {
        let total: f32 = player_query.iter()
            .map(|health| {
                let current = health.current.to_num::<f32>();
                let max = health.max.to_num::<f32>();
                if max > 0.0 { current / max } else { 0.0 }
            })
            .sum();
        (total / player_query.iter().count() as f32).clamp(0.0, 1.0)
    };

    // === MAP TO MUSIC PARAMETERS ===

    let target_params = calculate_music_parameters(
        enemy_count,
        wave_number,
        wave_phase,
        player_health_pct,
        &config,
    );

    // === SMOOTH INTERPOLATION ===

    let current = &mut harmonium.config;
    let smoothing = config.smoothing_factor;

    // Lerp each parameter toward target
    current.tempo = lerp(current.tempo, target_params.tempo, smoothing);
    current.density = lerp(current.density, target_params.density, smoothing);
    current.tension = lerp(current.tension, target_params.tension, smoothing);
}

/// Calculate target music parameters based on game state
fn calculate_music_parameters(
    enemy_count: usize,
    wave_number: u32,
    wave_phase: WavePhase,
    player_health_pct: f32,
    config: &AdaptiveMusicConfig,
) -> GenerativeConfig {
    // === AROUSAL (Tempo/BPM) ===
    // Based on: enemy count + wave progression
    // More enemies + higher wave = faster tempo

    let enemy_intensity = (enemy_count as f32 - config.low_enemy_threshold as f32)
        / (config.high_enemy_threshold as f32 - config.low_enemy_threshold as f32);
    let enemy_intensity = enemy_intensity.clamp(0.0, 1.0);

    let wave_intensity = (wave_number as f32 / 10.0).min(1.0); // Scale over first 10 waves

    // Combine: 60% enemy count, 40% wave number
    let arousal = (enemy_intensity * 0.6 + wave_intensity * 0.4).clamp(0.0, 1.0);

    // Map arousal to BPM: 90-160 range
    let base_tempo = 90.0;
    let max_tempo = 160.0;
    let tempo = base_tempo + (arousal * (max_tempo - base_tempo));

    // === DENSITY (Rhythm Complexity) ===
    // Based on: wave phase + enemy count
    // Active combat = more complex rhythms

    let phase_density = match wave_phase {
        WavePhase::NotStarted => 0.2,
        WavePhase::GracePeriod => 0.3,
        WavePhase::Spawning => 0.6,
        WavePhase::InProgress => 0.7,
        WavePhase::WaveComplete => 0.4,
    };

    let enemy_density = (enemy_count as f32 / config.high_enemy_threshold as f32).min(1.0);

    // Combine: 50/50 phase and enemy count
    let density = (phase_density * 0.5 + enemy_density * 0.5).clamp(0.2, 0.9);

    // === TENSION (Dissonance) ===
    // Based on: danger level (many enemies + low health)
    // High danger = high tension

    let health_danger = 1.0 - player_health_pct; // Invert: low health = high danger
    let enemy_danger = enemy_intensity;

    // Combine: 60% health, 40% enemies (health is more important for tension)
    let tension = (health_danger * 0.6 + enemy_danger * 0.4).clamp(0.1, 0.9);

    GenerativeConfig {
        rhythm_mode: harmonium_bevy::harmonium_core::sequencer::RhythmMode::Euclidean,
        tempo,
        density,
        tension,
        steps: 16,
    }
}

/// Linear interpolation helper
fn lerp(current: f32, target: f32, t: f32) -> f32 {
    current + (target - current) * t
}

// === DEBUG SYSTEM ===

/// Debug overlay showing current music parameter mappings
#[cfg(debug_assertions)]
pub fn debug_adaptive_music(
    mut gizmos: Gizmos,
    harmonium_query: Query<&HarmoniumSource>,
    wave_state: Res<WaveState>,
    enemy_query: Query<&Enemy>,
    player_query: Query<&Health, With<Player>>,
) {
    let Ok(harmonium) = harmonium_query.single() else {
        return;
    };

    let enemy_count = enemy_query.iter().count();
    let wave = wave_state.current_wave;

    let player_health_pct = if player_query.is_empty() {
        0.0
    } else {
        let total: f32 = player_query.iter()
            .map(|h| (h.current.to_num::<f32>() / h.max.to_num::<f32>()).clamp(0.0, 1.0))
            .sum();
        total / player_query.iter().count() as f32
    };

    // Display debug info (this would need a proper UI system)
    // For now, just log periodically
    // TODO: Add proper debug UI overlay
}
