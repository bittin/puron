//! Tests of the popup's About page: its entries and their order, its
//! texts, its source rows, and its place as the popup's second page.
//! The plan functions take the version as an argument, `1.6.0` here; a
//! case through the applet's state machine uses the crate's own version.
#![allow(non_snake_case)]

mod common;

use std::ffi::OsString;

use common::*;
use cosmic::iced::Size;
use cosmic::widget::{container, Id};
use cosmic::Theme;
use iced_test::{selector, Simulator};
use puron_applet::about::{self, Clicked, Entry, Input, Package, Plan, Source};
use puron_applet::app::{Event, Page};
use puron_applet::i18n::{self, test_catalog};
use puron_applet::popup::{self, Event as PopupEvent, Reply};
use puron_applet::view::View;
use puron_core::rows::{Kind, Provider};
use puron_engine::engine::{LmReport, ProviderState, Snapshot};
use puron_testkit::{cfg, eng, loc};

const LIMIT: u64 = 5;
const CHILD: &str = "PURON_ABOUT_CHILD";

/// The source set `all` of the About fixtures: lm-sensors available with
/// 12 readings, NVMe with 2, NVIDIA not available, udisks2 with 1,
/// liquidctl not available; in the page's order.
const ALL: [Source; 5] = [
    Source::Available(12),
    Source::Available(2),
    Source::NotAvailable,
    Source::Available(1),
    Source::NotAvailable,
];

fn input(package: Package, sources: [Source; 5]) -> Input {
    Input {
        version: "1.6.0".to_string(),
        package,
        sources,
    }
}

/// The plan as short strings, one per entry.
fn notation(plan: &Plan) -> Vec<String> {
    plan.entries
        .iter()
        .map(|e| match e {
            Entry::Back { icon, text } => format!("back {icon} {text}"),
            Entry::Separator => "—".into(),
            Entry::Name(t) => format!("name {t}"),
            Entry::Version(t) => format!("version {t}"),
            Entry::Summary(t) => format!("summary {t}"),
            Entry::Field { label, values } => format!("field {label} {}", values.join(" | ")),
            Entry::Heading(t) => format!("heading {t}"),
            Entry::Source { name, state, count } => match count {
                Some(n) => format!("source {name} {state} {n}"),
                None => format!("source {name} {state}"),
            },
            Entry::Link { text, .. } => format!("link {text}"),
            Entry::Copy { text } => format!("copy {text}"),
        })
        .collect()
}

/// Every text that the plan draws, the link addresses aside.
fn texts(plan: &Plan) -> Vec<String> {
    let mut v = Vec::new();
    for e in &plan.entries {
        match e {
            Entry::Back { text, .. } | Entry::Copy { text } | Entry::Link { text, .. } => {
                v.push(text.clone())
            }
            Entry::Name(t) | Entry::Version(t) | Entry::Summary(t) | Entry::Heading(t) => {
                v.push(t.clone())
            }
            Entry::Field { label, values } => {
                v.push(label.clone());
                v.extend(values.iter().cloned());
            }
            Entry::Source { name, state, count } => {
                v.push(name.to_string());
                v.push(state.clone());
                v.extend(count.map(|n| n.to_string()));
            }
            Entry::Separator => {}
        }
    }
    v
}

fn source_rows(plan: &Plan) -> Vec<String> {
    notation(plan)
        .into_iter()
        .filter(|s| s.starts_with("source "))
        .collect()
}

type Sim = Simulator<'static, Clicked, Theme, cosmic::Renderer>;

fn simulate_at(plan: &Plan, width: f32) -> Sim {
    let content = container(about::view(plan)).id(Id::new("root"));
    Simulator::with_size(Default::default(), Size::new(width, 1000.0), content)
}

fn bounds(sim: &mut Sim, id: Id) -> cosmic::iced::Rectangle {
    sim.find(selector::id(id.clone()))
        .unwrap_or_else(|e| panic!("find {id:?}: {e:?}"))
        .bounds()
}

