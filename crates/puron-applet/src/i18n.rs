//! The translated texts: the Fluent catalogs, the choice of language at
//! start-up, and the lookup of a message. This file is compiled into
//! `puron-applet`, the one program. The crate's `i18n.toml` gives the
//! catalog directory, `i18n/` at the workspace root, the domain `puron`
//! and the fallback language `en`, which the `fl!` macro reads at build
//! time.

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::sync::OnceLock;

use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::unic_langid::LanguageIdentifier;
use i18n_embed::{I18nAssets, LanguageLoader};
use puron_core::format::{Text as ValueText, Unit};

/// The catalogs' domain: each catalog file is `<language>/puron.ftl`.
pub const DOMAIN: &str = "puron";
/// The language used when no other one matches, and for any message
/// that the chosen catalog lacks.
pub const FALLBACK: &str = "en";

/// The shipped catalogs, `i18n/<language>/puron.ftl` at the workspace
/// root. rust-embed embeds them in a release build; a debug build reads
/// them from that folder at run time.
#[derive(rust_embed::RustEmbed)]
#[folder = "../../i18n/"]
pub struct Catalogs;

/// A source of catalog files, as `start_with` takes it: the shipped
/// catalogs, or test catalogs.
pub type Assets = Box<dyn I18nAssets + Send + Sync + 'static>;

struct State {
    loader: FluentLanguageLoader,
    /// English alone, from the same catalogs as `loader`. `text` takes a
    /// message from here when the chosen catalog holds it but cannot give
    /// it, so that a reader never sees a half-formatted text.
    english: FluentLanguageLoader,
    #[allow(dead_code)]
    assets: Assets,
}

static STATE: OnceLock<State> = OnceLock::new();

fn english() -> LanguageIdentifier {
    FALLBACK.parse().expect("fallback tag")
}

/// A loader for the `puron` domain, with English as its fallback and no
/// catalog loaded yet.
pub fn new_loader() -> FluentLanguageLoader {
    FluentLanguageLoader::new(DOMAIN, english())
}

/// Turns one locale value from the environment, such as `de_DE.UTF-8`
/// or `sr_RS@latin`, into a language tag: everything from the first `.`
/// or `@` is dropped and `_` becomes `-`. `None` when the value is not
/// UTF-8 or the rest does not parse as a tag.
fn tag(value: &OsStr) -> Option<LanguageIdentifier> {
    let v = value.to_str()?;
    let v = v.split(['.', '@']).next().unwrap_or("");
    v.replace('_', "-").parse().ok()
}

/// The languages to try, in order of preference, read through `get`
/// from the usual locale variables. An empty variable counts as unset.
///
/// The locale is the first of `LC_ALL`, `LC_MESSAGES` and `LANG` that is
/// set. With no locale, or the locale `C`, `C.<encoding>` or `POSIX`, the
/// list is English alone and `LANGUAGE` is ignored. Otherwise the list is
/// the entries of `LANGUAGE`, a colon-separated list, or the locale when
/// `LANGUAGE` has no entry. Entries that are not language tags are
/// dropped; when none is left, the list is English alone.
pub fn languages(get: impl Fn(&str) -> Option<OsString>) -> Vec<LanguageIdentifier> {
    let set = |name: &str| get(name).filter(|v| !v.is_empty());
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|name| set(name));
    let Some(locale) = locale else {
        return vec![english()];
    };
    let c = locale
        .to_str()
        .is_some_and(|l| l == "C" || l == "POSIX" || l.starts_with("C."));
    if c {
        return vec![english()];
    }
    let mut values: Vec<OsString> = set("LANGUAGE")
        .map(|v| {
            v.to_string_lossy()
                .split(':')
                .filter(|e| !e.is_empty())
                .map(OsString::from)
                .collect()
        })
        .unwrap_or_default();
    if values.is_empty() {
        values.push(locale);
    }
    let list: Vec<LanguageIdentifier> = values.iter().filter_map(|v| tag(v)).collect();
    if list.is_empty() {
        vec![english()]
    } else {
        list
    }
}

