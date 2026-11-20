//! The popup's Settings page: what it shows, how its controls change the
//! settings, and the page drawn from it. The page is the popup's third
//! page: the entry "Settings" of the temperatures shows it, and its
//! "Back" returns, as for the About page.
//!
//! The page has 20 items, numbered from 1 in the order they are shown,
//! in five sections. Messages and the tables below address items by that
//! number. `plan` describes the page as data, which the tests read
//! without a display, and `view` turns a plan into widgets.
//!
//! The page holds no state of its own. `app::App` keeps the values the
//! controls show and the text being typed into the number field, and
//! hands them to `edit` and `leave`, which say what to store. A stored
//! value is applied only when the settings watch reports it, as a change
//! made by another program is, so a change is never applied twice and a
//! store that fails is never applied.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, column, container, divider, dropdown, icon, row, scrollable, settings, text,
    text_input, toggler, tooltip, Id,
};
use puron_config::Value;
use puron_core::settings::{
    Settings, TemperatureUnit, TextSize, POLL_INTERVAL_MAX, POLL_INTERVAL_MIN,
};

use crate::about::BACK_ICON;
use crate::look;
use crate::popup::{menu_button, padded, Event, Reply, PAGE_SIDE_PADDING};

/// The icon name of the number field's − button.
pub const MINUS_ICON: &str = "list-remove-symbolic";
/// The icon name of the number field's + button.
pub const PLUS_ICON: &str = "list-add-symbolic";
/// The width of the number field's text input, in logical pixels.
pub const INPUT_WIDTH: f32 = 48.0;

/// What the page is drawn from.
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    /// The value each control shows.
    pub values: Settings,
    /// The text being typed into the number field and not yet taken, if
    /// any. The field shows it in place of the value.
    pub typed: Option<String>,
    /// Whether the latest store of a control's setting failed.
    pub notice: bool,
}

/// The control of an item, with what it currently shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// The number field (item 1, the poll interval): its value, the text
    /// it shows (the typed text while there is one, else the value),
    /// whether − and + are enabled, and whether typed text waits to be
    /// taken.
    Number {
        value: u32,
        text: String,
        minus: bool,
        plus: bool,
        typing: bool,
    },
    Switch(bool),
    /// A drop-down list (the two text sizes and the temperature unit):
    /// its entry texts and the index of the selected one.
    List {
        items: Vec<String>,
        shown: usize,
    },
}

/// One row of a section: a label and the control that edits one setting.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub label: String,
    pub control: Kind,
    /// The settings key the control stores (one of `puron_config::KEYS`).
    pub key: &'static str,
    /// The tooltip, if the item has one: the ID of its translation
    /// message and the translated text.
    pub tooltip: Option<(&'static str, String)>,
}

/// A group of items under a heading.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub heading: String,
    pub items: Vec<Item>,
}

/// One drawn entry of the page, from top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// The entry that returns to the temperatures: an icon from the COSMIC
    /// theme and the text "Back".
    Back {
        icon: &'static str,
        text: String,
    },
    Separator,
    /// The line shown while the latest store of a setting has failed.
    Notice(String),
    Section(Section),
}

/// The page's entries in the order they are drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub entries: Vec<Entry>,
    /// The space in px at the left and right of the page, always
    /// `popup::PAGE_SIDE_PADDING`.
    pub side_padding: u16,
}

impl Plan {
    /// The sections, in order.
    pub fn sections(&self) -> Vec<&Section> {
        self.entries
            .iter()
            .filter_map(|e| match e {
                Entry::Section(s) => Some(s),
                _ => None,
            })
            .collect()
    }

    /// Every item across the sections; item `n` is at index `n - 1`.
    pub fn items(&self) -> Vec<&Item> {
        self.sections()
            .into_iter()
            .flat_map(|s| s.items.iter())
            .collect()
    }

    /// The notice's text, while the page shows one.
    pub fn notice(&self) -> Option<&str> {
        self.entries.iter().find_map(|e| match e {
            Entry::Notice(t) => Some(t.as_str()),
            _ => None,
        })
    }
}

