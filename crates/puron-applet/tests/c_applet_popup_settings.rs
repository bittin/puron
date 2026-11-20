//! Tests of the popup's Settings page as it is drawn: its entries, items,
//! texts and tooltips, its layout in the simulator at the popup's widths
//! and height limit, and what a click on each part of it sends.
//!
//! The plan comes from `settings::plan` for a settings fixture of
//! `puron_testkit::cfg`, or from the applet's state machine after the
//! event `Settings`. The layout tests draw `settings::view` in the iced
//! test simulator, find widgets by the IDs that `view` gives them, and
//! check their rectangles in logical pixels at scale 1. Building the
//! simulators counts toward each test's time limit.
#![allow(non_snake_case)]

mod common;

use std::path::Path;
use std::process::Command;

use common::*;
use cosmic::iced::mouse::{self, ScrollDelta};
use cosmic::iced::{Event as UiEvent, Point, Rectangle, Size, Vector};
use cosmic::widget::{container, Id};
use cosmic::Theme;
use iced_test::selector::{Candidate, Selector, Target};
use iced_test::Simulator;
use puron_applet::app::{Event, Page};
use puron_applet::popup::{Event as PopupEvent, Reply};
use puron_applet::settings::{self, Choice, Clicked, Edit, Entry, Input, Item, Kind, Plan};
use puron_applet::view::View;
use puron_config::Config;
use puron_testkit::cfg;

/// The time limit of each test, in seconds.
const LIMIT: u64 = 5;
/// The tolerance, in logical pixels, of the checks that compare edges.
const TOL: f32 = 0.5;
/// The popup's height limit, libcosmic's for `applet::popup_container`.
const HEIGHT: f32 = 1000.0;
/// The narrowest and the widest popup.
const NARROW: f32 = 360.0;
const WIDE: f32 = 472.0;

type Sim<'a> = Simulator<'a, Clicked, Theme, cosmic::Renderer>;

/// The Settings plan of the settings stored in fixture `name`, with no
/// typed text and a notice when `notice` is true.
fn plan_of(name: &str, notice: bool) -> Plan {
    puron_applet::start();
    let fx = cfg::build(name);
    settings::plan(&Input {
        values: Config::new(&fx.root).load().settings,
        typed: None,
        notice,
    })
}

/// The plan of the `good` fixture without a notice.
fn plan() -> Plan {
    plan_of("good", false)
}

/// The Settings plan that the applet's state machine draws after Start
/// and Settings on fixture `name`.
fn app_plan(name: &str) -> Plan {
    puron_applet::start();
    let fx = cfg::build(name);
    let mut a = app(&fx.root, &handle(0));
    a.handle(Event::Start);
    a.handle(Event::Settings);
    let v = View::of(&a);
    assert_eq!(v.page, Page::Settings);
    v.settings_page.expect("a Settings plan")
}

/// Every item of `plan`; item `n` is at index `n - 1`.
fn items(plan: &Plan) -> Vec<Item> {
    plan.items().into_iter().cloned().collect()
}

fn item(plan: &Plan, n: usize) -> Item {
    items(plan)[n - 1].clone()
}

fn tip(plan: &Plan, n: usize) -> Option<String> {
    item(plan, n).tooltip.map(|(_, t)| t)
}

/// What each item shows, as text: the number field's value, `on` or
/// `off` for a switch, or the text of the selected list entry.
fn shown(plan: &Plan) -> Vec<String> {
    items(plan)
        .iter()
        .map(|i| match &i.control {
            Kind::Number { value, .. } => value.to_string(),
            Kind::Switch(on) => if *on { "on" } else { "off" }.to_string(),
            Kind::List { items, shown } => items[*shown].clone(),
        })
        .collect()
}

