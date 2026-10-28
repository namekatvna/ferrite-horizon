use crate::{Args, Hooks, Mod};

pub mod sdk {
    wit_bindgen::generate!({
        path: "wit",
        world: "horizon-ui-mod",
        pub_export_macro: true,
        export_macro_name: "__export_horizon_mod",
        default_bindings_module: "ferrite_api::__private::sdk",
    });

    pub use exports::horizon::mods::events::{Guest, Registration};
    pub use exports::horizon::mods::ui_events::Guest as UiEventsGuest;
    pub use horizon::mods::{chat, command, game, hud, log, storage};
}

pub struct Adapter<M> {
    inner: M,
    hooks: Hooks<M>,
    ticks: u64,
}

impl<M: Mod> Adapter<M> {
    pub fn init() -> Result<(Self, sdk::Registration), String> {
        let mut hooks = Hooks::new();
        let inner = M::init(&mut hooks).map_err(crate::Error::into_message)?;
        let registration = hooks.registration();
        let adapter = Self {
            inner,
            hooks,
            ticks: 0,
        };
        Ok((adapter, registration))
    }

    pub fn inner(&self) -> &M {
        &self.inner
    }

    pub fn tick(&mut self, tick: u64) {
        self.ticks += 1;
        if let Some(handler) = self.hooks.tick {
            handler(&mut self.inner, tick);
        }
        for &(interval, handler) in &self.hooks.timers {
            if self.ticks % interval == 0 {
                handler(&mut self.inner);
            }
        }
    }

    pub fn frame(&mut self, seconds: f32) {
        if let Some(handler) = self.hooks.frame {
            handler(&mut self.inner, seconds);
        }
    }

    pub fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        match self.hooks.find_command(name) {
            Some(handler) => {
                handler(&mut self.inner, Args::new(args)).map_err(crate::Error::into_message)
            }
            None => Err(format!("Unknown command /{name}")),
        }
    }

    pub fn chat(&mut self, text: &str) {
        if let Some(handler) = self.hooks.chat {
            handler(&mut self.inner, text);
        }
    }
}
