use std::collections::HashMap;

use ferrite_api::server::prelude::*;

const MAX_BLOCKS: u64 = 4096;
const UNDO_STEPS: usize = 8;

#[derive(FromArgs)]
struct Point {
    pos: Option<BlockPos>,
}

#[derive(FromArgs)]
struct Set {
    block: BlockState,
}

#[derive(FromArgs)]
struct Replace {
    from: BlockState,
    to: BlockState,
}

#[derive(Default)]
struct Session {
    pos1: Option<BlockPos>,
    pos2: Option<BlockPos>,
    undo: Vec<(Dimension, Vec<(BlockPos, BlockState)>)>,
}

#[derive(Default)]
struct Builder {
    sessions: HashMap<String, Session>,
}

impl ServerMod for Builder {
    fn init(hooks: &mut ServerHooks<Self>) -> Result<Self, Error> {
        hooks
            .parsed_command("pos1", |m, player, point: Point| {
                m.mark(player, point, true)
            })
            .parsed_command("pos2", |m, player, point: Point| {
                m.mark(player, point, false)
            })
            .parsed_command("set", Self::set)
            .parsed_command("replace", Self::replace)
            .command("undo", Self::undo)
            .command("sel", Self::selection)
            .on_leave(|m, player| {
                m.sessions.remove(&player.id);
            });
        Ok(Self::default())
    }
}

impl Builder {
    fn mark(&mut self, player: &Player, point: Point, first: bool) -> CommandResult {
        let pos = point.pos.unwrap_or_else(|| player.block_pos());
        let session = self.sessions.entry(player.id.clone()).or_default();
        let (slot, name) = if first {
            (&mut session.pos1, "First")
        } else {
            (&mut session.pos2, "Second")
        };
        *slot = Some(pos);
        reply(format!("{name} corner at {pos}"))
    }

    fn selection(&mut self, player: &Player, _args: Args) -> CommandResult {
        let (a, b) = self.corners(player)?;
        reply(format!("{a} to {b}: {} blocks", BlockPos::volume(a, b)))
    }

    fn set(&mut self, player: &Player, set: Set) -> CommandResult {
        let changed = self.edit(player, |_| Some(set.block.clone()))?;
        reply(format!("Set {changed} blocks to {}", set.block.id()))
    }

    fn replace(&mut self, player: &Player, replace: Replace) -> CommandResult {
        let changed = self.edit(player, |old| {
            old.is(replace.from.id()).then(|| replace.to.clone())
        })?;
        reply(format!(
            "Replaced {changed} {} with {}",
            replace.from.id(),
            replace.to.id()
        ))
    }

    fn undo(&mut self, player: &Player, _args: Args) -> CommandResult {
        let session = self.sessions.entry(player.id.clone()).or_default();
        let (dimension, blocks) = session.undo.pop().ok_or("Nothing to undo")?;
        let restored = world::set_blocks(&dimension, blocks)?;
        reply(format!("Restored {restored} blocks"))
    }

    fn corners(&self, player: &Player) -> Result<(BlockPos, BlockPos), Error> {
        let session = self.sessions.get(&player.id);
        match session.map(|s| (s.pos1, s.pos2)) {
            Some((Some(a), Some(b))) => Ok((a, b)),
            _ => Err("Select a box first with /pos1 and /pos2".into()),
        }
    }

    fn edit(
        &mut self,
        player: &Player,
        change: impl Fn(&BlockState) -> Option<BlockState>,
    ) -> Result<usize, Error> {
        let (a, b) = self.corners(player)?;
        let volume = BlockPos::volume(a, b);
        if volume > MAX_BLOCKS {
            return Err(format!("The box has {volume} blocks; the most is {MAX_BLOCKS}").into());
        }
        let dimension = &player.dimension;
        let mut before = Vec::new();
        let mut after = Vec::new();
        for pos in BlockPos::cuboid(a, b) {
            let Some(old) = world::block(dimension, pos) else {
                continue;
            };
            if let Some(new) = change(&old).filter(|new| *new != old) {
                after.push((pos, new));
                before.push((pos, old));
            }
        }
        let changed = world::set_blocks(dimension, after)?;
        let undo = &mut self.sessions.entry(player.id.clone()).or_default().undo;
        undo.push((dimension.clone(), before));
        if undo.len() > UNDO_STEPS {
            undo.remove(0);
        }
        Ok(changed)
    }
}

export_server_mod!(Builder);

#[cfg(test)]
mod tests {
    use ferrite_api::testing::{ServerWorld, TestServer};

    use super::*;

    fn server() -> TestServer<Builder> {
        let mut server = TestServer::<Builder>::start_in(
            ServerWorld::default().with_block(BlockPos::new(1, 64, 1), "stone"),
        );
        server.join("Alex");
        server
    }

    #[test]
    fn set_fills_the_selection_and_undo_restores_it() {
        let mut server = server();
        assert_eq!(
            server.command("Alex", "pos1"),
            Ok(Some("First corner at 0 64 0".into()))
        );
        server.command("Alex", "pos2 ~2 ~1 ~2").unwrap();
        assert_eq!(
            server.command("Alex", "sel"),
            Ok(Some("0 64 0 to 2 65 2: 18 blocks".into()))
        );
        assert_eq!(
            server.command("Alex", "set glass"),
            Ok(Some("Set 18 blocks to minecraft:glass".into()))
        );
        assert!(server.block(BlockPos::new(2, 65, 2)).is("glass"));

        assert_eq!(
            server.command("Alex", "undo"),
            Ok(Some("Restored 18 blocks".into()))
        );
        assert!(server.block(BlockPos::new(1, 64, 1)).is("stone"));
        assert!(server.block(BlockPos::new(2, 65, 2)).is_air());
        assert_eq!(
            server.command("Alex", "undo"),
            Err("Nothing to undo".into())
        );
    }

    #[test]
    fn replace_changes_only_matching_blocks() {
        let mut server = server();
        server.command("Alex", "pos1 0 64 0").unwrap();
        server.command("Alex", "pos2 2 64 2").unwrap();
        assert_eq!(
            server.command("Alex", "replace stone gold_block"),
            Ok(Some(
                "Replaced 1 minecraft:stone with minecraft:gold_block".into()
            ))
        );
        assert!(server.block(BlockPos::new(1, 64, 1)).is("gold_block"));
        assert!(server.block(BlockPos::new(0, 64, 0)).is_air());
    }

    #[test]
    fn selections_are_per_player_and_limited() {
        let mut server = server();
        server.join("Steve");
        server.command("Alex", "pos1").unwrap();
        server.command("Alex", "pos2 ~20 ~20 ~20").unwrap();
        assert_eq!(
            server.command("Steve", "set stone"),
            Err("Select a box first with /pos1 and /pos2".into())
        );
        assert_eq!(
            server.command("Alex", "set stone"),
            Err("The box has 9261 blocks; the most is 4096".into())
        );
    }
}
