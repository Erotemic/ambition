# Mary-O drawn entirely from parts

**State:** IN PROGRESS. Parent:
[`runtime-rigged-sprite-animation.md`](runtime-rigged-sprite-animation.md).
Jon accepted the recommendations below (D1–D6) on 2026-10-02. Mary-O goes first;
every rig-document character follows the same road.

## Run progress

| Phase | State | Receipt |
| --- | --- | --- |
| P0 offline gate | DONE | renderer `tests/test_mary_o_part_flipbook.py`: strict parity + `largest_wrong_blob`, census from the sheet's rows, four poisons |
| P1 one composition rule | DONE, two deviations noted below | `a23d30620` (renderer bump) |
| P2 every row from parts | DONE | `a23d30620`; `mary_os_flipbooks_draw_every_row_from_parts` (Rust) |
| P3 in-engine parity | DONE | `scripts/measure_rigged_parity.py` + `crates/ambition_render/examples/rigged_sprite_parity.rs`; both facings inside A ≤ 1%, B ≤ 10 px |
| P4 interpolation | DONE | schema 2 (tracks, per-clip `tween`); `RiggedSpriteAsset::tween_into` + `CharacterAnimator::frame_phase`; renderer `tween_draws` |
| P5a body-riding effects on parts (D4) | DONE | shared impostor atlas: `actors::rigged` (`RiggedImpostorAtlas`, `ImpostorUnpremultiply`); `PortalPieceTint` removed |
| P5b baked residency | DONE | parts-only realization (`NO_BAKED_IMAGE`, `CharacterSpriteAsset::parts_only`); the reveal barrier and the binders wait on the part pages |
| P5c stop shipping the baked PNG | open | exclude a parts-only character's `_spritesheet.png` from packaging; keep it generated as the offline oracle |
| P6a player robot v3 from parts | DONE | schema 3 (placement, draw and frame opacity); continuous placement for supersampled rigs; mirror rows; gamma-space impostor; renderer `tests/test_player_robot_v3_part_flipbook.py`; in engine all 1,888 frames ≤ 0.04%, blob ≤ 1 |
| P6b the largest sheets: noether, PCA, patent clerk, the swing fighters | DONE | published from parts; `scripts/measure_published_flipbooks.py` redraws every published flipbook against its published sheet (all inside D6); in engine noether and patent clerk blob ≤ 1, PCA inside D6 after snapped parts gained their border; runtime cell classes 288 / 576 / 896 |
| P6b' the remaining rig characters: oiler, paradox_barber, data_lovelace, neil_ongras_turfson, hunny_horror_boss, companion_dog, m_leblanc, charley_beagle_svg | DONE | `publish_rig_flipbook`; every frame inside D6 offline and in engine; a continuous replay skips the frame's clipped edge band (`EDGE_BAND`) |
| P6c procedural painters and every remaining character (about 120) | DONE offline, publishing | the shape recorder (`blending_draw` reports each ink op; `text` by its pixel change), reduction on the frame's own grid or sample positions, and stacked-edge merging; config generators publish through `publish_generator_flipbook`; every target's frames hash identical before and after its seams; every frame replays inside D6 (nearly all at 0 wrong pixels) |

### Every character, largest sheets first (Jon, 2026-10-03)

Jon's goal since 2026-10-03: every character drawn from parts by default,
faithfully, the largest sheets first. The order is by published sheet bytes,
measured 2026-10-03. The polygon sizes in a plain directory listing include
orphaned `.1`–`.5` pages from August; the live sheets are page 0 alone.

