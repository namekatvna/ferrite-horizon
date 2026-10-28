mod args;
mod error;
mod hooks;
mod mod_trait;

#[doc(hidden)]
pub mod __private;

pub mod chat;
pub mod log;

pub use args::{Args, CommandResult};
pub use error::Error;
pub use hooks::{ChatHandler, CommandHandler, FrameHandler, Hooks, TickHandler};
pub use mod_trait::Mod;

pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod prelude {
    pub use crate::{Args, CommandResult, Error, Hooks, Mod, chat, export_mod, log};
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

            impl sdk::__Guest for Export {
                fn init() -> Result<sdk::Registration, String> {
                    let (m, registration) = <Adapter<$t> as sdk::Mod>::init()?;
                    STATE.with(|s| *s.borrow_mut() = Some(m));
                    Ok(registration)
                }
                fn tick(tick: u64) {
                    with(|m| sdk::Mod::tick(m, tick))
                }
                fn frame(seconds: f32) {
                    with(|m| sdk::Mod::frame(m, seconds))
                }
                fn command(name: String, args: String) -> Result<Option<String>, String> {
                    let mut out = Err("The mod is not running".to_string());
                    with(|m| out = sdk::Mod::command(m, &name, &args));
                    out
                }
                fn chat(text: String) {
                    with(|m| sdk::Mod::chat(m, &text))
                }
            }

            impl sdk::__UiEventsGuest for Export {
                fn select(_id: u32, _row: u32) {}
                fn closed() {}
                fn settings_opened() {}
            }

            #[cfg(target_arch = "wasm32")]
            $crate::__private::sdk::__export_horizon_mod!(Export with_types_in $crate::__private::sdk);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = (
                <Export as sdk::__Guest>::init,
                <Export as sdk::__UiEventsGuest>::select,
            );
        };
    };
}