/// Whether `path` is a catalog that the program reads: `<name>/puron.ftl`
/// where `<name>` is a language tag written exactly as the tag prints,
/// such as `de` or `pt-BR`. `pt_BR`, `PT-br` or a folder of notes are not.
fn is_catalog(path: &str) -> bool {
    match path.split_once('/') {
        // A tag compares equal to a text when it prints as that text.
        Some((name, "puron.ftl")) => name.parse::<LanguageIdentifier>().is_ok_and(|l| l == name),
        _ => false,
    }
}

/// A view of a catalog source that lists only the files for which
/// `is_catalog` holds and passes every read through. i18n_embed refuses
/// every language when one folder name is not a language tag, so without
/// this view a single misnamed folder of a translator would leave the
/// program with no text at all, English included.
struct Tagged<'a>(&'a dyn I18nAssets);

impl I18nAssets for Tagged<'_> {
    fn get_files(&self, file_path: &str) -> Vec<Cow<'_, [u8]>> {
        self.0.get_files(file_path)
    }
    fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(self.0.filenames_iter().filter(|f| is_catalog(f)))
    }
}

/// Loads into `loader` the catalogs of `assets` that i18n_embed's
/// negotiation picks for `list`, and returns the first language picked.
/// When nothing in `list` matches a catalog, it loads English and returns
/// English. A folder that is not named as a language tag is ignored.
pub fn choose(
    loader: &FluentLanguageLoader,
    assets: &dyn I18nAssets,
    list: &[LanguageIdentifier],
) -> LanguageIdentifier {
    let assets = &Tagged(assets);
    let chosen = i18n_embed::select(loader, assets, list).unwrap_or_default();
    match chosen.first() {
        Some(l) => l.clone(),
        None => {
            let _ = i18n_embed::select(loader, assets, &[english()]);
            english()
        }
    }
}

/// Builds the process-wide state from `assets`: the loader of the
/// languages that `list` asks for, and English alone. Returns the state
/// and the chosen language.
fn set_up(assets: Assets, list: &[LanguageIdentifier]) -> (State, LanguageIdentifier) {
    let loader = new_loader();
    let chosen = choose(&loader, assets.as_ref(), list);
    let fallback = new_loader();
    choose(&fallback, assets.as_ref(), &[english()]);
    let state = State {
        loader,
        english: fallback,
        assets,
    };
    (state, chosen)
}

/// Sets up the process-wide loader from `assets` and the locale variables
/// that `get` reads, and returns the chosen language. Only the first set-up
/// in a process counts: once `start_with` or `loader` has run, a later
/// call changes nothing and returns the language already loaded.
pub fn start_with(assets: Assets, get: impl Fn(&str) -> Option<OsString>) -> LanguageIdentifier {
    let mut chosen = None;
    let state = STATE.get_or_init(|| {
        let (state, language) = set_up(assets, &languages(get));
        chosen = Some(language);
        state
    });
    chosen.unwrap_or_else(|| state.loader.current_language())
}

/// `start_with` the shipped catalogs and the process environment.
pub fn start() -> LanguageIdentifier {
    start_with(Box::new(Catalogs), |name| std::env::var_os(name))
}

/// The process-wide state. When no `start` has run, the first call sets
/// it up with English alone.
fn state() -> &'static State {
    STATE.get_or_init(|| set_up(Box::new(Catalogs), &[english()]).0)
}

/// The process-wide loader. When no `start` has run, the first call sets
/// it up with English alone.
pub fn loader() -> &'static FluentLanguageLoader {
    &state().loader
}

/// The text of the message with the literal ID `$id`, as `text` gives it.
/// The closure is never called: it is there so that `i18n_embed_fl::fl!`
/// still fails the build when the English catalog has no message with
/// that ID.
macro_rules! fl {
    ($id:literal) => {{
        let _ = || i18n_embed_fl::fl!($crate::i18n::loader(), $id);
        $crate::i18n::text($id)
    }};
}

