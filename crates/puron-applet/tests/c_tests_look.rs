//! Tests of the look of the popup's About page, and of what the look of
//! the About and Settings pages leaves as it was: their entries, order,
//! texts and clicks.
//!
//! The About plan is that of version 1.6.0, the Debian package and the
//! source set `all` (lm-sensors available with 12 readings, NVMe with 2,
//! NVIDIA not available, udisks2 with 1, liquidctl not available) unless
//! a test names another. The layout tests draw it in iced's headless test
//! simulator at the narrowest and the widest popup width and compare
//! rectangles in logical pixels with a tolerance of half a pixel.
#![allow(non_snake_case)]

use cosmic::iced::{Rectangle, Size};
use cosmic::widget::{container, Id};
use cosmic::Theme;
use iced_test::{selector, Simulator};
use puron_applet::about::{self, Align, Clicked, Entry, Input, Package, Place, Plan, Source};
use puron_applet::look::{Role, TextStyle};
use puron_applet::popup::{self, Event, Reply};
use puron_applet::settings;
use puron_config::Config;
use puron_testkit::cfg;

const LIMIT: u64 = 5;
const TOL: f32 = 0.5;
const HEIGHT: f32 = 1000.0;

/// The source set `all`, in the page's order.
const ALL: [Source; 5] = [
    Source::Available(12),
    Source::Available(2),
    Source::NotAvailable,
    Source::Available(1),
    Source::NotAvailable,
];
/// The source set `off`: all five off.
const OFF: [Source; 5] = [Source::Off; 5];

fn plan_of(package: Package, sources: [Source; 5]) -> Plan {
    puron_applet::start();
    about::plan(&Input {
        version: "1.6.0".to_string(),
        package,
        sources,
    })
}

fn plan() -> Plan {
    plan_of(Package::Deb, ALL)
}

type Sim = Simulator<'static, Clicked, Theme, cosmic::Renderer>;

fn sim(plan: &Plan, width: f32) -> Sim {
    let root = container(about::view(plan)).id(Id::new("root"));
    Simulator::with_size(Default::default(), Size::new(width, HEIGHT), root)
}

fn rect(s: &mut Sim, id: Id) -> Rectangle {
    s.find(selector::id(id.clone()))
        .unwrap_or_else(|e| panic!("find {id:?}: {e:?}"))
        .bounds()
}

