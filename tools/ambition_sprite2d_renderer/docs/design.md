# Design notes

## Direction

The sprite renderer now uses one shared character pipeline for robot, goblin,
boss, and sandbag:

1. load a small YAML job,
2. choose a target adapter,
3. sample a deterministic spec,
4. render animation frames,
5. compose a labeled sprite sheet and manifest.

The adapter layer keeps target-specific drawing isolated while config shape,
manifest shape, body metrics, canonical previews, and generated-sheet naming
stay stable.

## Shared animation vocabulary

`animation_vocab.py` is the canonical place for row names. The core vocabulary
matches the runtime character grid:

```text
idle, walk, run, jump, fall, slash, hit, death, blink_out, blink_in, dash
```

The extended review vocabulary covers player mechanics that exist or are likely
near-term but do not yet have a complete Rust animation selector:

```text
crouch, wall_slide, wall_jump, ledge_grab, climb, swim, interact, talk, block
```

Extended rows should be generated in Python first, reviewed visually, and then
wired into Rust deliberately in a later integration pass.

## Robot target

The side-view robot remains the primary player-character design reference. It
now supports the core runtime rows plus an extended review set for mechanics
that previously fell back to weak or mismatched poses:

- crouch / compressed idle,
- wall slide,
- wall jump,
- ledge grab / pull-up,
- climb,
- swim,
- interact,
- talk,
- block / guard.

`configs/robot.yaml` stays runtime-compatible. `configs/player_extended.yaml`
is the review sheet for the richer set.

## Sandbag target

Sandbag is no longer only a tack-on renderer. `SandbagAdapter` wraps the same
procedural sandbag drawing code behind `BaseAdapter`, and `configs/sandbag.yaml`
renders an 11-row runtime-compatible sheet with `crop: false` so 128×128 cells
are preserved.

The tack-on command renders the sparse idle/hit/death output:

```bash
python -m ambition_sprite2d_renderer render sandbag
```

## Variants

`draw-all` uses the config filename as the output stem. This allows several
jobs for the same adapter without overwriting outputs:

```text
robot_spritesheet.png
robot_runner_spritesheet.png
robot_guardian_spritesheet.png
player_extended_spritesheet.png
```

Future variants should normally be new YAML jobs first. Add new target code only
when a variant needs a different body plan or renderer.

## Shared rig primitives

`rig.py` defines future reusable rig pieces:

- `Bone`
- `SocketSpec`
- `FaceGuide`
- `Rig.validate()`

The robot target demonstrates the desired direction; future goblin and sandbag
work should migrate toward named sockets, consistent face guides, weapon
sockets, and validator-friendly pose data.

## Side-view walk-cycle baseline

The side-facing biped lanes now share one authored walk/run philosophy: an
8-frame contact/down/passing/up loop driven by ankle targets, with the knee
solved by a two-bone IK pass and an explicit near/far limb draw order. This
produces planted feet, cleaner silhouettes, and more stable depth reads than
the older direct-angle-only leg swing.

See `walk_cycle_baseline.md` for the practical recipe, the target list, and the warning that mostly-front-facing rigs such as ninja should not borrow this side-profile treatment.

## Package standards

- Keep target code under `ambition_sprite2d_renderer/targets/<category>/`
  where category is one of `characters/`, `props/`, `tiles/`, `icons/`.
  The registry walks these dirs at import time; see
  [`registry/discovery.py`](../ambition_sprite2d_renderer/registry/discovery.py)
  for the discovery contract and the README's "Adding a new sprite"
  section for the practical walkthrough.
- Generic helpers (drawing primitives, the `build_sheet` pipeline,
  RON emitters) live at the package root —
  [`tackon_sheet.py`](../ambition_sprite2d_renderer/authoring/tackon_sheet.py) and
  [`common_draw.py`](../ambition_sprite2d_renderer/authoring/common_draw.py).
- Character-family helpers (shared by several characters in a family,
  e.g. pirates) live under `targets/characters/` with a leading
  underscore so discovery skips them — see `_pirate_common.py`.
- Keep historical prototypes in `ambition_sprite2d_renderer/legacy/`.
- Keep the adapter API small: `animations`, `sample_spec`, `render_frame`.
- Keep YAML jobs human-editable and deterministic.
- Keep generated sprite sheets and manifests outside the package, normally in
  `generated/` or a deliberate asset install destination.
