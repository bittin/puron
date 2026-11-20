//! Tests of the look that the popup's pages share: the side padding of
//! every page, and the color roles of the About and Settings pages with
//! the one function that maps them to the theme's colors.
//!
//! The temperatures come from the captured readings with the default
//! settings and the pinned list `P1` of `pins.json`; the About page from
//! version 1.6.0, the Debian package and the source set `all` (lm-sensors
//! available with 12 readings, NVMe with 2, NVIDIA not available, udisks2
//! with 1, liquidctl not available); the Settings page from the `good`
//! settings fixture. The layout tests draw a page in iced's headless test
//! simulator at the narrowest and the widest popup width, find widgets
//! by the IDs that the views give them, and compare their rectangles in
//! logical pixels with a tolerance of half a pixel.
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::process::Command;

use cosmic::applet::cosmic_panel_config::PanelSize;
use cosmic::applet::{Context, Size as AppletSize};
use cosmic::iced::{Color, Rectangle, Size};
use cosmic::widget::{container, Id};
use cosmic::Theme;
use iced_test::{selector, Simulator};
use puron_applet::about::{self, Package, Place, Source};
use puron_applet::look::{self, Role, ROLES};
use puron_applet::panel;
use puron_applet::popup::{self, Drawn, PAGE_SIDE_PADDING};
use puron_applet::settings;
use puron_config::Config;
use puron_core::menu::{self, Menu, Report};
use puron_core::rows::RawReading;
use puron_core::settings::Settings;
use puron_testkit::cfg;

const LIMIT: u64 = 5;
/// The tolerance in px of the checks that compare edges.
const TOL: f32 = 0.5;
/// The popup's height limit.
const HEIGHT: f32 = 1000.0;

/// The icon size in px that libcosmic gives for panel size M.
fn icon_size() -> u16 {
    panel::icon_size(&Context {
        size: AppletSize::PanelSize(PanelSize::M),
        ..Context::default()
    })
}

/// The source set `all`, in the page's order.
const ALL: [Source; 5] = [
    Source::Available(12),
    Source::Available(2),
    Source::NotAvailable,
    Source::Available(1),
    Source::NotAvailable,
];

fn raw(name: &str) -> String {
    std::fs::read_to_string(puron_testkit::fixture("raw", name)).expect("read fixture")
}

/// The temperatures' layout of the captured readings with the default
/// settings and lm-sensors answered.
fn capture_menu() -> Menu {
    let raw: Vec<RawReading> = serde_json::from_str(&raw("capture.json")).expect("capture");
    let s = Settings::default();
    menu::menu(&menu::shown(&raw, &s), &s, Report::Answered, None, false)
}

/// The pinned list `P1` of `pins.json`.
fn p1() -> Vec<String> {
    let sets: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&raw("pins.json")).expect("parse pins.json");
    serde_json::from_value(sets["P1"].clone()).expect("P1")
}

fn readings_plan() -> popup::Plan {
    popup::plan(&capture_menu(), &p1(), icon_size())
}

fn about_plan() -> about::Plan {
    about::plan(&about::Input {
        version: "1.6.0".to_string(),
        package: Package::Deb,
        sources: ALL,
    })
}

/// The Settings plan of the `good` fixture, with no typed text and no
/// notice.
fn settings_plan() -> settings::Plan {
    let fx = cfg::build("good");
    settings::plan(&settings::Input {
        values: Config::new(&fx.root).load().settings,
        typed: None,
        notice: false,
    })
}

/// A simulator that draws `content` in a popup `width` px wide, with the
/// ID `root` on the whole of it.
fn sim<'a, M: Clone + 'a>(
    content: cosmic::Element<'a, M>,
    width: f32,
) -> Simulator<'a, M, Theme, cosmic::Renderer> {
    let root = container(content).id(Id::new("root"));
    Simulator::with_size(Default::default(), Size::new(width, HEIGHT), root)
}

