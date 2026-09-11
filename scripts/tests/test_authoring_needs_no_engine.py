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

# ⛔⛔ **TWO LISTS, BECAUSE A SUBSTRING MATCHER ANSWERS ABOUT A DIFFERENT
# POPULATION.** The first version used one substring list containing
# `ambition_content` — which matched `ambition_content_pack`, the Bevy-free
# envelope crate (ron + serde + thiserror) that an outside author SHOULD depend
# on. The guard failed on a dependency that is exactly what I2 wants. A peer hit
# the same shape the same day with `git grep "GroundItem {"` matching an
# unrelated enum variant: a matcher that is confidently wrong about WHICH set it
# is describing.
#
# ⇒ A PREFIX where the family is real (`bevy_ecs`, `bevy_app` and a future
# `ambition_render_foo` all belong), an EXACT name where it is one crate.
FORBIDDEN_PREFIXES = (
    "bevy",
    "wgpu",
    "winit",
    "ambition_platformer2d",
    "ambition_render",
    "ambition_audio",
    "ambition_demo",
)
FORBIDDEN_EXACT = (
    "ambition_characters",
    "ambition_combat",
    "ambition_content",
    "ambition_app",
)


def _is_engine(name: str) -> bool:
    return name in FORBIDDEN_EXACT or any(
        name.startswith(p) for p in FORBIDDEN_PREFIXES
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

    found = sorted(name for name in closure if _is_engine(name))
    assert not found, (
        f"authoring a move resolves {len(found)} engine crate(s): {found}\n"
        "⇒ I1's whole claim is that writing a move is a pure value computation. "
        "An author who must build the engine graph to emit a `MoveSpec` pays the "
        "engine's link time for a data edit, which is the iteration cost this "
        "packet exists to remove. Check what the fixture's manifest gained, and "
        "check `ambition_entity_catalog`'s own dependencies — a proc-macro or "
        "build edge counts."
    )


@pytest.mark.parametrize(
    "name", ["bevy", "bevy_ecs", "ambition_platformer2d", "ambition_characters"]
)
def test_the_classifier_still_rejects_what_it_must(name: str) -> None:
    """⭐ THE POISON, RUN EVERY TIME rather than by hand once. The check above is
    a `not found` assertion over a list, and the failure mode of those is that
    the matcher stopped matching."""
    assert _is_engine(name), f"the classifier no longer rejects `{name}`"


@pytest.mark.parametrize(
    "name", ["ambition_content_pack", "ambition_entity_catalog", "ron", "serde"]
)
def test_the_classifier_does_not_reject_a_pure_value_crate(name: str) -> None:
    """⛔⛔ THE CONTROL THAT THIS FILE DID NOT HAVE, AND THE ABSENCE COST A FALSE
    RED. `ambition_content_pack` is `ron + serde + thiserror` — the envelope an
    outside author emits through, and precisely what I2 wants in this closure.
    A substring list containing `ambition_content` rejected it."""
    assert not _is_engine(name), (
        f"`{name}` is a pure value crate and the classifier calls it the engine, "
        "so this guard fails on exactly the dependency the packet asks for"
    )
