use std::fmt;

use ferrite_api::prelude::*;
use ferrite_api::testing::{Screen, TestGame};
use ferrite_api::ui::Widget;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Fast,
    Slow,
}

impl Mode {
    const ALL: [Self; 2] = [Self::Fast, Self::Slow];
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Fast => "Fast",
            Self::Slow => "Slow",
        })
    }
}

#[derive(Default)]
struct Menu {
    visible: bool,
    mode: Mode,
    picks: Vec<&'static str>,
    closed: u32,
    builds: std::cell::Cell<u32>,
}

impl Mod for Menu {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .settings(Self::settings)
            .on_screen_closed(|m| m.closed += 1)
            .command("menu", Self::menu)
            .command("flip", Self::flip);
        Ok(Self::default())
    }
}

impl Menu {
    fn settings(&self, page: &mut Page<Self>) {
        self.builds.set(self.builds.get() + 1);
        page.toggle("Visible", self.visible, |m, on| m.visible = on)
            .cycle("Mode", &Mode::ALL, &self.mode, |m, mode| m.mode = mode);
    }

    fn screen(&self, page: &mut Page<Self>) {
        page.set_title("Pick")
            .list(&["red", "blue"], self.picks.last(), |m, color| {
                m.picks.push(color)
            })
            .button("Done", |_| ui::close());
    }

    fn menu(&mut self, _args: Args) -> CommandResult {
        ui::open(Self::screen);
        Ok(None)
    }

    fn flip(&mut self, _args: Args) -> CommandResult {
        self.visible = !self.visible;
        ui::refresh();
        Ok(None)
    }
}

fn button(label: &str) -> Widget {
    Widget::Button {
        label: label.into(),
    }
}

fn settings(visible: &str, mode: &str) -> Vec<Widget> {
    vec![
        button(&format!("Visible: {visible}")),
        button(&format!("Mode: {mode}")),
    ]
}

fn screen(selected: Option<u32>) -> Screen {
    Screen {
        title: "Pick".into(),
        widgets: vec![
            Widget::List {
                rows: vec!["red".into(), "blue".into()],
                selected,
            },
            button("Done"),
        ],
    }
}

#[test]
fn settings_page_is_shown_at_load_and_follows_clicks() {
    let mut game = TestGame::<Menu>::start();
    assert_eq!(game.settings(), settings("OFF", "Fast"));

    game.click_setting("Mode").click_setting("Visible");
    assert_eq!(game.settings(), settings("ON", "Slow"));
    assert!(game.state().visible);
    assert_eq!(game.state().mode, Mode::Slow);

    game.open_settings();
    assert_eq!(game.settings(), settings("ON", "Slow"));
}

#[test]
fn refresh_rebuilds_after_changes_made_elsewhere() {
    let mut game = TestGame::<Menu>::start();
    game.command("flip").unwrap();
    assert_eq!(game.settings(), settings("ON", "Fast"));

    let builds = game.state().builds.get();
    game.tick();
    assert_eq!(game.state().builds.get(), builds);
}

#[test]
fn screens_open_update_and_close() {
    let mut game = TestGame::<Menu>::start();
    assert_eq!(game.screen(), None);

    game.command("/menu").unwrap();
    assert_eq!(game.screen(), Some(screen(None)));

    game.pick("blue");
    assert_eq!(game.state().picks, ["blue"]);
    assert_eq!(game.screen(), Some(screen(Some(1))));

    game.click("Done");
    assert_eq!(game.screen(), None);
    assert_eq!(game.state().closed, 0);
}

#[test]
fn player_closing_the_screen_calls_the_hook() {
    let mut game = TestGame::<Menu>::start();
    game.command("menu").unwrap();

    game.close_screen();
    assert_eq!(game.state().closed, 1);
    game.close_screen();
    assert_eq!(game.state().closed, 1);
}

#[test]
#[should_panic(expected = "no button `Speed` on the settings page:\n  button `Visible: OFF`")]
fn missing_widgets_are_reported_with_what_is_there() {
    TestGame::<Menu>::start().click_setting("Speed");
}
