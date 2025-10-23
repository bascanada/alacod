

use bevy::prelude::*;
use bevy_ggrs::ggrs::PlayerType;
use bevy_matchbox::{prelude::PeerState, MatchboxSocket};
use bevy_fixed::rng::RollbackRng;

use crate::{
    character::player::jjrs::PeerConfig,
    core::{AppState, OnlineState}, 
    jjrs::{GggrsSessionConfiguration, GggrsSessionConfigurationState, GgrsPlayer, GgrsSessionBuilding},
};


// For matchbox socket connection

pub fn start_matchbox_socket(mut commands: Commands, ggrs_config: Res<GggrsSessionConfiguration>) {
    let url = format!(
        "{}/{}",
        ggrs_config.matchbox_url, ggrs_config.lobby
    );
    commands.insert_resource(MatchboxSocket::new_unreliable(url));

    info!("start p2p connection with CID={}", ggrs_config.cid);
}

pub fn wait_for_players(
    mut commands: Commands,
    mut app_state: ResMut<NextState<AppState>>,
    mut socket: ResMut<MatchboxSocket>,
    ggrs_config: Res<GggrsSessionConfiguration>,
    online_state: Res<OnlineState>,
    session_state: Res<GggrsSessionConfigurationState>,
) {
    if !matches!(online_state.as_ref(), OnlineState::Online) {
        return;
    }

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
    
    // Log the current state of player connections
    if players.len() < num_players {
        info!("Waiting for players: {}/{} connected", players.len(), num_players);
        return; // wait for more players
    }

    if !session_state.ready {
        info!("All players connected ({}/{}), but waiting for session configuration to be ready", players.len(), num_players);
        return;
    }

    info!("All {} players are connected and ready, transitioning to GameLoading", num_players);

    commands.insert_resource(GgrsSessionBuilding{
        players: players.iter().enumerate().map(|(i, x)| GgrsPlayer { handle: i, is_local: matches!(x, PlayerType::Local)}).collect(),
    });

    app_state.set(AppState::GameLoading);

}


pub fn system_after_map_loaded(
    mut commands: Commands,

    mut app_state: ResMut<NextState<AppState>>,
    mut socket: Option<ResMut<MatchboxSocket>>,
    ggrs_config: Res<GggrsSessionConfiguration>,
    online_state: Res<OnlineState>,
) {
    if !matches!(online_state.as_ref(), OnlineState::Online) {
        return;
    }

    let socket = socket.as_mut().unwrap();

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