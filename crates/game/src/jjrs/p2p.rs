

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
    }, collider::{spawn_test_wall, CollisionSettings}, core::AppState, global_asset::GlobalAsset, jjrs::{GggrsSessionConfiguration, GgrsPlayer, GgrsSessionBuilding}, weapons::WeaponsConfig
};


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
    mut commands: Commands,
    mut app_state: ResMut<NextState<AppState>>,
    mut socket: ResMut<MatchboxSocket>,
    ggrs_config: Res<GggrsSessionConfiguration>,
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

    info!("all players are ready, loading the game");

    commands.insert_resource(GgrsSessionBuilding{
        players: players.iter().enumerate().map(|(i, x)| GgrsPlayer { handle: i, is_local: matches!(x, PlayerType::Local)}).collect(),
    });

    app_state.set(AppState::GameLoading);

}


pub fn system_after_map_loaded(
    mut commands: Commands,

    mut app_state: ResMut<NextState<AppState>>,
    mut socket: ResMut<MatchboxSocket>,
    ggrs_config: Res<GggrsSessionConfiguration>,
) {
    let channel = socket.take_channel(0).unwrap();
    let num_players = ggrs_config.connection.max_player;

    // start the GGRS session
    let mut session_builder = ggrs::SessionBuilder::<PeerConfig>::new()
        .with_num_players(num_players)
        .with_max_prediction_window(12)
        .with_input_delay(ggrs_config.connection.input_delay);

    let players = socket.players();

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }

    let ggrs_session = session_builder
        .start_p2p_session(channel)
        .expect("failed to start session");

    commands.insert_resource(RollbackRng::new(12345));
    commands.insert_resource(bevy_ggrs::Session::P2P(ggrs_session));

    app_state.set(AppState::InGame);
}