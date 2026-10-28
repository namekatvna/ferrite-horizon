# Ferrite modding guide

Ferrite mods are Rust crates compiled to WebAssembly and run by Horizon in a sandbox. You write against `ferrite-api` only.

## Setup

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod
```

`Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
ferrite-api = { git = "https://github.com/namekatvna/ferrite-horizon" }
serde = { version = "1", features = ["derive"] }
```

`horizon-mod.toml` lists what the mod may do. Horizon refuses a mod that uses an API whose capability is missing here.

```toml
id = "my-mod"
name = "My Mod"
version = "0.1.0"
description = "What it does."
authors = ["you"]
api = "0.1"
side = "client"
capabilities = ["chat"]
```

| Module | Capability |
|---|---|
| `log` | none |
| `chat` | `chat` |
| `game` | `game-read` |
| `hud` | `hud` |
| `storage`, `Config` | `storage` |
| `command` | `command` |

## A complete mod

```rust
use ferrite_api::prelude::*;

struct Greeter {
    greeted: u32,
}

impl Mod for Greeter {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .command("greet", Self::greet)
            .on_tick(Self::tick);
        Ok(Self { greeted: 0 })
    }
}

impl Greeter {
    fn greet(&mut self, args: Args) -> CommandResult {
        let name = args.word(0).unwrap_or("world");
        self.greeted += 1;
        chat::show(format!("Hello, {name}!"));
        Ok(Some(format!("Greeted {} times", self.greeted)))
    }

    fn tick(&mut self, _tick: u64) {}
}

