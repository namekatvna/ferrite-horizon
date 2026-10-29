# Changelog

All notable changes to Ferrite are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/); until 1.0, a minor version may break the API.

## Unreleased

### Added

- `ferrite-api`: the `Mod` trait and `export_mod!`, with hooks for ticks, frames, timers (`every`, `every_seconds`), chat and commands.
- Commands described as types with `#[derive(FromArgs)]`, generated usage and error messages, and `parsed_command` to receive the parsed value.
- `reply` and `no_reply` for command results.
- `game` (player, clock, raycast), `hud` panels, `chat`, `command` (vanilla commands as the player), `log`, `storage`.
- `Config<T>`: JSON settings that survive added, removed and broken fields; `update`, `edit`, `reset`, `reload`.
- `ui`: a settings page in Options > Mods and the mod's own screens, with handlers bound to widgets and pages rebuilt after events.
- `#[derive(Settings)]` and `Page::config` / `Page::fields`: a settings page built from a config struct, with `#[setting(label, options, range, step, skip)]`.
- `#[derive(Choice)]` and `Page::choice` for enums picked on a page; `Corner` implements `Choice`.
- Page handlers accept any closure, including ones that capture values.
- `input`: mod keys rebindable in Options > Mods (`key`, `on_key`, `on_key_press`, `on_key_release`, `input::is_down`), the mouse wheel (`on_scroll`, `input::capture_scroll`), claimed items (`on_item_use`, `on_item_release`), the held item, vanilla controls and clicks per second. Short key names (`"c"`, `"left.shift"`).
- Example mod `keystrokes`.
- Server mods: `server::ServerMod`, `ServerHooks` (commands, ticks, timers, joins, leaves, chat, broken and placed blocks), `export_server_mod!`, `server::players`, `server::world` with batched `set_blocks`, `BlockState`, `Player`, and `cargo ferrite new --server`.
- `#[derive(FromArgs)]` structs become typed server arguments (numbers with their type's range, `bool`, words, `BlockState`, `BlockPos` with `~`, `Player`); `BlockPos` fields read three numbers in client commands.
- `BlockPos::cuboid`, `volume` and `neighbors`.
- `testing::TestServer` and `ServerWorld`.
- Example mod `builder`.
- `ServerHooks::simple_command` for commands without arguments, `server::PlayerMap` for per-player state, `world::fill` for boxes of one state and `world::blocks` to read a box.
- `testing::TestGame`: run a mod in `cargo test` against a fake game.
- `cargo ferrite new | build | test | run | check | doctor`.
- Example mods `hello` and `compass`, the modding guide and CI.

### Changed

- The release profile of the workspace and of new mods adds `codegen-units = 1` and `strip = true`: mods are 14–17% smaller.
- **Breaking:** registering `on_tick`, `on_frame`, `on_chat` or `on_screen_closed` again adds a handler instead of replacing the previous one.
- Server block edits use 2–6 times less of Horizon's time budget: block states from the game and full ids are no longer copied, and `set_blocks` passes states to the game without copying them. `examples/builder` now edits up to 32 768 blocks at once instead of 4096.
- **Breaking:** `world::set_block` and `world::set_blocks` take states as `world::AsBlockState` (`BlockState`, `&BlockState`, `&str`, `String`) instead of `Into<BlockState>`.
