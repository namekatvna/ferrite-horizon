mod args;
mod error;
mod hooks;
mod host;
mod mod_trait;

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

#[macro_export]
macro_rules! export_mod {
    ($t:ty) => {
        const _: () = {
            use ::std::cell::RefCell;
            use $crate::__private::{Adapter, sdk};

            ::std::thread_local! {
                static STATE: RefCell<Option<Adapter<$t>>> = const { RefCell::new(None) };
            }

            fn with(f: impl FnOnce(&mut Adapter<$t>)) {
                STATE.with(|s| {
                    if let Some(m) = s.borrow_mut().as_mut() {
                        f(m)
                    }
                })
            }

            struct Export;

            impl sdk::Guest for Export {
                fn init() -> Result<sdk::Registration, String> {
                    let (m, registration) = Adapter::<$t>::init()?;
                    STATE.with(|s| *s.borrow_mut() = Some(m));
                    Ok(registration)
                }
                fn tick(tick: u64) {
                    with(|m| m.tick(tick))
                }
                fn frame(seconds: f32) {
                    with(|m| m.frame(seconds))
                }
                fn command(name: String, args: String) -> Result<Option<String>, String> {
                    let mut out = Err("The mod is not running".to_string());
                    with(|m| out = m.command(&name, &args));
                    out
                }
                fn chat(text: String) {
                    with(|m| m.chat(&text))
                }
            }

            impl sdk::UiEventsGuest for Export {
                fn select(id: u32, row: u32) {
                    with(|m| m.ui_select(id, row))
                }
                fn closed() {
                    with(|m| m.ui_closed())
                }
                fn settings_opened() {
                    with(|m| m.ui_settings_opened())
                }
            }

            #[cfg(target_arch = "wasm32")]
            $crate::__private::sdk::__export_horizon_mod!(Export with_types_in $crate::__private::sdk);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = (
                <Export as sdk::Guest>::init,
                <Export as sdk::UiEventsGuest>::select,
            );
        };
    };
}