/// The About plan for version 1.6.0, the Debian package and the source
/// set `all`. Expected: exactly the 20 entries of the page in order, with
/// no icon but Back's; drawn at the narrowest and the widest popup width,
/// no entry starts above the end of the one before, and none reaches past
/// the content's right edge.
#[test]
fn c_tests_33() {
    puron_testkit::limit("C-tests-33", LIMIT, || {
        puron_applet::start();
        let plan = about::plan(&input(Package::Deb, ALL));
        assert_eq!(
            notation(&plan),
            [
                "back go-previous-symbolic Back",
                "—",
                "name Puron",
                "version Version 1.6.0",
                "summary Monitor hardware sensors",
                "field Package Debian package",
                "field License Source code: MIT | Applet program as distributed: GPL-3.0-only",
                "field Copyright © 2026 Eric Lalonde",
                "heading Sensor sources",
                "source lm-sensors available 12",
                "source NVMe available 2",
                "source NVIDIA not available",
                "source udisks2 available 1",
                "source liquidctl not available",
                "heading Links",
                "link Homepage",
                "link Source code",
                "link Report a problem",
                "link Third-party notices",
                "copy Copy details",
            ]
        );
        let icons: Vec<_> = plan
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Back { icon, .. } => Some(*icon),
                _ => None,
            })
            .collect();
        assert_eq!(icons, ["go-previous-symbolic"]);
        for width in [popup::WIDTH, popup::MAX_WIDTH] {
            let mut sim = simulate_at(&plan, width);
            let root = bounds(&mut sim, Id::new("root"));
            let mut bottom = f32::MIN;
            for i in 0..plan.entries.len() {
                let r = bounds(&mut sim, popup::entry_id(i));
                assert!(
                    r.y >= bottom - 0.5,
                    "width {width}: entry {i} at {} above {bottom}",
                    r.y
                );
                assert!(
                    r.x + r.width <= root.x + root.width + 0.5,
                    "width {width}: entry {i} reaches {}",
                    r.x + r.width
                );
                bottom = r.y + r.height;
            }
        }
    });
}

/// A snapshot with 12 lm-sensors readings (8 temperatures, 3 fans, 1
/// voltage), 2 nvme-cli and 1 udisks2 reading, and the provider states of
/// the source set `all`.
fn about_snapshot() -> Snapshot {
    let mut readings = Vec::new();
    for i in 0..12u32 {
        let mut r = eng::reading(Provider::LmSensors, i, &format!("t{i}"), 40.0);
        r.kind = Some(match i {
            8..=10 => Kind::Fan,
            11 => Kind::Voltage,
            _ => Kind::Temperature,
        });
        readings.push(r);
    }
    readings.push(eng::reading(Provider::NvmeCli, 0, "n0", 40.0));
    readings.push(eng::reading(Provider::NvmeCli, 1, "n1", 41.0));
    readings.push(eng::reading(Provider::Udisks2, 0, "u0", 30.0));
    Snapshot {
        readings,
        lm_sensors: LmReport::Answered,
        providers: vec![
            (Provider::LmSensors, ProviderState::Available),
            (Provider::Liquidctl, ProviderState::NotAvailable),
            (Provider::Nvidia, ProviderState::NotAvailable),
            (Provider::Udisks2, ProviderState::Available),
            (Provider::NvmeCli, ProviderState::Available),
        ],
    }
}

/// The page that the view of `a` shows, and its About plan if any.
fn view_about(a: &puron_applet::app::App) -> (Page, Option<Plan>) {
    let v = View::of(a);
    (v.page, v.about)
}

