use std::fmt;

use crate::game::BlockPos;
use crate::hooks::{TICKS_PER_SECOND, TickHandler, TimerHandler};
use crate::server::edit;
use crate::server::world::{DEFAULT_EDITS_PER_TICK, EditDone, MAX_EDITS_PER_TICK};
use crate::server::{ArgumentKind, ArgumentSpec, BlockState, Player, Values};
use crate::{Args, CommandResult, FromArgs};

pub type ServerCommandHandler<M> = for<'a> fn(&mut M, &'a Player, Args<'a>) -> CommandResult;
pub type TypedServerCommandHandler<M, T> = fn(&mut M, &Player, T) -> CommandResult;
pub type SimpleServerCommandHandler<M> = fn(&mut M, &Player) -> CommandResult;
pub type PlayerHandler<M> = fn(&mut M, &Player);
pub type ServerChatHandler<M> = for<'a> fn(&mut M, &'a Player, &'a str);
pub type BlockHandler<M> = for<'a> fn(&mut M, &'a Player, BlockPos, &'a BlockState);
pub type EditDoneHandler<M> = fn(&mut M, &mut EditDone);

pub(crate) type Run<M> =
    Box<dyn for<'a, 'b> Fn(&mut M, &'a Player, Args<'a>, &'b Values<'b>) -> CommandResult>;

pub(crate) struct Command<M> {
    pub(crate) name: String,
    pub(crate) arguments: Vec<ArgumentSpec>,
    pub(crate) run: Run<M>,
}

pub struct ServerHooks<M> {
    pub(crate) commands: Vec<Command<M>>,
    pub(crate) tick: Vec<TickHandler<M>>,
    pub(crate) timers: Vec<(u64, TimerHandler<M>)>,
    pub(crate) join: Vec<PlayerHandler<M>>,
    pub(crate) leave: Vec<PlayerHandler<M>>,
    pub(crate) chat: Vec<ServerChatHandler<M>>,
    pub(crate) block_break: Vec<BlockHandler<M>>,
    pub(crate) block_place: Vec<BlockHandler<M>>,
    pub(crate) edit_done: Vec<EditDoneHandler<M>>,
    pub(crate) edits_per_tick: Option<usize>,
    pub(crate) run_edits: Option<fn()>,
}

impl<M: 'static> ServerHooks<M> {
    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
            tick: Vec::new(),
            timers: Vec::new(),
            join: Vec::new(),
            leave: Vec::new(),
            chat: Vec::new(),
            block_break: Vec::new(),
            block_place: Vec::new(),
            edit_done: Vec::new(),
            edits_per_tick: None,
            run_edits: None,
        }
    }

    pub fn command(
        &mut self,
        name: impl Into<String>,
        handler: ServerCommandHandler<M>,
    ) -> &mut Self {
        self.add(
            name.into(),
            free_text(),
            Box::new(
                move |state: &mut M, player: &Player, args: Args<'_>, _: &Values<'_>| {
                    handler(state, player, args)
                },
            ),
        )
    }

    pub fn simple_command(
        &mut self,
        name: impl Into<String>,
        handler: SimpleServerCommandHandler<M>,
    ) -> &mut Self {
        self.add(
            name.into(),
            Vec::new(),
            Box::new(
                move |state: &mut M, player: &Player, _: Args<'_>, _: &Values<'_>| {
                    handler(state, player)
                },
            ),
        )
    }

    pub fn parsed_command<T: FromArgs + 'static>(
        &mut self,
        name: impl Into<String>,
        handler: TypedServerCommandHandler<M, T>,
    ) -> &mut Self {
        match T::server_arguments() {
            Some(arguments) => self.add(
                name.into(),
                arguments,
                Box::new(
                    move |state: &mut M, player: &Player, _: Args<'_>, values: &Values<'_>| {
                        handler(state, player, T::from_server(values)?)
                    },
                ),
            ),
            None => self.add(
                name.into(),
                free_text(),
                Box::new(
                    move |state: &mut M, player: &Player, args: Args<'_>, _: &Values<'_>| {
                        handler(state, player, args.parse()?)
                    },
                ),
            ),
        }
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

    pub fn on_join(&mut self, handler: PlayerHandler<M>) -> &mut Self {
        self.join.push(handler);
        self
    }

    pub fn on_leave(&mut self, handler: PlayerHandler<M>) -> &mut Self {
        self.leave.push(handler);
        self
    }

    pub fn on_chat(&mut self, handler: ServerChatHandler<M>) -> &mut Self {
        self.chat.push(handler);
        self
    }

    pub fn on_block_break(&mut self, handler: BlockHandler<M>) -> &mut Self {
        self.block_break.push(handler);
        self
    }

    pub fn on_block_place(&mut self, handler: BlockHandler<M>) -> &mut Self {
        self.block_place.push(handler);
        self
    }

    pub fn queue_edits(&mut self, blocks_per_tick: usize) -> &mut Self {
        self.edits_per_tick = Some(blocks_per_tick.clamp(1, MAX_EDITS_PER_TICK));
        self.run_edits = Some(edit::run_tick);
        self
    }

    pub fn on_edit_done(&mut self, handler: EditDoneHandler<M>) -> &mut Self {
        self.edits_per_tick.get_or_insert(DEFAULT_EDITS_PER_TICK);
        self.run_edits = Some(edit::run_tick);
        self.edit_done.push(handler);
        self
    }

    pub fn commands(&self) -> impl Iterator<Item = &str> {
        self.commands.iter().map(|c| c.name.as_str())
    }

    pub(crate) fn find_command(&self, name: &str) -> Option<&Command<M>> {
        self.commands.iter().find(|c| c.name == name)
    }

    fn add(&mut self, name: String, arguments: Vec<ArgumentSpec>, run: Run<M>) -> &mut Self {
        let name = name.strip_prefix('/').unwrap_or(&name).to_owned();
        let command = Command {
            name,
            arguments,
            run,
        };
        match self.commands.iter_mut().find(|c| c.name == command.name) {
            Some(existing) => *existing = command,
            None => self.commands.push(command),
        }
        self
    }
}

fn free_text() -> Vec<ArgumentSpec> {
    vec![ArgumentSpec::new("args", ArgumentKind::Text, true)]
}

impl<M> fmt::Debug for ServerHooks<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerHooks")
            .field(
                "commands",
                &self.commands.iter().map(|c| &c.name).collect::<Vec<_>>(),
            )
            .field("tick", &self.tick.len())
            .field("timers", &self.timers.len())
            .field("join", &self.join.len())
            .field("leave", &self.leave.len())
            .field("chat", &self.chat.len())
            .field("block_break", &self.block_break.len())
            .field("block_place", &self.block_place.len())
            .field("edit_done", &self.edit_done.len())
            .field("edits_per_tick", &self.edits_per_tick)
            .finish()
    }
}
