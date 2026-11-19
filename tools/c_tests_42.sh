#!/bin/sh
# Checks that each product file that names the copyright holder of
# Puron's own files names "Eric Lalonde", the holder that the About page
# shows: LICENSE, every holder line of debian/copyright, and the one Rust
# source of the applet that holds the page's copyright line. The web
# addresses keep the account name elalonde. The metainfo's developer
# name is checked by tools/c_tests_19.sh.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_42.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-42: FAIL $*"; exit 1; }
# count <want> <what> <grep arguments...>
count() {
    want=$1; what=$2; shift 2
    n=$(grep "$@")
    [ "$n" = "$want" ] || fail "$what: $n, want $want"
}
count 1 "LICENSE lines 'Copyright (c) 2026 Eric Lalonde'" -cx 'Copyright (c) 2026 Eric Lalonde' LICENSE
count 0 "LICENSE lines that name elalonde" -c elalonde LICENSE
d=debian/copyright
count 1 "$d lines 'Copyright: 2026 Eric Lalonde'" -cx 'Copyright: 2026 Eric Lalonde' "$d"
count 2 "$d holder lines of the two binaries" \
    -c '^Copyright: 2026 Eric Lalonde, and the authors of the Rust crates the binary is$' "$d"
count 1 "$d lines ' Copyright (c) 2026 Eric Lalonde'" -cx ' Copyright (c) 2026 Eric Lalonde' "$d"
count 0 "$d lines that name '2026 elalonde'" -c '2026 elalonde' "$d"
count 1 "$d Source lines" -cx 'Source: https://github.com/elalonde/puron' "$d"
files=$(grep -rlF '© 2026 Eric Lalonde' crates/puron-applet/src --include='*.rs' | wc -l)
[ "$files" = 1 ] || fail "applet sources that hold '© 2026 Eric Lalonde': $files, want 1"
echo "C-tests-42: ok"
exit 0
