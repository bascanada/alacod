use std::fs::File;
use std::io::BufReader;

use bevy_ecs_ldtk::ldtk::LdtkJson;
use serde_json::from_reader;

pub fn load_ldtk_json_file(path: &str) -> Result<LdtkJson, serde_json::Error> {
    let file = File::open(path).expect(format!("failed to load file: {}", path).as_str());
    let reader = BufReader::new(file);

    // Deserialize the JSON string into your data structure
    return from_reader(reader);
}
