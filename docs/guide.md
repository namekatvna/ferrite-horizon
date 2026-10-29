# Ferrite modding guide

Ferrite mods are Rust crates compiled to WebAssembly and run by Horizon in a sandbox. You write against `ferrite-api` only.

A **client mod** runs in the player's game: HUD, screens, keys, commands for that player. A **server mod** runs where the world is simulated: every player's joins, commands and broken blocks, and the world's blocks. Most of this guide is about client mods; see [Server mods](#server-mods) for the other side.

## Setup

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon --rev cba52f4e5d7f99c592a26823fe8727beafe6ebff cargo-horizon-mod
cargo install --git https://github.com/namekatvna/ferrite-horizon cargo-ferrite
cargo ferrite doctor
```

Create a mod:

```sh
cargo ferrite new my-mod
cd my-mod
cargo ferrite test
```

This makes a crate with a `/hello` command, its manifest and a test script. `--server` makes a server mod instead. `--name "My Mod"` sets the display name. `--ferrite-path <dir>` uses a local Ferrite checkout instead of git. Inside a Cargo workspace the mod reuses the workspace's `ferrite-api` dependency; add it to the workspace members.

`Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
ferrite-api = { git = "https://github.com/namekatvna/ferrite-horizon" }
serde = { version = "1", features = ["derive"] }
```

`horizon-mod.toml` lists what the mod may do. Horizon refuses a mod that uses an API whose capability is missing here. The id uses lowercase letters, digits and `_`.

```toml
id = "my_mod"
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
| `ui`, `hooks.settings` | `ui`, also listed in `experimental = ["ui"]` |
| `input`, `hooks.key`, `on_scroll`, `on_item_use` | `input`, also listed in `experimental = ["input"]` |

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
        reply(format!("Greeted {} times", self.greeted))
    }

    fn tick(&mut self, _tick: u64) {}
}

export_mod!(Greeter);
```

Build, test and play:

```sh
cargo ferrite build
cargo ferrite test
cargo ferrite run --game <path to horizon-client>
```

These run `cargo horizon-mod` with the same arguments.

Or copy `target/horizon-mod/<id>.hmod` into `horizon-mods/` next to the game's `options.txt` and turn it on in Options > Mods. Every mod is off until turned on.

## Hooks

`init` runs once when the mod loads. Register everything there; only what you register is ever called, so unused hooks cost nothing.

| Registration | Handler signature | Called |
|---|---|---|
| `command("name", f)` | `fn(&mut self, Args) -> CommandResult` | `/name ...` typed by the player |
| `parsed_command("name", f)` | `fn(&mut self, T) -> CommandResult`, `T: FromArgs` | the same, with the line already parsed |
| `on_tick(f)` | `fn(&mut self, u64)` | 20 times a second |
| `every(ticks, f)` | `fn(&mut self)` | once every `ticks` ticks |
| `every_seconds(n, f)` | `fn(&mut self)` | once every `n` seconds |
| `on_frame(f)` | `fn(&mut self, f32)` | every frame, with its duration in seconds |
| `on_chat(f)` | `fn(&mut self, &str)` | every chat line the player receives |
| `settings(f)` | `fn(&self, &mut Page<Self>)` | builds the mod's page in Options > Mods |
| `on_screen_closed(f)` | `fn(&mut self)` | the player left the mod's screen |

Handlers are methods (`Self::greet`) or closures without captures (`|m, _| m.ticks += 1`). `on_tick`, `on_frame`, `on_chat`, `on_screen_closed` and timers can be registered any number of times; every handler runs, in the order registered, so separate features can keep separate handlers. Registering a command name twice keeps the last handler. A leading `/` in the name is ignored. Timers count ticks since the mod loaded and fire after each full interval.

`game::player()` and `game::clock()` ask the game once per hook call; calling them again in the same handler is free.

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

hooks.parsed_command("wp", Self::waypoint);

fn waypoint(&mut self, command: Waypoint) -> CommandResult {
    match command {
        Waypoint::List => reply(self.list()),
        Waypoint::Add { name, x, y, z } => self.add(name, x, y, z),
        Waypoint::Remove { name } => self.remove(&name),
        Waypoint::Note { name, text } => self.note(&name, text),
    }
}
```

`parsed_command` parses the line before calling the handler and answers the player with the error below when it does not fit. With `command`, call `args.parse::<Waypoint>()?` yourself.

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
    reply(format!("Going to {x} {y} ({note})"))
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