/// What each item shows when nothing is stored, in item order.
const DEFAULTS: [&str; 20] = [
    "5",
    "on",
    "Match panel size",
    "Match dock size",
    "°C",
    "on",
    "on",
    "on",
    "on",
    "off",
    "on",
    "off",
    "off",
    "on",
    "off",
    "off",
    "on",
    "on",
    "on",
    "on",
];
/// What each item shows for the `good` fixture, in item order. Every value
/// differs from its default.
const GOOD: [&str; 20] = [
    "7", "off", "20 px", "12 px", "°F", "off", "off", "off", "off", "on", "off", "on", "on", "off",
    "on", "on", "off", "off", "off", "off",
];

/// The Settings plan of the `good` fixture with no notice: Back and the
/// separator first, no notice, then each item as heading, label, control
/// kind, key and tooltip ID. Expected: the dump equals the table
/// `c_applet_popup_28.tsv` line for line (`diff` exits 0), five headings
/// and 20 items, and nothing else.
#[test]
fn c_applet_popup_28() {
    puron_testkit::limit("C-applet.popup-28", LIMIT, || {
        let fx = cfg::build("good");
        let plan = plan();
        assert!(
            matches!(&plan.entries[0], Entry::Back { icon: "go-previous-symbolic", text } if text == "Back"),
            "first entry {:?}",
            plan.entries[0]
        );
        assert_eq!(plan.entries[1], Entry::Separator, "second entry");
        assert_eq!(plan.notice(), None, "no notice");
        let mut dump = String::new();
        for e in &plan.entries[2..] {
            let Entry::Section(s) = e else {
                panic!("not a section: {e:?}");
            };
            for i in &s.items {
                let kind = match i.control {
                    Kind::Number { .. } => "number",
                    Kind::Switch(_) => "switch",
                    Kind::List { .. } => "list",
                };
                let tip = i.tooltip.as_ref().map_or("none", |(id, _)| id);
                dump += &format!("{}\t{}\t{kind}\t{}\t{tip}\n", s.heading, i.label, i.key);
            }
        }
        let got = fx.tmp.join("plan.tsv");
        std::fs::write(&got, dump).unwrap();
        let want = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/c_applet_popup_28.tsv");
        let out = Command::new("diff")
            .arg(&want)
            .arg(&got)
            .output()
            .expect("run diff");
        assert!(
            out.status.success(),
            "diff exit {}:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout)
        );
        assert_eq!(plan.sections().len(), 5, "five headings");
        assert_eq!(items(&plan).len(), 20, "twenty items");
    });
}

/// Through the applet's state machine, with nothing stored every item
/// shows its default, and with the `good` fixture every item shows the
/// stored value.
#[test]
fn c_applet_popup_29() {
    puron_testkit::limit("C-applet.popup-29", LIMIT, || {
        assert_eq!(shown(&app_plan("empty-none")), DEFAULTS, "X-cfg-empty");
        assert_eq!(shown(&app_plan("good")), GOOD, "X-cfg-good");
    });
}

