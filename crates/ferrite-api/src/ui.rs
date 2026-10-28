use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::Display;

use crate::host;
use crate::hud::Color;

pub const MAX_WIDGETS: usize = 64;
pub const MAX_TEXT_CHARS: usize = 256;
pub const MAX_ROWS: usize = 512;

pub(crate) const SCREEN_IDS: u32 = 0x1000_0000;

pub type PageBuilder<M> = fn(&M, &mut Page<M>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Widget {
    Text {
        text: String,
        color: Option<Color>,
    },
    Button {
        label: String,
    },
    Item {
        item: String,
        name: String,
    },
    List {
        rows: Vec<String>,
        selected: Option<u32>,
    },
}

type Action<M> = Box<dyn Fn(&mut M, u32)>;

pub struct Page<M> {
    title: String,
    widgets: Vec<Widget>,
    actions: Vec<Option<Action<M>>>,
}

impl<M: 'static> Page<M> {
    pub(crate) fn build(state: &M, builder: PageBuilder<M>) -> Self {
        let mut page = Self {
            title: String::new(),
            widgets: Vec::new(),
            actions: Vec::new(),
        };
        builder(state, &mut page);
        page
    }

    pub fn set_title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = clip(title.into());
        self
    }

    pub fn text(&mut self, text: impl Into<String>) -> &mut Self {
        self.push(
            Widget::Text {
                text: clip(text.into()),
                color: None,
            },
            None,
        )
    }

    pub fn colored(&mut self, text: impl Into<String>, color: Color) -> &mut Self {
        self.push(
            Widget::Text {
                text: clip(text.into()),
                color: Some(color),
            },
            None,
        )
    }

    pub fn item(&mut self, item: impl Into<String>, name: impl Into<String>) -> &mut Self {
        self.push(
            Widget::Item {
                item: item.into(),
                name: clip(name.into()),
            },
            None,
        )
    }

    pub fn button(&mut self, label: impl Into<String>, on_click: fn(&mut M)) -> &mut Self {
        self.push(
            Widget::Button {
                label: clip(label.into()),
            },
            Some(Box::new(move |state, _| on_click(state))),
        )
    }

    pub fn toggle(
        &mut self,
        label: impl Display,
        value: bool,
        on_change: fn(&mut M, bool),
    ) -> &mut Self {
        let shown = if value { "ON" } else { "OFF" };
        self.push(
            Widget::Button {
                label: clip(format!("{label}: {shown}")),
            },
            Some(Box::new(move |state, _| on_change(state, !value))),
        )
    }

    pub fn cycle<T: Clone + PartialEq + Display + 'static>(
        &mut self,
        label: impl Display,
        options: &[T],
        current: &T,
        on_change: fn(&mut M, T),
    ) -> &mut Self {
        let next = options
            .iter()
            .position(|option| option == current)
            .map_or(0, |i| (i + 1) % options.len().max(1));
        let next = options.get(next).cloned();
        self.push(
            Widget::Button {
                label: clip(format!("{label}: {current}")),
            },
            Some(Box::new(move |state, _| {
                if let Some(next) = &next {
                    on_change(state, next.clone());
                }
            })),
        )
    }

    pub fn list<T: Clone + PartialEq + Display + 'static>(
        &mut self,
        options: &[T],
        selected: Option<&T>,
        on_select: fn(&mut M, T),
    ) -> &mut Self {
        let options: Vec<T> = options.iter().take(MAX_ROWS).cloned().collect();
        let rows = options.iter().map(|o| clip(o.to_string())).collect();
        let selected = selected
            .and_then(|s| options.iter().position(|o| o == s))
            .and_then(|i| u32::try_from(i).ok());
        self.push(
            Widget::List { rows, selected },
            Some(Box::new(move |state, row| {
                if let Some(option) = options.get(row as usize) {
                    on_select(state, option.clone());
                }
            })),
        )
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn widgets(&self) -> &[Widget] {
        &self.widgets
    }

    pub(crate) fn activate(&self, state: &mut M, index: u32, row: u32) -> bool {
        match self.actions.get(index as usize) {
            Some(Some(action)) => {
                action(state, row);
                true
            }
            _ => false,
        }
    }

    fn push(&mut self, widget: Widget, action: Option<Action<M>>) -> &mut Self {
        if self.widgets.len() < MAX_WIDGETS {
            self.widgets.push(widget);
            self.actions.push(action);
        }
        self
    }
}

impl<M> std::fmt::Debug for Page<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("title", &self.title)
            .field("widgets", &self.widgets)
            .finish()
    }
}

pub fn open<M: 'static>(builder: PageBuilder<M>) {
    use_host();
    REQUESTS.with(|r| r.borrow_mut().screen = Some(ScreenRequest::Open(Box::new(builder))));
}

pub fn close() {
    use_host();
    REQUESTS.with(|r| r.borrow_mut().screen = Some(ScreenRequest::Close));
}

