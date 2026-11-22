//! Tests of the look of the temperatures: the icon tile of each section
//! heading, the icon of each device heading, the cards that hold the
//! lines, the check mark's color, the icons of "Settings" and "About", and
//! the color of each value by how hot it runs. Each test must finish
//! within 5 seconds (`puron_testkit::limit`).
//!
//! The plan tests read the popup plan without a display. The layout tests
//! draw the plan in iced's headless test simulator, find widgets by the
//! IDs that `popup::view` gives them, and compare their rectangles in
//! logical pixels with a tolerance of half a pixel. The readings come from
//! the fixtures under crates/puron-testkit/fixtures/raw/ (kinds.json,
//! heat.json, drives.json, capture.json, pins.json); the icon names are
//! checked against crates/puron-testkit/fixtures/icons/names.tsv, the list
//! of icons that the applet is known to find.
#![allow(non_snake_case)]

mod common;

use std::collections::BTreeMap;

use cosmic::applet::cosmic_panel_config::PanelSize;
use cosmic::applet::{Context, Size as AppletSize};
use cosmic::iced::{Rectangle, Size};
use cosmic::widget::{container, Id};
use cosmic::Theme;
use iced_test::{selector, Simulator};
use puron_applet::look::Role;
use puron_applet::panel;
use puron_applet::popup::{self, Clicked, Drawn, Plan};
use puron_core::menu::{self, Menu, Report};
use puron_core::rows::RawReading;
use puron_core::settings::{Settings, TemperatureUnit};

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

fn raw(name: &str) -> String {
    std::fs::read_to_string(puron_testkit::fixture("raw", name)).expect("read fixture")
}

/// The reading set `set` of the fixture `file`.
fn readings(file: &str, set: &str) -> Vec<RawReading> {
    let mut sets = puron_testkit::raw_sets(file, |r: &mut RawReading, v| r.value = Some(v));
    sets.remove(set)
        .unwrap_or_else(|| panic!("set {set} of {file}"))
}

fn capture() -> Vec<RawReading> {
    serde_json::from_str(&raw("capture.json")).expect("parse capture.json")
}

/// The pinned list `P1` of `pins.json`.
fn p1() -> Vec<String> {
    let sets: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&raw("pins.json")).expect("parse pins.json");
    serde_json::from_value(sets["P1"].clone()).expect("P1")
}

/// The layout of `raw` under `settings`, with lm-sensors answered and the
/// older pins `waiting` waiting for it.
fn lay(raw: &[RawReading], settings: &Settings, waiting: &[String]) -> Menu {
    let shown = menu::shown(raw, settings);
    menu::menu_with(&shown, settings, Report::Answered, None, false, waiting)
}

fn plan_of(menu: &Menu, pins: &[String]) -> Plan {
    popup::plan(menu, pins, icon_size())
}

fn fahrenheit() -> Settings {
    Settings {
        temperature_unit: TemperatureUnit::Fahrenheit,
        ..Settings::default()
    }
}

/// The plan of the drive set `example` in Fahrenheit with the pins of a
/// new install: a CPU group, one drive, a Wi-Fi card, a fan and a
/// voltage.
fn example_plan() -> Plan {
    let raw = readings("drives.json", "example");
    let s = fahrenheit();
    let shown = menu::shown(&raw, &s);
    let m = menu::menu(&shown, &s, Report::Answered, None, false);
    plan_of(&m, &shown.new_install_pins())
}

/// Whether `name` is a line of the icon list of the test fixtures.
fn listed(name: &str) -> bool {
    let names = std::fs::read_to_string(puron_testkit::fixture("icons", "names.tsv"))
        .expect("read names.tsv");
    names.lines().any(|l| l.split('\t').nth(2) == Some(name))
}

/// The identity of the reading of `raw` named `name`; heat.json holds
/// lm-sensors temperatures only, apart from set `edge`.
fn id_of(raw: &[RawReading], name: &str) -> String {
    let r = raw
        .iter()
        .find(|r| r.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("reading {name}"));
    format!("temperature/lm-sensors/{}", r.key)
}

