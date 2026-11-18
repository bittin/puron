//! Tests of the panel items (`panel`). Each test must finish within 2
//! seconds (`puron_testkit::limit`). The fixtures are under
//! crates/puron-testkit/fixtures/raw/ (capture.json, summary.json,
//! dupes.json, rebuild.json, pins.json, kinds.json, cpu.json). Unless a
//! test says otherwise, the settings are the defaults and the icon memory
//! starts empty.
// A test name may carry a case after two underscores.
#![allow(non_snake_case)]

use std::collections::BTreeMap;

use puron_core::format::{Text, Unit};
use puron_core::menu::{self, Entry, IconKind, Report};
use puron_core::panel::{self, Icon, Item, Panel, State};
use puron_core::rows::{Kind, RawReading};
use puron_core::settings::{Settings, TemperatureUnit, TextSize};

const LIMIT: u64 = 2;

fn json<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let text = std::fs::read_to_string(puron_testkit::fixture("raw", name)).expect("read fixture");
    serde_json::from_str(&text).expect("parse fixture")
}

fn capture() -> Vec<RawReading> {
    json("capture.json")
}

fn set(file: &str, name: &str) -> Vec<RawReading> {
    puron_testkit::raw_sets(file, |r: &mut RawReading, v| r.value = Some(v))
        .remove(name)
        .expect("fixture set")
}

/// A set of rebuild.json. Set S1 holds the readings of capture.json; a
/// test in c_core_menu.rs checks that.
fn snapshot(name: &str) -> Vec<RawReading> {
    set("rebuild.json", name)
}

fn pinned(name: &str) -> Vec<String> {
    json::<BTreeMap<String, Vec<String>>>("pins.json")
        .remove(name)
        .expect("X-raw-pins set")
}

/// The identity of the reading of `raw` named `name`. It is built as an
/// lm-sensors identity, so it is right for lm-sensors readings only. A
/// text that names no reading is returned as it is.
fn id(raw: &[RawReading], name: &str) -> String {
    match raw.iter().find(|r| r.name.as_deref() == Some(name)) {
        Some(r) => {
            let class = match r.kind {
                Some(Kind::Fan) => "fan",
                Some(Kind::Voltage) => "voltage",
                Some(Kind::Power) => "power",
                _ => "temperature",
            };
            format!("{class}/lm-sensors/{}", r.key)
        }
        None => name.to_string(),
    }
}

fn ids(raw: &[RawReading], names: &[&str]) -> Vec<String> {
    names.iter().map(|n| id(raw, n)).collect()
}

fn settings_with(change: impl FnOnce(&mut Settings)) -> Settings {
    let mut s = Settings::default();
    change(&mut s);
    s
}

fn run(raw: &[RawReading], pinned: &[String], settings: &Settings, state: &State) -> Panel {
    panel::panel(&menu::shown(raw, settings), pinned, settings, state)
}

fn lay(raw: &[RawReading], pinned: &[String]) -> Panel {
    run(raw, pinned, &Settings::default(), &State::default())
}

/// A value text with an English unit symbol; `N/A` for "no value".
fn text(value: &Option<Text>) -> String {
    match value {
        None => "N/A".to_string(),
        Some(t) => {
            let unit = match t.unit {
                None => "",
                Some(Unit::Celsius) => "°C",
                Some(Unit::Fahrenheit) => "°F",
                Some(Unit::Rpm) => "rpm",
                Some(Unit::Volt) => "V",
                Some(Unit::Watt) => "W",
            };
            format!("{}{unit}", t.number)
        }
    }
}

fn kind(k: IconKind) -> &'static str {
    match k {
        IconKind::GeneralTemperature => "general",
        IconKind::CpuTemperature => "cpu",
        IconKind::GpuTemperature => "gpu",
        IconKind::DriveTemperature => "drive",
        IconKind::WifiTemperature => "wifi",
        IconKind::WiredNetworkTemperature => "wired",
        IconKind::MemoryTemperature => "memory",
        IconKind::Fan => "fan",
        IconKind::Voltage => "voltage",
        IconKind::Power => "power",
    }
}

/// The items as short strings: the icon kind and the value text, such as
/// `general 45°C`. `∅` stands for an icon whose kind is not known, `⚠` for
/// a pin with no line, and `empty` for the empty-state item. With icons
/// off, the icon word is left out.
fn render(p: &Panel) -> Vec<String> {
    p.items
        .iter()
        .map(|i| match i {
            Item::EmptyState => "empty".to_string(),
            Item::Pin { icon, value } => {
                let value = value.as_ref().map_or("⚠".to_string(), text);
                match icon {
                    Icon::None => value,
                    Icon::EmptySlot => format!("∅ {value}"),
                    Icon::Kind(k) => format!("{} {value}", kind(*k)),
                }
            }
        })
        .collect()
}

