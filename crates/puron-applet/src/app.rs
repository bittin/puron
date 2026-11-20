//! The applet's logic, with no UI code. `App` takes one `Event` at a time
//! and returns the `Output`s it produced; `launch` starts the browser
//! opener; `run` is the event loop that feeds `App`. The engine handle,
//! the config and the opener's search path are parameters, so tests can
//! drive `App` with a fake engine and without a display.

use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use puron_config::{Config, Delivery, Loaded, ProblemKind, StoreResult, Value, PINNED};
use puron_core::menu::{self, Key, Menu, Report, Shown, Update};
use puron_core::panel::{self, Panel, State};
use puron_core::pins;
use puron_core::rows::{Provider, ShownClasses};
use puron_core::settings::Settings;
use puron_engine::diag;
use puron_engine::engine::{Engine, LmReport, Snapshot};
use puron_engine::registry;

use crate::about::{self, Package};
use crate::copy;
use crate::settings;

/// The calls that `App` makes on the engine. `Engine` implements it; the
/// tests use a fake.
pub trait Handle: Send + Sync {
    fn start(&self, settings: &Settings);
    fn poll_now(&self);
    fn set_interval(&self, secs: u32);
    fn set_providers(&self, settings: &Settings);
    /// Stops the engine. Returns once its worker threads have ended or
    /// 1 s has passed, whichever comes first.
    fn shutdown(&self);
    /// Waits for a snapshot that has not been returned yet and returns it.
    /// The event loop's snapshot thread calls it over and over.
    fn next(&self) -> Snapshot;
    /// The latest snapshot if it has not been returned yet, without
    /// waiting.
    fn try_next(&self) -> Option<Snapshot>;
}

/// The real engine. `start` gives it the adapter of each provider from
/// `puron_engine::registry`.
impl Handle for Engine {
    fn start(&self, settings: &Settings) {
        Engine::start(
            self,
            settings,
            Box::new(|p| -> Box<dyn puron_engine::registry::Source> {
                match p {
                    Provider::LmSensors => Box::new(registry::lm_sensors()),
                    Provider::Liquidctl => Box::new(registry::liquidctl()),
                    Provider::Nvidia => Box::new(registry::nvidia()),
                    Provider::Udisks2 => Box::new(registry::udisks2()),
                    Provider::NvmeCli => Box::new(registry::nvme()),
                }
            }),
        );
    }
    fn poll_now(&self) {
        Engine::poll_now(self)
    }
    fn set_interval(&self, secs: u32) {
        Engine::set_interval(self, secs)
    }
    fn set_providers(&self, settings: &Settings) {
        Engine::set_providers(self, settings)
    }
    fn shutdown(&self) {
        Engine::shutdown(self)
    }
    fn next(&self) -> Snapshot {
        Engine::next(self)
    }
    fn try_next(&self) -> Option<Snapshot> {
        Engine::try_next(self)
    }
}

/// One input to `App::handle`.
#[derive(Clone, Debug)]
pub enum Event {
    /// The first event: load the config and start the engine.
    Start,
    /// A new snapshot from the engine.
    Snapshot(Snapshot),
    /// A change of the config, from `Config::watch`.
    Delivery(Delivery),
    /// A click on the popup line with this identity.
    PinClick(String),
    /// A click on the entry "Settings": the popup shows the Settings page.
    Settings,
    /// A message of a control of the Settings page: its change is stored.
    Edit(settings::Edit),
    /// A click on the no-readings message: open the homepage in the
    /// browser.
    HomepageLink,
    /// A click on the entry "About": the popup shows the About page.
    About,
    /// A click on "Back" on the About page or the Settings page: the popup
    /// shows the temperatures again.
    Back,
    /// The window loop opens the popup. It always opens on the
    /// temperatures, even when it was closed on another page.
    PopupOpened,
    /// The popup closed, by a click outside it, the panel button or a
    /// link.
    PopupClosed,
    /// A click on a link of the About page: open its address in the
    /// browser.
    OpenLink(String),
    /// A click on "Copy details" on the About page: build the summary
    /// that the window loop puts on the clipboard.
    CopyDetails,
    /// Shut the engine down and exit. Only the first one counts.
    Stop,
}

/// A call that `App` made on the engine handle, as `Output::Engine`
/// records it.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Start(Settings),
    PollNow,
    SetInterval(u32),
    SetProviders(Settings),
    Shutdown,
}

/// The result of `launch`: the program started, or it could not be
/// started, with the `errno` of the failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launched {
    Started,
    Failed(i32),
}

