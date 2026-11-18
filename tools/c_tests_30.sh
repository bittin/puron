#!/bin/sh
# Checks tools/flatpak-cosmic-manifest.sh, which prints the manifest that
# the COSMIC Flatpak repository builds. That copy must be the manifest
# with one change: the first source of the module "puron" becomes the
# release's git source, with the tag v<version> and the given commit. Any
# other difference, such as a lost permission or a "dir" source left next
# to the git source, would ship a Flatpak that Puron's own build and CI
# never built.
#
# The script runs on this tree with the workspace version, and again on a
# copy of the script and the manifest whose Cargo.toml has another version,
# so that a tag that does not follow the version cannot pass because it
# happens to equal the current one. A wrong version, a commit that is not
# 40 hexadecimal digits, and no arguments must each exit 1 with nothing on
# standard output. The copy is generated, never committed, so no file
# under flatpak/ may name it.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_30.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-30: FAIL $*"; exit 1; }
jq --version > /dev/null 2>&1 || { echo "HOST NEED jq"; exit 1; }
script=tools/flatpak-cosmic-manifest.sh
manifest=flatpak/io.github.elalonde.Puron.json
test -f "$script" || fail "no $script"
test -f "$manifest" || fail "no $manifest"
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
commit=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
version=$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || fail "no workspace version in Cargo.toml"

# copy_ok <tree> <version>: the script of <tree> run with <version> exits 0,
# and its output is the manifest of <tree> with only the first source of
# the module puron changed, to the git source of that version.
copy_ok() {
    sh "$1/$script" "$2" "$commit" > "$tmp/copy.json" 2> "$tmp/copy.err" ||
        { cat "$tmp/copy.err"; fail "$script $2 exits non-zero in $1"; }
    jq -S 'del(.modules[1].sources[0])' "$tmp/copy.json" > "$tmp/a.json" || fail "the copy for $2 is not JSON"
    jq -S 'del(.modules[1].sources[0])' "$1/$manifest" > "$tmp/b.json" || fail "$manifest is not JSON"
    cmp -s "$tmp/a.json" "$tmp/b.json" ||
        { diff "$tmp/b.json" "$tmp/a.json" | head -n 10; fail "the copy for $2 differs from the manifest beyond its first puron source"; }
    jq -e --arg t "v$2" --arg c "$commit" '.modules[1].name == "puron" and .modules[1].sources[0] == {
        "type": "git", "url": "https://github.com/elalonde/puron.git", "tag": $t, "commit": $c}' \
        "$tmp/copy.json" > /dev/null || fail "the first puron source of the copy for $2 is not the git source of v$2"
    jq -e '.modules[1].sources[1] == "cargo-sources.json"' "$tmp/copy.json" > /dev/null ||
        fail "the second puron source of the copy for $2 is not cargo-sources.json"
}
copy_ok . "$version"

# The same on a copy whose workspace version is another one.
other=9.8.7
[ "$version" != "$other" ] || other=9.8.6
mkdir -p "$tmp/tree/tools" "$tmp/tree/flatpak" || exit 2
cp "$script" "$tmp/tree/tools/" && cp "$manifest" "$tmp/tree/flatpak/" || exit 2
sed '/^\[workspace\.package\]/,/^\[/ s/^version = ".*"$/version = "'"$other"'"/' Cargo.toml > "$tmp/tree/Cargo.toml" || exit 2
copy_ok "$tmp/tree" "$other"

# refused <what> <args...>: exit 1 and nothing on standard output.
refused() {
    what=$1; shift
    sh "$script" "$@" > "$tmp/out" 2> /dev/null
    r=$?
    [ "$r" = 1 ] || fail "$what: exit $r, not 1"
    [ ! -s "$tmp/out" ] || fail "$what: wrote to standard output"
}
wrong=0.0.1
[ "$version" != "$wrong" ] || wrong=0.0.2
refused "version $wrong" "$wrong" "$commit"
refused "a commit of 39 digits" "$version" aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
refused "no argument"

[ -z "$(git ls-files flatpak/ | grep cosmic)" ] || fail "a tracked file under flatpak/ has cosmic in its name"
echo "C-tests-30: ok"
