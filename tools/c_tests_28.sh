#!/bin/sh
# Checks the verdict of the Flatpak sandbox probe, tools/flatpak-probe.sh,
# on hand-written facts, and that the probe stays a script.
#
# The probe compares what the host and the Flatpak's sandbox can read. A
# rule that it silently drops, or a fact that it lets through, would let
# a Flatpak that cannot see the sensors or the lm-sensors configuration
# pass. So the check writes the facts of a host and of a sandbox that
# agree and expects the mode installed to pass with no FAIL line; then,
# one change at a time, a sandbox with other sensor chips, with the
# liquidctl program, without the host's lm-sensors configuration, with a
# missing file, without the COSMIC icons, and without the libsensors
# line at all, each of which must fail with exactly one FAIL line for
# that key. The mode build-dir must skip the icon line, which only an
# installed app can be judged on. Collecting on this host must print
# exactly the eight host keys, sorted. Last, no crate may hold a probe:
# a probe option or binary in the programs would be code that only the
# Flatpak needs.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_28.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-28: FAIL $*"; exit 1; }
p=tools/flatpak-probe.sh
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
tab=$(printf '\t')

printf '%s\n' "cosmic-theme${tab}yes" "cpus${tab}8" "dmi${tab}Box" \
    "hwmon${tab}acpitz,coretemp,nvme" "journal${tab}yes" "nvme${tab}Disk A" \
    "sensors-conf${tab}abc" "sensors-d${tab}x.conf" > "$tmp/H"
printf '%s\n' "cosmic-icons${tab}4" "cpus${tab}8" "dmi${tab}Box" "files${tab}11" \
    "hwmon${tab}acpitz,coretemp,nvme" "journal${tab}yes" "leftovers${tab}0" \
    "libsensors${tab}yes" "liquidctl${tab}no" "nvme${tab}Disk A" \
    "runtime-libsensors${tab}no" "sensors-conf${tab}abc" "sensors-d${tab}x.conf" \
    "xdg-open${tab}yes" > "$tmp/S"

sh "$p" --compare installed "$tmp/H" "$tmp/S" > "$tmp/out" 2>&1 \
    || { cat "$tmp/out"; fail "agreeing facts: --compare installed exits non-zero"; }
grep -q FAIL "$tmp/out" && { cat "$tmp/out"; fail "agreeing facts: a FAIL line"; }

# Writes a copy of the sandbox facts as $tmp/<name>, with key $2 set to
# $3, or with its line deleted when $3 is empty.
variant() {
    awk -F '\t' -v k="$2" -v v="$3" 'BEGIN { OFS = "\t" } $1 == k { if (v == "") next; $2 = v } { print }' \
        "$tmp/S" > "$tmp/$1"
}

# Runs the mode installed on the copy $1 and expects exit 1 with exactly
# one FAIL line, for key $2.
fails_on() {
    sh "$p" --compare installed "$tmp/H" "$tmp/$1" > "$tmp/out" 2>&1
    rc=$?
    [ "$rc" = 1 ] || { cat "$tmp/out"; fail "case $1: exit $rc, want 1"; }
    [ "$(grep -c FAIL "$tmp/out")" = 1 ] || { cat "$tmp/out"; fail "case $1: not exactly one FAIL line"; }
    grep -q "^PROBE $2 FAIL" "$tmp/out" || { cat "$tmp/out"; fail "case $1: no line PROBE $2 FAIL"; }
}

variant a hwmon coretemp,nvme
fails_on a hwmon
variant b liquidctl yes
fails_on b liquidctl
variant c sensors-conf -
fails_on c sensors-conf
variant d files 10
fails_on d files
variant e cosmic-icons 3
fails_on e cosmic-icons
variant f libsensors ""
grep -q '^libsensors' "$tmp/f" && fail "case f: the libsensors line is still there"
fails_on f libsensors

sh "$p" --compare build-dir "$tmp/H" "$tmp/e" > "$tmp/out" 2>&1 \
    || { cat "$tmp/out"; fail "build-dir with 3 icons: exits non-zero"; }
grep -qx 'PROBE cosmic-icons skip build-dir' "$tmp/out" \
    || { cat "$tmp/out"; fail "build-dir: no line PROBE cosmic-icons skip build-dir"; }

sh "$p" --collect host > "$tmp/host" 2>&1 || { cat "$tmp/host"; fail "--collect host exits non-zero"; }
keys=$(cut -f1 "$tmp/host" | paste -sd' ' -)
[ "$keys" = "cosmic-theme cpus dmi hwmon journal nvme sensors-conf sensors-d" ] \
    || fail "--collect host prints the keys '$keys'"

found=$(find crates -iname '*probe*')
[ -z "$found" ] || fail "a probe in the crates: $found"
found=$(find . -name Cargo.toml -not -path './target/*' -not -path './.git/*' \
    -exec grep -l '^[[:space:]]*name[[:space:]]*=[[:space:]]*".*probe' {} +)
[ -z "$found" ] || fail "a package named after a probe: $found"
echo "C-tests-28: ok"
