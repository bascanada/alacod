use std::net::SocketAddr;

use bevy::prelude::*;
use utils::{cid::generate_random_correlation_id};

use crate::{
    core::OnlineState,
    jjrs::{GggrsConnectionConfiguration, GggrsSessionConfiguration},
};

#[cfg(not(target_arch = "wasm32"))]
mod cli;
#[cfg(target_arch = "wasm32")]
mod web;

pub fn get_args() -> (
    u16,
    usize,
    Vec<String>,
    Vec<SocketAddr>,
    String,
    String,
    String,
    bool,
    String,
    String,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use clap::Parser;
        let args = cli::Opt::parse();

        (
            args.local_port.unwrap_or(0),
            args.number_player.unwrap_or(0),
            args.players.unwrap_or(vec![]),
            args.spectators.unwrap_or(vec![]),
            args.matchbox.unwrap_or(String::new()),
            args.lobby.unwrap_or(String::new()),
            args.cid.unwrap_or(generate_random_correlation_id()),
            args.telemetry,
            args.telemetry_url,
            args.telemetry_auth,
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        use web::read_canvas_data_system;
        let args = read_canvas_data_system();
        return (
            0,
            args.number_player.unwrap_or(1),
            vec!["localhost".to_string()],
            vec![],
            args.matchbox.unwrap_or(String::new()),
            args.lobby.unwrap_or(String::new()),
            generate_random_correlation_id(),
            args.telemetry,
            args.telemetry_url,
            args.telemetry_auth,
        );
    }
}

pub struct BaseArgsPlugin;

impl Plugin for BaseArgsPlugin {
    fn build(&self, app: &mut App) {
        let (local_port, mut nbr_player, players, _, matchbox, lobby, cid, telemetry, telemetry_url, telemetry_auth) = get_args();

        if nbr_player == 0 {
            nbr_player = players.len()
        }

        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(utils::logs::NativeLogPlugin(cid.clone()));

        app.add_plugins(telemetry::TelemetryPlugin);
        app.insert_resource(telemetry::TelemetryConfig {
            enabled: telemetry,
            url: telemetry_url,
            auth_token: telemetry_auth,
        });

        app
            .insert_resource(if !matchbox.is_empty() {
                OnlineState::Online
            } else {
                OnlineState::Offline
            })
            .insert_resource(GggrsSessionConfiguration {
                cid,
                matchbox: !matchbox.is_empty(),
                lobby: lobby.clone(),
                matchbox_url: matchbox.clone(),
                connection: GggrsConnectionConfiguration {
                    input_delay: 5,
                    max_player: nbr_player,
                    desync_interval: 10,
                    socket: players.len() > 1,
                    udp_port: local_port,
                },
                players,
            });
    }
}