/// The message `id` in the chosen language, or in English when the chosen
/// catalog lacks it. It is English too when the chosen catalog holds the
/// message but cannot give it: a message with no text of its own (only
/// attributes), or one that names a variable, a term or another message
/// that does not exist. Fluent would then write the missing name in
/// braces into the text, such as `{-links}`. The English catalog, which
/// every build checks, never needs this second lookup. Unlike `fl!`, the
/// ID is not checked at build time.
pub fn text(id: &str) -> String {
    let state = state();
    state
        .loader
        .with_fluent_message_and_bundle(id, |message, bundle| {
            let mut errors = Vec::new();
            let text = bundle.format_pattern(message.value()?, None, &mut errors);
            errors.is_empty().then(|| text.into_owned())
        })
        .flatten()
        .unwrap_or_else(|| state.english.get(id))
}

/// The text written after a value of `unit`, such as `°C` or ` rpm`,
/// from the catalog. Any space belongs to the suffix.
fn suffix(unit: Unit) -> String {
    match unit {
        Unit::Celsius => fl!("suffix-celsius"),
        Unit::Fahrenheit => fl!("suffix-fahrenheit"),
        Unit::Rpm => fl!("suffix-rpm"),
        Unit::Volt => fl!("suffix-volt"),
        Unit::Watt => fl!("suffix-watt"),
    }
}

/// The text of a reading's value: the number as `puron_core::format`
/// wrote it, followed by the unit's suffix when the value has a unit.
/// `None`, a reading without a value, gives the catalog's `no-value`
/// text. The number is joined to the suffix here; the catalog never sees
/// it as an argument, so the text holds none of the Unicode isolation
/// marks that Fluent puts around arguments.
pub fn value_text(value: Option<&ValueText>) -> String {
    match value {
        None => fl!("no-value"),
        Some(t) => {
            let mut s = t.number.clone();
            if let Some(unit) = t.unit {
                s.push_str(&suffix(unit));
            }
            s
        }
    }
}

/// The text of a pin's value on the panel. The outer `None` means the
/// pin has no line in the popup and gives the catalog's `missing-value`
/// text; otherwise the text is `value_text` of the inner value.
pub fn panel_value(value: Option<Option<&ValueText>>) -> String {
    match value {
        None => fl!("missing-value"),
        Some(v) => value_text(v),
    }
}

/// Catalogs for tests, in Esperanto (`eo`), a language the program does
/// not ship: a full one, one that lacks a message, and one with the
/// faults a translator's file may have. Each is served together with the
/// shipped English catalog alone, so that English is there to fall back
/// on and no test reads a shipped translation, which a contributor may
/// change at any time. This module is built only with the feature
/// `test-catalog`.
#[cfg(feature = "test-catalog")]
pub mod test_catalog {
    use std::borrow::Cow;

    use i18n_embed::{AssetsMultiplexor, I18nAssets};

    use super::{Assets, Catalogs};

    const ENGLISH: &str = "en/puron.ftl";
    const ESPERANTO: &str = "eo/puron.ftl";

    /// The full test catalog, embedded from the testkit's
    /// `fixtures/loc/eo/` and served as `eo/puron.ftl`.
    #[derive(rust_embed::RustEmbed)]
    #[folder = "../puron-testkit/fixtures/loc/eo/"]
    #[prefix = "eo/"]
    pub struct Eo;

    /// The shipped catalogs reduced to `en/puron.ftl`. With every shipped
    /// catalog served, a test that asks for French before Esperanto would
    /// get a shipped French catalog instead of the test one.
    pub struct EnglishOnly(Catalogs);

