use std::collections::BTreeMap;

use super::fake::{ServerState, with};
use super::{Fake, World, boot_common};
use crate::__private::ServerAdapter;
use crate::__private::server_sdk::{ArgumentKind, Registration, ServerCommand};
use crate::game::{BlockPos, Dimension, Vec3};
use crate::log::Level;
use crate::server::{ArgumentValue, BlockState, GameMode, Player, ServerMod};

#[derive(Clone, Debug, PartialEq)]
pub struct ServerWorld {
    pub blocks: BTreeMap<BlockPos, BlockState>,
    pub biome: String,
    pub storage: BTreeMap<String, Vec<u8>>,
    pub storage_quota: Option<usize>,
}

impl Default for ServerWorld {
    fn default() -> Self {
        Self {
            blocks: BTreeMap::new(),
            biome: "minecraft:plains".to_owned(),
            storage: BTreeMap::new(),
            storage_quota: None,
        }
    }
}

impl ServerWorld {
    pub fn with_block(mut self, pos: BlockPos, state: impl Into<BlockState>) -> Self {
        self.blocks.insert(pos, state.into());
        self
    }

    pub fn with_file(mut self, key: &str, contents: impl AsRef<[u8]>) -> Self {
        self.storage
            .insert(key.to_owned(), contents.as_ref().to_vec());
        self
    }
}

pub struct TestServer<M: ServerMod> {
    adapter: ServerAdapter<M>,
    registration: Registration,
    tick: u64,
}

impl<M: ServerMod> TestServer<M> {
    pub fn start() -> Self {
        Self::start_in(ServerWorld::default())
    }

    pub fn start_in(world: ServerWorld) -> Self {
        Self::try_start_in(world).unwrap_or_else(|e| panic!("the mod refused to start: {e}"))
    }

    pub fn try_start_in(world: ServerWorld) -> Result<Self, String> {
        let overworld = Dimension::Overworld.id().to_owned();
        with(|f| {
            *f = Fake {
                world: World {
                    storage: world.storage,
                    storage_quota: world.storage_quota,
                    ..World::title_screen()
                },
                server: ServerState {
                    blocks: world
                        .blocks
                        .into_iter()
                        .map(|(pos, state)| ((overworld.clone(), pos), state.into()))
                        .collect(),
                    biome: world.biome,
                    ..ServerState::default()
                },
                ..Fake::default()
            }
        });
        let (adapter, registration) = boot::<M>()?;
        let mut server = Self {
            adapter,
            registration,
            tick: 0,
        };
        server.apply_changes();
        Ok(server)
    }

    pub fn restart(&mut self) -> &mut Self {
        if let Err(e) = self.try_restart() {
            panic!("the mod refused to restart: {e}");
        }
        self
    }

    pub fn try_restart(&mut self) -> Result<(), String> {
        with(|f| f.server.pending.clear());
        (self.adapter, self.registration) = boot::<M>()?;
        self.apply_changes();
        Ok(())
    }

    pub fn state(&self) -> &M {
        self.adapter.inner()
    }

    pub fn commands(&self) -> Vec<String> {
        self.registration
            .commands
            .iter()
            .map(|c| c.name.clone())
            .collect()
    }

    pub fn join(&mut self, name: &str) -> &mut Self {
        let player = Player {
            id: player_id(name),
            name: name.to_owned(),
            position: Vec3::new(0.5, 64.0, 0.5),
            dimension: Dimension::Overworld,
            game_mode: GameMode::Survival,
            health: 20.0,
        };
        with(|f| {
            f.server.online.retain(|p| p.name != name);
            f.server.online.push(player.clone());
        });
        if self.registration.joins {
            self.adapter.joined(&player);
        }
        self.apply_changes();
        self
    }

    pub fn leave(&mut self, name: &str) -> &mut Self {
        let player = self.player(name);
        with(|f| f.server.online.retain(|p| p.name != name));
        if self.registration.joins {
            self.adapter.left(&player);
        }
        self.apply_changes();
        self
    }