fn assert_items(p: &Panel, want: &[&str]) {
    assert_eq!(render(p), want);
}

#[test]
fn c_core_panel_1() {
    puron_testkit::limit("C-core.panel-1", LIMIT, || {
        let raw = capture();
        assert_eq!(pinned("P1"), ids(&raw, &["CPU fan", "WiFi", "CPU", "SSD"]));
        let s = settings_with(|s| s.temperature_unit = TemperatureUnit::Fahrenheit);
        let p = run(&raw, &pinned("P1"), &s, &State::default());
        assert_items(&p, &["cpu 163°F", "drive 100°F", "wifi 91°F", "fan 0rpm"]);
    });
}

/// The value text of the line of `identity` in the popup layout.
fn menu_value(raw: &[RawReading], settings: &Settings, identity: &str) -> Option<Text> {
    let m = menu::menu(
        &menu::shown(raw, settings),
        settings,
        Report::Answered,
        None,
        false,
    );
    let found = m.entries.iter().find_map(|e| match e {
        Entry::Line(i) if i.identity == identity => Some(i.value.clone()),
        _ => None,
    });
    found.expect("identity in the menu")
}

#[test]
fn c_core_panel_2() {
    puron_testkit::limit("C-core.panel-2", LIMIT, || {
        let raw = capture();
        let p1 = pinned("P1");
        let cases: [fn(&mut Settings); 7] = [
            |_| {},
            |s| s.temperature_unit = TemperatureUnit::Fahrenheit,
            |s| s.show_decimals = true,
            |s| s.show_temperature_unit = false,
            |s| s.show_rotation_rate_unit = false,
            |s| s.show_voltage_unit = false,
            |s| s.show_power_unit = false,
        ];
        for change in cases {
            let s = settings_with(change);
            let p = run(&raw, &p1, &s, &State::default());
            assert_eq!(p.items.len(), p1.len());
            let shown = menu::shown(&raw, &s);
            let lines = shown.lines().filter(|l| p1.contains(&l.identity));
            for (item, line) in p.items.iter().zip(lines) {
                let Item::Pin { value, .. } = item else {
                    panic!("pin item")
                };
                let identity = &line.identity;
                assert_eq!(value, &Some(menu_value(&raw, &s, identity)), "{identity}");
            }
        }
    });
}

#[test]
fn c_core_panel_3() {
    puron_testkit::limit("C-core.panel-3", LIMIT, || {
        let raw = capture();
        let on = run(&raw, &pinned("P1"), &Settings::default(), &State::default());
        assert_eq!(on.padding, 0);
        let off = settings_with(|s| s.show_panel_icons = false);
        let off = run(&raw, &pinned("P1"), &off, &State::default());
        assert_eq!(off.padding, 4);
    });
}

#[test]
fn c_core_panel_4() {
    puron_testkit::limit("C-core.panel-4", LIMIT, || {
        let raw = capture();
        assert!(pinned("P2").is_empty());
        assert_items(&lay(&raw, &pinned("P2")), &["empty"]);
        let off = settings_with(|s| s.show_panel_icons = false);
        assert_items(
            &run(&raw, &pinned("P2"), &off, &State::default()),
            &["empty"],
        );
    });
}

#[test]
fn c_core_panel_5() {
    puron_testkit::limit("C-core.panel-5", LIMIT, || {
        let raw = capture();
        let pins = ids(
            &raw,
            &["@maximum/lm-sensors/nvme-pci-0100", "WiFi", "@maximum/cpu"],
        );
        // The items follow popup order, not pinned order, and each item has
        // the icon of its device: the CPU group's Maximum line the CPU, the
        // drive's Maximum line the drive, and the Wi-Fi card's reading the
        // Wi-Fi sign.
        assert_items(&lay(&raw, &pins), &["cpu 73°C", "drive 45°C", "wifi 33°C"]);
    });
}

#[test]
fn c_core_panel_6() {
    puron_testkit::limit("C-core.panel-6", LIMIT, || {
        assert_items(&lay(&[], &pinned("P3")), &["∅ ⚠", "∅ ⚠"]);
    });
}

