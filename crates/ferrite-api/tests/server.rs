use ferrite_api::server::prelude::*;
use ferrite_api::server::{ArgumentKind, ArgumentSpec};
use ferrite_api::testing::{ServerWorld, TestServer};

#[derive(FromArgs)]
struct Pillar {
    height: u8,
    block: Option<BlockState>,
}

#[derive(FromArgs)]
struct Mark {
    at: BlockPos,
    #[rest]
    note: Option<String>,
}

#[derive(FromArgs)]
struct Greet {
    who: Player,
    loud: Option<bool>,
}

#[derive(FromArgs)]
enum Homes {
    #[default]
    List,
    Set {
        name: String,
    },
}

#[derive(Default)]
struct Builder {
    joined: Vec<String>,
    broken: Vec<String>,
    homes: Vec<String>,
    marks: Vec<(BlockPos, Option<String>)>,
}

impl ServerMod for Builder {
    fn init(hooks: &mut ServerHooks<Self>) -> Result<Self, Error> {
        hooks
            .parsed_command("pillar", Self::pillar)
            .parsed_command("mark", |m, _, mark: Mark| {
                m.marks.push((mark.at, mark.note));
                no_reply()
            })
            .parsed_command("greet", Self::greet)
            .parsed_command("homes", Self::homes)
            .command("echo", |_, player, args| {
                reply(format!("{}: {}", player.name, args.as_str()))
            })
            .command("big", Self::big)
            .on_join(|m, player| {
                m.joined.push(player.name.clone());
                player.tell(format!("Welcome, {}!", player.name));
            })
            .on_leave(|_, player| players::broadcast(format!("{} left", player.name)))
            .on_block_break(Self::broken);
        Ok(Self::default())
    }
}

impl Builder {
    fn pillar(&mut self, player: &Player, pillar: Pillar) -> CommandResult {
        let base = player.block_pos().offset(2, 0, 0);
        let block = pillar
            .block
            .unwrap_or_else(|| BlockState::from("glowstone"));
        let placed = world::set_blocks(
            &player.dimension,
            (0..i32::from(pillar.height)).map(|dy| (base.offset(0, dy, 0), block.clone())),
        )?;
        let now = world::block(&player.dimension, base).unwrap();
        reply(format!(
            "{placed} blocks, base was {} during the command",
            now.id()
        ))
    }

    fn greet(&mut self, _player: &Player, greet: Greet) -> CommandResult {
        let text = format!("Hi {}", greet.who.name);
        greet.who.tell(if greet.loud == Some(true) {
            text.to_uppercase()
        } else {
            text
        });
        no_reply()
    }

    fn homes(&mut self, _player: &Player, homes: Homes) -> CommandResult {
        match homes {
            Homes::List => reply(self.homes.join(", ")),
            Homes::Set { name } => {
                self.homes.push(name);
                no_reply()
            }
        }
    }

    fn big(&mut self, player: &Player, _args: Args) -> CommandResult {
        let base = player.block_pos();
        let changed = world::set_blocks(
            &player.dimension,
            BlockPos::cuboid(base, base.offset(9, 49, 9)).map(|pos| (pos, "stone")),
        )?;
        reply(changed.to_string())
    }

    fn broken(&mut self, player: &Player, pos: BlockPos, state: &BlockState) {
        self.broken
            .push(format!("{} broke {} at {pos}", player.name, state.id()));
        if state.is("diamond_ore") {
            let _ = world::set_block(&player.dimension, pos, "bedrock");
        }
    }
}

#[test]
fn typed_arguments_are_derived_from_the_struct() {
    assert_eq!(
        <Pillar as FromArgs>::server_arguments().unwrap(),
        [
            ArgumentSpec::new(
                "height",
                ArgumentKind::Integer {
                    min: Some(0),
                    max: Some(255)
                },
                false
            ),
            ArgumentSpec::new("block", ArgumentKind::Word, true),
        ]
    );
    assert_eq!(
        <Mark as FromArgs>::server_arguments().unwrap(),
        [
            ArgumentSpec::new("at", ArgumentKind::BlockPos, false),
            ArgumentSpec::new("note", ArgumentKind::Text, true),
        ]
    );
    assert!(<Homes as FromArgs>::server_arguments().is_none());
}

