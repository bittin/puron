#!/bin/sh
# Prints the lm-sensors release that the Flatpak builds its libsensors
# from, as one line "<tag> <commit>". The Flatpak manifest is the one
# file that names them: the Flatpak's source offer and the source archive
# both take them from here, so a new lm-sensors release changes one file.
# Exits 1 unless the sources of the manifest's module libsensors hold
# exactly one tag and one commit. A key written twice in one source
# counts twice, because a JSON reader would quietly keep only the last
# one and the two files could then name different releases.
# Usage: tools/lm-sensors-ref.sh
cd "$(dirname "$0")/.." || exit 2
python3 -I - flatpak/io.github.elalonde.Puron.json <<'EOF'
import json
import sys


class Obj(dict):
    """A JSON object that also keeps every key and value as written."""


def keep_pairs(pairs):
    obj = Obj(pairs)
    obj.pairs = pairs
    return obj


with open(sys.argv[1], encoding="utf-8") as f:
    manifest = json.load(f, object_pairs_hook=keep_pairs)
modules = [m for m in manifest.get("modules", [])
           if isinstance(m, dict) and m.get("name") == "libsensors"]
found = {"tag": [], "commit": []}
for module in modules:
    for source in module.get("sources", []):
        for key, value in getattr(source, "pairs", []):
            if key in found:
                found[key].append(value)
if len(modules) != 1 or len(found["tag"]) != 1 or len(found["commit"]) != 1:
    sys.exit("lm-sensors-ref: the module libsensors of the manifest must hold "
             "exactly one tag and one commit")
print(found["tag"][0], found["commit"][0])
EOF