#[test]
fn c_core_panel_7() {
    puron_testkit::limit("C-core.panel-7", LIMIT, || {
        let p5 = pinned("P5");
        assert_eq!(p5, ids(&capture(), &["WiFi", "CPU"]));
        let s = Settings::default();
        let first = run(&snapshot("S1"), &p5, &s, &State::default());
        assert_items(&first, &["cpu 73°C", "wifi 33°C"]);
        let second = run(&snapshot("S3"), &p5, &s, &first.state);
        assert_items(&second, &["cpu 73°C", "wifi ⚠"]);
        let third = run(&snapshot("S1"), &p5, &s, &second.state);
        assert_items(&third, &["cpu 73°C", "wifi 33°C"]);
    });
}

#[test]
fn c_core_panel_8() {
    puron_testkit::limit("C-core.panel-8", LIMIT, || {
        let p5 = pinned("P5");
        let s = Settings::default();
        let first = run(&snapshot("S3"), &p5, &s, &State::default());
        assert_items(&first, &["cpu 73°C", "∅ ⚠"]);
        let second = run(&snapshot("S1"), &p5, &s, &first.state);
        assert_items(&second, &["cpu 73°C", "wifi 33°C"]);
    });
}

#[test]
fn c_core_panel_10() {
    puron_testkit::limit("C-core.panel-10", LIMIT, || {
        let p5 = pinned("P5");
        let on = Settings::default();
        let off = settings_with(|s| s.show_panel_icons = false);
        let first = run(&snapshot("S1"), &p5, &on, &State::default());
        let second = run(&snapshot("S3"), &p5, &off, &first.state);
        assert_items(&second, &["73°C", "⚠"]);
        let third = run(&snapshot("S3"), &p5, &on, &second.state);
        assert_items(&third, &["cpu 73°C", "∅ ⚠"]);
    });
}

#[test]
fn c_core_panel_11() {
    puron_testkit::limit("C-core.panel-11", LIMIT, || {
        // The pin is the Maximum line of chip puron-isa-0000. Sets D and C
        // have two temperatures on that chip, set F none. The item keeps
        // its icon through the three snapshots: a value, no value, then no
        // line at all.
        let pins = ["@maximum/lm-sensors/puron-isa-0000".to_string()];
        let s = Settings::default();
        let d = set("summary.json", "D");
        let first = run(&d, &pins, &s, &State::default());
        assert_items(&first, &["general 0°C"]);
        let second = run(&set("summary.json", "C"), &pins, &s, &first.state);
        assert_items(&second, &["general N/A"]);
        let third = run(&set("summary.json", "F"), &pins, &s, &second.state);
        assert_items(&third, &["general ⚠"]);
        let hidden = settings_with(|s| s.show_temperature = false);
        let fourth = run(&d, &pins, &hidden, &third.state);
        assert_items(&fourth, &["empty"]);
    });
}

#[test]
fn c_core_panel_12() {
    puron_testkit::limit("C-core.panel-12", LIMIT, || {
        let raw = set("summary.json", "B");
        let t1 = raw
            .iter()
            .find(|r| r.name.as_deref() == Some("t1"))
            .expect("t1");
        assert_eq!(t1.value, Some(-5.0));
        assert_items(&lay(&raw, &ids(&raw, &["t1"])), &["general N/A"]);
    });
}

#[test]
fn c_core_panel_13() {
    puron_testkit::limit("C-core.panel-13", LIMIT, || {
        let raw = capture();
        assert_eq!(pinned("P4"), ids(&raw, &["gone", "CPU"]));
        assert_items(&lay(&raw, &pinned("P4")), &["cpu 73°C", "∅ ⚠"]);
    });
}

#[test]
fn c_core_panel_14() {
    puron_testkit::limit("C-core.panel-14", LIMIT, || {
        let raw = capture();
        let p1 = pinned("P1");
        let first = run(&raw, &p1, &Settings::default(), &State::default());
        let fan_hidden = settings_with(|s| s.show_rotation_rate = false);
        let second = run(&raw, &p1, &fan_hidden, &first.state);
        assert_items(&second, &["cpu 73°C", "drive 38°C", "wifi 33°C"]);
        let third = run(&raw, &p1, &Settings::default(), &second.state);
        assert_items(&third, &["cpu 73°C", "drive 38°C", "wifi 33°C", "fan 0rpm"]);
        let temperature_hidden = settings_with(|s| s.show_temperature = false);
        let fourth = run(&raw, &pinned("P4"), &temperature_hidden, &third.state);
        assert_items(&fourth, &["∅ ⚠"]);
    });
}

