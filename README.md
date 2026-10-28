# Ferrite

[![CI](https://github.com/namekatvna/ferrite-horizon/actions/workflows/ci.yml/badge.svg)](https://github.com/namekatvna/ferrite-horizon/actions/workflows/ci.yml)

A mod API and toolkit for [Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft Java. Write mods in plain Rust against one stable crate; Ferrite handles the WebAssembly plumbing, the game's limits and the boilerplate.

> Early development (0.0.x): the API may still change between releases.

Ferrite is unofficial and not affiliated with the Horizon project, Mojang or Microsoft.

## A mod in Ferrite

```rust
use ferrite_api::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    visible: bool,
    corner: Corner,
}

#[derive(FromArgs)]
enum Compass {
    #[default]
    Toggle,
    Corner { corner: Corner },
}

struct Hud {
    settings: Config<Settings>,
}

impl Mod for Hud {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .on_tick(Self::draw)
            .parsed_command("compass", Self::compass)
            .settings(|hud, page| {
                page.toggle("Visible", hud.settings.visible, |m, on| m.settings.edit(|s| s.visible = on));
            });
        Ok(Self { settings: Config::load() })
    }
}

impl Hud {
    fn draw(&mut self, _tick: u64) {
        if let Some(player) = game::player().filter(|_| self.settings.visible) {
            Panel::new(self.settings.corner)
                .line(format!("XYZ {}", player.block_pos()))
                .colored(format!("Facing {}", player.facing()), Color::AQUA)
                .show();
        } else {
            hud::clear();
        }
    }

    fn compass(&mut self, command: Compass) -> CommandResult {
        match command {
            Compass::Toggle => self.settings.update(|s| s.visible = !s.visible)?,
            Compass::Corner { corner } => self.settings.update(|s| s.corner = corner)?,
        }
        no_reply()
    }
}

export_mod!(Hud);
```

## What you get

| | |
|---|---|
| **Hooks** | ticks, frames, timers (`every_seconds(5, ...)`), chat, commands; register only what you use |
| **Commands** | `#[derive(FromArgs)]` turns an enum into a command with generated usage and error messages |
| **HUD** | text panels in a corner; unchanged panels are not resent to the game |
| **Settings** | `Config<T>`: JSON settings that survive new and removed fields and broken files |
| **UI** | a settings page in Options > Mods and the mod's own screens, with handlers bound to widgets |
| **Storage** | typed key/value data per mod |
| **Testing** | `TestGame` runs the mod in `cargo test` against a fake game; `cargo ferrite test` in Horizon's sandbox |
| **Tooling** | `cargo ferrite new / build / test / run / check / doctor` |

## Quick start

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon --rev cba52f4e5d7f99c592a26823fe8727beafe6ebff cargo-horizon-mod
cargo install --git https://github.com/namekatvna/ferrite-horizon cargo-ferrite

cargo ferrite new my-mod
cd my-mod
cargo test
cargo ferrite test
```

Copy `target/horizon-mod/my_mod.hmod` into `horizon-mods/` next to the game's `options.txt` and turn it on in Options > Mods.

Read the [modding guide](docs/guide.md) next; [`examples/`](examples) has complete mods.

## Repository

| Path | |
|---|---|
| `crates/ferrite-api` | the API mods depend on |
| `crates/ferrite-macros` | `#[derive(FromArgs)]` |
| `crates/cargo-ferrite` | the `cargo ferrite` command |
| `examples/` | `hello` and `compass` |
| `docs/` | the guide and the list of features Horizon does not offer yet |

## License

[MIT](LICENSE)
