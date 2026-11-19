//! The facts that the popup's About page shows about this program: the
//! kind of package it was built for, its license lines, its copyright
//! line and its web addresses.

/// The kind of package that this program was built for. The build sets
/// it (`make install` with `PACKAGE`); the program never tests at run
/// time how it was installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Package {
    /// The Debian package.
    Deb,
    /// The Flatpak.
    Flatpak,
    /// Any other build, for example `make install` or `cargo build` from
    /// a source tree.
    Source,
}

impl Package {
    /// The kind for the value of `PURON_PACKAGE` at build time: `deb` and
    /// `flatpak` name their package, and no value or an empty one is a
    /// build from source. The Makefile refuses any other value, so a
    /// plain `cargo build` is the only way to get one here, and it counts
    /// as a build from source too.
    pub fn of(value: Option<&str>) -> Package {
        match value {
            Some("deb") => Package::Deb,
            Some("flatpak") => Package::Flatpak,
            _ => Package::Source,
        }
    }

    /// The kind that this program was built with. `make install-bin` sets
    /// `PURON_PACKAGE` for the applet's build, and cargo builds the applet
    /// again when the value changes.
    pub fn built() -> Package {
        Package::of(option_env!("PURON_PACKAGE"))
    }
}

/// The copyright line of the page. It names the holder that `LICENSE`
/// names, and it is the same in every language.
pub const COPYRIGHT: &str = "© 2026 Eric Lalonde";
