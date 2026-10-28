mod args;
mod error;
mod export;
mod hooks;
mod host;
mod mod_trait;
mod runtime;

#[doc(hidden)]
pub mod __private;

pub mod chat;
pub mod command;
#[cfg(feature = "serde")]
pub mod config;
pub mod game;
pub mod hud;
pub mod log;
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
pub mod testing;
pub mod ui;

pub use args::{ArgCursor, Args, CommandResult, FromArgs, no_reply, reply};
pub use error::Error;
pub use ferrite_macros::FromArgs;
pub use hooks::{
    ChatHandler, CommandHandler, FrameHandler, Hooks, ParsedCommandHandler, ScreenClosedHandler,
    TICKS_PER_SECOND, TickHandler, TimerHandler,
};
pub use mod_trait::Mod;

pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod prelude {
    #[cfg(feature = "serde")]
    pub use crate::config::Config;
    pub use crate::hud::{Color, Corner, Panel};
    pub use crate::ui::Page;
    pub use crate::{
        Args, CommandResult, Error, FromArgs, Hooks, Mod, chat, command, export_mod, game, hud,
        log, no_reply, reply, storage, ui,
    };
}
