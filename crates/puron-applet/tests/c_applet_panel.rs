//! Tests of the panel button, each with a time limit of 5 s. Some check
//! the panel plan alone; others lay out the widgets built from the plan
//! in iced's headless test simulator and measure them. Unless a test says
//! otherwise, the panel is of type Panel, at size M, and the text size is
//! the default, `TextSize::MatchPanel`.
//! Unless a test says otherwise, the items come from
//! `puron_core::panel::panel` with the captured readings and the pinned
//! list `P1` of `pins.json`. The icon theme test runs in a fresh run of
//! this test binary whose `XDG_DATA_DIRS` holds the project's icon files
//! in a temporary hicolor tree.
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cosmic::applet::cosmic_panel_config::PanelSize;
use cosmic::applet::{Context, PanelType, Size as AppletSize};
use cosmic::iced::theme::Base;
use cosmic::iced::{Length, Rectangle, Size};
use cosmic::widget::{container, Id};
use cosmic::Theme;
use iced_test::{selector, Simulator};
use puron_applet::panel::{self, Element, Orientation, Place, Plan};
use puron_config::Config;
use puron_core::menu;
use puron_core::panel::{Panel, State};
use puron_core::rows::RawReading;
use puron_core::settings::{Settings, TextSize};
use puron_testkit::cfg;

const LIMIT: u64 = 5;
const CHILD: &str = "PURON_PANEL_CHILD";
/// The button's size across the panel in px: the height of a horizontal
/// button, the width of a vertical one.
const BUTTON: f32 = 48.0;
/// The simulator window's size along the panel in px.
const LONG: f32 = 600.0;
/// The scale factor at which a snapshot is rendered.
const SCALE: f32 = 2.0;

const FAN: &str = "puron-fan-symbolic";
const TEMP: &str = "puron-temperature-symbolic";
const GPU: &str = "puron-gpu-temperature-symbolic";
const DRIVE: &str = "drive-harddisk-system-symbolic";
const VOLT: &str = "puron-voltage-symbolic";

fn raw(name: &str) -> String {
    std::fs::read_to_string(puron_testkit::fixture("raw", name)).expect("read fixture")
}

fn capture() -> Vec<RawReading> {
    serde_json::from_str(&raw("capture.json")).expect("parse capture.json")
}

fn pinned(name: &str) -> Vec<String> {
    serde_json::from_str::<BTreeMap<String, Vec<String>>>(&raw("pins.json"))
        .expect("parse pins.json")
        .remove(name)
        .expect("X-raw-pins set")
}

fn settings(icons: bool) -> Settings {
    Settings {
        show_panel_icons: icons,
        ..Settings::default()
    }
}

fn core_panel(raw: &[RawReading], pins: &[String], icons: bool) -> Panel {
    let s = settings(icons);
    puron_core::panel::panel(&menu::shown(raw, &s), pins, &s, &State::default())
}

/// The icon size in px that libcosmic gives for panel size M.
fn m_icon_size() -> f32 {
    PanelSize::M.get_applet_icon_size(true) as f32
}

/// Where the button is drawn in most tests: panel size M, on a panel of
/// type Panel, in orientation `o`.
fn m_place(o: Orientation) -> Place {
    place(PanelSize::M, PanelType::Panel, o)
}

/// The place that libcosmic's context gives for a panel of `size` and
/// `panel_type`, in orientation `o`. Size and type are set here, so the
/// environment cannot change them.
fn place(size: PanelSize, panel_type: PanelType, o: Orientation) -> Place {
    Place {
        orientation: o,
        ..Place::of(&Context {
            size: AppletSize::PanelSize(size),
            panel_type,
            ..Context::default()
        })
    }
}

fn plan_of(p: &Panel, o: Orientation) -> Plan {
    panel::plan(p, &m_place(o), &Settings::default())
}

fn p1_plan(icons: bool, o: Orientation) -> Plan {
    plan_of(&core_panel(&capture(), &pinned("P1"), icons), o)
}

/// The plan's elements, one string each: the item index, then
/// `icon <name>`, `slot` or `text <text>`.
fn kinds(plan: &Plan) -> Vec<String> {
    plan.elements
        .iter()
        .map(|e| match e {
            Element::Icon { item, name } => format!("{item} icon {name}"),
            Element::Slot { item } => format!("{item} slot"),
            Element::Text { item, text } => format!("{item} text {text}"),
        })
        .collect()
}

fn names(plan: &Plan) -> Vec<&'static str> {
    plan.elements
        .iter()
        .filter_map(|e| match e {
            Element::Icon { name, .. } => Some(*name),
            _ => None,
        })
        .collect()
}

type Sim = Simulator<'static, (), Theme, cosmic::Renderer>;

/// A simulator holding the button: the plan's content in a container
/// with the ID `root`, `BUTTON` px across the panel, in a window `LONG` px
/// along it.
fn simulate(plan: &Plan) -> Sim {
    let content = container(panel::view::<()>(plan)).id(Id::new("root"));
    let (button, window) = match plan.orientation {
        Orientation::Horizontal => (
            content.height(Length::Fixed(BUTTON)),
            Size::new(LONG, BUTTON),
        ),
        Orientation::Vertical => (
            content.width(Length::Fixed(BUTTON)),
            Size::new(BUTTON, LONG),
        ),
    };
    Simulator::with_size(Default::default(), window, button)
}

fn bounds(sim: &mut Sim, id: Id) -> Rectangle {
    sim.find(selector::id(id.clone()))
        .unwrap_or_else(|e| panic!("find {id:?}: {e:?}"))
        .bounds()
}

/// The laid-out rectangles of the plan's elements, in plan order.
fn rects(sim: &mut Sim, plan: &Plan) -> Vec<Rectangle> {
    (0..plan.elements.len())
        .map(|i| bounds(sim, panel::element_id(i)))
        .collect()
}

fn near(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn is_text(e: &Element) -> bool {
    matches!(e, Element::Text { .. })
}

/// A rendered snapshot's pixels, as RGBA rows at `SCALE`; `at` drops the
/// alpha.
struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Pixels {
    fn at(&self, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]]
    }

    /// The pixels inside a layout rectangle, whose edges are scaled and
    /// rounded inward.
    fn inside(&self, r: &Rectangle) -> Vec<[u8; 3]> {
        let x0 = (r.x * SCALE).ceil() as u32;
        let y0 = (r.y * SCALE).ceil() as u32;
        let x1 = (((r.x + r.width) * SCALE).floor() as u32).min(self.width);
        let y1 = (((r.y + r.height) * SCALE).floor() as u32).min(self.height);
        let mut v = Vec::new();
        for y in y0..y1 {
            for x in x0..x1 {
                v.push(self.at(x, y));
            }
        }
        v
    }
}

