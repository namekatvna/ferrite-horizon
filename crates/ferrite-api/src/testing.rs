use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::__private::{Adapter, sdk::Registration};
use crate::Mod;
use crate::game::{Clock, Dimension, Hit, Player, Vec3};
use crate::hud::Panel;
use crate::input::{self, Clicks, Control, Event, HeldItem, ItemUse};
use crate::log::Level;
use crate::ui::{Widget, widget_id};

pub(crate) mod fake;

use fake::{Fake, with};

pub const EYE_HEIGHT: f64 = 1.62;

#[derive(Clone, Debug, PartialEq)]
pub struct World {
    pub player: Option<Player>,
    pub clock: Option<Clock>,
    pub hit: Option<Hit>,
    pub storage: BTreeMap<String, Vec<u8>>,
    pub storage_quota: Option<usize>,
    pub held_item: Option<HeldItem>,
    pub controls_down: Vec<Control>,
    pub clicks: Clicks,
}

impl Default for World {
    fn default() -> Self {
        Self {
            player: Some(default_player()),
            clock: Some(Clock::default()),
            hit: None,
            storage: BTreeMap::new(),
            storage_quota: None,
            held_item: None,
            controls_down: Vec::new(),
            clicks: Clicks::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredKey {
    pub name: String,
    pub label: String,
    pub key: String,
}

impl World {
    pub fn title_screen() -> Self {
        Self {
            player: None,
            clock: None,
            ..Self::default()
        }
    }

    pub fn with_file(mut self, key: &str, contents: impl AsRef<[u8]>) -> Self {
        self.storage
            .insert(key.to_owned(), contents.as_ref().to_vec());
        self
    }
}

pub fn default_player() -> Player {
    Player {
        position: Vec3::new(0.5, 64.0, 0.5),
        yaw: 0.0,
        pitch: 0.0,
        health: Player::MAX_HEALTH,
        food: Player::MAX_FOOD,
        dimension: Dimension::Overworld,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    pub title: String,
    pub widgets: Vec<Widget>,
}

pub struct TestGame<M: Mod> {
    adapter: Adapter<M>,
    registration: Registration,
    tick: u64,
    using: Option<(HeldItem, u64)>,
}

impl<M: Mod> TestGame<M> {
    pub fn start() -> Self {
        Self::start_in(World::default())
    }

    pub fn start_in(world: World) -> Self {
        Self::try_start_in(world).unwrap_or_else(|e| panic!("the mod refused to start: {e}"))
    }

    pub fn try_start_in(world: World) -> Result<Self, String> {
        with(|f| {
            *f = Fake {
                world,
                ..Fake::default()
            }
        });
        let (adapter, registration) = boot::<M>()?;
        Ok(Self {
            adapter,
            registration,
            tick: 0,
            using: None,
        })
    }

    pub fn restart(&mut self) -> &mut Self {
        if let Err(e) = self.try_restart() {
            panic!("the mod refused to restart: {e}");
        }
        self
    }

    pub fn try_restart(&mut self) -> Result<(), String> {
        with(|f| {
            f.panel = None;
            f.settings.clear();
            f.screen = None;
            f.keys.clear();
            f.claimed_items.clear();
            f.scroll_captured = false;
            f.events.clear();
        });
        self.using = None;
        (self.adapter, self.registration) = boot::<M>()?;
        Ok(())
    }

    pub fn keys(&self) -> Vec<RegisteredKey> {
        with(|f| f.keys.clone())
    }

    pub fn claimed_items(&self) -> Vec<String> {
        with(|f| f.claimed_items.clone())
    }

    pub fn is_scroll_captured(&self) -> bool {
        with(|f| f.scroll_captured)
    }

    pub fn press_key(&mut self, name: &str) -> &mut Self {
        self.key_event(name, Event::KeyDown(name.to_owned()))
    }

    pub fn release_key(&mut self, name: &str) -> &mut Self {
        self.key_event(name, Event::KeyUp(name.to_owned()))
    }

    pub fn tap_key(&mut self, name: &str) -> &mut Self {
        self.press_key(name).release_key(name)
    }

    pub fn scroll(&mut self, steps: f32) -> &mut Self {
        if self.is_scroll_captured() {
            self.send(Event::Scroll(steps));
        }
        self
    }

    pub fn start_using(&mut self, item: &str) -> &mut Self {
        let id = input::item_id(item);
        let held = with(|f| {
            let held = match f.world.held_item.take() {
                Some(held) if held.id == id => held,
                _ => HeldItem {
                    id: id.clone(),
                    count: 1,
                    slot: 0,
                },
            };
            f.world.held_item = Some(held.clone());
            held
        });
        if with(|f| f.claimed_items.contains(&id)) {
            self.using = Some((held.clone(), self.tick));
            self.send(Event::UseStart(ItemUse {
                item: held,
                ticks: 0,
            }));
        }
        self
    }

    pub fn stop_using(&mut self) -> &mut Self {
        if let Some((item, since)) = self.using.take() {
            let ticks = u32::try_from(self.tick - since).unwrap_or(u32::MAX);
            self.send(Event::UseStop(ItemUse { item, ticks }));
        }
        self
    }

    fn key_event(&mut self, name: &str, event: Event) -> &mut Self {
        let keys = self.keys();
        if !keys.iter().any(|k| k.name == name) {
            let declared: Vec<&str> = keys.iter().map(|k| k.name.as_str()).collect();
            panic!("the mod has no key `{name}`; it declared {declared:?}");
        }
        self.send(event)
    }

    fn send(&mut self, event: Event) -> &mut Self {
        with(|f| f.events.push(event));
        self.adapter.input();
        self
    }

    pub fn state(&self) -> &M {
        self.adapter.inner()
    }

    pub fn commands(&self) -> &[String] {
        &self.registration.commands
    }

    pub fn world(&self) -> World {
        with(|f| f.world.clone())
    }

    pub fn update_world(&mut self, change: impl FnOnce(&mut World)) -> &mut Self {
        with(|f| change(&mut f.world));
        self
    }

    pub fn tick(&mut self) -> &mut Self {
        self.ticks(1)
    }

    pub fn ticks(&mut self, count: u64) -> &mut Self {
        for _ in 0..count {
            self.tick += 1;
            with(|f| {
                if let Some(clock) = &mut f.world.clock {
                    clock.game_time += 1;
                    clock.day_time += 1;
                }
            });
            if self.registration.tick {
                self.adapter.tick(self.tick);
            }
        }
        self
    }

    pub fn frame(&mut self, seconds: f32) -> &mut Self {
        if self.registration.frame {
            self.adapter.frame(seconds);
        }
        self
    }

    pub fn command(&mut self, line: &str) -> Result<Option<String>, String> {
        let line = line.trim();
        let line = line.strip_prefix('/').unwrap_or(line);
        let (name, args) = line.split_once(' ').unwrap_or((line, ""));
        self.adapter.command(name, args)
    }

    pub fn receive_chat(&mut self, text: &str) -> &mut Self {
        if self.registration.chat {
            self.adapter.chat(text);
        }
        self
    }

    pub fn take_chat(&mut self) -> Vec<String> {
        with(|f| std::mem::take(&mut f.chat))
    }

    pub fn take_commands(&mut self) -> Vec<String> {
        with(|f| std::mem::take(&mut f.commands))
    }

    pub fn take_quiet_commands(&mut self) -> Vec<String> {
        with(|f| std::mem::take(&mut f.quiet_commands))
    }

    pub fn take_log(&mut self) -> Vec<(Level, String)> {
        with(|f| std::mem::take(&mut f.log))
    }

    pub fn panel(&self) -> Option<Panel> {
        with(|f| f.panel.clone())
    }

    pub fn panel_lines(&self) -> Vec<String> {
        self.panel()
            .map(|p| p.lines().iter().map(|l| l.text.clone()).collect())
            .unwrap_or_default()
    }

    pub fn file(&self, key: &str) -> Option<Vec<u8>> {
        with(|f| f.world.storage.get(key).cloned())
    }

    pub fn file_text(&self, key: &str) -> Option<String> {
        self.file(key)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn settings(&self) -> Vec<Widget> {
        with(|f| f.settings.clone())
    }

    pub fn screen(&self) -> Option<Screen> {
        with(|f| f.screen.clone())
    }

    pub fn open_settings(&mut self) -> &mut Self {
        self.adapter.ui_settings_opened();
        self
    }

    pub fn click_setting(&mut self, label: &str) -> &mut Self {
        let index = button(&self.settings(), label, "settings page");
        self.adapter.ui_select(widget_id(index, false), 0);
        self
    }

    pub fn pick_setting(&mut self, row: &str) -> &mut Self {
        let (index, picked) = list_row(&self.settings(), row, "settings page");
        self.adapter.ui_select(widget_id(index, false), picked);
        self
    }

    pub fn click(&mut self, label: &str) -> &mut Self {
        let index = button(&self.open_screen().widgets, label, "screen");
        self.adapter.ui_select(widget_id(index, true), 0);
        self
    }

    pub fn pick(&mut self, row: &str) -> &mut Self {
        let (index, picked) = list_row(&self.open_screen().widgets, row, "screen");
        self.adapter.ui_select(widget_id(index, true), picked);
        self
    }

    pub fn close_screen(&mut self) -> &mut Self {
        if with(|f| f.screen.take()).is_some() {
            self.adapter.ui_closed();
        }
        self
    }

    fn open_screen(&self) -> Screen {
        self.screen()
            .unwrap_or_else(|| panic!("the mod has no screen open"))
    }
}

fn boot<M: Mod>() -> Result<(Adapter<M>, Registration), String> {
    crate::hud::forget_shown();
    crate::ui::forget_state();
    crate::input::forget_state();
    Adapter::<M>::init()
}

fn button(widgets: &[Widget], label: &str, place: &str) -> usize {
    widgets
        .iter()
        .position(|w| match w {
            Widget::Button { label: shown } => {
                shown == label
                    || shown
                        .strip_prefix(label)
                        .is_some_and(|rest| rest.starts_with(": "))
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("no button `{label}` on the {place}:{}", describe(widgets)))
}

fn list_row(widgets: &[Widget], row: &str, place: &str) -> (usize, u32) {
    widgets
        .iter()
        .enumerate()
        .find_map(|(index, w)| match w {
            Widget::List { rows, .. } => rows
                .iter()
                .position(|r| r == row)
                .and_then(|r| u32::try_from(r).ok())
                .map(|r| (index, r)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no list row `{row}` on the {place}:{}", describe(widgets)))
}

fn describe(widgets: &[Widget]) -> String {
    if widgets.is_empty() {
        return " nothing".to_owned();
    }
    let mut out = String::new();
    for widget in widgets {
        let _ = match widget {
            Widget::Text { text, .. } => write!(out, "\n  text `{text}`"),
            Widget::Button { label } => write!(out, "\n  button `{label}`"),
            Widget::Item { item, name } => write!(out, "\n  item `{item}` `{name}`"),
            Widget::List { rows, .. } => write!(out, "\n  list {rows:?}"),
        };
    }
    out
}
