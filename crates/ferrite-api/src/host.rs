#[cfg(not(target_arch = "wasm32"))]
pub(crate) use crate::testing::fake::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::cell::{Cell, RefCell};

    use crate::__private::sdk::{chat, command, game, hud, input, log, storage, ui};
    use crate::__private::server_sdk;
    use crate::game::{BlockPos, Clock, Hit, Player, Vec3};
    use crate::hud::{Corner, Line};
    use crate::input::{Clicks, ControlState, Event, HeldItem, ItemUse};
    use crate::log::Level;
    use crate::ui::{Widget, widget_id};

    pub(crate) fn input_register_key(name: &str, label: &str, key: &str) -> Result<(), String> {
        input::register_key(name, label, key)
    }

    pub(crate) fn input_claim_items(items: &[String]) {
        input::claim_items(items);
    }

    pub(crate) fn input_claim_wheel(on: bool) {
        input::claim_wheel(on);
    }

    pub(crate) fn input_poll() -> Vec<Event> {
        input::poll()
            .into_iter()
            .map(|event| match event {
                input::Event::KeyDown(name) => Event::KeyDown(name),
                input::Event::KeyUp(name) => Event::KeyUp(name),
                input::Event::Wheel(steps) => Event::Scroll(steps),
                input::Event::UseStart(using) => Event::UseStart(item_use(using)),
                input::Event::UseStop(using) => Event::UseStop(item_use(using)),
            })
            .collect()
    }

    pub(crate) fn input_held() -> Option<HeldItem> {
        input::held().map(held_item)
    }

    pub(crate) fn input_mapping(name: &str) -> Option<ControlState> {
        input::mapping(name).map(|m| ControlState {
            key: m.label,
            down: m.down,
        })
    }

    pub(crate) fn input_clicks() -> Clicks {
        let (attack, use_item) = input::clicks_per_second();
        Clicks { attack, use_item }
    }

    fn held_item(item: input::HeldItem) -> HeldItem {
        HeldItem {
            id: item.id,
            count: item.count,
            slot: item.slot,
        }
    }

    fn item_use(using: input::ItemUse) -> ItemUse {
        ItemUse {
            item: held_item(using.item),
            ticks: using.ticks,
        }
    }

    pub(crate) fn log(level: Level, message: &str) {
        let level = match level {
            Level::Debug => log::Level::Debug,
            Level::Info => log::Level::Info,
            Level::Warn => log::Level::Warn,
            Level::Error => log::Level::Error,
        };
        log::write(level, message);
    }

    pub(crate) fn chat_show(text: &str) {
        chat::show(text);
    }

    pub(crate) fn command_run(line: &str) {
        command::run(line);
    }

    pub(crate) fn command_run_quiet(lines: &[String]) {
        command::run_quiet(lines);
    }

    thread_local! {
        static PLAYER: RefCell<Option<Option<Player>>> = const { RefCell::new(None) };
        static CLOCK: Cell<Option<Option<Clock>>> = const { Cell::new(None) };
    }

    pub(crate) fn forget_snapshot() {
        PLAYER.with(|p| p.borrow_mut().take());
        CLOCK.with(|c| c.set(None));
        ONLINE.with(|o| o.borrow_mut().take());
    }

    thread_local! {
        static ONLINE: RefCell<Option<Vec<crate::server::Player>>> = const { RefCell::new(None) };
    }

    pub(crate) fn server_players() -> Vec<crate::server::Player> {
        ONLINE.with(|cached| {
            cached
                .borrow_mut()
                .get_or_insert_with(|| {
                    server_sdk::players::online()
                        .into_iter()
                        .map(crate::__private::player)
                        .collect()
                })
                .clone()
        })
    }

    pub(crate) fn server_tell(player: &str, text: &str) -> bool {
        server_sdk::players::tell(player, text)
    }

    pub(crate) fn server_broadcast(text: &str) {
        server_sdk::players::broadcast(text);
    }

    pub(crate) fn server_get_block(dimension: &str, pos: BlockPos) -> Option<String> {
        server_sdk::world::get_block(dimension, pos.x, pos.y, pos.z)
    }

    pub(crate) fn server_get_biome(dimension: &str, pos: BlockPos) -> Option<String> {
        server_sdk::world::get_biome(dimension, pos.x, pos.y, pos.z)
    }

    pub(crate) fn server_set_blocks(
        dimension: &str,
        changes: &[(BlockPos, String)],
    ) -> Result<(), String> {
        let changes: Vec<server_sdk::world::BlockChange> = changes
            .iter()
            .map(|(pos, state)| server_sdk::world::BlockChange {
                x: pos.x,
                y: pos.y,
                z: pos.z,
                state: state.clone(),
            })
            .collect();
        server_sdk::world::set_blocks(dimension, &changes)
    }

    pub(crate) fn player() -> Option<Player> {
        PLAYER.with(|cached| {
            cached
                .borrow_mut()
                .get_or_insert_with(|| {
                    game::get_player().map(|p| Player {
                        position: Vec3::new(p.x, p.y, p.z),
                        yaw: p.yaw,
                        pitch: p.pitch,
                        health: p.health,
                        food: p.food,
                        dimension: p.dimension.into(),
                    })
                })
                .clone()
        })
    }

    pub(crate) fn clock() -> Option<Clock> {
        CLOCK.with(|cached| {
            if let Some(clock) = cached.get() {
                return clock;
            }
            let clock = game::get_clock().map(|c| Clock {
                game_time: c.game_time,
                day_time: c.day_time,
            });
            cached.set(Some(clock));
            clock
        })
    }

    pub(crate) fn raycast(distance: f32) -> Option<Hit> {
        game::raycast(distance).map(|h| Hit {
            block_pos: BlockPos::new(h.block_x, h.block_y, h.block_z),
            point: Vec3::new(h.x, h.y, h.z),
            block: h.block,
        })
    }

    pub(crate) fn hud_set_panel(corner: Corner, lines: &[Line]) {
        let corner = match corner {
            Corner::TopLeft => hud::Corner::TopLeft,
            Corner::TopRight => hud::Corner::TopRight,
            Corner::BottomLeft => hud::Corner::BottomLeft,
            Corner::BottomRight => hud::Corner::BottomRight,
        };
        let lines: Vec<hud::Line> = lines
            .iter()
            .map(|l| hud::Line {
                text: l.text.clone(),
                color: l.color.to_rgb(),
            })
            .collect();
        hud::set_panel(corner, &lines);
    }

    pub(crate) fn hud_clear() {
        hud::clear();
    }

    pub(crate) fn storage_get(key: &str) -> Option<Vec<u8>> {
        storage::get(key)
    }

    pub(crate) fn storage_set(key: &str, value: &[u8]) -> Result<(), String> {
        storage::set(key, value)
    }

    pub(crate) fn storage_delete(key: &str) {
        storage::delete(key);
    }

    pub(crate) fn ui_set_settings(widgets: &[Widget]) {
        ui::set_settings(&to_host(widgets, false));
    }

    pub(crate) fn ui_open_screen(title: &str, widgets: &[Widget]) {
        ui::open_screen(&ui::Screen {
            title: title.to_owned(),
            widgets: to_host(widgets, true),
        });
    }

    pub(crate) fn ui_close_screen() {
        ui::close_screen();
    }

    fn to_host(widgets: &[Widget], screen: bool) -> Vec<ui::Widget> {
        widgets
            .iter()
            .enumerate()
            .map(|(index, widget)| {
                let id = widget_id(index, screen);
                match widget {
                    Widget::Text { text, color } => ui::Widget::Text(ui::TextWidget {
                        id,
                        text: text.clone(),
                        color: color.map_or(0, |c| c.to_rgb()),
                    }),
                    Widget::Button { label } => ui::Widget::Button(ui::ButtonWidget {
                        id,
                        label: label.clone(),
                    }),
                    Widget::Item { item, name } => ui::Widget::Item(ui::ItemWidget {
                        id,
                        item: item.clone(),
                        name: name.clone(),
                    }),
                    Widget::List { rows, selected } => ui::Widget::List(ui::ListWidget {
                        id,
                        rows: rows.clone(),
                        selected: *selected,
                    }),
                }
            })
            .collect()
    }
}
