pub use horizon_mod_sdk as sdk;

use crate::{Args, Hooks, Mod};

pub struct Adapter<M> {
    inner: M,
    hooks: Hooks<M>,
}

impl<M: Mod> Adapter<M> {
    pub fn inner(&self) -> &M {
        &self.inner
    }
}

impl<M: Mod> sdk::Mod for Adapter<M> {
    fn init() -> Result<(Self, sdk::Registration), String> {
        let mut hooks = Hooks::new();
        let inner = M::init(&mut hooks).map_err(crate::Error::into_message)?;
        let registration = hooks.registration();
        Ok((Self { inner, hooks }, registration))
    }

    fn tick(&mut self, tick: u64) {
        if let Some(handler) = self.hooks.tick {
            handler(&mut self.inner, tick)
        }
    }

    fn frame(&mut self, seconds: f32) {
        if let Some(handler) = self.hooks.frame {
            handler(&mut self.inner, seconds)
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        match self.hooks.find_command(name) {
            Some(handler) => {
                handler(&mut self.inner, Args::new(args)).map_err(crate::Error::into_message)
            }
            None => Err(format!("Unknown command /{name}")),
        }
    }

    fn chat(&mut self, text: &str) {
        if let Some(handler) = self.hooks.chat {
            handler(&mut self.inner, text)
        }
    }
}