pub fn refresh() {
    REQUESTS.with(|r| r.borrow_mut().refresh = true);
}

pub(crate) enum ScreenRequest {
    Open(Box<dyn Any>),
    Close,
}

#[derive(Default)]
pub(crate) struct Requests {
    pub(crate) screen: Option<ScreenRequest>,
    pub(crate) refresh: bool,
}

thread_local! {
    static REQUESTS: RefCell<Requests> = RefCell::new(Requests::default());
}

pub(crate) fn take_requests() -> Requests {
    REQUESTS.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

pub(crate) struct HostUi {
    pub(crate) set_settings: fn(&[Widget]),
    pub(crate) open_screen: fn(&str, &[Widget]),
    pub(crate) close_screen: fn(),
}

static HOST: HostUi = HostUi {
    set_settings: host::ui_set_settings,
    open_screen: host::ui_open_screen,
    close_screen: host::ui_close_screen,
};

thread_local! {
    static ACTIVE_HOST: Cell<Option<&'static HostUi>> = const { Cell::new(None) };
}

pub(crate) fn use_host() {
    ACTIVE_HOST.with(|active| active.set(Some(&HOST)));
}

pub(crate) fn active_host() -> Option<&'static HostUi> {
    ACTIVE_HOST.with(Cell::get)
}

pub(crate) fn widget_id(index: usize, screen: bool) -> u32 {
    let index = u32::try_from(index).unwrap_or(u32::MAX);
    if screen { SCREEN_IDS + index } else { index }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn forget_state() {
    REQUESTS.with(|r| *r.borrow_mut() = Requests::default());
    ACTIVE_HOST.with(|active| active.set(None));
}

fn clip(mut text: String) -> String {
    if let Some((cut, _)) = text.char_indices().nth(MAX_TEXT_CHARS) {
        text.truncate(cut);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct State {
        on: bool,
        corner: u8,
        clicks: u32,
        picked: Option<&'static str>,
    }

    fn settings(state: &State, page: &mut Page<State>) {
        page.set_title("Ignored on settings")
            .text("Header")
            .toggle("Visible", state.on, |s, on| s.on = on)
            .cycle("Corner", &[1u8, 2, 3], &state.corner, |s, c| s.corner = c)
            .button("Click", |s| s.clicks += 1)
            .list(&["a", "b"], state.picked.as_ref(), |s, v| {
                s.picked = Some(v)
            });
    }

    #[test]
    fn widgets_describe_the_state() {
        let state = State {
            on: true,
            corner: 2,
            picked: Some("b"),
            ..State::default()
        };
        let page = Page::build(&state, settings);
        assert_eq!(page.title(), "Ignored on settings");
        assert_eq!(
            page.widgets(),
            [
                Widget::Text {
                    text: "Header".into(),
                    color: None
                },
                Widget::Button {
                    label: "Visible: ON".into()
                },
                Widget::Button {
                    label: "Corner: 2".into()
                },
                Widget::Button {
                    label: "Click".into()
                },
                Widget::List {
                    rows: vec!["a".into(), "b".into()],
                    selected: Some(1)
                },
            ]
        );
    }

    #[test]
    fn actions_change_the_state() {
        let mut state = State {
            corner: 3,
            ..State::default()
        };
        let page = Page::build(&state, settings);
        assert!(!page.activate(&mut state, 0, 0));
        assert!(page.activate(&mut state, 1, 0));
        assert!(page.activate(&mut state, 2, 0));
        assert!(page.activate(&mut state, 3, 0));
        assert!(page.activate(&mut state, 4, 0));
        assert!(page.activate(&mut state, 4, 9));
        assert!(!page.activate(&mut state, 99, 0));
        assert!(state.on);
        assert_eq!(state.corner, 1);
        assert_eq!(state.clicks, 1);
        assert_eq!(state.picked, Some("a"));
    }

    #[test]
    fn cycle_starts_over_from_an_unknown_value() {
        let mut state = State {
            corner: 7,
            ..State::default()
        };
        let page = Page::build(&state, settings);
        page.activate(&mut state, 2, 0);
        assert_eq!(state.corner, 1);
    }

    #[test]
    fn limits_are_applied() {
        let page = Page::build(&State::default(), |_, page: &mut Page<State>| {
            for i in 0..100 {
                page.text("ж".repeat(300 + i));
            }
        });
        assert_eq!(page.widgets().len(), MAX_WIDGETS);
        let Widget::Text { text, .. } = &page.widgets()[0] else {
            panic!("text expected");
        };
        assert_eq!(text.chars().count(), MAX_TEXT_CHARS);
    }

    #[test]
    fn screen_ids_do_not_collide_with_settings_ids() {
        assert_eq!(widget_id(4, false), 4);
        assert_eq!(widget_id(4, true), SCREEN_IDS + 4);
        assert!(widget_id(MAX_WIDGETS, false) < widget_id(0, true));
    }
}