/// The new value a switch or a list gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// A switch turned on (true) or off.
    Switch(bool),
    /// A list's entry at this index: the panel or dock text size, in the
    /// order of `TextSize::ALL`, or the temperature unit, in the order of
    /// `UNITS`.
    Entry(usize),
}

/// A message of one of the page's controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// The switch or list of item `item` was set to `choice`.
    Change { item: usize, choice: Choice },
    /// The number field's text, as it stands after a keystroke.
    Typed(String),
    /// Enter in the number field.
    Enter,
    /// The number field lost the keyboard focus.
    FocusLost,
    /// The number field's + button.
    Plus,
    /// The number field's − button.
    Minus,
}

/// The widget message of a click or a keystroke on the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clicked {
    Back,
    Edit(Edit),
}

/// The settings key of each item: entry `n - 1` belongs to item `n`.
/// `debug_logging` and `pinned` have no item; the page does not edit
/// them.
pub const KEYS: [&str; 20] = [
    puron_config::POLL_INTERVAL_SECS,
    puron_config::SHOW_PANEL_ICONS,
    puron_config::PANEL_TEXT_SIZE,
    puron_config::DOCK_TEXT_SIZE,
    puron_config::TEMPERATURE_UNIT,
    puron_config::SHOW_TEMPERATURE_UNIT,
    puron_config::SHOW_ROTATION_RATE_UNIT,
    puron_config::SHOW_VOLTAGE_UNIT,
    puron_config::SHOW_POWER_UNIT,
    puron_config::SHOW_DECIMALS,
    puron_config::PROVIDER_LM_SENSORS,
    puron_config::PROVIDER_LIQUIDCTL,
    puron_config::PROVIDER_NVIDIA,
    puron_config::PROVIDER_LM_SENSORS_DRIVES,
    puron_config::PROVIDER_UDISKS2,
    puron_config::PROVIDER_NVME,
    puron_config::SHOW_TEMPERATURE,
    puron_config::SHOW_ROTATION_RATE,
    puron_config::SHOW_VOLTAGE,
    puron_config::SHOW_POWER,
];

/// The five sections, as the first and last item number of each. `heading`
/// gives their headings in the same order.
const SECTIONS: [(usize, usize); 5] = [(1, 10), (11, 12), (13, 13), (14, 16), (17, 20)];

/// The translated heading of section `section`, counted from 0.
fn heading(section: usize) -> String {
    match section {
        0 => fl!("heading-display"),
        1 => fl!("heading-generic"),
        2 => fl!("heading-gpu"),
        3 => fl!("heading-drive"),
        _ => fl!("heading-shown"),
    }
}

/// The label of item `item`. Items 11 to 16 switch a sensor source on or
/// off. All of them but item 14 are labelled with the source's product
/// name, which is not translated. Item 14 switches a part of a source,
/// the drives that lm-sensors reads, so its label holds a translated
/// word. The other labels are translated.
fn label(item: usize) -> String {
    match item {
        1 => fl!("label-poll"),
        2 => fl!("label-panel-icons"),
        3 => fl!("label-panel-text-size"),
        4 => fl!("label-dock-text-size"),
        5 => fl!("label-temp-unit"),
        6 => fl!("label-temp-suffix"),
        7 => fl!("label-fan-suffix"),
        8 => fl!("label-volt-suffix"),
        9 => fl!("label-watt-suffix"),
        10 => fl!("label-decimals"),
        11 => "lm-sensors".to_string(),
        12 => "liquidctl".to_string(),
        13 => "Nvidia".to_string(),
        14 => fl!("label-lm-sensors-drives"),
        15 => "Udisks2".to_string(),
        16 => "nvme-cli".to_string(),
        17 => fl!("class-temperature"),
        18 => fl!("class-rotation"),
        19 => fl!("class-voltage"),
        _ => fl!("class-power"),
    }
}

