//! The facts that the popup's About page shows about this program: the
//! kind of package it was built for, its license lines, its copyright
//! line, its web addresses and the state of each source of sensors, and
//! the page drawn from them. The page is the popup's second page: the
//! entry "About" of the temperatures shows it, and its "Back" returns.

use cosmic::applet;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, divider, icon, row, scrollable, text};
use puron_core::menu::HOMEPAGE_URL;
use puron_core::rows::Provider;
use puron_core::settings::Settings;
use puron_engine::engine::{ProviderState, Snapshot};
use puron_engine::registry;

use crate::popup::{entry_id, Event, Reply};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub entries: Vec<Entry>,
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
    Plan { entries }
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

/// The page's content, built from the plan alone with libcosmic's applet
/// widgets, so that it looks like the temperatures. Each entry sits in a
/// container with the ID `popup::entry_id(index)`. Texts wrap instead of
/// being cut, and content taller than the popup scrolls.
pub fn view<'a>(plan: &Plan) -> cosmic::Element<'a, Clicked> {
    let children: Vec<cosmic::Element<'a, Clicked>> = plan
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| -> cosmic::Element<'a, Clicked> {
            let content: cosmic::Element<'a, Clicked> = match e {
                Entry::Back {
                    icon: name,
                    text: t,
                } => {
                    let parts = row::with_children(vec![
                        icon::from_name(*name).size(16).symbolic(true).icon().into(),
                        text(t.clone()).into(),
                    ])
                    .spacing(8)
                    .align_y(Alignment::Center);
                    applet::menu_button(parts).on_press(Clicked::Back).into()
                }
                Entry::Separator => applet::padded_control(divider::horizontal::default()).into(),
                Entry::Name(t) => applet::padded_control(text::title3(t.clone())).into(),
                Entry::Version(t) | Entry::Summary(t) => {
                    applet::padded_control(text(t.clone()).width(Length::Fill)).into()
                }
                Entry::Field { label, values } => {
                    let values = column::with_children(
                        values
                            .iter()
                            .map(|v| text(v.clone()).width(Length::Fill).into())
                            .collect::<Vec<cosmic::Element<'a, Clicked>>>(),
                    )
                    .width(Length::FillPortion(2));
                    let parts = row::with_children(vec![
                        text::heading(label.clone())
                            .width(Length::FillPortion(1))
                            .into(),
                        values.into(),
                    ])
                    .spacing(8);
                    applet::padded_control(parts).into()
                }
                Entry::Heading(t) => {
                    applet::padded_control(text::heading(t.clone()).width(Length::Fill)).into()
                }
                Entry::Source { name, state, count } => {
                    let count = count.map(|n| n.to_string()).unwrap_or_default();
                    let parts = row::with_children(vec![
                        text(*name).width(Length::FillPortion(1)).into(),
                        text(state.clone()).width(Length::FillPortion(2)).into(),
                        text(count).into(),
                    ])
                    .spacing(8);
                    applet::padded_control(parts).into()
                }
                Entry::Link { text: t, url } => {
                    applet::menu_button(text(t.clone()).width(Length::Fill))
                        .on_press(Clicked::Link(url.clone()))
                        .into()
                }
                Entry::Copy { text: t } => {
                    applet::padded_control(button::standard(t.clone()).on_press(Clicked::Copy))
                        .into()
                }
            };
            container(content)
                .id(entry_id(i))
                .width(Length::Fill)
                .into()
        })
        .collect();
    scrollable(column::with_children(children).width(Length::Fill)).into()
}
