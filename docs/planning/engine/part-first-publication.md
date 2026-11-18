# Part-first publication: characters are published as parts, not frames

**State:** SELECTED, the top queue row (Jon, 2026-11-18: "This has to be the
highest priority in the queue to get everything on an efficient part based
path"). Parent: [runtime rigged sprite animation](runtime-rigged-sprite-animation.md).
Predecessor: [Mary-O drawn entirely from parts](mary-o-part-realization.md),
whose P5c (keep the baked oracle) this plan replaces.

## The defect

The game draws a character that publishes a part flipbook from its part
atlas and per-frame draw table, and never loads its baked sheet
(`parts_only_realization`, `ambition_platformer2d_assets::character_sprites`).
The publisher still paints every frame of that character whole at its
supersampled resolution, packs the frames into sheets, and builds the
half, quarter and potato sheets from them. The flipbook is recorded while it
paints, and every frame is painted again to check the draws against it.

Measured 2026-11-18 on the published store (`published.json`, one census of
every `sprites/*_spritesheet.ron`):

| sheets | count | frames | baked sheet bytes (all tiers) |
| --- | --- | --- | --- |
| characters with a part flipbook | 472 | 73,531 | 1.22 GB |
| characters without one | 114 | 6,055 | 115 MB |
| props | 809 | 5,838 | 126 MB |
| projectiles | 7 | 296 | 1 MB |

Of the 476 flipbooks, 471 draw every row from parts; 4 are realized baked by
the cost rule (`realization_by_cost`) and 1 is a hybrid. So almost all the
frame painting in a regeneration is for pictures no player sees. One
example: `gnu_ton_boss` took 93 minutes to publish, and its GNU2 frames cost
about 2.5 s each, nearly all of it rotating large rasters onto a frame the
game never loads (the giant loads one 1 MP part page).

### Where a publish spends its time (PF0, measured 2026-11-18)

cProfile of one publish worker each (`python -m cProfile -m
ambition_sprite2d_renderer.publication --worker <spec>`):

| | `hypatia_prime` (rig document, 176 frames) | `gnu_ton_boss` (giant, GNU2, scholar, fists) |
| --- | --- | --- |
| total | 21 s | 5,660 s (94 min) |
| frames painted for the sheet | 6.9 s | 3,924 s painting (`compose`), both passes |
| the same frames painted again, recorded, for the flipbook | 6.5 s | (in the above: every frame painted twice) |
| flipbook replay and sharing checks | 2.5 s | ~800 s |
| review images (`build_hitbox_debug`) | — | 424 s |
| quality tiers of the sheet | 1.0 s | ~200 s |
| quality tiers of the parts | 1.25 s | — |

GNU2's 929 frame paints cost 3,309 s alone (3.6 s each, rotating large
piece rasters onto a frame). Recording a draw is cheap: a piece's raster is
cached (`_pieces.rasterize`), so a record-only frame is its transforms.

### What reads painted pixels today

- `render_sheet`: the auto-crop (the union alpha box of every frame), the body
  box (`alpha_bbox_metrics` of the first frame), each mapped row's hurtbox
  (`pose_bodies="art"`: the union of its frames' visible boxes), the clipped
  edge warning, the canonical and preview images, and the pages.
- `build_rig_flipbook`: the replay guard and the near-twin and symmetric-half
  sharing (`_withdrawn_until_replayed`), each against every painted frame.
- The engine (until PF2): `build_optional_via_catalog` required the sheet
  page to exist before it looked for parts, and a quality tier was chosen
  only where its sheet page existed.

## The rule

1. A character's publish records each frame's draws (part rasters, their
   transforms, order, opacity, tint) and does not composite the frame.
2. Painting a whole frame happens only where something reads the picture:
   - the replay guard, on a fixed sample of each clip (its first frame, its
     middle frame and every frame a tween changes realization), not on every
     frame;
   - the canonical frame (`_canonical.png`) and portraits, one picture each;
   - the reference sheet, on request (below).
3. A character drawn from parts publishes no baked sheet pages. Its sheet
   record keeps the frame algebra the game builds a body with (rows, frame
   counts, durations, frame size, feet, boxes, seams, surfaces), computed from
   the draws, not from painted pixels.
4. The reference sheet (every frame painted, for review) is drawn on request
   (`scripts/regen/sprites.sh --reference <target>`), never by
   `scripts/regen/assets.sh` (Jon, 2026-11-18: "should be drawn as needed,
   not by default").
5. The asset inspector shows a character's canonical frame, and plays its
   animations from the part atlas and draw table in the page, so a page of
   characters costs part atlases, not sheets (Jon, 2026-11-18: "handle parts
   when it shows animations to be space and memory efficient").

## Phases

| Phase | What | Acceptance |
| --- | --- | --- |
| PF0 measure | Per-target build seconds in the publish log; the census above as a script; profile of `gnu_ton_boss` | the census and the profile are reproducible by a command |
| PF1 record without painting | `recorded_paint` gets a record-only mode: the seams record and return without painting; a supersampled frame's reduction is applied to the recorded parts as it is now. Metadata read from frame pixels (body boxes, unions) is computed from the draws | `gnu_ton_boss` publishes in a fraction of 93 minutes with the same draws: the sampled replay holds at today's bounds (`REPLAY_ROUNDING`, `CONTINUOUS_REPLAY_PARITY`) |
| PF2 no baked pages (engine half DONE 2026-11-18: a parts-only sheet and tier realize with no sheet page) | A parts-only character publishes no `_spritesheet*.png` at any tier; the game resolves the body from the sheet record and the part table; tests that read baked pages read parts | an all-parts character renders and animates in the game with no baked page in the store; the store's character sheet bytes fall by the measured 1.22 GB |
| PF3 reference on demand | `--reference <target>` paints the full sheet into the review tree; the asset inspector animates from parts and shows the canonical frame | `assets.sh` paints no reference sheet; the inspector plays a clip with no sheet present |
| PF4 smoother motion | Tween every part clip whose consecutive draws pair (`tween_draws`, `_same_picture_redrawn`), not only the selected locomotion loops; a clip authored to step (a cut, a flash) keeps stepping | clips step only where authored; the in-between overhang guards hold |
| PF5 convert the rest | The 114 character sheets with no flipbook get a part publisher (rig document, pieces, or the generator recorder); the 4 cost-rule baked flipbooks are re-measured | every character is drawn from parts; props and projectiles stay frames unless a prop animates many frames |
| PF6 measure | A full regeneration's wall time and store bytes, before and after | recorded here with the commands that measured them |

## Constraints kept

- The draws are the published truth: a frame the publisher no longer paints
  is still checked on its sample, and a target that paints outside the
  recorded seams (a direct `ImageDraw` over the frame) is caught by the
  sample of the clip it does it in. A target whose paint the recorder cannot
  see keeps painting until it is moved onto the seams.
- Quality tiers are made from parts (`generate_visual_quality_variants.py`
  crops and downsamples each part), never from a packed page.
- Gameplay never reads painted pixels: semantic landmarks come from the part
  tables (`baked_landmarks`), as now.
- No new coupling: the game keeps reading the sheet record and the part
  table it reads today.
