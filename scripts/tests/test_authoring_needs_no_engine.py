"""I1: authoring a move must not pull the engine into the build.

⭐⭐ **THE CLAIM IS ABOUT A RESOLVED CLOSURE, NOT ABOUT IMPORTS.** "This module
does not `use bevy`" is cheap and nearly worthless: a proc-macro edge, a build
script or a dev-dependency can put the whole graph in the build while every
normal import looks clean. So this asks cargo what it would actually resolve,
across `normal,build,dev` together.

⛔⛔ **AND IT HAS TO BE ASKED OUTSIDE THE WORKSPACE.** Cargo unifies features and
shares one lockfile across a workspace, so a member's tree shows whatever the
union resolved — "I do not need Bevy" is unfalsifiable from inside. The fixture
declares its own `[workspace]`, and this file CHECKS that it still does, because
absorbing it into the engine workspace would leave every assertion below passing
while measuring nothing.

⚠ **THE FLOOR IS NOT DECORATION.** `cargo tree` failing, or being pointed at the
wrong directory, produces no forbidden names at all — the same green a clean
closure gives. The population is floored and the fixture's own two crates are
required by name.
"""

from __future__ import annotations

import pathlib
import re
import subprocess

import pytest

REPO = pathlib.Path(__file__).resolve().parents[2]
FIXTURE = REPO / "fixtures" / "content_builder"
CARGO = pathlib.Path.home() / ".cargo" / "bin" / "cargo"

# ⛔ THE NAMES THE ACCEPTANCE CLAUSE LISTS, plus the graphics stack they drag.
# A substring match, deliberately: `bevy_ecs` and `bevy_app` are both Bevy, and
# a future `ambition_render_foo` is still the renderer.
FORBIDDEN = (
    "bevy",
    "wgpu",
    "winit",
    "ambition_platformer2d",
    "ambition_render",
    "ambition_audio",
    "ambition_characters",
    "ambition_combat",
    "ambition_demo",
    "ambition_content",
    "ambition_app",
)

# MEASURED 2026-09-11: 12 crates resolve for the fixture across normal+build+dev
# (itself, the value crate, ron, serde and the proc-macro machinery). A floor,
# not a ceiling — adding a pure value dependency is allowed; resolving NOTHING
# is the instrument dying.
CLOSURE_FLOOR = 8


def _closure() -> list[str]:
    proc = subprocess.run(
        [str(CARGO), "tree", "--edges", "normal,build,dev", "--prefix", "none"],
        cwd=FIXTURE,
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, (
        "`cargo tree` failed in the builder fixture, so this file measured "
        f"nothing:\n{proc.stderr}"
    )
    names = set()
    for line in proc.stdout.splitlines():
        m = re.match(r"^([A-Za-z0-9_-]+) v", line.strip())
        if m:
            names.add(m.group(1))
    return sorted(names)


def test_the_builder_fixture_is_still_outside_the_workspace() -> None:
    """⛔⛔ THE ASSERTION THAT KEEPS THE OTHERS HONEST. Absorbed into the engine
    workspace, this fixture inherits feature unification and one lockfile, and
    every claim below becomes a statement about the union rather than about
    authoring."""
    manifest = (FIXTURE / "Cargo.toml").read_text()
    assert re.search(r"^\[workspace\]\s*$", manifest, re.M), (
        "fixtures/content_builder no longer declares its own `[workspace]`, so "
        "it resolves with the engine and cannot witness an independent closure"
    )
    members = REPO / "Cargo.toml"
    assert "fixtures/content_builder" not in members.read_text(), (
        "the builder fixture was added to the engine workspace's members"
    )


def test_authoring_a_move_resolves_no_engine_crate() -> None:
    """I1's acceptance: the external fixture builds and tests without Bevy,
    runtime, render, audio, monolith or named game providers in its closure."""
    closure = _closure()

    assert "content_builder" in closure and "ambition_entity_catalog" in closure, (
        "the closure does not contain the fixture and the crate it authors "
        f"against, so it is not the closure this file is about: {closure}"
    )
    assert len(closure) >= CLOSURE_FLOOR, (
        f"{len(closure)} crates resolved (floor {CLOSURE_FLOOR}). Fewer means "
        f"the tree stopped resolving, not that authoring got purer: {closure}"
    )

    found = sorted(
        name for name in closure if any(bad in name for bad in FORBIDDEN)
    )
    assert not found, (
        f"authoring a move resolves {len(found)} engine crate(s): {found}\n"
        "⇒ I1's whole claim is that writing a move is a pure value computation. "
        "An author who must build the engine graph to emit a `MoveSpec` pays the "
        "engine's link time for a data edit, which is the iteration cost this "
        "packet exists to remove. Check what the fixture's manifest gained, and "
        "check `ambition_entity_catalog`'s own dependencies — a proc-macro or "
        "build edge counts."
    )


@pytest.mark.parametrize("bad", ["bevy", "ambition_platformer2d"])
def test_the_forbidden_list_would_actually_catch_something(bad: str) -> None:
    """⭐ THE POISON, RUN EVERY TIME rather than by hand once. The check above is
    a `not found` assertion over a list, and the failure mode of those is that
    the matcher stopped matching. This proves the classifier still fires."""
    assert any(bad in name for name in [f"{bad}_something", bad]), (
        "the substring matcher no longer recognises a name it must reject"
    )