/// One thing that `App::handle` did. An event returns its outputs in the
/// order they happened; `run` passes them to its `view` callback and exits
/// on `Exit`.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// A call on the engine handle.
    Engine(Command),
    /// The pinned list was written to the config, with the result.
    Store(Vec<String>, StoreResult),
    /// Diagnostic logging was switched on or off (`diag::switch`).
    Switch(bool),
    /// A diagnostic event, by its ID, such as `display-updated`. It is
    /// recorded even while diagnostic logging is off and `diag::event`
    /// drops it.
    Diag(&'static str),
    /// A new popup layout.
    Layout {
        menu: Menu,
        /// True when a change of the shown classes asked for a rebuild.
        forced: bool,
    },
    /// New panel items.
    Panel(Panel),
    /// A program was started, or could not be, from this path.
    Launch(PathBuf, Launched),
    /// The process is to exit now; the engine's shutdown has returned.
    Exit,
    /// The text that "Copy details" puts on the clipboard. The window loop
    /// writes it there.
    Copy(String),
    /// A setting changed on the Settings page was written to the config,
    /// with the result. It applies only when the config watch reports it.
    Setting {
        key: &'static str,
        value: Value,
        result: StoreResult,
    },
}

/// Which page the popup shows: the temperatures, or the About page or the
/// Settings page in their place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Temperatures,
    About,
    Settings,
}

/// Where `App` looks for the program it can start, the browser opener. It
/// is a parameter so that tests can point it elsewhere.
#[derive(Clone, Debug)]
pub struct Programs {
    /// The directories searched for `xdg-open`, in the form of `PATH`.
    pub search_path: OsString,
}

impl Programs {
    /// The location for the installed applet: `xdg-open` along the
    /// applet's own `PATH`, left empty when it is not set.
    pub fn production() -> Programs {
        Programs {
            search_path: std::env::var_os("PATH").unwrap_or_default(),
        }
    }
}

/// The program that opens the homepage in the user's browser.
pub const OPENER: &str = "xdg-open";

/// Where the pinned list stands. `App` moves it only in these ways:
/// - from `Starting` to `NewInstall`, `Waiting` or `Settled` when the
///   config is loaded, according to the loaded value;
/// - from `NewInstall` to `Waiting` or `Settled` at a pin click, or when a
///   config change brings a pinned list that another process wrote;
/// - from `Waiting` to `Settled` once no pin waits any more;
/// - from `Waiting` or `Settled` to whichever of the two fits the list
///   after a pin click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinPhase {
    /// Before `Event::Start`: no config has been loaded.
    Starting,
    /// The config has no pinned list. The lines that
    /// `Shown::new_install_pins` picks from the latest readings are pinned
    /// in memory only; nothing is stored until the first pin click.
    NewInstall,
    /// The stored list holds an older pin, `@maximum` or `@cpu`, which
    /// stands for a line instead of naming one by its identity. It waits
    /// until lm-sensors has read (or is switched off) and the popup has a
    /// temperature line; then `puron_core::pins::moved` replaces it with
    /// a line's identity.
    Waiting,
    /// The list came from the config or from a click, and no pin in it
    /// waits. A stored value that could not be read counts as an empty
    /// list.
    Settled,
}

/// `Waiting` when `list` holds a pin that waits, else `Settled`.
fn stored_phase(list: &[String]) -> PinPhase {
    if list.iter().any(|e| pins::waits(e)) {
        PinPhase::Waiting
    } else {
        PinPhase::Settled
    }
}

/// The applet's state: the settings, the latest snapshot, the lines it
/// gives, the current layout and panel, and the pinned list, together
/// with the engine handle, the config and the program locations it was
/// made with. It changes only through `handle`.
pub struct App {
    handle: Arc<dyn Handle>,
    config: Config,
    programs: Programs,
    settings: Settings,
    last: Option<Snapshot>,
    /// Every line that the popup can show for `last` and `settings`.
    shown: Shown,
    /// The key of the current layout. The next layout compares its own
    /// key with it to tell whether the popup must be rebuilt.
    last_key: Option<Key>,
    /// Set when a config change altered the shown classes: the layout
    /// built at the next snapshot is a forced rebuild. With every provider
    /// off, the delivery builds that layout itself and clears the flag.
    forced: bool,
    panel: Panel,
    menu: Option<Menu>,
    /// The classes that the settings showed when the current layout was
    /// built. A pin click passes these, not the current settings, to
    /// `puron_core::pins::click`.
    layout_classes: ShownClasses,
    /// The pinned list in memory. It can differ from the stored one, for
    /// example during `NewInstall` or after a failed store.
    pinned: Vec<String>,
    pin_phase: PinPhase,
    /// The last store of the pinned list failed. It is tried again at the
    /// next snapshot.
    store_pending: bool,
    /// Set by the first `Event::Stop`; later ones do nothing.
    stopped: bool,
    /// The popup's page. It lasts only while the popup is open: each
    /// opening shows the temperatures.
    page: Page,
    /// The file that "Copy details" reads the kernel release from.
    kernel_file: PathBuf,
    /// The value each control of the Settings page shows. Each opening of
    /// the page starts from `settings`; a control's change sets its own
    /// value here, and `settings` follows only when the watch reports it.
    values: Settings,
    /// The text typed into the page's number field and not yet taken.
    typed: Option<String>,
    /// The latest store of a control's setting failed; the page shows a
    /// notice until a later one succeeds.
    notice: bool,
}

