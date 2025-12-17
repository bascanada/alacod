use bevy::prelude::*;
use wasm_bindgen::JsCast; // For safe casting

#[derive(Default, Debug)]
pub struct CanvasConfig {
    pub number_player: Option<usize>,
    pub matchbox: Option<String>,
    pub lobby: Option<String>,
    pub telemetry: bool,
    pub telemetry_url: String,
    pub telemetry_auth: String,
}

pub fn read_canvas_data_system() -> CanvasConfig {
    let mut config = CanvasConfig::default();

    // Use web-sys to get the document and the element
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    // Use the same selector Bevy uses
    let canvas_element = document
        .query_selector("#bevy-canvas") // Or get_element_by_id if you prefer
        .expect("query_selector failed")
        .expect("should have #bevy-canvas element in the DOM");

    config.matchbox = canvas_element.get_attribute("data-matchbox");
    config.lobby = canvas_element.get_attribute("data-lobby");

    // Telemetry settings
    if let Some(enabled) = canvas_element.get_attribute("data-telemetry") {
        config.telemetry = enabled == "true";
    }

    if let Some(url) = canvas_element.get_attribute("data-telemetry-url") {
        config.telemetry_url = url;
    } else {
        config.telemetry_url = "http://localhost:5080/api/default/default/_json".to_string();
    }

    if let Some(auth) = canvas_element.get_attribute("data-telemetry-auth") {
        config.telemetry_auth = auth;
    }

    if let Some(nbr_str) = canvas_element.get_attribute("data-number-player") {
        match nbr_str.parse::<usize>() {
            Ok(nbr) => config.number_player = Some(nbr),
            Err(e) => error!("Failed to parse initial score '{}': {}", nbr_str, e),
        }
    }

    info!("Read config from canvas: {:?}", config);

    return config;
}
