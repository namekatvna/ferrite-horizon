pub use horizon_mod_sdk as sdk;

use crate::{Hooks, Mod};

pub struct Adapter<M>(M);

impl<M: Mod> Adapter<M> {
    pub fn inner(&self) -> &M {
        &self.0
    }
}

impl<M: Mod> sdk::Mod for Adapter<M> {
    fn init() -> Result<(Self, sdk::Registration), String> {
        let mut hooks = Hooks::default();
        let inner = M::init(&mut hooks)?;
        Ok((Self(inner), hooks.into_registration()))
    }

    fn tick(&mut self, tick: u64) {
        self.0.tick(tick)
    }

    fn frame(&mut self, seconds: f32) {
        self.0.frame(seconds)
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        self.0.command(name, args)
    }

    fn chat(&mut self, text: &str) {
        self.0.chat(text)
    }
}