| Character | Sheet | How it is drawn | State |
| --- | --- | --- | --- |
| noether | 48 MB, 7 pages, 875 frames at 496 × 528 | `RigDocument`, supersample 2 at render scale 2; a breathing blurred "hum" behind every frame | recorded: all 875 frames replay with 0 wrong pixels. The hum is painted at 1/8 resolution and drawn enlarged (`composite_scaled_layer`, a draw's `scale`): against the full-resolution hum no pixel moves more than 16 levels. One 5.1 MB part page at 1/4 (smaller at 1/8). Takes a 576 px cell (cell classes) |
| perfect_cellular_automaton | 26 MB, 7 pages, 913 frames at 654 × 846 | `RigDocument` at frame resolution (snapped, parts turned bilinear: `rotation_filter`); effects through `compose_rig_frame` | recorded: all 913 frames replay exactly; one 399 KB part page. Takes an 896 px cell |
| player_robot_v3 | 9.1 MB, 1,888 frames at 256 × 256 | `RigDocument`, supersample 4 | DONE, below |
| pointed / pugnacious / projectile polygon, carl_stargan, director, officer, performer, medic | 3.7 to 5.8 MB each (live pages) | `RigDocument`, supersample 2 to 4; authored strike effects (`swing_effects.composite_authored_effect`), drawn from the frames before them, so a clip renders at once and is cached | published: `composite_authored_effect` goes through the seams and each clip is recorded whole from the UNCACHED clip function (`render_clip`). All frames redraw their sheets at most 0.09%, blob 3 (`scripts/measure_published_flipbooks.py`). carl_stargan 564 KB of parts for 4.3 MB, director 161 KB for 4.4 MB, officer 185 KB for 4.0 MB |
| niels_boar | 3.5 MB | procedural `ImageDraw` body, squashed and rotated as one raster | recorded (P6c): the turned or squashed body rides as one picture, the rest as shapes; 290 frames, worst blob 1 |
| patent_clerk | 4.8 MB, 875 frames at 224 × 224 | `RigDocument`, supersample 4; effects through `compose_rig_frame` | recorded: all frames replay (≤ 0.02%, blob ≤ 1); one 341 KB part page |
| flying_spaghetti_monster_boss | 7 MB, 79 frames | procedural `ImageDraw`; tentacles are splines that deform every frame | recorded (P6c): reduced 4.6x, so each shape is sampled where the frame's resize samples it; 79 frames at 0 wrong pixels, 0.22 of full-frame texels; the sauce reveal map stays a data sheet |

⛔ **A turned snapped part needs a transparent border too.** A part turned
by `blit_rotated` fades a pixel outward past its raster, and the GPU draws a
part only inside its rect: PCA's turned outlines lost a one-pixel edge in game
(blobs of 7–8 on ten frames). Every published part now carries `PART_BORDER`
transparent texels; a snapped part's pivot stays whole, so the replay is the
same picture. PCA then passed in engine, all 1,826 captures.

**The procedural painters (P6c, 2026-10-03).** Most characters are drawn by
`ImageDraw` from joint positions every frame: limbs are capsules between moving
joints, coats free polygons, nothing rigid to name. They are recorded shape by
shape: `blending_draw` reports every ink op to the recorder (an alpha-0 ink
drawn directly cuts its coverage out of what is under it; `text`, which can
replace pixels at any alpha, is recorded by the pixels it changed). The
painter's last steps go through rigdoc's seams (`downsampled_canvas`,
`composite_canvas`, `composite_layer`); a layer painted some other way (turned,
blurred, rescaled) rides as one picture. Every converted target was hashed
before and after: the frames are byte-identical.

Three rules made the replay exact rather than close:

* ⛔ **Reduce a shape where the frame reduces it.** A shape reduced from its own
  corner lands between frame pixels and is resampled. Padded so its corner is
  on the supersample grid, it reduces as that region of the frame and lands on
  whole pixels. A frame resized by a factor that is not whole (the FSM's 4.6x,
  a pirate's per-frame fit, stochastic_parrot_v2's 1.32x enlargement) samples
  each shape over the frame pixels it reaches with PIL's `box`: the same sample
  positions as the frame's own resize.
* ⛔ **Two edges reduced apart do not stack as one.** A boot and its sole share
  an edge; each half covered, stacked they are three quarters covered
  (vera_ruin's sole at alpha 200 where the render has 65). Each raster is
  checked where it lands against the canvas reduced whole; where they differ
  by more than 32 levels it is merged with the rasters before it within the
  filter's reach, one paint-order run at a time.
* A track is named once a frame: a second layer of one name is numbered.

The cost is honest, not small: a shape that only translates is reused, one
that changes is a new part, and merging folds a body into fewer, larger parts
(three AI-era bodies turned as one layer are a single picture a frame).
Published part pages run 0.1 to 0.8 of the full frames' texels, typically
under their trimmed sheets; puppy_slug (1.6) and ninja_shadow_oni_leader
(1.23) cost more than their sheets.

⛔ **Recorded shape by shape, a procedural painter is NOT a part character
(census, 2026-10-03).** Faithful, yes; cheap, no: a limb drawn already turned
into the frame is a new part at every angle, so 71 of 143 flipbooks held more
texels than their own baked sheet or took over 64 draws a frame (the goblins
140). Two answers, both landed:

* **The road is a measured verdict.** `part_flipbook.realization_by_cost`
  decides at publish: parts unless the packed part pages hold more texels than
  the sheet's pages or a frame takes more than 64 draws. The flipbook records it
  (`realize: parts | baked`); `rigged_pages_in` honours it. The capability and
  the flipbook stay; only the road changes. Census and ledger:
  `scripts/measure_part_flipbook_cost.py`,
  `dev/ambition_dev_measurements/part_flipbook_cost.jsonl`.
* **The old painters are rebuilt as rigs** (Jon: exact pixels not required for
  the old characters, fix how they are constructed). `authoring/shape_rig.py`:
  a piece is painted once in its own frame and placed by
  `rigdoc.blit_rotated`, so the sheet is composed from pieces and the flipbook
  stores each once. Goblins 631 parts / 140 draws -> 27 / 22; the shadow oni
  leader 2307 / 139 -> 34 / 26; girdle 3489 / 59 -> 293 / 28; the toons ~300 ->
  ~25 parts at 0.02-0.4x their sheets. Every rebuilt character replays its new
  frames inside D6 and passes the cost rule (fsm_noodling excepted).

**After the rebuild (census, 2026-10-03, renderer `7bd8024`):** 142 of the 143
published flipbooks draw from parts and only fsm_noodling is drawn baked. Their
part pages hold 58.0 MTexel against the sheets' 532.1 (0.109x; 26.2 against
262.7 MiB), and the tiers come to about 0.11x. The median character's worst
frame takes 21 draws, and the worst character (georg_canter) 50. Every
flipbook redraws its sheet inside D6 (`scripts/measure_published_flipbooks.py`).
In-engine parity covered all 142 part-drawn characters in both facings
(45,516 frames, `scripts/measure_rigged_parity.py`). It found one game defect:
the impostor ignored a mirror row's mirrored feet anchor, so the player robot
drew 3 to 5 px off in all 944 of its left-facing frames. That is fixed, and the
robot's 3,776 frames now pass. One residual is explained and not gated:
ninja_shadow_duelist's headband tail, a 1 px diagonal line placed at a
quarter pixel. The GPU samples it between texels, which costs about 30% of
its alpha against PIL's rounding: 14 frames, 0.07%, blob 12. ⚠ The draw tables are now the larger download: 42.4 MiB of
RON against 26.2 MiB of part pages.

Also: always-rigid neighbouring draws are composited into one part when that
adds no texels (`_merge_rigid_neighbours`; noether -9% draws), and an impostor
atlas renders only on a frame where one of its cells changed.

⛔ **A flipbook is the SHEET's, keyed by the sheet's name.** A generator's sheets
record the generator as their `target` (`robot_archivist`'s is "robot"); looked
up by target, one sheet was handed another's flipbook and the hall panicked.
`CharacterSheetSpec::base_sheet_key()` is the key everywhere a flipbook is found.

⛔ **A tier table is derived, never recorded again.** Recorded at a tier's scale,
a shape recording merged differently and 71 tier tables named another part
count, which the game refuses. A tier render (`quality_tier_render()`) publishes
no flipbook; `build_parts_variant` derives the table from the full one. Census:
`every_published_flipbook_realizes_its_sheet_at_every_tier`.

⛔ **The pirates meet D6 now.** Their old flipbook transformed parts drawn at one
scale and measured 5.5% and a blob of 69 against frames each fitted by its own
LANCZOS scale. Recorded through `sheet_build.downsample`'s seams, each shape is
resized by its frame's own fit: every frame inside D6, at 1.00 to 1.07 of the
sheet's texels (a shape scaled per frame is reused by no other frame).

⛔ **The harness put frames half a pixel off.** It rounded a feet-anchored
root's FEET to a whole pixel; a sheet whose anchor is off its pixel grid
(paradox_barber's feet are half a pixel off it) then drew every frame half a
pixel off, and the GPU resampled the whole frame (a faint extra row under the
shoes, blob 8). The example now lands the frame's top left on whole pixels.

⛔ **Three sheets state two different feet.** director, officer and medic
publish a `feet_pixel` 8 to 26 px from the point their `feet_anchor_norm`
names (officer's `feet_pixel` lies below its 171 px frame; found 2026-10-03).
The game places a body by the anchor, and a flipbook's draws are relative to
its own `feet_pixel`, so both roads put every frame pixel in the same place;
only a reader that mixes the two is wrong. The parity harness did (every frame
of the three, baked and parts alike, 10 px low) and now places its oracle by
the anchor. The sheets' metadata disagreement itself is open.

The in-engine harness also accepts a frame inside D6 of the game's OWN baked
draw when it is outside D6 of the crisp published-draw oracle, and lists every
such frame: the two roads a player sees are then the same picture. director's
`punch`[1] is the case: parts and baked draws identical (0 wrong pixels), both
softer than the oracle (a blob of 8).

### Small part pages (Jon, 2026-10-04)

Jon's rule: a distinct raster on a part page is art no other raster gives by a
ZERO-COST transform. A mirror or a quarter turn is one source with a transform
on its draw. A colour variant is one sheet with an engine colour shift. An
affine or projective warp resamples pixels, so it is a judgement per case, not
a rule.

* **Census of the waste:** `scripts/measure_part_waste.py` (ledger kind
  `part_flipbook_waste`). Baseline, 47.9 MTexel of parts:
  * effect layers recorded whole each frame: 33.7%
  * shapes painted already posed: 29.0%
  * exact duplicates: 0.1%

  Jon's "identical art" is near-identical art: faces that differ only by
  expression, and limbs that differ by a pixel of length.
* **Lossless sharing:** `part_flipbook._share_transformed_parts`. A part equal
  to a mirror, quarter turn or transpose of another is drawn as that part with
  `M = R S L` split into a turn and a signed scale. It is replay-guarded. A
  tweened track that changed part is never made one part, because a flip
  would interpolate through zero.
* **Faces:** `_toon_rig.face` / `overlay_piece` give a base head plus eye and
  mouth overlays. 16 toon configs and Trent have 64-76% smaller part pages
  (Sybil 91,580 -> 25,536 texels).
* **Effects:** `FxCanvas(pieces=True)` paints each primitive once at full
  alpha and places it with its alpha as the draw's opacity, within a per-frame
  draw budget. `FxCanvas.place` places authored glyphs. This is opt-in
  because it was measured per character:
  * Carl Stargan: -28%.
  * Perfect Cellular Automaton: -6%.
  * Noether: +17%.

  Their effects change shape every frame (growing radii, moving points), and
  an effect raster that is another at a different alpha is 0-7% of them. They
  need their effects redesigned as glyphs, not a pipeline pass.
* **Colour shift:** `CharacterColorShift` (hue, saturation, value) on a
  part-drawn body is applied per impostor cell by
  `impostor_unpremultiply.wgsl`, after compositing, in sRGB space. The HSV
  math was checked against Python's colorsys. It is for enemy variants and
  buffs; the baked road cannot turn a hue. Next: a catalog field naming a
  variant as a sheet plus a shift, so variants share one sheet (the three
  heavy pirates, the goblins).
* **Near twins and symmetric halves:** `_share_near_parts` draws a part that
  is another a texel over or a shade darker as that part (with a draw `tint`).
  `_split_symmetric_parts` stores a part that is its own mirror as one half
  drawn twice. Both only nominate; the replay keeps or withdraws each
  candidate on its own and prints what it withdrew. Where art must stay
  different, list its tracks with `part_flipbook.keep_distinct(target, ...)`.
* **A squash is a draw scale:** `bone.<b>.scale_y` reaches `blit_rotated(scale_y=)`,
  which squashes about the whole-pixel pivot row (`rigdoc.squashed_sprite`,
  quantized to the table's 4 places). A continuous flipbook stores the
  unsquashed raster with the squash on the draw. A snapped one bakes it.
  Robot v3 had stored 13 heads and 30 torsos, one per squash value.
* **A raster and its mirror reduce as mirrors** (`_reduce_part`). The 4x
  raster is reduced in whichever orientation has the lower digest, so a part
  the rig draws turned round is the same part mirrored. Before, robot v3's
  `air_back` head, face and antennas were stored twice, a texel apart.
  Robot v3 went from 165 parts / 161,509 part texels to 73 / 53,515.

### Player robot v3 from parts (2026-10-03)

The second character, and the first that is supersampled, mirrored and faded.
What it added, all of it general:

- **Continuous placement.** A supersampled rig paints every part at 4x and
  reduces the frame once, so no part lands on a whole frame pixel.
  `rigdoc.downsampled_canvas` (the reduction, now a seam) hands each recorded
  part a raster reduced on its own, by the same filter, placed where
  `blit_rotated` put it on the 4x grid, divided by 4 (unrounded, a part was up
  to a quarter pixel off: a blob of 7). Each reduced part keeps 2 transparent
  texels round it (`PART_BORDER`): trimmed to its alpha box, its half-covered
  edge row was smeared outward by the resampler (alpha 82 for 10). The
  flipbook says `placement: Continuous` (schema 3).
- **Mirrored rows** (`~mirrored`, the robot's other side) are recorded through
  `rigdoc.mirrored_canvas`: the same parts, `scale.x = -1`, turned the other
  way. The runtime puts the draw's scale in the slot's transform.
- **A draw's opacity** (the smash blade fading out, the swap sets'
  cross-fades) is published per draw and drawn as the slot sprite's alpha.
- **A frame's opacity** (the death fade) fades the frame AS ONE PICTURE
  (`rigdoc.faded_canvas`). The runtime fades the body's cell after its parts
  are composited, in the un-premultiplying pass (`ImpostorCellOpacity`).
  Faded part by part, the torso would show through the arm.
- **The blink is the engine's (2026-10-09).** A row named `blink_out` or
  `blink_in` (and its mirror) has a teleport warp (`BodyWarp::of_row`). The
  un-premultiplying pass cuts the composited body into vertical slivers that
  slide apart, rise and fade, or come together (`warped` in
  `impostor_unpremultiply.wgsl`; the numbers are the ones of the baked
  `teleport_body` overlay). The sheet draws a plain pose in those rows, plus
  its portal-ring and sliver pieces. A warp row is composited as one image, as
  a row that fades is. Player robot v3 is the first sheet with plain rows.
  `capture_scene --body-warp out|in[@SECONDS]` shows the warp on each
  part-drawn body.
  - Before (2026-10-04) the body's own pieces each drifted and faded
    (`robot_side._teleport_warp`). The parts are few and they overlap, so the
    blink read as a fade.
  - The warp is for a row that draws the body whole
    (`RiggedSpriteAsset::row_draws_the_body_whole`): no frame fades as one
    picture, and no part the `idle` row draws fades on its own. A sheet says
    so with its own draws; there is no flag and no list. Measured on the published
    tables, 2026-10-09 (the test prints the two lists): 17 have a blink row.
    Six draw the body whole there and get the engine's blink:
    `player_robot_v3`, `player_robot_v2`, `robot`, `goblin`,
    `goblin_shaman_staff` and `performer` (the two older robots since their
    generator's blink rows are plain poses too, `robot_side.py`; the last
    three were not looked at in a capture). Eleven fade or take apart their
    own body and keep their own blink, as before: `alice`, `bob`, `director`,
    `medic`, `ninja_shadow_duelist`, `ninja_shadow_oni_leader`, `officer`,
    `perfect_cellular_automaton`, `pointed_polygon`, `projectile_polygon` and
    `pugnacious_polygon`. Each gets the engine's when it is published again
    with plain rows.
  - The baked fallback frame of player robot v3 (drawn only when a body fits
    no cell, or its pages are not ready) is the plain pose with its portal
    pieces.
- Every other effect (jets, shield quarters, line blade, orb, beam) is a piece.
  Effects went from 475k to 32k texels, and the body is unchanged.
- `IMPOSTOR_CELL` is 288 (the robot's 256 px frame plus margins);
  `every_published_flipbook_fits_an_impostor_cell` holds every published
  flipbook to it, with the margin its draws need: the cell covers each
  published frame, each in-between of a tweened clip, and a body whose
  `PartPose` reaches past it draws directly. The parity oracle is still
  clipped to the cell (queue row
  [RIG-IMPOSTOR-CONTAINMENT](../queue.md#rig-impostor-containment--a-part-drawn-body-is-drawn-whole-or-refused)).

The replay guard of a continuous flipbook forgives 64 levels and NO place
(`CONTINUOUS_REPLAY_TOLERANCE`): the usual pixel of slack forgave the robot's
head drawn a pixel off. Measured over all 1,888 frames: worst parity 0.15%,
largest blob 6 (two frames, `ledge_getup`[3] and its mirror), at D6's bound. A
visible part a pixel off makes a blob of 15 to 174.

In engine (`scripts/measure_rigged_parity.py --target player_robot_v3
--centre-anchored`, llvmpipe): every one of the 1,888 frames within 0.04% and a
blob of 1. That needed one more fix, general to every character:

- ⛔ **The impostor blends in gamma space.** The baked frame is composited from
  stored sRGB values (PIL); the impostor's sRGB target blended the parts in
  linear light, so every anti-aliased outline over another part came out
  lighter (the robot's dark outline drew 102 where the frame has 1; 25 rows
  failed, blobs to 68). Part pages were then read raw (a part-page loader,
  the harness included) into a plain `Rgba8Unorm` target, and the
  un-premultiplying pass decoded once. Mary-O improved too: at most 0.14% and a
  blob of 6 in both facings and both anchors (it was a blob of 9), so the
  harness bound is D6's 6 again. (Superseded twice on 2026-10-05: first the
  atlas cameras blended in gamma space directly, and the raw loader was
  removed; then one law for both roads, the world's linear light
  (`rendering::impostor_compositing`), with part pages ordinary sRGB images and
  the parity gate scoring against a runtime-law oracle. See
  `semantic-part-rendering-and-ragdolls.md`.)

Size: one 549 KB part page (905,216 packed texels, 360 parts) against the 9.1
MB sheet. The draw table is 3.8 MB of RON (38,418 draws), baked into the build.
It took 60 ms to parse, and every realization paid it again, each tier twice:
a tier table carried a copy of every draw (3 MB a tier). Now a flipbook is
parsed once per process (`RiggedSpriteAsset::baked`), and a tier table carries
its parts alone (30 KB; `generate_visual_quality_variants.py`). A compact
encoding of the draws is a follow-up.

Deviations from the recommendations, both reaching the same end:

- **D2, placement.** The baked render keeps its whole-pixel placement. Instead,
  the rig flipbook publishes that same placement (`placement == "snapped"`), so
  baked frames did not change for every rig-document character. The oracle and
  the parts follow one rule. `build_rig_flipbook` refuses any frame its draws do
  not redraw to within one level of compositing rounding. Measured: 0 wrong
  pixels on all 87 frames.
- **D1, recolor.** The recolor happens at the SVG source (one derived file per
  palette), not per part raster. The old frame-level recolor never reached
  rotated thin features: resampling moved the sleeve stripes off the exact
  palette values, so they stayed red. Baked transition frames changed by up to
  3.46%, and that change is the fix.

What P3 measured (llvmpipe, texel per pixel, 2026-10-02):

- **The game draws the published draws.** Mary-O's parts drawn by Bevy match
  the published draws, drawn the baked road's way and unclipped, at worst 0.19%
  of pixels and a 9-pixel blob, in both facings. The gate's blob bound is 10, not
  6: the GPU turns a part by bilinear sampling and PIL by bicubic, so a turned
  outline can differ along a one-pixel line. Dropping any visible draw makes a
  blob of 12 or more (618 draws measured). A cut 6×6 hole fails the gate (blob
  36, 0.38%).
- **The game's BAKED draw is the less faithful one.** Drawn by Bevy, the baked
  frame is 0.9–4.3% off its own published frame, with blobs up to 109, on every
  frame. Cause not measured; trim and anchor rounding giving a half-pixel
  bilinear blur is the suspect. This affects every character drawn baked. Filed
  as a finding, not fixed here.
- **The baked frame cuts off art that the parts draw.** On 28 of 87 frames
  (mostly the shrink clips, plus skid, crouch-walk and death), feet or the cap
  run 2–8 px past the 160×192 frame. The baked sheet loses those pixels; the
  parts keep them. This is the renderer's existing "DRAWING RUNS OFF THE
  LOGICAL FRAME" defect. Parts fix it. To make baked match, the frame would
  need overscan.
- Three draws are entirely covered (a sparkle layer, a hidden leg). Culling them
  at publish is a small saving.

What P4 measured:

- Mary-O's `walk`, `crouch_walk`, `climb` and `swim` publish `tween: Linear`.
  Everything else steps (D3).
- Every tweened part, at t = 1/4, 1/2 and 3/4 of every tweened frame, sits
  within 0.80 px and 0.00000° of where the renderer places it in the lerped
  pose. The 0.80 px is the keyframes' whole-pixel rounding.
- In-engine, a slot half-way through a walk frame sits exactly half-way between
  its two keyframe places (`a_tweened_clip_draws_between_its_frames`). A runtime
  that does not lerp fails it.
- `measure_rigged_parity.py --phase T` draws in-betweens through the GPU and
  reports them without gating them. PIL rounds a tweened part to whole pixels
  and the GPU does not, so their raster difference (1–2%) measures rounding,
  not the tween.

What P5a built and measured:

- **One quad per body, drawn from parts.** The parts are drawn by a private
  camera into a cell of one shared impostor atlas. The ROOT draws its cell,
  named as an atlas frame of the atlas layout, at its own size, feet, tint and
  flip. The portal compositor, the hit flash and Mary-O's star-power overlay
  all read the root's image and atlas frame, so they draw the parts' frame —
  tweened in-betweens included — and none reads the baked sheet. The root is no
  longer drawn at zero alpha, and `PortalPieceTint` (its only reason) is gone.
- **Two cameras for every body, not two per body.** A camera pair per body cost
  about 1.4 ms per actor on llvmpipe; the shared atlas costs one pass. The atlas
  grows a step when full (1, 2, 4, 6 cells per side). At 6×6 from the start, the
  per-frame clear and shade of the whole target cost a single body 13 ms on
  llvmpipe; grown, one body adds 1.5 ms, 10 add 8 ms, 30 add 15 ms (llvmpipe; a
  hardware GPU is the open measurement). Sprite batches: 2 for any number of
  bodies (was 2 per body).
- **A class is never full (2026-10-03).** When the last page of a cell class is
  full at its most size, the class opens one more page: another atlas of that
  size, with its own targets and cameras, to the right of the last. Before, a
  body that found its class full kept its baked sheet. A parts-only body (P5b)
  has no baked page, so it drew `NO_BAKED_IMAGE`, which is nothing: the 37th
  small body of a room was invisible. Witness:
  `a_class_with_every_cell_taken_opens_a_page` (37 parts-only raiders, two
  pages, each body draws its own cell, and each page renders for its own cells
  only). Measured in `hall_of_characters` with every published flipbook
  realized from parts (the publish of renderer `dff162f`, before the cost
  verdict): 79 bodies of the first class had no cell and drew nothing; with
  pages, none, and the first class holds its 115 bodies in 4 pages. Not
  measured: the count at the shipped cost verdict, and what four pages cost a
  frame.
- **The second camera un-premultiplies.** A sprite drawn over a transparent
  clear stores premultiplied colour. Mary-O measured no difference without the
  division (her only partial alpha is a one-pixel dark outline), but a
  character with soft translucent effects would darken. The quad divides it out
  (`impostor_unpremultiply.wgsl`). A poisoned shader fails the in-engine gate on
  every frame (blob 522–882).
- In-engine parity through the impostor is unchanged: worst 0.20%, blob 9, both
  facings. The cell is the frame plus 16 px of margin, so the 2–8 px the baked
  frame cuts off are drawn.
- A frame larger than a cell (288 px with margins since 2026-10-03; 256 before), or a full 36-cell atlas,
  keeps its baked sheet and warns once.

⛔ P5a placed every centre-anchored body half a body too high. That is every
player with a sheet-authored quad, Mary-O included: `character_render_basis`
builds them at `Anchor::CENTER`, with the translation at the quad's centre. The
impostor quad assumed the root's anchor is its feet. The tests and the harness
built only feet-anchored roots, so nothing could see it (Jon saw it in the game,
2026-10-03).

Fixed by deriving the cell quad from the root's own basis (`cell_quad`), as the
baked frame is derived. Guards:

- `the_impostor_lands_where_the_baked_frame_would_for_either_anchor` asks the
  game's builder for both conventions. The old formula fails it at the player's
  feet: 0 vs −57 world units.
- `measure_rigged_parity.py --centre-anchored` builds the root as a player is
  built and mirrors the oracle about the root, not the feet. The old formula
  reads 97.5% wrong; fixed, both conventions are inside the bounds.

What P5b changed:

- With the rigged sprites admitted, a sheet whose every row is a part clip is
  realized from its part pages alone. Its baked pages are never requested, so
  they are never decoded or resident. The page slots keep their atlas layouts
  and hold `NO_BAKED_IMAGE`, an id nothing loads or draws. This applies to
  Mary-O's three forms and the five pirates.
- The decode reads `CharacterSpriteAssets::parts_admitted`, which the character
  runtime mirrors from `RiggedSpriteAdmission` before the materializer runs.
- The actor and prop binders wait on `CharacterSpriteAsset::presentation_images`
  (the part pages, for a parts-only realization). The room reveal manifests the
  part pages for such a character.
- Witness: in the drawn demo, `mary_o_is_realized_from_her_parts_with_no_baked_page_requested`.
  It includes a control (a baked page the room does request is found by the same
  lookup). A decode forced back to baked fails it, naming
  `sprites/mary_o_v2_spritesheet.png`.
- Resident texels (reasoned from the published pages): Mary-O's three forms hold
  865,937 part texels plus one atlas cell (shared; 256² when measured, 288² since the robot), against
  1,601,774 baked. Only the demanded forms are resident: the short form alone is
  179,200 against 318,166.

Size after P2: packed part pages are 0.541 of the three sheets' texels
(865,937 / 1,601,774). Fire `transform`'s aura overlays dominate.

Jon asked on 2026-10-02 for every Mary-O sprite and animation to be part-based.
That makes Mary-O the customer that two of the parent's non-goals were waiting
for. For Mary-O only, this plan lifts "replacing all baked sheets" and "general
skeletal interpolation before a customer needs it".

## Goal and acceptance

- Every row of all three forms (`mary_o_v2`, `_tall`, `_fire`: 31 rows, 87
  frames) is drawn from parts. `baked_clips` is empty.
- At every published frame time, a player cannot tell the part-drawn body from
  the baked one. The only accepted differences are from aliasing and sampling.
  The bounds are under [Validation](#validation-build-it-first).
- Clips authored to tween interpolate smoothly between keyframes.
- The baked sheet is no longer resident for Mary-O, and later no longer shipped.
  Resident texture bytes go below today's baked bytes.

## What is true now (measured 2026-10-02)

Scripts are in the renderer repo:
`scripts/measure_mary_o_parts_{census,reconstruction,tween}.py`. Each takes
about 7 seconds.

- Mary-O's `RigDocument` paints every body pixel with `rigdoc.blit_rotated`.
  Recording those blits and drawing them again reproduces every non-effect row
  exactly: 0.00% against the raw render, in all 31 rows. This includes the
  front-view `death`, which draws 7 or 8 parts from the front SVG. It also
  includes `grow`, which alternates the short and tall documents, 12 parts.
- No blit uses opacity below 1.00, on any frame.
- Only five rows need more than parts. The figures are the worst parts-only
  frame, with the standard metric (threshold 64, radius 1):

  | Row | Worst | What is missing |
  | --- | --- | --- |
  | short `shrink` | 2.48% | power-loss sparkles, outfit stars, palette recolor |
  | short `big_shrink` | 17.12% | sparkles, sleeve wings, stars, recolor |
  | tall `shrink` | 6.76% | sparkles, sleeve wings, stars, recolor |
  | fire `fireball` | 7.10% | fire orb |
  | fire `transform` | 37.08% | transform aura, stars, recolor, sleeve wings, orb |

- **Parts, plus each part recolored, plus each effect as an overlay layer,
  reproduce every frame of every row.** The worst frame is 1.01% (standard
  metric). The leftover is anti-aliased edge pixels: `_recolor` remaps exact
  colours on the finished frame, so it misses edge pixels that the overlapping
  parts blended.
- The published walk flipbook disagrees with the baked sheet by up to 1.74%. A
  `death` flipbook would disagree by up to 2.56% (fire). This comes from
  `PartFlipbook.recompose`, not from the sheet: the published sheet equals the
  raw render (0.00%). `blit_rotated` rounds the pivot and the world position to
  whole pixels and rotates in a padded canvas. The flipbook places parts at
  fractional positions and resamples them bicubically.
- **A single missing effect layer costs only 1.43% (median frame).** That is
  under today's 2.5% bound. The aggregate metric cannot tell a missing star or
  orb from anti-aliasing noise.

## The gaps, by mechanism

| Mechanism | Frames | Fix | Format change |
| --- | --- | --- | --- |
| Front-view death | 3 | add the row | none |
| Form alternation (`grow`) | 4 | add the row | none |
| Palette recolor, a whole-frame post-process in `mary_o_v2_svg_poc._recolor` | 16 | recolor each part raster (D1) | none (option A) |
| Effects drawn behind the rig (`transform_aura`, power-loss sparkles) | 29 | one overlay part per layer, drawn before the body | none (the pirates' `_overlay_raster` already does this) |
| Effects drawn in front (sleeve wings, outfit stars, fire orb) | 22 | one overlay part per layer, drawn after the body | none |
| Pixel snapping in `blit_rotated` | every rotated or half-pixel part | one placement rule for oracle and runtime (D2) | none |

The effects are drawn at logical resolution and upscaled with NEAREST. As
overlays they must land on whole pixels at runtime, so they stay crisp.

## Asset size (measured, texels before atlas packing)

| Realization | Texels | Share of baked |
| --- | --- | --- |
| Baked sheets, three forms (643 KB PNG) | 1,601,774 | 100% |
| All rows from parts, recolor variants baked (option A): 147 part rasters (379,328) + 52 effect overlays (246,424) | 625,752 | 39% |
| All rows from parts, palette applied at runtime (option B): 53 part rasters (128,974) + overlays | 375,398 | 23% |

Packing adds overhead. The pirates saved 38% after packing. Effect overlays are
two thirds of option B. Turning the aura and sparkles into reusable sprites
would need per-draw opacity and scale, so it is a later step. The ultrapack
holds Mary-O frames that no character load reads; that weight can be dropped
now.

The saving appears only when the baked sheet stops being resident. Today a
rigged body holds both (1.62x for the admiral).

## Interpolation (measured)

- Mary-O's rig has one level. Each limb is one rigid shape that rotates about
  its joint, and the torso only translates (`body_lean` shifts `body_x`). So
  this simple tween:
  - lerp each part's pivot position
  - lerp its angle by the shortest path

  matches exactly what the renderer draws for the lerped `Pose`. It gave 0.00%
  on all 37 in-betweens of the looping rows (`walk`, `crouch_walk`, `climb`,
  `swim`, swings up to 130°). The same check without the angle lerp gives
  5–7%, so the check can fail.
- **Format:** each draw needs a stable track id (its rig part name), and each
  clip needs an authored tween policy (`step` or `linear`). A track that changes
  raster, appears, disappears or changes draw order steps at that boundary. The
  parent's rule against choosing by runtime heuristic applies, so the policy is
  published data. This is a schema bump to `PART_FLIPBOOK_SCHEMA_VERSION` 2.
- **Runtime:**
  - The phase is `animator.elapsed / duration`, which exists.
  - Slaved clips zero `elapsed`, and their `clip_phase` is private. It must be
    exposed.
  - Slots must be keyed by track. Today slot `i` takes draw `i`
    (`actors/rigged.rs:331-345`).
  - The plug-in point is `drive_rigged_presentations`
    (`actors/rigged.rs:318-321`).
- **Oracle for in-betweens:** the renderer draws the lerped pose. The gate
  compares the tween at t = 0.25, 0.5 and 0.75 against it.
- The walk changes only by translation (0° swings). A smooth walk changes its
  three-frame snap. Which clips tween is D3.

## Validation (build it first)

1. **Offline data parity** (renderer pytest). Check every frame of every row of
   every form: the published flipbook recomposed, against the published baked
   frame.
   - **A:** wrong-pixel fraction ≤ 1.0%, standard metric.
   - **B:** largest connected wrong blob ≤ 6 px (proposed). B exists because A
     passes a whole missing effect layer.
   - **Census:** the gate asserts 87 frames and 31 rows checked, so an empty or
     narrowed corpus cannot print ok.
   - **Poisons, each must fail at its subject:** drop one overlay; move one part
     1 px; swap the draw order of two overlapping parts; skip the recolor.
2. **In-engine parity** (Bevy, headless capture). Draw the same body baked and
   from parts, at every row and frame (pinned with `ActorAnimOverride`, not
   keys), at the game zoom and 1:1, facing both ways, at every quality tier.
   Apply metrics A and B. This catches GPU filtering, half-pixel placement,
   flip, draw order and tint. No such comparison exists today
   (`rigged_sprite_trial.rs` checks presence, not pixels). GPU rotation sampling
   against PIL is **not measured**; this is the biggest unknown.
3. **Effects that ride the body:** hit flash, portal pieces, the quasar
   star-power overlay, stance squash, split screen. Compare each drawn over
   parts with the same effect over baked.
4. **Human review:** a page or GIF per row with baked | parts | diff heat map,
   and each tween at 4x slow motion. Jon's eye is the final acceptance.

## What still reads the baked image at runtime (survey 2026-10-02)

Nothing reads character pixels on the CPU. Body geometry comes from the sheet
RON, compiled in by `ambition_sprite_sheet/build.rs`, so it survives without the
PNG.

| Consumer | Reads | Location |
| --- | --- | --- |
| Hit flash and blink (and its portal pieces) | root image + atlas rect | `rendering/hit_flash.rs:213-235, 720-735` |
| Portal compositor (root is the only candidate) | root image + rect, `PortalPieceTint` | `ambition_portal2d_presentation/src/far_side.rs:217-247` |
| Mary-O quasar star overlay | root image + frame | `game/ambition_demo_mary_o/src/quasar_shader.rs:246-273` |
| Generic `SpriteEffect`, deep-dream overlay (not on Mary-O today) | host sprite image | `crates/ambition_sprite_fx/src/lib.rs` |
| Loader: no baked PNG means no `CharacterSpriteAsset`, so no parts | file existence | `character_sprites/assets.rs:491-499` |
| Part page folder is derived from the baked texture path | path | `character_sprites/rigged.rs:94-95` |
| Room-transition barrier waits for baked pages only | readiness | `room_transition_assets.rs:257-298` |
| `ov1_draws_the_world.rs` asserts the baked PNG exists | test | `:263-277` |
| Dev placeholder toggle swaps every sprite image, slots included | image | `actors/overlays.rs:199-262` |

Interpolated frames make the first three wrong even while the baked sheet is
resident. Their silhouette is the baked keyframe, not the tween.

## Phases

| # | Work | Acceptance |
| --- | --- | --- |
| P0 | Validation 1 over the current hybrid, with census and poisons. Commit the measurement scripts. | Gate red on each poison, green on walk |
| P1 | One composition rule in the renderer: per-part recolor (D1) and continuous placement (D2) in the baked render too, so the oracle and the parts use the same maths | Jon reviews the sub-pixel change (≤ 2.6%); goldens re-baselined |
| P2 | The flipbook covers every row: the recorder captures recolored parts and effect overlays in draw order; `PART_ROWS` is deleted | Validation 1 green at A and B on 87 frames; a texel-saving floor test |
| P3 | Validation 2 harness; fix what it finds (sampling, half pixel, flip) | Mary-O app suite green with all rows from parts |
| P4 | Interpolation: track ids and per-clip tween policy (schema 2), keyed slots, phase, in-between oracle | Tween gate green; review GIF accepted |
| P5 | Retire the baked sheet for Mary-O: move hit flash, portal and quasar off the root image (D4), fix the loader gates, add part pages to the room-transition manifest, stop loading the PNG, then stop shipping it. The renderer keeps generating it as the offline oracle. | Resident bytes below baked; Validations 2 and 3 green |
| P6 | Other characters: pirates (already all parts) through P4/P5, then the other rig-document characters | Per character |

## Decisions for Jon

- **D1 — recolor per part in the baked render too.** Recommended. The oracle
  and the parts then agree exactly. Otherwise a ≤ 1.01% edge residual stays on
  16 frames.
- **D2 — continuous placement in the baked render too.** Recommended. Tweens
  need continuous placement anyway. The baked frames change by up to 2.6%,
  sub-pixel.
- **D3 — which clips tween.** Proposed: locomotion loops tween; transitions and
  one-frame rows step.
- **D4 — how body-riding effects see a part-drawn body.**
  - (a) A per-body offscreen composite. Parts render to one texture per drawn
    body, which becomes the single quad for portal, hit flash and quasar.
  - (b) Reimplement each effect per slot.

  (a) keeps the portal's one-quad contract and generalizes to every character.
  Its cost is one small render target per visible rigged body. Reasoned, not
  measured.
- **D5 — palette at runtime (option B).** Later, and only if 16% more saving
  matters. It needs a palette material on every slot.
- **D6 — the bounds.** Proposed: A ≤ 1.0%, B ≤ 6 px, at frame times and
  in-betweens.

## Risks

- GPU sampling of rotated parts is not measured (Validation 2 decides).
- Effect overlays dominate the size.
- Tier tables come from cropping and downsampling parts. Effects must use
  NEAREST, and every tier needs Validation 2.
- Tweens change the feel, not gameplay. Hurtboxes and body metrics stay
  simulation facts.