#[test]
fn c_core_panel_15() {
    puron_testkit::limit("C-core.panel-15", LIMIT, || {
        let raw = snapshot("S1");
        let s = Settings::default();
        let first = run(&raw, &pinned("P5"), &s, &State::default());
        let cpu = ids(&raw, &["CPU"]);
        let second = run(&raw, &cpu, &s, &first.state);
        let held: Vec<&String> = second.state.icons.keys().collect();
        assert_eq!(held, [&cpu[0]]);
    });
}

#[test]
fn c_core_panel_16() {
    puron_testkit::limit("C-core.panel-16", LIMIT, || {
        #[derive(serde::Deserialize)]
        struct Kinds {
            #[serde(rename = "R")]
            readings: Vec<RawReading>,
            #[serde(rename = "P")]
            pinned: Vec<String>,
        }
        let k: Kinds = json("kinds.json");
        // A second reading of the drive gives it a Maximum line, which has
        // the drive's icon.
        let mut raw = k.readings;
        let drive = raw[2].clone();
        raw.push(RawReading {
            name: Some("d6".to_string()),
            key: "k6".to_string(),
            ..drive
        });
        let mut pins = k.pinned;
        pins.push("@maximum/lm-sensors/nvme-pci-0100".to_string());
        let p = lay(&raw, &pins);
        let icons: Vec<Icon> = p
            .items
            .iter()
            .map(|i| match i {
                Item::Pin { icon, .. } => *icon,
                Item::EmptyState => panic!("pin item"),
            })
            .collect();
        assert_eq!(
            icons,
            [
                IconKind::GpuTemperature,
                IconKind::DriveTemperature,
                IconKind::DriveTemperature,
                IconKind::GeneralTemperature,
                IconKind::Fan,
                IconKind::Voltage,
                IconKind::Power,
            ]
            .map(Icon::Kind)
        );
    });
}

#[test]
fn c_core_panel_17() {
    puron_testkit::limit("C-core.panel-17", LIMIT, || {
        let s1 = snapshot("S1");
        let s3 = snapshot("S3");
        let cpu = ids(&s1, &["CPU"]);
        let both = ids(&s1, &["CPU", "WiFi"]);
        let s = Settings::default();
        let first = run(&s1, &cpu, &s, &State::default());
        let second = run(&s1, &both, &s, &first.state);
        assert_items(&second, &["cpu 73°C", "wifi 33°C"]);
        let third = run(&s3, &cpu, &s, &second.state);
        let fourth = run(&s3, &both, &s, &third.state);
        assert_items(&fourth, &["cpu 73°C", "∅ ⚠"]);
    });
}

#[test]
fn c_core_panel_18() {
    puron_testkit::limit("C-core.panel-18", LIMIT, || {
        // Set D of dupes.json, two readings with the same key `k`, given
        // the values 50 and 60. The pin shows the first one's value.
        let mut raw = set("dupes.json", "D");
        assert_eq!(raw.len(), 2);
        raw[0].value = Some(50.0);
        raw[1].value = Some(60.0);
        assert_eq!(raw[0].name.as_deref(), Some("a"));
        let p = lay(&raw, &["temperature/lm-sensors/k".to_string()]);
        assert_items(&p, &["general 50°C"]);
    });
}

#[test]
fn c_core_panel_19() {
    puron_testkit::limit("C-core.panel-19", LIMIT, || {
        let text = std::fs::read_to_string(puron_testkit::fixture("raw", "cpu.json")).unwrap();
        let mut sets: std::collections::BTreeMap<String, Vec<puron_core::rows::RawReading>> =
            serde_json::from_str(&text).unwrap();
        let raw = sets.remove("example").unwrap();
        let pinned = vec!["@maximum/cpu".to_string()];
        let mut s = puron_core::settings::Settings::default();
        let value = |s: &puron_core::settings::Settings| {
            let shown = puron_core::menu::shown(&raw, s);
            let state = puron_core::panel::State::default();
            puron_core::panel::panel(&shown, &pinned, s, &state).items
        };
        // Temperatures shown: the pin of the CPU group's Maximum line has a
        // value.
        let items = value(&s);
        assert!(matches!(
            items.as_slice(),
            [puron_core::panel::Item::Pin {
                value: Some(Some(_)),
                ..
            }]
        ));
        // Temperatures hidden: the line is gone, the panel shows only the
        // empty-state item, and a click on another line keeps the pin.
        s.show_temperature = false;
        let shown = puron_core::menu::shown(&raw, &s);
        assert!(shown.line("@maximum/cpu").is_none());
        assert_eq!(value(&s), [puron_core::panel::Item::EmptyState]);
        let classes = puron_core::rows::ShownClasses {
            temperature: false,
            ..puron_core::rows::ShownClasses::ALL
        };
        let fan = "fan/lm-sensors/example_acpi-isa-0000/fan1".to_string();
        let after = puron_core::pins::click(&pinned, &fan, std::slice::from_ref(&fan), &classes);
        assert_eq!(after, ["@maximum/cpu".to_string(), fan]);
    });
}

