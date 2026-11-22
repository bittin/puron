//! The popup that opens from the panel button: a `Plan` built from the
//! `Menu` that `puron_core::menu` laid out, the widgets drawn from it, and
//! what a click on them does. A click on a line becomes an `Event` for the
//! event loop in `app`; this module chooses no entry itself and never
//! changes the pinned list (`puron_core::pins` does that).

use cosmic::iced::advanced::layout::Limits;
use cosmic::iced::advanced::text::{self as advanced_text, Paragraph as _};
use cosmic::iced::alignment::Vertical;
use cosmic::iced::widget::text::Wrapping;
use cosmic::iced::{Alignment, Length, Padding, Pixels, Size};
use cosmic::widget::{self, button, column, container, divider, row, scrollable, text, Id, Space};
use puron_core::menu::{
    Entry, Heading, Heat, IconKind, Item, LineWord, Menu, MessageKind, Section,
};
use puron_core::pins;
use puron_core::rows::{self as core_rows, NAME_LIMIT};

use crate::i18n;
use crate::look::{self, Role};
use crate::panel::{icon_name, section_icon_name};

/// The icon name of the check mark drawn in front of a pinned line.
pub const CHECK_MARK: &str = "object-select-symbolic";
/// The check mark's side, in px. It does not follow the panel's icon size.
pub const CHECK_MARK_SIZE: u16 = 14;
/// The width in px of the slot in front of every line's text, whether or
/// not it holds a mark: the 14 px mark, then 6 px of gap to the text.
pub const CHECK_SLOT: f32 = 20.0;
/// A device heading's indent in px from the section heading's left edge
/// (the left edge of its tile) to the device's icon.
pub const DEVICE_INDENT: f32 = 4.0;
/// The side in px of the icon of a device heading.
pub const DEVICE_ICON: u16 = 16;
/// The space in px between a device heading's icon and its name, so that
/// the name starts 28 px right of the section heading's left edge.
pub const DEVICE_GAP: f32 = 8.0;
/// The space in px between a card's left edge and a line's check slot. A
/// card starts at the section heading's left edge.
pub const LINE_PADDING_LEFT: f32 = 8.0;
/// The space in px between a line's value and its card's right edge.
pub const LINE_PADDING_RIGHT: f32 = 14.0;
/// The space in px above and below a line inside its card. libcosmic's
/// list column gives each item a row at least 32 px high, so a line is
/// at least `LINE_MIN_HEIGHT` high.
const LINE_PADDING_VERTICAL: f32 = 1.0;
/// The least height in px of a line in its card.
pub const LINE_MIN_HEIGHT: f32 = 34.0;
/// The space in px below each card, before the next entry.
const CARD_GAP: f32 = 8.0;
/// The icon in front of the entry "Settings": a gear.
pub const SETTINGS_ICON: &str = "preferences-system-symbolic";
/// The icon in front of the entry "About": an "i" in a circle.
pub const ABOUT_ICON: &str = "help-about-symbolic";
/// The side in px of the icons of "Settings" and "About".
pub const LINK_ICON: u16 = 16;
/// The space in px between the icon of "Settings" or "About" and its
/// text.
pub const LINK_GAP: f32 = 10.0;
/// The space in px at the left and at the right of every page of the
/// popup: the temperatures, the About page and the Settings page. Each
/// page takes it from here, so that a Settings section heading starts
/// where the temperatures' section heading does. It is fixed instead of
/// following the theme's density, whose spacing would give 16 px on a
/// compact desktop and 32 px on a spacious one.
pub const PAGE_SIDE_PADDING: u16 = 24;
/// The popup's narrowest width in px. libcosmic's
/// `applet::popup_container` fixes the width at this value; `limits`
/// replaces that with the width that `width` computes.
pub const WIDTH: f32 = 360.0;
/// The popup's widest width in px: room for a line whose name has 44
/// characters, taken as 7 px each on average in the default font. That
/// is 48 of padding, 8 of line padding at the left, 20 of check slot, 308
/// of name, 8 of gap, 64 of value and 14 of line padding at the right,
/// 470 in all, with 2 px to spare.
pub const MAX_WIDTH: f32 = 472.0;
/// The height limit in px that libcosmic's `applet::popup_container`
/// sets, kept by `limits`. Content taller than this scrolls.
const MAX_HEIGHT: f32 = 1000.0;
/// The least space in px that `fit_width` counts between a line's name
/// and its value. `view` draws no gap of its own.
const NAME_GAP: f32 = 8.0;
/// The least width in px counted for a value, so that a value that grows
/// by a digit does not widen the popup. "100.0°F" fits in it.
const VALUE_ROOM: f32 = 64.0;
/// The text size in px that `measure` assumes: libcosmic's default size
/// for applet text.
const TEXT_SIZE: f32 = 14.0;

