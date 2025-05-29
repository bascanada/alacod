use bevy::prelude::*;
use utils::frame::FrameCount;

use crate::plugins::GameInfo;

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
            font: font,
            font_size: 16.0,
            ..Default::default()
        },
        TextLayout::new_with_justify(JustifyText::Center),
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
    mut query: Query<&mut Text, With<FrameCountText>>, // Query for mutable Text components with our marker
) {
    if let Ok(mut text) = query.get_single_mut() {
        text.0 = format!("{} : {}", game_info.version, frame_count.frame);
    } else {
        // Optional: Log a warning if the text entity wasn't found or multiple exist.
        warn!("Could not find unique FrameCountText entity to update.");
    }
}

pub struct FrameDebugUIPlugin;

impl Plugin for FrameDebugUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_frame_counter_ui);
        app.add_systems(Update, update_frame_counter_text);
    }
}
