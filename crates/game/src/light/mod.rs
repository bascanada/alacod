use bevy::prelude::*;
use bevy_light_2d::plugin::Light2dPlugin;



#[derive(Default)]
pub struct ZLightPlugin;

impl Plugin for ZLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Light2dPlugin);
    }
}