export_mod!(Greeter);
```

Build, test and play:

```sh
cargo horizon-mod build
cargo horizon-mod test
cargo horizon-mod run --game <path to horizon-client>
```

Or copy `target/horizon-mod/<id>.hmod` into `horizon-mods/` next to the game's `options.txt` and turn it on in Options > Mods. Every mod is off until turned on.

## Hooks

`init` runs once when the mod loads. Register everything there; only what you register is ever called, so unused hooks cost nothing.

| Registration | Handler signature | Called |
|---|---|---|
| `command("name", f)` | `fn(&mut self, Args) -> CommandResult` | `/name ...` typed by the player |
| `on_tick(f)` | `fn(&mut self, u64)` | 20 times a second |
| `every(ticks, f)` | `fn(&mut self)` | once every `ticks` ticks |
| `every_seconds(n, f)` | `fn(&mut self)` | once every `n` seconds |
| `on_frame(f)` | `fn(&mut self, f32)` | every frame, with its duration in seconds |
| `on_chat(f)` | `fn(&mut self, &str)` | every chat line the player receives |

Handlers are methods (`Self::greet`) or closures without captures (`|m, _| m.ticks += 1`). Registering a command name twice keeps the last handler. A leading `/` in the name is ignored. Timers count ticks since the mod loaded and fire after each full interval; any number can be registered.

Returning `Err` from `init` keeps the mod off and shows the message to the player.

## Commands

Describe a command as a type and let Ferrite parse it:

```rust
#[derive(FromArgs)]
enum Waypoint {
    #[default]
    List,
    Add { name: String, x: i32, y: i32, z: i32 },
    Remove { name: String },
    Note { name: String, #[rest] text: String },
}

fn waypoint(&mut self, args: Args) -> CommandResult {
    match args.parse()? {
        Waypoint::List => Ok(Some(self.list())),
        Waypoint::Add { name, x, y, z } => self.add(name, x, y, z),
        Waypoint::Remove { name } => self.remove(&name),
        Waypoint::Note { name, text } => self.note(&name, text),
    }
}
```

| You write | Player types |
|---|---|
| variant `SetColor` | `set-color` (any case) |
| field `x: i32` | one word parsed as `i32` |
| field `color: Option<u32>` | an optional word; optional fields come last |
| `#[rest] text: String` | everything left, spaces kept; last field only |
| `#[default]` on a unit variant | used when nothing is typed |

Structs parse positionally: `#[derive(FromArgs)] struct Teleport { x: f64, y: f64, z: f64 }`. Any type implementing `FromStr` with a printable error works as a field, including `Corner`.

Errors are generated from the type and point at the problem:

```
Missing <z>. Usage: /wp add <name> <x> <y> <z>
Invalid <y> `two`: invalid digit found in string
Unknown option `fly`. Usage: /wp list | add <name> <x> <y> <z> | remove <name> | note <name> <text...>
Unexpected `now`. Usage: /wp remove <name>
```

`Waypoint::USAGE` holds the usage line for help messages. Mistakes in the type itself, such as `#[rest]` on a middle field, are compile errors.

## Arguments by hand

For simple commands, read `Args` directly:

```rust
fn teleport(&mut self, args: Args) -> CommandResult {
    let x: i32 = args.get(0)?;
    let y: i32 = args.get_or(1, 64)?;
    let note = args.rest(2);
    Ok(Some(format!("Going to {x} {y} ({note})")))
}
```

| Method | Returns |
|---|---|
| `as_str()` | the whole line after the command name, trimmed |
| `is_empty()`, `len()` | whether there are words, how many |
| `word(i)` | `Option<&str>` |
| `words()` | iterator over words |
| `rest(i)` | everything from word `i`, inner spacing kept |
| `get::<T>(i)` | parsed word, `Missing argument #n` or `Invalid argument #n \`w\`: reason` |
| `get_or(i, default)` | like `get`, but `default` when the word is missing |
| `parse::<T>()` | the whole line as a `FromArgs` type |
| `command()` | the command's name |

`CommandResult` is `Result<Option<String>, Error>`: `Ok(Some(text))` is feedback, `Ok(None)` is silent, `Err(e)` is shown in red. `Error` converts from anything printable, so `?` works on parse errors, storage errors and strings: `Err("no such home".into())`.

## Game state (`game`)

```rust
if let Some(player) = game::player() {
    let pos = player.block_pos();
    let facing = player.facing();
    let looking = player.look_direction();
    let nether = player.dimension == game::Dimension::Nether;
}
if let Some(clock) = game::clock() {
    let (hours, minutes) = clock.hours_minutes();
    let night = clock.is_night();
}
if let Some(hit) = game::raycast(64.0) {
    let block = hit.block;
}
```

`player()` and `clock()` are `None` on the title screen. Values are a snapshot of the current tick.

| Type | Highlights |
|---|---|
| `Player` | `position: Vec3`, `yaw`, `pitch`, `health`, `food`, `dimension`, `block_pos()`, `facing()`, `look_direction()` |
| `Clock` | `game_time`, `day_time`, `day()`, `time_of_day()`, `hours_minutes()`, `is_day()`, `is_night()` |
| `Hit` | `block_pos`, `point`, `block` id |
| `Vec3` | `+`, `-`, `* f64`, `length()`, `distance()`, `horizontal_distance()`, `block_pos()` |
| `BlockPos` | `center()`, `offset()`, prints as `x y z` |
| `Dimension` | `Overworld`, `Nether`, `End`, `Other(id)`, `id()`, `name()` |
| `Direction` | `South`, `West`, `North`, `East` from yaw, as vanilla |

## HUD (`hud`)

One panel per mod, up to 16 lines of 256 characters, drawn by the game until changed.

```rust
Panel::new(Corner::TopRight)
    .line("Plain white")
    .colored("Warning", Color::GOLD)
    .colored("Custom", Color::rgb(120, 200, 255))
    .show();

hud::clear();
```

`show()` skips the call to the game when the panel is identical to the one on screen, so building and showing a panel every tick is cheap. Extra lines and characters are dropped instead of failing. Build panels in a loop with `push`:

```rust
let mut panel = Panel::new(Corner::BottomLeft);
for (name, pos) in &self.homes {
    panel.push(format!("{name}: {pos}"));
}
panel.show();
```

`Color` has the 16 vanilla text colors (`Color::RED`, `Color::AQUA`, ...), `Color::rgb(r, g, b)` and `Color::hex(0xRRGGBB)`. `Corner` parses from `top-left`, `top_right`, `Bottom Left` and so on.

## Settings (`Config`)

For anything the player may want to edit. Stored as pretty JSON in the mod's storage.

```rust
#[derive(Serialize, Deserialize)]
struct Settings {
    radius: u32,
    corner: Corner,
}

impl Default for Settings {
    fn default() -> Self {
        Self { radius: 16, corner: Corner::TopLeft }
    }
}

let mut settings: Config<Settings> = Config::load();
let radius = settings.radius;
settings.update(|s| s.radius = 32)?;
settings.reset()?;
```

| Behaviour | Result |
|---|---|
| No file yet | defaults are used and written |
| A field was added in a new mod version | it gets its default, other values stay, the file is updated |
| A field was removed | it is dropped from the file |
| The file is broken or has a wrong type | defaults are used, a warning is logged, the file is left for the player to fix |

`Config::load()` uses `config.json`; `Config::load_named("waypoints")` uses `waypoints.json`. `Config` derefs to your struct. `update` and `reset` save immediately; `reload()` reads the file again.

## Data (`storage`)

For data the mod keeps for itself. Typed values use a compact binary format.

```rust
storage::save("homes", &self.homes)?;
let homes: Vec<Home> = storage::load_or_default("homes")?;
let maybe: Option<Home> = storage::load("last")?;
storage::remove("last")?;
```

Raw bytes: `storage::get`, `storage::set`, `storage::contains`. Keys are 1-64 characters of `a-z 0-9 _ . -`; others fail with `StorageError::InvalidKey` before reaching the game. Storage has a per-mod quota.

The binary format is not self-describing: changing a saved struct makes old data fail to load with `StorageError::Decode`. Use `Config` for data whose shape will change, or version your keys (`homes.v2`).

## Commands as the player (`command`)

```rust
command::run("/time set day");
command::run_quiet(["/setblock 0 64 0 stone", "/setblock 1 64 0 stone"]);
```

`run` shows feedback in chat as if typed; at most 8 per hook call. `run_quiet` shows nothing and takes any number of lines, sent in batches of 256. Both run after the hook returns, with the player's permissions. The leading `/` is optional.

## Chat and log

```rust
chat::show("Only you see this");
log::info("goes to the game log as [mod <id>] ...");
log::warn("...");
```

## Testing

Unit-test plain Rust logic with `cargo test`. Test the mod as the game runs it with `cargo horizon-mod test`, which plays `tests/*.toml` scripts in Horizon's sandbox:

```toml
player = { x = 0.5, y = 64.0, z = 0.5, yaw = 180.0 }
clock = { day-time = 6000 }

[[step]]
expect.commands = ["compass"]

[[step]]
ticks = 1
expect.panel = ["XYZ 0 64 0", "Facing North"]
expect.corner = "top-left"

[[step]]
command = "compass corner nowhere"
expect.error = "Invalid argument #2 `nowhere`: expected top-left, top-right, bottom-left or bottom-right"

[[step]]
reload = true
ticks = 1
expect.panel = ["XYZ 0 64 0", "Facing North"]
```

See `examples/compass` for a mod using every module, with scripts covering its HUD and saved settings.

## Limits

| Limit | Value |
|---|---|
| Memory per mod | 64 MiB |
| Mod file | 32 MiB |
| Tick budget | about 1 ms |
| Frame budget | about 0.1 ms |
| `command::run` per hook | 8 |
| HUD | 16 lines, 256 characters each |

A mod over budget is flagged; a runaway mod is stopped without affecting others.
