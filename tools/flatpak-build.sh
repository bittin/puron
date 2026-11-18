#!/bin/sh
# Builds the Flatpak from this tree and writes it as one bundle file,
# which installs with flatpak install --user <bundle file> and takes its
# runtime from Flathub. It is the one build line of the Flatpak, for a
# local build and for CI.
#
# It first checks the host and prints one line "HOST NEED <name>" for
# each missing tool, runtime or SDK extension, and then stops before it
# builds. With PURON_FLATPAK_INSTALL_DEPS=1 the runtimes are not checked;
# flatpak-builder installs the missing ones from Flathub instead.
#
# The build runs with the options of the COSMIC Flatpak repository's own
# build, so that a manifest that builds here builds there too, with one
# exception: it runs without --sandbox. The manifest's source is this
# tree, the folder above the manifest's folder, and flatpak-builder
# refuses a source outside the manifest's folder in a sandboxed build.
# The copy of the manifest for that repository takes the source from git
# instead, so the repository still builds it with --sandbox.
# Its state, the build directory and the repository are kept
# outside the tree, in PURON_FLATPAK_DIR (default
# ~/.cache/puron-flatpak). The build has no time limit.
# Exits 0 when every step did, and prints the bundle's path, size and
# SHA-256.
# Usage: tools/flatpak-build.sh [<bundle file>]   (default /tmp/puron-test.flatpak)
cd "$(dirname "$0")/.." || exit 2
bundle=${1:-/tmp/puron-test.flatpak}
dir=${PURON_FLATPAK_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/puron-flatpak}
manifest=flatpak/io.github.elalonde.Puron.json
app=io.github.elalonde.Puron
need=0
for tool in flatpak flatpak-builder python3; do
    command -v "$tool" > /dev/null 2>&1 || { echo "HOST NEED $tool"; need=1; }
done
[ "$need" = 0 ] || exit 1
if [ "${PURON_FLATPAK_INSTALL_DEPS:-0}" != 1 ]; then
    # The runtime, the SDK and the SDK extensions that the manifest names,
    # each on the manifest's branch, one per line.
    refs=$(python3 -I -c '
import json, sys
m = json.load(open(sys.argv[1], encoding="utf-8"))
for r in [m["runtime"], m["sdk"]] + m.get("sdk-extensions", []):
    print(r + "//" + m["runtime-version"])
' "$manifest") || exit 1
    for ref in $refs; do
        flatpak info "$ref" > /dev/null 2>&1 || { echo "HOST NEED $ref"; need=1; }
    done
    [ "$need" = 0 ] || exit 1
    deps=
else
    deps=--install-deps-from=flathub
fi
sh tools/flatpak-cargo-sources.sh || exit 1
mkdir -p "$dir" || exit 2
# $deps is either empty or one word, so it is left unquoted on purpose.
# shellcheck disable=SC2086
flatpak-builder --user --force-clean $deps \
    --state-dir="$dir/state" --repo="$dir/repo" "$dir/build" "$manifest" \
    || { echo "flatpak-build: flatpak-builder failed"; exit 1; }
flatpak build-bundle "$dir/repo" "$bundle" "$app" \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    || { echo "flatpak-build: flatpak build-bundle failed"; exit 1; }
size=$(stat -c %s "$bundle") || exit 1
sum=$(sha256sum "$bundle" | cut -d' ' -f1)
echo "flatpak-build: $bundle $size bytes sha256 $sum"
