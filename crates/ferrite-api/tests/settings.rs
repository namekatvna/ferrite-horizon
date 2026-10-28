use ferrite_api::prelude::*;
use ferrite_api::testing::{TestGame, World};
use ferrite_api::ui::Widget;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Choice)]
enum Speed {
    #[default]
    Slow,
    VeryFast,
    #[choice(label = "Ludicrous!")]
    Ludicrous,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Settings)]
struct Options {
    show_coords: bool,
    corner: Corner,
    speed: Speed,
    #[setting(range = 4..=16, step = 4)]
    radius: u32,
    #[setting(options = ["red", "blue"], label = "Marker color")]
    color: String,
    #[setting(skip)]
    homes: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            show_coords: true,
            corner: Corner::TopLeft,
            speed: Speed::Slow,
            radius: 8,
            color: "red".into(),
            homes: vec!["base".into()],
        }
    }
}

struct Saved {
    options: Config<Options>,
}

impl Mod for Saved {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks.settings(|m, page| {
            page.config(&m.options, |m| &mut m.options)
                .button("Reset", |m| m.options.edit(|o| *o = Options::default()));
        });
        Ok(Self {
            options: Config::load(),
        })
    }
}

#[derive(Default)]
struct Session {
    options: Options,
}

impl Mod for Session {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks.settings(|m, page| {
            page.text("Not saved")
                .fields(&m.options, |m| &mut m.options);
        });
        Ok(Self::default())
    }
}

fn labels(game: &TestGame<impl Mod>) -> Vec<String> {
    game.settings()
        .into_iter()
        .map(|w| match w {
            Widget::Button { label } => label,
            Widget::Text { text, .. } => text,
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn every_field_gets_a_widget_with_a_readable_label() {
    let game = TestGame::<Saved>::start();
    assert_eq!(
        labels(&game),
        [
            "Show coords: ON",
            "Corner: Top left",
            "Speed: Slow",
            "Radius: 8",
            "Marker color: red",
            "Reset",
        ]
    );
}

#[test]
fn clicks_cycle_values_and_save_them() {
    let mut game = TestGame::<Saved>::start();
    game.click_setting("Show coords")
        .click_setting("Corner")
        .click_setting("Speed")
        .click_setting("Speed")
        .click_setting("Radius")
        .click_setting("Radius")
        .click_setting("Marker color");
    assert_eq!(
        labels(&game),
        [
            "Show coords: OFF",
            "Corner: Top right",
            "Speed: Ludicrous!",
            "Radius: 16",
            "Marker color: blue",
            "Reset",
        ]
    );

    game.restart();
    let options = &game.state().options;
    assert!(!options.show_coords);
    assert_eq!(options.speed, Speed::Ludicrous);
    assert_eq!(options.radius, 16);
    assert_eq!(options.homes, ["base"]);

    game.click_setting("Radius").click_setting("Speed");
    assert_eq!(game.state().options.radius, 4);
    assert_eq!(game.state().options.speed, Speed::Slow);
}

#[test]
fn a_value_outside_the_options_starts_over() {
    let game = TestGame::<Saved>::start_in(
        World::default().with_file("config.json", r#"{ "radius": 7, "color": "green" }"#),
    );
    assert!(labels(&game).contains(&"Radius: 7".to_owned()));
    let mut game = game;
    game.click_setting("Radius").click_setting("Marker color");
    assert_eq!(game.state().options.radius, 4);
    assert_eq!(game.state().options.color, "red");
}

#[test]
fn reset_restores_the_defaults() {
    let mut game = TestGame::<Saved>::start();
    game.click_setting("Show coords").click_setting("Reset");
    assert_eq!(*game.state().options.get(), Options::default());
}

#[test]
fn plain_structs_work_without_saving() {
    let mut game = TestGame::<Session>::start();
    game.click_setting("Corner");
    assert_eq!(game.state().options.corner, Corner::TopRight);
    assert_eq!(labels(&game)[0], "Not saved");
    assert!(game.world().storage.is_empty());
}