fn near(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn center_x(r: Rectangle) -> f32 {
    r.x + r.width / 2.0
}

fn right(r: Rectangle) -> f32 {
    r.x + r.width
}

fn bottom(r: Rectangle) -> f32 {
    r.y + r.height
}

/// Whether `r` lies within `area`, give or take `TOL`.
fn inside(r: Rectangle, area: Rectangle) -> bool {
    r.x >= area.x - TOL
        && r.y >= area.y - TOL
        && right(r) <= right(area) + TOL
        && bottom(r) <= bottom(area) + TOL
}

/// The About plan: the name, the version and the summary have the place
/// `header` and are centered; the name is in the title style, the version
/// in body text, both in the default color, and the summary in grey body
/// text. Drawn at the narrowest and the widest popup, each of the three
/// texts has its horizontal center within 1 px of the popup's, and the
/// name is taller than the version.
#[test]
fn c_tests_49() {
    puron_testkit::limit("C-tests-49", LIMIT, || {
        let plan = plan();
        let expect = [
            (TextStyle::Title3, Role::Default),
            (TextStyle::Body, Role::Default),
            (TextStyle::Body, Role::Muted),
        ];
        for (n, want) in (2..5).zip(expect) {
            let look = &plan.looks[n];
            assert_eq!(look.place, Place::Header, "entry {}", n + 1);
            assert_eq!(look.align, Align::Center, "entry {}", n + 1);
            assert_eq!(look.texts, [want], "entry {}", n + 1);
        }
        for width in [popup::WIDTH, popup::MAX_WIDTH] {
            let mut s = sim(&plan, width);
            let root = rect(&mut s, Id::new("root"));
            let texts: Vec<Rectangle> = (2..5)
                .map(|n| rect(&mut s, popup::element_id(n, "text")))
                .collect();
            for (n, r) in (2..5).zip(&texts) {
                assert!(
                    near(center_x(*r), center_x(root), 1.0),
                    "width {width}: entry {} centered at {} against {}",
                    n + 1,
                    center_x(*r),
                    center_x(root)
                );
            }
            assert!(
                texts[0].height > texts[1].height,
                "width {width}: name {:?} not taller than version {:?}",
                texts[0],
                texts[1]
            );
        }
    });
}

/// The About plan's places in order: Back and the separator are rows, the
/// name, version and summary the header, the three fields the facts card,
/// the heading "Sensor sources" on the page, the five sources their card,
/// the heading "Links" on the page, the four links their card and the
/// button on the page. The field labels are grey body text, and the two
/// headings grey small headings. Drawn at the widest popup, each entry of
/// a card lies inside its card, the three cards share their left and
/// right edges, and each heading lies between the card above it and the
/// card it heads.
#[test]
fn c_tests_50() {
    puron_testkit::limit("C-tests-50", LIMIT, || {
        let plan = plan();
        let places: Vec<Place> = plan.looks.iter().map(|l| l.place).collect();
        let mut want = vec![Place::Row; 2];
        want.extend([Place::Header; 3]);
        want.extend([Place::CardFacts; 3]);
        want.push(Place::Page);
        want.extend([Place::CardSources; 5]);
        want.push(Place::Page);
        want.extend([Place::CardLinks; 4]);
        want.push(Place::Page);
        assert_eq!(places, want);
        for n in 5..8 {
            assert_eq!(
                plan.looks[n].texts[0],
                (TextStyle::Body, Role::Muted),
                "label of entry {}",
                n + 1
            );
        }
        for n in [8, 14] {
            assert_eq!(
                plan.looks[n].texts,
                [(TextStyle::CaptionHeading, Role::Muted)],
                "heading entry {}",
                n + 1
            );
        }
        let mut s = sim(&plan, popup::MAX_WIDTH);
        let cards: Vec<Rectangle> = [Place::CardFacts, Place::CardSources, Place::CardLinks]
            .iter()
            .map(|p| rect(&mut s, about::card_id(*p)))
            .collect();
        for (n, place) in places.iter().enumerate() {
            let card = match place {
                Place::CardFacts => cards[0],
                Place::CardSources => cards[1],
                Place::CardLinks => cards[2],
                _ => continue,
            };
            let r = rect(&mut s, popup::entry_id(n));
            assert!(
                inside(r, card),
                "entry {} {r:?} outside its card {card:?}",
                n + 1
            );
        }
        for c in &cards[1..] {
            assert!(near(c.x, cards[0].x, TOL), "card {c:?} left edge");
            assert!(
                near(right(*c), right(cards[0]), TOL),
                "card {c:?} right edge"
            );
        }
        let sources = rect(&mut s, popup::entry_id(8));
        let links = rect(&mut s, popup::entry_id(14));
        assert!(
            bottom(cards[0]) <= sources.y + TOL,
            "facts card over heading 9"
        );
        assert!(
            bottom(sources) <= cards[1].y + TOL,
            "heading 9 over its card"
        );
        assert!(
            bottom(links) <= cards[2].y + TOL,
            "heading 15 over its card"
        );
    });
}

/// The role of each source's state text, and the style of its count.
fn source_roles(plan: &Plan) -> Vec<(String, Role)> {
    plan.entries
        .iter()
        .zip(&plan.looks)
        .filter_map(|(e, l)| match e {
            Entry::Source { name, state, .. } => {
                assert_eq!(
                    l.texts[2],
                    (TextStyle::Heading, Role::Default),
                    "{name} count"
                );
                Some((format!("{name} {state}"), l.texts[1].1))
            }
            _ => None,
        })
        .collect()
}

fn roles(list: &[(&str, Role)]) -> Vec<(String, Role)> {
    list.iter().map(|(t, r)| (t.to_string(), *r)).collect()
}

/// The source set `all` on the Debian package: the available sources are
/// green and the ones not available grey, each count in bold body text in
/// the default color. `flatpak-on`: liquidctl reads "not available in the
/// Flatpak" in the warning color. `off`: all five read "off" in grey.
#[test]
fn c_tests_51() {
    puron_testkit::limit("C-tests-51", LIMIT, || {
        assert_eq!(
            source_roles(&plan()),
            roles(&[
                ("lm-sensors available", Role::Success),
                ("NVMe available", Role::Success),
                ("NVIDIA not available", Role::Muted),
                ("udisks2 available", Role::Success),
                ("liquidctl not available", Role::Muted),
            ])
        );
        assert_eq!(
            source_roles(&plan_of(Package::Flatpak, ALL))[4],
            (
                "liquidctl not available in the Flatpak".to_string(),
                Role::Warning
            )
        );
        assert_eq!(
            source_roles(&plan_of(Package::Deb, OFF)),
            roles(&[
                ("lm-sensors off", Role::Muted),
                ("NVMe off", Role::Muted),
                ("NVIDIA off", Role::Muted),
                ("udisks2 off", Role::Muted),
                ("liquidctl off", Role::Muted),
            ])
        );
    });
}

/// The four links: each text is in the accent color, with the "open
/// outside" arrow in the accent color too. Drawn at the widest popup,
/// each arrow is 16 × 16 px and ends 14 px left of the links card's right
/// edge, and each text starts 14 px right of the card's left edge and
/// ends at or before the arrow.
#[test]
fn c_tests_52() {
    puron_testkit::limit("C-tests-52", LIMIT, || {
        let plan = plan();
        let links: Vec<usize> = (15..19).collect();
        for &n in &links {
            assert!(
                matches!(plan.entries[n], Entry::Link { .. }),
                "entry {}",
                n + 1
            );
            let look = &plan.looks[n];
            assert_eq!(
                look.texts,
                [(TextStyle::Body, Role::Accent)],
                "entry {}",
                n + 1
            );
            assert_eq!(
                look.icon,
                Some((about::LINK_ICON, Role::Accent)),
                "entry {}",
                n + 1
            );
        }
        assert_eq!(about::LINK_ICON, "insert-arrow-symbolic");
        let mut s = sim(&plan, popup::MAX_WIDTH);
        let card = rect(&mut s, about::card_id(Place::CardLinks));
        for &n in &links {
            let icon = rect(&mut s, popup::element_id(n, "icon"));
            let text = rect(&mut s, popup::element_id(n, "text"));
            assert!(
                near(icon.width, 16.0, TOL) && near(icon.height, 16.0, TOL),
                "entry {}: icon {icon:?}",
                n + 1
            );
            assert!(
                near(right(icon), right(card) - 14.0, TOL),
                "entry {}: icon ends at {} in card {card:?}",
                n + 1,
                right(icon)
            );
            assert!(
                near(text.x, card.x + 14.0, TOL),
                "entry {}: text starts at {} in card {card:?}",
                n + 1,
                text.x
            );
            assert!(
                right(text) <= icon.x + TOL,
                "entry {}: text {text:?} reaches the icon {icon:?}",
                n + 1
            );
        }
    });
}

/// An About plan as one line per entry, `kind<TAB>texts<TAB>click`, with
/// the texts joined by ` | `: the content alone, without the look.
fn about_dump(plan: &Plan) -> String {
    let mut out = String::new();
    for e in &plan.entries {
        let (kind, texts, click) = match e {
            Entry::Back { icon, text } => ("back", format!("{icon} | {text}"), "back".into()),
            Entry::Separator => ("separator", String::new(), "none".into()),
            Entry::Name(t) => ("name", t.clone(), "none".into()),
            Entry::Version(t) => ("version", t.clone(), "none".into()),
            Entry::Summary(t) => ("summary", t.clone(), "none".into()),
            Entry::Field { label, values } => (
                "field",
                format!("{label} | {}", values.join(" | ")),
                "none".into(),
            ),
            Entry::Heading(t) => ("heading", t.clone(), "none".into()),
            Entry::Source { name, state, count } => (
                "source",
                match count {
                    Some(c) => format!("{name} | {state} | {c}"),
                    None => format!("{name} | {state}"),
                },
                "none".into(),
            ),
            Entry::Link { text, url } => ("link", text.clone(), format!("link {url}")),
            Entry::Copy { text } => ("copy", text.clone(), "copy".into()),
        };
        out.push_str(&format!("{kind}\t{texts}\t{click}\n"));
    }
    out
}

/// A Settings plan as one line per entry, `kind<TAB>texts<TAB>click`: Back,
/// the separator, the notice, and for each section its heading and then
/// each item with its label, its tooltip and its list texts, and the
/// messages its control sends.
fn settings_dump(plan: &settings::Plan) -> String {
    let mut out = String::new();
    let mut n = 0;
    for e in &plan.entries {
        match e {
            settings::Entry::Back { icon, text } => {
                out.push_str(&format!("back\t{icon} | {text}\tback\n"))
            }
            settings::Entry::Separator => out.push_str("separator\t\tnone\n"),
            settings::Entry::Notice(t) => out.push_str(&format!("notice\t{t}\tnone\n")),
            settings::Entry::Section(s) => {
                out.push_str(&format!("section\t{}\tnone\n", s.heading));
                for item in &s.items {
                    n += 1;
                    let mut texts = vec![item.label.clone()];
                    texts.extend(item.tooltip.as_ref().map(|(_, t)| t.clone()));
                    let click = match &item.control {
                        settings::Kind::Number { .. } => "typed, enter, focus lost, plus, minus",
                        settings::Kind::Switch(_) => "switch",
                        settings::Kind::List { items, .. } => {
                            texts.extend(items.iter().cloned());
                            "list"
                        }
                    };
                    out.push_str(&format!(
                        "item {n}\t{}\t{click} {}\n",
                        texts.join(" | "),
                        item.key
                    ));
                }
            }
        }
    }
    out
}

fn dump_file(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The Settings plan of the `good` fixture.
fn settings_plan() -> settings::Plan {
    puron_applet::start();
    let fx = cfg::build("good");
    settings::plan(&settings::Input {
        values: Config::new(&fx.root).load().settings,
        typed: None,
        notice: false,
    })
}

/// The look leaves the content of both pages as it was. The About plans
/// of the source sets `all` and `flatpak-on`, and the Settings plan of the
/// `good` fixture, dumped without their look, equal the dump files written
/// from the pages' tables byte for byte. Every About entry that answers a
/// click gives its reply: Back goes back and a link opens its address,
/// both as before, and "Copy details" copies, the popup open; every
/// Settings message gives its event with the popup open.
#[test]
fn c_tests_54() {
    puron_testkit::limit("C-tests-54", LIMIT, || {
        let deb = plan();
        assert_eq!(about_dump(&deb), dump_file("c_tests_54_about.tsv"), "all");
        assert_eq!(
            about_dump(&plan_of(Package::Flatpak, ALL)),
            dump_file("c_tests_54_about_flatpak.tsv"),
            "flatpak-on"
        );
        let page = settings_plan();
        assert_eq!(
            settings_dump(&page),
            dump_file("c_tests_54_settings.tsv"),
            "settings"
        );
        for e in &deb.entries {
            let (message, want) = match e {
                Entry::Back { .. } => (Clicked::Back, (Some(Event::Back), false)),
                Entry::Link { url, .. } => (
                    Clicked::Link(url.clone()),
                    (Some(Event::OpenLink(url.clone())), true),
                ),
                Entry::Copy { .. } => (Clicked::Copy, (Some(Event::CopyDetails), false)),
                _ => continue,
            };
            assert_eq!(
                about::clicked(&deb, message),
                Reply {
                    event: want.0,
                    close: want.1
                },
                "{e:?}"
            );
        }
        let edits = [
            settings::Edit::Change {
                item: 2,
                choice: settings::Choice::Switch(true),
            },
            settings::Edit::Change {
                item: 3,
                choice: settings::Choice::Entry(1),
            },
            settings::Edit::Typed("9".into()),
            settings::Edit::Enter,
            settings::Edit::FocusLost,
            settings::Edit::Plus,
            settings::Edit::Minus,
        ];
        assert_eq!(
            settings::clicked(&page, settings::Clicked::Back),
            Reply {
                event: Some(Event::Back),
                close: false
            }
        );
        for e in edits {
            assert_eq!(
                settings::clicked(&page, settings::Clicked::Edit(e.clone())),
                Reply {
                    event: Some(Event::Edit(e.clone())),
                    close: false
                },
                "{e:?}"
            );
        }
    });
}
