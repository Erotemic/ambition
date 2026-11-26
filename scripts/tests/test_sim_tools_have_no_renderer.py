"""CB4: the tools' `sim` family links no renderer.

`ambition_app_tools --no-default-features --features sim` is the composed
Ambition game at `ambition_app/rl_sim`: the stepping commands (`headless`,
`rl_smoke`, `trace_replay`, ...). Inside the workspace `cargo tree -p` resolves
only the selected package's features, so this reads that selection's closure.

Measured 2026-11-25: 443 crate lines on the normal edges (496 before this
cut). The three edges that had kept the renderer:

- the LDtk runtime spine (`bevy_ecs_ldtk`, whose `bevy_ecs_tilemap` asks Bevy
  for `bevy_render`), now under the app's `render`, since only a render app
  spawns what the spine indexes;
- `bevy_dev_tools` (the FPS counter and the F1 panel), also under `render`;
- Avian's default `debug-plugin`, which only `smash_match_profile` (a `game`
  bin) needs.

Surviving CPU-side Bevy crates, named so a reader does not mistake them for a
renderer: `bevy_ui`, `bevy_text`, `bevy_sprite`, `bevy_gizmos`, and `naga`
through `bevy_shader` (the shader asset type, not a GPU device).
"""

from __future__ import annotations

import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[2]
CARGO = pathlib.Path.home() / ".cargo" / "bin" / "cargo"

# Exact crate names: the renderer, the window, the GPU and the LDtk runtime.
FORBIDDEN = (
    "bevy_render",
    "bevy_core_pipeline",
    "bevy_pbr",
    "bevy_sprite_render",
    "bevy_ui_render",
    "bevy_winit",
    "wgpu",
    "winit",
    "ambition_render",
    "bevy_ecs_ldtk",
    "bevy_ecs_tilemap",
)

# The app, content and harness must be in the closure, and a closure this
# small means `cargo tree` resolved almost nothing.
CLOSURE_FLOOR = 300


def _closure(features: str) -> list[str]:
    proc = subprocess.run(
        [
            str(CARGO), "tree", "-p", "ambition_app_tools", "--no-default-features",
            "--features", features, "--edges", "normal", "--prefix", "none",
        ],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, f"`cargo tree` failed:\n{proc.stderr}"
    names = set()
    for line in proc.stdout.splitlines():
        m = re.match(r"^([A-Za-z0-9_-]+) v", line.strip())
        if m:
            names.add(m.group(1))
    return sorted(names)


def test_the_sim_tools_link_no_renderer() -> None:
    closure = _closure("sim")
    for required in ("ambition_app", "ambition_content", "ambition_sim_harness"):
        assert required in closure, f"{required} is not in the `sim` closure: {closure}"
    assert len(closure) >= CLOSURE_FLOOR, (
        f"{len(closure)} crates resolved (floor {CLOSURE_FLOOR}): the tree stopped resolving"
    )
    found = sorted(name for name in closure if name in FORBIDDEN)
    assert not found, (
        f"the `sim` tools link {found}. Find the edge with `cargo tree -p "
        "ambition_app_tools --no-default-features --features sim -e features -i bevy`"
    )


def test_the_game_tools_still_link_the_renderer() -> None:
    # The control: the same instrument sees the renderer where it must be.
    closure = _closure("game")
    for required in ("bevy_render", "ambition_render", "bevy_ecs_ldtk"):
        assert required in closure, f"`game` lost {required}; the instrument or the cut is wrong"
