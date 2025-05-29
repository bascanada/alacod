use bevy_ggrs::GgrsConfig;
use bevy_matchbox::prelude::PeerId;
use serde::{Deserialize, Serialize};

use super::input::BoxInput;

pub type BoxConfig = GgrsConfig<BoxInput>;
pub type PeerConfig = GgrsConfig<BoxInput, PeerId>;
