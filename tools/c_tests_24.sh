#!/bin/sh
# Checks that the Flatpak installs Puron through the Makefile alone, the
# one install list that the Debian package uses too. The manifest's
# module puron may only run make install and make install-licenses, so a
# file that one package installs by a command of its own cannot be
# missing from the other. Over all modules, the one command that is not
# make is the libsensors module's install of the lm-sensors license.
# The Makefile's install target must still name its five parts, and
# install-licenses must exist.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_24.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-24: FAIL $*"; exit 1; }
jq --version > /dev/null 2>&1 || { echo "HOST NEED jq"; exit 1; }
m=flatpak/io.github.elalonde.Puron.json
test -f "$m" || fail "no $m"
check() { jq -e "$1" "$m" > /dev/null 2>&1 || fail "$2"; }
check '.modules[1]."build-commands" == [
    "make install PREFIX=/app",
    "make install-licenses PREFIX=/app VENDOR_DIR=/run/build/puron/cargo/vendor"
  ]' "the build commands of the module puron"
check '.modules[1].sources as $s |
    ($s | length) == 2
    and ($s[0] | type) == "object" and $s[0].type == "dir" and $s[0].path == ".."
    and $s[1] == "cargo-sources.json"' "the sources of the module puron"
check '[.modules[]."build-commands"[] | select(startswith("make ") | not)]
    == ["install -Dm0644 COPYING.LGPL /app/share/licenses/io.github.elalonde.Puron/lm-sensors-COPYING.LGPL"]' \
    "a build command other than make, beside the lm-sensors license line"
line=$(grep '^install:' Makefile)
for t in install-bin install-icons install-desktop install-appicon install-metainfo; do
    printf '%s\n' "$line" | tr ' ' '\n' | grep -qx "$t" || fail "the install target does not name $t"
done
make -n install-licenses PREFIX=/app VENDOR_DIR=/v > /dev/null 2>&1 || fail "make -n install-licenses"
echo "C-tests-24: ok"
exit 0
