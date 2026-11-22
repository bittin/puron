//! The facts that the popup's About page shows about this program: the
//! kind of package it was built for, its license lines, its copyright
//! line, its web addresses and the state of each source of sensors, and
//! the page drawn from them. The page is the popup's second page: the
//! entry "About" of the temperatures shows it, and its "Back" returns.

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, divider, row, scrollable, Id, Space};
use puron_core::menu::HOMEPAGE_URL;
use puron_core::rows::Provider;
use puron_core::settings::Settings;
use puron_engine::engine::{ProviderState, Snapshot};
use puron_engine::registry;

use crate::look::{self, Role, TextStyle};
use crate::popup::{element_id, entry_id, menu_button, padded, Event, Reply, PAGE_SIDE_PADDING};

/// The kind of package that this program was built for. The build sets
/// it (`make install` with `PACKAGE`); the program never tests at run
/// time how it was installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Package {
    /// The Debian package.
    Deb,
    /// The Flatpak.
    Flatpak,
    /// Any other build, for example `make install` or `cargo build` from
    /// a source tree.
    Source,
}

impl Package {
    /// The kind for the value of `PURON_PACKAGE` at build time: `deb` and
    /// `flatpak` name their package, and no value or an empty one is a
    /// build from source. The Makefile refuses any other value, so a
    /// plain `cargo build` is the only way to get one here, and it counts
    /// as a build from source too.
    pub fn of(value: Option<&str>) -> Package {
        match value {
            Some("deb") => Package::Deb,
            Some("flatpak") => Package::Flatpak,
            _ => Package::Source,
        }
    }

    /// The kind that this program was built with. `make install-bin` sets
    /// `PURON_PACKAGE` for the applet's build, and cargo builds the applet
    /// again when the value changes.
    pub fn built() -> Package {
        Package::of(option_env!("PURON_PACKAGE"))
    }
}

/// The copyright line of the page. It names the holder that `LICENSE`
/// names, and it is the same in every language.
pub const COPYRIGHT: &str = "© 2026 Eric Lalonde";

/// The address of the source repository, the metainfo's `vcs-browser`.
pub const SOURCE_URL: &str = "https://github.com/elalonde/puron";
/// The address for problem reports, the metainfo's `bugtracker`.
pub const ISSUES_URL: &str = "https://github.com/elalonde/puron/issues";
/// The icon in front of "Back", from the COSMIC icon theme.
pub const BACK_ICON: &str = "go-previous-symbolic";

/// The address of the README's license section at the release tag of
/// `version`, which lists the third-party libraries and their licenses
/// for exactly that release.
pub fn notices_url(version: &str) -> String {
    format!("https://github.com/elalonde/puron/blob/v{version}/README.md#license")
}

/// The version of this program: the workspace version it was built from.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The providers in the order of the page's "Sensor sources", with the
/// name the page shows for each. These names are product names and are
/// never translated.
pub const SOURCES: [(Provider, &str); 5] = [
    (Provider::LmSensors, "lm-sensors"),
    (Provider::NvmeCli, "NVMe"),
    (Provider::Nvidia, "NVIDIA"),
    (Provider::Udisks2, "udisks2"),
    (Provider::Liquidctl, "liquidctl"),
];

/// What the page says about one provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Puron does not run it: its switch is off.
    Off,
    /// It runs, and its latest start or read found nothing to read.
    NotAvailable,
    /// It runs, with this many readings in the latest snapshot, of every
    /// class, whether the settings show them or not.
    Available(usize),
}

/// The state of each provider of `SOURCES`, in that order, from the
/// engine's snapshot `last`. The count is the number of the provider's
/// readings in the snapshot. Before the first snapshot, and for a
/// provider that a snapshot does not list, a provider whose switch is on
/// in `settings` counts as available with no readings, so that a start
/// that has not reported yet does not read as a problem.
pub fn sources(last: Option<&Snapshot>, settings: &Settings) -> [Source; 5] {
    SOURCES.map(|(p, _)| {
        let state = last
            .and_then(|s| s.providers.iter().find(|q| q.0 == p))
            .map(|q| q.1);
        let state = state.unwrap_or(if registry::switch(settings, p) {
            ProviderState::Available
        } else {
            ProviderState::Off
        });
        match state {
            ProviderState::Off => Source::Off,
            ProviderState::NotAvailable => Source::NotAvailable,
            ProviderState::Available => Source::Available(
                last.map_or(0, |s| s.readings.iter().filter(|r| r.provider == p).count()),
            ),
        }
    })
}

