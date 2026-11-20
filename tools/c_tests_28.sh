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
# missing file, without the COSMIC icons, without the result of the
# libsensors start at all, with the start failing as it does in a bundle
# without SENSORS_SYSFS_ROOT, with the start writing the "Not a sysfs
# mount." warning as a bundle without the library patch does, with no
# chip found on a host that has hwmon devices, and with more chips than
# devices, each of which must fail with exactly one FAIL line for that
# key. A host and a sandbox with no hwmon device and no chip must pass.
# The probe must read the permissions from the app's own metadata,
# never from the view that adds the user's overrides. The mode build-dir
# must skip the icon line, which only an installed app can be judged on.
# Collecting on this host must print exactly the eight host keys, sorted.
# Last, no crate may hold a probe: a probe option or binary in the
# programs would be code that only the Flatpak needs.
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
printf '%s\n' "cosmic-icons${tab}4" "cpus${tab}8" "dmi${tab}Box" "files${tab}10" \
    "hwmon${tab}acpitz,coretemp,nvme" "journal${tab}yes" "leftovers${tab}0" \
    "libsensors-chips${tab}3" "libsensors-init${tab}0" "libsensors-stderr${tab}0" \
    "liquidctl${tab}no" "nvme${tab}Disk A" \
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
variant d files 9
fails_on d files
variant e cosmic-icons 3
fails_on e cosmic-icons
variant f libsensors-init ""
grep -q '^libsensors-init' "$tmp/f" && fail "case f: the libsensors-init line is still there"
fails_on f libsensors-init
variant g libsensors-init -4
fails_on g libsensors-init
variant h libsensors-stderr 25
fails_on h libsensors-stderr
variant i libsensors-chips 0
fails_on i libsensors-chips
grep -qx 'PROBE libsensors-chips FAIL 3 0' "$tmp/out" \
    || { cat "$tmp/out"; fail "case i: no line PROBE libsensors-chips FAIL 3 0"; }
variant j libsensors-chips 4
fails_on j libsensors-chips

# A host without hwmon devices: the sandbox must find no chip, and pass.
awk -F '\t' 'BEGIN { OFS = "\t" } $1 == "hwmon" { $2 = "-" } { print }' "$tmp/H" > "$tmp/H0"
awk -F '\t' 'BEGIN { OFS = "\t" } $1 == "hwmon" { $2 = "-" } $1 == "libsensors-chips" { $2 = 0 } { print }' \
    "$tmp/S" > "$tmp/S0"
sh "$p" --compare installed "$tmp/H0" "$tmp/S0" > "$tmp/out" 2>&1 \
    || { cat "$tmp/out"; fail "no hwmon devices: --compare installed exits non-zero"; }
grep -qx 'PROBE libsensors-chips ok' "$tmp/out" \
    || { cat "$tmp/out"; fail "no hwmon devices: no line PROBE libsensors-chips ok"; }

# The permissions come from the app's metadata, which holds what the
# manifest asked for, not from a view that also holds overrides.
[ "$(grep -c 'show-permissions' "$p")" = 0 ] || fail "the probe reads flatpak info --show-permissions"
[ "$(grep -c 'show-metadata' "$p")" -ge 1 ] || fail "the probe does not read flatpak info --show-metadata"

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
