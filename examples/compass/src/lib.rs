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
            .parsed_command("compass", Self::compass)
            .command("day", Self::day);
        Ok(Self {
            settings: Config::load(),
        })
    }
}

impl Compass {
    fn settings_page(&self, page: &mut Page<Self>) {
        page.toggle("Visible", self.settings.visible, |m, on| {
            m.settings.edit(|s| s.visible = on)
        })
        .cycle(
            "Corner",
            &Corner::ALL,
            &self.settings.corner,
            |m, corner| m.settings.edit(|s| s.corner = corner),
        )
        .button("Reset", |m| m.settings.edit(|s| *s = Settings::default()));
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

    fn compass(&mut self, command: CompassCommand) -> CommandResult {
        match command {
            CompassCommand::Toggle => {
                let visible = self.settings.update(|s| {
                    s.visible = !s.visible;
                    s.visible
                })?;
                reply(format!("Compass {}", if visible { "on" } else { "off" }))
            }
            CompassCommand::Corner { corner } => {
                self.settings.update(|s| s.corner = corner)?;
                reply(format!("Compass moved to {corner}"))
            }
            CompassCommand::Reset => {
                self.settings.reset()?;
                reply("Compass settings reset")
            }
        }
    }

    fn day(&mut self, _args: Args) -> CommandResult {
        command::run("/time set day");
        chat::show("Good morning!");
        no_reply()
    }
}

export_mod!(Compass);

#[cfg(test)]
mod tests {
    use ferrite_api::testing::{TestGame, World};

    use super::*;

    #[test]
    fn panel_follows_the_player_and_the_clock() {
        let mut game = TestGame::<Compass>::start();
        game.update_world(|w| {
            w.player.as_mut().unwrap().yaw = 180.0;
            w.clock.as_mut().unwrap().day_time = 13_000;
        })
        .tick();
        assert_eq!(
            game.panel_lines(),
            ["XYZ 0 64 0", "Facing North", "Overworld 19:00"]
        );

        game.update_world(|w| *w = World::title_screen()).tick();
        assert_eq!(game.panel(), None);
    }

    #[test]
    fn settings_page_edits_the_saved_config() {
        let mut game = TestGame::<Compass>::start();
        assert_eq!(
            game.settings(),
            [
                ui::Widget::Button {
                    label: "Visible: ON".into()
                },
                ui::Widget::Button {
                    label: "Corner: top-left".into()
                },
                ui::Widget::Button {
                    label: "Reset".into()
                },
            ]
        );

        game.click_setting("Corner").click_setting("Corner").tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::BottomLeft);
        assert!(
            game.file_text("config.json")
                .unwrap()
                .contains("bottom-left")
        );

        game.click_setting("Visible").tick();
        assert_eq!(game.panel(), None);

        game.restart().tick();
        assert_eq!(game.panel(), None);
        game.click_setting("Reset").tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::TopLeft);
    }

    #[test]
    fn commands_and_the_page_stay_in_sync() {
        let mut game = TestGame::<Compass>::start();
        assert_eq!(
            game.command("compass corner bottom-right"),
            Ok(Some("Compass moved to bottom-right".into()))
        );
        assert!(game.settings().contains(&ui::Widget::Button {
            label: "Corner: bottom-right".into()
        }));

        game.command("day").unwrap();
        assert_eq!(game.take_commands(), ["time set day"]);
        assert_eq!(game.take_chat(), ["Good morning!"]);
    }
}
