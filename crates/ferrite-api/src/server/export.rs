#[macro_export]
macro_rules! export_server_mod {
    ($t:ty) => {
        const _: () = {
            use ::std::cell::RefCell;
            use $crate::__private::{self as private, ServerAdapter, server_sdk as sdk};

            ::std::thread_local! {
                static STATE: RefCell<Option<ServerAdapter<$t>>> = const { RefCell::new(None) };
            }

            fn with(f: impl FnOnce(&mut ServerAdapter<$t>)) {
                STATE.with(|s| {
                    if let Some(m) = s.borrow_mut().as_mut() {
                        f(m)
                    }
                })
            }

            struct Export;

            impl sdk::Guest for Export {
                fn init() -> Result<sdk::Registration, String> {
                    let (m, registration) = ServerAdapter::<$t>::init()?;
                    STATE.with(|s| *s.borrow_mut() = Some(m));
                    Ok(registration)
                }
                fn tick(tick: u64) {
                    with(|m| m.tick(tick))
                }
                fn player_joined(player: sdk::players::Player) {
                    let player = private::player(player);
                    with(|m| m.joined(&player))
                }
                fn player_left(player: sdk::players::Player) {
                    let player = private::player(player);
                    with(|m| m.left(&player))
                }
                fn chat(player: sdk::players::Player, text: String) {
                    let player = private::player(player);
                    with(|m| m.chat(&player, &text))
                }
                fn command(
                    player: sdk::players::Player,
                    name: String,
                    args: String,
                    values: Vec<(String, sdk::ArgumentValue)>,
                ) -> Result<Option<String>, String> {
                    let player = private::player(player);
                    let values = private::values(values);
                    let mut out = Err("The mod is not running".to_string());
                    with(|m| out = m.command(&player, &name, &args, &values));
                    out
                }
                fn block_broken(player: sdk::players::Player, pos: sdk::BlockPos, state: String) {
                    let player = private::player(player);
                    let state = private::block_state(state);
                    with(|m| m.block_broken(&player, private::block_pos(pos), &state))
                }
                fn block_placed(player: sdk::players::Player, pos: sdk::BlockPos, state: String) {
                    let player = private::player(player);
                    let state = private::block_state(state);
                    with(|m| m.block_placed(&player, private::block_pos(pos), &state))
                }
            }

            #[cfg(target_arch = "wasm32")]
            $crate::__private::server_sdk::__export_horizon_server_mod!(Export with_types_in $crate::__private::server_sdk);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = <Export as sdk::Guest>::init;
        };
    };
}
