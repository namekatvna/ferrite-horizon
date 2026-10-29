use std::borrow::Cow;
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
        if is_trimmed(&state) && has_namespace(&state) {
            return Self(state);
        }
        Self::from_text(&state)
    }

    fn from_text(state: &str) -> Self {
        let state = state.trim();
        if has_namespace(state) {
            Self(state.to_owned())
        } else {
            Self::with_namespace(state)
        }
    }

    pub(crate) fn text(state: &str) -> Cow<'_, str> {
        if is_trimmed(state) && has_namespace(state) {
            Cow::Borrowed(state)
        } else {
            Cow::Owned(Self::from_text(state).0)
        }
    }

    fn with_namespace(state: &str) -> Self {
        let mut full = String::with_capacity(Self::NAMESPACE.len() + state.len());
        full.push_str(Self::NAMESPACE);
        full.push_str(state);
        Self(full)
    }

    const NAMESPACE: &'static str = "minecraft:";

    pub fn air() -> Self {
        Self(Self::AIR.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn id(&self) -> &str {
        let end = self
            .0
            .bytes()
            .position(|byte| byte == b'[')
            .unwrap_or(self.0.len());
        &self.0[..end]
    }

    pub fn is(&self, id: &str) -> bool {
        let id = if is_trimmed(id) { id } else { id.trim() };
        let own = self.id();
        if own.len() == id.len() {
            return own == id;
        }
        own.len() == Self::NAMESPACE.len() + id.len()
            && own.as_bytes()[Self::NAMESPACE.len()..] == *id.as_bytes()
            && own.starts_with(Self::NAMESPACE)
            && !has_namespace(id)
    }

    pub(crate) fn from_host(state: String) -> Self {
        Self(state)
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

fn has_namespace(state: &str) -> bool {
    for &byte in state.as_bytes() {
        match byte {
            b':' => return true,
            b'[' => return false,
            _ => {}
        }
    }
    false
}

fn is_trimmed(state: &str) -> bool {
    let bytes = state.as_bytes();
    let plain = |byte: Option<&u8>| byte.is_none_or(|b| b.is_ascii_graphic());
    plain(bytes.first()) && plain(bytes.last())
}

impl fmt::Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for BlockState {
    fn from(state: &str) -> Self {
        Self::from_text(state)
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
        Ok(Self::from_text(text))
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

    #[test]
    fn states_are_normalized_once() {
        assert_eq!(BlockState::new("  stone ").as_str(), "minecraft:stone");
        assert_eq!(
            BlockState::new(" minecraft:stone\n").as_str(),
            "minecraft:stone"
        );
        assert_eq!(
            BlockState::new("oak_log[axis=y]").as_str(),
            "minecraft:oak_log[axis=y]"
        );
        assert_eq!(
            BlockState::from("chest[facing=north,type=left]").id(),
            "minecraft:chest"
        );
        assert_eq!(BlockState::text("minecraft:stone"), "minecraft:stone");
        assert!(matches!(
            BlockState::text("minecraft:stone"),
            Cow::Borrowed(_)
        ));
        assert!(matches!(BlockState::text("stone"), Cow::Owned(_)));
    }

    #[test]
    fn is_matches_whole_ids_only() {
        let stone = BlockState::from("stone");
        assert!(stone.is(" stone ") && stone.is("minecraft:stone"));
        assert!(!stone.is("tone") && !stone.is("minecraft:ston") && !stone.is("stones"));
        assert!(!stone.is("other:stone") && !stone.is(""));
        let ruby = BlockState::from("mymod:ruby");
        assert!(ruby.is("mymod:ruby") && !ruby.is("ruby"));
        assert!(!BlockState::from("minecraft:foo:bar").is("foo:bar"));
    }
}
