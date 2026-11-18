#!/bin/sh
# Prints the Flatpak manifest in the form that the COSMIC Flatpak
# repository (github.com/pop-os/cosmic-flatpak) builds: Puron's manifest,
# flatpak/io.github.elalonde.Puron.json, with the first source of the
# module "puron" replaced by the release's git source:
#
#   {"type": "git", "url": "https://github.com/elalonde/puron.git",
#    "tag": "v<version>", "commit": "<commit>"}
#
# Everything else stays as it is. Puron's own manifest builds from the
# directory it sits in (a "dir" source), which is what a local or CI build
# needs, while that repository builds a published tag; generating the copy
# keeps one manifest, so the two can never disagree on the build steps or
# the permissions. The copy is never edited by hand and never committed
# here: the release tool writes it for the pull request to that repository.
# The module "libsensors" keeps its patch source, which names
# lm-sensors-quiet-sysfs-root.patch beside the manifest, so the copy
# builds only with that file next to it; the release tool copies it
# there from flatpak/ of the same tag.
#
# The version must be the workspace version of Cargo.toml, so that the tag
# in the copy is the release that this tree is. The commit is the tag's
# commit, given as 40 lower-case hexadecimal digits, so that the build is
# pinned even if the tag were moved.
#
# Exits 0 with the copy on standard output. Exits 1, with the reason on
# standard error and nothing on standard output, when the arguments are
# wrong, when the version is not the workspace version, or when the module
# "puron" is missing or does not start with a "dir" source.
# Usage: tools/flatpak-cosmic-manifest.sh <version> <commit>
cd "$(dirname "$0")/.." || exit 2
die() { echo "flatpak-cosmic-manifest: $*" >&2; exit 1; }
jq --version > /dev/null 2>&1 || { echo "HOST NEED jq" >&2; exit 1; }
[ $# = 2 ] || die "usage: tools/flatpak-cosmic-manifest.sh <version> <commit>"
version=$1 commit=$2
manifest=flatpak/io.github.elalonde.Puron.json
workspace=$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
[ -n "$workspace" ] || die "no version in [workspace.package] of Cargo.toml"
[ "$version" = "$workspace" ] || die "version $version is not the workspace version $workspace"
printf '%s\n' "$commit" | grep -qxE '[0-9a-f]{40}' || die "commit '$commit' is not 40 lower-case hexadecimal digits"
[ -f "$manifest" ] || die "no $manifest"
jq -e '[.modules[] | select(.name == "puron")] | length == 1' "$manifest" > /dev/null 2>&1 ||
    die "$manifest has no single module named puron"
jq -e '.modules[] | select(.name == "puron") | .sources[0] | type == "object" and .type == "dir"' "$manifest" > /dev/null 2>&1 ||
    die "the module puron of $manifest does not start with a dir source"
# The copy goes to a variable first, so that a jq failure prints nothing.
copy=$(jq --arg tag "v$version" --arg commit "$commit" '
    .modules |= map(if .name == "puron" then .sources[0] = {
        "type": "git",
        "url": "https://github.com/elalonde/puron.git",
        "tag": $tag,
        "commit": $commit
    } else . end)' "$manifest") || die "jq failed on $manifest"
printf '%s\n' "$copy"
