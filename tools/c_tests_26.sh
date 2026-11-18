#!/bin/sh
# Checks that no code of Puron tells the Flatpak apart from the Debian
# package, so that both packages behave the same. The one exception is
# the settings crate, crates/puron-config/src/lib.rs: inside a Flatpak
# the home configuration folder is moved, and that file reads FLATPAK_ID
# once to find the shared settings. No other Rust source under a crate's
# src/ may name Flatpak in any case, the settings crate may read
# FLATPAK_ID in one line only, outside comments, and no file of a crate
# may read Flatpak's /.flatpak-info.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_26.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-26: FAIL $*"; exit 1; }
found=$(grep -rli flatpak crates/*/src --include='*.rs')
[ "$found" = crates/puron-config/src/lib.rs ] \
    || fail "the Rust sources that name Flatpak are '$(echo $found)', want crates/puron-config/src/lib.rs"
n=$(grep FLATPAK_ID crates/puron-config/src/lib.rs | grep -cv '^[[:space:]]*//')
[ "$n" = 1 ] || fail "crates/puron-config/src/lib.rs reads FLATPAK_ID in $n lines, want 1"
found=$(grep -rl 'flatpak-info' crates)
[ -z "$found" ] || fail "files that read flatpak-info: $(echo $found)"
echo "C-tests-26: ok"
exit 0