    pub fn player(&self, name: &str) -> Player {
        with(|f| {
            f.server
                .online
                .iter()
                .find(|p| p.name == name)
                .cloned()
                .unwrap_or_else(|| {
                    let online: Vec<&str> =
                        f.server.online.iter().map(|p| p.name.as_str()).collect();
                    panic!("`{name}` is not online; online: {online:?}; call join first")
                })
        })
    }

    pub fn update_player(&mut self, name: &str, change: impl FnOnce(&mut Player)) -> &mut Self {
        self.player(name);
        with(|f| {
            if let Some(player) = f.server.online.iter_mut().find(|p| p.name == name) {
                change(player);
            }
        });
        self
    }

    pub fn command(&mut self, as_player: &str, line: &str) -> Result<Option<String>, String> {
        let player = self.player(as_player);
        let line = line.trim();
        let line = line.strip_prefix('/').unwrap_or(line);
        let (name, args) = line.split_once(' ').unwrap_or((line, ""));
        let Some(command) = self.registration.commands.iter().find(|c| c.name == name) else {
            return Err(format!("Unknown command /{name}"));
        };
        let online = with(|f| f.server.online.clone());
        let values = parse_arguments(args, command, &player, &online)?;
        let result = self.adapter.command(&player, name, args, &values);
        self.apply_changes();
        result
    }

    pub fn chat(&mut self, as_player: &str, text: &str) -> &mut Self {
        let player = self.player(as_player);
        if self.registration.chat {
            self.adapter.chat(&player, text);
        }
        self.apply_changes();
        self
    }

    pub fn break_block(&mut self, as_player: &str, pos: BlockPos) -> &mut Self {
        let player = self.player(as_player);
        let previous = self.block_in(&player.dimension, pos);
        with(|f| {
            f.server
                .blocks
                .remove(&(player.dimension.id().to_owned(), pos))
        });
        if self.registration.blocks {
            self.adapter.block_broken(&player, pos, &previous);
        }
        self.apply_changes();
        self
    }

    pub fn place_block(
        &mut self,
        as_player: &str,
        pos: BlockPos,
        state: impl Into<BlockState>,
    ) -> &mut Self {
        let player = self.player(as_player);
        let state = state.into();
        with(|f| {
            f.server.blocks.insert(
                (player.dimension.id().to_owned(), pos),
                state.as_str().to_owned(),
            )
        });
        if self.registration.blocks {
            self.adapter.block_placed(&player, pos, &state);
        }
        self.apply_changes();
        self
    }

    pub fn tick(&mut self) -> &mut Self {
        self.ticks(1)
    }

    pub fn ticks(&mut self, count: u64) -> &mut Self {
        for _ in 0..count {
            self.tick += 1;
            if self.registration.tick {
                self.adapter.tick(self.tick);
            }
            self.apply_changes();
        }
        self
    }

    pub fn block(&self, pos: BlockPos) -> BlockState {
        self.block_in(&Dimension::Overworld, pos)
    }

    pub fn block_in(&self, dimension: &Dimension, pos: BlockPos) -> BlockState {
        with(|f| {
            f.server
                .blocks
                .get(&(dimension.id().to_owned(), pos))
                .map_or_else(BlockState::air, |s| BlockState::new(s.as_str()))
        })
    }

    pub fn set_block(&mut self, pos: BlockPos, state: impl Into<BlockState>) -> &mut Self {
        let state = state.into();
        with(|f| {
            f.server.blocks.insert(
                (Dimension::Overworld.id().to_owned(), pos),
                state.as_str().to_owned(),
            )
        });
        self
    }

    pub fn take_told(&mut self) -> Vec<(String, String)> {
        with(|f| std::mem::take(&mut f.server.told))
    }

    pub fn take_broadcasts(&mut self) -> Vec<String> {
        with(|f| std::mem::take(&mut f.server.broadcasts))
    }

    pub fn take_log(&mut self) -> Vec<(Level, String)> {
        with(|f| std::mem::take(&mut f.log))
    }

    pub fn file(&self, key: &str) -> Option<Vec<u8>> {
        with(|f| f.world.storage.get(key).cloned())
    }