fn kind_word(k: ProblemKind) -> &'static str {
    match k {
        ProblemKind::Unreadable => "unreadable",
        ProblemKind::WrongType => "wrong-type",
        ProblemKind::Clamped => "clamped",
    }
}

impl App {
    pub fn new(handle: Arc<dyn Handle>, config: Config, programs: Programs) -> App {
        let settings = Settings::default();
        App {
            handle,
            config,
            programs,
            shown: menu::shown(&[], &settings),
            layout_classes: settings.shown_classes(),
            settings,
            last: None,
            last_key: None,
            forced: false,
            panel: Panel {
                items: Vec::new(),
                padding: 0,
                state: State::default(),
                boxes: Vec::new(),
            },
            menu: None,
            pinned: Vec::new(),
            pin_phase: PinPhase::Starting,
            store_pending: false,
            stopped: false,
            page: Page::Temperatures,
            kernel_file: PathBuf::from(copy::KERNEL_RELEASE_FILE),
            values: Settings::default(),
            typed: None,
            notice: false,
        }
    }

    /// The same `App` reading the kernel release for "Copy details" from
    /// `path` instead of the running kernel's file, for tests.
    pub fn with_kernel_release_file(mut self, path: PathBuf) -> App {
        self.kernel_file = path;
        self
    }

    /// The page that the popup shows.
    pub fn page(&self) -> Page {
        self.page
    }

    /// What the About page is drawn from: this program's version, the
    /// package kind it was built for, and the state of each source of
    /// sensors in the latest snapshot.
    pub fn about_input(&self) -> about::Input {
        about::Input {
            version: about::VERSION.to_string(),
            package: Package::built(),
            sources: about::sources(self.last.as_ref(), &self.settings),
        }
    }

