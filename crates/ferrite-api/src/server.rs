use std::fmt;
use std::str::FromStr;

use crate::Error;
use crate::game::{BlockPos, Dimension, Vec3};

mod args;
mod export;
mod hooks;
pub mod players;
pub(crate) mod runtime;
pub mod world;

pub use args::{ArgumentKind, ArgumentSpec, ArgumentValue, ServerArgument, Values};
pub use hooks::{
    BlockHandler, PlayerHandler, ServerChatHandler, ServerCommandHandler, ServerHooks,
    TypedServerCommandHandler,
};

pub trait ServerMod: Sized + 'static {
    fn init(hooks: &mut ServerHooks<Self>) -> Result<Self, Error>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
            Self::Spectator => "spectator",
        }
    }
}

impl fmt::Display for GameMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub position: Vec3,
    pub dimension: Dimension,
    pub game_mode: GameMode,
    pub health: f32,
}

impl Player {
    pub fn block_pos(&self) -> BlockPos {
        self.position.block_pos()
    }

    pub fn tell(&self, text: impl AsRef<str>) -> bool {
        players::tell(self, text)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerArgumentError;

impl fmt::Display for PlayerArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a player can only be an argument of a server command")
    }
}

impl FromStr for Player {
    type Err = PlayerArgumentError;

    fn from_str(_text: &str) -> Result<Self, PlayerArgumentError> {
        Err(PlayerArgumentError)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "String", into = "String"))]
pub struct BlockState(String);

impl BlockState {
    pub const AIR: &'static str = "minecraft:air";

    pub fn new(state: impl Into<String>) -> Self {
        let state = state.into();
        let state = state.trim();
        let id_end = state.find('[').unwrap_or(state.len());
        if state[..id_end].contains(':') {
            Self(state.to_owned())
        } else {
            Self(format!("minecraft:{state}"))
        }
    }

    pub fn air() -> Self {
        Self(Self::AIR.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn id(&self) -> &str {
        self.0.split('[').next().unwrap_or(&self.0)
    }

    pub fn is(&self, id: &str) -> bool {
        let id = id.trim();
        if id.contains(':') {
            self.id() == id
        } else {
            self.id().strip_prefix("minecraft:") == Some(id)
        }
    }

    pub fn is_air(&self) -> bool {
        matches!(
            self.id(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    }

    pub fn property(&self, name: &str) -> Option<&str> {
        let properties = self.0.split_once('[')?.1.strip_suffix(']')?;
        properties
            .split(',')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| key.trim() == name)
            .map(|(_, value)| value.trim())
    }
}

impl fmt::Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for BlockState {
    fn from(state: &str) -> Self {
        Self::new(state)
    }
}

impl From<String> for BlockState {
    fn from(state: String) -> Self {
        Self::new(state)
    }
}

impl From<BlockState> for String {
    fn from(state: BlockState) -> Self {
        state.0
    }
}

impl From<&BlockState> for BlockState {
    fn from(state: &BlockState) -> Self {
        state.clone()
    }
}

impl FromStr for BlockState {
    type Err = std::convert::Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(text))
    }
}

pub mod prelude {
    #[cfg(feature = "serde")]
    pub use crate::config::Config;
    pub use crate::game::{BlockPos, Dimension, Vec3};
    pub use crate::server::{BlockState, GameMode, Player, ServerHooks, ServerMod, players, world};
    pub use crate::{
        Args, CommandResult, Error, FromArgs, export_server_mod, log, no_reply, reply, storage,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_states_read_like_ids() {
        let log = BlockState::from("oak_log[axis=y,waterlogged=false]");
        assert_eq!(log.as_str(), "minecraft:oak_log[axis=y,waterlogged=false]");
        assert_eq!(log.id(), "minecraft:oak_log");
        assert!(log.is("oak_log") && log.is("minecraft:oak_log") && !log.is("log"));
        assert_eq!(log.property("axis"), Some("y"));
        assert_eq!(log.property("snowy"), None);
        assert_eq!(BlockState::from("mymod:ruby").id(), "mymod:ruby");
        assert!(BlockState::from("cave_air").is_air());
        assert!(!BlockState::from("stone").is_air());
    }
}
