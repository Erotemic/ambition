#!/usr/bin/env python3
"""A raw `KeyCode` read must not name a key a shipped input preset binds.

⛔⛤ **THIS GUARD EXISTS BECAUSE THE CLONE HOTKEY SHIPPED AND NOBODY COULD SEE
IT.** `request_player_clone_on_key` read `KeyCode::KeyK` straight out of
`ButtonInput` in `Update`, while `wasd_jkl()` bound `K` to `burst` and
`wasd_uipo()` bound it to `utility`. A player on either shipped preset spawned a
debug clone every time they used that action. It was found by a 2026-09-18
review reading the presets by hand, and the whole feature was deleted
(`89d78a4a5`). Nothing in the tree could have found it a second time.

⇒ The invariant is one sentence: **the preset table is the only thing allowed to
decide what a bound key means.** A system outside `ambition_input` that reaches
past it and reads the raw key has quietly given that key a second meaning, and
the player gets both.

# # What this checks

1. Every raw `just_pressed` / `pressed` / `just_released` of a `KeyCode` in
   production code outside `ambition_input`, whose key a preset BINDS, is
   named in `ADJUDICATED` with the reason it is safe.
2. The two populations have not collapsed (`FLOORS`) — a parse that finds no
   bindings would call every read safe.

⛔⛤ **A DISPLAY TABLE READS EXACTLY LIKE A BINDING TABLE, and the first version
of this script counted 39 bound keys when the answer is 35.** `presets.rs` ends
with a `KeyCode::KeyM => "M"` match that turns a keycode into a label for the
rebinding UI; grepping `KeyCode::(\\w+)` sweeps all 36 of those arms in beside
the real bindings. MEASURED: that inflation alone produced a false finding —
`KeyCode::KeyM` in the map menu, which no preset binds at all. ⇒ A binding is a
STRUCT FIELD ASSIGNMENT, `field: KeyCode::X`, and that is what is matched.

    python3 scripts/check_raw_key_reads_do_not_collide_with_presets.py
"""

from __future__ import annotations

import functools
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_rollback_mutators_run_in_sim as sim  # noqa: E402
import measure_user_settings_in_simulation as settings  # noqa: E402

REPO = Path(__file__).resolve().parent.parent

#: The preset table: the one authority for what a bound key means.
PRESETS = "crates/ambition_input/src/presets.rs"

#: The crate that owns key meaning. A raw read inside it is the mechanism, not
#: a bypass of it.
OWNER = "crates/ambition_input/"

#: `field: KeyCode::X` — a binding. NOT `KeyCode::X => "X"`, which is a label.
_BINDING = re.compile(r"[a-z_]+\s*:\s*KeyCode::([A-Za-z0-9]+)")
#: A raw read of a physical key.
_RAW_READ = re.compile(
    r"\b(?:just_pressed|pressed|just_released)\s*\(\s*"
    r"(?:[A-Za-z_][A-Za-z0-9_]*::)*KeyCode::([A-Za-z0-9]+)\s*\)"
)

#: ⛔ Anti-vacuity. If either parse collapses, every read reads as safe.
FLOORS = {"bound keys": 20, "raw reads": 5}

#: `(key, file)` → why reading this bound key raw is correct here.
#:
#: A row is a READING, never a waiver: it says what keeps the two meanings
#: apart, and a reader can check it.
ADJUDICATED: dict[tuple[str, str], str] = {
    ("ShiftLeft", "crates/ambition_platformer2d_shared_tangle/src/developer_hotkeys.rs"): (
        "⭐ A MODIFIER, NOT A SECOND MEANING. Read as "
        "`keys.pressed(ShiftLeft) || keys.pressed(ShiftRight)` to QUALIFY another "
        "hotkey (`developer_hotkeys.rs:82`), never as an action of its own. "
        "`wasd_uipo` binds `ShiftLeft` to `modifier` and `ShiftRight` to `walk`, so "
        "a developer holding shift also walks — which is what holding a walk key "
        "does, and the two meanings do not contend for one press (read 2026-09-18)"
    ),
    ("ShiftRight", "crates/ambition_platformer2d_shared_tangle/src/developer_hotkeys.rs"): (
        "⭐ The same modifier read, other hand. See the `ShiftLeft` row "
        "(read 2026-09-18)"
    ),
    ("KeyR", "crates/ambition_load_presentation/src/basic_presentation.rs"): (
        "✅ GATED OUT OF GAMEPLAY BY STATE, not by convention. The read is guarded "
        "by `active.phase == LoadForegroundPhase::Failed` plus a retryable failure "
        "(`basic_presentation.rs:71-73`), so it can only fire on a failed LOAD "
        "screen — where `arrows_qwer`'s `secondary: KeyCode::KeyR` has nothing to "
        "act on because no session is running. ⚠ The gate is the argument; moving "
        "this read out of the `Failed` arm would make it a real collision "
        "(read 2026-09-18)"
    ),
    ("KeyN", "crates/ambition_menu/src/map/input.rs"): (
        "⛔ LIVE, AND IT IS THE CLONE'S DEFECT EXACTLY — filed, not excused. "
        "`handle_map_menu_hotkeys` reads raw `KeyCode::KeyN` to toggle the minimap "
        "(`map/input.rs:40`) and runs whenever a session world exists, `.after` "
        "`CoreSimulation` (`map/mod.rs:226-239`). BOTH shipped presets bind `N` to "
        "`taunt` — `wasd_jkl` and `wasd_uipo`, the same two presets whose `K` "
        "binding killed the player clone. ⇒ On either preset a taunt also toggles "
        "the minimap. Milder than the clone (no rollback state, no simulation "
        "write) and the same mechanism, so it is recorded here with its reading "
        "rather than waived: the repair is to route the toggle through a bound "
        "action like `M`'s sibling intent, not to pick a different raw key "
        "(read 2026-09-18)"
    ),
}


