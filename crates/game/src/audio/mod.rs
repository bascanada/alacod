use std::io::Cursor;

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

#[derive(Component)]
pub struct AudioState {}

pub struct ZAudioPlugin {}

impl Plugin for ZAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AudioPlugin);
        app.add_plugins(SpatialAudioPlugin);
        //app.add_systems(Startup, play_loop);
    }
}

fn play_loop(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    audio.play(asset_server.load("sounds/loop.ogg")).looped();
}