`CommandResult` is `Result<Option<String>, Error>`: `reply(text)` (same as `Ok(Some(text))`) is feedback, `no_reply()` (`Ok(None)`) is silent, `Err(e)` is shown in red. `Error` converts from anything printable, so `?` works on parse errors, storage errors and strings: `Err("no such home".into())`.

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

`Config::load()` uses `config.json`; `Config::load_named("waypoints")` uses `waypoints.json`. `Config` derefs to your struct. `update` and `reset` save immediately and return the storage error, for use with `?` in commands. `edit` saves too but writes a failure to the log instead, for handlers that cannot return an error (buttons, ticks). `reload()` reads the file again.

## Settings page and screens (`ui`)

Experimental: needs `capabilities = ["ui"]` and `experimental = ["ui"]` in `horizon-mod.toml`.

A page is built by a function of the mod's state. Each interactive widget carries its handler, so there are no widget ids to track. After a handler runs, Ferrite builds the pages again and sends only what changed.

```rust
impl Mod for Compass {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks.settings(Self::settings_page).command("waypoints", Self::waypoints);
        Ok(Self { settings: Config::load(), waypoints: Vec::new() })
    }
}

impl Compass {
    fn settings_page(&self, page: &mut Page<Self>) {
        page.toggle("Visible", self.settings.visible, |m, on| m.settings.edit(|s| s.visible = on))
            .choice("Corner", &self.settings.corner, |m, c| m.settings.edit(|s| s.corner = c))
            .button("Reset", |m| m.settings.edit(|s| *s = Settings::default()));
    }

    fn waypoints(&mut self, _args: Args) -> CommandResult {
        ui::open(Self::waypoint_screen);
        no_reply()
    }

    fn waypoint_screen(&self, page: &mut Page<Self>) {
        page.set_title("Waypoints")
            .list(&self.waypoints, None, |m, name| m.travel(name))
            .button("Close", |_| ui::close());
    }
}
```

| Widget | Shows | Handler gets |
|---|---|---|
| `text(s)`, `colored(s, color)` | a line of text | nothing |
| `item(id, name)` | an item icon and a name | nothing |
| `button(label, f)` | a button | `fn(&mut self)` |
| `toggle(label, value, f)` | `label: ON/OFF` | the new value |
| `cycle(label, options, current, f)` | `label: current` | the next option |
| `choice(label, current, f)` | `label: Current` for a `Choice` type | the next choice |
| `list(options, selected, f)` | rows to pick from | the picked option |
| `config(&self.settings, \|m\| &mut m.settings)` | one widget per field of a `#[derive(Settings)]` config | saves the change |
| `fields(&self.options, \|m\| &mut m.options)` | the same for a struct kept only in memory | changes it |

Handlers are closures or methods; they may capture values.

The settings page appears once the mod loads and is rebuilt when the player opens it. `ui::open(f)` shows a screen over the game (a second call replaces it), `ui::close()` closes it. Pages are rebuilt automatically after every click, command and chat line; only a change made in a tick, frame or timer handler needs `ui::refresh()`. Pages hold at most 64 widgets; texts are cut to 256 characters, lists to 512 rows. `set_title` only applies to screens: the settings page is titled with the mod's name.

### A settings page from the config (`#[derive(Settings)]`)

Instead of a widget per field, derive `Settings` on the config struct and show it with one call:

```rust
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Choice)]
enum Speed {
    Slow,
    VeryFast,
    #[choice(label = "Ludicrous!")]
    Ludicrous,
}

#[derive(Serialize, Deserialize, Settings)]
struct Settings {
    show_coords: bool,
    corner: Corner,
    speed: Speed,
    #[setting(range = 4..=64, step = 4)]
    radius: u32,
    #[setting(options = ["red", "green", "blue"], label = "Marker color")]
    color: String,
    #[setting(skip)]
    homes: Vec<Home>,
}

fn settings_page(&self, page: &mut Page<Self>) {
    page.config(&self.settings, |m| &mut m.settings)
        .button("Reset", |m| m.settings.edit(|s| *s = Settings::default()));
}
```

This shows `Show coords: ON`, `Corner: Top left`, `Speed: Slow`, `Radius: 16` and `Marker color: red`. A click moves to the next value, saves the config and updates the page; after the last value it starts over.

