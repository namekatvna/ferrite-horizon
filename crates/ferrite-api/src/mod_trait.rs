use crate::{Error, Hooks};

pub trait Mod: Sized + 'static {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error>;
}