@functools.cache
def _preset_text(repo: Path) -> str:
    return settings.without_comments((repo / PRESETS).read_text(errors="replace"))


def bound_keys(repo: Path = REPO) -> set[str]:
    """Physical keys a shipped preset assigns to an action."""
    return set(_BINDING.findall(_preset_text(repo)))


def label_only_keys(repo: Path = REPO) -> set[str]:
    """Keys that appear ONLY in the keycode→label match, and bind nothing.

    Kept as a function rather than a comment because it is the specimen that
    proves the binding filter is doing something: if this set ever empties, the
    filter has stopped distinguishing the two tables.
    """
    text = _preset_text(repo)
    labels = set(re.findall(r"KeyCode::([A-Za-z0-9]+)\s*=>", text))
    return labels - bound_keys(repo)


def raw_reads(repo: Path = REPO) -> dict[tuple[str, str], int]:
    """`(key, file)` → how many times production reads that key raw."""
    found: dict[tuple[str, str], int] = {}
    for path, text in sim._production_sources(repo):
        rel = path.relative_to(repo).as_posix()
        if rel.startswith(OWNER):
            continue
        for match in _RAW_READ.finditer(settings.without_comments(text)):
            found[(match.group(1), rel)] = found.get((match.group(1), rel), 0) + 1
    return found


def collisions(repo: Path = REPO) -> dict[tuple[str, str], int]:
    """Raw reads of a key a preset binds."""
    bound = bound_keys(repo)
    return {k: n for k, n in raw_reads(repo).items() if k[0] in bound}


def main() -> int:
    bound = bound_keys()
    reads = raw_reads()
    sizes = {"bound keys": len(bound), "raw reads": len(reads)}
    short = [f"{what}: {n} (floor {FLOORS[what]})" for what, n in sizes.items() if n < FLOORS[what]]
    if short:
        print(
            "the scan lost reach — every verdict below would be about the parser:\n  "
            + "\n  ".join(short),
            file=sys.stderr,
        )
        return 1
    if not label_only_keys():
        print(
            "no key is label-only any more, so the binding filter cannot be shown to "
            "distinguish `field: KeyCode::X` from the `KeyCode::X => \"X\"` label "
            "table — which is the miscount this script was written around.",
            file=sys.stderr,
        )
        return 1

    found = collisions()
    unadjudicated = sorted(k for k in found if k not in ADJUDICATED)
    if unadjudicated:
        print(
            f"{len(unadjudicated)} raw `KeyCode` read(s) name a key a shipped preset "
            "binds, with no reading saying why that is safe:\n\n  "
            + "\n  ".join(f"KeyCode::{key}  {file}" for key, file in unadjudicated)
            + "\n\nA bound key has a meaning the preset table owns. Reading it raw "
            "gives it a second one, and the player gets both — which is how the "
            "player-clone hotkey shipped on `K` while two presets bound `K` to a "
            "gameplay action. Route the intent through a binding, or add the row "
            "with the argument that keeps the two apart (a modifier, or a state "
            "gate that excludes gameplay).",
            file=sys.stderr,
        )
        return 1

    stale = sorted(k for k in ADJUDICATED if k not in found)
    if stale:
        print(
            "ADJUDICATED names reads this scan no longer sees:\n\n  "
            + "\n  ".join(f"KeyCode::{key}  {file}" for key, file in stale)
            + "\n\nEither the read was routed through a binding — delete the row in "
            "the same commit and say so — or the scan stopped seeing it, which is "
            "the more likely reading and the more dangerous one.",
            file=sys.stderr,
        )
        return 1

    live = sum(1 for reading in ADJUDICATED.values() if reading.startswith("⛔"))
    print(
        f"ok: {len(reads)} raw `KeyCode` read(s) outside `{OWNER}`, "
        f"{len(found)} of them naming one of the {len(bound)} preset-bound keys, "
        f"every one read ({len(label_only_keys())} further key(s) appear only in the "
        "label table and bind nothing)"
    )
    if live:
        print(f"⚠ {live} of those readings is a LIVE collision, filed rather than excused.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
