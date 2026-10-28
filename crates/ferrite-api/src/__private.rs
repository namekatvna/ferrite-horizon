pub use crate::runtime::Adapter;
pub use crate::server::runtime::ServerAdapter;

use crate::game::{BlockPos, Vec3};
use crate::server::{ArgumentValue, BlockState, GameMode, Player};

pub mod sdk {
    wit_bindgen::generate!({
        path: "wit",
        world: "horizon-ui-mod",
        pub_export_macro: true,
        export_macro_name: "__export_horizon_mod",
        default_bindings_module: "ferrite_api::__private::sdk",
    });

    pub use exports::horizon::mods::events::{Guest, Registration};
    pub use exports::horizon::mods::ui_events::Guest as UiEventsGuest;
    pub use horizon::mods::{chat, command, game, hud, input, log, storage, ui};
}

pub mod server_sdk {
    wit_bindgen::generate!({
        path: "wit",
        world: "horizon-server-mod",
        pub_export_macro: true,
        export_macro_name: "__export_horizon_server_mod",
        default_bindings_module: "ferrite_api::__private::server_sdk",
        with: {
            "horizon:mods/log@0.1.0": crate::__private::sdk::log,
            "horizon:mods/storage@0.1.0": crate::__private::sdk::storage,
        },
    });

    pub use exports::horizon::mods::server_events::{
        Argument, ArgumentKind, ArgumentValue, BlockPos, Guest, Registration, ServerCommand,
    };
    pub use horizon::mods::{players, world};
}

pub fn player(player: server_sdk::players::Player) -> Player {
    Player {
        id: player.id,
        name: player.name,
        position: Vec3::new(player.x, player.y, player.z),
        dimension: player.dimension.into(),
        game_mode: match player.game_mode {
            server_sdk::players::GameMode::Survival => GameMode::Survival,
            server_sdk::players::GameMode::Creative => GameMode::Creative,
            server_sdk::players::GameMode::Adventure => GameMode::Adventure,
            server_sdk::players::GameMode::Spectator => GameMode::Spectator,
        },
        health: player.health,
    }
}

pub fn block_pos(pos: server_sdk::BlockPos) -> BlockPos {
    BlockPos::new(pos.x, pos.y, pos.z)
}

pub fn block_state(state: String) -> BlockState {
    BlockState::new(state)
}

pub fn values(values: Vec<(String, server_sdk::ArgumentValue)>) -> Vec<(String, ArgumentValue)> {
    values
        .into_iter()
        .map(|(name, value)| {
            let value = match value {
                server_sdk::ArgumentValue::Word(s) => ArgumentValue::Word(s),
                server_sdk::ArgumentValue::Text(s) => ArgumentValue::Text(s),
                server_sdk::ArgumentValue::Integer(n) => ArgumentValue::Integer(n),
                server_sdk::ArgumentValue::Number(n) => ArgumentValue::Number(n),
                server_sdk::ArgumentValue::Boolean(b) => ArgumentValue::Boolean(b),
                server_sdk::ArgumentValue::Player(p) => ArgumentValue::Player(player(p)),
                server_sdk::ArgumentValue::BlockPos(p) => ArgumentValue::BlockPos(block_pos(p)),
            };
            (name, value)
        })
        .collect()
}
