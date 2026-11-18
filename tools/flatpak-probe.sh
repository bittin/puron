#!/bin/sh
# Checks that Puron's Flatpak sees the same machine as the host: the
# sensor chips, the drives, the lm-sensors configuration and the rest of
# what the programs read. The Flatpak runs in a sandbox, so a missing
# bind or permission would show up only as rows that silently do not
# appear; this script reads the same facts in both places and compares
# them, without adding any option to the programs themselves.
#
# Usage:
#   tools/flatpak-probe.sh --collect host|sandbox
#     Prints the facts of the place it runs in, one "<key><TAB><value>"
#     line each, sorted by key, "-" for a value that cannot be read. The
#     caller says which place it is. In the sandbox the host's /etc is
#     at /run/host/etc. Some keys exist only for one of the two places.
#   tools/flatpak-probe.sh --compare build-dir|installed <host file> <sandbox file>
#     Prints one verdict line per rule, "PROBE <key> ok",
#     "PROBE <key> FAIL <host value> <sandbox value>" or
#     "PROBE <key> skip <reason>", and exits 1 when a line says FAIL. A
#     key that a rule needs and a file lacks fails, shown as "(none)";
#     a key that only the sandbox has shows "-" as its host value.
#   tools/flatpak-probe.sh --build-dir <dir>
#     Collects on the host, then in the sandbox of the build directory
#     <dir> (flatpak-builder --run, which grants the permissions of the
#     finished app), compares them in the mode build-dir, and checks the
#     permissions in <dir>/metadata against the manifest's finish-args.
#   tools/flatpak-probe.sh --installed
#     The same for the app installed with flatpak, started with
#     flatpak run, in the mode installed.
# In the last two forms the script itself is fed to sh in the sandbox on
# standard input, since the sandbox cannot see this tree.
#
# In the sandbox the script also starts the bundled libsensors once, the
# way the programs start it, and reports what that start returned, how
# many chips it found and how many bytes it wrote to standard error. A
# library that cannot start shows no sensor rows at all, and one that
# warns at each start fills the journal, since the applet starts it at
# every poll; neither shows in the files alone.
#
# The journal socket and the host's icon themes are judged only for an
# installed app: the sandbox of a build directory need not bind them the
# way flatpak run does. runtime-libsensors is printed for the record and
# never judged; it tells whether the runtime already ships the library
# that the Flatpak builds itself.
# Exits 0, 1 when a check fails or the sandbox cannot start, 2 for a
# usage error.

app=io.github.elalonde.Puron

usage() {
    echo "usage: tools/flatpak-probe.sh --collect host|sandbox | --compare build-dir|installed <host file> <sandbox file> | --build-dir <dir> | --installed" >&2
    exit 2
}

# Prints its standard input as one value: lines trimmed of trailing
# blanks, sorted and joined with ",", or "-" when there is nothing.
joined() {
    v=$(sed 's/[[:space:]]*$//' | LC_ALL=C sort | paste -sd, -)
    [ -n "$v" ] && printf '%s\n' "$v" || echo -
}

yes_no() {
    if "$@" > /dev/null 2>&1; then echo yes; else echo no; fi
}