fn rect<M: Clone>(s: &mut Simulator<'_, M, Theme, cosmic::Renderer>, id: Id) -> Rectangle {
    s.find(selector::id(id.clone()))
        .unwrap_or_else(|e| panic!("find {id:?}: {e:?}"))
        .bounds()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

/// Asserts that `r` starts `PAGE_SIDE_PADDING` right of the popup's left
/// edge `root`.
fn at_left(what: &str, width: f32, r: Rectangle, root: Rectangle) {
    let side = f32::from(PAGE_SIDE_PADDING);
    assert!(
        near(r.x, root.x + side),
        "width {width}: {what} left edge {} against {}",
        r.x,
        root.x + side
    );
}

/// Asserts that `r` ends `PAGE_SIDE_PADDING` left of the popup's right
/// edge.
fn at_right(what: &str, width: f32, r: Rectangle, root: Rectangle) {
    let side = f32::from(PAGE_SIDE_PADDING);
    assert!(
        near(r.x + r.width, root.x + root.width - side),
        "width {width}: {what} right edge {} against {}",
        r.x + r.width,
        root.x + root.width - side
    );
}

/// The temperatures' plan, the About plan and the Settings plan each carry
/// the side padding, and it is the shared constant, 24 px.
#[test]
fn c_applet_popup_44__plans() {
    puron_testkit::limit("C-applet.popup-44", LIMIT, || {
        puron_applet::start();
        assert_eq!(PAGE_SIDE_PADDING, 24);
        assert_eq!(readings_plan().side_padding, PAGE_SIDE_PADDING, "readings");
        assert_eq!(about_plan().side_padding, PAGE_SIDE_PADDING, "About");
        assert_eq!(settings_plan().side_padding, PAGE_SIDE_PADDING, "Settings");
    });
}

/// The index in `plan` of the section heading "Temperature".
fn temperature_heading(plan: &popup::Plan) -> usize {
    plan.entries
        .iter()
        .position(|e| matches!(e, Drawn::Section { text, .. } if text == "Temperature"))
        .expect("a Temperature heading")
}

/// The temperatures drawn at the narrowest and the widest popup: the icon
/// of the section heading "Temperature" starts 24 px right of the popup's
/// left edge, and every line's value ends 24 px left of its right edge.
#[test]
fn c_applet_popup_44__readings() {
    puron_testkit::limit("C-applet.popup-44", LIMIT, || {
        puron_applet::start();
        let plan = readings_plan();
        let heading = temperature_heading(&plan);
        for width in [popup::WIDTH, popup::MAX_WIDTH] {
            let mut s = sim(popup::view(&plan), width);
            let root = rect(&mut s, Id::new("root"));
            let icon = rect(&mut s, popup::element_id(heading, "icon"));
            at_left("Temperature icon", width, icon, root);
            let mut lines = 0;
            for (i, e) in plan.entries.iter().enumerate() {
                if let Drawn::Line(l) = e {
                    let value = rect(&mut s, popup::element_id(i, "value"));
                    at_right(&format!("value of {}", l.text), width, value, root);
                    lines += 1;
                }
            }
            assert!(lines > 0, "no line drawn");
        }
    });
}

/// The About page drawn at the narrowest and the widest popup: Back's
/// icon, the two headings, the three cards and the "Copy details" button
/// start 24 px right of the popup's left edge, and the cards and the
/// button end 24 px left of its right edge.
#[test]
fn c_applet_popup_44__about() {
    puron_testkit::limit("C-applet.popup-44", LIMIT, || {
        puron_applet::start();
        let plan = about_plan();
        let headings: Vec<usize> = plan
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, about::Entry::Heading(_)))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(headings.len(), 2, "headings");
        let copy = plan.entries.len() - 1;
        assert!(matches!(plan.entries[copy], about::Entry::Copy { .. }));
        for width in [popup::WIDTH, popup::MAX_WIDTH] {
            let mut s = sim(about::view(&plan), width);
            let root = rect(&mut s, Id::new("root"));
            let back = rect(&mut s, popup::element_id(0, "icon"));
            at_left("Back icon", width, back, root);
            for &h in &headings {
                let r = rect(&mut s, popup::entry_id(h));
                at_left(&format!("heading {h}"), width, r, root);
            }
            for place in [Place::CardFacts, Place::CardSources, Place::CardLinks] {
                let r = rect(&mut s, about::card_id(place));
                at_left(&format!("{place:?}"), width, r, root);
                at_right(&format!("{place:?}"), width, r, root);
            }
            let r = rect(&mut s, popup::entry_id(copy));
            at_left("Copy details", width, r, root);
            at_right("Copy details", width, r, root);
        }
    });
}

/// The Settings page drawn at the narrowest and the widest popup: Back's
/// icon starts 24 px right of the popup's left edge.
#[test]
fn c_applet_popup_44__settings() {
    puron_testkit::limit("C-applet.popup-44", LIMIT, || {
        puron_applet::start();
        let plan = settings_plan();
        for width in [popup::WIDTH, popup::MAX_WIDTH] {
            let mut s = sim(settings::view(&plan), width);
            let root = rect(&mut s, Id::new("root"));
            let back = rect(&mut s, settings::id("back-icon"));
            at_left("Back icon", width, back, root);
        }
    });
}

/// No page of the popup takes its side padding from libcosmic's applet
/// helpers, whose padding follows the theme's density: a search of the
/// applet's source for their names finds nothing.
#[test]
fn c_applet_popup_44__source() {
    puron_testkit::limit("C-applet.popup-44", LIMIT, || {
        let out = Command::new("grep")
            .args([
                "-rnE",
                "menu_control_padding|padded_control",
                "crates/puron-applet/src",
            ])
            .current_dir(puron_testkit::workspace_root())
            .output()
            .expect("run grep");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "",
            "the applet still names libcosmic's padding"
        );
        assert_eq!(out.status.code(), Some(1), "grep exit");
    });
}

/// The color that the theme gives `role`, read from the theme's own fields
/// as the table of roles names them.
fn expected(role: Role, theme: &Theme) -> Option<Color> {
    let t = theme.cosmic();
    let p = &t.palette;
    let c = match role {
        Role::Default => return None,
        Role::Muted => {
            let mut c: Color = p.neutral_9.into();
            c.a = 0.7;
            return Some(c);
        }
        Role::Success => t.success_color(),
        Role::Warning => t.warning_color(),
        Role::Accent => t.accent_text_color(),
        Role::Yellow => p.accent_yellow,
        Role::Purple => p.accent_purple,
        Role::Indigo => p.accent_indigo,
        Role::Green => p.accent_green,
    };
    Some(c.into())
}

/// Under libcosmic's built-in dark and light themes, each role gives the
/// color of the theme that the table of roles names, and the default role
/// gives none; every other role has a different color in the dark theme
/// than in the light one, so no role is a fixed color.
#[test]
fn c_applet_popup_47() {
    puron_testkit::limit("C-applet.popup-47", LIMIT, || {
        let dark = Theme::dark();
        let light = Theme::light();
        for role in ROLES {
            for (name, theme) in [("dark", &dark), ("light", &light)] {
                assert_eq!(
                    look::color(role, theme),
                    expected(role, theme),
                    "{role:?} in the {name} theme"
                );
            }
            if role != Role::Default {
                assert_ne!(
                    look::color(role, &dark),
                    look::color(role, &light),
                    "{role:?} is the same in both themes"
                );
            }
        }
        assert_eq!(look::color(Role::Default, &dark), None);
    });
}