/// A reading's line in the popup, drawn left to right as the check slot,
/// the text and the value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The reading's identity: the text that the pinned list stores and
    /// that a click on the line sends.
    pub identity: String,
    /// Whether the check mark is drawn, that is, whether `identity` is in
    /// the pinned list.
    pub pinned: bool,
    /// The name, shortened to fit `NAME_LIMIT` characters together with
    /// the default line's suffix.
    pub text: String,
    /// The value text. The line of a pin that waits for lm-sensors shows
    /// the missing-value text instead.
    pub value: String,
    /// The color role of the value: the theme's warning color for a
    /// temperature that runs warm, its red for one that runs hot.
    pub value_role: Role,
    /// The card that holds the line, counted from 1 in plan order. The
    /// lines of one device, or of one fan, voltage or power section, share
    /// a card.
    pub card: usize,
}

/// One entry of the popup, made from one entry of the `Menu`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Drawn {
    /// A section heading: the class's icon in a tile of the section's
    /// color, and its name. A click does nothing.
    Section {
        icon: &'static str,
        text: String,
        role: Role,
    },
    /// A device heading, such as a drive's name or "CPU", with the icon of
    /// its kind of device in that kind's color. A click does nothing.
    Device {
        text: String,
        icon: &'static str,
        role: Role,
    },
    Line(Line),
    Separator,
    /// The message shown when there are no readings, which tells the user
    /// what to do. A click on it opens the homepage.
    Message {
        text: String,
    },
    /// The entry "About", which shows the About page in the popup. It keeps
    /// the name of the homepage link that it was until version 1.6.
    HomepageLink {
        text: String,
        icon: &'static str,
        role: Role,
    },
    /// The entry "Settings", which shows the Settings page in the popup.
    SettingsLink {
        text: String,
        icon: &'static str,
        role: Role,
    },
}

/// Everything the popup draws, as plain data. `view` builds the widgets
/// from it alone, so tests can check a plan without a display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub entries: Vec<Drawn>,
    /// The color role of the check mark of a pinned line.
    pub check_role: Role,
    /// The panel's icon size in px. The popup draws nothing at this size
    /// any more (a section's icon is in its tile at a fixed size); `plan`
    /// and `width` keep it so that their callers do not change.
    pub icon_size: u16,
    /// The space in px at the left and right of each entry, always
    /// `PAGE_SIDE_PADDING`.
    pub side_padding: u16,
}

/// What a click asks the event loop in `app` to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A click on the line with this identity. `puron_core::pins::click`
    /// decides the new pinned list.
    PinClick(String),
    /// Open the project homepage in the browser.
    HomepageLink,
    /// Show the Settings page in place of the temperatures.
    Settings,
    /// Show the About page in place of the temperatures.
    About,
    /// Show the temperatures again, from the About page or the Settings
    /// page.
    Back,
    /// Open this address of the About page in the browser.
    OpenLink(String),
    /// Copy the About page's summary to the clipboard.
    CopyDetails,
    /// A message of a control of the Settings page.
    Edit(crate::settings::Edit),
}

