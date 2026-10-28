use crate::__private::server_sdk as sdk;
use crate::game::BlockPos;
use crate::server::{
    ArgumentKind, ArgumentValue, BlockState, Player, ServerHooks, ServerMod, Values,
};
use crate::{Args, host};

pub struct ServerAdapter<M> {
    inner: M,
    hooks: ServerHooks<M>,
    ticks: u64,
}

impl<M: ServerMod> ServerAdapter<M> {
    pub fn init() -> Result<(Self, sdk::Registration), String> {
        host::forget_snapshot();
        let mut hooks = ServerHooks::new();
        let inner = M::init(&mut hooks).map_err(crate::Error::into_message)?;
        let registration = registration(&hooks);
        Ok((
            Self {
                inner,
                hooks,
                ticks: 0,
            },
            registration,
        ))
    }

    pub fn inner(&self) -> &M {
        &self.inner
    }

    pub fn tick(&mut self, tick: u64) {
        host::forget_snapshot();
        self.ticks += 1;
        for handler in &self.hooks.tick {
            handler(&mut self.inner, tick);
        }
        for &(interval, handler) in &self.hooks.timers {
            if self.ticks % interval == 0 {
                handler(&mut self.inner);
            }
        }
    }

    pub fn joined(&mut self, player: &Player) {
        host::forget_snapshot();
        for handler in &self.hooks.join {
            handler(&mut self.inner, player);
        }
    }

    pub fn left(&mut self, player: &Player) {
        host::forget_snapshot();
        for handler in &self.hooks.leave {
            handler(&mut self.inner, player);
        }
    }

    pub fn chat(&mut self, player: &Player, text: &str) {
        host::forget_snapshot();
        for handler in &self.hooks.chat {
            handler(&mut self.inner, player, text);
        }
    }

    pub fn command(
        &mut self,
        player: &Player,
        name: &str,
        args: &str,
        values: &[(String, ArgumentValue)],
    ) -> Result<Option<String>, String> {
        host::forget_snapshot();
        let Some(command) = self.hooks.find_command(name) else {
            return Err(format!("Unknown command /{name}"));
        };
        (command.run)(
            &mut self.inner,
            player,
            Args::for_command(name, args),
            &Values::new(values),
        )
        .map_err(crate::Error::into_message)
    }

    pub fn block_broken(&mut self, player: &Player, pos: BlockPos, state: &BlockState) {
        host::forget_snapshot();
        for handler in &self.hooks.block_break {
            handler(&mut self.inner, player, pos, state);
        }
    }

    pub fn block_placed(&mut self, player: &Player, pos: BlockPos, state: &BlockState) {
        host::forget_snapshot();
        for handler in &self.hooks.block_place {
            handler(&mut self.inner, player, pos, state);
        }
    }
}

fn registration<M: 'static>(hooks: &ServerHooks<M>) -> sdk::Registration {
    sdk::Registration {
        commands: hooks
            .commands
            .iter()
            .map(|command| sdk::ServerCommand {
                name: command.name.clone(),
                arguments: command
                    .arguments
                    .iter()
                    .map(|spec| sdk::Argument {
                        name: spec.name.clone(),
                        kind: match spec.kind {
                            ArgumentKind::Word => sdk::ArgumentKind::Word,
                            ArgumentKind::Text => sdk::ArgumentKind::Text,
                            ArgumentKind::Integer { min, max } => {
                                sdk::ArgumentKind::Integer((min, max))
                            }
                            ArgumentKind::Number => sdk::ArgumentKind::Number,
                            ArgumentKind::Boolean => sdk::ArgumentKind::Boolean,
                            ArgumentKind::Player => sdk::ArgumentKind::Player,
                            ArgumentKind::BlockPos => sdk::ArgumentKind::BlockPos,
                        },
                        optional: spec.optional,
                    })
                    .collect(),
            })
            .collect(),
        tick: !hooks.tick.is_empty() || !hooks.timers.is_empty(),
        joins: !hooks.join.is_empty() || !hooks.leave.is_empty(),
        chat: !hooks.chat.is_empty(),
        blocks: !hooks.block_break.is_empty() || !hooks.block_place.is_empty(),
    }
}
