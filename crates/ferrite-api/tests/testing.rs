use ferrite_api::game::{BlockPos, Hit, Vec3};
use ferrite_api::log::Level;
use ferrite_api::prelude::*;
use ferrite_api::testing::{TestGame, World};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Settings {
    corner: Corner,
    greeting: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            corner: Corner::TopLeft,
            greeting: "Hello".into(),
        }
    }
}

struct Probe {
    settings: Config<Settings>,
    heard: Vec<String>,
    frames: u32,
}

impl Mod for Probe {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .on_tick(Self::tick)
            .on_frame(|m, _| m.frames += 1)
            .on_chat(|m, text| m.heard.push(text.to_owned()))
            .command("greet", Self::greet)
            .command("corner", Self::corner)
            .command("look", Self::look)
            .command("build", Self::build);
        log::info("probe: loaded");
        Ok(Self {
            settings: Config::load(),
            heard: Vec::new(),
            frames: 0,
        })
    }
}

impl Probe {
    fn tick(&mut self, _tick: u64) {
        match (game::player(), game::clock()) {
            (Some(player), Some(clock)) => {
                let (hours, minutes) = clock.hours_minutes();
                Panel::new(self.settings.corner)
                    .line(format!("XYZ {}", player.block_pos()))
                    .line(format!("{hours:02}:{minutes:02}"))
                    .show();
            }
            _ => hud::clear(),
        }
    }

    fn greet(&mut self, args: Args) -> CommandResult {
        chat::show(format!(
            "{}, {}!",
            self.settings.greeting,
            args.word(0).unwrap_or("world")
        ));
        Ok(None)
    }

    fn corner(&mut self, args: Args) -> CommandResult {
        let corner: Corner = args.get(0)?;
        self.settings.update(|s| s.corner = corner)?;
        Ok(Some(format!("Moved to {corner}")))
    }

    fn look(&mut self, _args: Args) -> CommandResult {
        Ok(Some(match game::raycast(5.0) {
            Some(hit) => format!("{} at {}", hit.block, hit.block_pos),
            None => "nothing".into(),
        }))
    }

    fn build(&mut self, _args: Args) -> CommandResult {
        command::run("/time set day");
        command::run_quiet(["/setblock 0 64 0 stone", "setblock 1 64 0 stone"]);
        Ok(None)
    }
}

struct Refuses;

impl Mod for Refuses {
    fn init(_hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        Err("needs a server".into())
    }
}

#[test]
fn a_new_game_matches_horizons_sandbox_defaults() {
    let mut game = TestGame::<Probe>::start();
    assert_eq!(game.commands(), ["greet", "corner", "look", "build"]);
    assert_eq!(game.take_log(), [(Level::Info, "probe: loaded".to_owned())]);

    game.tick();
    assert_eq!(game.panel_lines(), ["XYZ 0 64 0", "06:00"]);
    assert_eq!(game.panel().unwrap().corner(), Corner::TopLeft);
    assert_eq!(game.world().clock.unwrap().game_time, 1);
}

#[test]
fn the_world_can_be_changed_between_ticks() {
    let mut game = TestGame::<Probe>::start();
    game.update_world(|w| {
        let player = w.player.as_mut().unwrap();
        player.position = Vec3::new(-10.5, 70.0, 3.2);
        w.clock.as_mut().unwrap().day_time = 17_999;
    })
    .tick();
    assert_eq!(game.panel_lines(), ["XYZ -11 70 3", "00:00"]);

    game.update_world(|w| *w = World::title_screen()).tick();
    assert_eq!(game.panel(), None);
}

#[test]
fn chat_commands_and_frames_are_recorded() {
    let mut game = TestGame::<Probe>::start();
    assert_eq!(game.command("greet Steve"), Ok(None));
    assert_eq!(game.take_chat(), ["Hello, Steve!"]);
    assert!(game.take_chat().is_empty());

    game.command("build").unwrap();
    assert_eq!(game.take_commands(), ["time set day"]);
    assert_eq!(
        game.take_quiet_commands(),
        ["setblock 0 64 0 stone", "setblock 1 64 0 stone"]
    );

    game.receive_chat("<Alex> hi").frame(0.016).frame(0.016);
    assert_eq!(game.state().heard, ["<Alex> hi"]);
    assert_eq!(game.state().frames, 2);

    assert_eq!(
        game.command("nope"),
        Err("Unknown command /nope".to_owned())
    );
}

#[test]
fn raycast_sees_the_hit_within_reach() {
    let hit = Hit {
        block_pos: BlockPos::new(0, 65, 3),
        point: Vec3::new(0.5, 65.62, 3.0),
        block: "minecraft:stone".into(),
    };
    let mut game = TestGame::<Probe>::start_in(World {
        hit: Some(hit),
        ..World::default()
    });
    assert_eq!(
        game.command("look"),
        Ok(Some("minecraft:stone at 0 65 3".into()))
    );

    game.update_world(|w| w.player.as_mut().unwrap().position.z = -10.0);
    assert_eq!(game.command("look"), Ok(Some("nothing".into())));
}

#[test]
fn config_lives_in_storage_and_survives_a_restart() {
    let mut game = TestGame::<Probe>::start_in(
        World::default().with_file("config.json", r#"{ "greeting": "Hi" }"#),
    );
    game.command("greet").unwrap();
    assert_eq!(game.take_chat(), ["Hi, world!"]);
    assert!(
        game.file_text("config.json")
            .unwrap()
            .contains("\"corner\": \"top-left\"")
    );

    assert_eq!(
        game.command("corner bottom-right"),
        Ok(Some("Moved to bottom-right".into()))
    );
    game.tick();
    assert!(game.panel().is_some());

    game.restart();
    assert_eq!(game.panel(), None);
    game.tick();
    assert_eq!(game.panel().unwrap().corner(), Corner::BottomRight);
}

#[test]
fn storage_quota_is_enforced() {
    let mut game = TestGame::<Probe>::start_in(World {
        storage_quota: Some(16),
        ..World::default()
    });
    assert!(game.take_log().iter().any(|(level, message)| {
        *level == Level::Warn && message.contains("could not create config.json")
    }));
    assert!(game.command("corner top-right").is_err());
}

#[test]
fn a_refused_start_reports_the_reason() {
    let err = TestGame::<Refuses>::try_start_in(World::default()).err();
    assert_eq!(err.as_deref(), Some("needs a server"));
}