/// Whether the source at `index` of `SOURCES` is liquidctl in the
/// Flatpak with its switch on. The Flatpak cannot run the liquidctl
/// program of the host, so the page says so instead of "not available".
pub fn flatpak_liquidctl(package: Package, index: usize, source: Source) -> bool {
    package == Package::Flatpak && SOURCES[index].0 == Provider::Liquidctl && source != Source::Off
}

/// What the page is drawn from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub version: String,
    pub package: Package,
    pub sources: [Source; 5],
}

/// One entry of the About page, top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Returns to the temperatures; drawn after the icon `icon`.
    Back {
        icon: &'static str,
        text: String,
    },
    Separator,
    /// The program's name.
    Name(String),
    /// "Version" and the version number.
    Version(String),
    /// The short text that says what the program does.
    Summary(String),
    /// A label and its values, each value on its own line.
    Field {
        label: String,
        values: Vec<String>,
    },
    Heading(String),
    /// A provider's name, its state text and, when it is available, its
    /// count of readings.
    Source {
        name: &'static str,
        state: String,
        count: Option<usize>,
    },
    /// A link that opens `url` in the browser.
    Link {
        text: String,
        url: String,
    },
    /// The button that copies a summary for a problem report.
    Copy {
        text: String,
    },
}

/// The About page as plain data, which `view` draws and tests read.
/// `looks` holds the look of each entry, at the same index as the entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub entries: Vec<Entry>,
    pub looks: Vec<Look>,
    /// The space in px at the left and right of the page, always
    /// `popup::PAGE_SIDE_PADDING`.
    pub side_padding: u16,
}

/// The icon at the right of each link: an arrow up and to the right, the
/// usual sign for a link that opens outside the popup.
pub const LINK_ICON: &str = "insert-arrow-symbolic";

/// Where an entry of the About page sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A full-width entry like those of the temperatures: "Back" and the
    /// separator.
    Row,
    /// The centered block of the name, the version and the summary.
    Header,
    /// The card that holds the package, license and copyright fields.
    CardFacts,
    /// The card that holds the sensor sources.
    CardSources,
    /// The card that holds the links.
    CardLinks,
    /// On the page itself, outside any card: the two headings and the
    /// "Copy details" button.
    Page,
}

/// How an entry's content lies across its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
}

/// The kind of button that a clickable entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// libcosmic's applet menu button, as wide as the popup.
    Menu,
    /// A button with no background, in the class of libcosmic's links.
    Link,
    /// libcosmic's accent button, the class of `button::suggested`.
    Suggested,
}

/// How one entry of the About page is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    pub place: Place,
    pub align: Align,
    /// The style and color role of each of the entry's texts, in this
    /// order: a field's label, then each of its values; a source's name,
    /// its state text and its count (an empty text unless it is
    /// available); for any other entry its one text. The separator has
    /// none.
    pub texts: Vec<(TextStyle, Role)>,
    /// The icon the entry draws, with its color role.
    pub icon: Option<(&'static str, Role)>,
    /// The kind of button, for an entry that answers a click.
    pub press: Option<Press>,
}

/// The color role of the state text of the source at `index` of
/// `SOURCES`: green when it is available, the warning color for liquidctl
/// in the Flatpak, grey when it is off or not available.
pub fn state_role(package: Package, index: usize, source: Source) -> Role {
    if flatpak_liquidctl(package, index, source) {
        return Role::Warning;
    }
    match source {
        Source::Available(_) => Role::Success,
        Source::Off | Source::NotAvailable => Role::Muted,
    }
}

