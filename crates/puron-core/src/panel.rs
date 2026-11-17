//! What the panel shows: one item per pin of a shown class, or a single
//! icon when there is none. This module builds the items and draws
//! nothing. Its state (`State`) is returned to the caller, who passes it
//! back on the next call: the icon memory, and the box memory that keeps
//! the panel from changing width each time a value gets shorter.
//!
//! A pin's *box* is the room its value text takes on the panel. The box is
//! as wide as the widest of the value and the pin's *box text*: a text of
//! the form of the pin's values, written with the digit 8 (the widest
//! digit in common interface fonts), such as `8888 rpm` for a fan. The
//! box text has at least the usual number of digits of its class and
//! grows when a value needs more, so a value that gets shorter leaves its
//! box as wide as before, and the pins after it do not move.

use std::collections::BTreeMap;

use crate::format::{format, Text, TEMPERATURE_DIGITS, VOLTAGE_POWER_DIGITS};
use crate::menu::{icon_kind, IconKind, Shown};
use crate::rows::{class_of_identity, Class};
use crate::settings::{Settings, TemperatureUnit, TextSize};

/// The padding in pixels on each side of an item's label when panel icons
/// are shown.
pub const PADDING_WITH_ICONS: u32 = 0;
/// The same padding when panel icons are switched off.
pub const PADDING_WITHOUT_ICONS: u32 = 4;

/// The icon next to a pinned reading in the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// Panel icons are switched off (`Settings::show_panel_icons`).
    None,
    /// The icon of the reading's class and family.
    Kind(IconKind),
    /// Icons are on, but the pin's line has not been found since icons
    /// were switched on or the state was created, so its kind is unknown.
    EmptySlot,
}

/// One panel item: a pin, or the empty state when no pin is of a shown
/// class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// `value` is `None` when the pin has no line in the popup (the panel
    /// shows "⚠"), and `Some(None)` when its line has no value.
    Pin {
        icon: Icon,
        value: Option<Option<Text>>,
    },
    /// Shown alone when there is no item to show: an icon and no value.
    EmptyState,
}

/// What `panel` remembers between calls. `State::default()` is the state
/// at start-up: nothing remembered, so every box starts at the usual
/// width of its class.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// The icon memory: the icon kind last found for each pin, so that a
    /// pin whose line disappears keeps its icon.
    pub icons: BTreeMap<String, IconKind>,
    /// The box memory: for each pin item, the number of digits before the
    /// point of its box text. It only grows while the pin stays on the
    /// panel, so a box never gets narrower.
    pub boxes: BTreeMap<String, usize>,
    /// The settings that the box memory was built under; `None` at
    /// start-up.
    pub box_keys: Option<BoxKeys>,
}

/// The settings that change the panel's value texts or their size. When
/// any of them differs from the ones the box memory was built under, the
/// memory is emptied: a box grown for one unit or text size would
/// otherwise stay too wide under another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoxKeys {
    pub temperature_unit: TemperatureUnit,
    pub show_decimals: bool,
    pub show_temperature_unit: bool,
    pub show_rotation_rate_unit: bool,
    pub show_voltage_unit: bool,
    pub show_power_unit: bool,
    pub panel_text_size: TextSize,
    pub dock_text_size: TextSize,
}

impl BoxKeys {
    /// The box keys of `settings`.
    pub fn of(settings: &Settings) -> BoxKeys {
        let s = settings;
        BoxKeys {
            temperature_unit: s.temperature_unit,
            show_decimals: s.show_decimals,
            show_temperature_unit: s.show_temperature_unit,
            show_rotation_rate_unit: s.show_rotation_rate_unit,
            show_voltage_unit: s.show_voltage_unit,
            show_power_unit: s.show_power_unit,
            panel_text_size: s.panel_text_size,
            dock_text_size: s.dock_text_size,
        }
    }
}

/// What `panel` returns: the items, the label padding, and the state to
/// pass back on the next call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    pub items: Vec<Item>,
    /// Label padding in px, on each side.
    pub padding: u32,
    pub state: State,
    /// The box text of each item, at the item's index in `items`. It is
    /// `None` for the empty-state item and for a pin whose entry names no
    /// class, such as a text that is no identity: no usual width is known
    /// for it, so its box is as wide as its widest text alone.
    pub boxes: Vec<Option<Text>>,
}

/// The panel for the pinned list `pinned`, given the popup's lines `shown`
/// and the state that the previous call returned.
pub fn panel(shown: &Shown, pinned: &[String], settings: &Settings, state: &State) -> Panel {
    let s = settings;
    let icons_on = s.show_panel_icons;
    // Remember the icon kind of each pin whose line is found now. With
    // icons off, the memory is cleared and nothing is remembered.
    let mut icons = if icons_on {
        state.icons.clone()
    } else {
        BTreeMap::new()
    };
    if icons_on {
        for id in pinned {
            if let Some((kind, _)) = found(shown, id, s) {
                icons.insert(id.clone(), kind);
            }
        }
    }
    // Forget the icons of entries that are no longer pinned.
    icons.retain(|id, _| pinned.contains(id));
    // One item per pin of a shown class, in popup order; pins with no
    // line in the popup follow, in pinned-list order.
    let mut visible: Vec<&String> = pinned.iter().filter(|id| !hidden(id, s)).collect();
    visible.sort_by_key(|id| shown.place(id).unwrap_or(usize::MAX));
    let mut items: Vec<Item> = visible
        .into_iter()
        .map(|id| {
            let icon = if !icons_on {
                Icon::None
            } else {
                icons.get(id).map_or(Icon::EmptySlot, |k| Icon::Kind(*k))
            };
            // `None` whenever `shown` has no line for the pin, whatever the
            // reason.
            let value = found(shown, id, s).map(|(_, v)| v);
            Item::Pin { icon, value }
        })
        .collect();
    if items.is_empty() {
        // Nothing is pinned, or every pin is of a hidden class.
        items.push(Item::EmptyState);
    }
    let padding = if icons_on {
        PADDING_WITH_ICONS
    } else {
        PADDING_WITHOUT_ICONS
    };
    Panel {
        items,
        padding,
        state: State {
            icons,
            ..State::default()
        },
        boxes: Vec::new(),
    }
    .with_boxes(shown, pinned, s, state)
}

