#!/bin/sh
# Checks the Flatpak manifest, flatpak/io.github.elalonde.Puron.json: its
# identity, runtime and SDK, its permissions, its build environment and
# its two modules. The permissions are the promise that the Flatpak makes
# to its users: exactly seven, so that Puron gets no network access, can
# start no program outside the sandbox and can write only its settings.
# The manifest may not hold build-args, because the COSMIC Flatpak
# repository builds with flatpak-builder --sandbox, which refuses them.
#
# libclang, which the libsensors binding needs at build time, comes either
# from the SDK itself or from one LLVM SDK extension; the check accepts
# those two forms and no mix of them. libsensors must read the host's
# lm-sensors configuration, which Flatpak shows at /run/host/etc, and
# cargo must build offline from the vendored crates. The COSMIC base app
# may join only together with the notice on its icon themes.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_23.sh
cd "$(dirname "$0")/.." || exit 2
fail() { echo "C-tests-23: FAIL $*"; exit 1; }
jq --version > /dev/null 2>&1 || { echo "HOST NEED jq"; exit 1; }
m=flatpak/io.github.elalonde.Puron.json
test -f "$m" || fail "no $m"
# check <jq filter> <what failed>: the filter must give true.
check() { jq -e "$1" "$m" > /dev/null 2>&1 || fail "$2"; }
check '.id == "io.github.elalonde.Puron"' "id"
check '.runtime == "org.freedesktop.Platform"' "runtime"
check '."runtime-version" == "25.08"' "runtime-version"
check '.sdk == "org.freedesktop.Sdk"' "sdk"
check '.command == "puron-applet"' "command"
check '(."finish-args" | sort) == ([
    "--socket=wayland",
    "--share=ipc",
    "--device=dri",
    "--filesystem=xdg-config/cosmic:rw",
    "--filesystem=host-etc:ro",
    "--system-talk-name=org.freedesktop.UDisks2",
    "--talk-name=com.system76.CosmicSettingsDaemon"
  ] | sort)' "finish-args are not the seven permissions"
check '[.. | objects | has("build-args")] | any | not' "a build-args key"
check '."sdk-extensions"[0] == "org.freedesktop.Sdk.Extension.rust-stable"' "the first SDK extension"
# The SDK's own libclang: rust-stable alone. An LLVM extension: the same
# version number in the extension, the path, the library path and
# LIBCLANG_PATH.
check '."build-options" as $b | ."sdk-extensions" as $x |
    if ($x | length) == 1 then
        $b."append-path" == "/usr/lib/sdk/rust-stable/bin"
        and ($b | has("prepend-ld-library-path") | not)
        and ($b.env | has("LIBCLANG_PATH") | not)
    elif ($x | length) == 2 and ($x[1] | test("^org\\.freedesktop\\.Sdk\\.Extension\\.llvm[0-9]+$")) then
        ($x[1] | ltrimstr("org.freedesktop.Sdk.Extension.llvm")) as $n
        | $b."append-path" == "/usr/lib/sdk/rust-stable/bin:/usr/lib/sdk/llvm\($n)/bin"
        and $b."prepend-ld-library-path" == "/usr/lib/sdk/llvm\($n)/lib"
        and $b.env.LIBCLANG_PATH == "/usr/lib/sdk/llvm\($n)/lib"
    else false end' "the SDK extensions and their paths are neither of the two forms"
check '."build-options".env as $e |
    $e.CARGO_HOME == "/run/build/puron/cargo"
    and $e.CARGO_NET_OFFLINE == "true"
    and $e.LMSENSORS_PATH == "/app"
    and ($e | keys - ["CARGO_HOME", "CARGO_NET_OFFLINE", "LMSENSORS_PATH", "LIBCLANG_PATH"]) == []' \
    "the build environment"
check '.cleanup == ["/include", "/man"]' "cleanup"
check '[.modules[].name] == ["libsensors", "puron"]' "the modules are not libsensors, then puron"
check '.modules[0]."build-commands"[0] == "make PREFIX=/app ETCDIR=/run/host/etc BUILD_STATIC_LIB=0 install-lib"' \
    "the libsensors build command"
check '.modules[0].sources as $s |
    ($s | length) == 1
    and $s[0].type == "git"
    and $s[0].url == "https://github.com/lm-sensors/lm-sensors.git"
    and $s[0].tag == "V3-6-2"
    and ($s[0].commit | test("^[0-9a-f]{40}$"))' "the libsensors source"
if [ -f flatpak/icon-themes-notice.txt ]; then
    check '.base == "com.system76.Cosmic.BaseApp" and ."base-version" == "stable"' \
        "flatpak/icon-themes-notice.txt exists, but the base app is not the COSMIC one"
else
    check '(has("base") or has("base-version")) | not' \
        "a base app without flatpak/icon-themes-notice.txt"
fi
echo "C-tests-23: ok"
exit 0