| Field | Widget |
|---|---|
| `bool` | on/off button |
| a `Choice` type: `Corner`, or an enum with `#[derive(Choice)]` | the enum's values, labelled `TopLeft` → `Top left` |
| `#[setting(options = [...])]` | the listed values; string literals become the field's type |
| `#[setting(range = a..=b, step = n)]` | the numbers in the range, `step` defaults to 1 |
| `#[setting(skip)]` | nothing |

`#[setting(label = "...")]` replaces the label made from the field name (`show_coords` → `Show coords`), and `#[choice(label = "...")]` the one made from a variant name. `Choice` needs `Clone` and `PartialEq`. A field of another type without `options`, `range` or `skip` is a compile error that points at the field and says which of these to add.

## Keys, the mouse wheel and items (`input`)

Experimental: needs `capabilities = ["input"]` and `experimental = ["input"]` in `horizon-mod.toml`.

```rust
hooks
    .on_key("menu", "Open waypoints", "m", Self::open_menu)
    .key("zoom", "Zoom (hold)", "c")
    .on_key_press("zoom", |_| input::capture_scroll(true))
    .on_key_release("zoom", |_| input::capture_scroll(false))
    .on_scroll(|m, steps| m.zoom += steps)
    .on_item_use("spyglass", |m, using| m.aim(using.item.slot))
    .on_item_release("spyglass", |m, using| m.fire(using.ticks));

fn tick(&mut self, _tick: u64) {
    if input::is_down("zoom") { ... }
}
```

| Registration | Handler | Called |
|---|---|---|
| `key(name, label, default)` | none | declares a key; read it with `input::is_down(name)` |
| `on_key(name, label, default, f)` | `fn(&mut self)` | declares a key and runs `f` when it goes down |
| `on_key_press(name, f)`, `on_key_release(name, f)` | `fn(&mut self)` | the declared key goes down, comes up |
| `on_scroll(f)` | `fn(&mut self, f32)` | wheel steps, positive away from the player, while the mod captures the wheel |
| `on_item_use(item, f)`, `on_item_release(item, f)` | `fn(&mut self, &ItemUse)` | the use button goes down or up with that item in the main hand |

Each key appears on the mod's page in Options > Mods with its `label`; the player can bind another key there, and that one wins. `name` is how the mod refers to the key. `default` is a short key name: `"c"`, `"f6"`, `"left.shift"`, `"space"` mean `key.keyboard.<name>`; a name starting with `key.` is used as is (`"key.mouse.middle"`). A mod has at most 16 keys and claims at most 64 items; items without a namespace are `minecraft:` items.

Claiming an item takes its use away from the game: no placing, eating or swinging while the mod handles it. `ItemUse` has the `item` (`id`, `count`, hotbar `slot`) and `ticks`, how long the button was held (0 when it goes down). `input::capture_scroll(true)` takes the mouse wheel from the hotbar until `capture_scroll(false)` or the mod stops.

| Query | Returns |
|---|---|
| `input::is_down(name)` | whether the mod's key is held now |
| `input::held_item()` | `Option<HeldItem>` in the main hand |
| `input::control(Control::Jump)` | `Option<ControlState>`: the bound key's name (`"Space"`) and whether it is down, for `Forward`, `Left`, `Back`, `Right`, `Jump`, `Sneak`, `Sprint`, `Attack`, `Use` |
| `input::clicks_per_second()` | `Clicks { attack, use_item }` over the last second |

Events arrive once a tick (and once a frame for mods with `on_frame`), before the tick handlers, so `is_down` is current in `on_tick`. A screen opening releases every held key. Mistakes in `init` — a key declared twice, a handler for an undeclared key, an unknown key name, too many keys — stop the mod with a message naming the problem.

In tests, `press_key(name)`, `release_key(name)`, `tap_key(name)`, `scroll(steps)`, `start_using(item)` and `stop_using()` run the handlers at once; `World` has `held_item`, `controls_down` and `clicks`; `keys()`, `claimed_items()` and `is_scroll_captured()` show what the mod registered. Horizon's sandbox has no key bindings, so `input::control` returns `None` in `cargo ferrite test`.

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

The `command` module runs the game's own commands, as if the player typed them. It is not for the mod's commands: those are registered with `hooks.command` and `hooks.parsed_command` (see Commands).

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

## Server mods

Experimental in Horizon: the manifest says `api = "0.3"` and `side = "server"`.

