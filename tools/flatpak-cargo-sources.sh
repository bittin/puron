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
# Cargo.lock names libcosmic's git repository under two spellings, with
# and without a ?rev= query, because Puron asks for one revision while
# other crates taken from git ask for its default branch. The
# generator keys git sources by the URL without its query, so the cargo
# configuration that it writes (the last source of the list, an inline
# file "config" for cargo) replaces only the spelling with the rev, and
# the offline build stops because cargo cannot fetch the other one. The
# script therefore adds a second replacement for libcosmic, with no rev,
# to that configuration. When the list does not hold exactly one cargo
# configuration, it exits 1; when the replacement is already there, it
# leaves the list as it is. Cargo.lock itself is never changed.
# With --append <file> the script applies only this last step, in place,
# to a list made earlier: it runs no generator and uses no network.
#
# A run that fails leaves no output file behind, and with --append leaves
# the file as it was.
# Usage: tools/flatpak-cargo-sources.sh [<output file>]
#        tools/flatpak-cargo-sources.sh --append <file>
if [ "$1" = --append ]; then
    [ -n "$2" ] || { echo "usage: tools/flatpak-cargo-sources.sh --append <file>"; exit 2; }
    # The file is named from the caller's folder, before the cd below.
    case "$2" in
        /*) ;;
        *) set -- "$1" "$PWD/$2" ;;
    esac
fi
cd "$(dirname "$0")/.." || exit 2
rev=41c20aa10819cdb2a4f3ca171758a96d1955c018
sum=0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT

# Adds the replacement for libcosmic without a rev to the cargo
# configuration in the list $1. The new list is written to a second file
# and moved over $1 only when the step succeeded and changed something.
append() {
    rm -f "$tmp/appended.json"
    python3 -I -c '
import json, sys, tomllib
name = "libcosmic-default-branch"
url = "https://github.com/pop-os/libcosmic"
src, dst = sys.argv[1], sys.argv[2]
with open(src, encoding="utf-8") as f:
    sources = json.load(f)
configs = [s for s in sources if isinstance(s, dict) and s.get("type") == "inline"
           and s.get("dest") == "cargo" and s.get("dest-filename") == "config"]
if len(configs) != 1:
    print("flatpak-cargo-sources: %s has %d cargo configurations, not 1" % (src, len(configs)))
    sys.exit(1)
text = configs[0].get("contents", "")
if name in tomllib.loads(text).get("source", {}):
    sys.exit(0)
if text and not text.endswith("\n"):
    text += "\n"
configs[0]["contents"] = text + "\n[source.%s]\ngit = \"%s\"\nreplace-with = \"vendored-sources\"\n" % (name, url)
with open(dst, "w", encoding="utf-8") as f:
    json.dump(sources, f, indent=4)
' "$1" "$tmp/appended.json" || { echo "flatpak-cargo-sources: cannot add the libcosmic source to $1"; return 1; }
    if [ -e "$tmp/appended.json" ]; then
        mv "$tmp/appended.json" "$1" || return 1
    fi
    return 0
}

if [ "$1" = --append ]; then
    append "$2" || exit 1
    echo "flatpak-cargo-sources: $2"
    exit 0
fi

out=${1:-flatpak/cargo-sources.json}
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
append "$tmp/cargo-sources.json" || exit 1
mv "$tmp/cargo-sources.json" "$out" || exit 1
echo "flatpak-cargo-sources: $out"
