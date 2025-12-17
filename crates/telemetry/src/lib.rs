use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub mod panic;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub level: String,
    pub message: String,
    pub frame: Option<u64>,
    pub checksum_local: Option<u128>,
    pub checksum_remote: Option<u128>,
    pub extra: Option<String>,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogPayload {
    pub stream: String,
    pub values: Vec<Vec<serde_json::Value>>,
}

#[derive(Resource, Clone, Debug)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub url: String,
    pub auth_token: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://localhost:5080/api/default/default/_json".to_string(),
            auth_token: "".to_string(),
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct TelemetryBuffer {
    pub events: Arc<Mutex<Vec<TelemetryEvent>>>,
}

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TelemetryConfig>()
           .init_resource::<TelemetryBuffer>();

        panic::register_panic_hook(app);
    }
}

pub fn send_event(config: &TelemetryConfig, event: TelemetryEvent) {
    if !config.enabled {
        return;
    }

    // We will implement the actual sending logic here or via a system.
    // Since this might be called from systems, we probably want to queue it
    // or spawn a task.
    // For now, let's just print it to verify.
    // println!("Telemetry Event: {:?}", event);

    // Actually, for Desync, we can spawn a task here.
    let config = config.clone();

    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        crate::panic::send_log_blocking(&config, &event);
    });

    #[cfg(target_arch = "wasm32")]
    {
        let event = event.clone();
        wasm_bindgen_futures::spawn_local(async move {
            crate::panic::send_log_wasm(&config, &event).await;
        });
    }
}
