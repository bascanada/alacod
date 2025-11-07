use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemySpawnConfig {
    // Can add more config later if needed (spawn type, rate, etc.)
}

impl Default for EnemySpawnConfig {
    fn default() -> Self {
        Self {}
    }
}
