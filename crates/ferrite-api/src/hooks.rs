use std::fmt;

use crate::__private::sdk::Registration;
use crate::input::{self, ItemUse, MAX_ITEMS, MAX_KEYS};
use crate::ui::PageBuilder;
use crate::{Args, CommandResult, FromArgs};

pub type CommandHandler<M> = for<'a> fn(&mut M, Args<'a>) -> CommandResult;
pub type ParsedCommandHandler<M, T> = fn(&mut M, T) -> CommandResult;
pub type TickHandler<M> = fn(&mut M, u64);
pub type FrameHandler<M> = fn(&mut M, f32);
pub type ChatHandler<M> = for<'a> fn(&mut M, &'a str);
pub type TimerHandler<M> = fn(&mut M);
pub type ScreenClosedHandler<M> = fn(&mut M);
pub type KeyHandler<M> = fn(&mut M);
pub type ScrollHandler<M> = fn(&mut M, f32);
pub type ItemUseHandler<M> = for<'a> fn(&mut M, &'a ItemUse);

pub(crate) type Command<M> = Box<dyn for<'a> Fn(&mut M, Args<'a>) -> CommandResult>;

pub const TICKS_PER_SECOND: u64 = 20;

pub(crate) struct Key<M> {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) default: String,
    pub(crate) press: Vec<KeyHandler<M>>,
    pub(crate) release: Vec<KeyHandler<M>>,
}

pub(crate) struct ItemHandlers<M> {
    pub(crate) item: String,
    pub(crate) start: Vec<ItemUseHandler<M>>,
    pub(crate) stop: Vec<ItemUseHandler<M>>,
}

pub struct Hooks<M> {
    pub(crate) commands: Vec<(String, Command<M>)>,
    pub(crate) tick: Vec<TickHandler<M>>,
    pub(crate) timers: Vec<(u64, TimerHandler<M>)>,
    pub(crate) frame: Vec<FrameHandler<M>>,
    pub(crate) chat: Vec<ChatHandler<M>>,
    pub(crate) settings: Option<PageBuilder<M>>,
    pub(crate) screen_closed: Vec<ScreenClosedHandler<M>>,
    pub(crate) keys: Vec<Key<M>>,
    pub(crate) scroll: Vec<ScrollHandler<M>>,
    pub(crate) items: Vec<ItemHandlers<M>>,
    pub(crate) problems: Vec<String>,
}

