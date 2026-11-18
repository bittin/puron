#!/bin/sh
# Checks Puron's app icon, the picture that software stores and COSMIC
# Settings show for it, and its install step. The full color icon must be
# plain SVG on a 128 grid with exactly its six colors, written as
# six-digit values in fill, stroke or stop-color attributes, and none of
# the elements or attributes that some renderers refuse or that could
# pull in another file (text, image, filter, script, style). The
# symbolic icon must be on a 16 grid with a single fill color and no
# stroke color, because the desktop theme recolors it and a second color
# would be lost. Then make install-appicon into a temporary root must
# install exactly the two files, unchanged and with mode 644, under
# PREFIX=/app as the Flatpak builds it.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_18.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-18: FAIL $*"; exit 1; }
python3 --version > /dev/null 2>&1 || { echo "HOST NEED python3"; exit 1; }
full=data/icons/hicolor/scalable/apps/io.github.elalonde.Puron.svg
sym=data/icons/hicolor/symbolic/apps/io.github.elalonde.Puron-symbolic.svg
test -f "$full" || fail "missing $full"
test -f "$sym" || fail "missing $sym"
python3 -I - "$full" "$sym" <<'EOF' || exit 1
import re
import sys
import xml.dom.minidom

full_path, sym_path = sys.argv[1], sys.argv[2]
COLOR_ATTRS = ("fill", "stroke", "stop-color")
FULL_COLORS = {"#2fb5a3", "#1f5f8b", "#7fd8cb", "#f7f7f2", "#f2b33d", "#e5533d"}
HEX = re.compile(r"#[0-9a-fA-F]{6}")
URL = re.compile(r"url\(#[^)]+\)")


def fail(msg):
    print(f"C-tests-18: FAIL {msg}")
    sys.exit(1)


def load(path):
    try:
        return xml.dom.minidom.parse(path)
    except Exception as e:
        fail(f"{path} does not parse as XML: {e}")


def elements(doc):
    return doc.getElementsByTagName("*")


def local(node):
    return node.localName or node.tagName


def check_root(doc, path, view_box):
    root = doc.documentElement
    if local(root) != "svg":
        fail(f"{path}: the root is {local(root)}, not svg")
    if root.getAttribute("viewBox") != view_box:
        fail(f'{path}: viewBox is "{root.getAttribute("viewBox")}", not "{view_box}"')


def check_no_elements(doc, path, names):
    for e in elements(doc):
        if local(e) in names:
            fail(f"{path}: has a {local(e)} element")


full = load(full_path)
check_root(full, full_path, "0 0 128 128")
check_no_elements(full, full_path, {"text", "image", "filter", "script", "style"})
colors = set()
for e in elements(full):
    if e.hasAttribute("style"):
        fail(f"{full_path}: a {local(e)} element has a style attribute")
    for a in COLOR_ATTRS:
        if not e.hasAttribute(a):
            continue
        v = e.getAttribute(a)
        if HEX.fullmatch(v):
            colors.add(v.lower())
        elif v != "none" and not URL.fullmatch(v):
            fail(f'{full_path}: {a}="{v}" is not a six-digit color, none or url(#...)')
if colors != FULL_COLORS:
    extra = sorted(colors - FULL_COLORS)
    missing = sorted(FULL_COLORS - colors)
    fail(f"{full_path}: colors differ: extra {extra}, missing {missing}")

sym = load(sym_path)
check_root(sym, sym_path, "0 0 16 16")
check_no_elements(sym, sym_path, {"text", "image"})
fills = {e.getAttribute("fill") for e in elements(sym) if e.hasAttribute("fill")} - {"none"}
if len(fills) != 1:
    fail(f"{sym_path}: {len(fills)} fill values other than none: {sorted(fills)}")
strokes = {e.getAttribute("stroke") for e in elements(sym) if e.hasAttribute("stroke")} - {"none"}
if strokes:
    fail(f"{sym_path}: stroke values other than none: {sorted(strokes)}")
EOF
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
root=$tmp/root
make -s install-appicon DESTDIR="$root" PREFIX=/app > "$tmp/install.txt" 2>&1 \
    || { cat "$tmp/install.txt"; fail "make install-appicon"; }
for pair in "$full scalable" "$sym symbolic"; do
    src=${pair% *}
    dst=$root/app/share/icons/hicolor/${pair#* }/apps/${src##*/}
    test -f "$dst" || fail "not installed: $dst"
    cmp -s "$src" "$dst" || fail "$dst differs from $src"
    [ "$(stat -c %a "$dst")" = 644 ] || fail "$dst has mode $(stat -c %a "$dst")"
done
n=$(find "$root" -type f | wc -l)
[ "$n" = 2 ] || fail "install-appicon wrote $n files, want 2: $(find "$root" -type f | tr '\n' ' ')"
echo "C-tests-18: ok"
exit 0
