pub mod p2p;
pub mod local;


use std::{default, net::SocketAddr};

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

// Shared configuration between the client for the ggrs configuration
// to apply to their game
pub struct GggrsConnectionConfiguration {
    pub max_player: usize,
    pub input_delay: usize,
    pub desync_interval: u32,
    pub socket: bool,
    pub udp_port: u16,
}

// Shared configuration between the client for the matchbox + ggrs configuration
#[derive(Resource)]
pub struct GggrsSessionConfiguration {
    pub cid: String,
    pub matchbox: bool,
    pub matchbox_url: String,
    pub lobby: String,
    pub connection: GggrsConnectionConfiguration,
    pub players: Vec<String>,
}

// This state is used to mark if extra external settings need to be configure
// the ggrs system will wait for this to be true before Loading the map
// if you need to have extra ui to configure your game
#[derive(Resource, Default)]
pub struct GggrsSessionConfigurationState {
    pub ready: bool
}

impl GggrsSessionConfigurationState {
    pub fn ready() -> Self {
        Self { ready: true }
    }
}


pub struct GgrsPlayer {
    pub handle: usize,
    pub is_local: bool,

}

// Resource to keep the information that will be used to generate the P2PSession
// after all player have joined and the game configuration is aggreed on
#[derive(Resource)]
pub struct GgrsSessionBuilding {
    pub players: Vec<GgrsPlayer>
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
