#!/bin/sh
# Prints one line for each translation, every catalog <language>/puron.ftl
# but English: how many English texts it translates, how many of its
# entries the program does not know, and how many lines it holds that
# the program cannot read. A translation is a contributor's file and the
# program shows in English whatever it lacks or gets wrong, so nothing a
# translation holds makes this script fail: it is information for the
# translator, printed by make check.
# Usage: sh tools/catalog-report.sh [directory]; the directory holds the
# catalog folders and is i18n/ of the source tree when none is given.
# Exits 2 only when <directory>/en/puron.ftl is missing, a fault of the
# tree and not of a translation; otherwise 0.
#
# The lines are read with simple rules that follow the Fluent format
# closely enough for a translator: a line that starts with "#" is a
# comment; "name = text" starts a message and "-name = text" a term; an
# indented line continues the entry above it; any other line is one that
# does not parse. A message counts as one that does not parse when it has
# no text or its braces do not pair up. The program itself may still show
# a message in English that this report counts as translated, such as one
# that names a term the catalog lacks.
LC_ALL=C
export LC_ALL
dir=${1:-$(dirname "$0")/../i18n}
en=$dir/en/puron.ftl
[ -f "$en" ] || { echo "catalog-report: no $en" >&2; exit 2; }
for f in "$dir"/*/puron.ftl; do
    [ -f "$f" ] || continue
    name=${f%/puron.ftl}
    name=${name##*/}
    [ "$name" = en ] && continue
    if ! printf '%s\n' "$name" | grep -Eq '^[a-z]{2,3}(-[A-Z][a-z]{3})?(-([A-Z]{2}|[0-9]{3}))?$'; then
        echo "translation $name: ignored, the folder name is not a language tag such as de or pt-BR"
        continue
    fi
    if ! iconv -f UTF-8 -t UTF-8 "$f" > /dev/null 2>&1; then
        echo "translation $name: not UTF-8, shown in English"
        continue
    fi
    # The first file is English, read only for its message IDs; the
    # second is the translation.
    awk -v name="$name" '
        function finish(  v, o, c) {
            if (cur == "msg") {
                v = val; gsub(/[ \t]/, "", v)
                o = gsub(/\{/, "{", val); c = gsub(/\}/, "}", val)
                if (v == "" || o != c) { if (file == 2) broken++ }
                else if (file == 1) known[id] = 1
                else good[id] = 1
            }
            cur = ""
        }
        FNR == 1 { finish(); file++ }
        { sub(/\r$/, "") }
        /^[ \t]*$/ { next }
        /^#/ { finish(); next }
        /^[A-Za-z][A-Za-z0-9_-]* *=/ {
            finish(); cur = "msg"
            id = $0; sub(/[ =].*/, "", id)
            val = $0; sub(/^[^=]*=/, "", val)
            next
        }
        /^-[A-Za-z][A-Za-z0-9_-]* *=/ {
            finish(); cur = "other"
            t = $0; sub(/[ =].*/, "", t)
            if (file == 2) terms[t] = 1
            next
        }
        /^[ \t]/ {
            if (cur == "msg") val = val " " $0
            else if (cur == "") { if (file == 2) bad++; cur = "other" }
            next
        }
        { finish(); if (file == 2) bad++; cur = "other" }
        END {
            finish()
            n = 0; for (i in known) n++
            t = 0; u = 0
            for (i in good) { if (i in known) t++; else u++ }
            for (i in terms) u++
            printf "translation %s: %d of %d entries, %d unknown, %d lines that do not parse\n", name, t, n, u, bad + broken
        }' "$en" "$f"
done
exit 0
