
use std::net::SocketAddr;

use clap::Parser;

#[derive(Parser)]
pub struct Opt {
    #[clap(short, long,)]
    pub matchbox: Option<String>,
    #[clap(short, long)]
    pub number_player: Option<usize>,
    #[clap(short, long)]
    pub local_port: Option<u16>,
    #[clap(short, long, num_args = 1..)]
    pub players: Option<Vec<String>>,
    #[clap(short, long, num_args = 1..)]
    pub spectators: Option<Vec<SocketAddr>>,
}