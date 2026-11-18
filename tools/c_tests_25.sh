#!/bin/sh
# Checks that the Flatpak's cargo source list is generated, never
# committed, and made only by the pinned generator. A committed list
# would fall behind Cargo.lock; a generator other than the pinned one
# would decide unchecked what goes into the build.
#
# flatpak/cargo-sources.json must be ignored by git and absent from its
# index. tools/flatpak-cargo-sources.sh must name the generator's commit
# and its SHA-256 once each, and must refuse a generator of another hash
# before running it: given a one-line Python file that prints "other", it
# exits 1, names the sha256 in its output, prints no "other" and writes
# no output file. That run uses no network.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_25.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-25: FAIL $*"; exit 1; }
n=$(grep -cx 'flatpak/cargo-sources.json' .gitignore)
[ "$n" = 1 ] || fail "grep -cx 'flatpak/cargo-sources.json' .gitignore prints $n"
git check-ignore -q flatpak/cargo-sources.json || fail "git does not ignore flatpak/cargo-sources.json"
[ -z "$(git ls-files flatpak/cargo-sources.json)" ] || fail "flatpak/cargo-sources.json is in git's index"
s=tools/flatpak-cargo-sources.sh
n=$(grep -c 41c20aa10819cdb2a4f3ca171758a96d1955c018 "$s")
[ "$n" = 1 ] || fail "the generator's commit is named $n times in $s"
n=$(grep -c 0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380 "$s")
[ "$n" = 1 ] || fail "the generator's hash is named $n times in $s"
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
echo 'print("other")' > "$tmp/generator.py"
PURON_FLATPAK_GENERATOR=$tmp/generator.py sh "$s" "$tmp/out.json" > "$tmp/stdout.txt" 2> "$tmp/stderr.txt"
r=$?
[ "$r" = 1 ] || fail "a generator of another hash: exit $r, want 1"
cat "$tmp/stdout.txt" "$tmp/stderr.txt" | grep -q sha256 || fail "a generator of another hash: the output names no sha256"
grep -q other "$tmp/stdout.txt" && fail "a generator of another hash was run"
test -e "$tmp/out.json" && fail "a generator of another hash: an output file was written"
echo "C-tests-25: ok"
exit 0