```toml
id = "builder"
name = "Builder"
version = "0.1.0"
api = "0.3"
side = "server"
capabilities = ["players", "world"]
```

> **Single player:** the world runs on a built-in server, and that server starts its mods once, when the world loads. A server mod turned on in Options > Mods while you are in a world starts only after **Save and Quit to Title** and loading the world again; until then the game answers its commands with `Unknown or incomplete command`. Client mods start at once. If the mod still does nothing, Options > Mods shows `(Stopped: ...)` with the reason.

```rust
use ferrite_api::server::prelude::*;

#[derive(FromArgs)]
struct Pillar {
    height: u8,
    block: Option<BlockState>,
}

#[derive(Default)]
struct Beacons;

impl ServerMod for Beacons {
    fn init(hooks: &mut ServerHooks<Self>) -> Result<Self, Error> {
        hooks
            .parsed_command("pillar", Self::pillar)
            .on_join(|_, player| { player.tell(format!("Welcome, {}!", player.name)); })
            .on_block_break(|_, player, pos, state| {
                if state.is("diamond_ore") {
                    players::broadcast(format!("{} found diamonds at {pos}", player.name));
                }
            });
        Ok(Self)
    }
}

impl Beacons {
    fn pillar(&mut self, player: &Player, pillar: Pillar) -> CommandResult {
        let base = player.block_pos().offset(2, 0, 0);
        let block = pillar.block.unwrap_or_else(|| "glowstone".into());
        world::set_blocks(
            &player.dimension,
            (0..i32::from(pillar.height)).map(|dy| (base.offset(0, dy, 0), block.clone())),
        )?;
        no_reply()
    }
}

export_server_mod!(Beacons);
```

| Registration | Handler | Called |
|---|---|---|
| `command(name, f)` | `fn(&mut self, &Player, Args) -> CommandResult` | `/name ...` typed by a player |
| `parsed_command(name, f)` | `fn(&mut self, &Player, T) -> CommandResult` | the same, with a `FromArgs` value |
| `simple_command(name, f)` | `fn(&mut self, &Player) -> CommandResult` | `/name` with nothing after it; the chat box shows no arguments and the server refuses extra text |
| `on_tick(f)`, `every(n, f)`, `every_seconds(n, f)` | as for client mods | server ticks |
| `on_join(f)`, `on_leave(f)` | `fn(&mut self, &Player)` | a player joins or leaves |
| `on_chat(f)` | `fn(&mut self, &Player, &str)` | a chat message a player sends |
| `on_block_break(f)`, `on_block_place(f)` | `fn(&mut self, &Player, BlockPos, &BlockState)` | a player broke a block (the state is what was there) or placed one |
| `queue_edits(blocks_per_tick)` | | turns on queued edits (below) |
| `on_edit_done(f)` | `fn(&mut self, &mut EditDone)` | a queued edit finished, was cancelled or failed; also turns on queued edits |

### Commands with typed arguments

A `#[derive(FromArgs)]` struct whose fields are numbers, `bool`, `String`, `BlockState`, `BlockPos` or `Player` (each also as `Option<_>`) becomes typed arguments: the player's chat box completes and colours them, and the server checks them before the mod runs, with vanilla's errors.

| Field | Player types | Notes |
|---|---|---|
| `i8`, `i16`, `i32`, `u8`, `u16`, `u32` | a whole number | the type's range is enforced: `u8` refuses 300 |
| `f32`, `f64` | a number | |
| `bool` | `true` or `false` | |
| `String`, `BlockState` | one word: letters, digits, `_ - . +` | `stone` means `minecraft:stone`; mark the last field `#[rest]` for free text |
| `BlockPos` | `x y z`, each also `~` or `~n` | relative to the player |
| `Player` | a name, `@s` or `@p` | |

Enums with subcommands, and structs with other field types, are parsed from the text of the line instead, with the same messages as client commands.

### Players and the world

| Call | Does |
|---|---|
| `players::online()` | the players on the server, as of this hook |
| `players::find(name)` | an online player by name, any case |
| `player.tell(text)`, `players::tell(&player, text)` | a message to one player; false when they left |
| `players::broadcast(text)` | a message to everyone and the server log |
| `world::block(&dimension, pos)` | `Option<BlockState>`: none where no chunk is loaded |
| `world::blocks(&dimension, a, b)` | every loaded `(BlockPos, BlockState)` of the box from `a` to `b` |
| `world::biome(&dimension, pos)` | the biome id |
| `world::set_block(&dimension, pos, state)`, `world::set_blocks(&dimension, changes)` | changes blocks as `/setblock` does, after the hook returns |
| `world::fill(&dimension, a, b, state)` | sets the whole box from `a` to `b` to one state |