/// A pinned line of a drive that lm-sensors reads, and the panel. While
/// drives are read, the pin has an item: with its value when the drive's
/// readings are there, and with no value when they are missing. While
/// drives are switched off (`provider_lm_sensors_drives`), the pin of the
/// reading and the pin of the drive's Maximum line have no item, and a
/// pin of another device keeps its own. With lm-sensors off, the switch
/// has no effect and the pin keeps its item.
#[test]
fn c_core_panel_20() {
    puron_testkit::limit("C-core.panel-20", LIMIT, || {
        let all = set("cpu.json", "sections");
        let chip = "nvme-pci-0100";
        let of_drive = |r: &RawReading| r.chip.as_deref().is_some_and(puron_core::rows::drive_chip);
        let reading = all
            .iter()
            .find(|r| r.chip.as_deref() == Some(chip))
            .expect("a reading of the drive");
        let drive = format!("temperature/lm-sensors/{}", reading.key);
        let maximum = format!("@maximum/lm-sensors/{chip}");
        // What the lm-sensors adapter gives while it skips drive chips.
        let skipped: Vec<RawReading> = all.iter().filter(|r| !of_drive(r)).cloned().collect();
        assert!(skipped.len() < all.len());
        let on = Settings::default();
        let off = settings_with(|s| s.provider_lm_sensors_drives = false);
        let items = |raw: &[RawReading], pinned: &[String], s: &Settings| {
            run(raw, pinned, s, &State::default()).items
        };
        let one = [drive.clone()];
        assert!(matches!(
            items(&all, &one, &on).as_slice(),
            [Item::Pin {
                value: Some(Some(_)),
                ..
            }]
        ));
        assert!(matches!(
            items(&skipped, &one, &on).as_slice(),
            [Item::Pin { value: None, .. }]
        ));
        assert_eq!(items(&skipped, &one, &off), [Item::EmptyState]);
        assert_eq!(
            items(&skipped, std::slice::from_ref(&maximum), &off),
            [Item::EmptyState]
        );
        let cpu = "@maximum/cpu".to_string();
        let three = [drive, cpu.clone(), maximum];
        assert_eq!(items(&skipped, &three, &off), items(&skipped, &[cpu], &off));
        assert_eq!(items(&skipped, &three, &off).len(), 1);
        assert_eq!(items(&skipped, &three, &on).len(), 3);
        // With lm-sensors itself off, the drive switch has no effect: the
        // pin keeps its item, with no value.
        let none = settings_with(|s| {
            s.provider_lm_sensors = false;
            s.provider_lm_sensors_drives = false;
        });
        assert!(matches!(
            items(&skipped, &one, &none).as_slice(),
            [Item::Pin { value: None, .. }]
        ));
    });
}

// The tests below are of the boxes: the box text that each pin item gets,
// whose width its value's box keeps on the panel, and the box memory that
// lets a box grow but never shrink. They use the readings of kinds.json,
// one of each class and family, all six pinned.

/// The identity of the fan reading of kinds.json.
const FAN: &str = "fan/lm-sensors/k3";

/// The readings and the pinned list of kinds.json.
fn kinds() -> (Vec<RawReading>, Vec<String>) {
    #[derive(serde::Deserialize)]
    struct Kinds {
        #[serde(rename = "R")]
        readings: Vec<RawReading>,
        #[serde(rename = "P")]
        pinned: Vec<String>,
    }
    let k: Kinds = json("kinds.json");
    (k.readings, k.pinned)
}

/// The readings of kinds.json with the fan at `rpm`.
fn fan_at(rpm: f64) -> Vec<RawReading> {
    let (mut raw, _) = kinds();
    let fan = raw.iter_mut().find(|r| r.key == "k3").expect("fan reading");
    fan.value = Some(rpm);
    raw
}

/// The readings of kinds.json without the fan.
fn no_fan() -> Vec<RawReading> {
    let (mut raw, _) = kinds();
    raw.retain(|r| r.key != "k3");
    raw
}

