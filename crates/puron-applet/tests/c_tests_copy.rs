//! Tests of "Copy details" on the About page: the exact text it copies,
//! that the text holds nothing private, and what a click on the button
//! does. They are kept apart from the page's other tests, as the button's
//! code is, so that the button can be removed alone.
#![allow(non_snake_case)]

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use common::*;
use puron_applet::about::{self, Clicked, Entry, Package, Source};
use puron_applet::app::{App, Event, Output, Page, Programs};
use puron_applet::copy;
use puron_applet::i18n::{self, test_catalog};
use puron_applet::popup::{Event as PopupEvent, Reply};
use puron_core::rows::{Provider, RawReading};
use puron_engine::engine::{LmReport, ProviderState, Snapshot};
use puron_testkit::{cfg, eng, loc};

const LIMIT: u64 = 5;
const CHILD: &str = "PURON_COPY_CHILD";

/// Writes the kernel release file `name` of the About fixtures into
/// `dir` and returns its path; K5 is a file that does not exist.
fn kernel_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let body: Option<String> = match name {
        "K1" => Some("6.12.115+deb13.1-amd64\n".into()),
        "K2" => Some("6.1.0\nnode-a\n".into()),
        "K3" => Some("6.1.0 node-a\n".into()),
        "K4" => Some(format!("{}\n", "a".repeat(65))),
        "K5" => None,
        "K6" => Some(String::new()),
        "K7" => Some("6.1.0-rc1~bpo_2+x\n".into()),
        _ => panic!("no kernel file {name}"),
    };
    if let Some(b) = body {
        std::fs::write(&path, b).expect("write a kernel file");
    }
    path
}

