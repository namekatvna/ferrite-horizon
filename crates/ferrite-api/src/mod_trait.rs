use crate::Hooks;

pub trait Mod: Sized + 'static {
    fn init(hooks: &mut Hooks) -> Result<Self, String>;

    fn tick(&mut self, _tick: u64) {}

    fn frame(&mut self, _seconds: f32) {}

    fn command(&mut self, _name: &str, _args: &str) -> Result<Option<String>, String> {
        Err("This command does nothing".into())
    }

    fn chat(&mut self, _text: &str) {}
}
