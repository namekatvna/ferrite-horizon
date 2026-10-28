use ferrite_api::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Settings {
    visible: bool,
    corner: Corner,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            visible: true,
            corner: Corner::TopLeft,
        }
    }
}

#[derive(FromArgs)]
enum CompassCommand {
    #[default]
    Toggle,
    Corner {
        corner: Corner,
    },
    Reset,
}

struct Compass {
    settings: Config<Settings>,
}

impl Mod for Compass {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .on_tick(Self::tick)
            .settings(Self::settings_page)
            .command("compass", Self::compass)
            .command("day", Self::day);
        Ok(Self {
            settings: Config::load(),
        })
    }
}

impl Compass {
    fn settings_page(&self, page: &mut Page<Self>) {
        page.toggle("Visible", self.settings.visible, |m, on| {
            m.change(|s| s.visible = on)
        })
        .cycle(
            "Corner",
            &Corner::ALL,
            &self.settings.corner,
            |m, corner| m.change(|s| s.corner = corner),
        )
        .button("Reset", |m| {
            if let Err(error) = m.settings.reset() {
                log::warn(format!("compass: {error}"));
            }
        });
    }

    fn change(&mut self, edit: impl FnOnce(&mut Settings)) {
        if let Err(error) = self.settings.update(edit) {
            log::warn(format!("compass: {error}"));
        }
    }

    fn tick(&mut self, _tick: u64) {
        let (Some(player), Some(clock)) = (game::player(), game::clock()) else {
            hud::clear();
            return;
        };
        if !self.settings.visible {
            hud::clear();
            return;
        }
        let (hours, minutes) = clock.hours_minutes();
        let time_color = if clock.is_night() {
            Color::GRAY
        } else {
            Color::YELLOW
        };
        Panel::new(self.settings.corner)
            .line(format!("XYZ {}", player.block_pos()))
            .colored(format!("Facing {}", player.facing()), Color::AQUA)
            .colored(
                format!("{} {hours:02}:{minutes:02}", player.dimension),
                time_color,
            )
            .show();
    }

    fn compass(&mut self, args: Args) -> CommandResult {
        match args.parse()? {
            CompassCommand::Toggle => {
                let visible = self.settings.update(|s| {
                    s.visible = !s.visible;
                    s.visible
                })?;
                Ok(Some(format!(
                    "Compass {}",
                    if visible { "on" } else { "off" }
                )))
            }
            CompassCommand::Corner { corner } => {
                self.settings.update(|s| s.corner = corner)?;
                Ok(Some(format!("Compass moved to {corner}")))
            }
            CompassCommand::Reset => {
                self.settings.reset()?;
                Ok(Some("Compass settings reset".into()))
            }
        }
    }

    fn day(&mut self, _args: Args) -> CommandResult {
        command::run("/time set day");
        chat::show("Good morning!");
        Ok(None)
    }
}

export_mod!(Compass);