/// The digits before the point that a box of `class` starts with: as many
/// as a value of the class usually has, so that the box does not grow in
/// normal use.
fn start_digits(class: Class) -> usize {
    match class {
        Class::Fan => 4,
        Class::Temperature => 3,
        Class::Voltage => 2,
        Class::Power => 3,
    }
}

/// The number of digits before the point of a pin's value: the ASCII
/// digits of its number up to the first `.`. A sign is not a digit. A pin
/// with no line, or whose line has no value, has none.
fn value_digits(value: &Option<Option<Text>>) -> usize {
    match value {
        Some(Some(t)) => t
            .number
            .split('.')
            .next()
            .map_or(0, |whole| whole.bytes().filter(u8::is_ascii_digit).count()),
        _ => 0,
    }
}

/// The box text of a pin of `class` whose box has `digits` digits before
/// the point: the form that `format` gives a value of the class under
/// `settings`, with every digit an 8. A voltage or power carries its sign,
/// and the number of decimals is that of `format`.
fn box_text(class: Class, digits: usize, settings: &Settings) -> Text {
    let whole = "8".repeat(digits);
    let after = match class {
        Class::Fan => 0,
        Class::Temperature => TEMPERATURE_DIGITS[usize::from(settings.show_decimals)],
        Class::Voltage | Class::Power => VOLTAGE_POWER_DIGITS,
    };
    let sign = match class {
        Class::Voltage | Class::Power => "+",
        Class::Fan | Class::Temperature => "",
    };
    let number = if after == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{}", "8".repeat(after))
    };
    // The unit, or none when the settings hide it, is the one that
    // `format` gives every value of the class, so a box text and the
    // values in its box always agree.
    let unit = format(class, Some(0.0), settings).and_then(|t| t.unit);
    Text { number, unit }
}

/// The pinned entries that give an item, in the order of the items:
/// those that the settings do not hide, in popup order, then those with
/// no line in the popup, in pinned-list order. This is the order in which
/// `panel` builds its items.
fn item_identities<'a>(
    shown: &Shown,
    pinned: &'a [String],
    settings: &Settings,
) -> Vec<&'a String> {
    let mut ids: Vec<&String> = pinned.iter().filter(|id| !hidden(id, settings)).collect();
    ids.sort_by_key(|id| shown.place(id).unwrap_or(usize::MAX));
    ids
}

impl Panel {
    /// Adds the box texts to a panel whose items are built, and the box
    /// memory to its state, from the box memory of `old`, the state that
    /// the previous call returned.
    fn with_boxes(
        mut self,
        shown: &Shown,
        pinned: &[String],
        settings: &Settings,
        old: &State,
    ) -> Panel {
        let keys = BoxKeys::of(settings);
        // A changed unit, decimal setting or text size starts every box
        // again. The state at start-up has no keys and nothing to empty.
        let memory = if old.box_keys.as_ref().is_some_and(|k| *k != keys) {
            BTreeMap::new()
        } else {
            old.boxes.clone()
        };
        let ids = item_identities(shown, pinned, settings);
        // Only the pins that give an item keep an entry: a pin that is
        // unpinned or hidden starts again at its class's usual width when
        // it comes back. A pin whose line is missing still gives an item,
        // so it keeps its box.
        let mut kept = BTreeMap::new();
        let mut boxes = Vec::with_capacity(self.items.len());
        for (i, item) in self.items.iter().enumerate() {
            let text = match (item, ids.get(i)) {
                (Item::Pin { value, .. }, Some(id)) => class_of_identity(id).map(|class| {
                    let digits = memory
                        .get(*id)
                        .copied()
                        .unwrap_or(0)
                        .max(start_digits(class))
                        .max(value_digits(value));
                    kept.insert((*id).clone(), digits);
                    box_text(class, digits, settings)
                }),
                _ => None,
            };
            boxes.push(text);
        }
        self.boxes = boxes;
        self.state.boxes = kept;
        self.state.box_keys = Some(keys);
        self
    }
}

/// The icon kind and value text of the line of `id`, or `None` when
/// `shown` has no such line.
fn found(shown: &Shown, id: &str, settings: &Settings) -> Option<(IconKind, Option<Text>)> {
    let line = shown.line(id)?;
    // The line's value goes through `format`, as in the menu.
    Some((icon_kind(line), format(line.class, line.value, settings)))
}

/// Whether the settings hide the line of pin `id`: its class is switched
/// off (`show_temperature`, `show_rotation_rate`, `show_voltage`,
/// `show_power`), or it is a line of a drive while lm-sensors does not
/// read drives (`provider_lm_sensors_drives`). Such a pin has no item on
/// the panel.
fn hidden(id: &str, settings: &Settings) -> bool {
    settings.shown_classes().hides(id)
}