/// The tooltip of item `item`; only items 3, 4, 10, 11, 12 and 14 have
/// one.
fn tooltip_of(item: usize) -> Option<(&'static str, String)> {
    match item {
        3 => Some(("tip-panel-text-size", fl!("tip-panel-text-size"))),
        4 => Some(("tip-dock-text-size", fl!("tip-dock-text-size"))),
        10 => Some(("tip-decimals", fl!("tip-decimals"))),
        11 => Some(("tip-lm-sensors", fl!("tip-lm-sensors"))),
        12 => Some(("tip-liquidctl", fl!("tip-liquidctl"))),
        14 => Some(("tip-lm-sensors-drives", fl!("tip-lm-sensors-drives"))),
        _ => None,
    }
}

/// The entry texts of a text size list, in the order of `TextSize::ALL`.
/// The match value has its own text in each list, `match_text`; a fixed
/// size is its number joined to the translated unit, so the number itself
/// is never looked up in the catalog.
fn text_size_items(match_text: String) -> Vec<String> {
    let px = fl!("suffix-px");
    TextSize::ALL
        .iter()
        .map(|t| match t.fixed_px() {
            Some(n) => format!("{n}{px}"),
            None => match_text.clone(),
        })
        .collect()
}

/// The setting in `s` that the text size list of item `item` changes.
/// `None` for any item but 3 and 4.
fn text_size(s: &mut Settings, item: usize) -> Option<&mut TextSize> {
    match item {
        3 => Some(&mut s.panel_text_size),
        4 => Some(&mut s.dock_text_size),
        _ => None,
    }
}

/// The temperature units in the order of the list's entries.
pub const UNITS: [TemperatureUnit; 2] = [TemperatureUnit::Celsius, TemperatureUnit::Fahrenheit];

/// The list's entry texts, in the order of `UNITS`.
fn unit_items() -> Vec<String> {
    vec![fl!("suffix-celsius"), fl!("suffix-fahrenheit")]
}

/// The setting in `s` that the switch of item `item` changes. `None` for
/// an item without a switch (1, 3, 4 and 5) or a number outside 1 to 20.
fn switch(s: &mut Settings, item: usize) -> Option<&mut bool> {
    Some(match item {
        2 => &mut s.show_panel_icons,
        6 => &mut s.show_temperature_unit,
        7 => &mut s.show_rotation_rate_unit,
        8 => &mut s.show_voltage_unit,
        9 => &mut s.show_power_unit,
        10 => &mut s.show_decimals,
        11 => &mut s.provider_lm_sensors,
        12 => &mut s.provider_liquidctl,
        13 => &mut s.provider_nvidia,
        14 => &mut s.provider_lm_sensors_drives,
        15 => &mut s.provider_udisks2,
        16 => &mut s.provider_nvme,
        17 => &mut s.show_temperature,
        18 => &mut s.show_rotation_rate,
        19 => &mut s.show_voltage,
        20 => &mut s.show_power,
        _ => return None,
    })
}

/// The poll interval, in seconds, that the typed text `text` gives.
///
/// Leading ASCII white space is skipped. The longest prefix that reads as
/// a decimal number is taken: an optional sign, digits with at most one
/// decimal point, and an exponent only if the exponent has a digit. The
/// rest of the text is ignored, and text with no digit at its start reads
/// as 0. The number is then clamped to 1..=60 and its fraction dropped,
/// so "abc" gives 1, "7.9s" gives 7 and "1e3" gives 60.
pub fn read_interval(text: &str) -> u32 {
    let t = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut digits = 0;
    let mut dot = false;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            digits += 1;
        } else if b[i] == b'.' && !dot {
            dot = true;
        } else {
            break;
        }
        i += 1;
    }
    let number = if digits == 0 {
        0.0
    } else {
        // Take the exponent only if it has at least one digit, so that a
        // trailing "e" or "e+" is ignored.
        let mut j = i;
        if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
            j += 1;
            if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                j += 1;
            }
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                i = j;
            }
        }
        t[..i].parse::<f64>().unwrap_or(0.0)
    };
    number.clamp(POLL_INTERVAL_MIN as f64, POLL_INTERVAL_MAX as f64) as u32
}