/// The copy outputs of an event.
fn copies(out: &[Output]) -> Vec<String> {
    out.iter()
        .filter_map(|o| match o {
            Output::Copy(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn is_child() -> bool {
    std::env::var_os(CHILD).is_some()
}

fn env_var(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// The copied text for version 1.6.0 with kernel K1: first for the
/// Flatpak and the source set `flatpak-on`, then for the Debian package
/// with every provider off. Expected: exactly the lines of the page's
/// format, each ended by a line feed.
#[test]
fn c_tests_45() {
    puron_testkit::limit("C-tests-45", LIMIT, || {
        let dir = puron_testkit::temp_dir("c-tests-45");
        let kernel = copy::kernel_release(&kernel_file(&dir, "K1"));
        let on = [
            Source::Available(12),
            Source::Available(2),
            Source::NotAvailable,
            Source::Available(1),
            Source::NotAvailable,
        ];
        assert_eq!(
            copy::text("1.6.0", Package::Flatpak, &kernel, &on),
            "Puron 1.6.0\n\
             Package: Flatpak\n\
             App ID: io.github.elalonde.Puron\n\
             Kernel: 6.12.115+deb13.1-amd64\n\
             lm-sensors: available, 12 readings\n\
             NVMe: available, 2 readings\n\
             NVIDIA: not available\n\
             udisks2: available, 1 reading\n\
             liquidctl: not available in the Flatpak\n"
        );
        assert_eq!(
            copy::text("1.6.0", Package::Deb, &kernel, &[Source::Off; 5]),
            "Puron 1.6.0\n\
             Package: Debian package\n\
             App ID: io.github.elalonde.Puron\n\
             Kernel: 6.12.115+deb13.1-amd64\n\
             lm-sensors: off\n\
             NVMe: off\n\
             NVIDIA: off\n\
             udisks2: off\n\
             liquidctl: off\n"
        );
    });
}

/// With Esperanto asked for (map E1) and the test catalog whose texts are
/// the English ones after "EO ", Copy details through the state machine.
/// Expected: one copy whose text starts with the crate's version and holds
/// no "EO ": the copied text never comes from the catalog.
#[test]
fn c_tests_45__catalog() {
    puron_testkit::limit("C-tests-45", LIMIT, || {
        if !is_child() {
            let mut cmd = puron_testkit::rerun_self("c_tests_45__catalog");
            for v in loc::VARS {
                cmd.env_remove(v);
            }
            for (k, v) in loc::env("E1") {
                cmd.env(k, v);
            }
            let out = cmd.env(CHILD, "1").output().expect("run child");
            assert!(
                out.status.success(),
                "child: {}\n{}{}",
                out.status,
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            return;
        }
        assert_eq!(
            i18n::start_with(test_catalog::eo(), env_var).to_string(),
            "eo"
        );
        let dir = puron_testkit::temp_dir("c-tests-45-eo");
        let c = cfg::build("good");
        let mut a = app(&c.root, &handle(0)).with_kernel_release_file(kernel_file(&dir, "K1"));
        a.handle(Event::Start);
        a.handle(Event::About);
        let got = copies(&a.handle(Event::CopyDetails));
        assert_eq!(got.len(), 1);
        assert!(
            got[0].starts_with(&format!("Puron {}\n", about::VERSION)),
            "{:?}",
            got[0]
        );
        assert!(!got[0].contains("EO "), "{:?}", got[0]);
    });
}

/// A reading of the private-looking snapshot.
fn private_reading(
    provider: Provider,
    position: u32,
    chip: Option<&str>,
    device: Option<&str>,
    key: &str,
    name: &str,
) -> RawReading {
    let mut r = eng::reading(provider, position, name, 40.0);
    r.chip = chip.map(str::to_string);
    r.device = device.map(str::to_string);
    r.key = key.to_string();
    r
}

/// The node name of this host, from `uname`.
fn node_name() -> String {
    // SAFETY: `uname` fills the zeroed struct; its fields end with NUL.
    unsafe {
        let mut u: libc::utsname = std::mem::zeroed();
        assert_eq!(libc::uname(&mut u), 0);
        std::ffi::CStr::from_ptr(u.nodename.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

/// The name of the user who runs the test.
fn user_name() -> String {
    // SAFETY: getpwuid returns a pointer to a static record or null.
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() {
            return String::new();
        }
        std::ffi::CStr::from_ptr((*pw).pw_name)
            .to_string_lossy()
            .into_owned()
    }
}

/// The state machine after a snapshot whose names carry private-looking
/// values (a serial in a device name, a WWN, a MAC address, a configured
/// label) with one of them pinned, then Copy details with kernel K1 and
/// with each of K2 to K7. Expected: the copied text is exactly the
/// summary of the format, and neither it nor the About plan holds any of
/// those values, the host's node name, the user's name or the home
/// directory; the copied text holds no "/"; the kernel line follows the
/// rule for each file.
#[test]
fn c_tests_46() {
    puron_testkit::limit("C-tests-46", LIMIT, || {
        puron_applet::start();
        let dir = puron_testkit::temp_dir("c-tests-46");
        let chip = Some("puron-isa-0000");
        let nvme = Some("Acme NVMe 4TB S5GXNF0R123456");
        let snap = Snapshot {
            readings: vec![
                private_reading(Provider::LmSensors, 0, chip, None, "temp1", "Office desk"),
                private_reading(
                    Provider::LmSensors,
                    1,
                    chip,
                    None,
                    "temp2",
                    "wwn-0x5000c500a1b2c3d4",
                ),
                private_reading(
                    Provider::LmSensors,
                    2,
                    chip,
                    None,
                    "temp3",
                    "aa:bb:cc:dd:ee:ff",
                ),
                private_reading(Provider::NvmeCli, 0, None, nvme, "Composite", "Composite"),
                private_reading(Provider::NvmeCli, 1, None, nvme, "Sensor 1", "Sensor 1"),
            ],
            lm_sensors: LmReport::Answered,
            providers: vec![
                (Provider::LmSensors, ProviderState::Available),
                (Provider::Liquidctl, ProviderState::Off),
                (Provider::Nvidia, ProviderState::Off),
                (Provider::Udisks2, ProviderState::Off),
                (Provider::NvmeCli, ProviderState::Available),
            ],
        };
        let c = cfg::build("good");
        let mut a: App = app(&c.root, &handle(0));
        a.handle(Event::Start);
        a.handle(Event::Snapshot(snap));
        let pin = "temperature/lm-sensors/puron-isa-0000/temp1".to_string();
        a.handle(Event::PinClick(pin));
        a.handle(Event::About);
        let mut a = a.with_kernel_release_file(kernel_file(&dir, "K1"));
        let got = copies(&a.handle(Event::CopyDetails));
        assert_eq!(got.len(), 1);
        let text = got[0].clone();
        assert_eq!(
            text,
            format!(
                "Puron {}\n\
                 Package: Built from source\n\
                 App ID: io.github.elalonde.Puron\n\
                 Kernel: 6.12.115+deb13.1-amd64\n\
                 lm-sensors: available, 3 readings\n\
                 NVMe: available, 2 readings\n\
                 NVIDIA: off\n\
                 udisks2: off\n\
                 liquidctl: off\n",
                about::VERSION
            )
        );
        assert!(!text.contains('/'), "{text:?}");
        let plan = about::plan(&a.about_input());
        let mut drawn: Vec<String> = Vec::new();
        for e in &plan.entries {
            drawn.push(format!("{e:?}"));
        }
        let mut private: Vec<String> = [
            "Acme",
            "S5GXNF0R123456",
            "wwn-",
            "aa:bb:cc",
            "Office desk",
            "puron-isa-0000",
            "temp1",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for s in [node_name(), user_name()] {
            if s.chars().count() >= 4 {
                private.push(s);
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            private.push(home.to_string_lossy().into_owned());
        }
        for p in &private {
            assert!(!text.contains(p.as_str()), "the copied text holds {p:?}");
            for d in &drawn {
                assert!(!d.contains(p.as_str()), "the About plan holds {p:?}: {d}");
            }
        }
        for (name, want) in [
            ("K2", "6.1.0"),
            ("K3", "unknown"),
            ("K4", "unknown"),
            ("K5", "unknown"),
            ("K6", "unknown"),
            ("K7", "6.1.0-rc1~bpo_2+x"),
        ] {
            let mut b = a.with_kernel_release_file(kernel_file(&dir, name));
            let got = copies(&b.handle(Event::CopyDetails));
            let line = got[0].lines().nth(3).unwrap_or_default().to_string();
            assert_eq!(line, format!("Kernel: {want}"), "{name}");
            a = b;
        }
    });
}

/// The About page's last entry is the button "Copy details"; a click on
/// it sends Copy details and leaves the popup open. Through the state
/// machine on the About page, with the fake opener on the search path:
/// exactly one copy output with a text, no program started, and the page
/// stays the About page.
#[test]
fn c_tests_47() {
    let file = puron_testkit::temp_dir("c-tests-47").join("args.txt");
    std::env::set_var("PURON_TEST_ARGFILE", &file);
    puron_testkit::limit("C-tests-47", 10, move || {
        puron_applet::start();
        let dir = puron_testkit::temp_dir("c-tests-47-kernel");
        let c = cfg::build("good");
        let open = puron_testkit::workspace_root().join("crates/puron-testkit/fixtures/prog/open");
        let programs = Programs {
            settings_dir: open.clone(),
            search_path: open.into_os_string(),
        };
        let mut a = app_with(&c.root, &handle(0), programs)
            .with_kernel_release_file(kernel_file(&dir, "K1"));
        a.handle(Event::Start);
        a.handle(Event::About);
        let plan = about::plan(&a.about_input());
        assert_eq!(
            plan.entries.last(),
            Some(&Entry::Copy {
                text: "Copy details".to_string()
            })
        );
        assert_eq!(
            about::clicked(&plan, Clicked::Copy),
            Reply {
                event: Some(PopupEvent::CopyDetails),
                close: false
            }
        );
        let out = a.handle(Event::CopyDetails);
        let got = copies(&out);
        assert_eq!(got.len(), 1, "{out:?}");
        assert!(!got[0].is_empty());
        assert!(
            !out.iter().any(|o| matches!(o, Output::Launch(..))),
            "{out:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!file.exists(), "the opener was started");
        assert_eq!(a.page(), Page::About);
    });
}
