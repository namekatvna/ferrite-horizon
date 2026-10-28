use ferrite_api::prelude::*;

struct Hello {
    ticks: u64,
}

impl Mod for Hello {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .command("hello", Self::hello)
            .command("ticks", Self::ticks)
            .on_tick(|hello, _| hello.ticks += 1);
        log::info("hello: loaded");
        Ok(Self { ticks: 0 })
    }
}

impl Hello {
    fn hello(&mut self, args: Args) -> CommandResult {
        let who = if args.is_empty() {
            "world"
        } else {
            args.as_str()
        };
        chat::show(format!("Hello, {who}!"));
        Ok(None)
    }

    fn ticks(&mut self, _args: Args) -> CommandResult {
        Ok(Some(format!("{} ticks since load", self.ticks)))
    }
}

export_mod!(Hello);
