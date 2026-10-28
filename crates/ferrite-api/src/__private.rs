use crate::ui::{self, Page, PageBuilder, SCREEN_IDS, ScreenRequest};
use crate::{Args, Hooks, Mod, host};

pub mod sdk {
    wit_bindgen::generate!({
        path: "wit",
        world: "horizon-ui-mod",
        pub_export_macro: true,
        export_macro_name: "__export_horizon_mod",
        default_bindings_module: "ferrite_api::__private::sdk",
    });

    pub use exports::horizon::mods::events::{Guest, Registration};
    pub use exports::horizon::mods::ui_events::Guest as UiEventsGuest;
    pub use horizon::mods::{chat, command, game, hud, log, storage, ui};
}

struct Screen<M> {
    builder: PageBuilder<M>,
    page: Page<M>,
}

pub struct Adapter<M> {
    inner: M,
    hooks: Hooks<M>,
    ticks: u64,
    settings: Option<Page<M>>,
    screen: Option<Screen<M>>,
}

impl<M: Mod> Adapter<M> {
    pub fn init() -> Result<(Self, sdk::Registration), String> {
        host::forget_snapshot();
        let mut hooks = Hooks::new();
        let inner = M::init(&mut hooks).map_err(crate::Error::into_message)?;
        let registration = hooks.registration();
        let mut adapter = Self {
            inner,
            hooks,
            ticks: 0,
            settings: None,
            screen: None,
        };
        adapter.refresh_settings(true);
        adapter.flush_ui(false);
        Ok((adapter, registration))
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
        self.flush_ui(false);
    }

    pub fn frame(&mut self, seconds: f32) {
        host::forget_snapshot();
        for handler in &self.hooks.frame {
            handler(&mut self.inner, seconds);
        }
        self.flush_ui(false);
    }

    pub fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        host::forget_snapshot();
        let result = match self.hooks.find_command(name) {
            Some(handler) => handler(&mut self.inner, Args::for_command(name, args))
                .map_err(crate::Error::into_message),
            None => Err(format!("Unknown command /{name}")),
        };
        self.flush_ui(true);
        result
    }

    pub fn chat(&mut self, text: &str) {
        host::forget_snapshot();
        for handler in &self.hooks.chat {
            handler(&mut self.inner, text);
        }
        self.flush_ui(true);
    }

    pub fn ui_select(&mut self, id: u32, row: u32) {
        host::forget_snapshot();
        let acted = match id.checked_sub(SCREEN_IDS) {
            Some(index) => self
                .screen
                .as_ref()
                .is_some_and(|screen| screen.page.activate(&mut self.inner, index, row)),
            None => self
                .settings
                .as_ref()
                .is_some_and(|page| page.activate(&mut self.inner, id, row)),
        };
        self.flush_ui(acted);
    }

    pub fn ui_closed(&mut self) {
        host::forget_snapshot();
        if self.screen.take().is_some() {
            for handler in &self.hooks.screen_closed {
                handler(&mut self.inner);
            }
        }
        self.flush_ui(true);
    }

    pub fn ui_settings_opened(&mut self) {
        host::forget_snapshot();
        self.refresh_settings(true);
        self.flush_ui(false);
    }

    fn flush_ui(&mut self, refresh: bool) {
        let Some(host) = ui::active_host() else {
            return;
        };
        let requests = ui::take_requests();
        match requests.screen {
            Some(ScreenRequest::Open(builder)) => {
                if let Ok(builder) = builder.downcast::<PageBuilder<M>>() {
                    let page = Page::build(&self.inner, *builder);
                    (host.open_screen)(page.title(), page.widgets());
                    self.screen = Some(Screen {
                        builder: *builder,
                        page,
                    });
                }
            }
            Some(ScreenRequest::Close) => {
                let was_open = self.screen.take().is_some();
                if was_open {
                    (host.close_screen)();
                }
            }
            None => {}
        }
        if refresh || requests.refresh {
            self.refresh_settings(false);
            self.refresh_screen();
        }
    }

    fn refresh_settings(&mut self, force: bool) {
        let (Some(builder), Some(host)) = (self.hooks.settings, ui::active_host()) else {
            return;
        };
        let page = Page::build(&self.inner, builder);
        let changed = self
            .settings
            .as_ref()
            .is_none_or(|old| old.widgets() != page.widgets());
        if force || changed {
            (host.set_settings)(page.widgets());
        }
        self.settings = Some(page);
    }

    fn refresh_screen(&mut self) {
        if let (Some(screen), Some(host)) = (self.screen.as_mut(), ui::active_host()) {
            let page = Page::build(&self.inner, screen.builder);
            if page.title() != screen.page.title() || page.widgets() != screen.page.widgets() {
                (host.open_screen)(page.title(), page.widgets());
            }
            screen.page = page;
        }
    }
}