# Prints "yes" when one of the data folders of the sandbox has the
# COSMIC theme's scalable icon $1. The host's themes reach the sandbox
# only through such a folder (/run/host/share for an installed app).
cosmic_icon() {
    printf '%s\n' "${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" | tr : '\n' | while IFS= read -r d; do
        for f in "$d"/icons/Cosmic/scalable/*/"$1".svg; do
            if [ -e "$f" ]; then echo yes; exit 0; fi
        done
    done
}

# Starts the bundled libsensors once and prints three lines: the value
# that sensors_init returned, the count of chips it found (0 when the
# start failed), and the count of bytes it wrote to standard error; "-"
# for the first two when the start printed nothing, as without python3
# or without the library. No variable is set here: the start must see
# the sandbox's own environment, as the programs do.
libsensors_start() {
    err=$(mktemp 2> /dev/null) || {
        printf 'libsensors-chips\t-\nlibsensors-init\t-\nlibsensors-stderr\t-\n'
        return
    }
    # Standard input is the rest of this script when it runs in the
    # sandbox, so the child must not be able to read it.
    out=$(python3 -I -c '
import ctypes
try:
    lib = ctypes.CDLL("/app/lib/libsensors.so.5")
except OSError:
    raise SystemExit(1)
lib.sensors_init.argtypes = [ctypes.c_void_p]
lib.sensors_init.restype = ctypes.c_int
lib.sensors_get_detected_chips.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_int)]
lib.sensors_get_detected_chips.restype = ctypes.c_void_p
init = lib.sensors_init(None)
chips = 0
if init == 0:
    nr = ctypes.c_int(0)
    while lib.sensors_get_detected_chips(None, ctypes.byref(nr)) is not None:
        chips += 1
    lib.sensors_cleanup()
print(init, chips)
' < /dev/null 2> "$err")
    init=- found=-
    case $out in
        *' '*) init=${out% *} found=${out#* } ;;
    esac
    printf 'libsensors-chips\t%s\n' "$found"
    printf 'libsensors-init\t%s\n' "$init"
    printf 'libsensors-stderr\t%s\n' "$(wc -c < "$err" | tr -d ' ')"
    rm -f "$err"
}

collect() {
    case $1 in
        host) etc=/etc ;;
        sandbox) etc=/run/host/etc ;;
        *) usage ;;
    esac
    {
        printf 'hwmon\t%s\n' "$(cat /sys/class/hwmon/*/name 2> /dev/null | joined)"
        printf 'nvme\t%s\n' "$(cat /sys/class/nvme/*/model 2> /dev/null | joined)"
        printf 'dmi\t%s\n' "$(cat /sys/class/dmi/id/product_name 2> /dev/null | joined)"
        if [ -r /proc/cpuinfo ]; then
            printf 'cpus\t%s\n' "$(grep -c '^processor' /proc/cpuinfo)"
        else
            printf 'cpus\t-\n'
        fi
        if [ -r "$etc/sensors3.conf" ]; then
            printf 'sensors-conf\t%s\n' "$(sha256sum < "$etc/sensors3.conf" | cut -d' ' -f1)"
        else
            printf 'sensors-conf\t-\n'
        fi
        printf 'sensors-d\t%s\n' "$(ls -A "$etc/sensors.d" 2> /dev/null | joined)"
        printf 'journal\t%s\n' "$(yes_no test -S /run/systemd/journal/socket)"
        if [ "$1" = host ]; then
            printf 'cosmic-theme\t%s\n' "$(yes_no test -d /usr/share/icons/Cosmic)"
        else
            libsensors_start
            printf 'xdg-open\t%s\n' "$(yes_no command -v xdg-open)"
            printf 'liquidctl\t%s\n' "$(yes_no command -v liquidctl)"
            n=0
            for f in /app/bin/puron-applet /app/bin/puron-settings \
                /app/share/applications/$app.desktop \
                /app/share/metainfo/$app.metainfo.xml \
                /app/share/icons/hicolor/scalable/apps/$app.svg \
                /app/share/icons/hicolor/symbolic/apps/$app-symbolic.svg \
                /app/share/licenses/$app/LICENSE \
                /app/share/licenses/$app/GPL-3.0.txt \
                /app/share/licenses/$app/SOURCE-OFFER \
                /app/share/licenses/$app/THIRD-PARTY-NOTICES.txt \
                /app/share/licenses/$app/lm-sensors-COPYING.LGPL; do
                [ -e "$f" ] && n=$((n + 1))
            done
            printf 'files\t%s\n' "$n"
            n=0
            for f in /app/include /app/man; do
                [ -e "$f" ] && n=$((n + 1))
            done
            printf 'leftovers\t%s\n' "$n"
            n=0
            for name in drive-harddisk-system-symbolic network-wireless-symbolic \
                network-wired-symbolic object-select-symbolic; do
                [ -n "$(cosmic_icon "$name")" ] && n=$((n + 1))
            done
            printf 'cosmic-icons\t%s\n' "$n"
            printf 'runtime-libsensors\t%s\n' "$(yes_no test -e /usr/lib/x86_64-linux-gnu/libsensors.so.5)"
        fi
    } | LC_ALL=C sort
}

