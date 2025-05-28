use bevy::prelude::*;
use bevy_fixed::math::calculate_time_remaining_seconds;
use utils::frame::FrameCount;

use crate::{character::player::LocalPlayer, plugins::AppState};

use super::{WeaponInventory, WeaponModeState, WeaponModesState, WeaponState};


#[derive(Component)]
struct CurrentWeaponText;

#[derive(Component)]
struct AmmoText;

#[derive(Component)]
struct ReloadingText;



fn setup_weapon_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands.spawn((
        ReloadingText,
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: 16.0,
            ..Default::default()
        },
        TextLayout::new_with_justify(JustifyText::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(37.0),
            left: Val::Px(5.0),
            ..default()
        },
    ));


    commands.spawn((
        CurrentWeaponText,
        Text::new("Weapon: "),
        TextFont {
            font: font.clone(),
            font_size: 16.0,
            ..Default::default()
        },
        TextLayout::new_with_justify(JustifyText::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(5.0),
            ..default()
        },
    ));

    commands.spawn((
        AmmoText,
        Text::new("Ammo: "),
        TextFont {
            font: font,
            font_size: 16.0,
            ..Default::default()
        },
        TextLayout::new_with_justify(JustifyText::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(3.0),
            left: Val::Px(5.0),
            ..default()
        },
    ));
}

fn update_weapons_text(
    frame: Res<FrameCount>,
    q_player: Query<&WeaponInventory, With<LocalPlayer>>,
    weapon_query: Query<(&WeaponState, &WeaponModesState)>,
    mut q_weapon: Query<&mut Text, (With<CurrentWeaponText>, Without<AmmoText>)>,
    mut q_ammo: Query<&mut Text, (With<AmmoText>, Without<CurrentWeaponText>)>,
    mut q_reloading: Query<&mut Text, (With<ReloadingText>, Without<CurrentWeaponText>, Without<AmmoText>)>,
) {
    if let Ok(inventory) = q_player.get_single() {
        let active_weapon = inventory.active_weapon();
        if let Ok((state, modes_state)) = weapon_query.get(active_weapon.0) {
            let active_weapon_state = modes_state.modes.get(&state.active_mode).unwrap();
            if let Ok(mut text) = q_weapon.get_single_mut() {
                text.0 = format!("Weapon: {} - {}", active_weapon.1.config.name, state.active_mode);
            }
            if let Ok(mut text) = q_ammo.get_single_mut() {
                text.0 = format!("Ammo: {} / {}", active_weapon_state.mag_ammo, active_weapon_state.mag_quantity)
            }

            if let Ok(mut text) = q_reloading.get_single_mut() {
                text.0 = if inventory.is_reloading() {
                    format!("{:.2}s", calculate_time_remaining_seconds(inventory.reloading_ending_frame.unwrap(), frame.frame))
                } else {
                    format!("")
                };
            }
        }
    }
}


#[derive(Default)]
pub struct WeaponDebugUIPlugin;

impl Plugin for WeaponDebugUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup_weapon_ui);
        app.add_systems(Update, update_weapons_text.run_if(in_state(AppState::InGame)));
    }
}