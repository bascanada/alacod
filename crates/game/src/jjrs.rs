use std::{net::SocketAddr};

use animation::SpriteSheetConfig;
use bevy::{color::palettes::{css::TURQUOISE, tailwind::{ORANGE_300, PURPLE_300}}, prelude::*};
use bevy_fixed::{fixed_math, rng::RollbackRng};
use bevy_ggrs::{ggrs::PlayerType, prelude::*};
use bevy_matchbox::{prelude::PeerState, MatchboxSocket};
use ggrs::UdpNonBlockingSocket;
use map::game::entity::map::enemy_spawn::EnemySpawnerComponent;
use utils::net_id::GgrsNetIdFactory;

use crate::{
    character::{
        config::CharacterConfig,
        enemy::spawning::EnemySpawnerState,
        player::{create::create_player, jjrs::PeerConfig},
    },
    collider::{spawn_test_wall, CollisionSettings},
    core::AppState,
    global_asset::GlobalAsset,
    weapons::WeaponsConfig,
};

pub struct GggrsConnectionConfiguration {
    pub max_player: usize,
    pub input_delay: usize,
    pub desync_interval: u32,
    pub socket: bool,
    pub udp_port: u16,
}

#[derive(Resource)]
pub struct GggrsSessionConfiguration {
    pub cid: String,
    pub matchbox: bool,
    pub matchbox_url: String,
    pub lobby: String,
    pub connection: GggrsConnectionConfiguration,
    pub players: Vec<String>,
}

// For local connection

pub fn setup_ggrs_local(
    mut app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    collision_settings: Res<CollisionSettings>,
    global_assets: Res<GlobalAsset>,
    character_asset: Res<Assets<CharacterConfig>>,
    weapons_asset: Res<Assets<WeaponsConfig>>,

    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: Res<Assets<SpriteSheetConfig>>,
    session_config: Res<GggrsSessionConfiguration>,

    ggrs_config: Res<GggrsSessionConfiguration>,

    mut id_provider: ResMut<GgrsNetIdFactory>,
) {
    info!("start local connection with CID={}", ggrs_config.cid);

    let mut sess_build = SessionBuilder::<PeerConfig>::new()
        .with_num_players(session_config.connection.max_player)
        .with_desync_detection_mode(ggrs::DesyncDetection::On {
            interval: session_config.connection.desync_interval,
        })
        .with_input_delay(session_config.connection.input_delay);

    for (i, addr) in session_config.players.iter().enumerate() {
        let local = addr == "localhost";
        if local {
            sess_build = sess_build
                .add_player(PlayerType::Local, i)
                .expect("Failed to add player");
        } else {
            let _remote_addr: SocketAddr = addr.parse().unwrap();
            //sess_build = sess_build.add_player(PlayerType::Remote(remote_addr), i).expect("Failed to add player");
        }
        create_player(
            &mut commands,
            &global_assets,
            &weapons_asset,
            &character_asset,
            &collision_settings,
            &asset_server,
            &mut texture_atlas_layouts,
            &sprint_sheet_assets,
            local,
            i,
            &mut id_provider,
        );
    }

    spawn_test_map(&mut commands, &mut id_provider, &collision_settings);

    // Start a synctest session
    let sess = if !session_config.connection.socket {
        let sess = sess_build
            .start_synctest_session()
            .expect("Failed to start synctest session");

        Session::SyncTest(sess)
    } else {
        let _socket = UdpNonBlockingSocket::bind_to_port(session_config.connection.udp_port)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to bind udp to {}",
                    session_config.connection.udp_port
                )
            });
        panic!("");
        //let sess = sess_build.start_p2p_session(socket).expect("failed to start p2p session");

        //Session::P2P(sess)
    };

    // Insert the GGRS session resource
    commands.insert_resource(RollbackRng::new(12345));
    commands.insert_resource(sess);

    app_state.set(AppState::InGame);
}

// For matchbox socket connection

pub fn start_matchbox_socket(mut commands: Commands, ggrs_config: Res<GggrsSessionConfiguration>) {
    let url = format!(
        "{}/{}?next={}",
        ggrs_config.matchbox_url, ggrs_config.lobby, ggrs_config.connection.max_player
    );
    commands.insert_resource(MatchboxSocket::new_unreliable(url));

    info!("start p2p connection with CID={}", ggrs_config.cid);
}

pub fn wait_for_players(
    mut app_state: ResMut<NextState<AppState>>,
    character_asset: Res<Assets<CharacterConfig>>,

    collision_settings: Res<CollisionSettings>,

    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sprint_sheet_assets: Res<Assets<SpriteSheetConfig>>,
    _session_config: Res<GggrsSessionConfiguration>,

    mut commands: Commands,
    global_assets: Res<GlobalAsset>,
    weapons_asset: Res<Assets<WeaponsConfig>>,
    mut socket: ResMut<MatchboxSocket>,
    ggrs_config: Res<GggrsSessionConfiguration>,
    mut id_provider: ResMut<GgrsNetIdFactory>,
) {
    // regularly call update_peers to update the list of connected peers
    let Ok(peer_changes) = socket.try_update_peers() else {
        warn!("socket dropped");
        return;
    };

    // Check for new connections
    for (peer, new_state) in peer_changes {
        // you can also handle the specific dis(connections) as they occur:
        match new_state {
            PeerState::Connected => info!("peer {peer} connected"),
            PeerState::Disconnected => info!("peer {peer} disconnected"),
        }
    }
    let players = socket.players();

    let num_players = ggrs_config.connection.max_player;
    if players.len() < num_players {
        return; // wait for more players
    }

    info!("All peers have joined, going in-game");
    // TODO

    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<PeerConfig>::new()
        .with_num_players(num_players)
        .with_max_prediction_window(12)
        .with_input_delay(ggrs_config.connection.input_delay);

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");

        let is_local = matches!(player, PlayerType::Local);

        create_player(
            &mut commands,
            &global_assets,
            &weapons_asset,
            &character_asset,
            &collision_settings,
            &asset_server,
            &mut texture_atlas_layouts,
            &sprint_sheet_assets,
            is_local,
            i,
            &mut id_provider,
        );
    }

    spawn_test_map(&mut commands, &mut id_provider, &collision_settings);

    // move the channel out of the socket (required because GGRS takes ownership of it)
    let channel = socket.take_channel(0).unwrap();

    // start the GGRS session
    let ggrs_session = session_builder
        .start_p2p_session(channel)
        .expect("failed to start session");

    commands.insert_resource(RollbackRng::new(12345));
    commands.insert_resource(bevy_ggrs::Session::P2P(ggrs_session));

    app_state.set(AppState::InGame);
}

pub fn log_ggrs_events(mut session: ResMut<bevy_ggrs::Session<PeerConfig>>) {
    if let Session::P2P(session) = session.as_mut() {
        for event in session.events() {
            info!("GGRS Event: {:?}", event);
            match event {
                GgrsEvent::Disconnected { addr } => {
                    panic!("Other player@{:?} disconnected", addr)
                }
                GgrsEvent::DesyncDetected {
                    frame,
                    local_checksum,
                    remote_checksum,
                    addr,
                } => {
                    error!(
                        "Desync detected on frame {} local {} remote {}@{:?}",
                        frame, local_checksum, remote_checksum, addr
                    );
                }
                _ => (),
            }
        }
    }
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
