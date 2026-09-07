#!/usr/bin/env python3
"""Assert the host actually has the system packages setup DECLARES it needs.

`scripts/setup/system_packages.sh` lists ~35 apt packages as required. Nothing
checked that the list HELD on a given box, and a partially-provisioned host is
not a theoretical worry:

    2026-09-07, this machine -- `libasound2-dev` and `libudev-dev` present,
    `libfontconfig1-dev` and `libvulkan1` ABSENT, `desktop_check.sh` green.

⛔ WHY THE DEFAULT BUILD CANNOT CATCH IT. `desktop_check.sh` compiles
`ambition_app` at default features, which does not need fontconfig. The lane that
does is the FEATURE UNION (`cargo check --workspace --all-features`), because
`bevy_text -> parley -> fontique -> yeslogic-fontconfig-sys` links it, arriving
via `bevy_dev_tools -> bevy_internal -> bevy` -- and the union job lives inside
`--run-everything`, so nothing routine touches it. ⇒ The
absence surfaced only after a ~20-minute build, as a pkg-config panic inside a
third-party build script. This turns that into a named list in under a second.

⚠ THAT CHAIN WAS WRONG HERE UNTIL 2026-09-07 and said
`bevy_rich_text3d -> cosmic-text -> yeslogic-fontconfig-sys`, read off lockfile
proximity rather than asked of cargo. `cosmic-text` is in the tree (through
`bevy_lunex`) and reaches fontconfig ZERO times. The error was not pedantic: the
wrong chain implied the union could be approximated by excluding the 3D-cube UI
crate, and MEASURED, a near-union with that crate excluded fails identically. I
corrected it in `queue.md` the same day and left it standing HERE for six hours --
a retraction that fixes one of two copies is why this file's own rule below exists.

⭐ ONE AUTHORITY: the package list is PARSED from `system_packages.sh`, never
copied here. A second copy would be the defect this repo keeps removing, and it
would rot the first time somebody adds a package.

⛔ A REPORT BY DEFAULT, NOT A GATE, AND THE REASON IS MEASURED. On this host NINE
of 33 are absent -- fontconfig plus the whole X11/xcb/xkb set -- and the box builds
and tests fine, because it is headless and never opens a window. So "missing" does
not mean "broken"; the declared list targets a desktop dev host and one list serves
several kinds of machine. A check that is red on every run of a legitimately
headless box stops being read, which is how the repo lost the value of other
absolute-count guards. `--strict` exits 1 for a host that has opted into being
fully provisioned.

⚠ IT CHECKS PRESENCE, NOT SUFFICIENCY. `dpkg-query` answers "is this package
installed", which is the same identifier the setup script installs -- no apt-name
to pkg-config-name mapping, so no guessing. It cannot tell you a package is
installed but broken, and it says nothing on a non-dpkg host, where it exits 0
with a stated reason rather than pretending to have checked.
"""

from __future__ import annotations

import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SETUP = ROOT / "scripts" / "setup" / "system_packages.sh"


def declared_packages() -> list[str]:
    """The `required_pkgs` array, read from the setup script itself."""
    text = SETUP.read_text()
    m = re.search(r"local -a required_pkgs=\(\n(.*?)\n\s*\)", text, re.S)
    if not m:
        raise SystemExit(
            f"could not find `required_pkgs=(...)` in {SETUP} -- the array was "
            "renamed or reshaped, and this check is now blind. Fix the pattern; "
            "do not copy the list here."
        )
    pkgs = []
    for line in m.group(1).splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            pkgs.append(line)
    return pkgs


def main(argv: list[str]) -> int:
    strict = "--strict" in argv
    if shutil.which("dpkg-query") is None:
        print("check_declared_system_packages: no dpkg-query on this host, so the "
              "declared apt list cannot be verified here. NOT a pass -- unchecked.")
        return 0

    pkgs = declared_packages()
    # An empty parse would report a clean host while checking nothing.
    if len(pkgs) < 10:
        raise SystemExit(
            f"parsed only {len(pkgs)} declared packages from {SETUP}, which is far "
            "below the ~35 it carries -- the parse broke rather than the host being tidy."
        )

    missing = []
    for pkg in pkgs:
        proc = subprocess.run(
            ["dpkg-query", "-W", "-f=${Status}", pkg],
            capture_output=True, text=True, check=False,
        )
        if "install ok installed" not in proc.stdout:
            missing.append(pkg)

    print(f"{len(pkgs)} packages declared by {SETUP.relative_to(ROOT)}")
    if not missing:
        print("all present")
        return 0

    print(f"\nMISSING ({len(missing)}):")
    for pkg in missing:
        print(f"  {pkg}")
    # ⛔ THE REASSURANCE COVERED PACKAGES IT WAS NEVER ABOUT. "A headless box
    # legitimately lacks the X11/xcb/xkb set" is true and was printed under a list
    # of NINE, one of which is `libfontconfig1-dev` -- not an X11 package, and
    # measured on 2026-09-07 to block `cargo check -p <crate> --all-features`
    # outright. One absence was answering two questions. The split is DERIVED from
    # the package name, not from a list kept here.
    windowing = sorted(m for m in missing if m.startswith(("libx", "libwayland")))
    other = sorted(m for m in missing if m not in set(windowing))
    if other:
        print(
            f"\n⛔ NOT covered by the headless exemption ({len(other)}): "
            f"{', '.join(other)}"
        )
        print(
            "   These are not windowing libraries, so 'this box never opens a "
            "window'\n   does not explain them. `libfontconfig1-dev` blocks "
            "`--all-features` on\n   ANY box: bevy's own text stack links it "
            "through parley -> fontique."
        )
    print(
        "\nThis host is partially provisioned. A default `cargo check` can still be "
        "green: the lanes that need these are the feature union and the render/audio "
        "paths. Fix with `scripts/setup/system_packages.sh` (needs sudo), or install "
        f"the names above.\nA headless box legitimately lacks the "
        f"{len(windowing)} windowing package(s) above; pass --strict to make "
        f"this an error."
    )
    return 1 if strict else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
