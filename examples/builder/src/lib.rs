use std::collections::VecDeque;

use ferrite_api::server::prelude::*;
use ferrite_api::server::world::{Edit, EditDone, EditId};

const MAX_BLOCKS: u64 = 1 << 19;
const BLOCKS_PER_TICK: usize = 4096;
const UNDO_STEPS: usize = 8;
const UNDO_BLOCKS: u64 = 1 << 20;

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

struct Running {
    id: EditId,
    message: String,
}

#[derive(Default)]
struct Session {
    pos1: Option<BlockPos>,
    pos2: Option<BlockPos>,
    undo: VecDeque<Edit>,
    running: Option<Running>,
}

#[derive(Default)]
struct Builder {
    sessions: PlayerMap<Session>,
}

impl ServerMod for Builder {
    fn init(hooks: &mut ServerHooks<Self>) -> Result<Self, Error> {
        hooks
            .queue_edits(BLOCKS_PER_TICK)
            .parsed_command("pos1", |m, player, point: Point| {
                m.mark(player, point, true)
            })
            .parsed_command("pos2", |m, player, point: Point| {
                m.mark(player, point, false)
            })
            .parsed_command("set", Self::set)
            .parsed_command("replace", Self::replace)
            .simple_command("undo", Self::undo)
            .simple_command("cancel", Self::cancel)
            .simple_command("sel", Self::selection)
            .on_edit_done(Self::finished)
            .on_leave(|m, player| {
                m.sessions.remove(player);
            });
        Ok(Self::default())
    }
}

impl Builder {
    fn mark(&mut self, player: &Player, point: Point, first: bool) -> CommandResult {
        let pos = point.pos.unwrap_or_else(|| player.block_pos());
        let session = self.sessions.get_or_default(player);
        let (slot, name) = if first {
            (&mut session.pos1, "First")
        } else {
            (&mut session.pos2, "Second")
        };
        *slot = Some(pos);
        reply(format!("{name} corner at {pos}"))
    }

    fn selection(&mut self, player: &Player) -> CommandResult {
        let (a, b) = self.corners(player)?;
        reply(format!("{a} to {b}: {} blocks", BlockPos::volume(a, b)))
    }

    fn set(&mut self, player: &Player, set: Set) -> CommandResult {
        let (a, b) = self.selected_box(player)?;
        let message = format!("Set {{n}} blocks to {}", set.block.id());
        let edit = Edit::fill(&player.dimension, a, b, set.block).record_undo();
        self.start(player, edit, message)
    }

    fn replace(&mut self, player: &Player, replace: Replace) -> CommandResult {
        let (a, b) = self.selected_box(player)?;
        let message = format!(
            "Replaced {{n}} {} with {}",
            replace.from.id(),
            replace.to.id()
        );
        let edit = Edit::replace(&player.dimension, a, b, replace.from, replace.to).record_undo();
        self.start(player, edit, message)
    }

    fn undo(&mut self, player: &Player) -> CommandResult {
        self.ensure_idle(player)?;
        let session = self.sessions.get_or_default(player);
        let edit = session.undo.pop_back().ok_or("Nothing to undo")?;
        self.start(player, edit, "Restored {n} blocks".to_owned())
    }

    fn cancel(&mut self, player: &Player) -> CommandResult {
        let running = self.sessions.get(player).and_then(|s| s.running.as_ref());
        match running {
            Some(running) if world::cancel(running.id) => no_reply(),
            _ => Err("Nothing to cancel".into()),
        }
    }

    fn start(&mut self, player: &Player, edit: Edit, message: String) -> CommandResult {
        self.ensure_idle(player)?;
        let size = edit.size();
        let id = edit.by(player).queue()?;
        self.sessions.get_or_default(player).running = Some(Running { id, message });
        match world::progress(id) {
            Some(progress) => reply(format!(
                "Working on {size} blocks ({}% done); /cancel stops it",
                progress.percent()
            )),
            None => no_reply(),
        }
    }

    fn finished(&mut self, done: &mut EditDone) {
        let Some(player) = done.player.clone() else {
            return;
        };
        let Some(session) = self.sessions.get_mut(&player) else {
            return;
        };
        let message = match session.running.take() {
            Some(running) if running.id == done.id => running.message,
            other => {
                session.running = other;
                "Changed {n} blocks".to_owned()
            }
        };
        if let Some(undo) = done.take_undo().filter(|undo| undo.size() > 0) {
            session.undo.push_back(undo);
            while session.undo.len() > UNDO_STEPS
                || session.undo.iter().map(Edit::size).sum::<u64>() > UNDO_BLOCKS
            {
                session.undo.pop_front();
            }
        }
        let mut text = message.replace("{n}", &done.changed.to_string());
        if let Some(error) = &done.error {
            text = format!("{text}, then stopped: {error}");
        } else if done.cancelled {
            text.push_str(" before /cancel");
        }
        player.tell(text);
    }

