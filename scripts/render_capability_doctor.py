#!/usr/bin/env python3
"""Can this machine render an offscreen frame, and if not, WHAT IS MISSING?

⛔⛔ THIS EXISTS BECAUSE THE QUESTION WAS ANSWERED WRONG. A 2026-08-29 review read
`moveset_render`'s failure message — which named a rollback session — concluded
the VM had no Vulkan adapter, and recommended installing `mesa-vulkan-drivers`.
Measured on the same VM: Lavapipe was installed and `moveset_render` produced
real engine PNGs. The renderer was never the problem.

⭐ SO THE ANSWER SHOULD COST A SECOND, NOT A FIVE-MINUTE APP BOOT. `moveset_render`
composes the whole game before it can fail; this reads the loader and the ICD
directory, which is what actually decides whether WGPU has a device behind it.

⛔ IT REPORTS, IT DOES NOT PROVE. An ICD on disk is necessary and not sufficient —
a driver can still refuse. The authoritative answer is an engine render
succeeding, and this says so rather than claiming more than it looked at.

    python3 scripts/render_capability_doctor.py
    python3 scripts/render_capability_doctor.py --json
"""

from __future__ import annotations

import argparse
import ctypes.util
import json
from pathlib import Path

# Where the Vulkan loader looks for installable client drivers. A loader with an
# empty directory here has no device behind it, which is exactly the state the
# review believed this VM was in.
ICD_DIRS = (
    Path("/usr/share/vulkan/icd.d"),
    Path("/usr/local/share/vulkan/icd.d"),
    Path("/etc/vulkan/icd.d"),
)

# The software rasterizer's ICD. ⭐ IT IS ENOUGH: `VisibleRenderMode::OffscreenGpu`
# creates no window and disables winit, so it needs an adapter that can render to
# a texture — not a physical GPU and not an X server.
LAVAPIPE_ICDS = ("lvp_icd.json",)

SUGGESTED_PACKAGE = "mesa-vulkan-drivers"


def loader() -> str | None:
    """The Vulkan loader's soname, or `None` when nothing provides it."""
    return ctypes.util.find_library("vulkan")


def icds() -> list[Path]:
    found: list[Path] = []
    for directory in ICD_DIRS:
        if not directory.is_dir():
            continue
        found.extend(sorted(p for p in directory.glob("*.json")))
    return found


def report() -> dict:
    lib = loader()
    manifests = icds()
    names = [p.name for p in manifests]
    software = [n for n in names if n in LAVAPIPE_ICDS]
    # ⛔ THE VERDICT IS ABOUT THE PAIR. A loader with no ICD and an ICD with no
    # loader fail the same way and want different fixes, so both are named.
    if lib is None:
        verdict, hint = "unavailable", f"no Vulkan loader; install {SUGGESTED_PACKAGE} and libvulkan1"
    elif not manifests:
        verdict, hint = (
            "unavailable",
            f"Vulkan loader present, no ICD installed — install {SUGGESTED_PACKAGE} "
            "for the Lavapipe software adapter",
        )
    else:
        verdict, hint = "likely", (
            "an ICD is installed, so WGPU should find an adapter — an engine "
            "render succeeding is the authoritative answer"
        )
    return {
        "vulkan_loader": lib,
        "icd_dir_searched": [str(d) for d in ICD_DIRS],
        "icds": names,
        "software_adapter": software,
        "offscreen_capture": verdict,
        "hint": hint,
        # ⚠ SAID OUT LOUD, because the whole point is not to over-claim: nothing
        # here started a device.
        "checked": "loader and ICD manifests on disk; no adapter was created",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="machine-readable")
    args = parser.parse_args()
    data = report()
    if args.json:
        print(json.dumps(data, indent=2))
        return 0
    print(f"vulkan loader     {data['vulkan_loader'] or 'ABSENT'}")
    print(f"vulkan ICDs       {', '.join(data['icds']) or 'NONE'}")
    print(f"software adapter  {', '.join(data['software_adapter']) or 'none (Lavapipe absent)'}")
    print(f"offscreen capture {data['offscreen_capture']}")
    print(f"                  {data['hint']}")
    print(f"checked           {data['checked']}")
    return 0 if data["offscreen_capture"] == "likely" else 1


if __name__ == "__main__":
    raise SystemExit(main())
