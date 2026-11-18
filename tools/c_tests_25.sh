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
#
# The script must also add cargo's source replacement for libcosmic
# without a rev, which the generator leaves out and without which the
# offline build cannot find that spelling of libcosmic. On a small list
# whose cargo configuration has only the crates.io entry, --append must
# keep that entry, add exactly the libcosmic table with no rev, and leave
# the other source and the configuration's other keys as they were. A
# second --append must change nothing, and a list without a cargo
# configuration must make it exit 1 and stay as it was. These runs use
# no network either.
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
cat > "$tmp/f.json" << 'EOF'
[{"type": "shell", "commands": ["true"]}, {"type": "inline", "contents": "[source.crates-io]\nreplace-with = \"vendored-sources\"\n", "dest": "cargo", "dest-filename": "config"}]
EOF
cat > "$tmp/f2.json" << 'EOF'
[{"type": "shell", "commands": ["true"]}]
EOF
cp "$tmp/f2.json" "$tmp/f2-before.json" || exit 2
jq -S '.[0]' "$tmp/f.json" > "$tmp/first-before.json" || exit 2
jq -S '.[1] | del(.contents)' "$tmp/f.json" > "$tmp/keys-before.json" || exit 2
sh "$s" --append "$tmp/f.json" > "$tmp/stdout.txt" 2>&1
r=$?
[ "$r" = 0 ] || fail "--append on a list with a cargo configuration: exit $r, want 0"
[ "$(jq length "$tmp/f.json")" = 2 ] || fail "--append: the list does not hold two sources"
jq -S '.[0]' "$tmp/f.json" > "$tmp/first-after.json" || fail "--append: the list is not JSON"
cmp -s "$tmp/first-before.json" "$tmp/first-after.json" || fail "--append changed the first source"
jq -S '.[1] | del(.contents)' "$tmp/f.json" > "$tmp/keys-after.json" || exit 2
cmp -s "$tmp/keys-before.json" "$tmp/keys-after.json" || fail "--append changed a key of the cargo configuration other than its contents"
python3 -I -c '
import json, sys, tomllib
got = tomllib.loads(json.load(open(sys.argv[1], encoding="utf-8"))[1]["contents"])
want = {"source": {"crates-io": {"replace-with": "vendored-sources"},
                   "libcosmic-default-branch": {"git": "https://github.com/pop-os/libcosmic",
                                                "replace-with": "vendored-sources"}}}
sys.exit(0 if got == want else 1)
' "$tmp/f.json" || fail "--append: the cargo configuration is not the crates.io entry plus the libcosmic entry with no rev"
cp "$tmp/f.json" "$tmp/f-once.json" || exit 2
sh "$s" --append "$tmp/f.json" > "$tmp/stdout.txt" 2>&1
r=$?
[ "$r" = 0 ] || fail "a second --append: exit $r, want 0"
cmp -s "$tmp/f-once.json" "$tmp/f.json" || fail "a second --append changed the list"
sh "$s" --append "$tmp/f2.json" > "$tmp/stdout.txt" 2>&1
r=$?
[ "$r" = 1 ] || fail "--append on a list without a cargo configuration: exit $r, want 1"
cmp -s "$tmp/f2-before.json" "$tmp/f2.json" || fail "--append on a list without a cargo configuration changed it"
echo "C-tests-25: ok"
exit 0