    fn ensure_idle(&self, player: &Player) -> Result<(), Error> {
        let running = self.sessions.get(player).and_then(|s| s.running.as_ref());
        match running.and_then(|running| world::progress(running.id)) {
            Some(progress) => Err(format!(
                "Still working on the last edit ({}% done); /cancel stops it",
                progress.percent()
            )
            .into()),
            None => Ok(()),
        }
    }

    fn corners(&self, player: &Player) -> Result<(BlockPos, BlockPos), Error> {
        match self.sessions.get(player).map(|s| (s.pos1, s.pos2)) {
            Some((Some(a), Some(b))) => Ok((a, b)),
            _ => Err("Select a box first with /pos1 and /pos2".into()),
        }
    }

    fn selected_box(&self, player: &Player) -> Result<(BlockPos, BlockPos), Error> {
        let (a, b) = self.corners(player)?;
        let volume = BlockPos::volume(a, b);
        if volume > MAX_BLOCKS {
            return Err(format!("The box has {volume} blocks; the most is {MAX_BLOCKS}").into());
        }
        Ok((a, b))
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

    fn told(server: &mut TestServer<Builder>) -> Vec<String> {
        server
            .take_told()
            .into_iter()
            .map(|(_, text)| text)
            .collect()
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
        assert_eq!(server.command("Alex", "set glass"), Ok(None));
        assert_eq!(told(&mut server), ["Set 18 blocks to minecraft:glass"]);
        assert!(server.block(BlockPos::new(2, 65, 2)).is("glass"));

        assert_eq!(server.command("Alex", "undo"), Ok(None));
        assert_eq!(told(&mut server), ["Restored 18 blocks"]);
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
        server.command("Alex", "replace stone gold_block").unwrap();
        assert_eq!(
            told(&mut server),
            ["Replaced 1 minecraft:stone with minecraft:gold_block"]
        );
        assert!(server.block(BlockPos::new(1, 64, 1)).is("gold_block"));
        assert!(server.block(BlockPos::new(0, 64, 0)).is_air());
    }

    #[test]
    fn big_boxes_run_over_ticks_and_can_be_cancelled() {
        let mut server = server();
        server.command("Alex", "pos1 0 0 0").unwrap();
        server.command("Alex", "pos2 63 15 63").unwrap();
        assert_eq!(
            server.command("Alex", "set stone"),
            Ok(Some(
                "Working on 65536 blocks (6% done); /cancel stops it".into()
            ))
        );
        assert_eq!(
            server.command("Alex", "set dirt"),
            Err("Still working on the last edit (6% done); /cancel stops it".into())
        );
        server.ticks(3);
        assert_eq!(server.command("Alex", "cancel"), Ok(None));
        assert_eq!(
            told(&mut server),
            ["Set 16384 blocks to minecraft:stone before /cancel"]
        );
        assert_eq!(
            server.command("Alex", "cancel"),
            Err("Nothing to cancel".into())
        );

        server.command("Alex", "undo").unwrap();
        assert_eq!(server.finish_edits(), 4);
        assert_eq!(told(&mut server), ["Restored 16384 blocks"]);
        assert!(server.block(BlockPos::new(0, 0, 0)).is_air());

        server.command("Alex", "set stone").unwrap();
        assert_eq!(server.finish_edits(), 16);
        assert_eq!(told(&mut server), ["Set 65536 blocks to minecraft:stone"]);
        assert!(server.block(BlockPos::new(63, 15, 63)).is("stone"));
    }

    #[test]
    fn selections_are_per_player_and_limited() {
        let mut server = server();
        server.join("Steve");
        server.command("Alex", "pos1").unwrap();
        server.command("Alex", "pos2 ~100 ~100 ~100").unwrap();
        assert_eq!(
            server.command("Steve", "set stone"),
            Err("Select a box first with /pos1 and /pos2".into())
        );
        assert_eq!(
            server.command("Alex", "set stone"),
            Err("The box has 1030301 blocks; the most is 524288".into())
        );
        assert_eq!(
            server.command("Alex", "undo 2"),
            Err("Incorrect argument for command".into())
        );
    }
}
