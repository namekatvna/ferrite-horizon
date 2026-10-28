use ferrite_api::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Settings)]
struct Settings {
    visible: bool,
    corner: Corner,
    show_clicks: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            visible: true,
            corner: Corner::TopRight,
            show_clicks: true,
        }
    }
}

struct Keystrokes {
    settings: Config<Settings>,
}

impl Mod for Keystrokes {
    fn init(hooks: &mut Hooks<Self>) -> Result<Self, Error> {
        hooks
            .on_key("toggle", "Show or hide keystrokes", "k", |m| {
                m.settings.edit(|s| s.visible = !s.visible)
            })
            .key("move", "Hold to move with the mouse wheel", "left.alt")
            .on_key_press("move", |_| input::capture_scroll(true))
            .on_key_release("move", |_| input::capture_scroll(false))
            .on_scroll(Self::move_panel)
            .on_tick(Self::draw)
            .settings(|m, page| {
                page.config(&m.settings, |m| &mut m.settings);
            });
        Ok(Self {
            settings: Config::load(),
        })
    }
}

impl Keystrokes {
    fn move_panel(&mut self, steps: f32) {
        let current = Corner::ALL
            .iter()
            .position(|c| *c == self.settings.corner)
            .unwrap_or(0);
        let next = if steps > 0.0 {
            current + 1
        } else {
            current + 3
        } % Corner::ALL.len();
        self.settings.edit(|s| s.corner = Corner::ALL[next]);
    }

    fn draw(&mut self, _tick: u64) {
        if !self.settings.visible {
            hud::clear();
            return;
        }
        let key = |control: Control| match input::control(control) {
            Some(state) if state.down => (format!("[{}]", state.key), Color::YELLOW),
            Some(state) => (state.key, Color::GRAY),
            None => ("?".to_owned(), Color::DARK_GRAY),
        };
        let mut panel = Panel::new(self.settings.corner);
        panel.push(key(Control::Forward));
        let (left, back, right) = (key(Control::Left), key(Control::Back), key(Control::Right));
        let any_down = [&left, &back, &right].iter().any(|k| k.1 == Color::YELLOW);
        panel.push((
            format!("{} {} {}", left.0, back.0, right.0),
            if any_down { Color::YELLOW } else { Color::GRAY },
        ));
        panel.push(key(Control::Jump));
        if self.settings.show_clicks {
            let clicks = input::clicks_per_second();
            panel.push(format!("{} | {} CPS", clicks.attack, clicks.use_item));
        }
        panel.show();
    }
}

export_mod!(Keystrokes);

#[cfg(test)]
mod tests {
    use ferrite_api::input::Clicks;
    use ferrite_api::testing::{TestGame, World};

    use super::*;

    #[test]
    fn shows_which_movement_keys_are_down() {
        let mut game = TestGame::<Keystrokes>::start_in(World {
            controls_down: vec![Control::Forward, Control::Jump],
            clicks: Clicks {
                attack: 7,
                use_item: 2,
            },
            ..World::default()
        });
        game.tick();
        assert_eq!(game.panel_lines(), ["[W]", "A S D", "[Space]", "7 | 2 CPS"]);
        assert_eq!(game.panel().unwrap().corner(), Corner::TopRight);
    }

    #[test]
    fn the_toggle_key_hides_and_shows_the_overlay() {
        let mut game = TestGame::<Keystrokes>::start();
        game.tap_key("toggle").tick();
        assert_eq!(game.panel(), None);
        assert!(
            game.file_text("config.json")
                .unwrap()
                .contains("\"visible\": false")
        );

        game.tap_key("toggle").tick();
        assert!(game.panel().is_some());
    }

    #[test]
    fn the_wheel_moves_the_overlay_only_while_the_move_key_is_held() {
        let mut game = TestGame::<Keystrokes>::start();
        game.scroll(1.0).tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::TopRight);

        game.press_key("move").scroll(1.0).tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::BottomLeft);
        game.scroll(-1.0).scroll(-1.0).tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::TopLeft);

        game.release_key("move").scroll(1.0).tick();
        assert_eq!(game.panel().unwrap().corner(), Corner::TopLeft);
        assert!(game.settings().contains(&ui::Widget::Button {
            label: "Corner: Top left".into()
        }));
    }
}
