use bevy::prelude::*;
use game::{args::BaseArgsPlugin, core::{CoreSetupConfig, CoreSetupPlugin}};

fn main() {

    let game_config = CoreSetupConfig {
        app_name: "zrl-character_tester".into()
    };

    let core_plugin = CoreSetupPlugin(game_config);

    App::new()
        .add_plugins(core_plugin.get_default_plugin())
        .add_plugins(core_plugin)
        .add_plugins(BaseArgsPlugin)

        .run();
}
