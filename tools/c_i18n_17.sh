#!/bin/sh
# Checks that lint A reads the comments of the English catalog and of no
# other catalog, so that a translator's comment never fails the build. In
# a scratch tree with lint A and its patterns, the testkit's
# fixtures/loc/contrib/puron.ftl, whose first comment names a home
# folder, is placed as i18n/en/puron.ftl and as i18n/eo/puron.ftl. Lint A
# must pass the Esperanto file and refuse the English one for that path.
cd "$(dirname "$0")/.." || exit 2
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
draft=crates/puron-testkit/fixtures/loc/contrib/puron.ftl
mkdir -p "$tmp/tools" "$tmp/i18n/en" "$tmp/i18n/eo" || exit 2
cp tools/lint-a.sh tools/lint-patterns "$tmp/tools/" || exit 2
cp "$draft" "$tmp/i18n/en/puron.ftl" || exit 2
cp "$draft" "$tmp/i18n/eo/puron.ftl" || exit 2
sh "$tmp/tools/lint-a.sh" i18n/eo/puron.ftl > "$tmp/eo" 2>&1
eo=$?
sh "$tmp/tools/lint-a.sh" i18n/en/puron.ftl > "$tmp/en" 2>&1
en=$?
if [ "$eo" = 0 ] && grep -qx 'lint-a: ok' "$tmp/eo" &&
    [ "$en" = 1 ] && grep -q '^lint-a: i18n/en/puron.ftl:1: home-path:' "$tmp/en"; then
    echo "c_i18n_17: ok"
    exit 0
fi
echo "c_i18n_17: lint A on the Esperanto file exited $eo:"
cat "$tmp/eo"
echo "c_i18n_17: lint A on the English file exited $en:"
cat "$tmp/en"
exit 1