/// The answer to a click: the event to send, if any, and whether the
/// popup closes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub event: Option<Event>,
    pub close: bool,
}

/// The widget message of a click. It names what was clicked, not its
/// position: a line is named by its identity, so the click still means
/// the line the user saw even if a newer view has arrived between the
/// draw and the click.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clicked {
    Line(String),
    Message,
    HomepageLink,
    SettingsLink,
}

/// The answer to a widget's click, given `plan`, the popup's plan at the
/// time of the click. A line that `plan` still shows is sent as
/// `Event::PinClick` and the popup stays open; a line that `plan` no
/// longer shows sends nothing. The message, the entry "About" and the
/// settings link answer as in `click`.
pub fn clicked(plan: &Plan, clicked: Clicked) -> Reply {
    let (event, close) = match clicked {
        Clicked::Line(id) => {
            let shown = plan
                .entries
                .iter()
                .any(|e| matches!(e, Drawn::Line(l) if l.identity == id));
            (shown.then_some(Event::PinClick(id)), false)
        }
        Clicked::Message => (Some(Event::HomepageLink), true),
        Clicked::HomepageLink => (Some(Event::About), false),
        Clicked::SettingsLink => (Some(Event::Settings), false),
    };
    Reply { event, close }
}

/// The answer to a click on a drawn entry. A line sends `Event::PinClick`
/// and leaves the popup open. The entries "About" and "Settings" send
/// `Event::About` and `Event::Settings` and leave it open too, because
/// their pages show in the same popup. The message sends
/// `Event::HomepageLink` and closes the popup, because the user turns to
/// the browser. Headings and separators
/// do nothing.
pub fn click(entry: &Drawn) -> Reply {
    let (event, close) = match entry {
        Drawn::Line(l) => (Some(Event::PinClick(l.identity.clone())), false),
        Drawn::Section { .. } | Drawn::Device { .. } | Drawn::Separator => (None, false),
        Drawn::Message { .. } => (Some(Event::HomepageLink), true),
        Drawn::HomepageLink { .. } => (Some(Event::About), false),
        Drawn::SettingsLink { .. } => (Some(Event::Settings), false),
    };
    Reply { event, close }
}

/// The popup plan for `menu`, with a check mark on each line that is in
/// `pinned`: one drawn entry per menu entry, in the menu's order. The
/// texts come from the catalog of the chosen language.
pub fn plan(menu: &Menu, pinned: &[String], icon_size: u16) -> Plan {
    // A line opens a new card when the entry before it is not a line: a
    // device heading, a section heading or nothing.
    let mut cards = 0;
    let mut in_card = false;
    let entries = menu
        .entries
        .iter()
        .map(|entry| {
            let line = matches!(entry, Entry::Line(_));
            if line && !in_card {
                cards += 1;
            }
            in_card = line;
            drawn(entry, pinned, cards)
        })
        .collect();
    Plan {
        entries,
        check_role: Role::Accent,
        icon_size,
        side_padding: PAGE_SIDE_PADDING,
    }
}

/// The drawn entry for `entry`; a line goes in card `card`.
fn drawn(entry: &Entry, pinned: &[String], card: usize) -> Drawn {
    match entry {
        Entry::Section(section) => Drawn::Section {
            icon: section_icon_name(*section),
            text: section_text(*section),
            role: section_role(*section),
        },
        Entry::Device(heading, kind) => Drawn::Device {
            text: heading_text(heading),
            icon: icon_name(*kind),
            role: kind_role(*kind),
        },
        Entry::Line(item) => Drawn::Line(line_of(item, pinned, card)),
        Entry::Separator => Drawn::Separator,
        Entry::Message { kind, .. } => Drawn::Message {
            text: match kind {
                MessageKind::SensorsDetect => fl!("message-detect"),
                MessageKind::Install => fl!("message-install"),
            },
        },
        Entry::HomepageLink { .. } => Drawn::HomepageLink {
            text: fl!("link-homepage"),
            icon: ABOUT_ICON,
            role: Role::Muted,
        },
        Entry::SettingsLink => Drawn::SettingsLink {
            text: fl!("link-settings"),
            icon: SETTINGS_ICON,
            role: Role::Muted,
        },
    }
}

