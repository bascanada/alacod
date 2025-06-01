use animation::SpriteSheetConfig;
use bevy::{color::palettes::{css::TURQUOISE, tailwind::{ORANGE_300, PURPLE_300}}, prelude::*};
use bevy_fixed::fixed_math;
use game::{
    args::BaseArgsPlugin, character::{config::CharacterConfig, enemy::spawning::EnemySpawnerState, player::create::create_player}, collider::{spawn_test_wall, CollisionSettings}, core::{AppState, CoreSetupConfig, CoreSetupPlugin}, global_asset::GlobalAsset, jjrs::{GggrsSessionConfiguration, GggrsSessionConfigurationState, GgrsSessionBuilding}, weapons::WeaponsConfig
};
use map::game::entity::map::enemy_spawn::EnemySpawnerComponent;
use utils::net_id::GgrsNetIdFactory;
use bevy_ggrs::AddRollbackCommandExtension;

fn main() {
    let game_config = CoreSetupConfig {
        app_name: "zrl-character_tester".into(),
    };

    let core_plugin = CoreSetupPlugin(game_config);

    App::new()
        .add_plugins(core_plugin.get_default_plugin())
        .add_plugins(core_plugin)
        .add_plugins(BaseArgsPlugin)

        // Because i don't have extra configuration yet we can directly start
        .insert_resource(GggrsSessionConfigurationState::ready())
        .add_systems(OnEnter(AppState::GameLoading), (
            setup_simple_background,
            system_game_loading,
        ))
        .run();
}



fn system_game_loading(
    mut app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    collision_settings: Res<CollisionSettings>,
    global_assets: Res<GlobalAsset>,
    character_asset: Res<Assets<CharacterConfig>>,
    weapons_asset: Res<Assets<WeaponsConfig>>,

    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: Res<Assets<SpriteSheetConfig>>,

    mut id_provider: ResMut<GgrsNetIdFactory>,

    ggrs_session_building: Res<GgrsSessionBuilding>,
) {

    for ggrs_player in ggrs_session_building.players.iter() {
        let i = ggrs_player.handle;
        let is_local = ggrs_player.is_local;

        let position = fixed_math::FixedVec3::new(
            fixed_math::new(-50.0 * i as f32),
            fixed_math::new(0.0),
            fixed_math::new(0.0),
        );

        create_player(
            &mut commands,
            &global_assets,
            &weapons_asset,
            &character_asset,
            &collision_settings,
            &asset_server,
            &mut texture_atlas_layouts,
            &sprint_sheet_assets,
            position,
            is_local,
            i,
            &mut id_provider,
        );
    }

    spawn_test_map(&mut commands, &mut id_provider, &collision_settings);


    app_state.set(AppState::GameStarting);

}



fn spawn_test_map(
    commands: &mut Commands,
    id_provider: &mut ResMut<GgrsNetIdFactory>,
    collision_settings: &Res<CollisionSettings>,
) {
    spawn_test_wall(
        commands,
        Vec3::new(-1500.0, 650.0, 0.0),
        Vec2::new(125.0, 500.0),
        collision_settings,
        Color::Srgba(ORANGE_300),
        id_provider.next("wall".into()),
    );

    spawn_test_wall(
        commands,
        Vec3::new(500.0, 250.0, 0.0),
        Vec2::new(125.0, 500.0),
        collision_settings,
        Color::Srgba(PURPLE_300),
        id_provider.next("wall".into()),
    );

    spawn_test_wall(
        commands,
        Vec3::new(400.0, 1450.0, 0.0),
        Vec2::new(500.0, 125.0),
        collision_settings,
        Color::Srgba(TURQUOISE),
        id_provider.next("wall".into()),
    );

    spawn_test_wall(
        commands,
        Vec3::new(700.0, -1350.0, 0.0),
        Vec2::new(500.0, 125.0),
        collision_settings,
        Color::Srgba(TURQUOISE),
        id_provider.next("wall".into()),
    );



    let spawn_positions = [
        Vec3::new(-1000., -1000., 0.0),
        Vec3::new(-1000., 1000., 0.0),
        Vec3::new(1000., -1000., 0.0),
        Vec3::new(1000., 1000., 0.0),
    ];

    for position in spawn_positions.iter() {
        spawn_test_enemy_spawner(commands, *position);
    }
}

fn spawn_test_enemy_spawner(commands: &mut Commands, position: Vec3) {
    let transform =
        fixed_math::FixedTransform3D::from_bevy_transform(&Transform::from_translation(position));

    commands
        .spawn((
            transform.to_bevy_transform(),
            transform,
            EnemySpawnerState::default(),
            EnemySpawnerComponent::default(),
        ))
        .add_rollback();
}


fn setup_simple_background(mut commands: Commands) {
    // Background parameters
    let tile_size = 400.0;
    let grid_size = 20; // This creates a 20x20 grid of tiles

    // Create a parent entity for all background tiles
    commands
        .spawn_empty()
        .insert(Name::new("Background"))
        .with_children(|parent| {
            // Create a simple checkered pattern
            for i in -grid_size / 2..grid_size / 2 {
                for j in -grid_size / 2..grid_size / 2 {
                    // Alternate colors in a checkered pattern
                    let is_dark = (i + j) % 2 == 0;
                    let color = if is_dark {
                        Color::srgb(0.2, 0.2, 0.25) // Dark blue-gray
                    } else {
                        Color::srgb(0.3, 0.3, 0.35) // Lighter blue-gray
                    };

                    // Spawn a square sprite
                    parent.spawn((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::new(tile_size, tile_size)),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(
                            i as f32 * tile_size,
                            j as f32 * tile_size,
                            -10.0, // Behind everything else
                        )),
                    ));
                }
            }
        });
}
