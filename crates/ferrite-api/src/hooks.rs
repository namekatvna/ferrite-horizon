use crate::__private::sdk::Registration;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hooks {
    commands: Vec<String>,
    tick: bool,
    frame: bool,
    chat: bool,
}

impl Hooks {
    pub fn command(&mut self, name: impl Into<String>) -> &mut Self {
        let name = name.into();
        if !self.commands.contains(&name) {
            self.commands.push(name);
        }
        self
    }

    pub fn on_tick(&mut self) -> &mut Self {
        self.tick = true;
        self
    }

    pub fn on_frame(&mut self) -> &mut Self {
        self.frame = true;
        self
    }

    pub fn on_chat(&mut self) -> &mut Self {
        self.chat = true;
        self
    }

    pub(crate) fn into_registration(self) -> Registration {
        let mut registration = self
            .commands
            .iter()
            .fold(Registration::new(), |r, name| r.command(name));
        if self.tick {
            registration = registration.on_tick();
        }
        if self.frame {
            registration = registration.on_frame();
        }
        if self.chat {
            registration = registration.on_chat();
        }
        registration
    }
}
