//! The text that "Copy details" on the About page puts on the clipboard:
//! a short summary for a problem report. It is plain English in every
//! language, so that a maintainer can read every report, and it is built
//! only from the values passed to `text`: the version, the package kind,
//! the kernel release and each provider's state. No host name, user
//! name, path, device, sensor name or label can reach it.
//!
//! This module, the button's entry and its click are the only code of
//! the button, so that the button can be removed alone.

use std::path::Path;

use crate::about::{flatpak_liquidctl, Package, Source, SOURCES};

/// The file that holds the running kernel's release, read at each click.
pub const KERNEL_RELEASE_FILE: &str = "/proc/sys/kernel/osrelease";

/// The application ID that the copied text names.
pub const APP_ID: &str = puron_config::CONFIG_ID;

/// The longest kernel release, in bytes, that the text takes as it is.
const KERNEL_RELEASE_MAX: usize = 64;

/// The kernel release from the file at `path`: its first line without the
/// line end. It is "unknown" when the file cannot be read, or the line is
/// empty, longer than 64 bytes, or holds a byte other than a letter, a
/// digit, `.`, `_`, `+`, `~` and `-`, so that nothing but a release
/// number reaches the text.
pub fn kernel_release(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return "unknown".to_string();
    };
    let line = bytes.split(|b| *b == b'\n').next().unwrap_or_default();
    let allowed = |b: &u8| b.is_ascii_alphanumeric() || b"._+~-".contains(b);
    if line.is_empty() || line.len() > KERNEL_RELEASE_MAX || !line.iter().all(allowed) {
        return "unknown".to_string();
    }
    String::from_utf8_lossy(line).into_owned()
}

/// The package kind in plain English.
fn package_text(package: Package) -> &'static str {
    match package {
        Package::Deb => "Debian package",
        Package::Flatpak => "Flatpak",
        Package::Source => "Built from source",
    }
}

/// The copied text: one line for each item, each ended by a line feed.
pub fn text(version: &str, package: Package, kernel: &str, sources: &[Source; 5]) -> String {
    let mut t = format!(
        "Puron {version}\nPackage: {}\nApp ID: {APP_ID}\nKernel: {kernel}\n",
        package_text(package)
    );
    for (i, (s, (_, name))) in sources.iter().zip(SOURCES).enumerate() {
        let state = match *s {
            _ if flatpak_liquidctl(package, i, *s) => "not available in the Flatpak".to_string(),
            Source::Off => "off".to_string(),
            Source::NotAvailable => "not available".to_string(),
            Source::Available(1) => "available, 1 reading".to_string(),
            Source::Available(n) => format!("available, {n} readings"),
        };
        t.push_str(&format!("{name}: {state}\n"));
    }
    t
}
