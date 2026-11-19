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
| PF3 reference on demand (DONE 2026-11-18, see [State](#state-2026-11-18)) | `--reference <target>` paints the full sheet into `.agent/sprite_reference/<target>/` (git-ignored, never published); the asset inspector animates from parts and shows the canonical frame | `assets.sh` paints no reference sheet; the inspector plays a clip with no sheet present |
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
  characters. **PF2b content (2026-11-18):** every boss sheet with a
  character whose sheet it is now names it (`boss_sheets.ron` `parts`:
  `npc_boss` for the Gradient Sentinel, `npc_mockingbird_boss`,
  `npc_smirking_behemoth_boss`, `npc_flying_spaghetti_monster_boss`,
  `npc_bossofthegame`, beside the T-rex, the Hunny Horror and the two
  Wolfensteins that did), and those nine sheets left `baked_pages`. A
  part-drawn boss whose flipbook has no face track flinches whole: it is
  drawn in its `Hit` frame from parts, as its baked sheet drew it
  (`rigged::frame_draws`, `RiggedSpriteAsset::has_expression_tracks`),
  unless parts posed from its row carry its hits. Under the audit, part-first:
  Gradient Sentinel 50 of 50 frames, Mockingbird 63/63, Smirking Behemoth
  26/26, FSM 79/79 (its sauce layer stays a painted sheet; its reveal map is
  drawn beside the seams and came out identical to the store's), Boss of
  the Game 85/85, T-rex 97/97, Hunny Horror 107/107, Wolfenstein 605/605,
  the Inugami 553/553 (painted for its flipbook: its paint reads pixels it
  painted). Published page-less into the local store, the Mockingbird drew
  the generic body: the boss catalog resolved a boss sheet only by a page
  file that exists (`try_path_for_load`). `boss::boss_sheet_path` now
  resolves a `drawn_from_parts` sheet by the page path it declares, which
  finds its record; with it the composed game's boss tests pass on the
  page-less Gradient Sentinel and Mockingbird. Still baked (`baked_pages` says why): the
  Clockwork Warden and the Hooded Sentinel (part flipbooks, no character
  row), the Gram Reaper and GNU-ton's scholar (no part flipbook), the giant
  gnu (`gnu_ton_boss`'s bespoke publish).
- **Final audit (2026-11-18, renderer `9347716`):** every flipbook module
  unit but `gnu_ton_boss` and the two Wolfensteins (boss art) published
  part-first with every frame painted and checked: 434 of 434 replay the
  paint within the guard; 414 publish page-less. The rest paint by design:
  boss art (`baked_pages`), `history_fukuzawa_yukichi` (66 draws in a frame,
  over the 64 budget), and the bespoke publishers below. Hypatia Prime,
  republished into the store page-less, realizes parts-only in the composed
  game with her part page loaded one frame after the request.
- **The generator road is part-first (2026-11-18).** `write_spritesheet`
  records a config sheet's flipbook without painting
  (`sheet._record_part_first`), measures the record off its redraw (the first
  frame painted, for the body box), lays the rows on a frame grid and writes
  no page; before, every frame was painted twice (the sheet, then the
  flipbook). A tier of a page-less config sheet is derived (record scaled,
  parts downsampled) instead of rendered again at the tier's scale
  (`publication._worker`). Under the audit: `robot` 173 of 173 frames,
  `general_hero` 67 of 67, `alice` 375 of 375, `goblin` 798 of 798 (its
  `blink_out` paints outside the seams, so it is recorded from its painted
  frames, still page-less). Only a generator that publishes a flipbook
  (`publishes_part_flipbook`: the toon, robot, goblin, boss, Alice and
  sandbag sides) goes part-first; the others are PF5.
- **The bespoke publishers (2026-11-18).** Noether, player_robot_v3 and
  Mary-O v2's three forms painted their sheet and then painted every frame
  again for their own flipbook; now `build_sheet(parts=True)` records the
  flipbook and `part_flipbook.write_part_first` writes it (their tweened
  rows, the robot's eye landmarks). Under the audit: player_robot_v3 2142 of
  2142 frames, Noether 938/938 (recorded from its painted frames: it paints
  outside the seams), Mary-O v2 25/25; Mary-O's strict test compares her
  flipbook with a painted reference sheet. **player_robot_v3 keeps its page
  for now** (`baked_pages`): every LDtk world's `PlayerStart` tileset
  (`sprite_player_robot_v3`) is that page, and the runtime decodes it at
  boot; the tileset must be retargeted first
  (`tools/ambition_ldtk_tools/propose_ldtk_tileset_retarget.py`, the worlds
  in `game/ambition_map_assets`). Painted on purpose: Jeff Hinter armored
  (87 draws in a frame, over the 64 budget) and Fukuzawa (66), the cosmic
  Mary-O (a hybrid: her transformation rows carry light on the whole figure
  that no part carries, so they stay baked clips).
- Not yet part-first: `sandbag` (its own sheet writer, an adapter-shaped
  record) and `gnu_ton_boss`'s layered publish (full, body, hands, fist,
  GNU2 and rider sheets; the boss road reads its body and hands layers).
- **PF4 measured (2026-11-18): tweening every pairable clip misplaces
  limbs.** Two findings. (1) A tweened clip moves its last frame toward its
  first (`tween_draws`, the runtime's `tween_targets`): right for a loop,
  wrong for a clip played once, so a once-only tween (last frame held)
  would be needed. (2) More decisive: Mary-O's in-between oracle (her rig
  posed at the lerped keyframe, `test_mary_o_part_flipbook`), run over
  every pose row with every clip tweened, misplaces parts the pairing rule
  accepts: her run and dash legs are a raster drawn again per frame at a
  new bend, which passes `_same_picture_redrawn` and is slid where the pose
  turns it (7–49°, up to 4 px), and `throw_up`'s far arm is one part half a
  turn apart, which the tween spins through 180°. That is why her `run`
  was never a tweened row: the clips that step are authored, by the
  oracle. PF4 is therefore per target: a clip tweens when an in-between
  oracle of its own rig holds it (as Mary-O's does for walk, crouch_walk,
  climb, swim), not because its draws pair. The work is an in-between
  render for each rig road (rig documents can pose a fractional keyframe;
  the generator road and the pieces cannot yet), then the oracle per
  target, then its tween rows.
- **PF3 done (2026-11-18).** `scripts/regen/sprites.sh --reference --target
  <name>` paints a part-first sheet's every frame into
  `.agent/sprite_reference/<name>/` (`PAINT_REFERENCE_ENV`; Hypatia in 16 s)
  and publishes nothing. The asset inspector plays a page-less sheet from its
  part atlas and draw table (`asset_index/sheets.read_part_draws`, a
  `parts/<key>/draws.js` loaded when the sheet is opened; `web/app.js`
  `drawPartFrame`), and draws its cover and body camera from parts
  (`thumbs.parts_task`); a baked sheet still plays from strips. The
  inspector's frame matches the renderer's `PartFlipbook.recompose` with 0
  wrong pixels on the frames checked; against a `--reference` paint of
  today's code, 42 of 44 sampled Hypatia frames match and her `run` differs by
  12% because the store's run clip predates today's paint (the full republish
  brings it level).

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