fn all_pins() -> Vec<String> {
    kinds().1
}

/// Each item as its value and its box text, such as `1234rpm 8888rpm`:
/// `⚠` for a pin with no line, `-` for no box text, and `empty` for the
/// empty-state item.
fn boxed(p: &Panel) -> Vec<String> {
    assert_eq!(p.boxes.len(), p.items.len(), "one box text per item");
    p.items
        .iter()
        .zip(&p.boxes)
        .map(|(i, b)| match i {
            Item::EmptyState => "empty".to_string(),
            Item::Pin { value, .. } => {
                let value = value.as_ref().map_or("⚠".to_string(), text);
                let b = b
                    .as_ref()
                    .map_or("-".to_string(), |t| text(&Some(t.clone())));
                format!("{value} {b}")
            }
        })
        .collect()
}

/// The fan's item, as `boxed` writes it, found by the unit of its box
/// text; `None` when no item has a box text in rpm.
fn fan_item(p: &Panel) -> Option<String> {
    let at = p
        .boxes
        .iter()
        .position(|b| b.as_ref().and_then(|t| t.unit) == Some(Unit::Rpm))?;
    Some(boxed(p)[at].clone())
}

/// Runs the pins of kinds.json with the fan at 12000 rpm, then at 0, and
/// returns the second panel, whose fan box has grown to five digits.
fn grown(settings: &Settings) -> Panel {
    let first = run(&fan_at(12000.0), &all_pins(), settings, &State::default());
    let second = run(&fan_at(0.0), &all_pins(), settings, &first.state);
    assert_eq!(fan_item(&second).as_deref(), Some("0rpm 88888rpm"));
    second
}

/// The fan's value goes 1234, 0, 12000, 0, 9999.9 rpm, with the state
/// passed on each time. Its box text starts at four digits, does not
/// shrink at 0, grows to five at 12000 and stays at five after it; a
/// 9999.9 rpm reading shows as 9999 and changes nothing. The other five
/// pins keep the box texts of their classes throughout.
#[test]
fn c_core_panel_21() {
    puron_testkit::limit("C-core.panel-21", LIMIT, || {
        let steps = [
            (1234.0, "1234rpm 8888rpm"),
            (0.0, "0rpm 8888rpm"),
            (12000.0, "12000rpm 88888rpm"),
            (0.0, "0rpm 88888rpm"),
            (9999.9, "9999rpm 88888rpm"),
        ];
        let mut state = State::default();
        for (rpm, fan) in steps {
            let p = run(&fan_at(rpm), &all_pins(), &Settings::default(), &state);
            assert_eq!(
                boxed(&p),
                [
                    "40°C 888°C",
                    "40°C 888°C",
                    "40°C 888°C",
                    fan,
                    "+12.00V +88.88V",
                    "+10.00W +888.88W",
                ],
                "fan at {rpm}"
            );
            state = p.state;
        }
    });
}

/// From the start state, the box text of each class: three digits for a
/// temperature, four for a fan, two for a voltage and three for a power,
/// with the sign, decimals and unit that the values of the class show
/// under the settings. Then pins with no line: the fan's and the CPU
/// group's Maximum line still get the box text of their class, which comes
/// from the pinned text, and a text that is no identity gets none.
#[test]
fn c_core_panel_22() {
    puron_testkit::limit("C-core.panel-22", LIMIT, || {
        let boxes = |s: &Settings| -> Vec<String> {
            let p = run(&kinds().0, &all_pins(), s, &State::default());
            p.boxes
                .iter()
                .map(|b| text(&Some(b.clone().expect("a box text"))))
                .collect()
        };
        assert_eq!(
            boxes(&Settings::default()),
            ["888°C", "888°C", "888°C", "8888rpm", "+88.88V", "+888.88W"]
        );
        let fahrenheit = settings_with(|s| {
            s.temperature_unit = TemperatureUnit::Fahrenheit;
            s.show_decimals = true;
        });
        assert_eq!(
            boxes(&fahrenheit),
            ["888.8°F", "888.8°F", "888.8°F", "8888rpm", "+88.88V", "+888.88W"]
        );
        let no_units = settings_with(|s| {
            s.show_temperature_unit = false;
            s.show_rotation_rate_unit = false;
            s.show_voltage_unit = false;
            s.show_power_unit = false;
        });
        assert_eq!(
            boxes(&no_units),
            ["888", "888", "888", "8888", "+88.88", "+888.88"]
        );
        let pins: Vec<String> = [FAN, "@maximum/cpu", "not-an-identity"]
            .map(String::from)
            .to_vec();
        let p = run(&[], &pins, &Settings::default(), &State::default());
        assert_eq!(boxed(&p), ["⚠ 8888rpm", "⚠ 888°C", "⚠ -"]);
    });
}

