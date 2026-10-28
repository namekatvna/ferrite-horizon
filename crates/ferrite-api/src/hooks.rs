use std::fmt;

use crate::__private::sdk::Registration;
use crate::{Args, CommandResult};

pub type CommandHandler<M> = for<'a> fn(&mut M, Args<'a>) -> CommandResult;
pub type TickHandler<M> = fn(&mut M, u64);
pub type FrameHandler<M> = fn(&mut M, f32);
pub type ChatHandler<M> = for<'a> fn(&mut M, &'a str);

pub struct Hooks<M> {
    pub(crate) commands: Vec<(String, CommandHandler<M>)>,
    pub(crate) tick: Option<TickHandler<M>>,
    pub(crate) frame: Option<FrameHandler<M>>,
    pub(crate) chat: Option<ChatHandler<M>>,
}

impl<M> Hooks<M> {
    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
            tick: None,
            frame: None,
            chat: None,
        }
    }

    pub fn command(&mut self, name: impl Into<String>, handler: CommandHandler<M>) -> &mut Self {
        let name = name.into();
        let name = name.strip_prefix('/').unwrap_or(&name).to_owned();
        match self.commands.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = handler,
            None => self.commands.push((name, handler)),
        }
        self
    }

    pub fn on_tick(&mut self, handler: TickHandler<M>) -> &mut Self {
        self.tick = Some(handler);
        self
    }

    pub fn on_frame(&mut self, handler: FrameHandler<M>) -> &mut Self {
        self.frame = Some(handler);
        self
    }

    pub fn on_chat(&mut self, handler: ChatHandler<M>) -> &mut Self {
        self.chat = Some(handler);
        self
    }

    pub fn commands(&self) -> impl Iterator<Item = &str> {
        self.commands.iter().map(|(name, _)| name.as_str())
    }

    pub(crate) fn find_command(&self, name: &str) -> Option<CommandHandler<M>> {
        self.commands
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, handler)| *handler)
    }

    pub(crate) fn registration(&self) -> Registration {
        Registration {
            commands: self.commands().map(str::to_owned).collect(),
            tick: self.tick.is_some(),
            frame: self.frame.is_some(),
            chat: self.chat.is_some(),
        }
    }
}

impl<M> fmt::Debug for Hooks<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hooks")
            .field("commands", &self.commands().collect::<Vec<_>>())
            .field("tick", &self.tick.is_some())
            .field("frame", &self.frame.is_some())
            .field("chat", &self.chat.is_some())
            .finish()
    }
}