/// A setting to store: its key and its new value.
pub type Store = (&'static str, Value);

/// Sets the shown poll interval to `value`, and asks to store it when it
/// differs from the shown one.
fn set_interval(values: &mut Settings, value: u32) -> Vec<Store> {
    if value == values.poll_interval_secs {
        return Vec::new();
    }
    values.poll_interval_secs = value;
    vec![(KEYS[0], Value::U32(value))]
}

/// Takes the typed text, if any: reads it with `read_interval` and asks to
/// store the result when it differs from the shown interval.
fn take_typed(values: &mut Settings, typed: &mut Option<String>) -> Vec<Store> {
    match typed.take() {
        Some(text) => set_interval(values, read_interval(&text)),
        None => Vec::new(),
    }
}

/// Applies one control's message to the shown `values` and the `typed`
/// text, and returns what to store, in order. A switch, a list choice, a
/// button press or taken text asks for a store at once, but only when it
/// changes the shown value; a keystroke changes only the typed text.
pub fn edit(values: &mut Settings, typed: &mut Option<String>, e: Edit) -> Vec<Store> {
    match e {
        Edit::Change {
            item: 5,
            choice: Choice::Entry(index),
        } => {
            let Some(&unit) = UNITS.get(index) else {
                return Vec::new();
            };
            if unit == values.temperature_unit {
                return Vec::new();
            }
            values.temperature_unit = unit;
            vec![(KEYS[4], Value::TemperatureUnit(unit))]
        }
        Edit::Change {
            item,
            choice: Choice::Entry(index),
        } => {
            let Some(&size) = TextSize::ALL.get(index) else {
                return Vec::new();
            };
            let Some(slot) = text_size(values, item) else {
                return Vec::new();
            };
            if *slot == size {
                return Vec::new();
            }
            *slot = size;
            vec![(KEYS[item - 1], Value::TextSize(size))]
        }
        Edit::Change {
            item,
            choice: Choice::Switch(on),
        } => {
            let Some(slot) = switch(values, item) else {
                return Vec::new();
            };
            if *slot == on {
                return Vec::new();
            }
            *slot = on;
            vec![(KEYS[item - 1], Value::Bool(on))]
        }
        // Whether a button acts depends on the shown value, not on any
        // typed text. A press first takes the typed text, then steps the
        // result by one if that stays within 1 to 60.
        Edit::Plus => {
            if values.poll_interval_secs >= POLL_INTERVAL_MAX {
                return Vec::new();
            }
            let mut out = take_typed(values, typed);
            let v = values.poll_interval_secs;
            if v < POLL_INTERVAL_MAX {
                out.extend(set_interval(values, v + 1));
            }
            out
        }
        Edit::Minus => {
            if values.poll_interval_secs <= POLL_INTERVAL_MIN {
                return Vec::new();
            }
            let mut out = take_typed(values, typed);
            let v = values.poll_interval_secs;
            if v > POLL_INTERVAL_MIN {
                out.extend(set_interval(values, v - 1));
            }
            out
        }
        Edit::Typed(text) => {
            *typed = Some(text);
            Vec::new()
        }
        Edit::Enter | Edit::FocusLost => take_typed(values, typed),
    }
}

/// What leaving the page stores, by "Back" or by the popup closing: the
/// typed text is taken as at a focus loss, except that an emptied field is
/// left alone, so that leaving does not store the 1 that an empty text
/// reads as.
pub fn leave(values: &mut Settings, typed: &mut Option<String>) -> Vec<Store> {
    if typed.as_deref() == Some("") {
        *typed = None;
    }
    take_typed(values, typed)
}

/// The shown values after the settings watch reported `new`: every control
/// takes the reported value, except the number field while text is being
/// typed into it, so that the text and the value it is compared with stay
/// as the user left them.
pub fn delivered(values: &Settings, typed: &Option<String>, new: &Settings) -> Settings {
    let mut next = new.clone();
    if typed.is_some() {
        next.poll_interval_secs = values.poll_interval_secs;
    }
    next
}

/// The plan of the page for `input`.
pub fn plan(input: &Input) -> Plan {
    let mut shown = input.values.clone();
    let mut entries = vec![
        Entry::Back {
            icon: BACK_ICON,
            text: fl!("about-back"),
        },
        Entry::Separator,
    ];
    if input.notice {
        entries.push(Entry::Notice(fl!("notice-store-failed")));
    }
    for (s, &(first, last)) in SECTIONS.iter().enumerate() {
        let items = (first..=last)
            .map(|item| {
                let control = match item {
                    1 => {
                        let value = shown.poll_interval_secs;
                        Kind::Number {
                            value,
                            text: input.typed.clone().unwrap_or_else(|| value.to_string()),
                            minus: value > POLL_INTERVAL_MIN,
                            plus: value < POLL_INTERVAL_MAX,
                            typing: input.typed.is_some(),
                        }
                    }
                    3 | 4 => Kind::List {
                        items: text_size_items(if item == 3 {
                            fl!("text-size-match")
                        } else {
                            fl!("text-size-match-dock")
                        }),
                        shown: {
                            let size = *text_size(&mut shown, item).expect("a text size");
                            TextSize::ALL.iter().position(|&t| t == size).unwrap_or(0)
                        },
                    },
                    5 => Kind::List {
                        items: unit_items(),
                        shown: UNITS
                            .iter()
                            .position(|&u| u == shown.temperature_unit)
                            .unwrap_or(0),
                    },
                    _ => Kind::Switch(*switch(&mut shown, item).expect("a switch")),
                };
                Item {
                    label: label(item),
                    control,
                    key: KEYS[item - 1],
                    tooltip: tooltip_of(item),
                }
            })
            .collect();
        entries.push(Entry::Section(Section {
            heading: heading(s),
            items,
        }));
    }
    Plan {
        entries,
        side_padding: PAGE_SIDE_PADDING,
    }
}

/// The answer to a message of the page. Every message leaves the popup
/// open: "Back" returns to the temperatures, and a control's message is
/// sent on so that the event loop stores its change.
pub fn clicked(_plan: &Plan, clicked: Clicked) -> Reply {
    let event = match clicked {
        Clicked::Back => Event::Back,
        Clicked::Edit(e) => Event::Edit(e),
    };
    Reply {
        event: Some(event),
        close: false,
    }
}

/// The widget ID with the name `name`, as the tests look it up.
///
/// `item-<N>` (N is the item number, from 1), `control-<N>`, `section-<N>`,
/// `minus` and `plus` are containers with no padding around the item row,
/// its control, the section and the number field's two buttons, so they
/// add nothing to the layout. `back` holds "Back" and `back-icon` its
/// icon, `separator` the line under it, `content` everything below that
/// line, `scroll` is the scroll area, `input` the number field's text
/// input and `notice` the line that reports a failed store.
pub fn id(name: impl Into<String>) -> Id {
    Id::new(name.into())
}

/// The number field: a − button, a text input and a + button in a row. A
/// button that is not enabled has no press message, so libcosmic draws it
/// disabled. A keystroke only records the text; Enter or leaving the field
/// takes it. libcosmic's text input reports a focus loss at every click
/// outside it, focused or not, so the field asks for that message only
/// while typed text waits to be taken: a click elsewhere on the page then
/// sends nothing but its own message.
fn number<'a>(
    text_shown: &'a str,
    minus: bool,
    plus: bool,
    typing: bool,
) -> cosmic::Element<'a, Clicked> {
    let space = cosmic::theme::spacing().space_xxs;
    let minus = button::icon(icon::from_name(MINUS_ICON))
        .on_press_maybe(minus.then_some(Clicked::Edit(Edit::Minus)));
    let plus = button::icon(icon::from_name(PLUS_ICON))
        .on_press_maybe(plus.then_some(Clicked::Edit(Edit::Plus)));
    let input = text_input("", text_shown)
        .id(id("input"))
        .width(Length::Fixed(INPUT_WIDTH))
        .on_input(|t| Clicked::Edit(Edit::Typed(t)))
        .on_submit(|_| Clicked::Edit(Edit::Enter));
    let input = if typing {
        input.on_unfocus(Clicked::Edit(Edit::FocusLost))
    } else {
        input
    };
    row::with_children(vec![
        container(minus).id(id("minus")).into(),
        input.into(),
        container(plus).id(id("plus")).into(),
    ])
    .spacing(space)
    .align_y(Alignment::Center)
    .into()
}