/// The color of a section's tile. Voltage and power share the lightning
/// icon, so their tiles differ in color.
fn section_role(section: Section) -> Role {
    match section {
        Section::Temperature => Role::Orange,
        Section::Fan => Role::Accent,
        Section::Voltage => Role::Yellow,
        Section::Power => Role::Purple,
    }
}

/// The color of the icon of a device of kind `kind`. The CPU, the GPU and
/// the drive have the colors of the Settings sections that show the same
/// icons; the two network cards share a green and the memory module the
/// drive's indigo, and are told apart by their icons. A device that Puron
/// does not tell apart is grey. A device heading never has the kind of a
/// fan, a voltage or a power reading, which have no heading.
fn kind_role(kind: IconKind) -> Role {
    match kind {
        IconKind::CpuTemperature => Role::Yellow,
        IconKind::GpuTemperature => Role::Purple,
        IconKind::DriveTemperature | IconKind::MemoryTemperature => Role::Indigo,
        IconKind::WifiTemperature | IconKind::WiredNetworkTemperature => Role::Green,
        IconKind::GeneralTemperature | IconKind::Fan | IconKind::Voltage | IconKind::Power => {
            Role::Muted
        }
    }
}

/// The color of a value that runs at `heat`.
fn heat_role(heat: Heat) -> Role {
    match heat {
        Heat::Normal => Role::Default,
        Heat::Warm => Role::Warning,
        Heat::Hot => Role::Destructive,
    }
}

/// A line's text before it is shortened: the reading's name, then the word
/// that the menu attached to the line (such as "composite"), with one
/// space between them. A line with no name shows the word alone. For the
/// line of a pin that waits for lm-sensors, the word is "CPU: waiting for
/// lm-sensors" in English.
fn line_name(item: &Item) -> String {
    let word = match item.word {
        None => return item.name.clone(),
        Some(LineWord::Composite) => fl!("line-composite"),
        Some(LineWord::Package) => fl!("line-package"),
        Some(LineWord::Temperature) => fl!("line-temperature"),
        Some(LineWord::Maximum) => fl!("row-maximum"),
        Some(LineWord::Waiting) => format!("{}: {}", fl!("row-cpu"), fl!("line-waiting")),
    };
    if item.name.is_empty() {
        word
    } else {
        format!("{} {word}", item.name)
    }
}

/// The drawn line for `item` in card `card`. The default line gets the
/// `cpu-default` suffix; the name is shortened first so that name and
/// suffix together fit `NAME_LIMIT` characters.
fn line_of(item: &Item, pinned: &[String], card: usize) -> Line {
    let suffix = if item.default {
        format!(" {}", fl!("cpu-default"))
    } else {
        String::new()
    };
    let room = NAME_LIMIT.saturating_sub(suffix.chars().count());
    Line {
        identity: item.identity.clone(),
        pinned: pins::is_pinned(pinned, &item.identity),
        text: format!("{}{suffix}", core_rows::cut_at_end(&line_name(item), room)),
        value: if item.word == Some(LineWord::Waiting) {
            fl!("missing-value")
        } else {
            i18n::value_text(item.value.as_ref())
        },
        value_role: heat_role(item.heat),
        card,
    }
}

fn section_text(section: Section) -> String {
    match section {
        Section::Temperature => fl!("class-temperature"),
        Section::Fan => fl!("section-fan"),
        Section::Voltage => fl!("class-voltage"),
        Section::Power => fl!("class-power"),
    }
}

fn heading_text(heading: &Heading) -> String {
    let text = match heading {
        Heading::Cpu => fl!("row-cpu"),
        Heading::Name(name) => name.clone(),
    };
    core_rows::cut_at_end(&text, NAME_LIMIT)
}

