//! The four links of the About page: their texts and addresses in order,
//! the reply to a click on each, and the launch of the browser opener
//! with the address. The search path holds only a fake `xdg-open` that
//! writes its arguments to a file. The test has its own binary because it
//! sets an environment variable, and a time limit of 10 s because it
//! starts processes.
#![allow(non_snake_case)]

mod common;

use std::time::Duration;

use common::*;
use puron_applet::about::{self, Clicked, Entry, Input, Package, Source};
use puron_applet::app::{Event, Launched, Output, Programs};
use puron_applet::popup::{Event as PopupEvent, Reply};
use puron_core::menu::HOMEPAGE_URL;
use puron_testkit::cfg;

/// The text of the metainfo's `url` element of type `kind`.
fn metainfo_url(xml: &str, kind: &str) -> String {
    let open = format!("<url type=\"{kind}\">");
    let start = xml.find(&open).unwrap_or_else(|| panic!("no url {kind}")) + open.len();
    let end = start + xml[start..].find("</url>").expect("end of url");
    xml[start..end].to_string()
}

/// The About plan for version 1.6.0. Expected: the links Homepage, Source
/// code, Report a problem and Third-party notices with their addresses,
/// the last at the release tag; the first three equal to the metainfo's
/// addresses and the homepage to the temperatures' homepage. A click on
/// each sends Open link with its address and closes the popup, and the
/// state machine starts the opener with exactly that address.
#[test]
fn c_tests_39() {
    let file = puron_testkit::temp_dir("c-tests-39").join("args.txt");
    std::env::set_var("PURON_TEST_ARGFILE", &file);
    puron_testkit::limit("C-tests-39", 10, move || {
        common::start_in_english();
        let plan = about::plan(&Input {
            version: "1.6.0".to_string(),
            package: Package::Deb,
            sources: [Source::Off; 5],
        });
        let links: Vec<(String, String)> = plan
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Link { text, url } => Some((text.clone(), url.clone())),
                _ => None,
            })
            .collect();
        let want = [
            ("Homepage", "https://elalonde.dev/projects/puron/"),
            ("Source code", "https://github.com/elalonde/puron"),
            (
                "Report a problem",
                "https://github.com/elalonde/puron/issues",
            ),
            (
                "Third-party notices",
                "https://github.com/elalonde/puron/blob/v1.6.0/README.md#license",
            ),
        ];
        let want: Vec<(String, String)> = want
            .iter()
            .map(|(t, u)| (t.to_string(), u.to_string()))
            .collect();
        assert_eq!(links, want);
        let xml = std::fs::read_to_string(
            puron_testkit::workspace_root().join("data/io.github.elalonde.Puron.metainfo.xml"),
        )
        .expect("read the metainfo");
        assert_eq!(links[0].1, HOMEPAGE_URL);
        assert_eq!(links[0].1, metainfo_url(&xml, "homepage"));
        assert_eq!(links[1].1, metainfo_url(&xml, "vcs-browser"));
        assert_eq!(links[2].1, metainfo_url(&xml, "bugtracker"));

        let c = cfg::build("good");
        let open = puron_testkit::workspace_root().join("crates/puron-testkit/fixtures/prog/open");
        let programs = Programs {
            search_path: open.into_os_string(),
        };
        let mut a = app_with(&c.root, &handle(0), programs);
        a.handle(Event::Start);
        for (_, url) in &links {
            assert_eq!(
                about::clicked(&plan, Clicked::Link(url.clone())),
                Reply {
                    event: Some(PopupEvent::OpenLink(url.clone())),
                    close: true
                }
            );
            let _ = std::fs::remove_file(&file);
            let out = a.handle(Event::OpenLink(url.clone()));
            assert!(
                out.iter()
                    .any(|o| matches!(o, Output::Launch(_, Launched::Started))),
                "{out:?}"
            );
            let got = loop {
                let s = std::fs::read_to_string(&file).unwrap_or_default();
                if s.ends_with('\n') {
                    break s;
                }
                std::thread::sleep(Duration::from_millis(5));
            };
            assert_eq!(got, format!("{url}\n"));
        }
    });
}