`world::set_blocks` takes any iterator of `(BlockPos, state)` and sends it in batches of 4096, the most Horizon takes in one call; it returns how many it sent. A state is a `BlockState`, a `&BlockState`, a `&str` or a `String`. Changes are applied after the hook returns, so `world::block` still sees the old state during the same command. `BlockPos::cuboid(a, b)` walks every position of a box and `BlockPos::volume(a, b)` counts them. Players get at most 8 messages per hook call.

`PlayerMap<T>` keeps a value per player, keyed by their id: `get`, `get_mut`, `get_or_default`, `insert`, `remove`, `retain_online`. Remove a player's entry in `on_leave` when it should not outlive the session.

#### Staying within Horizon's time budget

Horizon stops a server mod that runs more than about 100 million WebAssembly instructions in one command or event (50 million in a tick). Most of a block edit's cost is the text of block states, so:

- `world::fill` is the cheapest way to set a box: about 250 instructions a block, around 300 000 blocks per command.
- Pass `&BlockState` or a full id (`"minecraft:stone"`) to `set_blocks`: neither copies the text. A short id (`"stone"`) builds a new string for every block.
- Reading costs about 600 instructions a block. A command that reads, compares and writes each block handles about 45 000 blocks in one call.
- Anything bigger goes through the edit queue.

### Queued edits (`world::Edit`)

A queued edit is done a slice per tick, so a box of any size never stops the mod and never stalls the server. Ask for the queue in `init`, describe the edit, and queue it:

```rust
use ferrite_api::server::world::{Edit, EditDone};

hooks
    .queue_edits(4096)
    .on_edit_done(|_, done: &mut EditDone| {
        if let Some(player) = &done.player {
            player.tell(format!("Changed {} blocks", done.changed));
        }
    });

let edit = Edit::replace(&player.dimension, a, b, "stone", "glass")
    .by(player)
    .record_undo();
let id = edit.queue()?;
```

| Edit | Does |
|---|---|
| `Edit::fill(&dimension, a, b, state)` | the whole box to one state; reads nothing unless recording undo |
| `Edit::replace(&dimension, a, b, from, to)` | blocks with the id `from` (or exactly the state, when `from` has `[properties]`) to `to` |
| `Edit::map(&dimension, a, b, \|pos, old\| ...)` | any rule: return `Some(new)` to change the block, `None` to keep it |
| `Edit::set(&dimension, changes)` | a list of `(BlockPos, state)` |
| `Edit::restore(&dimension, snapshot)` | puts a `Snapshot` back |
| `.by(&player)` | the player who asked; `EditDone.player` gives them back |
| `.record_undo()` | keeps what each changed block was; `done.take_undo()` returns the edit that puts it back |
| `.size()` | blocks it will visit |
| `.queue()` | starts it: `Ok(EditId)`; a state the game refuses is an `Err` here when the first slice runs at once, else `EditDone.error` |

`queue()` starts work at once with what is left of this tick's budget, so a small edit is finished, and `on_edit_done` called, before the command returns. Every edit queued in one tick shares that tick's budget, and edits running together share each later tick. `world::progress(id)` gives `done` and `total` (and `percent()`) while an edit runs and `None` after; `world::cancel(id)` stops it, and `on_edit_done` sees `cancelled` with everything done so far, undo included. `EditDone.changed` counts the blocks changed; `error` holds the game's error if a block could not be set, after which the edit stops.

The budget counts blocks visited, at most `world::MAX_EDITS_PER_TICK` (16 384); `on_edit_done` alone gives `world::DEFAULT_EDITS_PER_TICK` (4096), which reads, records and writes in a few milliseconds a tick. Undo is recorded as one copy of each distinct state plus 16 bytes a block, about 16 MiB for a million blocks: bound how much undo a mod keeps, since the whole mod has 64 MiB. Queued edits live in memory: a server restart or a mod reload drops them. `TestServer::finish_edits()` runs ticks until the queue is empty and returns how many it took.