/// The popup's width in px: the width of its widest heading or line, plus
/// the side padding at both ends, but never less than [`WIDTH`] or more
/// than [`MAX_WIDTH`].
pub fn width(menu: &Menu, icon_size: u16) -> f32 {
    fit_width(
        menu,
        icon_size,
        2.0 * f32::from(PAGE_SIDE_PADDING),
        &measure,
    )
}

/// The computation of [`width`], with the entry's horizontal padding and
/// the text measure as parameters. A line counts its padding in the card
/// at both sides, check slot, name, gap and value (at least
/// [`VALUE_ROOM`]); a section heading its tile, gap and text; a device
/// heading its indent, icon, gap and text. Separators, the message and the
/// links do not count. Nothing depends on `icon_size` any more, since a
/// section's icon has a fixed size in its tile.
pub fn fit_width(menu: &Menu, icon_size: u16, padding: f32, measure: &dyn Fn(&str) -> f32) -> f32 {
    let widest = plan(menu, &[], icon_size)
        .entries
        .iter()
        .map(|e| match e {
            Drawn::Line(l) => {
                LINE_PADDING_LEFT
                    + CHECK_SLOT
                    + measure(&l.text)
                    + NAME_GAP
                    + measure(&l.value).max(VALUE_ROOM)
                    + LINE_PADDING_RIGHT
            }
            Drawn::Section { text, .. } => look::TILE + look::TILE_GAP + measure(text),
            Drawn::Device { text, .. } => {
                DEVICE_INDENT + f32::from(DEVICE_ICON) + DEVICE_GAP + measure(text)
            }
            _ => 0.0,
        })
        .fold(0.0, f32::max);
    (padding + widest).clamp(WIDTH, MAX_WIDTH)
}

/// The width in px of `content` on one line, in the default font at
/// [`TEXT_SIZE`].
fn measure(content: &str) -> f32 {
    type Paragraph = <cosmic::Renderer as advanced_text::Renderer>::Paragraph;
    Paragraph::with_text(advanced_text::Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(TEXT_SIZE),
        line_height: advanced_text::LineHeight::default(),
        font: cosmic::font::default(),
        align_x: advanced_text::Alignment::Default,
        align_y: Vertical::Top,
        shaping: advanced_text::Shaping::Advanced,
        wrapping: Wrapping::None,
        ellipsize: advanced_text::Ellipsize::None,
    })
    .min_width()
}

/// The size limits of a popup `width` px wide: exactly that width, and
/// the same height limits as libcosmic's `applet::popup_container`.
pub fn limits(width: f32) -> Limits {
    Limits::NONE
        .min_height(1.0)
        .min_width(width)
        .max_width(width)
        .max_height(MAX_HEIGHT)
}

/// The widget ID of the container of the drawn entry at `index` in
/// `Plan::entries`, so that tests can find its rectangle.
pub fn entry_id(index: usize) -> Id {
    Id::new(format!("e{index}"))
}

/// The widget ID of one part of the drawn entry at `index`. A line's parts
/// are `check`, `mark`, `name` and `value`; a section heading's are
/// `tile`, `icon` (the icon in the tile) and `heading`; a device heading's
/// `icon` and `name`; the icon and the text of "Settings" and "About"
/// `icon` and `name`.
pub fn element_id(index: usize, element: &str) -> Id {
    Id::new(format!("e{index}.{element}"))
}

/// The widget ID of the card with number `card` (`Line::card`).
pub fn card_id(card: usize) -> Id {
    Id::new(format!("card-{card}"))
}

/// The padding of one entry of a popup page: `PAGE_SIDE_PADDING` at the
/// left and right, and the theme's `space_xxs` spacing at the top and
/// bottom, as libcosmic's own applet menus have.
pub fn entry_padding() -> Padding {
    let v = f32::from(cosmic::theme::spacing().space_xxs);
    let side = f32::from(PAGE_SIDE_PADDING);
    Padding {
        top: v,
        bottom: v,
        left: side,
        right: side,
    }
}