/// A setting's name, a change of it, and the fan item that follows.
type Change = (&'static str, fn(&mut Settings), &'static str);

/// From a state whose fan box has grown to five digits, a change of any
/// of the eight settings that change the value texts or their size starts
/// every box again: with the fan at 0 the box is back at four digits.
/// A change of another setting keeps the grown box. A start counts the
/// value shown at once, so 12000 rpm under a new text size keeps five
/// digits, and the next call with the same settings is no new start.
#[test]
fn c_core_panel_23() {
    puron_testkit::limit("C-core.panel-23", LIMIT, || {
        let base = Settings::default();
        let fan_box = |p: &Panel| boxed(p)[3].clone();
        let changes: [Change; 10] = [
            (
                "temperature_unit",
                |s| s.temperature_unit = TemperatureUnit::Fahrenheit,
                "0rpm 8888rpm",
            ),
            ("show_decimals", |s| s.show_decimals = true, "0rpm 8888rpm"),
            (
                "show_temperature_unit",
                |s| s.show_temperature_unit = false,
                "0rpm 8888rpm",
            ),
            (
                "show_rotation_rate_unit",
                |s| s.show_rotation_rate_unit = false,
                "0 8888",
            ),
            (
                "show_voltage_unit",
                |s| s.show_voltage_unit = false,
                "0rpm 8888rpm",
            ),
            (
                "show_power_unit",
                |s| s.show_power_unit = false,
                "0rpm 8888rpm",
            ),
            (
                "panel_text_size",
                |s| s.panel_text_size = TextSize::Px20,
                "0rpm 8888rpm",
            ),
            (
                "dock_text_size",
                |s| s.dock_text_size = TextSize::Px10,
                "0rpm 8888rpm",
            ),
            (
                "poll_interval_secs",
                |s| s.poll_interval_secs = 7,
                "0rpm 88888rpm",
            ),
            (
                "show_panel_icons",
                |s| s.show_panel_icons = false,
                "0rpm 88888rpm",
            ),
        ];
        for (key, change, want) in changes {
            let from = grown(&base);
            let mut s = base.clone();
            change(&mut s);
            let p = run(&fan_at(0.0), &all_pins(), &s, &from.state);
            assert_eq!(fan_box(&p), want, "after a change of {key}");
        }
        let px20 = settings_with(|s| s.panel_text_size = TextSize::Px20);
        let at_12000 = run(&fan_at(12000.0), &all_pins(), &px20, &grown(&base).state);
        assert_eq!(fan_box(&at_12000), "12000rpm 88888rpm");
        let again = run(&fan_at(0.0), &all_pins(), &px20, &at_12000.state);
        assert_eq!(fan_box(&again), "0rpm 88888rpm");
    });
}

/// From a state whose fan box has grown to five digits: (a) unpinned, the
/// fan gives no item and the state keeps no box for it; pinned again, at
/// the end of the list, its box is back at four digits. (b) Hidden with
/// the fan class switched off, the same; shown again, four digits.
/// (c) When the fan's reading disappears, its item shows "⚠" and keeps
/// the grown box, which it still has when the reading comes back at 0.
#[test]
fn c_core_panel_24() {
    puron_testkit::limit("C-core.panel-24", LIMIT, || {
        let s = Settings::default();
        let without: Vec<String> = all_pins().into_iter().filter(|id| id != FAN).collect();
        let a = run(&fan_at(0.0), &without, &s, &grown(&s).state);
        assert_eq!(fan_item(&a), None, "(a) a fan item while unpinned");
        assert!(
            !a.state.boxes.contains_key(FAN),
            "(a) a box kept while unpinned"
        );
        let mut back = without.clone();
        back.push(FAN.to_string());
        let a = run(&fan_at(0.0), &back, &s, &a.state);
        assert_eq!(
            fan_item(&a).as_deref(),
            Some("0rpm 8888rpm"),
            "(a) pinned again"
        );

        let hidden = settings_with(|s| s.show_rotation_rate = false);
        let b = run(&fan_at(0.0), &all_pins(), &hidden, &grown(&s).state);
        assert_eq!(b.items.len(), 5, "(b) the fan item while hidden");
        assert!(
            !b.state.boxes.contains_key(FAN),
            "(b) a box kept while hidden"
        );
        let b = run(&fan_at(0.0), &all_pins(), &s, &b.state);
        assert_eq!(
            fan_item(&b).as_deref(),
            Some("0rpm 8888rpm"),
            "(b) shown again"
        );

        let c = run(&no_fan(), &all_pins(), &s, &grown(&s).state);
        assert_eq!(fan_item(&c).as_deref(), Some("⚠ 88888rpm"), "(c) missing");
        let c = run(&fan_at(0.0), &all_pins(), &s, &c.state);
        assert_eq!(fan_item(&c).as_deref(), Some("0rpm 88888rpm"), "(c) back");
    });
}

