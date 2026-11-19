//! The package kind that the About page shows comes from a value that the
//! build sets, never from a test at run time.

use puron_applet::about::Package;

/// The time limit of each test here, in seconds.
const LIMIT: u64 = 5;

/// The build values unset, empty, `deb` and `flatpak` give a build from
/// source, a build from source, the Debian package and the Flatpak. The
/// test build itself runs with no value, so it is a build from source.
#[test]
fn c_tests_34() {
    puron_testkit::limit("C-tests-34", LIMIT, || {
        assert_eq!(Package::of(None), Package::Source);
        assert_eq!(Package::of(Some("")), Package::Source);
        assert_eq!(Package::of(Some("deb")), Package::Deb);
        assert_eq!(Package::of(Some("flatpak")), Package::Flatpak);
        assert_eq!(Package::built(), Package::Source);
    });
}
