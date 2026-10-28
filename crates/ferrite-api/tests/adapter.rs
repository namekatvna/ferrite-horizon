use ferrite_api::__private::{Adapter, sdk};
use ferrite_api::prelude::*;

#[derive(Debug, Default)]
struct Counter {
    ticks: u64,
    lines: Vec<String>,
}

impl Mod for Counter {
    fn init(hooks: &mut Hooks) -> Result<Self, String> {
        hooks.command("count").command("count").on_tick().on_chat();
        Ok(Self::default())
    }

    fn tick(&mut self, _tick: u64) {
        self.ticks += 1;
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        match (name, args) {
            ("count", "") => Ok(Some(self.ticks.to_string())),
            ("count", other) => Err(format!("unexpected argument `{other}`")),
            _ => Err(format!("unknown command `{name}`")),
        }
    }

    fn chat(&mut self, text: &str) {
        self.lines.push(text.to_owned());
    }
}

struct Refuses;

impl Mod for Refuses {
    fn init(_hooks: &mut Hooks) -> Result<Self, String> {
        Err("missing config".into())
    }
}

struct Silent;

impl Mod for Silent {
    fn init(_hooks: &mut Hooks) -> Result<Self, String> {
        Ok(Self)
    }
}

export_mod!(Counter);

#[test]
fn hooks_become_registration() {
    let (_, registration) = <Adapter<Counter> as sdk::Mod>::init().unwrap();
    assert_eq!(registration.commands, ["count"]);
    assert!(registration.tick);
    assert!(registration.chat);
    assert!(!registration.frame);
}

#[test]
fn events_reach_the_mod() {
    let (mut adapter, _) = <Adapter<Counter> as sdk::Mod>::init().unwrap();
    for tick in 0..3 {
        sdk::Mod::tick(&mut adapter, tick);
    }
    sdk::Mod::chat(&mut adapter, "hi");

    assert_eq!(
        sdk::Mod::command(&mut adapter, "count", ""),
        Ok(Some("3".into()))
    );
    assert!(sdk::Mod::command(&mut adapter, "count", "x").is_err());
    assert_eq!(adapter.inner().lines, ["hi"]);
}

#[test]
fn init_error_is_passed_through() {
    let err = <Adapter<Refuses> as sdk::Mod>::init().err();
    assert_eq!(err.as_deref(), Some("missing config"));
}

#[test]
fn defaults_register_nothing() {
    let (mut adapter, registration) = <Adapter<Silent> as sdk::Mod>::init().unwrap();
    assert!(registration.commands.is_empty());
    assert!(!registration.tick && !registration.frame && !registration.chat);
    assert!(sdk::Mod::command(&mut adapter, "any", "").is_err());
}
