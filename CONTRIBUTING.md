# Contributing to Ferrite

Thanks for helping. This page covers building the project, the checks a change must pass and how the code is laid out.

## Setup

```sh
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon --rev cba52f4e5d7f99c592a26823fe8727beafe6ebff cargo-horizon-mod
```

CI uses Rust 1.99.1 (`RUST_TOOLCHAIN` in `.github/workflows/ci.yml`).

## Checks

Run what CI runs before opening a pull request:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p ferrite-api -p hello -p compass -p keystrokes --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
cargo run -p cargo-ferrite -- ferrite doctor
```

and, in each directory under `examples/`, `cargo ferrite test` (Horizon's sandbox).

## Layout

| Path | What lives there |
|---|---|
| `crates/ferrite-api/src/lib.rs` | the module list and the prelude: start reading here |
| `crates/ferrite-api/src/hooks.rs` | what a mod registers in `init` |
| `crates/ferrite-api/src/runtime.rs` | `Adapter`: dispatches the game's calls to the mod's handlers and keeps UI pages in sync |
| `crates/ferrite-api/src/host.rs` | the only code that calls the game; on other targets it calls `testing/fake.rs` |
| `crates/ferrite-api/src/export.rs` | `export_mod!` |
| `crates/ferrite-api/src/testing.rs` | `TestGame` and `World` for mod authors |
| `crates/ferrite-api/src/__private.rs` | the WIT bindings and what the macros need; not public API |
| `crates/ferrite-api/wit/` | Horizon's WIT, vendored, with its license and source revision |
| `crates/ferrite-macros` | `#[derive(FromArgs)]` |
| `crates/cargo-ferrite` | the `cargo ferrite` command and the `new` template |
| `examples/` | mods that double as end-to-end tests |
| `docs/guide.md` | the modding guide |
| `docs/api-gaps.md` | what Horizon does not offer mods yet |

A new game call goes in both halves of `host.rs`'s contract: the wasm side in `host.rs` and the fake in `testing/fake.rs`, so `TestGame` covers it.

## Conventions

- The code carries no comments; names, types and tests explain it. User documentation lives in `docs/guide.md` and must change with the API.
- Every public behaviour has a test: unit tests next to the code, `TestGame` tests in `crates/ferrite-api/tests/`, sandbox scripts in the examples.
- Commits follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat(api): ...`, `fix(cli): ...`), mark breaking changes with `!` and are listed in `CHANGELOG.md`. Each commit builds and passes the checks.
- Mods depend on `ferrite-api` only; never expose the WIT bindings to them.

## Updating Horizon's API

1. Copy the new `horizon.wit` from Horizon's `crates/horizon-mod-sdk/wit/` into `crates/ferrite-api/wit/`.
2. Update the revision in `crates/ferrite-api/wit/UPSTREAM`, `HORIZON_REV` in `crates/cargo-ferrite/src/horizon.rs`, and the install commands in `README.md`, `CONTRIBUTING.md` and `docs/guide.md`. A test fails while `HORIZON_REV` and `UPSTREAM` disagree.
3. Adapt `host.rs` and the fake, then run every check.

## Legal

- Never commit Minecraft assets, decompiled code or files from Horizon's `.local/`.
- Code taken from Horizon keeps its MIT notice.
- Ferrite is unofficial; do not present it as part of Horizon, Mojang or Microsoft.