/// From a state whose fan box has grown to five digits, every class is
/// switched off, so the panel shows only the empty-state item: the state
/// then keeps no box at all. With every class shown again and the fan at
/// 0, the fan's box is back at four digits.
#[test]
fn c_core_panel_24__empty_state() {
    puron_testkit::limit("C-core.panel-24", LIMIT, || {
        let s = Settings::default();
        let none_shown = settings_with(|s| {
            s.show_temperature = false;
            s.show_rotation_rate = false;
            s.show_voltage = false;
            s.show_power = false;
        });
        let empty = run(&fan_at(0.0), &all_pins(), &none_shown, &grown(&s).state);
        assert_eq!(boxed(&empty), ["empty"]);
        assert!(
            empty.state.boxes.is_empty(),
            "boxes kept: {:?}",
            empty.state.boxes
        );
        let back = run(&fan_at(0.0), &all_pins(), &s, &empty.state);
        assert_eq!(fan_item(&back).as_deref(), Some("0rpm 8888rpm"));
    });
}

/// The pinned list `one-each` of icons.json: one reading of each kind of
/// device, then a fan, a voltage and a power reading, by short name.
const ONE_EACH: [&str; 10] = ["cp", "ga", "dn", "w1", "e1", "m1", "o1", "fm", "vw", "pa"];

/// The items as their icon words, with ` ⚠` after a pin that has no line.
fn icon_words(p: &Panel) -> Vec<String> {
    p.items
        .iter()
        .map(|i| match i {
            Item::Pin {
                icon: Icon::Kind(k),
                value,
            } => format!("{}{}", kind(*k), if value.is_none() { " ⚠" } else { "" }),
            other => format!("{other:?}"),
        })
        .collect()
}

/// With one pin of each kind of icons.json, each item has the icon of its
/// device, not of its family: the CPU, Wi-Fi, wired network and memory
/// pins get their own kinds although their family is the general one.
/// When the Wi-Fi and memory readings then disappear, their items show
/// `⚠` and keep the icons of those kinds, which the icon memory holds.
#[test]
fn c_core_panel_25() {
    puron_testkit::limit("C-core.panel-25", LIMIT, || {
        let raw = set("icons.json", "kinds");
        let pins = ids(&raw, &ONE_EACH);
        let want = [
            "cpu", "gpu", "drive", "wifi", "wired", "memory", "general", "fan", "voltage", "power",
        ];
        let s = Settings::default();
        let first = run(&raw, &pins, &s, &State::default());
        // The items follow the popup's order; this puts the expected kinds
        // of the pins in that order.
        let shown = menu::shown(&raw, &s);
        let mut order: Vec<usize> = (0..pins.len()).collect();
        order.sort_by_key(|&i| shown.place(&pins[i]).expect("pinned line"));
        let in_order: Vec<String> = order.iter().map(|&i| want[i].to_string()).collect();
        assert_eq!(icon_words(&first), in_order);
        for (pin, k) in pins.iter().zip(want) {
            assert_eq!(
                first.state.icons.get(pin).map(|k| kind(*k)),
                Some(k),
                "{pin}"
            );
        }
        let mut fewer = raw.clone();
        fewer.retain(|r| !matches!(r.name.as_deref(), Some("w1" | "m1")));
        assert_eq!(fewer.len(), raw.len() - 2);
        let second = run(&fewer, &pins, &s, &first.state);
        // The missing pins come last, in the pinned list's order.
        let mut want_second: Vec<String> = order
            .iter()
            .filter(|&&i| !["w1", "m1"].contains(&ONE_EACH[i]))
            .map(|&i| want[i].to_string())
            .collect();
        want_second.extend(["wifi ⚠".to_string(), "memory ⚠".to_string()]);
        assert_eq!(icon_words(&second), want_second);
    });
}
