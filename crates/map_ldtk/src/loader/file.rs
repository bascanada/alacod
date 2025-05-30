use std::fs::File;
use std::io::BufReader;

use bevy_ecs_ldtk::ldtk::LdtkJson;
use serde_json::from_reader;

pub fn load_ldtk_json_file(path: &str) -> Result<LdtkJson, serde_json::Error> {
    let file = File::open(path).unwrap_or_else(|_| panic!("failed to load file: {}", path));
    let reader = BufReader::new(file);

    // Deserialize the JSON string into your data structure
    from_reader(reader)
}
