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
