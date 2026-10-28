use ferrite_api::input::{Clicks, ControlState, HeldItem};
use ferrite_api::prelude::*;
use ferrite_api::testing::{RegisteredKey, TestGame, World};

#[derive(Default)]
struct Zoom {
    menus: u32,
    zoomed_out: u32,
    zoom: f32,
    held_while_ticking: u32,
    aims: Vec<(String, u32)>,
}

impl Mod for Zoom {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .on_key("menu", "Open menu", "m", |m| m.menus += 1)
            .key("zoom", "Zoom", "c")
            .on_key_press("zoom", |_| input::capture_scroll(true))
            .on_key_release("zoom", Self::zoom_out)
            .on_scroll(|m, steps| m.zoom += steps)
            .on_tick(Self::tick)
            .on_item_use("spyglass", |m, using| {
                m.aims
                    .push((format!("start {}", using.item.id), using.ticks))
            })
            .on_item_release("minecraft:spyglass", |m, using| {
                m.aims.push(("stop".into(), using.ticks))
            });
        Ok(Self::default())
    }
}

impl Zoom {
    fn zoom_out(&mut self) {
        self.zoomed_out += 1;
        input::capture_scroll(false);
    }

    fn tick(&mut self, _tick: u64) {
        if input::is_down("zoom") {
            self.held_while_ticking += 1;
        }
    }
}

#[test]
fn keys_are_registered_with_minecraft_ids() {
    let game = TestGame::<Zoom>::start();
    assert_eq!(
        game.keys(),
        [
            RegisteredKey {
                name: "menu".into(),
                label: "Open menu".into(),
                key: "key.keyboard.m".into(),
            },
            RegisteredKey {
                name: "zoom".into(),
                label: "Zoom".into(),
                key: "key.keyboard.c".into(),
            },
        ]
    );
    assert_eq!(game.claimed_items(), ["minecraft:spyglass"]);
}

#[test]
fn presses_and_releases_reach_their_handlers() {
    let mut game = TestGame::<Zoom>::start();
    game.tap_key("menu").tap_key("menu");
    assert_eq!(game.state().menus, 2);

    game.press_key("zoom").ticks(3);
    assert_eq!(game.state().held_while_ticking, 3);
    game.release_key("zoom").ticks(2);
    assert_eq!(game.state().held_while_ticking, 3);
    assert_eq!(game.state().zoomed_out, 1);
}

#[test]
fn scrolling_reaches_the_mod_only_while_captured() {
    let mut game = TestGame::<Zoom>::start();
    game.scroll(1.0);
    assert_eq!(game.state().zoom, 0.0);

    game.press_key("zoom").scroll(1.0).scroll(-0.5);
    assert!(game.is_scroll_captured());
    assert_eq!(game.state().zoom, 0.5);

    game.release_key("zoom").scroll(4.0);
    assert!(!game.is_scroll_captured());
    assert_eq!(game.state().zoom, 0.5);
}

#[test]
fn claimed_items_report_use_and_how_long_it_lasted() {
    let mut game = TestGame::<Zoom>::start();
    game.start_using("minecraft:spyglass")
        .ticks(12)
        .stop_using();
    assert_eq!(
        game.state().aims,
        [
            ("start minecraft:spyglass".to_owned(), 0),
            ("stop".to_owned(), 12)
        ]
    );

    game.start_using("minecraft:bow").stop_using();
    assert_eq!(game.state().aims.len(), 2);
    assert_eq!(game.world().held_item.unwrap().id, "minecraft:bow");
}

struct Reader;

impl Mod for Reader {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks.command("hand", |_, _| {
            let held = input::held_item().map_or("nothing".to_owned(), |h| h.id);
            let jump = input::control(Control::Jump).unwrap();
            let clicks = input::clicks_per_second();
            reply(format!(
                "{held}, jump on {} {}, {} cps",
                jump.key, jump.down, clicks.attack
            ))
        });
        Ok(Self)
    }
}

#[test]
fn held_item_controls_and_clicks_come_from_the_world() {
    let mut game = TestGame::<Reader>::start_in(World {
        held_item: Some(HeldItem {
            id: "minecraft:compass".into(),
            count: 1,
            slot: 4,
        }),
        controls_down: vec![Control::Jump],
        clicks: Clicks {
            attack: 9,
            use_item: 0,
        },
        ..World::default()
    });
    assert_eq!(
        game.command("hand"),
        Ok(Some("minecraft:compass, jump on Space true, 9 cps".into()))
    );
    assert!(game.keys().is_empty());
    assert_eq!(
        input::control(Control::Sneak),
        Some(ControlState {
            key: "Left Shift".into(),
            down: false
        })
    );
}

struct Mistakes;

impl Mod for Mistakes {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .key("zoom", "Zoom", "c")
            .key("zoom", "Zoom again", "v")
            .on_key_press("zom", |_| {});
        Ok(Self)
    }
}

struct UnknownKey;

impl Mod for UnknownKey {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks.key("zoom", "Zoom", "key.joystick.a");
        Ok(Self)
    }
}

#[test]
fn mistakes_stop_the_mod_with_a_clear_message() {
    let err = TestGame::<Mistakes>::try_start_in(World::default())
        .err()
        .unwrap();
    assert_eq!(
        err,
        "key `zoom` is declared twice; on_key_press(\"zom\"): no key named `zom`; declare it first with hooks.key(\"zom\", \"Label\", \"c\") (declared: zoom)"
    );

    let err = TestGame::<UnknownKey>::try_start_in(World::default()).err();
    assert_eq!(
        err.as_deref(),
        Some("key `zoom` (key.joystick.a): unknown key key.joystick.a")
    );
}

#[test]
#[should_panic(expected = "the mod has no key `jump`; it declared [\"menu\", \"zoom\"]")]
fn pressing_an_undeclared_key_names_the_declared_ones() {
    TestGame::<Zoom>::start().press_key("jump");
}

#[test]
fn a_mod_without_input_never_polls() {
    let mut game = TestGame::<Reader>::start();
    game.scroll(1.0).tick();
    assert!(!game.is_scroll_captured());
}
