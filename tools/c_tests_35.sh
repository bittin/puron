#!/bin/sh
# Checks that the package kind reaches the applet's build in each package.
# make install-bin must run cargo build with PURON_PACKAGE set to the
# value of PACKAGE (empty for a build from source), a value other than
# deb, flatpak or empty must stop make before any command, and the Debian
# package's script must install with PACKAGE=deb. Only dry runs of make
# are used, so nothing is built.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_35.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-35: FAIL $*"; exit 1; }
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT

# build_line <want> [make argument]: the dry run's cargo build line must
# start with PURON_PACKAGE=<want> and a space.
build_line() {
    want=$1; shift
    make -s -n install-bin "$@" > "$tmp/out" 2>&1 || fail "make -n install-bin $* exits non-zero"
    line=$(grep 'cargo build' "$tmp/out")
    [ -n "$line" ] || fail "make -n install-bin $* prints no cargo build line"
    case $line in
        "PURON_PACKAGE=$want cargo build"*) ;;
        *) fail "make -n install-bin $*: '$line' does not hand PURON_PACKAGE=$want to cargo build" ;;
    esac
}
build_line ''
build_line deb PACKAGE=deb
build_line flatpak PACKAGE=flatpak
build_line '' PACKAGE=

# refused <value>: make stops with exit 2, names PACKAGE and runs no cargo.
refused() {
    make -n install "PACKAGE=$1" > "$tmp/out" 2>&1
    rc=$?
    [ "$rc" = 2 ] || fail "PACKAGE='$1': make exits $rc, want 2"
    grep -q PACKAGE "$tmp/out" || fail "PACKAGE='$1': the message does not name PACKAGE"
    ! grep -q cargo "$tmp/out" || fail "PACKAGE='$1': a line names cargo"
}
refused rpm
refused 'deb flatpak'

n=$(grep 'make -s install' tools/make-deb.sh | grep -c 'PREFIX=/usr PACKAGE=deb')
[ "$n" = 1 ] || fail "tools/make-deb.sh does not install with PREFIX=/usr PACKAGE=deb"
echo "C-tests-35: ok"
exit 0
