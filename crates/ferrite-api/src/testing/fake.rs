use std::cell::RefCell;

use super::{EYE_HEIGHT, RegisteredKey, Screen, World};
use crate::game::{Clock, Hit, Player, Vec3};
use crate::hud::{Corner, Line, Panel};
use crate::input::{Clicks, Control, ControlState, Event, HeldItem};
use crate::log::Level;
use crate::ui::Widget;

#[derive(Default)]
pub(super) struct Fake {
    pub(super) world: World,
    pub(super) log: Vec<(Level, String)>,
    pub(super) chat: Vec<String>,
    pub(super) commands: Vec<String>,
    pub(super) quiet_commands: Vec<String>,
    pub(super) panel: Option<Panel>,
    pub(super) settings: Vec<Widget>,
    pub(super) screen: Option<Screen>,
    pub(super) keys: Vec<RegisteredKey>,
    pub(super) claimed_items: Vec<String>,
    pub(super) scroll_captured: bool,
    pub(super) events: Vec<Event>,
}

pub(crate) fn input_register_key(name: &str, label: &str, key: &str) -> Result<(), String> {
    let known = ["key.keyboard.", "key.mouse."].iter().any(|prefix| {
        key.strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    });
    if !known {
        return Err(format!("unknown key {key}"));
    }
    with(|f| {
        f.keys.push(RegisteredKey {
            name: name.to_owned(),
            label: label.to_owned(),
            key: key.to_owned(),
        })
    });
    Ok(())
}

pub(crate) fn input_claim_items(items: &[String]) {
    with(|f| f.claimed_items = items.to_vec());
}

pub(crate) fn input_claim_wheel(on: bool) {
    with(|f| f.scroll_captured = on);
}

pub(crate) fn input_poll() -> Vec<Event> {
    with(|f| std::mem::take(&mut f.events))
}

pub(crate) fn input_held() -> Option<HeldItem> {
    with(|f| f.world.held_item.clone())
}

pub(crate) fn input_mapping(name: &str) -> Option<ControlState> {
    let control = Control::ALL.into_iter().find(|c| c.id() == name)?;
    let key = match control {
        Control::Forward => "W",
        Control::Left => "A",
        Control::Back => "S",
        Control::Right => "D",
        Control::Jump => "Space",
        Control::Sneak => "Left Shift",
        Control::Sprint => "Left Control",
        Control::Attack => "LMB",
        Control::Use => "RMB",
    };
    with(|f| {
        Some(ControlState {
            key: key.to_owned(),
            down: f.world.controls_down.contains(&control),
        })
    })
}

pub(crate) fn input_clicks() -> Clicks {
    with(|f| f.world.clicks)
}

thread_local! {
    static FAKE: RefCell<Fake> = RefCell::new(Fake::default());
}

pub(super) fn with<R>(f: impl FnOnce(&mut Fake) -> R) -> R {
    FAKE.with(|fake| f(&mut fake.borrow_mut()))
}

pub(crate) fn log(level: Level, message: &str) {
    with(|f| f.log.push((level, message.to_owned())));
}

pub(crate) fn forget_snapshot() {}

pub(crate) fn chat_show(text: &str) {
    with(|f| f.chat.push(text.to_owned()));
}

pub(crate) fn command_run(line: &str) {
    with(|f| f.commands.push(line.to_owned()));
}

pub(crate) fn command_run_quiet(lines: &[String]) {
    with(|f| f.quiet_commands.extend_from_slice(lines));
}

pub(crate) fn player() -> Option<Player> {
    with(|f| f.world.player.clone())
}

pub(crate) fn clock() -> Option<Clock> {
    with(|f| f.world.clock)
}

pub(crate) fn raycast(distance: f32) -> Option<Hit> {
    with(|f| {
        let eye = f.world.player.as_ref()?.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
        let hit = f.world.hit.as_ref()?;
        (eye.distance(hit.point) <= f64::from(distance)).then(|| hit.clone())
    })
}

pub(crate) fn hud_set_panel(corner: Corner, lines: &[Line]) {
    let mut panel = Panel::new(corner);
    for line in lines {
        panel.push(line.clone());
    }
    with(|f| f.panel = Some(panel));
}

pub(crate) fn hud_clear() {
    with(|f| f.panel = None);
}

pub(crate) fn storage_get(key: &str) -> Option<Vec<u8>> {
    with(|f| f.world.storage.get(key).cloned())
}

pub(crate) fn storage_set(key: &str, value: &[u8]) -> Result<(), String> {
    with(|f| {
        let others: usize = f
            .world
            .storage
            .iter()
            .filter(|(k, _)| *k != key)
            .map(|(_, v)| v.len())
            .sum();
        if f.world
            .storage_quota
            .is_some_and(|quota| others + value.len() > quota)
        {
            return Err("over quota".to_owned());
        }
        f.world.storage.insert(key.to_owned(), value.to_vec());
        Ok(())
    })
}

pub(crate) fn storage_delete(key: &str) {
    with(|f| f.world.storage.remove(key));
}

pub(crate) fn ui_set_settings(widgets: &[Widget]) {
    with(|f| f.settings = widgets.to_vec());
}

pub(crate) fn ui_open_screen(title: &str, widgets: &[Widget]) {
    with(|f| {
        f.screen = Some(Screen {
            title: title.to_owned(),
            widgets: widgets.to_vec(),
        })
    });
}

pub(crate) fn ui_close_screen() {
    with(|f| f.screen = None);
}
