
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

// Choisissez un type pour votre ID. usize ou u32 est courant.
// bevy_ggrs utilise souvent usize pour ses handles.
pub type StableIdType = usize; // Ou u32, etc.

#[derive(Component, Reflect, Serialize, Deserialize, Debug,  Clone, PartialEq, Eq, Hash)]
pub struct GgrsNetId(pub StableIdType, pub String);

// Implémentez Display pour une jolie journalisation
impl fmt::Display for GgrsNetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}-{})", self.1, self.0)
    }
}




#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct GgrsNetIdFactory {
    counter: StableIdType,
}

impl GgrsNetIdFactory {
    pub fn next(&mut self, name: String) -> GgrsNetId {
        self.counter += 1;

        GgrsNetId(self.counter, name)
    }
}