    /// What the Settings page is drawn from: the value each control
    /// shows, the text being typed into the number field, whether the
    /// latest store failed, and for the state lines under the provider
    /// switches the applied settings, the state of each source and the
    /// number of drive readings in the latest snapshot, and the package
    /// kind.
    pub fn settings_input(&self) -> settings::Input {
        settings::Input {
            values: self.values.clone(),
            typed: self.typed.clone(),
            notice: self.notice,
            applied: self.settings.clone(),
            sources: about::sources(self.last.as_ref(), &self.settings),
            drive_count: settings::drive_count(self.last.as_ref()),
            package: Package::built(),
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn pinned(&self) -> &[String] {
        &self.pinned
    }
    pub fn panel(&self) -> &Panel {
        &self.panel
    }
    pub fn menu(&self) -> Option<&Menu> {
        self.menu.as_ref()
    }
    pub fn pin_phase(&self) -> PinPhase {
        self.pin_phase
    }
    pub fn store_pending(&self) -> bool {
        self.store_pending
    }
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Handles one event and returns what it did, in order. A snapshot
    /// recomputes the shown lines, moves pins that are ready to move, and
    /// lays out the popup and the panel again.
    pub fn handle(&mut self, event: Event) -> Vec<Output> {
        let mut out = Vec::new();
        // Leaving the Settings page, by "Back" or by closing the popup,
        // takes the text typed into its number field first, as a focus
        // loss would, so that a value the user typed is not lost.
        if matches!(event, Event::Back | Event::PopupClosed) {
            let stores = settings::leave(&mut self.values, &mut self.typed);
            self.store_settings(stores, &mut out);
        }
        match event {
            Event::Start => self.start(&mut out),
            Event::Snapshot(s) => {
                self.last = Some(s);
                self.shown = self.compute_shown();
                self.move_pins(&mut out);
                let forced = self.forced;
                self.forced = false;
                self.layout(forced, &mut out);
            }
            Event::Delivery(d) => self.delivery(d, &mut out),
            Event::PinClick(id) => self.pin_click(&id, &mut out),
            // The page starts from the settings as last loaded or reported.
            // Nothing is stored and nothing is started.
            Event::Settings => {
                self.page = Page::Settings;
                self.values = self.settings.clone();
                self.typed = None;
            }
            Event::Edit(e) => {
                let stores = settings::edit(&mut self.values, &mut self.typed, e);
                self.store_settings(stores, &mut out);
            }
            Event::PopupClosed => {}
            Event::HomepageLink => self.open_url(menu::HOMEPAGE_URL, &mut out),
            Event::OpenLink(url) => self.open_url(&url, &mut out),
            // The page changes only on these events and on Settings. A snapshot or a
            // settings change keeps it, and keeps computing the
            // temperatures' layout, which "Back" then shows.
            Event::About => self.page = Page::About,
            Event::Back | Event::PopupOpened => self.page = Page::Temperatures,
            Event::CopyDetails => out.push(Output::Copy(self.copy_details())),
            Event::Stop => {
                if !self.stopped {
                    self.stopped = true;
                    self.handle.shutdown();
                    out.push(Output::Engine(Command::Shutdown));
                    out.push(Output::Exit);
                }
            }
        }
        out
    }

    /// Starts the browser opener with `url`. It looks for xdg-open along
    /// the search path. An empty or unset PATH has no directories, so
    /// nothing is found there: split_paths would give one empty entry,
    /// which means the current directory. When nothing is found, the
    /// launch fails with ENOENT.
    fn open_url(&self, url: &str, out: &mut Vec<Output>) {
        let path = &self.programs.search_path;
        let found = (!path.is_empty())
            .then(|| std::env::split_paths(path))
            .into_iter()
            .flatten()
            .map(|d| d.join(OPENER))
            .find(|p| is_executable(p));
        match found {
            Some(path) => {
                let r = launch(&path, &[url]);
                self.launched(path, r, out);
            }
            None => self.launched(PathBuf::from(OPENER), Launched::Failed(libc::ENOENT), out),
        }
    }

    /// The text of "Copy details": the version, the package kind, the
    /// kernel release read now, and the state of each source of sensors.
    fn copy_details(&self) -> String {
        let input = self.about_input();
        let kernel = copy::kernel_release(&self.kernel_file);
        copy::text(&input.version, input.package, &kernel, &input.sources)
    }

    /// Writes each of `stores` to the config, in order, and records it in
    /// `out`. A failed store is not undone: the control keeps the new
    /// value and the page shows a notice until a later store succeeds; the
    /// value is stored again only at the control's next change. Nothing is
    /// applied here: the config watch reports a stored value, and
    /// `delivery` applies it as a change made by another program.
    fn store_settings(&mut self, stores: Vec<settings::Store>, out: &mut Vec<Output>) {
        for (key, value) in stores {
            let result = self.config.store(key, &value);
            self.notice = result == StoreResult::Failed;
            out.push(Output::Setting { key, value, result });
        }
    }

    /// Records `e` in `out` and logs it. `diag::event` drops it while
    /// diagnostic logging is off.
    fn diag(&self, e: diag::Event<'_>, out: &mut Vec<Output>) {
        out.push(Output::Diag(e.id));
        diag::event(e);
    }

    /// Logs a `config-problem` event for each problem the config reported.
    fn problems(&self, problems: &[puron_config::Problem], out: &mut Vec<Output>) {
        for p in problems {
            self.diag(
                diag::applet::config_problem(p.key.unwrap_or(""), kind_word(p.kind)),
                out,
            );
        }
    }

    /// Handles `Event::Start`: loads the config, switches diagnostic
    /// logging as configured, builds the first layout and panel before any
    /// reading has arrived, and starts the engine.
    fn start(&mut self, out: &mut Vec<Output>) {
        let loaded = self.config.load();
        self.take_pinned(&loaded);
        self.settings = loaded.settings;
        diag::switch(self.settings.debug_logging);
        out.push(Output::Switch(self.settings.debug_logging));
        self.problems(&loaded.problems, out);
        self.shown = self.compute_shown();
        let waiting = self.waiting();
        let m = menu::menu_with(
            &self.shown,
            &self.settings,
            Report::NoneYet,
            None,
            false,
            &waiting,
        );
        self.set_menu(m, false, out);
        self.set_panel(State::default(), out);
        self.handle.start(&self.settings);
        out.push(Output::Engine(Command::Start(self.settings.clone())));
    }

    /// Takes the pinned list and its phase from a loaded config. Only a
    /// pinned key that is missing from the config means a new install. A
    /// stored value that cannot be read gives an empty list in `Settled`:
    /// no line in the popup has a check mark, and nothing is stored until
    /// a click.
    fn take_pinned(&mut self, loaded: &Loaded) {
        let unreadable = loaded.problems.iter().any(|p| p.key == Some(PINNED));
        match &loaded.settings.pinned {
            None if !unreadable => {
                self.pinned = Vec::new();
                self.pin_phase = PinPhase::NewInstall;
            }
            stored => {
                self.pinned = pins::start(stored.as_deref().unwrap_or_default());
                self.pin_phase = stored_phase(&self.pinned);
            }
        }
    }

    /// The pins that wait for lm-sensors, in list order, while the phase is
    /// `Waiting`; empty otherwise. The layout gives each one a line of its
    /// own, so that a click can remove it.
    fn waiting(&self) -> Vec<String> {
        if self.pin_phase != PinPhase::Waiting {
            return Vec::new();
        }
        self.pinned
            .iter()
            .filter(|e| pins::waits(e))
            .cloned()
            .collect()
    }

    fn compute_shown(&self) -> Shown {
        let raw = self.last.as_ref().map(|s| &s.readings[..]).unwrap_or(&[]);
        menu::shown(raw, &self.settings)
    }

    /// The state of lm-sensors in the latest snapshot, in the form the
    /// menu takes; `NoneYet` before the first snapshot.
    fn report(&self) -> Report {
        match self.last.as_ref().map(|s| s.lm_sensors) {
            None | Some(LmReport::NoneYet) => Report::NoneYet,
            Some(LmReport::Unavailable) => Report::Unavailable,
            Some(LmReport::Answered) => Report::Answered,
        }
    }

    /// Whether lm-sensors has completed a read and the latest snapshot
    /// holds at least one of its readings. A failed start, or a failed
    /// read with no earlier readings, does not count.
    fn lm_sensors_read(&self) -> bool {
        let lm = |s: &Snapshot| s.readings.iter().any(|r| r.provider == Provider::LmSensors);
        self.report() == Report::Answered && self.last.as_ref().is_some_and(lm)
    }

    fn set_menu(&mut self, m: Menu, forced: bool, out: &mut Vec<Output>) {
        self.last_key = Some(m.key.clone());
        self.layout_classes = self.settings.shown_classes();
        self.menu = Some(m.clone());
        out.push(Output::Layout { menu: m, forced });
    }

    /// Builds the panel from the shown lines and the pinned list, carrying
    /// `state` (the icon each pin last had) over from the previous panel.
    fn set_panel(&mut self, state: State, out: &mut Vec<Output>) {
        let p = panel::panel(&self.shown, &self.pinned, &self.settings, &state);
        self.panel = p.clone();
        out.push(Output::Panel(p));
    }

    /// Builds a new layout and panel after start: at a snapshot, or at once
    /// when unit suffixes are switched on or off or a waiting pin is
    /// removed.
    /// `forced` asks for a rebuild even when the layout key is unchanged.
    /// Logs `menu-rebuilt` when the popup is rebuilt, and `display-updated`
    /// every time.
    fn layout(&mut self, forced: bool, out: &mut Vec<Output>) {
        let m = menu::menu_with(
            &self.shown,
            &self.settings,
            self.report(),
            self.last_key.as_ref(),
            forced,
            &self.waiting(),
        );
        if m.update == Update::Rebuild {
            self.diag(diag::applet::menu_rebuilt(), out);
        }
        self.set_menu(m, forced, out);
        self.set_panel(self.panel.state.clone(), out);
        self.diag(diag::applet::display_updated(), out);
    }

    /// Handles a config change: applies the new settings at once and does
    /// only the work that the changed settings need. A new interval or a
    /// new set of providers goes to the engine. A change of the shown
    /// classes, the temperature unit, the decimals or the pinned list asks
    /// the engine for a poll, unless the providers changed too. A change of
    /// the shown classes also forces a rebuild at the next snapshot. When
    /// every provider is off, no snapshot follows a poll, so a change that
    /// asks for one lays out at once instead. A
    /// change of whether unit suffixes are shown lays out at once. A change
    /// of the panel icons, or a pinned list taken from another process
    /// during `NewInstall`, rebuilds only the panel. So does a change of a
    /// text size, so that every value's box starts again at the width of
    /// the new size; the panel button reads the size itself from the
    /// settings at its next drawing, which the new settings cause.
    fn delivery(&mut self, d: Delivery, out: &mut Vec<Output>) {
        // A list written from outside during a new install is taken as a
        // stored one, so that a click builds on it instead of replacing it.
        let outside = self.pin_phase == PinPhase::NewInstall
            && (d.loaded.settings.pinned.is_some()
                || d.loaded.problems.iter().any(|p| p.key == Some(PINNED)));
        if outside {
            self.take_pinned(&d.loaded);
        }
        let old = std::mem::replace(&mut self.settings, d.loaded.settings);
        let (o, n) = (&old, &self.settings);
        if o.debug_logging != n.debug_logging {
            diag::switch(n.debug_logging);
            out.push(Output::Switch(n.debug_logging));
        }
        self.problems(&d.loaded.problems, out);
        // The Settings page shows what was stored, also its own changes.
        self.values = settings::delivered(&self.values, &self.typed, &self.settings);
        let units = o.show_temperature_unit != n.show_temperature_unit
            || o.show_rotation_rate_unit != n.show_rotation_rate_unit
            || o.show_voltage_unit != n.show_voltage_unit
            || o.show_power_unit != n.show_power_unit;
        let shown_classes = o.show_temperature != n.show_temperature
            || o.show_rotation_rate != n.show_rotation_rate
            || o.show_voltage != n.show_voltage
            || o.show_power != n.show_power;
        let poll = shown_classes
            || o.temperature_unit != n.temperature_unit
            || o.show_decimals != n.show_decimals
            || o.pinned != n.pinned;
        let providers = o.provider_lm_sensors != n.provider_lm_sensors
            || o.provider_liquidctl != n.provider_liquidctl
            || o.provider_nvidia != n.provider_nvidia
            || o.provider_udisks2 != n.provider_udisks2
            || o.provider_nvme != n.provider_nvme
            || o.provider_lm_sensors_drives != n.provider_lm_sensors_drives;
        let interval = o.poll_interval_secs != n.poll_interval_secs;
        let icons = o.show_panel_icons != n.show_panel_icons;
        let none_on = !registry::PROVIDERS
            .iter()
            .any(|e| registry::switch(n, e.provider));
        // A new text size needs new panel items, as a change of the panel
        // icons does: the same lines give the same items, but every value's
        // box starts again at the width of the new size, and a box grown
        // under the old size gives up its extra width.
        let icons =
            icons || o.panel_text_size != n.panel_text_size || o.dock_text_size != n.dock_text_size;
        if interval {
            self.handle.set_interval(n.poll_interval_secs);
            out.push(Output::Engine(Command::SetInterval(n.poll_interval_secs)));
        }
        if providers {
            self.handle.set_providers(&self.settings);
            out.push(Output::Engine(Command::SetProviders(self.settings.clone())));
        } else if poll {
            self.handle.poll_now();
            out.push(Output::Engine(Command::PollNow));
        }
        if shown_classes {
            self.forced = true;
        }
        if poll && !providers && none_on {
            // The engine's poll skips every provider that is off, so with
            // all of them off no snapshot follows. The change is shown at
            // once from the last snapshot, and the forced rebuild is used
            // here instead of waiting for a snapshot that never comes.
            self.shown = self.compute_shown();
            let forced = std::mem::take(&mut self.forced);
            self.layout(forced, out);
        } else if units {
            self.shown = self.compute_shown();
            self.layout(false, out);
        } else if icons || outside {
            // Only the panel items change. No layout is built, so no
            // `display-updated` event is logged.
            self.set_panel(self.panel.state.clone(), out);
        }
    }

    /// Handles a click on a line: computes the new pinned list against the
    /// current layout, stores it, rebuilds the panel (or the whole layout
    /// when a waiting pin was removed), and asks the engine for a poll.
    fn pin_click(&mut self, id: &str, out: &mut Vec<Output>) {
        let ids = self
            .menu
            .as_ref()
            .map(|m| m.identities.clone())
            .unwrap_or_default();
        let waiting = self.waiting();
        self.pinned = pins::click(&self.pinned, id, &ids, &self.layout_classes);
        self.pin_phase = stored_phase(&self.pinned);
        self.store_pinned(out);
        if self.waiting() != waiting {
            // A waiting pin was removed: its line leaves the popup at once.
            self.layout(false, out);
        } else {
            self.set_panel(self.panel.state.clone(), out);
        }
        self.handle.poll_now();
        out.push(Output::Engine(Command::PollNow));
    }

    /// Writes the pinned list to the config. On success the settings take
    /// the stored list. On failure `store_pending` is set, so that the next
    /// snapshot tries again.
    fn store_pinned(&mut self, out: &mut Vec<Output>) {
        let r = self.config.store(PINNED, &Value::List(self.pinned.clone()));
        out.push(Output::Store(self.pinned.clone(), r));
        match r {
            StoreResult::Stored => {
                self.settings.pinned = Some(self.pinned.clone());
                self.store_pending = false;
            }
            StoreResult::Failed => {
                // Log one event for a run of failures, not one per retry.
                if !self.store_pending {
                    self.diag(diag::applet::pinned_store_failed(), out);
                }
                self.store_pending = true;
            }
        }
    }

    /// Updates the pinned list after a snapshot. During `NewInstall` the
    /// list becomes `Shown::new_install_pins` of the latest readings, in
    /// memory only; when that is empty, the list stays as it was. In the
    /// other phases after start, `puron_core::pins::moved` moves pins to
    /// the lines that now stand for them, and the list is stored when it
    /// changed or when an earlier store failed. The older pins `@maximum`
    /// and `@cpu` may move only once lm-sensors has read, or at once when
    /// lm-sensors is switched off, so that the CPU's lines are there to
    /// move to.
    fn move_pins(&mut self, out: &mut Vec<Output>) {
        let ready = match self.pin_phase {
            PinPhase::Starting => return,
            PinPhase::NewInstall => {
                let defaults = self.shown.new_install_pins();
                if !defaults.is_empty() {
                    self.pinned = defaults;
                }
                return;
            }
            PinPhase::Waiting => !self.settings.provider_lm_sensors || self.lm_sensors_read(),
            PinPhase::Settled => false,
        };
        let cpu_source = self.settings.old_cpu_source.as_deref();
        let list = pins::moved(&self.pinned, &self.shown, cpu_source, ready);
        let changed = list != self.pinned;
        self.pinned = list;
        self.pin_phase = stored_phase(&self.pinned);
        if changed || self.store_pending {
            self.store_pinned(out);
        }
    }

    /// Records a launch's result. A failure also logs a `launch-failed`
    /// event with the program's path and the `errno`.
    fn launched(&self, path: PathBuf, r: Launched, out: &mut Vec<Output>) {
        if let Launched::Failed(errno) = r {
            let program = path.to_string_lossy().into_owned();
            let errno = errno.to_string();
            self.diag(diag::applet::launch_failed(&program, &errno), out);
        }
        out.push(Output::Launch(path, r));
    }
}

/// Whether `p` is a file that this process may execute (`access` with
/// `X_OK`).
fn is_executable(p: &Path) -> bool {
    let Ok(c) = CString::new(p.as_os_str().as_bytes()) else {
        return false;
    };
    p.is_file() && unsafe { libc::access(c.as_ptr(), libc::X_OK) } == 0
}

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// Starts `path` with `args` and the applet's environment, detached from
/// the applet, and reports whether it could be started.
///
/// The applet forks a child, which starts a new session with `setsid`,
/// forks the grandchild and exits at once. The grandchild runs `execve`,
/// so the started program is not the applet's child. A close-on-exec pipe
/// carries the result back: a successful `execve` closes it with nothing
/// written, while a failed `execve` or a failed second fork writes its
/// `errno` (four bytes). The applet reaps the child, then reads the pipe
/// until end of file. A path or argument that holds a NUL byte fails with
/// `EINVAL`.
pub fn launch(path: &Path, args: &[&str]) -> Launched {
    // Everything the children use is built before the first fork: in a
    // threaded program, a forked child may make only async-signal-safe
    // calls, so it must not allocate.
    let Ok(prog) = CString::new(path.as_os_str().as_bytes()) else {
        return Launched::Failed(libc::EINVAL);
    };
    let mut argv = vec![prog.clone()];
    for a in args {
        match CString::new(*a) {
            Ok(c) => argv.push(c),
            Err(_) => return Launched::Failed(libc::EINVAL),
        }
    }
    let env: Vec<CString> = std::env::vars_os()
        .filter_map(|(k, v)| {
            let mut b = k.as_bytes().to_vec();
            b.push(b'=');
            b.extend_from_slice(v.as_bytes());
            CString::new(b).ok()
        })
        .collect();
    let mut argp: Vec<*const libc::c_char> = argv.iter().map(|c| c.as_ptr()).collect();
    argp.push(std::ptr::null());
    let mut envp: Vec<*const libc::c_char> = env.iter().map(|c| c.as_ptr()).collect();
    envp.push(std::ptr::null());
    let mut fds = [0 as libc::c_int; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Launched::Failed(errno());
    }
    let (rd, wr) = (fds[0], fds[1]);
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        let e = errno();
        unsafe {
            libc::close(rd);
            libc::close(wr);
        }
        return Launched::Failed(e);
    }
    if pid == 0 {
        // The intermediate child calls only setsid, fork, write and _exit.
        unsafe {
            libc::setsid();
            let g = libc::fork();
            if g == 0 {
                libc::execve(prog.as_ptr(), argp.as_ptr(), envp.as_ptr());
                let e = *libc::__errno_location();
                libc::write(wr, (&e as *const i32).cast(), 4);
                libc::_exit(127);
            }
            if g < 0 {
                let e = *libc::__errno_location();
                libc::write(wr, (&e as *const i32).cast(), 4);
                libc::_exit(1);
            }
            libc::_exit(0);
        }
    }
    unsafe { libc::close(wr) };
    let mut status = 0;
    while unsafe { libc::waitpid(pid, &mut status, 0) } < 0 && errno() == libc::EINTR {}
    let mut buf = [0u8; 4];
    let mut got = 0usize;
    while got < 4 {
        let n = unsafe { libc::read(rd, buf[got..].as_mut_ptr().cast(), 4 - got) };
        if n < 0 && errno() == libc::EINTR {
            continue;
        }
        if n <= 0 {
            break;
        }
        got += n as usize;
    }
    unsafe { libc::close(rd) };
    if got == 4 {
        Launched::Failed(i32::from_ne_bytes(buf))
    } else {
        Launched::Started
    }
}

/// Where an event of the event loop came from. `run` passes it to its
/// `view` callback with the event's outputs: `Start` for the first
/// event, `Subscription` for the engine's snapshots, `Delivery` for config
/// changes, `Stop` for SIGTERM or the end of the toolkit, and `Ui` for
/// clicks in the popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Start,
    Subscription,
    Delivery,
    Stop,
    Ui,
}

