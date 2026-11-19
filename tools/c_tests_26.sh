#!/bin/sh
# Checks that no program of Puron tests at run time whether it runs in a
# Flatpak, so that both packages behave the same. The one run-time read
# is in the settings crate, crates/puron-config/src/lib.rs: inside a
# Flatpak the home configuration folder is moved, and that file reads
# FLATPAK_ID once to find the shared settings. The applet also knows the
# package it was built for, but only from its build: one line of the
# applet reads PURON_PACKAGE with option_env!, and nothing reads it from
# the environment at run time. So the Rust sources that name Flatpak, in
# any case, are that settings file and the applet's own sources; FLATPAK_ID
# is read in one line of the settings file only, outside comments; and no
# file of a crate reads Flatpak's /.flatpak-info.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_26.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-26: FAIL $*"; exit 1; }
for f in $(grep -rli flatpak crates/*/src --include='*.rs'); do
    case $f in
        crates/puron-config/src/lib.rs | crates/puron-applet/src/*) ;;
        *) fail "$f names Flatpak; only crates/puron-config/src/lib.rs and the applet's sources may" ;;
    esac
done
code() { grep -rh "$1" crates --include='*.rs' | grep -v '^[[:space:]]*//'; }
n=$(grep FLATPAK_ID crates/puron-config/src/lib.rs | grep -cv '^[[:space:]]*//')
[ "$n" = 1 ] || fail "crates/puron-config/src/lib.rs reads FLATPAK_ID in $n lines, want 1"
# A test may clear FLATPAK_ID for a child it starts, so this count reads
# the crates' src directories only.
n=$(grep -rh FLATPAK_ID crates/*/src --include='*.rs' | grep -cv '^[[:space:]]*//')
[ "$n" = 1 ] || fail "the crates' sources read FLATPAK_ID in $n lines, want 1 (the settings crate's)"
n=$(code PURON_PACKAGE | wc -l)
[ "$n" = 1 ] || fail "the Rust sources under crates name PURON_PACKAGE in $n lines outside comments, want 1"
code PURON_PACKAGE | grep -qF 'option_env!("PURON_PACKAGE")' \
    || fail "the line that names PURON_PACKAGE does not read it with option_env!"
found=$(grep -rl PURON_PACKAGE crates --include='*.rs' | while read -r f; do
    grep PURON_PACKAGE "$f" | grep -qv '^[[:space:]]*//' && echo "$f"; done)
case $found in
    crates/puron-applet/src/*) ;;
    *) fail "PURON_PACKAGE is read in '$found', not in the applet's sources" ;;
esac
found=$(grep -rlE 'var(_os)?\("PURON_PACKAGE"\)' crates --include='*.rs')
[ -z "$found" ] || fail "files that read PURON_PACKAGE at run time: $(echo $found)"
found=$(grep -rl 'flatpak-info' crates)
[ -z "$found" ] || fail "files that read flatpak-info: $(echo $found)"
echo "C-tests-26: ok"
exit 0
