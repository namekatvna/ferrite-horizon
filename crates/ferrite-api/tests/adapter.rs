use ferrite_api::__private::{Adapter, sdk};
use ferrite_api::prelude::*;

#[derive(Debug, Default)]
struct Counter {
    ticks: u64,
    frames: u32,
    lines: Vec<String>,
}

impl Mod for Counter {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .command("count", Self::count)
            .command("/add", Self::add)
            .command("count", Self::count)
            .on_tick(Self::tick)
            .on_frame(|m, _| m.frames += 1)
            .on_chat(Self::chat);
        Ok(Self::default())
    }
}

impl Counter {
    fn tick(&mut self, _tick: u64) {
        self.ticks += 1;
    }

    fn chat(&mut self, text: &str) {
        self.lines.push(text.to_owned());
    }

    fn count(&mut self, args: Args) -> CommandResult {
        if !args.is_empty() {
            return Err(format!("unexpected `{}`", args.as_str()).into());
        }
        Ok(Some(self.ticks.to_string()))
    }

    fn add(&mut self, args: Args) -> CommandResult {
        self.ticks += args.get::<u64>(0)?;
        Ok(None)
    }
}

struct Refuses;

impl Mod for Refuses {
    fn init(_hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        Err("missing config".into())
    }
}

#[derive(Default)]
struct Timers {
    fast: u32,
    slow: u32,
}

impl Mod for Timers {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .every(5, |m| m.fast += 1)
            .every_seconds(1, |m| m.slow += 1)
            .every(0, |m| m.fast += 100);
        Ok(Self::default())
    }
}

struct Silent;

impl Mod for Silent {
    fn init(_hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        Ok(Self)
    }
}

export_mod!(Counter);

fn start<M: Mod>() -> (Adapter<M>, sdk::Registration) {
    Adapter::<M>::init().unwrap()
}

#[test]
fn handlers_become_registration() {
    let (_, registration) = start::<Counter>();
    assert_eq!(registration.commands, ["count", "add"]);
    assert!(registration.tick && registration.frame && registration.chat);
}

#[test]
fn events_reach_their_handlers() {
    let (mut adapter, _) = start::<Counter>();
    for tick in 0..3 {
        adapter.tick(tick);
    }
    adapter.frame(0.016);
    adapter.chat("hi");

    assert_eq!(adapter.inner().ticks, 3);
    assert_eq!(adapter.inner().frames, 1);
    assert_eq!(adapter.inner().lines, ["hi"]);
}

#[test]
fn commands_dispatch_by_name_with_parsed_args() {
    let (mut adapter, _) = start::<Counter>();

    assert_eq!(adapter.command("add", " 40 "), Ok(None));
    assert_eq!(adapter.command("count", ""), Ok(Some("40".into())));
    assert_eq!(
        adapter.command("add", "many"),
        Err("Invalid argument #1 `many`: invalid digit found in string".into())
    );
    assert_eq!(adapter.command("count", "x"), Err("unexpected `x`".into()));
    assert_eq!(
        adapter.command("nope", ""),
        Err("Unknown command /nope".into())
    );
}

#[test]
fn init_error_is_passed_through() {
    let err = Adapter::<Refuses>::init().err();
    assert_eq!(err.as_deref(), Some("missing config"));
}

#[test]
fn timers_fire_on_their_interval_and_need_the_tick_hook() {
    let (mut adapter, registration) = start::<Timers>();
    assert!(registration.tick);

    for tick in 0..4 {
        adapter.tick(tick);
    }
    assert_eq!(adapter.inner().fast, 400);
    assert_eq!(adapter.inner().slow, 0);

    for tick in 4..40 {
        adapter.tick(tick);
    }
    assert_eq!(adapter.inner().fast, 4000 + 8);
    assert_eq!(adapter.inner().slow, 2);
}

#[test]
fn defaults_register_nothing() {
    let (_, registration) = start::<Silent>();
    assert!(registration.commands.is_empty());
    assert!(!registration.tick && !registration.frame && !registration.chat);
}