/// (a) With fans hidden, a snapshot of 12 lm-sensors readings, 2 NVMe and
/// 1 udisks2 reading, then About: the source rows count every reading of
/// a provider, the hidden fans too. A second snapshot that makes NVIDIA
/// available with one reading changes its row while the page stays the
/// About page. (b) liquidctl in the Flatpak reads "not available in the
/// Flatpak" while its switch is on, "off" when it is off, and "not
/// available" in the Debian package.
#[test]
fn c_tests_37() {
    puron_testkit::limit("C-tests-37", LIMIT, || {
        puron_applet::start();
        let c = cfg::build("good");
        let h = handle(0);
        let mut a = app(&c.root, &h);
        a.handle(Event::Start);
        let d = delivery(&a, &["show_rotation_rate"], |s| {
            s.show_rotation_rate = false
        });
        a.handle(d);
        a.handle(Event::Snapshot(about_snapshot()));
        a.handle(Event::About);
        let (page, plan) = view_about(&a);
        assert_eq!(page, Page::About);
        assert_eq!(
            source_rows(&plan.expect("an About plan")),
            [
                "source lm-sensors available 12",
                "source NVMe available 2",
                "source NVIDIA not available",
                "source udisks2 available 1",
                "source liquidctl not available",
            ]
        );
        let mut s = about_snapshot();
        s.providers[2].1 = ProviderState::Available;
        s.readings
            .push(eng::reading(Provider::Nvidia, 0, "g0", 50.0));
        a.handle(Event::Snapshot(s));
        let (page, plan) = view_about(&a);
        assert_eq!(page, Page::About);
        assert_eq!(
            source_rows(&plan.expect("an About plan"))[2],
            "source NVIDIA available 1"
        );

        let liquidctl =
            |package, sources| source_rows(&about::plan(&input(package, sources)))[4].clone();
        let mut off = ALL;
        off[4] = Source::Off;
        assert_eq!(
            liquidctl(Package::Flatpak, ALL),
            "source liquidctl not available in the Flatpak"
        );
        let mut on = ALL;
        on[4] = Source::Available(3);
        assert_eq!(
            liquidctl(Package::Flatpak, on),
            "source liquidctl not available in the Flatpak"
        );
        assert_eq!(liquidctl(Package::Flatpak, off), "source liquidctl off");
        assert_eq!(
            liquidctl(Package::Deb, ALL),
            "source liquidctl not available"
        );
    });
}