/// The look of each of `entries`, the About entries of `input`.
fn looks(entries: &[Entry], input: &Input) -> Vec<Look> {
    let body = |role| (TextStyle::Body, role);
    let mut source = 0;
    entries
        .iter()
        .map(|e| {
            let (place, align, texts, icon, press) = match e {
                Entry::Back { icon, .. } => (
                    Place::Row,
                    Align::Start,
                    vec![body(Role::Default)],
                    Some((*icon, Role::Default)),
                    Some(Press::Menu),
                ),
                Entry::Separator => (Place::Row, Align::Start, vec![], None, None),
                Entry::Name(_) => (
                    Place::Header,
                    Align::Center,
                    vec![(TextStyle::Title3, Role::Default)],
                    None,
                    None,
                ),
                Entry::Version(_) => (
                    Place::Header,
                    Align::Center,
                    vec![body(Role::Default)],
                    None,
                    None,
                ),
                Entry::Summary(_) => (
                    Place::Header,
                    Align::Center,
                    vec![body(Role::Muted)],
                    None,
                    None,
                ),
                Entry::Field { values, .. } => {
                    let mut texts = vec![body(Role::Muted)];
                    texts.extend(values.iter().map(|_| body(Role::Default)));
                    (Place::CardFacts, Align::Start, texts, None, None)
                }
                Entry::Heading(_) => (
                    Place::Page,
                    Align::Start,
                    vec![(TextStyle::CaptionHeading, Role::Muted)],
                    None,
                    None,
                ),
                Entry::Source { .. } => {
                    let role = state_role(input.package, source, input.sources[source]);
                    source += 1;
                    (
                        Place::CardSources,
                        Align::Start,
                        vec![
                            body(Role::Default),
                            body(role),
                            (TextStyle::Heading, Role::Default),
                        ],
                        None,
                        None,
                    )
                }
                Entry::Link { .. } => (
                    Place::CardLinks,
                    Align::Start,
                    vec![body(Role::Accent)],
                    Some((LINK_ICON, Role::Accent)),
                    Some(Press::Link),
                ),
                // The accent button gives its text the theme's color for
                // text on the accent, so the text sets none of its own.
                Entry::Copy { .. } => (
                    Place::Page,
                    Align::Center,
                    vec![body(Role::Default)],
                    None,
                    Some(Press::Suggested),
                ),
            };
            Look {
                place,
                align,
                texts,
                icon,
                press,
            }
        })
        .collect()
}

/// The About page's entries for `input`, in the page's order. The texts
/// come from the catalog of the chosen language, except the fixed ones:
/// the name, the version number, "Flatpak", the license identifiers, the
/// copyright line, the provider names and the counts.
pub fn plan(input: &Input) -> Plan {
    let package = match input.package {
        Package::Deb => fl!("package-deb"),
        Package::Flatpak => "Flatpak".to_string(),
        Package::Source => fl!("package-source"),
    };
    let mut entries = vec![
        Entry::Back {
            icon: BACK_ICON,
            text: fl!("about-back"),
        },
        Entry::Separator,
        Entry::Name("Puron".to_string()),
        // The version goes after the entry and a space in the code: a
        // catalog argument would wrap it in isolation marks.
        Entry::Version(format!("{} {}", fl!("about-version"), input.version)),
        Entry::Summary(fl!("about-summary")),
        Entry::Field {
            label: fl!("about-package"),
            values: vec![package],
        },
        // The source code is MIT, but the applet program as built links
        // libraries under GPL-3.0-only, so one "MIT" alone would be wrong.
        Entry::Field {
            label: fl!("about-license"),
            values: vec![
                format!("{}: MIT", fl!("license-source")),
                format!("{}: GPL-3.0-only", fl!("license-program")),
            ],
        },
        Entry::Field {
            label: fl!("about-copyright"),
            values: vec![COPYRIGHT.to_string()],
        },
        Entry::Heading(fl!("about-sources")),
    ];
    for (i, (s, (_, name))) in input.sources.iter().zip(SOURCES).enumerate() {
        let (state, count) = match *s {
            _ if flatpak_liquidctl(input.package, i, *s) => (fl!("source-flatpak"), None),
            Source::Off => (fl!("source-off"), None),
            Source::NotAvailable => (fl!("source-unavailable"), None),
            Source::Available(n) => (fl!("source-available"), Some(n)),
        };
        entries.push(Entry::Source { name, state, count });
    }
    entries.push(Entry::Heading(fl!("about-links")));
    for (text, url) in [
        (fl!("link-about-homepage"), HOMEPAGE_URL.to_string()),
        (fl!("link-about-source"), SOURCE_URL.to_string()),
        (fl!("link-about-issues"), ISSUES_URL.to_string()),
        (fl!("link-about-notices"), notices_url(&input.version)),
    ] {
        entries.push(Entry::Link { text, url });
    }
    // The "Copy details" button. The page is complete without it; the
    // text it copies is built in `copy`.
    entries.push(Entry::Copy {
        text: fl!("about-copy"),
    });
    Plan {
        looks: looks(&entries, input),
        entries,
        side_padding: PAGE_SIDE_PADDING,
    }
}

/// The widget message of a click on the About page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clicked {
    Back,
    Link(String),
    Copy,
}

/// The answer to a click on the About page, given `plan`, the page's plan
/// at the time of the click. "Back" returns to the temperatures and
/// "Copy details" copies; both leave the popup open. A link opens its
/// address in the browser and closes the popup, as the temperatures'
/// links do; an address that `plan` does not hold sends nothing.
pub fn clicked(plan: &Plan, clicked: Clicked) -> Reply {
    let (event, close) = match clicked {
        Clicked::Back => (Some(Event::Back), false),
        Clicked::Link(url) => {
            let shown = plan
                .entries
                .iter()
                .any(|e| matches!(e, Entry::Link { url: u, .. } if *u == url));
            if shown {
                (Some(Event::OpenLink(url)), true)
            } else {
                (None, false)
            }
        }
        Clicked::Copy => (Some(Event::CopyDetails), false),
    };
    Reply { event, close }
}