/// A clickable entry of a popup page: libcosmic's applet menu button as
/// wide as the page, so that its hover band reaches both edges of the
/// popup, with `content` inside the padding of `entry_padding`.
pub fn menu_button<'a, M: Clone + 'a>(
    content: impl Into<cosmic::Element<'a, M>>,
) -> button::Button<'a, M> {
    button::custom(content)
        .class(cosmic::theme::Button::AppletMenu)
        .padding(entry_padding())
        .width(Length::Fill)
}

/// An entry of a popup page that answers no click: `content` inside the
/// padding of `entry_padding`, as wide as the page.
pub fn padded<'a, M: 'a>(
    content: impl Into<cosmic::Element<'a, M>>,
) -> container::Container<'a, M, cosmic::Theme, cosmic::Renderer> {
    container(content)
        .padding(entry_padding())
        .width(Length::Fill)
}

/// One drawn entry of the temperatures before the lines are put in their
/// cards: a line as an item of a list column, with its card number, or
/// any other entry as a whole widget.
enum Part<'a> {
    Line(usize, widget::list::ListButton<'a, Clicked>),
    Entry(cosmic::Element<'a, Clicked>),
}

/// The card of the lines `items`: libcosmic's list column, as a Settings
/// section's card, with a divider between two lines. It lies between the
/// page's side paddings, so it starts at the section heading's left edge,
/// with some space below it.
fn card<'a>(
    number: usize,
    items: Vec<widget::list::ListButton<'a, Clicked>>,
) -> cosmic::Element<'a, Clicked> {
    let padding = Padding {
        top: LINE_PADDING_VERTICAL,
        bottom: LINE_PADDING_VERTICAL,
        left: LINE_PADDING_LEFT,
        right: LINE_PADDING_RIGHT,
    };
    let column = items.into_iter().fold(
        widget::list_column().list_item_padding(padding),
        |col, item| col.add(item),
    );
    let side = f32::from(PAGE_SIDE_PADDING);
    container(container(column.into_element()).id(card_id(number)))
        .padding(Padding {
            top: 0.0,
            bottom: CARD_GAP,
            left: side,
            right: side,
        })
        .width(Length::Fill)
        .into()
}