/// How many lines of the English translation file `i18n/en/puron.ftl`
/// match `pattern`, as `grep -c` prints it.
fn grep_count(pattern: &str) -> String {
    let out = Command::new("grep")
        .args(["-c", pattern, "i18n/en/puron.ftl"])
        .current_dir(puron_testkit::workspace_root())
        .output()
        .expect("run grep");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Item 10's tooltip reads "Show additional digits after decimal point",
/// and the English translation file has no line with the misspelling
/// "additionnal".
#[test]
fn c_applet_popup_30() {
    puron_testkit::limit("C-applet.popup-30", LIMIT, || {
        assert_eq!(
            tip(&plan(), 10).as_deref(),
            Some("Show additional digits after decimal point")
        );
        assert_eq!(grep_count("additionnal"), "0");
    });
}

/// The two text size lists (items 3 and 4) have the panel and the dock
/// tooltips. Of items 11 to 20, only the lm-sensors, liquidctl and
/// "lm-sensors (drives only)" switches (items 11, 12 and 14) have a
/// tooltip, with the expected text.
#[test]
fn c_applet_popup_31() {
    puron_testkit::limit("C-applet.popup-31", LIMIT, || {
        let plan = plan();
        let got: Vec<_> = [3, 4]
            .into_iter()
            .chain(11..=20)
            .map(|n| tip(&plan, n))
            .collect();
        let mut want = vec![
            Some("Text size when Puron is on a panel".to_string()),
            Some("Text size when Puron is on a dock".to_string()),
            Some("Read sensors from lm-sensors".to_string()),
            Some("Read sensors from liquidctl (requires v1.7.0 or later)".to_string()),
            None,
            Some("Read drive temperatures from lm-sensors".to_string()),
        ];
        want.extend(std::iter::repeat_n(None, 6));
        assert_eq!(got, want);
    });
}

/// Set in the child process of `c_applet_popup_32`.
const CHILD32: &str = "PURON_SETTINGS_PAGE_EO_CHILD";

/// Under a test catalog that prefixes every English text with "EO ",
/// "Back", the notice, the headings, labels, tooltips and list entries
/// are translated, while the five product names (items 11 to 13, 15 and
/// 16) stay as they are. Item 14, "lm-sensors (drives only)", holds a
/// word and is translated. The text size lists show the translated match
/// texts and each number joined to the translated unit. The check runs
/// in a child process with `LANG=eo.UTF-8` and the other locale variables
/// removed.
#[test]
fn c_applet_popup_32() {
    /// The items whose label is a product name.
    const PRODUCT_ITEMS: [usize; 5] = [11, 12, 13, 15, 16];
    puron_testkit::limit("C-applet.popup-32", LIMIT, || {
        if std::env::var_os(CHILD32).is_none() {
            let mut cmd = puron_testkit::rerun_self("c_applet_popup_32");
            for v in puron_testkit::loc::VARS {
                cmd.env_remove(v);
            }
            let out = cmd
                .env("LANG", "eo.UTF-8")
                .env(CHILD32, "1")
                .output()
                .expect("run child");
            assert!(
                out.status.success(),
                "child: {}\n{}{}",
                out.status,
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            return;
        }
        use puron_applet::i18n;
        i18n::start_with(i18n::test_catalog::eo(), |k| std::env::var_os(k));
        let fx = cfg::build("good");
        let plan = settings::plan(&Input {
            values: Config::new(&fx.root).load().settings,
            typed: None,
            notice: true,
        });
        let en = puron_testkit::loc::english();
        let english = |t: &str| -> bool { en.iter().any(|(_, e)| e == t) };
        let eo = |t: &str| t.strip_prefix("EO ").is_some_and(english);
        match &plan.entries[0] {
            Entry::Back { text, .. } => assert!(eo(text), "Back {text:?}"),
            e => panic!("first entry {e:?}"),
        }
        let notice = plan.notice().expect("the notice");
        assert!(eo(notice), "notice {notice:?}");
        let products = ["lm-sensors", "liquidctl", "Nvidia", "Udisks2", "nvme-cli"];
        for s in plan.sections() {
            assert!(eo(&s.heading), "heading {:?}", s.heading);
            for i in &s.items {
                if products.contains(&i.label.as_str()) {
                    assert!(PRODUCT_ITEMS
                        .iter()
                        .any(|&n| item(&plan, n).label == i.label));
                } else {
                    assert!(eo(&i.label), "label {:?}", i.label);
                }
                if let Some((id, t)) = &i.tooltip {
                    assert!(eo(t), "tooltip {id} {t:?}");
                }
                if let Kind::List { items, .. } = &i.control {
                    if i.key == "temperature_unit" {
                        assert_eq!(items, &["EO °C", "EO °F"], "list items");
                    }
                }
            }
        }
        let labels: Vec<_> = PRODUCT_ITEMS.map(|n| item(&plan, n).label).to_vec();
        assert_eq!(labels, products, "product names unchanged");
        assert_eq!(item(&plan, 14).label, "EO lm-sensors (drives only)");
        let sizes = ["10", "12", "14", "16", "18", "20", "24"].map(|n| format!("{n}EO  px"));
        for (n, first) in [(3, "EO Match panel size"), (4, "EO Match dock size")] {
            let Kind::List { items, .. } = item(&plan, n).control else {
                panic!("item {n} is not a list");
            };
            assert_eq!(items[0], first, "item {n} first entry");
            assert_eq!(items[1..], sizes, "item {n} fixed sizes");
        }
    });
}

/// A simulator that draws the page of `plan` in a popup `width` wide and
/// as high as the popup's height limit.
fn sim(plan: &Plan, width: f32) -> Sim<'_> {
    let content = container(settings::view(plan)).id(Id::new("root"));
    Simulator::with_size(Default::default(), Size::new(width, HEIGHT), content)
}

/// Selects the widget with a given ID. A scroll area is visited twice:
/// first as a `PreOperation` candidate before its children, then as its
/// real target after them (at the iced revision this workspace pins). The
/// selector ignores every `PreOperation`, so that `scroll` gets the real
/// target with the scroll translation.
struct ById(Id);

impl Selector for ById {
    type Output = Target;

    fn select(&mut self, candidate: Candidate<'_>) -> Option<Target> {
        match candidate {
            Candidate::PreOperation { .. } => None,
            c if c.id() == Some(&self.0) => Some(Target::from(c)),
            _ => None,
        }
    }

    fn description(&self) -> String {
        format!("id == {:?}", self.0)
    }
}

fn target(sim: &mut Sim, name: &str) -> Target {
    sim.find(ById(Id::new(name.to_string())))
        .unwrap_or_else(|e| panic!("find {name}: {e:?}"))
}

/// The rectangle of the widget with ID `name`.
fn rect(sim: &mut Sim, name: &str) -> Rectangle {
    target(sim, name).bounds()
}

/// The scroll area's rectangle and its current scroll translation.
fn scroll(sim: &mut Sim) -> (Rectangle, Vector) {
    match target(sim, "scroll") {
        Target::Scrollable {
            bounds,
            translation,
            ..
        } => (bounds, translation),
        t => panic!("scroll is not a scroll area: {t:?}"),
    }
}

/// Scrolls to the end with one wheel event far larger than the content;
/// the scroll area stops at its last position. Returns the translation
/// there.
fn scroll_to_end(sim: &mut Sim) -> Vector {
    let (area, _) = scroll(sim);
    sim.point_at(area.center());
    sim.simulate([UiEvent::Mouse(mouse::Event::WheelScrolled {
        delta: ScrollDelta::Pixels { x: 0.0, y: -1.0e6 },
    })]);
    scroll(sim).1
}

/// Selects the text widget whose content is `text`, after skipping the
/// first `skip` such widgets in depth-first order; the same label can
/// occur twice. It is used instead of iced's `&str` selector because that
/// one also asks text inputs for their text, and libcosmic's
/// `TextInput::text` panics (`todo!()`) at the revision this workspace
/// pins.
struct Nth {
    text: String,
    skip: usize,
}

impl Selector for Nth {
    type Output = Rectangle;

    fn select(&mut self, candidate: Candidate<'_>) -> Option<Rectangle> {
        match candidate {
            Candidate::Text {
                bounds, content, ..
            } if content == self.text => {
                if self.skip == 0 {
                    Some(bounds)
                } else {
                    self.skip -= 1;
                    None
                }
            }
            _ => None,
        }
    }

    fn description(&self) -> String {
        format!("text #{} == {:?}", self.skip + 1, self.text)
    }
}

/// The texts that come before item `n`'s label in the page, in order:
/// "Back", the notice, each heading and each earlier label.
fn texts_before(plan: &Plan, n: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut k = 0;
    for e in &plan.entries {
        match e {
            Entry::Back { text, .. } => out.push(text.clone()),
            Entry::Notice(t) => out.push(t.clone()),
            Entry::Separator => {}
            Entry::Section(s) => {
                out.push(s.heading.clone());
                for i in &s.items {
                    k += 1;
                    if k == n {
                        return out;
                    }
                    out.push(i.label.clone());
                }
            }
        }
    }
    out
}

/// The rectangle of item `n`'s label text. When an earlier text of the
/// page has the same content, the matching text is counted past it.
fn label(sim: &mut Sim, plan: &Plan, n: usize) -> Rectangle {
    let text = item(plan, n).label;
    let skip = texts_before(plan, n).iter().filter(|t| **t == text).count();
    sim.find(Nth { text, skip })
        .unwrap_or_else(|e| panic!("label of item {n}: {e:?}"))
}

/// The rectangle of the heading of section `s` (counted from 0), found as
/// the first text with that content.
fn heading(sim: &mut Sim, plan: &Plan, s: usize) -> Rectangle {
    let text = plan.sections()[s].heading.clone();
    sim.find(Nth {
        text: text.clone(),
        skip: 0,
    })
    .unwrap_or_else(|e| panic!("heading {text}: {e:?}"))
}

fn item_rect(sim: &mut Sim, n: usize) -> Rectangle {
    rect(sim, &format!("item-{n}"))
}

fn control(sim: &mut Sim, n: usize) -> Rectangle {
    rect(sim, &format!("control-{n}"))
}

/// The rectangle of section `s` (counted from 1).
fn section(sim: &mut Sim, s: usize) -> Rectangle {
    rect(sim, &format!("section-{s}"))
}

fn near(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn mid_y(r: Rectangle) -> f32 {
    r.y + r.height / 2.0
}

/// Whether `r` lies within `area`, give or take `TOL`.
fn inside(r: Rectangle, area: Rectangle) -> bool {
    r.x >= area.x - TOL
        && r.y >= area.y - TOL
        && r.x + r.width <= area.x + area.width + TOL
        && r.y + r.height <= area.y + area.height + TOL
}

/// Where `r`, a layout rectangle inside the scroll area, is drawn once
/// the content is scrolled by `t`.
fn shifted(r: Rectangle, t: Vector) -> Rectangle {
    Rectangle {
        x: r.x - t.x,
        y: r.y - t.y,
        ..r
    }
}

/// At the widest popup, scrolled to the top and to the end, the sections
/// do not overlap: each one's top edge is at or below the bottom edge of
/// the one before. "Back" ends above the first section.
#[test]
fn c_applet_popup_33() {
    puron_testkit::limit("C-applet.popup-33", LIMIT, || {
        let plan = plan();
        let mut sim = sim(&plan, WIDE);
        let back = rect(&mut sim, "back");
        let first = section(&mut sim, 1);
        assert!(
            back.y + back.height <= first.y + TOL,
            "Back {back:?} over section 1 {first:?}"
        );
        for at_end in [false, true] {
            let t = if at_end {
                scroll_to_end(&mut sim)
            } else {
                scroll(&mut sim).1
            };
            let r: Vec<Rectangle> = (1..=5).map(|s| shifted(section(&mut sim, s), t)).collect();
            for s in 1..5 {
                assert!(
                    r[s].y >= r[s - 1].y + r[s - 1].height,
                    "end {at_end}: section {} top {} above section {} bottom {}",
                    s + 1,
                    r[s].y,
                    s,
                    r[s - 1].y + r[s - 1].height
                );
            }
        }
    });
}

/// In each section the heading is above the first item, each item is
/// below the one before, and there are 20 items in all.
#[test]
fn c_applet_popup_34() {
    puron_testkit::limit("C-applet.popup-34", LIMIT, || {
        let plan = plan();
        let mut sim = sim(&plan, WIDE);
        let mut n = 0;
        for (s, sec) in plan.sections().iter().enumerate() {
            let h = heading(&mut sim, &plan, s);
            let mut above = h.y + h.height;
            for _ in &sec.items {
                n += 1;
                let r = item_rect(&mut sim, n);
                assert!(
                    r.y >= above,
                    "section {}: item {n} top {} above {above}",
                    s + 1,
                    r.y
                );
                above = r.y + r.height;
            }
        }
        assert_eq!(n, 20, "items");
    });
}

/// The sections form one column: at the narrowest and the widest popup,
/// all five share the same left and right edges.
#[test]
fn c_applet_popup_35() {
    puron_testkit::limit("C-applet.popup-35", LIMIT, || {
        let plan = plan();
        for width in [NARROW, WIDE] {
            let mut sim = sim(&plan, width);
            let first = section(&mut sim, 1);
            for s in 2..=5 {
                let r = section(&mut sim, s);
                assert!(
                    near(r.x, first.x, TOL) && near(r.x + r.width, first.x + first.width, TOL),
                    "width {width}: section {s} {r:?} against section 1 {first:?}"
                );
            }
        }
    });
}

/// Each label starts at the left edge of its item row.
#[test]
fn c_applet_popup_36() {
    puron_testkit::limit("C-applet.popup-36", LIMIT, || {
        let plan = plan();
        let mut sim = sim(&plan, WIDE);
        for n in 1..=20 {
            let (l, i) = (label(&mut sim, &plan, n), item_rect(&mut sim, n));
            assert!(near(l.x, i.x, TOL), "item {n}: label {l:?} item {i:?}");
        }
    });
}

/// Each control ends at the right edge of its item row, and every item
/// row ends at the same right edge, the section's: a control placed after
/// its label with a fixed gap would end where the label's text ends.
#[test]
fn c_applet_popup_37() {
    puron_testkit::limit("C-applet.popup-37", LIMIT, || {
        let plan = plan();
        let mut sim = sim(&plan, WIDE);
        let first = item_rect(&mut sim, 1);
        for n in 1..=20 {
            let (c, i) = (control(&mut sim, n), item_rect(&mut sim, n));
            assert!(
                near(c.x + c.width, i.x + i.width, TOL),
                "item {n}: control {c:?} item {i:?}"
            );
            assert!(
                near(i.x + i.width, first.x + first.width, TOL),
                "item {n}: right edge {i:?} against item 1 {first:?}"
            );
        }
    });
}

/// Each item's label and control are centered vertically on the item
/// row, within one pixel.
#[test]
fn c_applet_popup_38() {
    puron_testkit::limit("C-applet.popup-38", LIMIT, || {
        let plan = plan();
        let mut sim = sim(&plan, WIDE);
        for n in 1..=20 {
            let i = item_rect(&mut sim, n);
            let l = label(&mut sim, &plan, n);
            let c = control(&mut sim, n);
            assert!(
                near(mid_y(l), mid_y(i), 1.0),
                "item {n}: label {l:?} item {i:?}"
            );
            assert!(
                near(mid_y(c), mid_y(i), 1.0),
                "item {n}: control {c:?} item {i:?}"
            );
        }
    });
}

/// In the narrowest popup every label still ends before its control,
/// every control ends inside the scroll area, and every control keeps the
/// width it has in the widest popup: the label gives way, not the control.
#[test]
fn c_applet_popup_39() {
    puron_testkit::limit("C-applet.popup-39", LIMIT, || {
        let plan = plan();
        let mut wide = sim(&plan, WIDE);
        let w472: Vec<f32> = (1..=20).map(|n| control(&mut wide, n).width).collect();
        let mut sim = sim(&plan, NARROW);
        let (area, _) = scroll(&mut sim);
        for n in 1..=20 {
            let l = label(&mut sim, &plan, n);
            let c = control(&mut sim, n);
            assert!(
                l.x + l.width <= c.x + TOL,
                "item {n}: label {l:?} over control {c:?}"
            );
            assert!(
                c.x + c.width <= area.x + area.width + TOL,
                "item {n}: control {c:?} past the scroll area {area:?}"
            );
            assert!(
                near(c.width, w472[n - 1], TOL),
                "item {n}: control width {} at 360, {} at 472",
                c.width,
                w472[n - 1]
            );
        }
    });
}

/// The number field shows its − button, text input and + button from left
/// to right, centered on one line, with a text input 48 pixels wide; the
/// buttons use the expected icon names.
#[test]
fn c_applet_popup_40() {
    puron_testkit::limit("C-applet.popup-40", LIMIT, || {
        assert_eq!(settings::MINUS_ICON, "list-remove-symbolic", "− icon");
        assert_eq!(settings::PLUS_ICON, "list-add-symbolic", "+ icon");
        let plan = plan();
        let mut sim = sim(&plan, WIDE);
        let r = [
            rect(&mut sim, "minus"),
            rect(&mut sim, "input"),
            rect(&mut sim, "plus"),
        ];
        for k in 1..3 {
            assert!(
                r[k].x >= r[k - 1].x + r[k - 1].width,
                "order: {:?} before {:?}",
                r[k - 1],
                r[k]
            );
            assert!(near(mid_y(r[k]), mid_y(r[0]), 1.0), "centers: {r:?}");
        }
        assert!(near(r[1].width, 48.0, TOL), "input width {}", r[1].width);
    });
}

/// The page is taller than the popup's height limit, so the popup takes
/// the limit and the page scrolls inside it: at the top "Back" and the
/// first heading lie inside the scroll area, and scrolled to the end, the
/// last item does.
#[test]
fn c_applet_popup_41() {
    puron_testkit::limit("C-applet.popup-41", LIMIT, || {
        let plan = plan();
        let mut top = sim(&plan, WIDE);
        let root = rect(&mut top, "root");
        assert!(near(root.height, HEIGHT, TOL), "popup height {root:?}");
        let (area, t) = scroll(&mut top);
        let last = item_rect(&mut top, 20);
        assert!(
            last.y + last.height > area.y + area.height + TOL,
            "content not taller than the popup: item 20 {last:?} area {area:?}"
        );
        let back = shifted(rect(&mut top, "back"), t);
        assert!(inside(back, area), "top: Back {back:?} area {area:?}");
        let h = shifted(heading(&mut top, &plan, 0), t);
        assert!(inside(h, area), "top: heading {h:?} area {area:?}");
        let mut end = sim(&plan, WIDE);
        let t = scroll_to_end(&mut end);
        let (area, _) = scroll(&mut end);
        let last = shifted(item_rect(&mut end, 20), t);
        assert!(
            inside(last, area),
            "end: item 20 {last:?} t {t:?} area {area:?}"
        );
    });
}

/// A snapshot of `sim` in the dark theme, as (width, height, RGBA bytes)
/// at scale 1. The snapshot goes through a PNG file in a temporary
/// directory, named after `tag`, which is removed afterwards.
fn pixels(sim: &mut Sim, tag: &str) -> (u32, u32, Vec<u8>) {
    let snap = sim.snapshot(&Theme::dark()).expect("snapshot");
    let dir = std::env::temp_dir().join(format!("puron-sp42-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    assert!(snap
        .matches_image(dir.join("snap.png"))
        .expect("write snapshot"));
    let file = std::fs::File::open(dir.join("snap-tiny-skia.png")).expect("snapshot png");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("png header");
    let mut rgba = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut rgba).expect("png frame");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    let _ = std::fs::remove_dir_all(&dir);
    (info.width, info.height, rgba)
}

/// The number of pixels that differ between `a` and `b`, leaving out
/// every pixel that touches `skip`.
fn changed(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>), skip: Option<Rectangle>) -> usize {
    assert_eq!((a.0, a.1), (b.0, b.1), "snapshot sizes");
    let mut n = 0;
    for y in 0..a.1 {
        for x in 0..a.0 {
            let (fx, fy) = (x as f32, y as f32);
            if let Some(r) = skip {
                if fx + 1.0 > r.x && fx < r.x + r.width && fy + 1.0 > r.y && fy < r.y + r.height {
                    continue;
                }
            }
            let i = ((y * a.0 + x) * 4) as usize;
            if a.2[i..i + 4] != b.2[i..i + 4] {
                n += 1;
            }
        }
    }
    n
}

/// Hovers the point `at` in a fresh simulator, by placing the cursor
/// there and sending one cursor move, and returns the snapshot.
fn hovered(plan: &Plan, at: Point, tag: &str) -> (u32, u32, Vec<u8>) {
    let mut sim = sim(plan, WIDE);
    sim.point_at(at);
    sim.simulate([UiEvent::Mouse(mouse::Event::CursorMoved { position: at })]);
    pixels(&mut sim, tag)
}

/// The tooltip belongs to the whole item row. Hovering item 10's label, or
/// its switch, changes pixels outside the switch, where the tooltip is
/// drawn. Item 13 has no tooltip, so hovering its label changes no pixel.
#[test]
fn c_applet_popup_42() {
    puron_testkit::limit("C-applet.popup-42", LIMIT, || {
        let plan = plan();
        let mut first = sim(&plan, WIDE);
        let base = pixels(&mut first, "base");
        let label10 = label(&mut first, &plan, 10).center();
        let control10 = control(&mut first, 10);
        let label13 = label(&mut first, &plan, 13).center();
        for (what, at) in [("label", label10), ("switch", control10.center())] {
            let after = hovered(&plan, at, what);
            let n = changed(&base, &after, Some(control10));
            assert!(
                n > 0,
                "item 10 {what} hover at {at:?}: no pixel changed outside {control10:?}"
            );
        }
        let after = hovered(&plan, label13, "item13");
        let n = changed(&base, &after, None);
        assert_eq!(n, 0, "item 13 hover at {label13:?}: {n} pixels changed");
    });
}

/// The messages that a press and release of the left mouse button at
/// `at` send, in a fresh simulator of `plan`.
fn click_at(plan: &Plan, at: Point) -> Vec<Clicked> {
    let mut sim = sim(plan, WIDE);
    sim.point_at(at);
    sim.simulate([
        UiEvent::Mouse(mouse::Event::CursorMoved { position: at }),
        UiEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        UiEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]);
    sim.into_messages().collect()
}

/// Through the simulator, on the page of the `good` fixture with the
/// notice shown: a click on "Back" sends Back, which returns to the
/// temperatures with the popup open; a click on item 10's switch, which is
/// on, sends its change to off, and a click on + sends Plus, each with the
/// popup open. A click on each section heading, the separator or the
/// notice sends nothing: seven such entries.
#[test]
fn c_applet_popup_43() {
    puron_testkit::limit("C-applet.popup-43", LIMIT, || {
        let plan = plan_of("good", true);
        let open = |event| Reply {
            event: Some(event),
            close: false,
        };
        let mut sim0 = sim(&plan, WIDE);
        let back = rect(&mut sim0, "back").center();
        let switch10 = control(&mut sim0, 10).center();
        let plus = rect(&mut sim0, "plus").center();
        let mut quiet = vec![
            ("separator", rect(&mut sim0, "separator").center()),
            ("notice", rect(&mut sim0, "notice").center()),
        ];
        for s in 0..5 {
            quiet.push(("heading", heading(&mut sim0, &plan, s).center()));
        }
        drop(sim0);

        let got = click_at(&plan, back);
        assert_eq!(got, [Clicked::Back], "Back");
        assert_eq!(
            settings::clicked(&plan, Clicked::Back),
            open(PopupEvent::Back)
        );

        let change = Edit::Change {
            item: 10,
            choice: Choice::Switch(false),
        };
        let got = click_at(&plan, switch10);
        assert_eq!(got, [Clicked::Edit(change.clone())], "item 10's switch");
        assert_eq!(
            settings::clicked(&plan, Clicked::Edit(change.clone())),
            open(PopupEvent::Edit(change))
        );

        let got = click_at(&plan, plus);
        assert_eq!(got, [Clicked::Edit(Edit::Plus)], "+");
        assert_eq!(
            settings::clicked(&plan, Clicked::Edit(Edit::Plus)),
            open(PopupEvent::Edit(Edit::Plus))
        );

        assert!(quiet.len() >= 7, "entries that answer no click");
        for (what, at) in quiet {
            let got = click_at(&plan, at);
            assert!(got.is_empty(), "{what} at {at:?}: {got:?}");
        }
    });
}
