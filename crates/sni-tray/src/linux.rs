use std::sync::Arc;

use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};
use ksni::{Category, Icon, ToolTip};

/// What the tray's clicks and menu do. Called on the tray's own thread.
pub struct Actions {
    /// Left click (with the click's screen position when the desktop reports one), or "Open".
    pub open: Box<dyn Fn(Option<(i32, i32)>) + Send + Sync>,
    pub refresh: Box<dyn Fn() + Send + Sync>,
    /// "Change LiteLLM URL…"
    pub settings: Box<dyn Fn() + Send + Sync>,
    pub quit: Box<dyn Fn() + Send + Sync>,
}

struct Tray {
    actions: Arc<Actions>,
    /// ARGB32, network byte order.
    icon: Icon,
    tooltip: String,
}

impl ksni::Tray for Tray {
    const MENU_ON_ACTIVATE: bool = false;

    fn id(&self) -> String {
        "litellm-usage".into()
    }

    fn title(&self) -> String {
        "LiteLLM Usage".into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn activate(&mut self, x: i32, y: i32) {
        let pos = (x > 0 || y > 0).then_some((x, y));
        (self.actions.open)(pos);
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![self.icon.clone()]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip { title: self.tooltip.clone(), ..Default::default() }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let item = |label: &str, act: fn(&Actions)| -> MenuItem<Self> {
            StandardItem { label: label.into(), activate: Box::new(move |t: &mut Self| act(&t.actions)), ..Default::default() }
                .into()
        };
        vec![
            item("Open", |a| (a.open)(None)),
            item("Refresh", |a| (a.refresh)()),
            item("Change LiteLLM URL…", |a| (a.settings)()),
            MenuItem::Separator,
            item("Quit", |a| (a.quit)()),
        ]
    }
}

/// A running tray icon.
pub struct SniTray(Handle<Tray>);

impl SniTray {
    /// Starts the icon. Fails when the desktop has no StatusNotifierItem host — fall back to the
    /// framework's own tray then.
    pub fn spawn(actions: Actions, rgba: &[u8], size: u32, tooltip: &str) -> Result<SniTray, String> {
        let tray = Tray { actions: Arc::new(actions), icon: icon(rgba, size), tooltip: tooltip.into() };
        tray.spawn().map(SniTray).map_err(|e| e.to_string())
    }

    /// New icon pixels (RGBA, `size`×`size`) and hover text.
    pub fn update(&self, rgba: &[u8], size: u32, tooltip: &str) {
        let icon = icon(rgba, size);
        let tooltip = tooltip.to_string();
        self.0.update(move |t| {
            t.icon = icon;
            t.tooltip = tooltip;
        });
    }
}

/// RGBA → the ARGB32 (network byte order) that StatusNotifierItem expects.
fn icon(rgba: &[u8], size: u32) -> Icon {
    let data = rgba.chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
    Icon { width: size as i32, height: size as i32, data }
}
