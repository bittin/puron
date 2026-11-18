#!/bin/sh
# Checks the desktop entry and that every file which names Puron's app ID
# names the same one. make install-desktop into a temporary root with
# PREFIX=/app, as the Flatpak builds it, must give an entry that
# desktop-file-validate accepts, with the Exec path under /app and Icon
# set to the app ID. Flatpak exports only an icon whose name starts with
# the app ID, so with any other Icon value COSMIC Settings and the store
# would show no icon for Puron. The Icon value must then match the
# metainfo's id and launchable, the two app icon files, the settings ID
# in crates/puron-config/src/lib.rs (under which both packages keep the
# settings) and, once the directory flatpak/ exists, the Flatpak
# manifest's file name and id. The manifest must be the one JSON file
# there, so that no second manifest can name another ID; the cargo
# source list that the Flatpak build writes beside it is not counted.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_20.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-20: FAIL $*"; exit 1; }
# desktop-file-validate prints no version. Release 0.27 calls the desktop
# category COSMIC an error and 0.28 accepts it, so the need is checked by
# asking it to accept an entry in that category.
d=$(mktemp -d) || exit 2
printf "[Desktop Entry]\nType=Application\nName=x\nExec=x\nCategories=COSMIC;\n" > "$d/x.desktop"
desktop-file-validate "$d/x.desktop" > /dev/null 2>&1; r=$?
rm -rf "$d"
[ "$r" -eq 0 ] || { echo "HOST NEED desktop-file-validate (desktop-file-utils 0.28 or later)"; exit 1; }
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
make -s install-desktop DESTDIR="$tmp/root" PREFIX=/app > "$tmp/install.txt" 2>&1 \
    || { cat "$tmp/install.txt"; fail "make install-desktop"; }
entry=$tmp/root/app/share/applications/io.github.elalonde.Puron.desktop
test -f "$entry" || fail "not installed: $entry"
desktop-file-validate "$entry" > "$tmp/validate.txt" 2>&1 \
    || { cat "$tmp/validate.txt"; fail "desktop-file-validate"; }
n=$(grep -cx 'Exec=/app/bin/puron-applet' "$entry")
[ "$n" = 1 ] || fail "grep -cx 'Exec=/app/bin/puron-applet' prints $n"
n=$(grep -cx 'Icon=io.github.elalonde.Puron' "$entry")
[ "$n" = 1 ] || fail "grep -cx 'Icon=io.github.elalonde.Puron' prints $n"
a=$(sed -n 's/^Icon=//p' "$entry")
# The component's own id is the one indented by two spaces; the <id>
# inside <provides> is deeper.
meta=data/io.github.elalonde.Puron.metainfo.xml
n=$(grep -cxF "  <id>$a</id>" "$meta")
[ "$n" = 1 ] || fail "the metainfo's id is not $a"
n=$(grep -cxF "  <launchable type=\"desktop-id\">$a.desktop</launchable>" "$meta")
[ "$n" = 1 ] || fail "the metainfo's launchable is not $a.desktop"
test -f "data/icons/hicolor/scalable/apps/$a.svg" || fail "no data/icons/hicolor/scalable/apps/$a.svg"
test -f "data/icons/hicolor/symbolic/apps/$a-symbolic.svg" \
    || fail "no data/icons/hicolor/symbolic/apps/$a-symbolic.svg"
n=$(grep -cxF "pub const CONFIG_ID: &str = \"$a\";" crates/puron-config/src/lib.rs)
[ "$n" = 1 ] || fail "CONFIG_ID in crates/puron-config/src/lib.rs is not $a"
if [ -d flatpak ]; then
    jq --version > /dev/null 2>&1 || { echo "HOST NEED jq"; exit 1; }
    manifests=$(find flatpak -maxdepth 1 -name '*.json' ! -name cargo-sources.json | LC_ALL=C sort | tr '\n' ' ')
    [ "$manifests" = "flatpak/$a.json " ] || fail "the JSON files of flatpak/ are '$manifests', want flatpak/$a.json"
    [ "$(jq -r .id "flatpak/$a.json")" = "$a" ] || fail "the id of flatpak/$a.json is not $a"
fi
echo "C-tests-20: ok"
exit 0