#[test]
fn typed_commands_build_and_changes_land_after_the_command() {
    let mut server = TestServer::<Builder>::start();
    server.join("Alex");
    assert_eq!(
        server.command("Alex", "pillar 3"),
        Ok(Some(
            "3 blocks, base was minecraft:air during the command".into()
        ))
    );
    assert!(server.block(BlockPos::new(2, 64, 0)).is("glowstone"));
    assert!(server.block(BlockPos::new(2, 66, 0)).is("glowstone"));
    assert!(server.block(BlockPos::new(2, 67, 0)).is_air());

    server.command("Alex", "/pillar 1 gold_block").unwrap();
    assert!(server.block(BlockPos::new(2, 64, 0)).is("gold_block"));
}

#[test]
fn the_server_checks_arguments_before_the_mod() {
    let mut server = TestServer::<Builder>::start();
    server.join("Alex");
    assert_eq!(
        server.command("Alex", "pillar 300"),
        Err("Integer must not be more than 255, found 300".into())
    );
    assert_eq!(
        server.command("Alex", "pillar"),
        Err("Unknown or incomplete command, see below for error".into())
    );
    assert_eq!(
        server.command("Alex", "pillar 2 minecraft:stone"),
        Err("Incorrect argument for command".into())
    );
    assert_eq!(
        server.command("Alex", "greet Herobrine"),
        Err("No player was found".into())
    );
}

#[test]
fn positions_and_players_resolve_like_vanilla() {
    let mut server = TestServer::<Builder>::start();
    server
        .join("Alex")
        .join("Steve")
        .update_player("Alex", |p| p.position = Vec3::new(10.5, 70.0, -3.5));
    server.command("Alex", "mark ~1 ~ ~-2 by the tree").unwrap();
    server.command("Alex", "mark 0 64 0").unwrap();
    assert_eq!(
        server.state().marks,
        [
            (BlockPos::new(11, 70, -6), Some("by the tree".to_owned())),
            (BlockPos::new(0, 64, 0), None),
        ]
    );

    server.take_told();
    server.command("Alex", "greet steve true").unwrap();
    server.command("Alex", "greet @s").unwrap();
    assert_eq!(
        server.take_told(),
        [
            ("Steve".to_owned(), "HI STEVE".to_owned()),
            ("Alex".to_owned(), "Hi Alex".to_owned()),
        ]
    );
}

#[test]
fn subcommands_and_free_text_parse_the_line() {
    let mut server = TestServer::<Builder>::start();
    server.join("Alex");
    server.command("Alex", "homes set base").unwrap();
    server.command("Alex", "homes set mine").unwrap();
    assert_eq!(
        server.command("Alex", "homes"),
        Ok(Some("base, mine".into()))
    );
    assert_eq!(
        server.command("Alex", "homes fly"),
        Err("Unknown option `fly`. Usage: /homes list | set <name>".into())
    );
    assert_eq!(
        server.command("Alex", "echo  hello   there "),
        Ok(Some("Alex: hello   there".into()))
    );
}

#[test]
fn joins_leaves_and_messages() {
    let mut server = TestServer::<Builder>::start();
    server.join("Alex").join("Steve").leave("Alex");
    assert_eq!(server.state().joined, ["Alex", "Steve"]);
    assert_eq!(
        server.take_told(),
        [
            ("Alex".to_owned(), "Welcome, Alex!".to_owned()),
            ("Steve".to_owned(), "Welcome, Steve!".to_owned()),
        ]
    );
    assert_eq!(server.take_broadcasts(), ["Alex left"]);
}

#[test]
fn block_events_see_what_was_there() {
    let mut server = TestServer::<Builder>::start_in(
        ServerWorld::default()
            .with_block(BlockPos::new(1, 60, 1), "minecraft:oak_log[axis=y]")
            .with_block(BlockPos::new(2, 60, 1), "diamond_ore"),
    );
    server
        .join("Alex")
        .break_block("Alex", BlockPos::new(1, 60, 1))
        .break_block("Alex", BlockPos::new(2, 60, 1));
    assert_eq!(
        server.state().broken,
        [
            "Alex broke minecraft:oak_log at 1 60 1",
            "Alex broke minecraft:diamond_ore at 2 60 1"
        ]
    );
    assert!(server.block(BlockPos::new(1, 60, 1)).is_air());
    assert!(server.block(BlockPos::new(2, 60, 1)).is("bedrock"));
}

#[test]
fn large_edits_are_split_into_host_sized_batches() {
    let mut server = TestServer::<Builder>::start();
    server.join("Alex");
    assert_eq!(server.command("Alex", "big"), Ok(Some("5000".into())));
    assert!(server.block(BlockPos::new(9, 113, 9)).is("stone"));
}
