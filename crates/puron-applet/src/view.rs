//! What the libcosmic side of the applet draws. After each event the
//! event loop sends a `View`, a copy of the parts of `App` that are drawn,
//! and the panel button and the popup are planned from it alone. Because
//! the settings travel with it, a new text size shows on the panel button
//! at the next drawing, from the last readings, with no work in the loop.

use puron_core::menu::Menu;
use puron_core::panel::Panel;
use puron_core::settings::Settings;

use crate::app::App;
use crate::panel::{self, Place};
use crate::popup;

/// A copy of the drawn parts of `App`. `menu` is `None` until the first
/// layout.
#[derive(Clone, Debug)]
pub struct View {
    pub menu: Option<Menu>,
    pub panel: Panel,
    pub pinned: Vec<String>,
    pub settings: Settings,
}

impl View {
    /// What is drawn before the event loop sends its first view: no
    /// panel items, no popup layout and the default settings.
    pub fn new() -> View {
        View {
            menu: None,
            panel: Panel {
                items: Vec::new(),
                padding: 0,
                state: Default::default(),
                boxes: Vec::new(),
            },
            pinned: Vec::new(),
            settings: Settings::default(),
        }
    }

    /// The drawn parts of `app` as they are now.
    pub fn of(app: &App) -> View {
        View {
            menu: app.menu().cloned(),
            panel: app.panel().clone(),
            pinned: app.pinned().to_vec(),
            settings: app.settings().clone(),
        }
    }

    /// The panel button's plan at `place`, with the text size that the
    /// settings give for that place.
    pub fn panel_plan(&self, place: &Place) -> panel::Plan {
        panel::plan(&self.panel, place, &self.settings)
    }

    /// The popup's plan, once there is a layout. The popup does not read
    /// the text size settings: its texts keep libcosmic's applet text
    /// size on a panel and on a dock alike, so that a large panel text
    /// does not make the list of every sensor grow with it.
    pub fn popup_plan(&self, place: &Place) -> Option<popup::Plan> {
        let menu = self.menu.as_ref()?;
        Some(popup::plan(menu, &self.pinned, place.icon_size))
    }

    /// The popup's width in px: fitted to the layout, or the narrowest
    /// width before the first layout.
    pub fn popup_width(&self, place: &Place) -> f32 {
        self.menu
            .as_ref()
            .map_or(popup::WIDTH, |m| popup::width(m, place.icon_size))
    }
}

impl Default for View {
    fn default() -> View {
        View::new()
    }
}
