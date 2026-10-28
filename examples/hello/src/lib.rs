use ferrite_api::prelude::*;

struct Hello {
    ticks: u64,
}

impl Mod for Hello {
    fn init(hooks: &mut Hooks) -> Result<Self, String> {
        hooks.command("hello").command("ticks").on_tick();
        log::info("hello: loaded");
        Ok(Self { ticks: 0 })
    }

    fn tick(&mut self, _tick: u64) {
        self.ticks += 1;
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        match name {
            "hello" => {
                let who = if args.is_empty() { "world" } else { args };
                chat::show(&format!("Hello, {who}!"));
                Ok(None)
            }
            "ticks" => Ok(Some(format!("{} ticks since load", self.ticks))),
            _ => Err(format!("unknown command /{name}")),
        }
    }
}

export_mod!(Hello);
