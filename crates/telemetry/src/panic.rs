use bevy::prelude::*;
use std::panic;
use std::sync::{Arc, Mutex};
use crate::{TelemetryConfig, TelemetryEvent};
use chrono::Utc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{Request, RequestInit, RequestMode, Response};

// We use a static mutex to store the config so the panic hook can access it.
// This is a bit hacky but standard for global panic hooks.
static TELEMETRY_CONFIG: Mutex<Option<TelemetryConfig>> = Mutex::new(None);

pub fn register_panic_hook(app: &mut App) {
    // We can't easily access the resource here because it might not be initialized yet
    // or we are just building the app.
    // Instead, we can add a startup system that sets the global config.
    app.add_systems(Startup, setup_panic_hook_config);
}

fn setup_panic_hook_config(config: Res<TelemetryConfig>) {
    let mut global_config = TELEMETRY_CONFIG.lock().unwrap();
    *global_config = Some(config.clone());

    if config.enabled {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            // Call the default hook first so we see the panic in console
            default_hook(info);

            // Now send telemetry
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                format!("Panic: {}", s)
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                format!("Panic: {}", s)
            } else {
                "Panic: Unknown payload".to_string()
            };

            let location = if let Some(location) = info.location() {
                format!(" at {}:{}:{}", location.file(), location.line(), location.column())
            } else {
                "".to_string()
            };

            let full_message = format!("{}{}", msg, location);

            let event = TelemetryEvent {
                level: "CRITICAL".to_string(),
                message: full_message,
                frame: None,
                checksum_local: None,
                checksum_remote: None,
                extra: None,
                timestamp: Utc::now().timestamp_micros(),
            };

            if let Some(config) = TELEMETRY_CONFIG.lock().unwrap().as_ref() {
                #[cfg(not(target_arch = "wasm32"))]
                send_log_blocking(config, &event);

                #[cfg(target_arch = "wasm32")]
                send_log_wasm_panic(config, &event);
            }
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_log_blocking(config: &TelemetryConfig, event: &TelemetryEvent) {
    // Construct OpenObserve payload
    // Payload format: JSON array of objects
    let payload = serde_json::json!([event]);

    let url = &config.url;
    let auth = &config.auth_token;

    // Basic Auth header if token is provided
    // If token is just the password/token, OpenObserve usually uses Basic Auth with username as email or just the token in header?
    // User provided "Basic Auth token" in the prompt "Add a --telemetry-auth arg (string) for the Basic Auth token."
    // So we assume the user provides the full base64 string or the token to be used.
    // If it's a "Basic Auth token", it usually means "Basic <base64>" or just the token.
    // Let's assume the user passes the header value or just the key.
    // Usually it is `Authorization: Basic <token>`.

    let mut request = ureq::post(url)
        .set("Content-Type", "application/json");

    if !auth.is_empty() {
        request = request.set("Authorization", auth);
    }

    match request.send_json(payload) {
        Ok(_) => eprintln!("Telemetry sent successfully."),
        Err(e) => eprintln!("Failed to send telemetry: {}", e),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn send_log_wasm_panic(config: &TelemetryConfig, event: &TelemetryEvent) {
    // In a panic on WASM, async execution might be cut short.
    // We can try to use `fetch` with `keepalive: true` which is designed for this.
    // We cannot await here, so we fire and forget, hoping the browser handles keepalive.

    let payload = serde_json::json!([event]);
    let body = serde_json::to_string(&payload).unwrap();

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);
    opts.body(Some(&JsValue::from_str(&body)));
    opts.keepalive(true); // Crucial for page unload/crash scenarios

    let url = config.url.clone();
    let auth = config.auth_token.clone();

    // We can't easily add headers to RequestInit directly in a way that overrides everything without using the Headers object
    // But RequestInit has a `headers` field.
    let headers = web_sys::Headers::new().unwrap();
    headers.append("Content-Type", "application/json").unwrap();
    if !auth.is_empty() {
        headers.append("Authorization", &auth).unwrap();
    }
    opts.headers(&headers);

    let request = Request::new_with_str_and_init(&url, &opts).unwrap();

    let window = web_sys::window().unwrap();
    let _ = window.fetch_with_request(&request); // Fire and forget
}

#[cfg(target_arch = "wasm32")]
pub async fn send_log_wasm(config: &TelemetryConfig, event: &TelemetryEvent) {
    let payload = serde_json::json!([event]);
    let body = serde_json::to_string(&payload).unwrap();

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);
    opts.body(Some(&JsValue::from_str(&body)));

    let headers = web_sys::Headers::new().unwrap();
    headers.append("Content-Type", "application/json").unwrap();
    if !config.auth_token.is_empty() {
        headers.append("Authorization", &config.auth_token).unwrap();
    }
    opts.headers(&headers);

    let request = Request::new_with_str_and_init(&config.url, &opts).unwrap();

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await;

    if let Err(e) = resp_value {
        web_sys::console::error_1(&e);
    }
}
