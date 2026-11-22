#!/bin/sh
# Checks that no translation can make the build fail. Over a copy of
# i18n/ with three faulty translations added, tools/catalog-report.sh
# must exit 0 and describe each one exactly: eo is a contributor's draft
# (the testkit's fixtures/loc/contrib/puron.ftl) with missing, unknown
# and unreadable entries; eo_bad is the same file under a folder name
# that is no language tag; de-AT is a file that is not UTF-8. The lines
# of the shipped translations are printed but not compared, so that the
# check needs no list of the languages that ship.
cd "$(dirname "$0")/.." || exit 2
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
draft=crates/puron-testkit/fixtures/loc/contrib/puron.ftl
cp -R i18n "$tmp/i18n" || exit 2
mkdir "$tmp/i18n/eo" "$tmp/i18n/eo_bad" "$tmp/i18n/de-AT" || exit 2
cp "$draft" "$tmp/i18n/eo/puron.ftl" || exit 2
cp "$draft" "$tmp/i18n/eo_bad/puron.ftl" || exit 2
printf 'about-back = \377\n' > "$tmp/i18n/de-AT/puron.ftl" || exit 2
sh tools/catalog-report.sh "$tmp/i18n" > "$tmp/out"
rc=$?
cat "$tmp/out"
cat > "$tmp/want" << 'EOF'
translation de-AT: not UTF-8, shown in English
translation eo: 3 of 70 entries, 2 unknown, 3 lines that do not parse
translation eo_bad: ignored, the folder name is not a language tag such as de or pt-BR
EOF
grep -E '^translation (de-AT|eo|eo_bad):' "$tmp/out" > "$tmp/got"
if [ "$rc" = 0 ] && cmp -s "$tmp/want" "$tmp/got"; then
    echo "c_i18n_10: ok"
    exit 0
fi
echo "c_i18n_10: the report exited $rc and printed for the faulty translations:"
cat "$tmp/got"
exit 1
