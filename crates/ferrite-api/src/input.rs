use std::cell::{Cell, RefCell};

use crate::host;

pub const MAX_KEYS: usize = 16;
pub const MAX_ITEMS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HeldItem {
    pub id: String,
    pub count: u32,
    pub slot: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemUse {
    pub item: HeldItem,
    pub ticks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Control {
    Forward,
    Left,
    Back,
    Right,
    Jump,
    Sneak,
    Sprint,
    Attack,
    Use,
}

impl Control {
    pub const ALL: [Self; 9] = [
        Self::Forward,
        Self::Left,
        Self::Back,
        Self::Right,
        Self::Jump,
        Self::Sneak,
        Self::Sprint,
        Self::Attack,
        Self::Use,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Forward => "key.forward",
            Self::Left => "key.left",
            Self::Back => "key.back",
            Self::Right => "key.right",
            Self::Jump => "key.jump",
            Self::Sneak => "key.sneak",
            Self::Sprint => "key.sprint",
            Self::Attack => "key.attack",
            Self::Use => "key.use",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ControlState {
    pub key: String,
    pub down: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Clicks {
    pub attack: u32,
    pub use_item: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Event {
    KeyDown(String),
    KeyUp(String),
    Scroll(f32),
    UseStart(ItemUse),
    UseStop(ItemUse),
}

pub fn is_down(key: &str) -> bool {
    PRESSED.with(|pressed| pressed.borrow().iter().any(|k| k == key))
}

pub fn capture_scroll(on: bool) {
    host::input_claim_wheel(on);
}

pub fn held_item() -> Option<HeldItem> {
    host::input_held()
}

pub fn control(control: Control) -> Option<ControlState> {
    host::input_mapping(control.id())
}

pub fn clicks_per_second() -> Clicks {
    host::input_clicks()
}

pub fn key_id(key: &str) -> String {
    let key = key.trim().to_ascii_lowercase();
    if key.starts_with("key.") {
        key
    } else {
        format!("key.keyboard.{key}")
    }
}

pub(crate) fn item_id(item: &str) -> String {
    let item = item.trim().to_ascii_lowercase();
    if item.contains(':') {
        item
    } else {
        format!("minecraft:{item}")
    }
}

pub(crate) struct HostInput {
    pub(crate) register_key: fn(&str, &str, &str) -> Result<(), String>,
    pub(crate) claim_items: fn(&[String]),
    pub(crate) poll: fn() -> Vec<Event>,
}

static HOST: HostInput = HostInput {
    register_key: host::input_register_key,
    claim_items: host::input_claim_items,
    poll: host::input_poll,
};

thread_local! {
    static ACTIVE_HOST: Cell<Option<&'static HostInput>> = const { Cell::new(None) };
    static PRESSED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn use_host() {
    ACTIVE_HOST.with(|active| active.set(Some(&HOST)));
}

pub(crate) fn active_host() -> Option<&'static HostInput> {
    ACTIVE_HOST.with(Cell::get)
}

pub(crate) fn note(event: &Event) {
    PRESSED.with(|pressed| {
        let mut pressed = pressed.borrow_mut();
        match event {
            Event::KeyDown(name) if !pressed.contains(name) => pressed.push(name.clone()),
            Event::KeyUp(name) => pressed.retain(|k| k != name),
            _ => {}
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn forget_state() {
    ACTIVE_HOST.with(|active| active.set(None));
    PRESSED.with(|pressed| pressed.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_key_names_become_minecraft_ids() {
        assert_eq!(key_id("C"), "key.keyboard.c");
        assert_eq!(key_id("left.shift"), "key.keyboard.left.shift");
        assert_eq!(key_id("key.mouse.middle"), "key.mouse.middle");
        assert_eq!(item_id("spyglass"), "minecraft:spyglass");
        assert_eq!(item_id("mymod:wand"), "mymod:wand");
    }

    #[test]
    fn key_state_follows_events() {
        note(&Event::KeyDown("zoom".into()));
        note(&Event::KeyDown("zoom".into()));
        assert!(is_down("zoom"));
        note(&Event::KeyUp("zoom".into()));
        assert!(!is_down("zoom"));
    }
}