/// The About plan of the Debian package: the License field holds the two
/// lines in order, and the copyright line is exactly the UTF-8 bytes of
/// "© 2026 Eric Lalonde".
#[test]
fn c_tests_38() {
    puron_testkit::limit("C-tests-38", LIMIT, || {
        puron_applet::start();
        let plan = about::plan(&input(Package::Deb, ALL));
        let field = |name: &str| {
            plan.entries
                .iter()
                .find_map(|e| match e {
                    Entry::Field { label, values } if label == name => Some(values.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("field {name}"))
        };
        assert_eq!(
            field("License"),
            [
                "Source code: MIT",
                "Applet program as distributed: GPL-3.0-only"
            ]
        );
        let copyright = field("Copyright");
        assert_eq!(copyright.len(), 1);
        let bytes: Vec<String> = copyright[0].bytes().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            bytes.join(" "),
            "c2 a9 20 32 30 32 36 20 45 72 69 63 20 4c 61 6c 6f 6e 64 65"
        );
    });
}

/// The state machine with a snapshot S1, the popup opened, then About;
/// snapshots S2 (values only) and S3 (a rebuild); Back; About; and the
/// popup opened again. Expected: the temperatures after the popup opens,
/// the About page after About while both snapshots arrive, with the view
/// carrying the About plan; the temperatures with S3's layout after Back;
/// and the temperatures again when the popup opens anew. The entry
/// "About" and Back each leave the popup open.
#[test]
fn c_tests_40() {
    puron_testkit::limit("C-tests-40", LIMIT, || {
        puron_applet::start();
        let c = cfg::build("good");
        let h = handle(0);
        let mut a = app(&c.root, &h);
        a.handle(Event::Start);
        a.handle(Event::Snapshot(rebuild("S1")));
        a.handle(Event::PopupOpened);
        assert_eq!(a.page(), Page::Temperatures);

        let temps = View::of(&a)
            .popup_plan(&puron_applet::panel::Place::of(&Default::default()))
            .expect("a popup plan");
        let w = temps.entries.len() - 1;
        assert_eq!(
            popup::clicked(&temps, popup::Clicked::HomepageLink),
            Reply {
                event: Some(PopupEvent::About),
                close: false
            },
            "the entry {:?}",
            temps.entries[w]
        );
        a.handle(Event::About);
        assert_eq!(a.page(), Page::About);
        let about_plan = about::plan(&a.about_input());
        for (name, snap) in [("S2", rebuild("S2")), ("S3", rebuild("S3"))] {
            a.handle(Event::Snapshot(snap));
            let (page, plan) = view_about(&a);
            assert_eq!(page, Page::About, "after {name}");
            assert_eq!(
                plan.as_ref(),
                Some(&about::plan(&a.about_input())),
                "after {name}"
            );
        }
        let s3 = a.menu().cloned().expect("S3's layout");
        assert_eq!(
            about::clicked(&about_plan, Clicked::Back),
            Reply {
                event: Some(PopupEvent::Back),
                close: false
            }
        );
        a.handle(Event::Back);
        assert_eq!(a.page(), Page::Temperatures);
        let v = View::of(&a);
        assert_eq!(v.about, None);
        assert_eq!(v.menu, Some(s3));
        a.handle(Event::About);
        assert_eq!(a.page(), Page::About);
        a.handle(Event::PopupOpened);
        assert_eq!(a.page(), Page::Temperatures);
    });
}

fn is_child() -> bool {
    std::env::var_os(CHILD).is_some()
}

fn env_var(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// Runs `test` again in a fresh run of this binary with only the locale
/// variables of `env`, and fails unless the child succeeds.
fn child(test: &str, env: &[(&str, &str)]) {
    let mut cmd = puron_testkit::rerun_self(test);
    for v in loc::VARS {
        cmd.env_remove(v);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.env(CHILD, "1").output().expect("run child");
    assert!(
        out.status.success(),
        "child {test}: {}\n{}{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// With Esperanto asked for (map E1) and the test catalog whose texts are
/// the English ones after "EO ", the About plan of the Flatpak: every text
/// from the catalog starts with "EO ", and the fixed texts (the name,
/// "Flatpak", the copyright line, the provider names and the counts)
/// stay as in English.
#[test]
fn c_tests_44() {
    puron_testkit::limit("C-tests-44", LIMIT, || {
        if !is_child() {
            return child("c_tests_44", loc::env("E1"));
        }
        assert_eq!(
            i18n::start_with(test_catalog::eo(), env_var).to_string(),
            "eo"
        );
        let plan = about::plan(&input(Package::Flatpak, ALL));
        let fixed = [
            "Puron",
            "Flatpak",
            "© 2026 Eric Lalonde",
            "lm-sensors",
            "NVMe",
            "NVIDIA",
            "udisks2",
            "liquidctl",
            "12",
            "2",
            "1",
        ];
        let all = texts(&plan);
        for t in &all {
            if fixed.contains(&t.as_str()) {
                continue;
            }
            assert!(t.starts_with("EO "), "{t:?} is not from the catalog");
        }
        for f in fixed {
            assert!(all.iter().any(|t| t == f), "{f} missing: {all:?}");
        }
        for want in [
            "EO Back",
            "EO Version 1.6.0",
            "EO Monitor hardware sensors",
            "EO Source code: MIT",
            "EO Applet program as distributed: GPL-3.0-only",
            "EO not available in the Flatpak",
            "EO Copy details",
        ] {
            assert!(all.iter().any(|t| t == want), "{want} missing: {all:?}");
        }
    });
}

/// The About plan's Package field for each package kind: "Debian
/// package", "Flatpak" and "Built from source".
#[test]
fn c_tests_34__plan() {
    puron_testkit::limit("C-tests-34", LIMIT, || {
        puron_applet::start();
        for (package, want) in [
            (Package::Deb, "field Package Debian package"),
            (Package::Flatpak, "field Package Flatpak"),
            (Package::Source, "field Package Built from source"),
        ] {
            let plan = about::plan(&input(package, ALL));
            assert_eq!(notation(&plan)[5], want, "{package:?}");
        }
    });
}
