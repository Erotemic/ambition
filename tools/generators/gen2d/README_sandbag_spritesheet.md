# Sandbag spritesheet generator (moved)

The sandbag generator moved into the standalone
[`tools/ambition_sprite2d_renderer`](../../ambition_sprite2d_renderer/README.md)
package and is now exposed as the `sandbag` target.

## Generate / install

```bash
python -m ambition_sprite2d_renderer render sandbag
python -m ambition_sprite2d_renderer render-publish sandbag
```

`render-publish` writes the sheet under
`tools/ambition_sprite2d_renderer/generated/sandbag/` and copies the PNG +
YAML into `crates/ambition_sandbox/assets/sprites/`.

## Compatibility shims

These continue to work but print a deprecation note:

- `tools/generators/gen2d/draw_sandbag_spritesheet.py`
- `tools/generators/gen2d/generate_sandbag_assets.sh`

## Animations (sparse default)

- `idle`: 6 frames, subtle breathing/bob
- `hit`: 4 frames, squash + recoil + impact burst
- `death`: 7 frames, topple/collapse + dust

The runtime resolves missing animations (walk/run/slash/jump/fall/blink/dash)
to `idle` at load time. Pass `--legacy-aliases` to also emit the 11-row
alias sheet for old-runtime compatibility.