/// The width in px of a field's label column on the About page.
const LABEL_WIDTH: f32 = 92.0;
/// The width in px of a source's name column.
const SOURCE_NAME_WIDTH: f32 = 82.0;
/// The width in px of a source's count column, at whose right edge the
/// count ends.
const COUNT_WIDTH: f32 = 28.0;
/// The space in px between the blocks below the separator: the header,
/// each card with its heading, and the button.
const BLOCK_GAP: f32 = 16.0;
/// The space in px between a heading and the card under it.
const HEADING_GAP: f32 = 6.0;

/// The widget ID of the card at `place`: `card-facts`, `card-sources` or
/// `card-links`, so that tests can find its rectangle. Any other place has
/// no card and gets `card-none`.
pub fn card_id(place: Place) -> Id {
    Id::new(match place {
        Place::CardFacts => "card-facts",
        Place::CardSources => "card-sources",
        Place::CardLinks => "card-links",
        _ => "card-none",
    })
}

/// The text at index `n` of `look.texts`, drawn in its style and role.
fn styled_text<'a>(content: String, look: &Look, n: usize) -> cosmic::Element<'a, Clicked> {
    let (style, role) = look
        .texts
        .get(n)
        .copied()
        .unwrap_or((TextStyle::Body, Role::Default));
    look::styled(content, style, role)
}

/// The widgets of entry `i`, without the container that carries its ID.
/// A text that tests measure sits in a container with the ID
/// `popup::element_id(i, part)`: Back's and a link's `icon`, a link's,
/// the header's, a heading's and the button's `text`, a field's `label`,
/// and a source's `state` and `count`; the "Copy details" button itself
/// sits in one with the ID `button`.
fn entry_view<'a>(i: usize, e: &Entry, look: &Look) -> cosmic::Element<'a, Clicked> {
    let part = |content: cosmic::Element<'a, Clicked>, name: &str| {
        container(content).id(element_id(i, name))
    };
    let icon_of = |size| {
        look.icon
            .map(|(name, role)| look::tinted_icon(name, size, role))
            .unwrap_or_else(|| Space::new().into())
    };
    match e {
        Entry::Back { text: t, .. } => {
            let parts = row::with_children(vec![
                part(icon_of(16), "icon").into(),
                styled_text(t.clone(), look, 0),
            ])
            .spacing(8)
            .align_y(Alignment::Center);
            menu_button(parts).on_press(Clicked::Back).into()
        }
        Entry::Separator => padded(divider::horizontal::default()).into(),
        Entry::Name(t) | Entry::Version(t) | Entry::Summary(t) | Entry::Heading(t) => {
            let horizontal = match look.align {
                Align::Center => Horizontal::Center,
                Align::Start => Horizontal::Left,
            };
            container(part(styled_text(t.clone(), look, 0), "text"))
                .width(Length::Fill)
                .align_x(horizontal)
                .into()
        }
        Entry::Field { label, values } => {
            let values = column::with_children(
                values
                    .iter()
                    .enumerate()
                    .map(|(n, v)| styled_text(v.clone(), look, n + 1))
                    .collect::<Vec<cosmic::Element<'a, Clicked>>>(),
            )
            .spacing(4)
            .width(Length::Fill);
            row::with_children(vec![
                part(styled_text(label.clone(), look, 0), "label")
                    .width(Length::Fixed(LABEL_WIDTH))
                    .into(),
                values.into(),
            ])
            .spacing(10)
            .into()
        }
        Entry::Source { name, state, count } => {
            let count = count.map(|n| n.to_string()).unwrap_or_default();
            let parts = row::with_children(vec![
                container(styled_text(name.to_string(), look, 0))
                    .width(Length::Fixed(SOURCE_NAME_WIDTH))
                    .into(),
                container(part(styled_text(state.clone(), look, 1), "state"))
                    .width(Length::Fill)
                    .into(),
                container(part(styled_text(count, look, 2), "count"))
                    .width(Length::Fixed(COUNT_WIDTH))
                    .align_x(Horizontal::Right)
                    .into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center);
            container(parts).padding([6.0, 0.0]).into()
        }
        Entry::Link { text: t, url } => {
            let parts = row::with_children(vec![
                part(styled_text(t.clone(), look, 0), "text")
                    .width(Length::Fill)
                    .into(),
                part(icon_of(16), "icon").into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center);
            button::custom(parts)
                .class(cosmic::theme::Button::Link)
                .padding([10.0, 14.0])
                .width(Length::Fill)
                .on_press(Clicked::Link(url.clone()))
                .into()
        }
        // libcosmic's `button::suggested` puts its label at the left of a
        // wide button, so the button is built from the same class with
        // the label centered in it.
        Entry::Copy { text: t } => {
            let space = cosmic::theme::spacing();
            let label = container(part(styled_text(t.clone(), look, 0), "text"))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);
            let pressed = button::custom(label)
                .class(press_class(look.press))
                .padding([0.0, f32::from(space.space_s)])
                .width(Length::Fill)
                .height(Length::Fixed(f32::from(space.space_l)))
                .on_press(Clicked::Copy);
            // The container that carries the button's ID sets no width of
            // its own, so its rectangle is the button's, whatever width the
            // button has.
            part(pressed.into(), "button").into()
        }
    }
}

