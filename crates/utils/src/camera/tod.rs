use bevy::prelude::*;
use bevy::input::ButtonInput;

pub fn move_camera(
    mut players: Query<&mut Transform, With<Camera2d>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let movement_direction = if input.pressed(KeyCode::KeyW) {
        (0, 1)
    } else if input.pressed(KeyCode::KeyA) {
        (-1, 0)
    } else if input.pressed(KeyCode::KeyS) {
        (0, -1)
    } else if input.pressed(KeyCode::KeyD) {
        (1, 0)
    } else {
        return;
    };

    for mut transform in players.iter_mut() {
        transform.translation.x += movement_direction.0 as f32 * 3.0;
        transform.translation.y += movement_direction.1 as f32 * 3.0;
    }

}


pub fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.projection.scale = 1.3;
    commands.spawn(camera);
}

