#!/bin/sh
# Writes flatpak/cargo-sources.json: every crate of Cargo.lock as a
# flatpak-builder source, so that the Flatpak builds with no network. The
# file is made from Cargo.lock at build time and never committed, so it
# cannot fall behind the lock file.
#
# The list is made by flatpak-cargo-generator.py of the Flatpak project's
# flatpak-builder-tools, at one fixed commit. The script downloads that
# file and runs it only when its SHA-256 is the one written here: a
# changed or replaced generator would decide what goes into the build, so
# it must not run. With PURON_FLATPAK_GENERATOR=<file> the script takes
# that file instead of the download and checks its hash all the same.
# The generator needs the Python modules aiohttp and tomlkit, git, and
# the network to read the crates' git repositories.
#
# A run that fails leaves no output file behind.
# Usage: tools/flatpak-cargo-sources.sh [<output file>]
cd "$(dirname "$0")/.." || exit 2
rev=41c20aa10819cdb2a4f3ca171758a96d1955c018
sum=0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380
out=${1:-flatpak/cargo-sources.json}
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
gen=$tmp/flatpak-cargo-generator.py
if [ -n "$PURON_FLATPAK_GENERATOR" ]; then
    cp "$PURON_FLATPAK_GENERATOR" "$gen" || exit 1
else
    curl -fsSL -o "$gen" \
        "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/$rev/cargo/flatpak-cargo-generator.py" \
        || { echo "flatpak-cargo-sources: cannot download the generator"; exit 1; }
fi
got=$(sha256sum "$gen" | cut -d' ' -f1)
[ "$got" = "$sum" ] \
    || { echo "flatpak-cargo-sources: the generator's sha256 is $got, not $sum; not run"; exit 1; }
python3 -I -c 'import aiohttp, tomlkit' 2> /dev/null \
    || { echo "HOST NEED python3 aiohttp tomlkit"; exit 1; }
python3 -I "$gen" Cargo.lock -o "$tmp/cargo-sources.json" > "$tmp/log.txt" 2>&1 \
    || { tail -n 20 "$tmp/log.txt"; echo "flatpak-cargo-sources: the generator failed"; exit 1; }
mv "$tmp/cargo-sources.json" "$out" || exit 1
echo "flatpak-cargo-sources: $out"