    pub fn file_text(&self, key: &str) -> Option<String> {
        self.file(key)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    fn apply_changes(&mut self) {
        with(|f| {
            for (dimension, pos, state) in std::mem::take(&mut f.server.pending) {
                if BlockState::new(state.as_str()).is_air() {
                    f.server.blocks.remove(&(dimension, pos));
                } else {
                    f.server.blocks.insert((dimension, pos), state);
                }
            }
        });
    }
}

fn boot<M: ServerMod>() -> Result<(ServerAdapter<M>, Registration), String> {
    boot_common();
    ServerAdapter::<M>::init()
}

fn player_id(name: &str) -> String {
    let hash = name
        .to_ascii_lowercase()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, b| {
            (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("00000000-0000-4000-8000-{:012x}", hash & 0xffff_ffff_ffff)
}

const INCOMPLETE: &str = "Unknown or incomplete command, see below for error";
const INCORRECT: &str = "Incorrect argument for command";

fn parse_arguments(
    line: &str,
    command: &ServerCommand,
    issuer: &Player,
    online: &[Player],
) -> Result<Vec<(String, ArgumentValue)>, String> {
    let mut rest = line.trim_start();
    let mut values = Vec::new();
    for argument in &command.arguments {
        if rest.trim().is_empty() {
            if argument.optional {
                break;
            }
            return Err(INCOMPLETE.to_owned());
        }
        let value = match argument.kind {
            ArgumentKind::Text => {
                let text = rest.trim().to_owned();
                rest = "";
                ArgumentValue::Text(text)
            }
            ArgumentKind::Word => {
                let word = next(&mut rest);
                if !word
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'+'))
                {
                    return Err(INCORRECT.to_owned());
                }
                ArgumentValue::Word(word.to_owned())
            }
            ArgumentKind::Integer((min, max)) => {
                let word = next(&mut rest);
                let n: i32 = word
                    .parse()
                    .map_err(|_| format!("Invalid integer '{word}'"))?;
                if let Some(min) = min.filter(|min| n < *min) {
                    return Err(format!("Integer must not be less than {min}, found {n}"));
                }
                if let Some(max) = max.filter(|max| n > *max) {
                    return Err(format!("Integer must not be more than {max}, found {n}"));
                }
                ArgumentValue::Integer(n)
            }
            ArgumentKind::Number => {
                let word = next(&mut rest);
                ArgumentValue::Number(
                    word.parse()
                        .map_err(|_| format!("Invalid double '{word}'"))?,
                )
            }
            ArgumentKind::Boolean => {
                let word = next(&mut rest);
                ArgumentValue::Boolean(match word {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(format!(
                            "Invalid boolean, expected 'true' or 'false' but found '{word}'"
                        ));
                    }
                })
            }
            ArgumentKind::Player => {
                let word = next(&mut rest);
                let player = match word {
                    "@s" | "@p" => issuer.clone(),
                    name => online
                        .iter()
                        .find(|p| p.name.eq_ignore_ascii_case(name))
                        .cloned()
                        .ok_or_else(|| "No player was found".to_owned())?,
                };
                ArgumentValue::Player(player)
            }
            ArgumentKind::BlockPos => {
                let origin = issuer.block_pos();
                let mut coordinate = |base: i32| -> Result<i32, String> {
                    let word = next(&mut rest);
                    if word.is_empty() {
                        return Err(INCOMPLETE.to_owned());
                    }
                    match word.strip_prefix('~') {
                        Some("") => Ok(base),
                        Some(offset) => offset
                            .parse::<i32>()
                            .map(|o| base + o)
                            .map_err(|_| format!("Invalid integer '{offset}'")),
                        None => word
                            .parse()
                            .map_err(|_| format!("Invalid integer '{word}'")),
                    }
                };
                let x = coordinate(origin.x)?;
                let y = coordinate(origin.y)?;
                let z = coordinate(origin.z)?;
                ArgumentValue::BlockPos(BlockPos::new(x, y, z))
            }
        };
        values.push((argument.name.clone(), value));
    }
    if !rest.trim().is_empty() {
        return Err(INCORRECT.to_owned());
    }
    Ok(values)
}

fn next<'a>(rest: &mut &'a str) -> &'a str {
    let trimmed = rest.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let (word, tail) = trimmed.split_at(end);
    *rest = tail;
    word
}
