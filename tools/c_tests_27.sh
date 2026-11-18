#!/bin/sh
# Checks the license files that the Flatpak carries and that the
# lm-sensors release it bundles is named in one file only.
#
# tools/lm-sensors-ref.sh must print the tag and the commit of the Flatpak
# manifest's libsensors source. make install-license-texts into a
# temporary root must install exactly Puron's LICENSE, the GPL-3.0 text
# and the source offer, each mode 644, plus the notice on the icon themes
# when the Flatpak is built on the COSMIC base app. The source offer is
# the Debian package's followed by the lm-sensors paragraph, with every
# placeholder filled: the version, the tag and the commit. Neither the
# tag nor the commit may be written in the Makefile, the source archive
# script or the paragraph's template, so that a new lm-sensors release
# cannot leave one of them behind. The Flatpak's libsensors carries one
# patch, so both the paragraph's template and the installed offer must
# name the patch file and must not claim the library is built without
# changes: the LGPL asks that the source offered for a changed library
# be the changed source. The libsensors module must install the
# lm-sensors license, which the LGPL asks to ship with the library.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_27.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-27: FAIL $*"; exit 1; }
jq --version > /dev/null 2>&1 || { echo "HOST NEED jq"; exit 1; }
m=flatpak/io.github.elalonde.Puron.json
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
sh tools/lm-sensors-ref.sh > "$tmp/ref.txt" 2>&1 || { cat "$tmp/ref.txt"; fail "tools/lm-sensors-ref.sh"; }
want=$(jq -r '.modules[0].sources[0] | .tag + " " + .commit' "$m")
[ "$(cat "$tmp/ref.txt")" = "$want" ] || fail "tools/lm-sensors-ref.sh prints '$(cat "$tmp/ref.txt")', want '$want'"
tag=${want% *}
commit=${want#* }
make -s install-license-texts DESTDIR="$tmp/root" PREFIX=/app > "$tmp/install.txt" 2>&1 \
    || { cat "$tmp/install.txt"; fail "make install-license-texts"; }
L=$tmp/root/app/share/licenses/io.github.elalonde.Puron
expect="$L/GPL-3.0.txt $L/LICENSE $L/SOURCE-OFFER"
if jq -e 'has("base")' "$m" > /dev/null; then
    expect="$L/GPL-3.0.txt $L/ICON-THEMES-NOTICE.txt $L/LICENSE $L/SOURCE-OFFER"
fi
got=$(find "$tmp/root" -type f | LC_ALL=C sort | tr '\n' ' ')
[ "$got" = "$expect " ] || fail "installed '$got', want '$expect '"
for f in $expect; do
    [ "$(stat -c %a "$f")" = 644 ] || fail "$f is not mode 644"
done
cmp -s "$L/LICENSE" LICENSE || fail "LICENSE differs"
cmp -s "$L/GPL-3.0.txt" data/licenses/GPL-3.0.txt || fail "GPL-3.0.txt differs"
if [ -f "$L/ICON-THEMES-NOTICE.txt" ]; then
    cmp -s "$L/ICON-THEMES-NOTICE.txt" flatpak/icon-themes-notice.txt || fail "ICON-THEMES-NOTICE.txt differs"
fi
offer=$L/SOURCE-OFFER
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
for v in "$version" "$tag" "$commit"; do
    [ "$(grep -cF "$v" "$offer")" -ge 1 ] || fail "SOURCE-OFFER does not hold $v"
done
grep -q '@' "$offer" && fail "SOURCE-OFFER holds an @"
a=$(wc -l < debian/source-offer.in)
b=$(wc -l < flatpak/source-offer-libsensors.in)
n=$(wc -l < "$offer")
[ "$n" = $((a + b + 1)) ] || fail "SOURCE-OFFER has $n lines, want $((a + b + 1))"
for f in Makefile tools/make-source-archive.sh flatpak/source-offer-libsensors.in; do
    for v in "$tag" "$commit"; do
        [ "$(grep -cF "$v" "$f")" = 0 ] || fail "$f names $v"
    done
done
for f in flatpak/source-offer-libsensors.in "$offer"; do
    [ "$(grep -cF flatpak/lm-sensors-quiet-sysfs-root.patch "$f")" = 1 ] \
        || fail "$f does not name flatpak/lm-sensors-quiet-sysfs-root.patch on exactly one line"
    [ "$(grep -c 'without changes' "$f")" = 0 ] || fail "$f says the library is built without changes"
done
grep -q 'lm-sensors-ref\.sh' tools/make-source-archive.sh \
    || fail "tools/make-source-archive.sh does not take the release from tools/lm-sensors-ref.sh"
jq -e '.modules[0]."build-commands"[1] == "install -Dm0644 COPYING.LGPL /app/share/licenses/io.github.elalonde.Puron/lm-sensors-COPYING.LGPL"' \
    "$m" > /dev/null || fail "the libsensors module does not install COPYING.LGPL"
echo "C-tests-27: ok"
exit 0