fn rgb8(c: cosmic::iced::Color) -> [u8; 3] {
    let q = |v: f32| (v * 255.0).round() as u8;
    [q(c.r), q(c.g), q(c.b)]
}

/// Renders the button with `Theme::dark()` and returns its pixels with the
/// theme's background and text colours. It fails unless the pixel at
/// (0, 0) has the background colour.
fn snapshot(sim: &mut Sim, test: &str) -> (Pixels, [u8; 3], [u8; 3]) {
    let theme = Theme::dark();
    let snap = sim.snapshot(&theme).expect("snapshot");
    let dir = scratch(test);
    let stem = dir.join("snap.png");
    assert!(snap.matches_image(&stem).expect("write snapshot"));
    let file = std::fs::File::open(dir.join("snap-tiny-skia.png")).expect("snapshot png");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("png header");
    let mut rgba = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut rgba).expect("png frame");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    let _ = std::fs::remove_dir_all(&dir);
    let px = Pixels {
        width: info.width,
        height: info.height,
        rgba,
    };
    let base = theme.base();
    let (bg, fg) = (rgb8(base.background_color), rgb8(base.text_color));
    assert_eq!(px.at(0, 0), bg, "background pixel");
    (px, bg, fg)
}

fn scratch(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("puron-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

#[test]
fn c_applet_panel_1() {
    puron_testkit::limit("C-applet.panel-1", LIMIT, || {
        let core = core_panel(&capture(), &pinned("P1"), true);
        let plan = plan_of(&core, Orientation::Horizontal);
        let texts: Vec<String> = core
            .items
            .iter()
            .map(|i| match i {
                puron_core::panel::Item::Pin { value, .. } => {
                    puron_applet::i18n::panel_value(value.as_ref().map(|v| v.as_ref()))
                }
                puron_core::panel::Item::EmptyState => panic!("pin item"),
            })
            .collect();
        let want: Vec<String> = [TEMP, DRIVE, TEMP, FAN]
            .iter()
            .zip(&texts)
            .enumerate()
            .flat_map(|(i, (n, t))| [format!("{i} icon {n}"), format!("{i} text {t}")])
            .collect();
        assert_eq!(kinds(&plan), want);
        let mut sim = simulate(&plan);
        let r = rects(&mut sim, &plan);
        for i in 1..r.len() {
            assert!(
                r[i].x >= r[i - 1].x + r[i - 1].width,
                "element {i} left {} before {} right {}",
                r[i].x,
                i - 1,
                r[i - 1].x + r[i - 1].width
            );
        }
    });
}

#[test]
fn c_applet_panel_2() {
    puron_testkit::limit("C-applet.panel-2", LIMIT, || {
        let plan = p1_plan(true, Orientation::Horizontal);
        let mut sim = simulate(&plan);
        let r = rects(&mut sim, &plan);
        let mut n = 0;
        for i in 1..r.len() {
            if is_text(&plan.elements[i - 1]) && !is_text(&plan.elements[i]) {
                let gap = r[i].x - (r[i - 1].x + r[i - 1].width);
                assert!(near(gap, 9.0, 0.5), "text {} to icon {i}: {gap}", i - 1);
                n += 1;
            }
        }
        assert_eq!(n, 3, "three text-to-icon gaps in P1");
    });
}

#[test]
fn c_applet_panel_3() {
    puron_testkit::limit("C-applet.panel-3", LIMIT, || {
        let on = p1_plan(true, Orientation::Horizontal);
        assert_eq!(on.padding, 0);
        assert_eq!(on.icon_padding, 9.0);
        let mut sim = simulate(&on);
        let r = rects(&mut sim, &on);
        for i in (0..r.len()).step_by(2) {
            let gap = r[i + 1].x - (r[i].x + r[i].width);
            assert!(near(gap, 9.0, 0.5), "icon {i} to its text: {gap}");
        }
        let off = p1_plan(false, Orientation::Horizontal);
        assert_eq!(off.padding, 4);
        assert!(off.elements.iter().all(is_text));
        let mut sim = simulate(&off);
        let root = bounds(&mut sim, Id::new("root"));
        let r = rects(&mut sim, &off);
        let first = r[0].x - root.x;
        assert!(
            near(first, 4.0, 0.5),
            "first text from the content's left: {first}"
        );
        for i in 1..r.len() {
            let gap = r[i].x - (r[i - 1].x + r[i - 1].width);
            assert!(near(gap, 8.0, 0.5), "texts {} and {i}: {gap}", i - 1);
        }
    });
}

#[test]
fn c_applet_panel_4() {
    puron_testkit::limit("C-applet.panel-4", LIMIT, || {
        let plan = p1_plan(true, Orientation::Horizontal);
        let mut sim = simulate(&plan);
        let root = bounds(&mut sim, Id::new("root"));
        assert_eq!(root.height, BUTTON);
        let centre = root.y + root.height / 2.0;
        let r = rects(&mut sim, &plan);
        let mut n = 0;
        for (i, e) in plan.elements.iter().enumerate() {
            if is_text(e) {
                let c = r[i].y + r[i].height / 2.0;
                assert!(near(c, centre, 1.0), "text {i} centre {c}, button {centre}");
                n += 1;
            }
        }
        assert_eq!(n, 4, "four texts in P1");
    });
}

#[test]
fn c_applet_panel_5() {
    puron_testkit::limit("C-applet.panel-5", LIMIT, || {
        let h = p1_plan(true, Orientation::Horizontal);
        let v = p1_plan(true, Orientation::Vertical);
        assert_eq!(v.orientation, Orientation::Vertical);
        assert_eq!(kinds(&v), kinds(&h));
        assert_eq!(v.elements.len(), 8);
        let mut sim = simulate(&v);
        let r = rects(&mut sim, &v);
        for i in 1..r.len() {
            assert!(
                r[i].y >= r[i - 1].y + r[i - 1].height,
                "element {i} top {} above {} bottom {}",
                r[i].y,
                i - 1,
                r[i - 1].y + r[i - 1].height
            );
        }
    });
}

#[test]
fn c_applet_panel_6() {
    puron_testkit::limit("C-applet.panel-6", LIMIT, || {
        let plan = p1_plan(true, Orientation::Vertical);
        let mut sim = simulate(&plan);
        let root = bounds(&mut sim, Id::new("root"));
        assert_eq!(root.width, BUTTON);
        let centre = root.x + root.width / 2.0;
        let r = rects(&mut sim, &plan);
        assert_eq!(r.len(), 8, "eight elements in P1");
        for (i, r) in r.iter().enumerate() {
            let c = r.x + r.width / 2.0;
            assert!(
                near(c, centre, 1.0),
                "element {i} centre {c}, button {centre}"
            );
        }
    });
}

#[test]
fn c_applet_panel_7() {
    puron_testkit::limit("C-applet.panel-7", LIMIT, || {
        let plan = plan_of(
            &core_panel(&[], &pinned("P5"), true),
            Orientation::Horizontal,
        );
        assert_eq!(kinds(&plan), ["0 slot", "0 text ⚠", "1 slot", "1 text ⚠"]);
        assert_eq!(puron_applet::i18n::text("missing-value"), "⚠");
        let mut sim = simulate(&plan);
        let r = rects(&mut sim, &plan);
        let size = m_icon_size();
        let (px, bg, _) = snapshot(&mut sim, "c_applet_panel_7");
        for i in [0, 2] {
            assert!(near(r[i].width, size, 0.5), "slot {i} width {}", r[i].width);
            assert!(
                near(r[i].height, size, 0.5),
                "slot {i} height {}",
                r[i].height
            );
            let p = px.inside(&r[i]);
            assert!(!p.is_empty());
            let other = p.iter().filter(|c| **c != bg).count();
            assert_eq!(other, 0, "slot {i}: pixels that are not background");
        }
    });
}

#[test]
fn c_applet_panel_8() {
    puron_testkit::limit("C-applet.panel-8", LIMIT, || {
        let plan = p1_plan(true, Orientation::Horizontal);
        let mut sim = simulate(&plan);
        let r = rects(&mut sim, &plan);
        let size = m_icon_size();
        let mut n = 0;
        for (i, e) in plan.elements.iter().enumerate() {
            if let Element::Icon { .. } = e {
                assert!(near(r[i].width, size, 0.5), "icon {i} width {}", r[i].width);
                assert!(
                    near(r[i].height, size, 0.5),
                    "icon {i} height {}",
                    r[i].height
                );
                n += 1;
            }
        }
        assert_eq!(n, 4);
    });
}

#[test]
fn c_applet_panel_9() {
    puron_testkit::limit("C-applet.panel-9", LIMIT, || {
        let k: serde_json::Value = serde_json::from_str(&raw("kinds.json")).expect("kinds.json");
        let readings: Vec<RawReading> = serde_json::from_value(k["R"].clone()).expect("R");
        let pins: Vec<String> = serde_json::from_value(k["P"].clone()).expect("P");
        let plan = plan_of(&core_panel(&readings, &pins, true), Orientation::Horizontal);
        assert_eq!(names(&plan), [GPU, DRIVE, TEMP, FAN, VOLT, VOLT]);
    });
}

/// Copies the project's four icon files into a hicolor tree under `share`
/// and writes a minimal `index.theme` for it, since the system may have
/// none for hicolor.
fn icon_tree(share: &Path) {
    let status = share.join("icons/hicolor/scalable/status");
    std::fs::create_dir_all(&status).expect("icon dir");
    let src =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/icons/hicolor/scalable/status");
    let mut n = 0;
    for f in std::fs::read_dir(&src).expect("data/icons") {
        let f = f.expect("entry").path();
        if f.extension().is_some_and(|x| x == "svg") {
            std::fs::copy(&f, status.join(f.file_name().unwrap())).expect("copy icon");
            n += 1;
        }
    }
    assert_eq!(n, 4, "Puron's four icon files");
    std::fs::write(
        share.join("icons/hicolor/index.theme"),
        "[Icon Theme]\nName=hicolor\nComment=Puron test\nDirectories=scalable/status\n\n\
         [scalable/status]\nSize=16\nMinSize=8\nMaxSize=512\nType=Scalable\nContext=Status\n",
    )
    .expect("index.theme");
}

#[test]
fn c_applet_panel_11() {
    puron_testkit::limit("C-applet.panel-11", LIMIT, || {
        if std::env::var_os(CHILD).is_none() {
            let dir = scratch("c_applet_panel_11-share");
            icon_tree(&dir);
            let out = puron_testkit::rerun_self("c_applet_panel_11")
                .env("XDG_DATA_DIRS", format!("{}:/usr/share", dir.display()))
                .env(CHILD, "1")
                .output()
                .expect("run child");
            let _ = std::fs::remove_dir_all(&dir);
            assert!(
                out.status.success(),
                "child: {}\n{}{}",
                out.status,
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            return;
        }
        let plan = p1_plan(true, Orientation::Horizontal);
        let mut sim = simulate(&plan);
        let r = rects(&mut sim, &plan);
        let (px, bg, fg) = snapshot(&mut sim, "c_applet_panel_11");
        let within = |a: u8, b: f32| (f32::from(a) - b).abs() <= 2.0;
        let mut n = 0;
        for (i, e) in plan.elements.iter().enumerate() {
            let Element::Icon { name, .. } = e else {
                continue;
            };
            let mut at_fg = 0;
            for p in px.inside(&r[i]) {
                if p == bg {
                    continue;
                }
                // How far the pixel lies from the background towards the
                // foreground, as t in [0, 1]: its projection onto the line
                // between the two colours. The pixel must then be within 2
                // of that blend in each channel.
                let d: Vec<f32> = (0..3)
                    .map(|c| f32::from(fg[c]) - f32::from(bg[c]))
                    .collect();
                let q: Vec<f32> = (0..3).map(|c| f32::from(p[c]) - f32::from(bg[c])).collect();
                let dd: f32 = d.iter().map(|x| x * x).sum();
                let t = ((0..3).map(|c| d[c] * q[c]).sum::<f32>() / dd).clamp(0.0, 1.0);
                let ok = (0..3).all(|c| within(p[c], f32::from(bg[c]) + t * d[c]));
                assert!(
                    ok,
                    "icon {i} {name}: pixel {p:?} off the segment {bg:?}..{fg:?}"
                );
                if (0..3).all(|c| within(p[c], f32::from(fg[c]))) {
                    at_fg += 1;
                }
            }
            assert!(
                at_fg > 0,
                "icon {i} {name}: no pixel at the foreground {fg:?}"
            );
            n += 1;
        }
        assert_eq!(n, 4);
    });
}

#[test]
fn c_applet_panel_12() {
    puron_testkit::limit("C-applet.panel-12", LIMIT, || {
        let p2 = pinned("P2");
        assert!(p2.is_empty());
        for icons in [true, false] {
            let plan = plan_of(&core_panel(&capture(), &p2, icons), Orientation::Horizontal);
            assert_eq!(kinds(&plan), [format!("0 icon {GPU}")], "icons {icons}");
        }
    });
}

#[test]
fn c_applet_panel_13() {
    puron_testkit::limit("C-applet.panel-13", LIMIT, || {
        let plan = p1_plan(false, Orientation::Horizontal);
        assert_eq!(plan.elements.len(), 4);
        assert!(plan.elements.iter().all(is_text), "{:?}", kinds(&plan));
    });
}

#[test]
fn c_applet_panel_16() {
    puron_testkit::limit("C-applet.panel-16", LIMIT, || {
        let plan = p1_plan(true, Orientation::Vertical);
        let mut sim = simulate(&plan);
        let r = rects(&mut sim, &plan);
        let (mut own, mut next) = (0, 0);
        for i in 1..r.len() {
            let gap = r[i].y - (r[i - 1].y + r[i - 1].height);
            match (is_text(&plan.elements[i - 1]), is_text(&plan.elements[i])) {
                (false, true) => {
                    assert!(near(gap, 5.0, 0.5), "icon {} to its text: {gap}", i - 1);
                    own += 1;
                }
                (true, false) => {
                    assert!(near(gap, 5.0, 0.5), "text {} to icon {i}: {gap}", i - 1);
                    next += 1;
                }
                _ => panic!("elements {} and {i}: not icon and text", i - 1),
            }
        }
        assert_eq!(
            (own, next),
            (4, 3),
            "P1's icon-to-text and text-to-icon gaps"
        );
    });
}

/// The eight text sizes in the order the settings list them, written out
/// here so that a reordered list in the code fails.
const SIZES: [TextSize; 8] = [
    TextSize::MatchPanel,
    TextSize::Px10,
    TextSize::Px12,
    TextSize::Px14,
    TextSize::Px16,
    TextSize::Px18,
    TextSize::Px20,
    TextSize::Px24,
];

/// COSMIC's five panel sizes, smallest first.
const PANEL_SIZES: [PanelSize; 5] = [
    PanelSize::XS,
    PanelSize::S,
    PanelSize::M,
    PanelSize::L,
    PanelSize::XL,
];

/// The default settings with icons on and the two text sizes `panel` and
/// `dock`.
fn sized(panel: TextSize, dock: TextSize) -> Settings {
    Settings {
        panel_text_size: panel,
        dock_text_size: dock,
        ..settings(true)
    }
}

/// The plan of the items of P1 with icons on, at `place`, with `s`.
fn p1_at(place: &Place, s: &Settings) -> Plan {
    panel::plan(&core_panel(&capture(), &pinned("P1"), true), place, s)
}

/// A horizontal place on a panel of type Panel at `size`.
fn h_panel(size: PanelSize) -> Place {
    place(size, PanelType::Panel, Orientation::Horizontal)
}

/// A simulator holding the button's content alone, without the button,
/// in a viewport 2000 × 200 px, so that nothing around the content limits
/// the size of its texts.
fn content_alone(plan: &Plan) -> Sim {
    let content = container(panel::view::<()>(plan)).id(Id::new("root"));
    Simulator::with_size(Default::default(), Size::new(2000.0, 200.0), content)
}

/// A simulator holding the panel button itself, as the applet draws it at
/// `place`, in a viewport 2000 × 200 px. Its rectangle has the ID
/// `button`.
fn in_button(plan: &Plan, place: &Place) -> Sim {
    let b = container(panel::button(plan, place, ())).id(Id::new("button"));
    Simulator::with_size(Default::default(), Size::new(2000.0, 200.0), b)
}

/// The rectangles of the plan's text elements, in plan order.
fn text_rects(sim: &mut Sim, plan: &Plan) -> Vec<Rectangle> {
    let r = rects(sim, plan);
    plan.elements
        .iter()
        .zip(r)
        .filter(|(e, _)| is_text(e))
        .map(|(_, r)| r)
        .collect()
}

/// The height of the `CPU` pin's value text, the first text of P1, which
/// the popup's order puts first.
fn cpu_height(sim: &mut Sim, plan: &Plan) -> f32 {
    text_rects(sim, plan)[0].height
}

fn mid_y(r: &Rectangle) -> f32 {
    r.y + r.height / 2.0
}

fn mid_x(r: &Rectangle) -> f32 {
    r.x + r.width / 2.0
}

/// On a panel of type Panel at size S (icon 20 px), with the dock's size
/// fixed at 10 px, each of the eight panel text sizes in turn gives the
/// plan a text size of 17 (`TextSize::MatchPanel`), 10, 12, 14, 16, 18, 20 and
/// 24 px. Then, with no readings and the pins of P5 at size M, both "⚠"
/// texts are higher at 20 px than at 10 px.
#[test]
fn c_applet_panel_17() {
    puron_testkit::limit("C-applet.panel-17", LIMIT, || {
        let s_place = h_panel(PanelSize::S);
        assert_eq!(s_place.icon_size, 20, "icon size at S");
        let got: Vec<u16> = SIZES
            .iter()
            .map(|&t| p1_at(&s_place, &sized(t, TextSize::Px10)).text_size)
            .collect();
        assert_eq!(got, [17, 10, 12, 14, 16, 18, 20, 24], "text sizes");

        let p5 = core_panel(&[], &pinned("P5"), true);
        let heights = |t: TextSize| -> Vec<f32> {
            let plan = panel::plan(&p5, &m_place(Orientation::Horizontal), &sized(t, t));
            assert_eq!(kinds(&plan), ["0 slot", "0 text ⚠", "1 slot", "1 text ⚠"]);
            let mut sim = simulate(&plan);
            text_rects(&mut sim, &plan)
                .iter()
                .map(|r| r.height)
                .collect()
        };
        let (big, small) = (heights(TextSize::Px20), heights(TextSize::Px10));
        assert_eq!(big.len(), 2, "two ⚠ texts");
        for i in 0..2 {
            assert!(
                big[i] > small[i],
                "⚠ {i}: {} at 20 px, {} at 10 px",
                big[i],
                small[i]
            );
        }
    });
}

/// At size S (icon 20 px) on a dock, with the panel's size fixed at
/// 24 px, each of the eight dock text sizes gives 17, 10, 12, 14, 16, 18,
/// 20 and 24 px. On a panel of another type, named "Panel 2", with 20 px
/// for panels and 10 px for docks, the panel's setting applies: 20 px.
#[test]
fn c_applet_panel_18() {
    puron_testkit::limit("C-applet.panel-18", LIMIT, || {
        let dock = place(PanelSize::S, PanelType::Dock, Orientation::Horizontal);
        let got: Vec<u16> = SIZES
            .iter()
            .map(|&t| p1_at(&dock, &sized(TextSize::Px24, t)).text_size)
            .collect();
        assert_eq!(got, [17, 10, 12, 14, 16, 18, 20, 24], "on a dock");
        let other = place(
            PanelSize::S,
            PanelType::Other("Panel 2".to_string()),
            Orientation::Horizontal,
        );
        let plan = p1_at(&other, &sized(TextSize::Px20, TextSize::Px10));
        assert_eq!(plan.text_size, 20, "on a panel of another type");
    });
}

/// With `TextSize::MatchPanel` on a panel, the icon size that libcosmic gives
/// for the sizes XS, S, M, L and XL and for the custom sizes 16, 40 and
/// 100 is 16, 20, 28, 32, 48, 8, 20 and 50 px, and the text size is seven
/// eighths of it rounded down, at least 8: 14, 17, 24, 28, 42, 8, 17 and
/// 43 px.
#[test]
fn c_applet_panel_19() {
    puron_testkit::limit("C-applet.panel-19", LIMIT, || {
        let sizes = PANEL_SIZES.into_iter().chain([
            PanelSize::Custom(16),
            PanelSize::Custom(40),
            PanelSize::Custom(100),
        ]);
        let s = sized(TextSize::MatchPanel, TextSize::Px10);
        let (icons, texts): (Vec<u16>, Vec<u16>) = sizes
            .map(|size| {
                let plan = p1_at(&h_panel(size), &s);
                (plan.icon_size, plan.text_size)
            })
            .unzip();
        assert_eq!(icons, [16, 20, 28, 32, 48, 8, 20, 50], "icon sizes");
        assert_eq!(texts, [14, 17, 24, 28, 42, 8, 17, 43], "text sizes");
    });
}

/// With `TextSize::MatchPanel`, icons on, on a horizontal panel at each size XS
/// to XL: the panel is 32, 40, 56, 64 and 80 px thick; laid out alone,
/// each text of the content is no higher than that, and the `CPU` text
/// grows strictly from size to size; in the button, each text's vertical
/// centre is within 1 px of the button's.
#[test]
fn c_applet_panel_20() {
    puron_testkit::limit("C-applet.panel-20", LIMIT, || {
        let s = sized(TextSize::MatchPanel, TextSize::Px10);
        let mut last = 0.0;
        for (size, thick) in PANEL_SIZES.into_iter().zip([32.0, 40.0, 56.0, 64.0, 80.0]) {
            let place = h_panel(size.clone());
            assert_eq!(f32::from(place.thickness()), thick, "thickness at {size:?}");
            let plan = p1_at(&place, &s);
            let mut sim = content_alone(&plan);
            let texts = text_rects(&mut sim, &plan);
            assert_eq!(texts.len(), 4, "four texts in P1");
            for (i, r) in texts.iter().enumerate() {
                assert!(r.height <= thick, "{size:?} text {i} height {}", r.height);
            }
            let cpu = texts[0].height;
            assert!(cpu > last, "{size:?}: CPU text {cpu}, before {last}");
            last = cpu;

            let mut sim = in_button(&plan, &place);
            let button = bounds(&mut sim, Id::new("button"));
            for (i, r) in text_rects(&mut sim, &plan).iter().enumerate() {
                assert!(
                    near(mid_y(r), mid_y(&button), 1.0),
                    "{size:?} text {i} centre {}, button {}",
                    mid_y(r),
                    mid_y(&button)
                );
            }
        }
    });
}

/// A fixed 24 px text is not fitted to the panel. On the custom size 16
/// (icon 8 px, 16 px thick) and at XL the plan keeps 24 px, and the
/// button at custom 16 stays 16 px high. In the button, the `CPU` text at
/// 24 px is as high at L as at XL, where it fits, and at each of them
/// higher than at 20 px. Laid out alone, the content's `CPU` text at
/// 24 px is as high at custom 16 as at XL.
#[test]
fn c_applet_panel_21() {
    puron_testkit::limit("C-applet.panel-21", LIMIT, || {
        let px24 = sized(TextSize::Px24, TextSize::Px10);
        let px20 = sized(TextSize::Px20, TextSize::Px10);
        let tiny = h_panel(PanelSize::Custom(16));
        assert_eq!((tiny.icon_size, tiny.thickness()), (8, 16), "custom 16");
        let (l, xl) = (h_panel(PanelSize::L), h_panel(PanelSize::XL));
        assert_eq!(p1_at(&tiny, &px24).text_size, 24, "plan at custom 16");
        assert_eq!(p1_at(&xl, &px24).text_size, 24, "plan at XL");

        let in_btn = |place: &Place, s: &Settings| -> f32 {
            let plan = p1_at(place, s);
            cpu_height(&mut in_button(&plan, place), &plan)
        };
        let (l24, xl24) = (in_btn(&l, &px24), in_btn(&xl, &px24));
        assert!(near(l24, xl24, 0.5), "24 px: {l24} at L, {xl24} at XL");
        assert!(l24 > in_btn(&l, &px20), "L: 24 px against 20 px");
        assert!(xl24 > in_btn(&xl, &px20), "XL: 24 px against 20 px");

        let plan = p1_at(&tiny, &px24);
        let mut sim = in_button(&plan, &tiny);
        let h = bounds(&mut sim, Id::new("button")).height;
        assert!(near(h, 16.0, 0.5), "button height {h} at custom 16");

        let alone = |place: &Place| -> f32 {
            let plan = p1_at(place, &px24);
            cpu_height(&mut content_alone(&plan), &plan)
        };
        let (a16, axl) = (alone(&tiny), alone(&xl));
        assert!(
            near(a16, axl, 0.5),
            "alone: {a16} at custom 16, {axl} at XL"
        );
    });
}

/// At size M with icons on, the text sizes 14, 24 and 10 px change the
/// plan only in its text size; in each, every icon is the icon size wide
/// and high, and 9 px separate each icon from its text and each text from
/// the next icon.
#[test]
fn c_applet_panel_22() {
    puron_testkit::limit("C-applet.panel-22", LIMIT, || {
        let place = m_place(Orientation::Horizontal);
        let plans: Vec<Plan> = [TextSize::Px14, TextSize::Px24, TextSize::Px10]
            .iter()
            .map(|&t| p1_at(&place, &sized(t, TextSize::Px20)))
            .collect();
        assert_eq!(
            plans.iter().map(|p| p.text_size).collect::<Vec<_>>(),
            [14, 24, 10]
        );
        for p in &plans[1..] {
            let same = Plan {
                text_size: plans[0].text_size,
                ..p.clone()
            };
            assert_eq!(same, plans[0], "plans differ in more than the text size");
        }
        let size = m_icon_size();
        for plan in &plans {
            let t = plan.text_size;
            let mut sim = simulate(plan);
            let r = rects(&mut sim, plan);
            for (i, e) in plan.elements.iter().enumerate() {
                if is_text(e) {
                    continue;
                }
                assert!(near(r[i].width, size, 0.5), "{t} px: icon {i} width");
                assert!(near(r[i].height, size, 0.5), "{t} px: icon {i} height");
                let gap = r[i + 1].x - (r[i].x + r[i].width);
                assert!(near(gap, 9.0, 0.5), "{t} px: icon {i} to text {gap}");
                if i > 0 {
                    let gap = r[i].x - (r[i - 1].x + r[i - 1].width);
                    assert!(near(gap, 9.0, 0.5), "{t} px: text to icon {i} {gap}");
                }
            }
        }
    });
}

/// Settings loaded from a store written by the previous version (every
/// other key of the `good` fixture, no text size entry), from an empty
/// store, and from a store whose text size entries hold `Default` and
/// `24`, all give `TextSize::MatchPanel`: on a panel and on a dock, 14 px at XS,
/// 24 px at M and 42 px at XL, in all 18 plans.
#[test]
fn c_applet_panel_23() {
    puron_testkit::limit("C-applet.panel-23", LIMIT, || {
        let update = cfg::build("good");
        for key in ["panel_text_size", "dock_text_size"] {
            std::fs::remove_file(update.id_dir().join(key)).expect("remove entry");
        }
        let empty = cfg::build("empty-none");
        let unusable = cfg::build("empty-none");
        cfg::write(&unusable.id_dir().join("panel_text_size"), b"Default");
        cfg::write(&unusable.id_dir().join("dock_text_size"), b"24");
        let mut n = 0;
        for (name, fx) in [
            ("update", &update),
            ("empty", &empty),
            ("unusable", &unusable),
        ] {
            let s = Config::new(&fx.root).load().settings;
            for panel_type in [PanelType::Panel, PanelType::Dock] {
                for (size, want) in [(PanelSize::XS, 14), (PanelSize::M, 24), (PanelSize::XL, 42)] {
                    let at = place(size.clone(), panel_type.clone(), Orientation::Horizontal);
                    let got = panel::plan(&core_panel(&capture(), &pinned("P1"), true), &at, &s);
                    assert_eq!(got.text_size, want, "{name}, {panel_type:?}, {size:?}");
                    n += 1;
                }
            }
        }
        assert_eq!(n, 18, "plans");
    });
}

/// On a vertical panel at size M with icons on, 24 px and then 10 px give
/// plans down the panel with those text sizes. In each, every element
/// starts below the end of the one before and is centred across the
/// button within 1 px, and the `CPU` text is higher at 24 px than at
/// 10 px.
#[test]
fn c_applet_panel_24() {
    puron_testkit::limit("C-applet.panel-24", LIMIT, || {
        let place = m_place(Orientation::Vertical);
        let mut heights = Vec::new();
        for (t, want) in [(TextSize::Px24, 24), (TextSize::Px10, 10)] {
            let plan = p1_at(&place, &sized(t, TextSize::Px20));
            assert_eq!(plan.orientation, Orientation::Vertical);
            assert_eq!(plan.text_size, want);
            let mut sim = simulate(&plan);
            let root = bounds(&mut sim, Id::new("root"));
            let r = rects(&mut sim, &plan);
            for i in 0..r.len() {
                if i > 0 {
                    let above = r[i - 1].y + r[i - 1].height;
                    assert!(r[i].y >= above, "{want} px: element {i} top above {above}");
                }
                assert!(
                    near(mid_x(&r[i]), mid_x(&root), 1.0),
                    "{want} px: element {i} centre {}, button {}",
                    mid_x(&r[i]),
                    mid_x(&root)
                );
            }
            heights.push(cpu_height(&mut sim, &plan));
        }
        assert!(heights[0] > heights[1], "CPU text heights {heights:?}");
    });
}

// The tests below are of the value boxes. Unless a test says otherwise,
// the items come from `puron_core::panel::panel` with the readings of
// kinds.json, one of each class and family, all six pinned; the fan is
// the fourth item.

/// The identity of the fan reading of kinds.json.
const FAN_ID: &str = "fan/lm-sensors/k3";

/// The readings and the pinned list of kinds.json.
fn kinds_json() -> (Vec<RawReading>, Vec<String>) {
    let v: serde_json::Value = serde_json::from_str(&raw("kinds.json")).expect("parse kinds.json");
    let readings = serde_json::from_value(v["R"].clone()).expect("readings");
    let pins = serde_json::from_value(v["P"].clone()).expect("pinned list");
    (readings, pins)
}

/// The readings of kinds.json with the fan at `rpm`, or with no value
/// when `rpm` is `None`.
fn kinds_fan(rpm: Option<f64>) -> Vec<RawReading> {
    let (mut raw, _) = kinds_json();
    raw.iter_mut().find(|r| r.key == "k3").expect("fan").value = rpm;
    raw
}

/// The readings of kinds.json without the fan.
fn kinds_no_fan() -> Vec<RawReading> {
    let (mut raw, _) = kinds_json();
    raw.retain(|r| r.key != "k3");
    raw
}

/// The panels for `steps`, readings in turn, of the pins `pins` under
/// `s`, with each panel's state passed on to the next.
fn panels(steps: &[Vec<RawReading>], pins: &[String], s: &Settings) -> Vec<Panel> {
    let mut state = State::default();
    let mut out = Vec::new();
    for raw in steps {
        let p = puron_core::panel::panel(&menu::shown(raw, s), pins, s, &state);
        state = p.state.clone();
        out.push(p);
    }
    out
}

/// The panels of every pin of kinds.json with the fan at each of `rpms`
/// in turn, under the default settings with icons on.
fn fan_steps(rpms: &[f64]) -> Vec<Panel> {
    let steps: Vec<Vec<RawReading>> = rpms.iter().map(|r| kinds_fan(Some(*r))).collect();
    panels(&steps, &kinds_json().1, &settings(true))
}

/// The index in `plan.elements` of the text element of item `item`.
fn text_of(plan: &Plan, item: usize) -> usize {
    plan.elements
        .iter()
        .position(|e| matches!(e, Element::Text { item: i, .. } if *i == item))
        .unwrap_or_else(|| panic!("no text for item {item}"))
}

/// The button of `plan` at `place`, in a viewport 2000 × 200 px, with the
/// rectangles of the box and the value of item `item`'s text, and the
/// button's own rectangle.
struct Measured {
    button: Rectangle,
    rects: Vec<Rectangle>,
    boxed: Rectangle,
    value: Rectangle,
}

fn measure(plan: &Plan, place: &Place, item: usize) -> Measured {
    let mut sim = in_button(plan, place);
    let i = text_of(plan, item);
    Measured {
        button: bounds(&mut sim, Id::new("button")),
        rects: rects(&mut sim, plan),
        boxed: bounds(&mut sim, panel::element_id(i)),
        value: bounds(&mut sim, panel::value_id(i)),
    }
}

/// The widths of `texts`, each laid out alone as a text of `size` px, in
/// one simulator beside the button of `plan` at `place`.
fn widths_alone(plan: &Plan, place: &Place, texts: &[String], size: u16) -> Vec<f32> {
    let mut children: Vec<cosmic::Element<'static, ()>> =
        vec![container(panel::button(plan, place, ())).into()];
    for (k, t) in texts.iter().enumerate() {
        children.push(
            container(cosmic::widget::text(t.clone()).size(size))
                .id(Id::new(format!("alone{k}")))
                .into(),
        );
    }
    let all = cosmic::widget::column::with_children(children);
    let mut sim: Sim = Simulator::with_size(Default::default(), Size::new(2000.0, 600.0), all);
    (0..texts.len())
        .map(|k| bounds(&mut sim, Id::new(format!("alone{k}"))).width)
        .collect()
}

fn widest(widths: &[f32]) -> f32 {
    widths.iter().copied().fold(0.0, f32::max)
}

fn right_edge(r: &Rectangle) -> f32 {
    r.x + r.width
}

/// The box texts of each pin in the plan: for each class its box text
/// with its unit, then the texts for no value and for a missing value.
/// Without the fan's unit the fan's first text loses its unit. Under the
/// Esperanto test catalog every text, the unit's and the two others',
/// comes from that catalog.
#[test]
fn c_applet_panel_25() {
    puron_testkit::limit("C-applet.panel-25", LIMIT, || {
        let (raw, pins) = kinds_json();
        let boxes = |s: &Settings| {
            let p = puron_core::panel::panel(&menu::shown(&raw, s), &pins, s, &State::default());
            panel::plan(&p, &m_place(Orientation::Horizontal), s).boxes
        };
        let with = |first: &str, none: &str, missing: &str| {
            vec![first.to_string(), none.to_string(), missing.to_string()]
        };
        if std::env::var_os(CHILD).is_some() {
            puron_applet::i18n::start_with(puron_applet::i18n::test_catalog::eo(), |n| {
                std::env::var_os(n)
            });
            let eo = |first: &str| with(first, "EO N/A", "EO ⚠");
            assert_eq!(
                boxes(&settings(true)),
                [
                    eo("888EO °C"),
                    eo("888EO °C"),
                    eo("888EO °C"),
                    eo("8888EO  rpm"),
                    eo("+88.88EO V"),
                    eo("+888.88EO W"),
                ]
            );
            return;
        }
        let en = |first: &str| with(first, "N/A", "⚠");
        assert_eq!(
            boxes(&settings(true)),
            [
                en("888°C"),
                en("888°C"),
                en("888°C"),
                en("8888 rpm"),
                en("+88.88V"),
                en("+888.88W"),
            ]
        );
        let no_unit = Settings {
            show_rotation_rate_unit: false,
            ..settings(true)
        };
        assert_eq!(boxes(&no_unit)[3], en("8888"));
        run_child_eo("c_applet_panel_25");
    });
}

/// Runs `test` again in a fresh run of this test binary under the locale
/// `eo`, where the test chooses the Esperanto test catalog, and fails when
/// that run fails. The catalog can be chosen only once in a process.
fn run_child_eo(test: &str) {
    let mut cmd = puron_testkit::rerun_self(test);
    for v in puron_testkit::loc::VARS {
        cmd.env_remove(v);
    }
    for (k, v) in puron_testkit::loc::env("E1") {
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

/// The fan at 1234 rpm, then at 0, with the state passed on: the fan's
/// text element is as wide at 0 as at 1234.
#[test]
fn c_applet_panel_26() {
    puron_testkit::limit("C-applet.panel-26", LIMIT, || {
        let place = m_place(Orientation::Horizontal);
        let p = fan_steps(&[1234.0, 0.0]);
        let s = settings(true);
        let at_1234 = measure(&panel::plan(&p[0], &place, &s), &place, 3);
        let at_0 = measure(&panel::plan(&p[1], &place, &s), &place, 3);
        assert!(
            near(at_0.boxed.width, at_1234.boxed.width, 0.5),
            "fan box {} at 0, {} at 1234",
            at_0.boxed.width,
            at_1234.boxed.width
        );
    });
}

/// The fan at 1234, 12000 and 0 rpm in turn: its box is wider at 12000
/// than at 1234, and keeps that width at 0.
#[test]
fn c_applet_panel_27() {
    puron_testkit::limit("C-applet.panel-27", LIMIT, || {
        let place = m_place(Orientation::Horizontal);
        let s = settings(true);
        let w: Vec<f32> = fan_steps(&[1234.0, 12000.0, 0.0])
            .iter()
            .map(|p| measure(&panel::plan(p, &place, &s), &place, 3).boxed.width)
            .collect();
        assert!(w[1] > w[0], "fan box {} at 12000, {} at 1234", w[1], w[0]);
        assert!(
            near(w[2], w[1], 0.5),
            "fan box {} at 0, {} at 12000",
            w[2],
            w[1]
        );
    });
}

/// At a text size of 20 px, from the start state, each box is exactly as
/// wide as the widest of its box texts laid out alone at 20 px, under the
/// default settings and in Fahrenheit with decimals. With the defaults,
/// each temperature box is wider than its value, `40°C`, which has a digit
/// fewer than the box text.
#[test]
fn c_applet_panel_28() {
    puron_testkit::limit("C-applet.panel-28", LIMIT, || {
        let place = m_place(Orientation::Horizontal);
        let (raw, pins) = kinds_json();
        let defaults = sized(TextSize::Px20, TextSize::MatchPanel);
        let fahrenheit = Settings {
            temperature_unit: puron_core::settings::TemperatureUnit::Fahrenheit,
            show_decimals: true,
            ..defaults.clone()
        };
        for (case, s) in [("defaults", &defaults), ("Fahrenheit", &fahrenheit)] {
            let p = panels(&[raw.clone()], &pins, s).remove(0);
            let plan = panel::plan(&p, &place, s);
            assert_eq!(plan.text_size, 20);
            for item in 0..p.items.len() {
                let m = measure(&plan, &place, item);
                let alone = widths_alone(&plan, &place, &plan.boxes[item], 20);
                assert!(
                    near(m.boxed.width, widest(&alone), 0.5),
                    "{case}, item {item}: box {} against texts {alone:?}",
                    m.boxed.width
                );
                if case == "defaults" && item < 3 {
                    assert!(
                        m.boxed.width > m.value.width,
                        "temperature {item}: box {} against value {}",
                        m.boxed.width,
                        m.value.width
                    );
                }
            }
        }
    });
}

/// Under the Esperanto test catalog, with the fan's unit hidden (box text
/// `8888`): the fan at 1000 rpm, then with no value, then with no reading
/// at all. The fan's box has the same width in the three cases, the width
/// of the widest of `8888` and the catalog's texts for no value and for a
/// missing value, each laid out alone; that is wider than `8888` alone.
#[test]
fn c_applet_panel_29() {
    puron_testkit::limit("C-applet.panel-29", LIMIT, || {
        if std::env::var_os(CHILD).is_none() {
            return run_child_eo("c_applet_panel_29");
        }
        puron_applet::i18n::start_with(puron_applet::i18n::test_catalog::eo(), |n| {
            std::env::var_os(n)
        });
        let place = m_place(Orientation::Horizontal);
        let s = Settings {
            show_rotation_rate_unit: false,
            ..settings(true)
        };
        let steps = [kinds_fan(Some(1000.0)), kinds_fan(None), kinds_no_fan()];
        let p = panels(&steps, &kinds_json().1, &s);
        // A pin with no line follows the others, so the fan is found by its
        // box text rather than by its place.
        let mut w = Vec::new();
        for p in &p {
            let plan = panel::plan(p, &place, &s);
            let fan = plan
                .boxes
                .iter()
                .position(|b| b.first().map(String::as_str) == Some("8888"))
                .expect("the fan's box texts");
            w.push(measure(&plan, &place, fan).boxed.width);
        }
        let plan = panel::plan(&p[0], &place, &s);
        let texts = ["8888", "EO N/A", "EO ⚠"].map(String::from).to_vec();
        assert_eq!(plan.boxes[3], texts);
        let alone = widths_alone(&plan, &place, &texts, plan.text_size);
        for (k, w) in w.iter().enumerate() {
            assert!(
                near(*w, widest(&alone), 0.5),
                "case {k}: box {w}, texts {alone:?}"
            );
        }
        assert!(widest(&alone) > alone[0], "texts {alone:?}");
    });
}

/// The fan at 0 rpm in its four-digit box: the value ends at the box's
/// right edge, and starts at least 1 px right of the box's left edge.
#[test]
fn c_applet_panel_30() {
    puron_testkit::limit("C-applet.panel-30", LIMIT, || {
        let place = m_place(Orientation::Horizontal);
        let p = fan_steps(&[0.0]);
        let m = measure(&panel::plan(&p[0], &place, &settings(true)), &place, 3);
        assert!(
            near(right_edge(&m.value), right_edge(&m.boxed), 0.5),
            "value right {}, box right {}",
            right_edge(&m.value),
            right_edge(&m.boxed)
        );
        assert!(
            m.value.x >= m.boxed.x + 1.0,
            "value left {}, box left {}",
            m.value.x,
            m.boxed.x
        );
    });
}

/// The fan at 1234 rpm, then at 0: the voltage's icon and text and the
/// power's icon and text, the four elements after the fan's text, start
/// where they started, and the button keeps its width.
#[test]
fn c_applet_panel_31() {
    puron_testkit::limit("C-applet.panel-31", LIMIT, || {
        let place = m_place(Orientation::Horizontal);
        let s = settings(true);
        let p = fan_steps(&[1234.0, 0.0]);
        let plan = panel::plan(&p[0], &place, &s);
        let fan = text_of(&plan, 3);
        assert_eq!(
            plan.elements.len(),
            fan + 5,
            "four elements after the fan's text"
        );
        let a = measure(&plan, &place, 3);
        let b = measure(&panel::plan(&p[1], &place, &s), &place, 3);
        for i in fan + 1..fan + 5 {
            assert!(
                near(b.rects[i].x, a.rects[i].x, 0.5),
                "element {i} left {} at 0, {} at 1234",
                b.rects[i].x,
                a.rects[i].x
            );
        }
        assert!(
            near(b.button.width, a.button.width, 0.5),
            "button {} at 0, {} at 1234",
            b.button.width,
            a.button.width
        );
    });
}

/// On a vertical panel, the fan pinned alone at 1234 rpm, then at 0: the
/// button keeps its width; the fan's box is centred across the button
/// within 1 px, and the value ends at the box's right edge. The button is
/// measured inside a container that shrinks to it, so that its width
/// follows its content.
#[test]
fn c_applet_panel_32() {
    puron_testkit::limit("C-applet.panel-32", LIMIT, || {
        let place = m_place(Orientation::Vertical);
        let s = settings(true);
        let steps = [kinds_fan(Some(1234.0)), kinds_fan(Some(0.0))];
        let p = panels(&steps, &[FAN_ID.to_string()], &s);
        let mut widths = Vec::new();
        for (k, p) in p.iter().enumerate() {
            let plan = panel::plan(p, &place, &s);
            assert_eq!(plan.orientation, Orientation::Vertical);
            let b = container(panel::button(&plan, &place, ()))
                .width(Length::Shrink)
                .id(Id::new("button"));
            let mut sim: Sim =
                Simulator::with_size(Default::default(), Size::new(2000.0, 600.0), b);
            let button = bounds(&mut sim, Id::new("button"));
            let i = text_of(&plan, 0);
            let boxed = bounds(&mut sim, panel::element_id(i));
            let value = bounds(&mut sim, panel::value_id(i));
            assert!(
                button.width < 2000.0,
                "step {k}: the button fills the viewport"
            );
            assert!(
                near(mid_x(&boxed), mid_x(&button), 1.0),
                "step {k}: box centre {}, button centre {}",
                mid_x(&boxed),
                mid_x(&button)
            );
            assert!(
                near(right_edge(&value), right_edge(&boxed), 0.5),
                "step {k}: value right {}, box right {}",
                right_edge(&value),
                right_edge(&boxed)
            );
            widths.push(button.width);
        }
        assert!(near(widths[1], widths[0], 0.5), "button widths {widths:?}");
    });
}
