use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, prelude::*};
use utils::frame::FrameCount;

use crate::core::GameInfo;

// You can also register resources.

pub fn increase_frame_system(mut frame_count: ResMut<FrameCount>) {
    frame_count.frame += 1;
}

// DEBUG

#[derive(Component)]
struct FrameCountText;

fn setup_frame_counter_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands.spawn((
        FrameCountText,
        Text::new("Frame Count"),
        TextFont {
            font,
            font_size: 16.0,
            ..Default::default()
        },
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        },
    ));
}

fn update_frame_counter_text(
    frame_count: Res<FrameCount>, // Access the FrameCount resource
    game_info: Res<GameInfo>,
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FrameCountText>>, // Query for mutable Text components with our marker
) {
    for mut text in &mut query {
    
    let fps_text = if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = fps.smoothed() {
            // Fixed width format: always 5 characters (e.g., " 60.0", "120.0")
            format!("{:>5.1}", value)
        } else {
            "  ...".to_string()
        }
    } else {
        "  ...".to_string()
    };
        
        // Fixed width format for frame count (8 characters) to prevent layout shifts
        text.0 = format!("{} : {:>8} | FPS: {}", game_info.version, frame_count.frame, fps_text);
    }
}

pub struct FrameDebugUIPlugin;

impl Plugin for FrameDebugUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_frame_counter_ui);
        app.add_systems(Update, update_frame_counter_text);
    }
}
