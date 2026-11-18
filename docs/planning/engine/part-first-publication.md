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
| PF2 no baked pages (engine half DONE 2026-11-18: a parts-only sheet and tier realize with no sheet page; the module road DONE 2026-11-18, see [State](#state-2026-11-18)) | A parts-only character publishes no `_spritesheet*.png` at any tier; the game resolves the body from the sheet record and the part table; tests that read baked pages read parts | an all-parts character renders and animates in the game with no baked page in the store; the store's character sheet bytes fall by the measured 1.22 GB |
| PF3 reference on demand | `--reference <target>` paints the full sheet into the review tree; the asset inspector animates from parts and shows the canonical frame | `assets.sh` paints no reference sheet; the inspector plays a clip with no sheet present |
| PF4 smoother motion | Tween every part clip whose consecutive draws pair (`tween_draws`, `_same_picture_redrawn`), not only the selected locomotion loops; a clip authored to step (a cut, a flash) keeps stepping | clips step only where authored; the in-between overhang guards hold |
| PF5 convert the rest | The 114 character sheets with no flipbook get a part publisher (rig document, pieces, or the generator recorder); the 4 cost-rule baked flipbooks are re-measured | every character is drawn from parts; props and projectiles stay frames unless a prop animates many frames |
| PF6 measure | A full regeneration's wall time and store bytes, before and after | recorded here with the commands that measured them |

## State (2026-11-18)

**The module road is part-first.** `build_sheet(parts=True)` (renderer
`authoring/sheet_build.py`) records every frame's draws
(`build_rig_flipbook(paint=False)`), paints the first row's first and
canonical frames and one sample of each clip, redraws the rest from the
flipbook for the crop, the boxes and the clipped-edge warning, and writes
no page and no preview. Its record states `drawn_from_parts` and keeps the
frame algebra on a frame grid. All 86 modules that publish a rig flipbook
(and the kits they share, so 437 sheets) call it; publication
(`artifacts.collect`, the quality tiers) skips a page-less sheet's pages.

- **A paint that reads its own pixels** (a swing trail drawn from the frames
  before it, a piece raster cropped to what it drew, a plant measured off the
  idle frames) comes out blank when only recorded, and the sampled replay
  refuses it. Such a sheet paints its frames for the flipbook, once, and
  still writes no page (the fallback in `_part_first_rows`). Seen on the
  swing fighters (`director`, `carl_stargan`, the polygons) and the
  behemoth's bursts.
- **Found on the way: a seam imported by name was never recorded.** A
  module that did `from .rigdoc import faded_canvas` called the original,
  so its frame rode as ONE whole-frame overlay: the spectral wolf and the
  pirates' mirrored rows published their frames as their "parts". The
  recorder now patches every binding (`_seam_bindings`); the wolf's part
  page fell from 1.7 MB to 110 KB.
- **The record is computed from the draws, not the paint.** Against a
  painted publish of the same code, 68 of 80 sheets differ by a pixel or two
  in their crop, feet and boxes (a redrawn edge pixel is not the painted
  one), and near-twin sharing is judged against the redraw. The measure of
  correctness is the painted publish's own: `AMBITION_PART_FIRST_AUDIT=1`
  paints every frame and holds the part-first flipbook to the replay guard
  against it.
- **The first full audit failed 256 of 434 sheets, two causes.** (1) The
  lossy sharing steps (near twins, symmetric halves) were judged against the
  unshared redraw, so their error stacked on the redraw's own; they are now
  judged against the paint of each frame a candidate is drawn in, painted on
  demand (`_PaintOnDemand`). Skipping them instead would have grown part
  pages by 7–25% (Alice 1.29 → 1.39 MTexel, Kenji 245k → 309k). (2) The
  townsfolk's composed `turnaround`/`turn_away` rows mirrored a view with a
  bare `transpose`, outside the seams (blank when recorded only, one
  whole-frame picture when painted); `chargen.sheets` now mirrors through
  `rigdoc.mirrored_canvas`.
- **Boss art keeps its pages (PF2b).** The boss road
  (`ambition_sprite_sheet::boss`) loads a boss sheet's pages and the room
  barrier waits on them, and most boss sheets name no character to draw
  their parts. `authored/sprites.yaml` `baked_pages` names every file in
  `data/boss_art_keys.ron`; `build_sheet` paints those as before.
  **PF2b engine half done (2026-11-18):** a boss sheet whose record is
  `drawn_from_parts` realizes `NO_BAKED_IMAGE` pages (`build_boss_pages`),
  the room barrier skips them, and a boss room demands and waits on the
  character its parts are drawn from (`BossSheetSpec::parts`) with its own
  characters. **PF2b content, open:** each boss sheet names its character
  (`npc_boss`, `npc_mockingbird_boss`, `npc_smirking_behemoth_boss`,
  `npc_flying_spaghetti_monster_boss`, `npc_bossofthegame` exist; the
  Clockwork Warden, the Hooded Sentinel, the Gram Reaper and GNU-ton's
  scholar have no character row), and a hit on a part-drawn boss whose
  flipbook has no face tracks must still read: today `hit_reaction` gives a
  part-drawn boss no `Hit` cell and `hurt_face` draws a face only from face
  tracks (`face_as`), so moving such a boss onto parts loses its hit
  reaction. Then each moved sheet leaves `baked_pages`.
- **Final audit (2026-11-18, renderer `9347716`):** every flipbook module
  unit but `gnu_ton_boss` and the two Wolfensteins (boss art) published
  part-first with every frame painted and checked: 434 of 434 replay the
  paint within the guard; 414 publish page-less. The rest paint by design:
  boss art (`baked_pages`), `history_fukuzawa_yukichi` (66 draws in a frame,
  over the 64 budget), and the bespoke publishers below. Hypatia Prime,
  republished into the store page-less, realizes parts-only in the composed
  game with her part page loaded one frame after the request.
- Not yet part-first: the bespoke publishers (`mary_o_v2` and its forms,
  `noether`, `player_robot_v3`, `jeff_hinter_armored`, `sandbag`), the generator road (`authoring/sheet.py`, 35 config
  sheets), `gnu_ton_boss`'s bespoke publish, and the asset inspector, which
  draws thumbnails from sheet pages (PF3).

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
