# Character texel density

Status: the mechanism is built; only Wolfenstein uses it. Rolling it out to
the cast is ON HOLD (Jon, 2026-10-17: "Hold off on the density for the entire
cast. Document it in docs/planning.").

## The problem

Jon, from in-game observation: "several characters are more blury than I want
them to be." A character's in-game size does not come from its frame's pixels.
The game reads its body box against the height it stands in the world: a
catalog `standing_height`, or a boss's body box. So a sheet's resolution
changes nothing about its size, only its sharpness.

The default camera (`CameraZoomPreset::Duel`) shows 320 world units of height.
At 1080p that is 3.375 screen pixels per unit. MEASURED 2026-10-17: the median
character sheet has 1.75 pixels per unit, so the game draws it about 2x
magnified, and it looks soft.

## The mechanism

- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/texel_density.py`
  holds the one number, `TEXELS_PER_WORLD_UNIT = 3.4`, and
  `published_frame(frame, body_px, world_height)`. That is the frame a
  character is published at so that its body has that density. The frame is
  never smaller than the authored one, and its sides are even.
- `Person(world_height=...)` in `_person_rig.py` and `Kit(out=, zoom=)` in
  `_ai_era_kit.py` paint at the supersampled size, then reduce to the
  published frame. Authoring stays in the 320 px logical frame. Only the
  output grows.
- Wolfenstein is the worked example. He is a 320 logical frame with a 165 px
  body standing 100 world units tall, published at 660 px.

## What a rollout costs

These are open issues, not yet decided:

- **Texture memory.** Going from 1.75 to 3.4 is about 3.8x the pixels per
  character (REASONED: (3.4/1.75)²). Pages past 2048 px break WebGL2, so
  sheets split into more pages (`test_sprite_page_dimensions.py` ratchets the
  count).
- **Composite cell.** The rigged-sprite composite cell is 896 px. A frame
  larger than that is refused, which caps density times frame size.
- **Mipmaps.** Above about 1.25x minification (zoomed-out cameras), a denser
  sheet aliases. The sheets have no mip chain today.
- **Quality tiers.** Low and Medium draw the half tier (`sprites_0_5x`), so a
  3.4 sheet there is 1.7, about today's full tier. A rollout does nothing for
  Low unless the tiers change too (Jon: screenshots at `low`).
- **Closer cameras.** A framed-cast zoom needs about 4.5 for one texel per
  pixel. 3.4 is set for the default camera.
- **Two world heights for one body.** Wolfenstein's catalog
  `standing_height` is 93, and his boss body box is 100. Density is
  computed against one of them, and the other then disagrees.
- **A hand-copied ratio.** `boss_sheets.ron`'s `collision_scale` must equal
  frame px / body px. It is typed by hand (1.939 for Wolfenstein), so a
  republish at a new density silently mis-sizes the boss unless it is
  re-typed. It should be read from the published manifest.
- **Per character work.** Each character needs its `world_height` set in its
  renderer script, then a republish, then a regen of the quality variants.

## When it resumes

1. Read `collision_scale` from the manifest instead of authoring it.
2. Settle one world height per body.
3. Decide the tiers and mipmaps.
4. Then republish the cast, one target at a time.
