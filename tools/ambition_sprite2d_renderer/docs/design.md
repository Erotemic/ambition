# Design notes

## Direction

This package is now organized around a target-neutral pipeline:

1. load a small YAML job,
2. choose a target adapter,
3. sample a deterministic character spec,
4. render frames through the target generator,
5. compose a labeled sprite sheet and manifest.

The adapter layer keeps target-specific rendering code isolated while the CLI,
manifest format, and YAML configuration stay stable.

## Robot target

The robot target lives in `proc2d_character_lab/targets/robot25d.py`. It is the
current polished target and should be treated as the design reference.

The robot is a left-facing side-scroller enemy. It uses:

- a 3D-aware head and torso volume,
- orthographic projection,
- visor-local eye placement,
- depth-aware limb ordering,
- automatic overscan / fit-to-frame for wide poses,
- labeled row export in `sheet.py`.

The boost animation is explicitly staged for leftward acceleration: head and
torso lead left, limbs trail right, and speed streaks stay behind the robot.

The death animation uses normal eyes at the start, transitions early to X eyes,
and then fades the X as power drops.

## Goblin target

The goblin target is still backed by `legacy/goblin_legacy.py`. It remains
useful for experimentation but is intentionally marked legacy because it still
uses more ad-hoc 2D placement.

## Shared rig primitives

`rig.py` defines future reusable rig pieces:

- `Bone`
- `SocketSpec`
- `FaceGuide`
- `Rig.validate()`

The robot target demonstrates the desired direction; future goblin work should
migrate toward named sockets, consistent face guides, weapon sockets, and
validator-friendly pose data.

## Package standards

- Keep target code in `proc2d_character_lab/targets/`.
- Keep one-off historical prototypes in `proc2d_character_lab/legacy/`.
- Keep the adapter API small: `animations`, `sample_spec`, `render_frame`.
- Keep YAML jobs human-editable and deterministic.
- Keep generated sprite sheets and manifests outside the package, normally in an
  `assets/` directory.
