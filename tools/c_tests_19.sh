#!/bin/sh
# Checks the AppStream metainfo, the file from which software stores take
# Puron's name, summary, description, links and category. make
# install-metainfo into a temporary root must install it unchanged with
# mode 644, and appstreamcli must accept the installed file in strict
# mode, the way the store's catalog step reads it. The validator does not
# know which values Puron must have, so the script then reads the file
# and compares each element with its expected value, in order. Among
# them is <provides> with com.system76.CosmicApplet: without it the
# file still validates, but the COSMIC Store would not list Puron as an
# applet or offer to place it on the panel.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_19.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-19: FAIL $*"; exit 1; }
python3 --version > /dev/null 2>&1 || { echo "HOST NEED python3"; exit 1; }
LC_ALL=C appstreamcli --version 2> /dev/null | grep -q "version: [1-9]" \
    || { echo "HOST NEED appstreamcli (AppStream 1.0 or later)"; exit 1; }
src=data/io.github.elalonde.Puron.metainfo.xml
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
make -s install-metainfo DESTDIR="$tmp/root" PREFIX=/app > "$tmp/install.txt" 2>&1 \
    || { cat "$tmp/install.txt"; fail "make install-metainfo"; }
m=$tmp/root/app/share/metainfo/io.github.elalonde.Puron.metainfo.xml
test -f "$m" || fail "not installed: $m"
cmp -s "$src" "$m" || fail "the installed metainfo differs from $src"
[ "$(stat -c %a "$m")" = 644 ] || fail "the installed metainfo has mode $(stat -c %a "$m")"
appstreamcli validate --no-net --strict "$m" > "$tmp/validate.txt" 2>&1 \
    || { cat "$tmp/validate.txt"; fail "appstreamcli validate --no-net --strict"; }
python3 -I - "$m" <<'EOF' || exit 1
import sys
import xml.etree.ElementTree as ET


def fail(msg):
    print(f"C-tests-19: FAIL {msg}")
    sys.exit(1)


try:
    root = ET.parse(sys.argv[1]).getroot()
except ET.ParseError as e:
    fail(f"the metainfo does not parse: {e}")

if root.tag != "component" or root.get("type") != "desktop-application":
    fail(f'the root is <{root.tag} type="{root.get("type")}">, not a desktop-application component')

# The children that the stores read, in the order the file must have them.
# Screenshots and release notes follow them; they have checks of their own.
ORDER = ["id", "name", "summary", "metadata_license", "project_license", "developer",
         "description", "launchable", "categories", "keywords", "url", "url", "url",
         "provides", "branding", "content_rating"]
seen = [c.tag for c in root if c.tag in set(ORDER)]
if seen != ORDER:
    fail(f"the elements are {seen}, want {ORDER}")
if root.find(".//icon") is not None:
    fail("the metainfo has an <icon> element; the icon comes from the desktop entry")


def one(tag):
    return root.find(tag)


def text_is(e, want, what):
    if e is None or e.text != want:
        fail(f"{what} is {None if e is None else repr(e.text)}, want {want!r}")


def children(e, what, want):
    """Check that e has exactly the children want, as (tag, attributes, text)."""
    got = [(c.tag, dict(c.attrib), c.text) for c in e]
    if got != want:
        fail(f"{what} has {got}, want {want}")


text_is(one("id"), "io.github.elalonde.Puron", "id")
text_is(one("name"), "Puron", "name")
text_is(one("summary"), "Monitor hardware sensors", "summary")
text_is(one("metadata_license"), "MIT", "metadata_license")
text_is(one("project_license"), "MIT AND GPL-3.0-only", "project_license")

dev = one("developer")
if dev.get("id") != "io.github.elalonde":
    fail(f'developer id is {dev.get("id")!r}, want "io.github.elalonde"')
children(dev, "developer", [("name", {}, "Eric Lalonde")])

children(one("description"), "description", [
    ("p", {}, "Puron shows temperatures, fan speeds, voltages and power readings in the panel "
              "and dock of the COSMIC™ desktop. Pin the readings that matter to the panel; "
              "the popup lists every sensor, grouped by device."),
    ("p", {}, "Puron reads lm-sensors, NVMe and SATA drives and NVIDIA GPUs. It only reads: "
              "it never changes a hardware setting."),
    ("p", {}, "The Flatpak package does not read liquidctl devices; the Debian package does."),
])

launch = one("launchable")
if launch.get("type") != "desktop-id":
    fail(f'launchable type is {launch.get("type")!r}, want "desktop-id"')
text_is(launch, "io.github.elalonde.Puron.desktop", "launchable")

children(one("categories"), "categories",
         [("category", {}, "System"), ("category", {}, "Monitor")])
children(one("keywords"), "keywords", [("keyword", {}, k) for k in (
    "sensors", "temperature", "fan", "voltage", "lm-sensors", "NVMe", "NVIDIA",
    "hardware", "panel")])

urls = [(u.get("type"), u.text) for u in root.findall("url")]
want_urls = [("homepage", "https://elalonde.dev/projects/puron/"),
             ("bugtracker", "https://github.com/elalonde/puron/issues"),
             ("vcs-browser", "https://github.com/elalonde/puron")]
if urls != want_urls:
    fail(f"url elements are {urls}, want {want_urls}")

children(one("provides"), "provides", [
    ("id", {}, "com.system76.CosmicApplet"),
    ("binary", {}, "puron-applet"),
    ("binary", {}, "puron-settings"),
])
children(one("branding"), "branding", [
    ("color", {"type": "primary", "scheme_preference": "light"}, "#f2b33d"),
    ("color", {"type": "primary", "scheme_preference": "dark"}, "#7a5515"),
])

rating = one("content_rating")
if rating.get("type") != "oars-1.1":
    fail(f'content_rating type is {rating.get("type")!r}, want "oars-1.1"')
if len(rating) != 0:
    fail("content_rating has children; Puron has none of the rated content")
EOF
echo "C-tests-19: ok"
exit 0
