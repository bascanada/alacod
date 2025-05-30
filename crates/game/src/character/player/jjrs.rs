use bevy_ggrs::GgrsConfig;
use bevy_matchbox::prelude::PeerId;

use super::input::BoxInput;

pub type BoxConfig = GgrsConfig<BoxInput>;
pub type PeerConfig = GgrsConfig<BoxInput, PeerId>;
