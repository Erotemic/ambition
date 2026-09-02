#!/usr/bin/env python3
"""What each sprite target DECLARES it installs, asked of the renderer itself.

⛔⛔ **NEVER GUESS `<target>*_spritesheet.png`.** That assumption is wrong for
every target that installs under a subdirectory or under sub-names:
`town_tileset` installs `town_tileset.png`/`.yaml`, `interdimensional_gate`
installs `_ring_`/`_portal_` sheets, `gnu_ton_apple` installs into the
`gnu_ton_boss/` subdir, and `hud_icons` installs `hud_stock_icon.png` — a name
with no textual relation to the target at all. Only the registry knows.

⭐ ONE IMPLEMENTATION, TWO CONSUMERS. `check_published_sheets_are_present.py`
imports `claimed_install_names`; `scripts/regen/sprites.sh` runs this file as a
script through the renderer's interpreter to key its output-digest cache. The
guess used to live in the cache layer as `_target_sheet_glob`, which is how a
19-minute publish of 137 targets ended in a silent `set -e` abort with no
message: four of them install nothing matching the glob, the digest helper
returned 1, and that status propagated out of the whole script.

Requires the renderer package, so run it with the sprite renderer's interpreter
(`scripts/lib/tool_python.sh` resolves it). Exits 3 when the package is not
importable — an ordinary state on a machine whose tool venv is not set up, and
a distinct condition from "this target installs nothing".
"""

from __future__ import annotations

import sys


def claimed_install_names(targets: list[str]) -> dict[str, list[str]] | None:
    """Map each known target to the install-relative paths it declares.

    Returns `None` when the renderer package is not importable, so a caller can
    report "cannot check" rather than inventing a verdict. Targets the registry
    does not know are omitted rather than reported empty; an unregistered name
    and a target that installs nothing are different facts.
    """
    try:
        from ambition_sprite2d_renderer.cli.commands import discover_all_targets
    except Exception:
        return None
    result = discover_all_targets()
    # `discover_all_targets` returns a `DiscoveryReport` of (targets, warnings).
    # Pick the mapping by TYPE rather than by index: the report is a structured
    # pair whose field order is not this module's to depend on.
    registry = result if isinstance(result, dict) else None
    if registry is None:
        registry = next((el for el in result if isinstance(el, dict)), None)
    if registry is None:
        return None
    claimed: dict[str, list[str]] = {}
    for target in targets:
        entry = registry.get(target)
        if entry is None:
            continue
        try:
            claimed[target] = list(entry.claimed_install_names())
        except Exception:
            continue
    return claimed


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: sprite_install_names.py <target>...", file=sys.stderr)
        return 2
    claimed = claimed_install_names(argv)
    if claimed is None:
        print(
            "the sprite renderer is not importable here, so what each target "
            "installs is unknown.\n"
            "  diagnose with: scripts/regen/sprites.sh --check-toolchain",
            file=sys.stderr,
        )
        return 3
    for target, names in claimed.items():
        for name in names:
            print(f"{target}\t{name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
