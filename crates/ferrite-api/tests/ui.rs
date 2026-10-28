use std::fmt;

use ferrite_api::__private::{Adapter, SCREEN_WIDGET_IDS, take_ui_calls};
use ferrite_api::prelude::*;
use ferrite_api::ui::{HostCall, Widget};

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

fn settings(visible: &str, mode: &str) -> HostCall {
    HostCall::Settings(vec![
        button(&format!("Visible: {visible}")),
        button(&format!("Mode: {mode}")),
    ])
}

fn screen(selected: Option<u32>) -> HostCall {
    HostCall::OpenScreen {
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

fn start() -> Adapter<Menu> {
    let (adapter, _) = Adapter::<Menu>::init().unwrap();
    adapter
}

#[test]
fn settings_page_is_pushed_at_load_and_follows_clicks() {
    let mut adapter = start();
    assert_eq!(take_ui_calls(), [settings("OFF", "Fast")]);

    adapter.ui_select(1, 0);
    adapter.ui_select(0, 0);
    assert_eq!(
        take_ui_calls(),
        [settings("OFF", "Slow"), settings("ON", "Slow")]
    );
    assert!(adapter.inner().visible);
    assert_eq!(adapter.inner().mode, Mode::Slow);

    adapter.ui_settings_opened();
    assert_eq!(take_ui_calls(), [settings("ON", "Slow")]);

    adapter.ui_select(7, 0);
    assert_eq!(take_ui_calls(), []);
}

#[test]
fn refresh_pushes_only_changes() {
    let mut adapter = start();
    take_ui_calls();

    adapter.command("flip", "").unwrap();
    assert_eq!(take_ui_calls(), [settings("ON", "Fast")]);

    ui::refresh();
    adapter.tick(0);
    assert_eq!(take_ui_calls(), []);
}

#[test]
fn screens_open_update_and_close() {
    let mut adapter = start();
    take_ui_calls();

    adapter.command("menu", "").unwrap();
    assert_eq!(take_ui_calls(), [screen(None)]);

    adapter.ui_select(SCREEN_WIDGET_IDS, 1);
    assert_eq!(adapter.inner().picks, ["blue"]);
    assert_eq!(take_ui_calls(), [screen(Some(1))]);

    adapter.ui_select(SCREEN_WIDGET_IDS + 1, 0);
    assert_eq!(take_ui_calls(), [HostCall::CloseScreen]);
    assert_eq!(adapter.inner().closed, 0);

    adapter.ui_select(SCREEN_WIDGET_IDS, 0);
    assert_eq!(adapter.inner().picks, ["blue"]);
}

#[test]
fn player_closing_the_screen_calls_the_hook() {
    let mut adapter = start();
    adapter.command("menu", "").unwrap();
    take_ui_calls();

    adapter.ui_closed();
    assert_eq!(adapter.inner().closed, 1);
    assert_eq!(take_ui_calls(), []);

    adapter.ui_closed();
    assert_eq!(adapter.inner().closed, 1);
}