`BlockState` wraps `minecraft:oak_log[axis=y]`: `id()` is `minecraft:oak_log`, `is("oak_log")` compares ids, `property("axis")` is `Some("y")`, `is_air()` covers every kind of air. `"stone".into()` is `minecraft:stone`.

`storage`, `Config` and `log` work in server mods as in client mods; `game`, `hud`, `chat`, `command`, `ui` and `input` are client only.

### Testing a server mod

```rust
let mut server = TestServer::<Builder>::start_in(
    ServerWorld::default().with_block(BlockPos::new(1, 64, 1), "stone"),
);
server.join("Alex").join("Steve");
assert_eq!(server.command("Alex", "pillar 3"), Ok(None));
assert!(server.block(BlockPos::new(2, 66, 0)).is("glowstone"));
assert_eq!(server.command("Alex", "pillar 300"), Err("Integer must not be more than 255, found 300".into()));
server.break_block("Steve", BlockPos::new(1, 64, 1));
assert_eq!(server.take_broadcasts(), ["..."]);
```

`TestServer` parses typed arguments as the server does (`~`, `@s`, ranges, vanilla's messages) and applies block changes after each call, as Horizon does. Players join at `0.5 64 0.5` in the overworld; `update_player(name, |p| ...)` moves them. It also has `leave`, `chat`, `place_block`, `tick`, `ticks`, `finish_edits`, `restart`, `block_in`, `set_block`, `take_told`, `take_log`, `file` and `file_text`. Unset blocks are air and the biome is `minecraft:plains`. In Horizon's sandbox, scripts use `join`, `as`, `break`, `place`, `blocks` and `expect.told`, `expect.broadcast`, `expect.blocks`: see `examples/builder`.

## Testing

### In Rust (`ferrite_api::testing`)

`cargo test` runs the mod natively against a fake game: no wasm build, normal `assert`s, a debugger works. Write the tests inside the mod crate (`#[cfg(test)] mod tests` in `src/lib.rs`): a `cdylib` crate cannot be linked from `tests/`.

```rust
#[cfg(test)]
mod tests {
    use ferrite_api::testing::{TestGame, World};

    use super::*;

    #[test]
    fn panel_follows_the_player() {
        let mut game = TestGame::<Compass>::start();
        game.update_world(|w| w.player.as_mut().unwrap().yaw = 180.0).tick();
        assert_eq!(game.panel_lines(), ["XYZ 0 64 0", "Facing North", "Overworld 06:00"]);

        game.click_setting("Corner").tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::TopRight);

        game.restart().tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::TopRight);
    }
}
```

The fake world starts like Horizon's sandbox: the player at `0.5 64 0.5` facing south with full health in the overworld, the clock at 0. Each tick moves the clock one tick on. `World::title_screen()` has no player and no clock; `World::default().with_file("config.json", "...")` starts with stored data.

| Drive the game | Read what the mod did |
|---|---|
| `tick()`, `ticks(n)`, `frame(seconds)` | `panel()`, `panel_lines()` |
| `command("compass corner top-right")` returns the result | `take_chat()`, `take_commands()`, `take_quiet_commands()` |
| `receive_chat(text)` | `take_log()` |
| `update_world(\|w\| ...)`: player, clock, `hit` for `game::raycast`, storage, `storage_quota` | `file(key)`, `file_text(key)`, `world()` |
| `open_settings()`, `click_setting(label)`, `pick_setting(row)` | `settings()` |
| `click(label)`, `pick(row)`, `close_screen()` on the mod's screen | `screen()` |
| `restart()`: the mod loads again, storage stays | `state()`: the mod's own struct, `commands()` |

A label matches a button exactly or up to its value: `click_setting("Corner")` clicks `Corner: top-left`. A missing button or row fails the test with the widgets that are there. `TestGame::try_start_in(world)` returns the error of a mod that refuses to start. Each test runs in its own thread with its own fake game; use one `TestGame` per test.

### In Horizon's sandbox

Test the mod as the game runs it with `cargo ferrite test`, which builds it to wasm and plays `tests/*.toml` scripts in Horizon's own sandbox:

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
| Server mod work per command or event | about 100 million instructions |
| Server mod work per tick | about 50 million instructions |
| Blocks per `world::set_blocks` call to the game | 4096 (Ferrite splits bigger lists) |
| Messages to players per hook | 8 |

A mod over budget is flagged; a runaway mod is stopped without affecting others.