/// The write end of the SIGTERM pipe, for the signal handler; -1 until
/// `stop_on_sigterm` has made the pipe.
static STOP_FD: AtomicI32 = AtomicI32::new(-1);

/// The SIGTERM handler. It only writes one byte to the pipe; a thread
/// started by `run` turns the byte into `Event::Stop`.
extern "C" fn on_sigterm(_: libc::c_int) {
    let fd = STOP_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        let b = 1u8;
        unsafe { libc::write(fd, (&b as *const u8).cast(), 1) };
    }
}

/// Installs the SIGTERM handler and returns the read end of its pipe. When
/// the pipe cannot be made, it returns -1 and SIGTERM keeps its default
/// action.
fn stop_on_sigterm() -> libc::c_int {
    let mut fds = [0 as libc::c_int; 2];
    unsafe {
        if libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) != 0 {
            return -1;
        }
        STOP_FD.store(fds[1], Ordering::SeqCst);
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = on_sigterm as extern "C" fn(libc::c_int) as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
    }
    fds[0]
}

/// The event loop. It never returns.
///
/// It first starts a thread that turns SIGTERM into `Event::Stop`, then
/// handles `Event::Start`, then starts the config watch and a thread that
/// forwards each new snapshot. From then on it handles the events from
/// `rx` one at a time, in arrival order: snapshots, config changes, stops,
/// and clicks that the UI sends through a clone of `tx`. After each event,
/// `view` gets the event's source, the state and the event's outputs. When
/// the outputs include `Output::Exit`, the process exits with status 0. If
/// every sender has gone, the loop handles that as `Event::Stop`.
pub fn run(
    mut app: App,
    tx: Sender<(Source, Event)>,
    rx: Receiver<(Source, Event)>,
    mut view: impl FnMut(Source, &App, &[Output]),
) -> ! {
    let stop = stop_on_sigterm();
    if stop >= 0 {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut b = 0u8;
            loop {
                let n = unsafe { libc::read(stop, (&mut b as *mut u8).cast(), 1) };
                if n == 1 {
                    let _ = tx.send((Source::Stop, Event::Stop));
                } else if n == 0 || errno() != libc::EINTR {
                    break;
                }
            }
        });
    }
    let out = app.handle(Event::Start);
    view(Source::Start, &app, &out);
    let dtx = std::sync::Mutex::new(tx.clone());
    let _watch = app.config().watch(move |d| {
        let _ = dtx
            .lock()
            .unwrap()
            .send((Source::Delivery, Event::Delivery(d)));
    });
    let handle = Arc::clone(&app.handle);
    let stx = tx.clone();
    std::thread::spawn(move || loop {
        let s = handle.next();
        if stx
            .send((Source::Subscription, Event::Snapshot(s)))
            .is_err()
        {
            break;
        }
    });
    drop(tx);
    loop {
        let (source, event) = rx.recv().unwrap_or((Source::Stop, Event::Stop));
        let out = app.handle(event);
        view(source, &app, &out);
        if out.contains(&Output::Exit) {
            std::process::exit(0);
        }
    }
}