/// The libcosmic button class of `press`, so that the button drawn is the
/// one that the plan names.
fn press_class(press: Option<Press>) -> cosmic::theme::Button {
    match press {
        Some(Press::Menu) => cosmic::theme::Button::AppletMenu,
        Some(Press::Link) => cosmic::theme::Button::Link,
        Some(Press::Suggested) => cosmic::theme::Button::Suggested,
        None => cosmic::theme::Button::Standard,
    }
}

/// The page's content, built from the plan alone with libcosmic's
/// widgets. "Back" and the separator are full-width entries like those of
/// the temperatures. Below them, between the two side paddings, come the
/// centered header, the card of the facts, the heading "Sensor sources"
/// with its card, the heading "Links" with its card, and the button.
///
/// Each entry sits in a container with the ID `popup::entry_id(index)`,
/// and each card has the ID of `card_id`. Texts wrap instead of being
/// cut, and content taller than the popup scrolls.
pub fn view<'a>(plan: &Plan) -> cosmic::Element<'a, Clicked> {
    // The entries in runs of the same place, in order; each entry on the
    // page itself is a run of its own, marked when it is a heading.
    let mut runs: Vec<(Place, bool, Vec<cosmic::Element<'a, Clicked>>)> = Vec::new();
    for (i, (e, look)) in plan.entries.iter().zip(&plan.looks).enumerate() {
        let entry: cosmic::Element<'a, Clicked> = container(entry_view(i, e, look))
            .id(entry_id(i))
            .width(Length::Fill)
            .into();
        match runs.last_mut() {
            Some((place, _, run)) if *place == look.place && look.place != Place::Page => {
                run.push(entry)
            }
            _ => runs.push((look.place, matches!(e, Entry::Heading(_)), vec![entry])),
        }
    }
    let mut rows: Vec<cosmic::Element<'a, Clicked>> = Vec::new();
    let mut blocks: Vec<cosmic::Element<'a, Clicked>> = Vec::new();
    // A heading waits here for the card that follows it, so that the two
    // form one block with the smaller gap between them.
    let mut heading: Option<cosmic::Element<'a, Clicked>> = None;
    for (place, is_heading, run) in runs {
        let block: cosmic::Element<'a, Clicked> = match place {
            Place::Row => {
                rows.extend(run);
                continue;
            }
            Place::Header => column::with_children(run)
                .spacing(2)
                .width(Length::Fill)
                .into(),
            Place::CardFacts => look::card(column::with_children(run).spacing(6), [12.0, 14.0])
                .id(card_id(place))
                .into(),
            Place::CardSources => look::card(column::with_children(run), [6.0, 14.0])
                .id(card_id(place))
                .into(),
            Place::CardLinks => look::card(column::with_children(run), [4.0, 0.0])
                .id(card_id(place))
                .into(),
            Place::Page => {
                let entry = column::with_children(run).width(Length::Fill).into();
                if is_heading {
                    blocks.extend(heading.replace(entry));
                    continue;
                }
                entry
            }
        };
        match heading.take() {
            Some(h) => blocks.push(
                column::with_children(vec![h, block])
                    .spacing(HEADING_GAP)
                    .width(Length::Fill)
                    .into(),
            ),
            None => blocks.push(block),
        }
    }
    blocks.extend(heading);
    let content = container(
        column::with_children(blocks)
            .spacing(BLOCK_GAP)
            .width(Length::Fill),
    )
    .padding(look::content_padding())
    .width(Length::Fill);
    rows.push(content.into());
    scrollable(column::with_children(rows).width(Length::Fill)).into()
}