/// The drawn line with identity `id`.
fn line<'a>(plan: &'a Plan, id: &str) -> &'a popup::Line {
    plan.entries
        .iter()
        .find_map(|e| match e {
            Drawn::Line(l) if l.identity == id => Some(l),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no line {id}"))
}

/// Each device heading's icon name and role, with the identity of the
/// first line under it.
fn devices(plan: &Plan) -> Vec<(&'static str, Role, String)> {
    let mut out = Vec::new();
    for (i, e) in plan.entries.iter().enumerate() {
        if let Drawn::Device { icon, role, .. } = e {
            let first = plan.entries[i + 1..].iter().find_map(|e| match e {
                Drawn::Line(l) => Some(l.identity.clone()),
                _ => None,
            });
            out.push((*icon, *role, first.expect("a line under the heading")));
        }
    }
    out
}

type Sim = Simulator<'static, Clicked, Theme, cosmic::Renderer>;

/// A simulator that draws `plan` in a popup `width` px wide, with the ID
/// `root` on the whole of it.
fn simulate(plan: &Plan, width: f32) -> Sim {
    let content = container(popup::view(plan)).id(Id::new("root"));
    Simulator::with_size(Default::default(), Size::new(width, HEIGHT), content)
}

fn rect(sim: &mut Sim, id: Id) -> Rectangle {
    sim.find(selector::id(id.clone()))
        .unwrap_or_else(|e| panic!("find {id:?}: {e:?}"))
        .bounds()
}

fn part(sim: &mut Sim, i: usize, name: &str) -> Rectangle {
    rect(sim, popup::element_id(i, name))
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

fn right(r: &Rectangle) -> f32 {
    r.x + r.width
}

fn bottom(r: &Rectangle) -> f32 {
    r.y + r.height
}

/// Whether `r` lies within `area`, give or take `TOL`.
fn inside(r: Rectangle, area: Rectangle) -> bool {
    r.x >= area.x - TOL
        && r.y >= area.y - TOL
        && right(&r) <= right(&area) + TOL
        && bottom(&r) <= bottom(&area) + TOL
}

/// The readings of the four sections: each section heading has the icon
/// of its class and the color of its tile. Voltage and Power share the
/// lightning icon and differ in color. Each icon is one that the applet
/// is known to find.
#[test]
fn c_applet_popup_49() {
    puron_testkit::limit("C-applet.popup-49", LIMIT, || {
        common::start_in_english();
        let raw: Vec<RawReading> = serde_json::from_value(
            serde_json::from_str::<BTreeMap<String, serde_json::Value>>(&raw("kinds.json"))
                .expect("parse kinds.json")
                .remove("R")
                .expect("X-raw-kinds rows"),
        )
        .expect("X-raw-kinds readings");
        let plan = plan_of(&lay(&raw, &Settings::default(), &[]), &[]);
        let got: Vec<(&str, &str, Role)> = plan
            .entries
            .iter()
            .filter_map(|e| match e {
                Drawn::Section { text, icon, role } => Some((text.as_str(), *icon, *role)),
                _ => None,
            })
            .collect();
        assert_eq!(
            got,
            [
                ("Temperature", "puron-temperature-symbolic", Role::Orange),
                ("Fan", "puron-fan-symbolic", Role::Accent),
                ("Voltage", "puron-voltage-symbolic", Role::Yellow),
                ("Power", "puron-voltage-symbolic", Role::Purple),
            ]
        );
        for (_, icon, _) in &got {
            assert!(listed(icon), "{icon} is not in names.tsv");
        }
    });
}

/// Set `limits` of heat.json has one device of each kind: each device
/// heading has the icon of its kind, the panel's, in the kind's color; a
/// device that Puron does not tell apart is grey. In set `waiting` the CPU
/// group that holds only the waiting line still has the CPU's icon. Each
/// icon is one that the applet is known to find.
#[test]
fn c_applet_popup_50() {
    puron_testkit::limit("C-applet.popup-50", LIMIT, || {
        common::start_in_english();
        let raw = readings("heat.json", "limits");
        let plan = plan_of(&lay(&raw, &Settings::default(), &[]), &[]);
        let want = [
            (
                "coretemp-isa-0000",
                "puron-cpu-temperature-symbolic",
                Role::Yellow,
            ),
            (
                "amdgpu-pci-0300",
                "puron-gpu-temperature-symbolic",
                Role::Purple,
            ),
            (
                "nvme-pci-0100",
                "drive-harddisk-system-symbolic",
                Role::Indigo,
            ),
            (
                "iwlwifi_1-virtual-0",
                "network-wireless-symbolic",
                Role::Green,
            ),
            ("ixgbe-pci-0900", "network-wired-symbolic", Role::Green),
            (
                "jc42-i2c-0-18",
                "puron-memory-temperature-symbolic",
                Role::Indigo,
            ),
            ("acpitz-acpi-0", "puron-temperature-symbolic", Role::Muted),
        ];
        let got = devices(&plan);
        assert_eq!(got.len(), 7, "{got:?}");
        for (icon, role, first) in &got {
            let (chip, want_icon, want_role) = want
                .iter()
                .find(|(chip, _, _)| first.contains(chip))
                .unwrap_or_else(|| panic!("no device for {first}"));
            assert_eq!((*icon, *role), (*want_icon, *want_role), "{chip}");
            assert!(listed(icon), "{icon} is not in names.tsv");
        }
        let raw = readings("heat.json", "waiting");
        let waiting = ["@cpu".to_string()];
        let plan = plan_of(&lay(&raw, &Settings::default(), &waiting), &waiting);
        assert_eq!(
            devices(&plan)[0],
            (
                "puron-cpu-temperature-symbolic",
                Role::Yellow,
                "@cpu".to_string()
            )
        );
    });
}

/// The drive set `example`: the lines of each device, and of the fan and
/// the voltage sections, are numbered with one card each, in plan order,
/// and no heading or other entry is in a card. Drawn 472 px wide, each
/// line lies inside the rectangle of its card, the cards do not overlap,
/// each card starts below the heading before it, and two lines of one
/// card are at least 1 px apart, where the divider lies.
#[test]
fn c_applet_popup_51() {
    puron_testkit::limit("C-applet.popup-51", LIMIT, || {
        common::start_in_english();
        let plan = example_plan();
        let cards: Vec<usize> = plan
            .entries
            .iter()
            .filter_map(|e| match e {
                Drawn::Line(l) => Some(l.card),
                _ => None,
            })
            .collect();
        assert_eq!(cards, [1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 4, 5]);
        let mut sim = simulate(&plan, popup::MAX_WIDTH);
        let boxes: Vec<Rectangle> = (1..=5).map(|n| rect(&mut sim, popup::card_id(n))).collect();
        for a in 0..boxes.len() {
            for b in a + 1..boxes.len() {
                assert!(
                    bottom(&boxes[a]) <= boxes[b].y + TOL,
                    "card {} {:?} overlaps card {} {:?}",
                    a + 1,
                    boxes[a],
                    b + 1,
                    boxes[b]
                );
            }
        }
        let mut heading: Option<Rectangle> = None;
        let mut last: Option<(usize, Rectangle)> = None;
        for (i, e) in plan.entries.iter().enumerate() {
            let r = rect(&mut sim, popup::entry_id(i));
            match e {
                Drawn::Line(l) => {
                    let card = boxes[l.card - 1];
                    assert!(inside(r, card), "line {i} {r:?} outside card {card:?}");
                    if let Some(h) = heading {
                        assert!(
                            card.y >= bottom(&h) - TOL,
                            "card {} {card:?} above its heading {h:?}",
                            l.card
                        );
                    }
                    if let Some((n, above)) = last {
                        if n == l.card {
                            assert!(
                                r.y - bottom(&above) >= 1.0 - TOL,
                                "lines {} and {i} of card {n} touch",
                                i - 1
                            );
                        }
                    }
                    last = Some((l.card, r));
                }
                Drawn::Section { .. } | Drawn::Device { .. } => {
                    heading = Some(r);
                    last = None;
                }
                _ => last = None,
            }
        }
    });
}

/// The captured readings with the pins `P1`: the check mark has the
/// accent color, and it is on the four pinned lines only.
#[test]
fn c_applet_popup_52() {
    puron_testkit::limit("C-applet.popup-52", LIMIT, || {
        common::start_in_english();
        let plan = plan_of(&lay(&capture(), &Settings::default(), &[]), &p1());
        assert_eq!(plan.check_role, Role::Accent);
        let marked: Vec<&str> = plan
            .entries
            .iter()
            .filter_map(|e| match e {
                Drawn::Line(l) if l.pinned => Some(l.text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(marked, ["coretemp: CPU", "SSD", "iwlwifi: WiFi", "CPU fan"]);
    });
}

/// The entries "Settings" and "About" have a grey gear and a grey "i",
/// with the readings and with the message of no readings alike; each icon
/// is one that the applet is known to find. Drawn 472 px wide, each icon
/// is 16 × 16 px and starts 24 px right of the popup's left edge, its text
/// starts 10 px after it, and the two share a vertical center.
#[test]
fn c_applet_popup_53() {
    puron_testkit::limit("C-applet.popup-53", LIMIT, || {
        common::start_in_english();
        let readings = plan_of(&lay(&capture(), &Settings::default(), &[]), &p1());
        let message = plan_of(&lay(&[], &Settings::default(), &[]), &[]);
        for plan in [&readings, &message] {
            let links: Vec<(&str, &str, Role)> = plan
                .entries
                .iter()
                .filter_map(|e| match e {
                    Drawn::SettingsLink { text, icon, role }
                    | Drawn::HomepageLink { text, icon, role } => {
                        Some((text.as_str(), *icon, *role))
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                links,
                [
                    ("Settings", "preferences-system-symbolic", Role::Muted),
                    ("About", "help-about-symbolic", Role::Muted),
                ]
            );
            for (_, icon, _) in &links {
                assert!(listed(icon), "{icon} is not in names.tsv");
            }
        }
        let mut sim = simulate(&readings, popup::MAX_WIDTH);
        let root = rect(&mut sim, Id::new("root"));
        for (i, e) in readings.entries.iter().enumerate() {
            if !matches!(e, Drawn::SettingsLink { .. } | Drawn::HomepageLink { .. }) {
                continue;
            }
            let icon = part(&mut sim, i, "icon");
            let text = part(&mut sim, i, "name");
            assert!(
                near(icon.width, 16.0) && near(icon.height, 16.0),
                "entry {i}: icon {icon:?}"
            );
            assert!(near(icon.x - root.x, 24.0), "entry {i}: icon {icon:?}");
            assert!(
                near(text.x, right(&icon) + 10.0),
                "entry {i}: text {text:?} after icon {icon:?}"
            );
            assert!(
                (icon.center().y - text.center().y).abs() <= 1.0,
                "entry {i}: icon {icon:?} and text {text:?} off center"
            );
        }
    });
}

/// Set `limits` of heat.json, in Celsius and then in Fahrenheit. Each
/// group of six readings lies below, at and above the warm limit of its
/// kind, then below, at and above its hot limit, so the values are drawn
/// in the normal color, the warning color three times and the red twice.
/// The Maximum lines are red; the wired network card at 60 °C is in the
/// warning color, the memory module at 80 °C red, the ACPI zone at 59.5 °C
/// normal. The colors are the same in both units, because the limits are
/// read on the value in °C, while the value shown changes unit.
#[test]
fn c_applet_popup_54() {
    puron_testkit::limit("C-applet.popup-54", LIMIT, || {
        common::start_in_english();
        let raw = readings("heat.json", "limits");
        let six = [
            Role::Default,
            Role::Warning,
            Role::Warning,
            Role::Warning,
            Role::Destructive,
            Role::Destructive,
        ];
        let groups = [
            ["Core 0", "Core 1", "Core 2", "Core 3", "Core 4", "Core 5"],
            ["g1", "g2", "g3", "g4", "g5", "g6"],
            ["d1", "d2", "d3", "d4", "d5", "d6"],
            ["w1", "w2", "w3", "w4", "w5", "w6"],
        ];
        let maxima = [
            "@maximum/cpu",
            "@maximum/lm-sensors/amdgpu-pci-0300",
            "@maximum/lm-sensors/nvme-pci-0100",
            "@maximum/lm-sensors/iwlwifi_1-virtual-0",
        ];
        let mut shown = Vec::new();
        for (settings, unit) in [(Settings::default(), "°C"), (fahrenheit(), "°F")] {
            let plan = plan_of(&lay(&raw, &settings, &[]), &[]);
            for names in groups {
                let got: Vec<Role> = names
                    .iter()
                    .map(|n| line(&plan, &id_of(&raw, n)).value_role)
                    .collect();
                assert_eq!(got, six, "{names:?} in {unit}");
            }
            for id in maxima {
                assert_eq!(
                    line(&plan, id).value_role,
                    Role::Destructive,
                    "{id} in {unit}"
                );
            }
            let role = |n: &str| line(&plan, &id_of(&raw, n)).value_role;
            assert_eq!(role("e1"), Role::Warning, "e1 in {unit}");
            assert_eq!(role("m1"), Role::Destructive, "m1 in {unit}");
            assert_eq!(role("o1"), Role::Default, "o1 in {unit}");
            shown.push(line(&plan, &id_of(&raw, "Core 1")).value.clone());
        }
        assert_eq!(shown, ["75°C", "167°F"]);
    });
}

/// The drive set `example`, drawn 360 and 472 px wide. Each section's
/// tile is 28 × 28 px with its 16 × 16 icon centered within 1 px, and the
/// heading text starts 10 px after the tile. Each device heading's icon is
/// 16 × 16 px, its name starts 8 px after it, and the two share a vertical
/// center. Each value ends 14 px inside its card's right edge, and each
/// line is at least 32 px high inside its 1 px of padding above and below.
#[test]
fn c_applet_popup_56() {
    puron_testkit::limit("C-applet.popup-56", LIMIT, || {
        common::start_in_english();
        let plan = example_plan();
        for width in [popup::WIDTH, popup::MAX_WIDTH] {
            let mut sim = simulate(&plan, width);
            for (i, e) in plan.entries.iter().enumerate() {
                match e {
                    Drawn::Section { .. } => {
                        let tile = part(&mut sim, i, "tile");
                        let icon = part(&mut sim, i, "icon");
                        let heading = part(&mut sim, i, "heading");
                        assert!(
                            near(tile.width, 28.0) && near(tile.height, 28.0),
                            "{width}: tile {i} {tile:?}"
                        );
                        assert!(
                            near(icon.width, 16.0) && near(icon.height, 16.0),
                            "{width}: tile icon {i} {icon:?}"
                        );
                        let (tc, ic) = (tile.center(), icon.center());
                        assert!(
                            (tc.x - ic.x).abs() <= 1.0 && (tc.y - ic.y).abs() <= 1.0,
                            "{width}: icon {icon:?} not centered in {tile:?}"
                        );
                        assert!(
                            near(heading.x, right(&tile) + 10.0),
                            "{width}: heading {heading:?} after tile {tile:?}"
                        );
                    }
                    Drawn::Device { .. } => {
                        let icon = part(&mut sim, i, "icon");
                        let name = part(&mut sim, i, "name");
                        assert!(
                            near(icon.width, 16.0) && near(icon.height, 16.0),
                            "{width}: device icon {i} {icon:?}"
                        );
                        assert!(
                            near(name.x, right(&icon) + 8.0),
                            "{width}: name {name:?} after icon {icon:?}"
                        );
                        assert!(
                            (icon.center().y - name.center().y).abs() <= 1.0,
                            "{width}: icon {icon:?} and name {name:?} off center"
                        );
                    }
                    Drawn::Line(l) => {
                        let card = rect(&mut sim, popup::card_id(l.card));
                        let value = part(&mut sim, i, "value");
                        assert!(
                            near(right(&value), right(&card) - 14.0),
                            "{width}: value of {} ends at {} in card {card:?}",
                            l.text,
                            right(&value)
                        );
                        let r = rect(&mut sim, popup::entry_id(i));
                        assert!(r.height >= 32.0 - TOL, "{width}: line {i} {r:?}");
                    }
                    _ => {}
                }
            }
        }
    });
}
