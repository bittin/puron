#!/bin/sh
# Builds target/source/puron-<version>-source.tar.xz, the complete
# corresponding source of a release: the tree at HEAD, the locked
# dependencies vendored with the cargo config for an offline build, the
# third-party notices, and the lm-sensors source that the Flatpak builds
# its libsensors from. It needs the network for the last one.
# Usage: tools/make-source-archive.sh   (or make source-archive)
cd "$(dirname "$0")/.." || exit 2
target=${CARGO_TARGET_DIR:-target}
case $target in /*) ;; *) target=$PWD/$target ;; esac
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "make-source-archive: no version in Cargo.toml"; exit 2; }
name=puron-$version
work=$target/source
rm -rf "$work" && mkdir -p "$work" || exit 2
git archive --prefix="$name/" HEAD | tar -xf - -C "$work" || exit 2
sh tools/vendor-tree.sh "$work/$name" "$work/scratch" || exit 1
python3 -I tools/gen-notices.py "$work/$name/vendor" "$work/scratch/metadata.json" \
    "$work/$name/THIRD-PARTY-NOTICES.txt" || exit 1
# The Flatpak bundles libsensors, so the archive also holds the complete
# source of the lm-sensors release that the Flatpak manifest names, in
# the folder lm-sensors-<tag> inside the top folder. It is fetched by
# commit, which is why this script needs the network; a fetch that fails
# or gives another commit stops it. The cargo build of the archive does
# not read that folder.
ref=$(sh tools/lm-sensors-ref.sh) || exit 1
lms_tag=${ref% *}
lms_commit=${ref#* }
lms_url=$(jq -er '.modules[] | select(.name == "libsensors") | .sources[0].url' \
    flatpak/io.github.elalonde.Puron.json) || { echo "make-source-archive: no lm-sensors URL"; exit 1; }
lms=$work/scratch/lm-sensors
git init -q "$lms" || exit 2
git -C "$lms" fetch -q --depth 1 "$lms_url" "$lms_commit" \
    || { echo "make-source-archive: cannot fetch lm-sensors $lms_commit"; exit 1; }
[ "$(git -C "$lms" rev-parse FETCH_HEAD)" = "$lms_commit" ] \
    || { echo "make-source-archive: the lm-sensors fetch is not $lms_commit"; exit 1; }
git -C "$lms" archive --prefix="$name/lm-sensors-$lms_tag/" FETCH_HEAD | tar -xf - -C "$work" || exit 2
out=$work/$name-source.tar.xz
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$(git log -1 --format=%ct)" \
    -C "$work" -cf - "$name" | xz -T0 -6 > "$out" || exit 1
rm -rf "${work:?}/$name" "$work/scratch"
echo "make-source-archive: $out"
