# Examples

Demo mods built with `ferrite-api`. Each one is its own crate (`cdylib`, target `wasm32-unknown-unknown`) with native tests in `src/lib.rs` and Horizon sandbox scripts in `tests/`.

| Mod | Shows |
|---|---|
| `hello` | commands, a tick handler, chat |
| `compass` | HUD panel, game state, `Config`, `#[derive(FromArgs)]` commands, a settings page from `#[derive(Settings)]` |
| `keystrokes` | mod keys, holding a key, the mouse wheel, vanilla controls and clicks per second |
