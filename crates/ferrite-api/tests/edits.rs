use ferrite_api::server::prelude::*;
use ferrite_api::server::world::{Edit, EditDone, EditId};
use ferrite_api::testing::{ServerWorld, TestServer};

#[derive(FromArgs)]
struct Size {
    side: i32,
}

#[derive(Default)]
struct Painter {
    last: Option<EditId>,
    undo: Option<Edit>,
    finished: Vec<(u64, bool, Option<String>)>,
}

impl ServerMod for Painter {
    fn init(hooks: &mut ServerHooks<Self>) -> Result<Self, Error> {
        hooks
            .queue_edits(100)
            .parsed_command("paint", |m, player, size: Size| {
                let far = BlockPos::new(size.side - 1, 63 + size.side, size.side - 1);
                let edit = Edit::fill(&player.dimension, BlockPos::new(0, 64, 0), far, "stone");
                m.start(edit.by(player).record_undo())
            })
            .parsed_command("swap", |m, player, size: Size| {
                let far = BlockPos::new(size.side - 1, 63 + size.side, size.side - 1);
                let edit = Edit::replace(
                    &player.dimension,
                    BlockPos::new(0, 64, 0),
                    far,
                    "stone",
                    "glass",
                );
                m.start(edit.by(player).record_undo())
            })
            .simple_command("checker", |m, player| {
                let edit = Edit::map(
                    &player.dimension,
                    BlockPos::new(0, 64, 0),
                    BlockPos::new(3, 64, 3),
                    |pos, old| {
                        ((pos.x + pos.z) % 2 == 0 && old.is_air()).then(|| "gold_block".into())
                    },
                );
                m.start(edit.by(player))
            })
            .simple_command("broken", |m, player| {
                let edit = Edit::fill(
                    &player.dimension,
                    BlockPos::new(0, 64, 0),
                    BlockPos::new(0, 64, 0),
                    "minecraft:NOPE",
                );
                m.start(edit.by(player))
            })
            .simple_command("undo", |m, _| {
                let edit = m.undo.take().ok_or("Nothing to undo")?;
                m.start(edit)
            })
            .simple_command("stop", |m, _| {
                let id = m.last.ok_or("Nothing running")?;
                if world::cancel(id) {
                    no_reply()
                } else {
                    Err("Already done".into())
                }
            })
            .simple_command("status", |m, _| {
                let id = m.last.ok_or("Nothing running")?;
                match world::progress(id) {
                    Some(p) => reply(format!("{}/{} ({}%)", p.done, p.total, p.percent())),
                    None => reply("done"),
                }
            })
            .on_edit_done(Self::done);
        Ok(Self::default())
    }
}

impl Painter {
    fn start(&mut self, edit: Edit) -> CommandResult {
        let size = edit.size();
        let id = edit.queue()?;
        self.last = Some(id);
        reply(format!("{id}: {size} blocks"))
    }

    fn done(&mut self, done: &mut EditDone) {
        self.finished
            .push((done.changed, done.cancelled, done.error.clone()));
        if let Some(undo) = done.take_undo() {
            self.undo = Some(undo);
        }
        if let Some(player) = &done.player {
            player.tell(format!("{} changed {}", done.id, done.changed));
        }
    }
}

struct Lazy;

impl ServerMod for Lazy {
    fn init(hooks: &mut ServerHooks<Self>) -> Result<Self, Error> {
        hooks.simple_command("paint", |_, player| {
            let pos = player.block_pos();
            Edit::fill(&player.dimension, pos, pos, "stone").queue()?;
            no_reply()
        });
        Ok(Self)
    }
}

fn painter() -> TestServer<Painter> {
    let mut server = TestServer::<Painter>::start();
    server.join("Alex");
    server.take_told();
    server
}

#[test]
fn small_edits_finish_inside_the_command() {
    let mut server = painter();
    assert_eq!(
        server.command("Alex", "paint 4"),
        Ok(Some("#1: 64 blocks".into()))
    );
    assert_eq!(
        server.take_told(),
        [("Alex".to_owned(), "#1 changed 64".to_owned())]
    );
    assert!(server.block(BlockPos::new(3, 67, 3)).is("stone"));
    assert_eq!(server.command("Alex", "status"), Ok(Some("done".into())));
}