/// The popup's content, built from the plan alone with libcosmic's applet
/// and list widgets, so that it looks like COSMIC's own popups and like
/// the Settings page. Every entry has `PAGE_SIDE_PADDING` at its left and
/// right. The lines of each card are the items of one list column (see
/// `card`), whose hover fills the item inside the card.
///
/// Each drawn entry sits in a container with the ID `entry_id(index)`; for
/// a line it is the line's content inside the item's padding. Inside it,
/// the parts have the IDs of `element_id` (`mark` only when the line is
/// pinned), and each card has `card_id`; an ID on a text is on the text's
/// own rectangle. A name stays on one line: when it is wider than its
/// space it is clipped there and never covers the value. Content taller
/// than the popup scrolls.
pub fn view<'a>(plan: &Plan) -> cosmic::Element<'a, Clicked> {
    let mark_side = f32::from(CHECK_MARK_SIZE);
    let parts: Vec<Part<'a>> = plan
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| -> Part<'a> {
            match e {
                Drawn::Line(l) => {
                    let check: cosmic::Element<'a, Clicked> = if l.pinned {
                        container(look::tinted_icon(
                            CHECK_MARK,
                            CHECK_MARK_SIZE,
                            plan.check_role,
                        ))
                        .id(element_id(i, "mark"))
                        .into()
                    } else {
                        Space::new()
                            .width(Length::Fixed(mark_side))
                            .height(Length::Fixed(mark_side))
                            .into()
                    };
                    let content = row::with_children(vec![
                        container(check)
                            .id(element_id(i, "check"))
                            .width(Length::Fixed(CHECK_SLOT))
                            .height(Length::Fixed(mark_side))
                            .into(),
                        container(
                            container(text(l.text.clone()).wrapping(Wrapping::None))
                                .id(element_id(i, "name")),
                        )
                        .width(Length::Fill)
                        .clip(true)
                        .into(),
                        container(look::plain(l.value.clone(), l.value_role))
                            .id(element_id(i, "value"))
                            .into(),
                    ])
                    .align_y(Alignment::Center);
                    // The content fills the item's row, which the list
                    // column makes at least 32 px high, so the line's
                    // rectangle is the whole of the item inside its padding.
                    let content = container(content)
                        .id(entry_id(i))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_y(Vertical::Center);
                    let item =
                        widget::list::button(content).on_press(Clicked::Line(l.identity.clone()));
                    Part::Line(l.card, item)
                }
                Drawn::Section {
                    icon,
                    text: t,
                    role,
                } => {
                    let tile =
                        look::icon_tile(icon, *role, element_id(i, "tile"), element_id(i, "icon"));
                    let parts = row::with_children(vec![
                        tile,
                        container(text::heading(t.clone()).wrapping(Wrapping::None))
                            .id(element_id(i, "heading"))
                            .into(),
                    ])
                    .spacing(look::TILE_GAP)
                    .align_y(Alignment::Center);
                    Part::Entry(container(padded(parts)).id(entry_id(i)).into())
                }
                Drawn::Device {
                    text: t,
                    icon,
                    role,
                } => {
                    let parts = row::with_children(vec![
                        container(look::tinted_icon(icon, DEVICE_ICON, *role))
                            .id(element_id(i, "icon"))
                            .into(),
                        container(text(t.clone()).wrapping(Wrapping::None))
                            .id(element_id(i, "name"))
                            .into(),
                    ])
                    .spacing(DEVICE_GAP)
                    .align_y(Alignment::Center);
                    Part::Entry(
                        container(container(padded(parts)).id(entry_id(i)))
                            .padding([0.0, 0.0, 0.0, DEVICE_INDENT])
                            .into(),
                    )
                }
                Drawn::Separator => Part::Entry(
                    container(padded(divider::horizontal::default()))
                        .id(entry_id(i))
                        .into(),
                ),
                Drawn::Message { text: t } => Part::Entry(
                    container(menu_button(text(t.clone())).on_press(Clicked::Message))
                        .id(entry_id(i))
                        .into(),
                ),
                Drawn::HomepageLink {
                    text: t,
                    icon,
                    role,
                }
                | Drawn::SettingsLink {
                    text: t,
                    icon,
                    role,
                } => {
                    let message = match e {
                        Drawn::HomepageLink { .. } => Clicked::HomepageLink,
                        _ => Clicked::SettingsLink,
                    };
                    let parts = row::with_children(vec![
                        container(look::tinted_icon(icon, LINK_ICON, *role))
                            .id(element_id(i, "icon"))
                            .into(),
                        container(text(t.clone())).id(element_id(i, "name")).into(),
                    ])
                    .spacing(LINK_GAP)
                    .align_y(Alignment::Center);
                    Part::Entry(
                        container(menu_button(parts).on_press(message))
                            .id(entry_id(i))
                            .into(),
                    )
                }
            }
        })
        .collect();
    // Each run of lines of one card number becomes that card.
    let mut children: Vec<cosmic::Element<'a, Clicked>> = Vec::new();
    let mut open: Option<(usize, Vec<widget::list::ListButton<'a, Clicked>>)> = None;
    for part in parts {
        match part {
            Part::Line(number, item) => match &mut open {
                Some((n, items)) if *n == number => items.push(item),
                _ => {
                    if let Some((n, items)) = open.take() {
                        children.push(card(n, items));
                    }
                    open = Some((number, vec![item]));
                }
            },
            Part::Entry(element) => {
                if let Some((n, items)) = open.take() {
                    children.push(card(n, items));
                }
                children.push(element);
            }
        }
    }
    if let Some((n, items)) = open.take() {
        children.push(card(n, items));
    }
    scrollable(column::with_children(children).width(Length::Fill)).into()
}