    impl I18nAssets for EnglishOnly {
        fn get_files(&self, file_path: &str) -> Vec<Cow<'_, [u8]>> {
            if file_path == ENGLISH {
                self.0.get_files(file_path)
            } else {
                Vec::new()
            }
        }
        fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
            Box::new(self.0.filenames_iter().filter(|f| f == ENGLISH))
        }
    }

    /// Catalog files held in memory, each a path such as `eo/puron.ftl`
    /// and its bytes, built by `eo_partial` and `eo_broken`.
    pub struct Files(Vec<(String, Vec<u8>)>);

    impl I18nAssets for Files {
        fn get_files(&self, file_path: &str) -> Vec<Cow<'_, [u8]>> {
            self.0
                .iter()
                .filter(|(path, _)| path == file_path)
                .map(|(_, data)| Cow::Borrowed(data.as_slice()))
                .collect()
        }
        fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
            Box::new(self.0.iter().map(|(path, _)| path.clone()))
        }
    }

    /// `assets` together with the shipped English catalog alone.
    fn with_english(assets: Assets) -> Assets {
        Box::new(AssetsMultiplexor::new([
            assets,
            Box::new(EnglishOnly(Catalogs)) as Assets,
        ]))
    }

    /// The text of the full test catalog.
    fn eo_text() -> String {
        let full = <Eo as rust_embed::RustEmbed>::get(ESPERANTO).expect("X-loc-eo");
        String::from_utf8(full.data.to_vec()).expect("UTF-8")
    }

    /// The full `eo` test catalog, together with the English catalog.
    pub fn eo() -> Assets {
        with_english(Box::new(Eo))
    }

    /// The `eo` test catalog without the message `link-homepage`,
    /// together with the English catalog. It panics unless exactly one
    /// line was removed.
    pub fn eo_partial() -> Assets {
        let text = eo_text();
        let kept: String = text
            .lines()
            .filter(|l| !l.starts_with("link-homepage ="))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(kept.lines().count() + 1, text.lines().count());
        with_english(Box::new(Files(vec![(
            ESPERANTO.to_string(),
            kept.into_bytes(),
        )])))
    }

    /// The `eo` test catalog with the faults that a translator's file may
    /// have, one per message: `link-homepage` missing, `no-value` naming
    /// a variable that no caller gives, `about-back` without its `=`,
    /// `about-license` with a brace that is never closed, `about-links`
    /// naming a term that does not exist, `about-package` with an
    /// attribute and no text, and at the end a message and a term that
    /// the English catalog does not have. Served with it are the full
    /// test catalog under the folder `not a tag`, which is no language
    /// tag, and the English catalog. It panics unless each replaced line
    /// is found exactly once.
    pub fn eo_broken() -> Assets {
        let edits: [(&str, &[&str]); 6] = [
            ("link-homepage = EO About", &[]),
            ("no-value = EO N/A", &["no-value = EO { $n } N/A"]),
            ("about-back = EO Back", &["about-back EO Back"]),
            (
                "about-license = EO License",
                &["about-license = EO { License"],
            ),
            (
                "about-links = EO Links",
                &["about-links = EO { -links } Links"],
            ),
            (
                "about-package = EO Package",
                &["about-package =", "    .title = EO Package"],
            ),
        ];
        let full = eo_text();
        let mut found = [0; 6];
        let mut broken = String::new();
        for line in full.lines() {
            match edits.iter().position(|(old, _)| *old == line) {
                Some(i) => {
                    found[i] += 1;
                    for new in edits[i].1 {
                        broken.push_str(new);
                        broken.push('\n');
                    }
                }
                None => {
                    broken.push_str(line);
                    broken.push('\n');
                }
            }
        }
        assert_eq!(found, [1; 6], "each edited line once in X-loc-eo");
        broken.push_str("link-wiki = EO Go to the wiki\n-brand = EO Puron\n");
        with_english(Box::new(Files(vec![
            (ESPERANTO.to_string(), broken.into_bytes()),
            ("not a tag/puron.ftl".to_string(), full.into_bytes()),
        ])))
    }
}
