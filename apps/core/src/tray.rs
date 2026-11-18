//! Icona e menu del tray (§13 «Fuori dalla finestra»).

use anyhow::Context;
use falco_proto::Status;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::core::{TrayCommand, TrayUpdate};
use crate::i18n;

const ICON_PNG: &[u8] = include_bytes!("../../ui/src-tauri/icons/32x32.png");

pub struct Tray {
    _icon: TrayIcon,
    status_menu: Submenu,
    online: CheckMenuItem,
    busy: CheckMenuItem,
    invisible: CheckMenuItem,
    available: CheckMenuItem,
    open: MenuItem,
    quit: MenuItem,
}

impl Tray {
    pub fn build(state: &TrayUpdate) -> anyhow::Result<Self> {
        let t = i18n::tray(state.language);
        let online = CheckMenuItem::new(t.online, true, false, None);
        let busy = CheckMenuItem::new(t.busy, true, false, None);
        let invisible = CheckMenuItem::new(t.invisible, true, false, None);
        let status_menu = Submenu::with_items(t.status, true, &[&online, &busy, &invisible])?;
        let available = CheckMenuItem::new(t.available, true, false, None);
        let open = MenuItem::new(t.open, true, None);
        let quit = MenuItem::new(t.quit, true, None);

        let menu = Menu::with_items(&[
            &status_menu,
            &available,
            &PredefinedMenuItem::separator(),
            &open,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;

        let img = image::load_from_memory(ICON_PNG).context("icona del tray")?.into_rgba8();
        let (w, h) = img.dimensions();
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(t.tooltip)
            .with_icon(Icon::from_rgba(img.into_raw(), w, h)?)
            .build()?;

        let mut tray =
            Self { _icon: icon, status_menu, online, busy, invisible, available, open, quit };
        tray.apply(state);
        Ok(tray)
    }

    pub fn command_for(&self, e: &MenuEvent) -> Option<TrayCommand> {
        let id = e.id();
        Some(if id == self.online.id() {
            TrayCommand::SetStatus(Status::Online)
        } else if id == self.busy.id() {
            TrayCommand::SetStatus(Status::Busy)
        } else if id == self.invisible.id() {
            TrayCommand::SetStatus(Status::Invisible)
        } else if id == self.available.id() {
            TrayCommand::SetAvailable(self.available.is_checked())
        } else if id == self.open.id() {
            TrayCommand::OpenUi
        } else if id == self.quit.id() {
            TrayCommand::Quit
        } else {
            return None;
        })
    }

    pub fn apply(&mut self, u: &TrayUpdate) {
        let t = i18n::tray(u.language);
        self.status_menu.set_text(t.status);
        self.online.set_text(t.online);
        self.busy.set_text(t.busy);
        self.invisible.set_text(t.invisible);
        self.available.set_text(t.available);
        self.open.set_text(t.open);
        self.quit.set_text(t.quit);
        self.online.set_checked(u.status == Status::Online);
        self.busy.set_checked(u.status == Status::Busy);
        self.invisible.set_checked(u.status == Status::Invisible);
        self.available.set_checked(u.host_available);
    }
}