# Prints the value of key $1 in the collected file $2; returns 1 when
# the file has no such key.
value() {
    awk -F '\t' -v k="$1" '$1 == k { sub(/^[^\t]*\t/, ""); print; found = 1; exit } END { exit !found }' "$2"
}

compare() {
    mode=$1 host=$2 sandbox=$3
    case $mode in build-dir | installed) ;; *) usage ;; esac
    [ -r "$host" ] && [ -r "$sandbox" ] || usage
    failed=0

    # The facts that the programs read through the sandbox must be the
    # host's own: the same chips, drives, machine and configuration.
    for k in hwmon nvme dmi cpus sensors-conf sensors-d; do
        same "$k"
    done

    # The bundled libsensors must start, silently, and find the chips.
    # Not every hwmon device becomes a chip (libsensors skips devices it
    # cannot place on a bus or that have no readable feature), and the
    # host need not have a libsensors of its own to count them with, so
    # the chip count is checked as a range: none when the host has no
    # hwmon device, else at least one and at most one per device.
    want libsensors-init 0
    want libsensors-stderr 0
    chips

    # What the Flatpak must hold, and the one program it must not.
    want xdg-open yes
    want liquidctl no
    want files 11
    want leftovers 0

    if [ "$mode" = installed ]; then
        same journal
    else
        echo "PROBE journal skip build-dir"
    fi

    if [ "$mode" = build-dir ]; then
        echo "PROBE cosmic-icons skip build-dir"
    elif ! theme=$(value cosmic-theme "$host"); then
        fail cosmic-icons "(none)" "$(value cosmic-icons "$sandbox" || echo "(none)")"
    elif [ "$theme" = yes ]; then
        want cosmic-icons 4
    else
        echo "PROBE cosmic-icons skip no COSMIC theme on the host"
    fi
    return "$failed"
}

fail() {
    echo "PROBE $1 FAIL $2 $3"
    failed=1
}

same() {
    h=$(value "$1" "$host") || h="(none)"
    s=$(value "$1" "$sandbox") || s="(none)"
    if [ "$h" = "$s" ] && [ "$h" != "(none)" ]; then
        echo "PROBE $1 ok"
    else
        fail "$1" "$h" "$s"
    fi
}

want() {
    s=$(value "$1" "$sandbox") || s="(none)"
    if [ "$s" = "$2" ]; then echo "PROBE $1 ok"; else fail "$1" - "$s"; fi
}

# Checks the sandbox's chip count against the host's hwmon devices. The
# failure line gives the host's device count as the host value.
chips() {
    if ! h=$(value hwmon "$host"); then
        fail libsensors-chips "(none)" "$(value libsensors-chips "$sandbox" || echo "(none)")"
        return
    fi
    if [ "$h" = - ]; then
        n=0
    else
        n=$(printf '%s\n' "$h" | tr , '\n' | wc -l | tr -d ' ')
    fi
    s=$(value libsensors-chips "$sandbox") || s="(none)"
    case $s in
        '' | *[!0-9]*) fail libsensors-chips "$n" "$s"; return ;;
    esac
    if { [ "$n" = 0 ] && [ "$s" = 0 ]; } || { [ "$n" -ge 1 ] && [ "$s" -ge 1 ] && [ "$s" -le "$n" ]; }; then
        echo "PROBE libsensors-chips ok"
    else
        fail libsensors-chips "$n" "$s"
    fi
}

