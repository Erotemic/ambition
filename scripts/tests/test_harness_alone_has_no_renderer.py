"""CB4: a consumer of the simulation harness alone links no renderer.

The claim is about a resolved closure, asked outside the workspace: inside
it, cargo unifies every member's features with the app's, so a renderer
would appear whether or not the harness asked for one. The fixture
`fixtures/harness_alone` declares its own `[workspace]`, and this file checks
that it still does. Its test (`cargo test` there) steps and observes a sim;
this file reads what that consumer links.

Measured 2026-11-25: 372 unique crates on the normal edges. Absent:
`bevy_render`, `wgpu`, `bevy_winit`, `ambition_render`, `ambition_app`,
`ambition_content`. Surviving CPU-side Bevy crates, named so a reader does not
mistake them for a renderer: `bevy_ui`, `bevy_text` and `bevy_sprite`,
through `ambition_platformer2d_host` (which asks Bevy for `bevy_ui`) and
`ambition_sprite_sheet` (which asks for `bevy_sprite`).
"""

from __future__ import annotations

import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[2]
FIXTURE = REPO / "fixtures" / "harness_alone"
CARGO = pathlib.Path.home() / ".cargo" / "bin" / "cargo"

# Exact crate names: the renderer, the window, the GPU and the product shell.
FORBIDDEN = (
    "bevy_render",
    "bevy_core_pipeline",
    "bevy_winit",
    "wgpu",
    "winit",
    "ambition_render",
    "ambition_app",
    "ambition_content",
)

# The fixture, the harness and the facade must be in the closure, and a
# closure this small means `cargo tree` resolved almost nothing.
CLOSURE_FLOOR = 200


def _closure() -> list[str]:
    proc = subprocess.run(
        [str(CARGO), "tree", "--edges", "normal", "--prefix", "none"],
        cwd=FIXTURE,
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, f"`cargo tree` failed in the harness fixture:\n{proc.stderr}"
    names = set()
    for line in proc.stdout.splitlines():
        m = re.match(r"^([A-Za-z0-9_-]+) v", line.strip())
        if m:
            names.add(m.group(1))
    return sorted(names)


def test_the_harness_fixture_is_still_outside_the_workspace() -> None:
    manifest = (FIXTURE / "Cargo.toml").read_text()
    assert re.search(r"^\[workspace\]\s*$", manifest, re.M), (
        "fixtures/harness_alone no longer declares its own `[workspace]`, so it "
        "resolves with the engine and cannot witness an independent closure"
    )
    assert "fixtures/harness_alone" not in (REPO / "Cargo.toml").read_text(), (
        "the harness fixture was added to the engine workspace's members"
    )


def test_a_harness_alone_consumer_links_no_renderer() -> None:
    closure = _closure()
    for required in ("harness_alone", "ambition_sim_harness", "ambition_platformer2d"):
        assert required in closure, f"{required} is not in the closure, so it is not this fixture's: {closure}"
    assert len(closure) >= CLOSURE_FLOOR, (
        f"{len(closure)} crates resolved (floor {CLOSURE_FLOOR}): the tree stopped resolving"
    )
    found = sorted(name for name in closure if name in FORBIDDEN)
    assert not found, (
        f"a consumer of the harness alone links {found}. The harness asks the "
        "facade for `rollback` only and keeps the renderer behind `capture`; "
        "check which manifest gained a default or a feature."
    )
