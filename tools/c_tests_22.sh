#!/bin/sh
# Checks that make install is the one install list of both packages: the
# Debian package and the Flatpak each run it, with PREFIX=/usr and
# PREFIX=/app, and neither copies files of its own. A dry run of make
# install with PREFIX=/app (make -n prints the commands and runs none, so
# nothing is built) must name the binaries, the desktop entry, the panel
# icons, both forms of the app icon and the metainfo, all under the
# prefix, and no license directory, which only the Flatpak's own target
# writes. A real run of the targets other than the build must then write
# exactly the expected files. Last, tools/check-deb.sh must list the app
# icon and the metainfo, so that a package without them fails its check.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_22.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-22: FAIL $*"; exit 1; }
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
make -n install DESTDIR=/c22 PREFIX=/app > "$tmp/dry.txt" 2>&1 \
    || { cat "$tmp/dry.txt"; fail "make -n install"; }
for p in /c22/app/bin /c22/app/share/applications/io.github.elalonde.Puron.desktop \
    /c22/app/share/icons/hicolor/scalable/status /c22/app/share/icons/hicolor/scalable/apps \
    /c22/app/share/icons/hicolor/symbolic/apps /c22/app/share/metainfo; do
    n=$(grep -cF "$p" "$tmp/dry.txt")
    [ "$n" -ge 1 ] || fail "make -n install names no $p"
done
n=$(grep -cF share/licenses "$tmp/dry.txt")
[ "$n" = 0 ] || fail "make -n install names share/licenses"
root=$tmp/root
make -s install-icons install-desktop install-appicon install-metainfo DESTDIR="$root" PREFIX=/app \
    > "$tmp/install.txt" 2>&1 || { cat "$tmp/install.txt"; fail "make install-icons install-desktop install-appicon install-metainfo"; }
{
    echo "$root/app/share/applications/io.github.elalonde.Puron.desktop"
    echo "$root/app/share/metainfo/io.github.elalonde.Puron.metainfo.xml"
    echo "$root/app/share/icons/hicolor/scalable/apps/io.github.elalonde.Puron.svg"
    echo "$root/app/share/icons/hicolor/symbolic/apps/io.github.elalonde.Puron-symbolic.svg"
    for f in data/icons/hicolor/scalable/status/*.svg; do
        echo "$root/app/share/icons/hicolor/scalable/status/${f##*/}"
    done
} | LC_ALL=C sort > "$tmp/want"
find "$root" -type f | LC_ALL=C sort > "$tmp/got"
cmp -s "$tmp/want" "$tmp/got" \
    || fail "the installed files differ: $(diff "$tmp/want" "$tmp/got" | grep '^[<>]' | head -n 3 | tr '\n' ' ')"
for p in usr/share/metainfo/io.github.elalonde.Puron.metainfo.xml \
    usr/share/icons/hicolor/scalable/apps/io.github.elalonde.Puron.svg \
    usr/share/icons/hicolor/symbolic/apps/io.github.elalonde.Puron-symbolic.svg; do
    n=$(grep -cF "$p" tools/check-deb.sh)
    [ "$n" -ge 1 ] || fail "tools/check-deb.sh does not name $p"
done
echo "C-tests-22: ok"
exit 0