# Compares the permissions in the flatpak keyfile $1, the app's own
# metadata, with the manifest's finish-args. Both are turned into
# finish-args form, sorted, and joined with ",". ":rw" is dropped
# because flatpak writes a read-write filesystem without it. Only the
# app's metadata is read, never the merged view that adds the user's
# and the system's overrides: an override is the machine owner's choice,
# not something the Flatpak asked for, and a global one would otherwise
# fail this check on that machine.
permissions() {
    python3 -I -c '
import configparser, json, sys

def norm(a):
    return a[:-3] if a.endswith(":rw") else a

want = sorted(norm(a) for a in json.load(open(sys.argv[1], encoding="utf-8")).get("finish-args", []))
kf = configparser.ConfigParser(interpolation=None, strict=False)
kf.optionxform = str
kf.read(sys.argv[2], encoding="utf-8")
flags = {"shared": "share", "sockets": "socket", "devices": "device",
         "features": "allow", "filesystems": "filesystem", "persistent": "persist"}
got = []
if kf.has_section("Context"):
    for key, val in kf.items("Context"):
        for v in filter(None, val.split(";")):
            got.append("--%s=%s" % (flags.get(key, key), norm(v)))
for section, prefix in (("Session Bus Policy", ""), ("System Bus Policy", "system-")):
    if kf.has_section(section):
        for name, policy in kf.items(section):
            got.append("--%s%s-name=%s" % (prefix, policy, name))
if kf.has_section("Environment"):
    for key, val in kf.items("Environment"):
        got.append("--env=%s=%s" % (key, val))
got.sort()
if got == want:
    print("PROBE permissions ok")
else:
    print("PROBE permissions FAIL %s %s" % (",".join(want), ",".join(got)))
    sys.exit(1)
' "$root/flatpak/$app.json" "$1"
}

# Collects on the host and in the sandbox, prints both, and gives the
# verdict of the mode and of the permissions.
run() {
    mode=$1
    root=$(pwd)
    tmp=$(mktemp -d) || exit 2
    trap 'rm -rf "$tmp"' EXIT
    sh tools/flatpak-probe.sh --collect host > "$tmp/host" || exit 1
    if [ "$mode" = build-dir ]; then
        dir=$2
        # flatpak-builder keeps a state folder in the current folder, so
        # it runs from the temporary one to leave the tree clean.
        (cd "$tmp" && flatpak-builder --run "$dir" "$root/flatpak/$app.json" \
            sh -s -- --collect sandbox) < tools/flatpak-probe.sh > "$tmp/sandbox" 2> "$tmp/err"
        rc=$?
        cp "$dir/metadata" "$tmp/permissions" 2> /dev/null || : > "$tmp/permissions"
    else
        flatpak run --command=sh "$app" -s -- --collect sandbox \
            < tools/flatpak-probe.sh > "$tmp/sandbox" 2> "$tmp/err"
        rc=$?
        flatpak info --show-metadata "$app" > "$tmp/permissions" 2>> "$tmp/err"
    fi
    if [ "$rc" != 0 ]; then
        echo "flatpak-probe: the sandbox did not start (exit $rc):"
        cat "$tmp/err"
        exit 1
    fi
    echo "== host"
    cat "$tmp/host"
    echo "== sandbox"
    cat "$tmp/sandbox"
    echo "== verdict"
    st=0
    compare "$mode" "$tmp/host" "$tmp/sandbox" || st=1
    permissions "$tmp/permissions" || st=1
    exit "$st"
}

case $1 in
    --collect)
        [ $# = 2 ] || usage
        collect "$2"
        ;;
    --compare)
        [ $# = 4 ] || usage
        compare "$2" "$3" "$4"
        ;;
    --build-dir)
        [ $# = 2 ] || usage
        # A relative <dir> names a folder of the caller's, so it is made
        # absolute before the script moves to the root of the tree.
        dir=$(cd "$2" 2> /dev/null && pwd) || { echo "flatpak-probe: no build directory $2"; exit 1; }
        cd "$(dirname "$0")/.." || exit 2
        run build-dir "$dir"
        ;;
    --installed)
        [ $# = 1 ] || usage
        cd "$(dirname "$0")/.." || exit 2
        run installed
        ;;
    *) usage ;;
esac
