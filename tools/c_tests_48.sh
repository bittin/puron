#!/bin/sh
# Checks that Puron is one program, puron-applet: the settings are a page
# of its popup, and no separate settings program, as up to version 1.5,
# is built, installed or packaged. The workspace's metadata must list
# exactly one binary target, puron-applet, and no package puron-settings.
# The test kit's binaries are left out: they are helpers that the tests
# run, and nothing installs them.
# A dry run of make install-bin (make -n prints the commands and runs
# none, so nothing is built) must name puron-applet and not
# puron-settings. The .deb build and check scripts, the Debian copyright,
# the Flatpak probe and the Flatpak manifest must not name it, and
# neither may the metainfo's provides element; the metainfo's release
# notes of earlier versions may.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_48.sh
cd "$(dirname "$0")/.." || exit 2
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
fail() { echo "C-tests-48: $*"; exit 1; }
old=puron-settings
cargo metadata --format-version 1 --no-deps --offline > "$tmp/meta.json" 2> "$tmp/meta.err" \
    || { cat "$tmp/meta.err"; fail "cargo metadata failed"; }
bins=$(jq -r '[.packages[] | select(.name != "puron-testkit") | .targets[]
    | select(.kind | index("bin")) | .name] | sort | join(" ")' "$tmp/meta.json") \
    || fail "jq failed"
[ "$bins" = puron-applet ] || fail "binary targets: $bins, want puron-applet alone"
n=$(jq --arg p "$old" '[.packages[] | select(.name == $p)] | length' "$tmp/meta.json")
[ "$n" = 0 ] || fail "the workspace has a package $old"
make -n install-bin DESTDIR=/c48 PREFIX=/usr > "$tmp/dry" 2>&1 || { cat "$tmp/dry"; fail "make -n install-bin failed"; }
grep -q 'puron-applet' "$tmp/dry" || fail "the dry run does not name puron-applet"
[ "$(grep -c "$old" "$tmp/dry")" = 0 ] || fail "the dry run names $old"
for f in tools/make-deb.sh tools/check-deb.sh debian/copyright tools/flatpak-probe.sh \
    flatpak/io.github.elalonde.Puron.json; do
    [ -f "$f" ] || fail "no $f"
    [ "$(grep -c "$old" "$f")" = 0 ] || fail "$f names $old"
done
meta=data/io.github.elalonde.Puron.metainfo.xml
sed -n '/<provides>/,/<\/provides>/p' "$meta" > "$tmp/provides"
grep -q '<binary>puron-applet</binary>' "$tmp/provides" || fail "provides has no puron-applet binary"
[ "$(grep -c "$old" "$tmp/provides")" = 0 ] || fail "provides names $old"
echo "C-tests-48: ok"
