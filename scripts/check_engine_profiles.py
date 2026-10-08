#!/usr/bin/env python3
"""A9: the supported engine profiles are named contracts, and their static half holds.

⭐ A profile is a promise about what is NOT there. The runtime half of the proof is
`ambition_platformer2d_host/tests/supported_profiles.rs` (each profile builds, steps a
real body, installs none of what it omits, and its session-edge parameters validate;
a control arm proves the probe can say yes). This checks the half a Rust test cannot:

1. **the registry and its markers agree** -- every `EngineProfile` in
   `profile.rs` has a `// profile-contract: <name>` marker and the reverse, so a profile
   cannot be added without a name the witnesses and the docs can point at.
2. **the witnesses name every profile** -- `supported_profiles.rs` iterates
   `SUPPORTED_PROFILES`, and this requires that it does (a witness file that stopped
   iterating the registry would pass for any list).
3. **the dependency closure says what the contract says** -- the headless host
   closure (`cargo tree -p ambition_platformer2d_host --no-default-features -e normal`)
   contains none of `HEADLESS_ABSENT`, the windowed one (default features) contains
   `WINDOWED_PRESENT`, and the crates behind the removable capabilities ARE linked in
   both (`LINKED_NOT_ABSENT`).

⚠ **WHAT THE THIRD CLAIM IS NOT.** The engine crates behind `Capability` are
unconditional dependencies (`Q106`), so a profile that omits one promises it is not
INSTALLED, not that it is not LINKED. `LINKED_NOT_ABSENT` makes that honest in the
other direction: the day one of those edges becomes optional this goes red, and the
contract can be upgraded to a closure claim instead of silently staying weaker.
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
PROFILE_RS = "crates/ambition_platformer2d_runtime/src/profile.rs"
WITNESS_RS = "crates/ambition_platformer2d_host/tests/supported_profiles.rs"
HOST = "ambition_platformer2d_host"

HEADLESS_ABSENT = ("ambition_render", "ambition_menu")
WINDOWED_PRESENT = ("ambition_render",)
LINKED_NOT_ABSENT = (
    "ambition_boss_encounter",
    "ambition_held_items",
    "ambition_conversation",
    "ambition_dialog",
    "ambition_abilities",
    "ambition_world_items",
    "ambition_cutscene",
)
MIN_PROFILES = 5


def markers(text: str) -> list[str]:
    return re.findall(r"^// profile-contract:\s*([a-z0-9-]+)\s*$", text, re.M)


def declared(text: str) -> list[str]:
    return re.findall(r'^\s*name:\s*"([a-z0-9-]+)",', text, re.M)


def crates_in(tree: str) -> set[str]:
    return set(re.findall(r"([A-Za-z0-9_-]+) v\d", tree))


def check_registry(profile_rs: str, witness_rs: str) -> list[str]:
    problems = []
    marked, named = markers(profile_rs), declared(profile_rs)
    if len(named) < MIN_PROFILES:
        problems.append(f"only {len(named)} profiles declared (floor {MIN_PROFILES}): the parse lost the registry")
    for name in sorted(set(named) - set(marked)):
        problems.append(f"profile `{name}` has no `// profile-contract:` marker")
    for name in sorted(set(marked) - set(named)):
        problems.append(f"marker `{name}` names no declared profile")
    if len(set(named)) != len(named):
        problems.append("a profile name is declared twice")
    if "SUPPORTED_PROFILES.iter()" not in witness_rs:
        problems.append("the witness no longer iterates SUPPORTED_PROFILES, so it can pass for any list")
    if "is_installed" not in witness_rs or "the_control_installs_every_capability" not in witness_rs:
        problems.append("the witness lost its control arm (the probe must be shown able to say yes)")
    return problems


def check_closure(headless_tree: str, windowed_tree: str) -> list[str]:
    problems = []
    headless, windowed = crates_in(headless_tree), crates_in(windowed_tree)
    if len(headless) < 40 or len(windowed) < 40:
        problems.append(f"closure parse found {len(headless)}/{len(windowed)} crates: the tree lost its shape")
    for crate in HEADLESS_ABSENT:
        if crate in headless:
            problems.append(f"headless closure contains `{crate}`: the headless profile promises it is absent")
    for crate in WINDOWED_PRESENT:
        if crate not in windowed:
            problems.append(f"windowed closure lacks `{crate}`: the windowed profile is not drawing")
    for crate in LINKED_NOT_ABSENT:
        for label, closure in (("headless", headless), ("windowed", windowed)):
            if crate not in closure:
                problems.append(
                    f"`{crate}` left the {label} closure: upgrade the profile contract from "
                    f"'not installed' to 'not linked' and remove it from LINKED_NOT_ABSENT"
                )
    return problems


def tree(*flags: str) -> str:
    return subprocess.run(
        ["cargo", "tree", "-p", HOST, "-e", "normal", *flags],
        cwd=REPO, check=True, capture_output=True, text=True,
    ).stdout


def main() -> int:
    problems = check_registry(
        (REPO / PROFILE_RS).read_text(encoding="utf-8"),
        (REPO / WITNESS_RS).read_text(encoding="utf-8"),
    )
    problems += check_closure(tree("--no-default-features"), tree())
    names = declared((REPO / PROFILE_RS).read_text(encoding="utf-8"))
    print(f"engine profiles: {len(names)} declared ({', '.join(names)})")
    for problem in problems:
        print(f"RED: {problem}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
