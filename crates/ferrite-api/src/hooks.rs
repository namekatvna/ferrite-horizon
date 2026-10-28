use std::fmt;

use crate::__private::sdk::Registration;
use crate::ui::PageBuilder;
use crate::{Args, CommandResult, FromArgs};

pub type CommandHandler<M> = for<'a> fn(&mut M, Args<'a>) -> CommandResult;
pub type ParsedCommandHandler<M, T> = fn(&mut M, T) -> CommandResult;
pub type TickHandler<M> = fn(&mut M, u64);
pub type FrameHandler<M> = fn(&mut M, f32);
pub type ChatHandler<M> = for<'a> fn(&mut M, &'a str);
pub type TimerHandler<M> = fn(&mut M);
pub type ScreenClosedHandler<M> = fn(&mut M);

pub(crate) type Command<M> = Box<dyn for<'a> Fn(&mut M, Args<'a>) -> CommandResult>;

pub const TICKS_PER_SECOND: u64 = 20;

pub struct Hooks<M> {
    pub(crate) commands: Vec<(String, Command<M>)>,
    pub(crate) tick: Vec<TickHandler<M>>,
    pub(crate) timers: Vec<(u64, TimerHandler<M>)>,
    pub(crate) frame: Vec<FrameHandler<M>>,
    pub(crate) chat: Vec<ChatHandler<M>>,
    pub(crate) settings: Option<PageBuilder<M>>,
    pub(crate) screen_closed: Vec<ScreenClosedHandler<M>>,
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
        }
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
            tick: !self.tick.is_empty() || !self.timers.is_empty(),
            frame: !self.frame.is_empty(),
            chat: !self.chat.is_empty(),
        }
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
            .finish()
    }
}
