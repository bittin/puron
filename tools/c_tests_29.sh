#!/bin/sh
# Checks that CI builds the Flatpak, probes its sandbox and keeps the
# bundle, in the one workflow file that the release tool reads.
#
# The job flatpak of .github/workflows/ci.yml is read as text, from its
# line "  flatpak:" to the next job. It must run on Ubuntu 24.04; build
# with tools/flatpak-build.sh, letting the build install its runtimes;
# then run tools/flatpak-probe.sh on the build directory; then upload
# the bundle as the artifact puron-flatpak. A step must decide before
# the build whether the push changes the Flatpak, and the build, probe
# and upload steps must each depend on that decision, so that a push
# that changes nothing of the Flatpak does not spend a full build. Every
# action is pinned by its commit, no step may fail without failing the
# job, and nothing may install the runtimes system-wide with sudo. The
# workflow directory must hold ci.yml alone: a second workflow would
# not be part of the run that a release waits for. The job check must
# still check the host's tools before make check, so that a validator
# that is missing or too old is named before the long build starts.
# Exits 0, or 1 naming the first failed check.
# Usage: tools/c_tests_29.sh
cd "$(dirname "$0")/.." || exit 2
python3 -I - <<'EOF'
import os
import re
import sys


def fail(msg):
    print("C-tests-29: FAIL " + msg)
    sys.exit(1)


files = sorted(os.listdir(".github/workflows"))
if files != ["ci.yml"]:
    fail("the workflow directory holds %s, want ci.yml alone" % files)
with open(".github/workflows/ci.yml", encoding="utf-8") as f:
    lines = f.read().split("\n")


def block(name):
    starts = [i for i, line in enumerate(lines) if line == "  %s:" % name]
    if len(starts) != 1:
        fail("the job %s is there %d times, want once" % (name, len(starts)))
    end = len(lines)
    for i in range(starts[0] + 1, len(lines)):
        if re.match(r"  [A-Za-z0-9_-]+:", lines[i]):
            end = i
            break
    return lines[starts[0]:end]


def only(lines, text, what):
    hits = [i for i, line in enumerate(lines) if text in line]
    if len(hits) != 1:
        fail("%d lines with %s in the job %s, want one" % (len(hits), text, what))
    return hits[0]


# The step that holds line i: from its "- " line to the next one.
def step(lines, i):
    starts = [j for j, line in enumerate(lines) if re.match(r"\s*- ", line) and j <= i]
    if not starts:
        fail("line %d of the job is in no step" % i)
    start = starts[-1]
    indent = len(lines[start]) - len(lines[start].lstrip())
    end = len(lines)
    for j in range(start + 1, len(lines)):
        s = lines[j]
        if s.strip() and len(s) - len(s.lstrip()) <= indent:
            end = j
            break
    return lines[start:end]


def guarded(lines, i, what):
    if not any(re.match(r"\s*-?\s*if:.*\.outputs\.build == 'true'", line) for line in step(lines, i)):
        fail("the %s step has no if: on the build decision" % what)


fp = block("flatpak")
if not any(line.strip() == "runs-on: ubuntu-24.04" for line in fp):
    fail("the job flatpak does not run on ubuntu-24.04")
b = only(fp, "tools/flatpak-build.sh", "flatpak")
if "PURON_FLATPAK_INSTALL_DEPS=1" not in fp[b]:
    fail("the build line does not set PURON_FLATPAK_INSTALL_DEPS=1")
p = only(fp, "tools/flatpak-probe.sh --build-dir", "flatpak")
if p < b:
    fail("the probe runs before the build")
u = [i for i, line in enumerate(fp) if re.search(r"uses: actions/upload-artifact@[0-9a-f]{40}(\s|$)", line)]
if len(u) != 1 or u[0] < p:
    fail("no upload of the bundle after the probe")
if not any(line.strip() == "name: puron-flatpak" for line in step(fp, u[0])):
    fail("the upload step does not name the artifact puron-flatpak")
d = [i for i, line in enumerate(fp) if "build=" in line and "GITHUB_OUTPUT" in line]
if not d or d[0] > b:
    fail("no step writes build= to GITHUB_OUTPUT before the build")
guarded(fp, b, "build")
guarded(fp, p, "probe")
guarded(fp, u[0], "upload")
for line in fp:
    if re.search(r"\buses:", line) and not re.search(r"uses:\s*\S+@[0-9a-f]{40}(\s|$)", line):
        fail("an action not pinned by its commit: " + line.strip())
    if "continue-on-error" in line:
        fail("continue-on-error in the job flatpak")
    if "sudo flatpak" in line:
        fail("sudo flatpak in the job flatpak")

ck = block("check")
h = only(ck, "tools/check-test-host.sh", "check")
m = [i for i, line in enumerate(ck) if line.strip() == "run: make check"]
if len(m) != 1 or m[0] < h:
    fail("the job check does not run make check once, after tools/check-test-host.sh")
print("C-tests-29: ok")
EOF