impl<M: 'static> Hooks<M> {
    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
            tick: Vec::new(),
            timers: Vec::new(),
            frame: Vec::new(),
            chat: Vec::new(),
            settings: None,
            screen_closed: Vec::new(),
            keys: Vec::new(),
            scroll: Vec::new(),
            items: Vec::new(),
            problems: Vec::new(),
        }
    }

    pub fn key(
        &mut self,
        name: impl Into<String>,
        label: impl Into<String>,
        default_key: &str,
    ) -> &mut Self {
        input::use_host();
        let name = name.into();
        if name.trim().is_empty() {
            self.problems.push("a key needs a name".to_owned());
        } else if self.keys.iter().any(|k| k.name == name) {
            self.problems
                .push(format!("key `{name}` is declared twice"));
        } else if self.keys.len() == MAX_KEYS {
            self.problems
                .push(format!("key `{name}`: a mod has at most {MAX_KEYS} keys"));
        } else {
            self.keys.push(Key {
                name,
                label: label.into(),
                default: input::key_id(default_key),
                press: Vec::new(),
                release: Vec::new(),
            });
        }
        self
    }

    pub fn on_key(
        &mut self,
        name: impl Into<String>,
        label: impl Into<String>,
        default_key: &str,
        on_press: KeyHandler<M>,
    ) -> &mut Self {
        let name = name.into();
        self.key(name.clone(), label, default_key)
            .on_key_press(&name, on_press)
    }

    pub fn on_key_press(&mut self, name: &str, handler: KeyHandler<M>) -> &mut Self {
        match self.keys.iter_mut().find(|k| k.name == name) {
            Some(key) => key.press.push(handler),
            None => self.undeclared_key("on_key_press", name),
        }
        self
    }

    pub fn on_key_release(&mut self, name: &str, handler: KeyHandler<M>) -> &mut Self {
        match self.keys.iter_mut().find(|k| k.name == name) {
            Some(key) => key.release.push(handler),
            None => self.undeclared_key("on_key_release", name),
        }
        self
    }

    pub fn on_scroll(&mut self, handler: ScrollHandler<M>) -> &mut Self {
        input::use_host();
        self.scroll.push(handler);
        self
    }

    pub fn on_item_use(&mut self, item: &str, handler: ItemUseHandler<M>) -> &mut Self {
        self.item(item).start.push(handler);
        self
    }

    pub fn on_item_release(&mut self, item: &str, handler: ItemUseHandler<M>) -> &mut Self {
        self.item(item).stop.push(handler);
        self
    }

    pub fn command(&mut self, name: impl Into<String>, handler: CommandHandler<M>) -> &mut Self {
        self.add_command(name.into(), Box::new(handler))
    }

    pub fn parsed_command<T: FromArgs + 'static>(
        &mut self,
        name: impl Into<String>,
        handler: ParsedCommandHandler<M, T>,
    ) -> &mut Self {
        self.add_command(
            name.into(),
            Box::new(move |state: &mut M, args: Args<'_>| handler(state, args.parse()?)),
        )
    }

    pub fn on_tick(&mut self, handler: TickHandler<M>) -> &mut Self {
        self.tick.push(handler);
        self
    }

    pub fn every(&mut self, ticks: u64, handler: TimerHandler<M>) -> &mut Self {
        self.timers.push((ticks.max(1), handler));
        self
    }

    pub fn every_seconds(&mut self, seconds: u64, handler: TimerHandler<M>) -> &mut Self {
        self.every(seconds.saturating_mul(TICKS_PER_SECOND), handler)
    }

    pub fn on_frame(&mut self, handler: FrameHandler<M>) -> &mut Self {
        self.frame.push(handler);
        self
    }

    pub fn on_chat(&mut self, handler: ChatHandler<M>) -> &mut Self {
        self.chat.push(handler);
        self
    }

    pub fn settings(&mut self, builder: PageBuilder<M>) -> &mut Self {
        crate::ui::use_host();
        self.settings = Some(builder);
        self
    }

    pub fn on_screen_closed(&mut self, handler: ScreenClosedHandler<M>) -> &mut Self {
        self.screen_closed.push(handler);
        self
    }

    pub fn commands(&self) -> impl Iterator<Item = &str> {
        self.commands.iter().map(|(name, _)| name.as_str())
    }

    pub(crate) fn find_command(&self, name: &str) -> Option<&Command<M>> {
        self.commands
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, handler)| handler)
    }

    pub(crate) fn registration(&self) -> Registration {
        Registration {
            commands: self.commands().map(str::to_owned).collect(),
            tick: !self.tick.is_empty() || !self.timers.is_empty() || self.uses_input(),
            frame: !self.frame.is_empty(),
            chat: !self.chat.is_empty(),
        }
    }

    pub(crate) fn uses_input(&self) -> bool {
        !self.keys.is_empty() || !self.scroll.is_empty() || !self.items.is_empty()
    }

    fn undeclared_key(&mut self, hook: &str, name: &str) {
        let declared: Vec<&str> = self.keys.iter().map(|k| k.name.as_str()).collect();
        self.problems.push(format!(
            "{hook}(\"{name}\"): no key named `{name}`; declare it first with hooks.key(\"{name}\", \"Label\", \"c\") (declared: {})",
            if declared.is_empty() { "none".to_owned() } else { declared.join(", ") }
        ));
    }

    fn item(&mut self, item: &str) -> &mut ItemHandlers<M> {
        input::use_host();
        let item = input::item_id(item);
        let index = match self.items.iter().position(|h| h.item == item) {
            Some(index) => index,
            None => {
                if self.items.len() == MAX_ITEMS {
                    self.problems.push(format!(
                        "item `{item}`: a mod claims at most {MAX_ITEMS} items"
                    ));
                }
                self.items.push(ItemHandlers {
                    item,
                    start: Vec::new(),
                    stop: Vec::new(),
                });
                self.items.len() - 1
            }
        };
        &mut self.items[index]
    }

    fn add_command(&mut self, name: String, handler: Command<M>) -> &mut Self {
        let name = name.strip_prefix('/').unwrap_or(&name).to_owned();
        match self.commands.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = handler,
            None => self.commands.push((name, handler)),
        }
        self
    }
}

impl<M> fmt::Debug for Hooks<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hooks")
            .field(
                "commands",
                &self.commands.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            )
            .field("tick", &self.tick.len())
            .field(
                "timers",
                &self.timers.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
            )
            .field("frame", &self.frame.len())
            .field("chat", &self.chat.len())
            .field("settings", &self.settings.is_some())
            .field("screen_closed", &self.screen_closed.len())
            .field(
                "keys",
                &self.keys.iter().map(|k| &k.name).collect::<Vec<_>>(),
            )
            .field("scroll", &self.scroll.len())
            .field(
                "items",
                &self.items.iter().map(|i| &i.item).collect::<Vec<_>>(),
            )
            .finish()
    }
}
