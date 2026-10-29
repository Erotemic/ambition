# Procedural parts bypass the shared transform cache

`shape_rig.place` calls `rigdoc.blit_rotated` without a prepared raster or
transform cache. Each nonzero turn allocates a padded canvas and converts its
pixels to premultiplied alpha. SVG rigs use `SpriteRaster` and the existing
byte-bounded `SpriteTransformCache` for the same work.

A live generation profile found this path in the Goblin and Pirate Heavy
painters. A private probe rendered 16 Goblin idle and run frames at 256 by 256
pixels with the configured supersampling. The direct path took 3.874 seconds.
The shared cache took 1.770 seconds. SHA-256 hashes of every frame matched.
The probe used bicubic rotation in both paths. Other render workers were active,
so repeat the timing in isolation before using it as a throughput claim.

Use one preparation and transform path for immutable procedural and SVG parts.
Keep the existing cache byte limit. Do not add an unbounded rotation cache or
quantize pose angles. Check recorded part replay, squash, opacity, and quality
tiers before admission. The 16-frame result does not prove full-sheet parity or
a complete publication speedup.

Source owners:

- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/shape_rig.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/rigdoc.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/core/draw.py`
