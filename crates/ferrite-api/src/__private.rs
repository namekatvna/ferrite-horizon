pub use crate::runtime::Adapter;

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
    pub use horizon::mods::{chat, command, game, hud, log, storage, ui};
}
