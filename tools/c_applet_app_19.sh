#!/bin/sh
# Checks the libcosmic features that puron-applet enables (it is the one
# package that uses libcosmic) against the list below (diff must exit
# 0), then runs cargo deny check licenses on the workspace (must exit 0).
# Usage: tools/c_applet_app_19.sh   (make check gives it 600 s, for
# cargo deny; its output goes to $PURON_DENY_OUT, default
# ${CARGO_TARGET_DIR:-target}/check/deny-features.txt)
# The feature list is the declared one (--no-deps: no resolution, so
# a changed list needs no download).
cd "$(dirname "$0")/.." || exit 2
export RUSTUP_AUTO_INSTALL=0
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
cargo metadata --format-version 1 --no-deps --offline > "$tmp/meta.json" || exit 2
jq -r '.packages[] | select(.name == "puron-applet") | .dependencies[]
    | select(.name == "libcosmic") | .features[]' "$tmp/meta.json" \
    | sort > "$tmp/got" || exit 2
printf '%s\n' winit tokio a11y dbus-config x11 wayland multi-window applet \
    | sort > "$tmp/want"
rc=0
diff "$tmp/want" "$tmp/got" || { echo "C-applet.app-19: puron-applet features differ"; exit 1; }
out=${PURON_DENY_OUT:-${CARGO_TARGET_DIR:-target}/check/deny-features.txt}
mkdir -p "$(dirname "$out")" || exit 2
"$HOME/.cargo/bin/cargo" deny check licenses > "$out" 2>&1
d=$?
echo "C-applet.app-19: cargo deny exit $d"
[ "$d" -eq 0 ] || rc=1
exit $rc