/// The control of item `n`, wrapped in its `control-<n>` container.
fn control<'a>(n: usize, c: &'a Kind) -> cosmic::Element<'a, Clicked> {
    let widget: cosmic::Element<'a, Clicked> = match c {
        Kind::Number {
            text,
            minus,
            plus,
            typing,
            ..
        } => number(text, *minus, *plus, *typing),
        Kind::Switch(on) => toggler(*on)
            .on_toggle(move |on| {
                Clicked::Edit(Edit::Change {
                    item: n,
                    choice: Choice::Switch(on),
                })
            })
            .into(),
        // The list has no popup of its own: it opens over the page,
        // inside the applet's popup.
        Kind::List { items, shown } => dropdown(items.as_slice(), Some(*shown), move |index| {
            Clicked::Edit(Edit::Change {
                item: n,
                choice: Choice::Entry(index),
            })
        })
        .into(),
    };
    container(widget).id(id(format!("control-{n}"))).into()
}

/// The page's content, drawn from `plan`: "Back" and the separator, as
/// wide as the popup like the temperatures' entries, then the notice and
/// the sections in one column between the two side paddings, all inside
/// one scroll area so that "Back" scrolls with the page. Each section is
/// a libcosmic settings section and each item a settings item row; a
/// label too long for its row wraps.
pub fn view<'a>(plan: &'a Plan) -> cosmic::Element<'a, Clicked> {
    let mut n = 0;
    let mut s = 0;
    let mut children: Vec<cosmic::Element<'a, Clicked>> = Vec::new();
    let mut below: Vec<cosmic::Element<'a, Clicked>> = Vec::new();
    let mut sections: Vec<cosmic::Element<'a, Clicked>> = Vec::new();
    for e in &plan.entries {
        match e {
            Entry::Back {
                icon: name,
                text: t,
            } => {
                let parts = row::with_children(vec![
                    container(icon::from_name(*name).size(16).symbolic(true).icon())
                        .id(id("back-icon"))
                        .into(),
                    text(t.as_str()).into(),
                ])
                .spacing(8)
                .align_y(Alignment::Center);
                let back = menu_button(parts).on_press(Clicked::Back);
                children.push(container(back).id(id("back")).width(Length::Fill).into());
            }
            Entry::Separator => children.push(
                container(padded(divider::horizontal::default()))
                    .id(id("separator"))
                    .width(Length::Fill)
                    .into(),
            ),
            Entry::Notice(t) => below.push(
                container(text(t.as_str()).width(Length::Fill))
                    .id(id("notice"))
                    .width(Length::Fill)
                    .into(),
            ),
            Entry::Section(sec) => {
                s += 1;
                let items = sec.items.iter().map(|it| {
                    n += 1;
                    let row =
                        settings::item::builder(it.label.as_str()).control(control(n, &it.control));
                    let item: cosmic::Element<'a, Clicked> =
                        container(row).id(id(format!("item-{n}"))).into();
                    match &it.tooltip {
                        // The tooltip covers the whole item row, at
                        // libcosmic's default position.
                        Some((_, tip)) => {
                            tooltip(item, text(tip.as_str()), tooltip::Position::default()).into()
                        }
                        None => item,
                    }
                });
                let section: cosmic::Element<'a, Clicked> = settings::section()
                    .title(sec.heading.as_str())
                    .extend(items.collect::<Vec<_>>())
                    .into();
                sections.push(container(section).id(id(format!("section-{s}"))).into());
            }
        }
    }
    below.push(settings::view_column(sections).into());
    let space = cosmic::theme::spacing().space_m;
    children.push(
        container(column::with_children(below).spacing(space))
            .id(id("content"))
            .padding(look::content_padding())
            .width(Length::Fill)
            .into(),
    );
    scrollable(column::with_children(children).width(Length::Fill))
        .id(id("scroll"))
        .width(Length::Fill)
        .into()
}