#[test]
fn large_edits_spread_over_ticks_within_the_budget() {
    let mut server = painter();
    server.command("Alex", "paint 10").unwrap();
    assert_eq!(
        server.command("Alex", "status"),
        Ok(Some("100/1000 (10%)".into()))
    );
    assert!(server.take_told().is_empty());
    server.tick();
    assert_eq!(
        server.command("Alex", "status"),
        Ok(Some("200/1000 (20%)".into()))
    );
    assert_eq!(server.finish_edits(), 8);
    assert_eq!(
        server.take_told(),
        [("Alex".to_owned(), "#1 changed 1000".to_owned())]
    );
    assert!(server.block(BlockPos::new(9, 73, 9)).is("stone"));
}

#[test]
fn commands_in_one_tick_share_its_budget() {
    let mut server = painter();
    server.command("Alex", "paint 10").unwrap();
    server.command("Alex", "checker").unwrap();
    assert_eq!(
        server.command("Alex", "status"),
        Ok(Some("0/16 (0%)".into()))
    );
    server.tick();
    assert_eq!(server.command("Alex", "status"), Ok(Some("done".into())));
    assert_eq!(world::pending_edits(), 1);
    assert_eq!(server.finish_edits(), 9);
}

#[test]
fn undo_puts_back_exactly_what_was_there() {
    let mut server = TestServer::<Painter>::start_in(
        ServerWorld::default()
            .with_block(BlockPos::new(1, 64, 1), "oak_log[axis=x]")
            .with_block(BlockPos::new(2, 65, 2), "stone"),
    );
    server.join("Alex");
    server.command("Alex", "paint 5").unwrap();
    server.finish_edits();
    assert_eq!(
        server.state().finished,
        [(124, false, None)],
        "the stone already there is not counted"
    );
    server.command("Alex", "undo").unwrap();
    server.finish_edits();
    assert_eq!(
        server.block(BlockPos::new(1, 64, 1)).as_str(),
        "minecraft:oak_log[axis=x]"
    );
    assert!(server.block(BlockPos::new(2, 65, 2)).is("stone"));
    assert!(server.block(BlockPos::new(4, 68, 4)).is_air());
}

#[test]
fn replace_and_map_change_only_matching_blocks() {
    let mut server = TestServer::<Painter>::start_in(
        ServerWorld::default()
            .with_block(BlockPos::new(0, 64, 0), "stone")
            .with_block(BlockPos::new(1, 64, 0), "dirt")
            .with_block(BlockPos::new(2, 64, 0), "stone"),
    );
    server.join("Alex");
    server.command("Alex", "swap 3").unwrap();
    assert!(server.block(BlockPos::new(0, 64, 0)).is("glass"));
    assert!(server.block(BlockPos::new(1, 64, 0)).is("dirt"));
    assert!(server.block(BlockPos::new(2, 64, 0)).is("glass"));
    assert!(server.block(BlockPos::new(1, 65, 1)).is_air());

    server.command("Alex", "checker").unwrap();
    assert!(server.block(BlockPos::new(1, 64, 1)).is("gold_block"));
    assert!(server.block(BlockPos::new(0, 64, 1)).is_air());
    assert!(server.block(BlockPos::new(0, 64, 0)).is("glass"));
    assert_eq!(server.state().finished.last(), Some(&(6, false, None)));
}

#[test]
fn cancelled_edits_report_and_undo_what_they_did() {
    let mut server = painter();
    server.command("Alex", "paint 10").unwrap();
    server.tick().tick();
    assert_eq!(server.command("Alex", "stop"), Ok(None));
    assert_eq!(server.state().finished, [(300, true, None)]);
    assert_eq!(server.command("Alex", "stop"), Err("Already done".into()));
    server.command("Alex", "undo").unwrap();
    server.finish_edits();
    assert!(server.block(BlockPos::new(0, 64, 0)).is_air());
    assert!(server.block(BlockPos::new(9, 66, 9)).is_air());
}

#[test]
fn bad_states_are_refused_by_the_command() {
    let mut server = painter();
    assert_eq!(
        server.command("Alex", "broken"),
        Err("unknown block state minecraft:NOPE".into())
    );
    assert_eq!(world::pending_edits(), 0);
}

#[test]
fn queueing_without_a_budget_explains_what_is_missing() {
    let mut server = TestServer::<Lazy>::start();
    server.join("Alex");
    assert_eq!(
        server.command("Alex", "paint"),
        Err(
            "queued edits need hooks.queue_edits(blocks_per_tick) or hooks.on_edit_done(..) in init"
                .into()
        )
    );
}
