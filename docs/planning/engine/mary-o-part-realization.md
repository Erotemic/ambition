# Mary-O drawn entirely from parts

**State:** IN PROGRESS. Parent:
[`runtime-rigged-sprite-animation.md`](runtime-rigged-sprite-animation.md).
Jon accepted the recommendations below (D1–D6) on 2026-10-02. Mary-O goes first;
every rig-document character follows the same road.

## Run progress

| Phase | State | Receipt |
| --- | --- | --- |
| P0 offline gate | DONE | renderer `tests/test_mary_o_part_flipbook.py`: strict parity + `largest_wrong_blob`, census from the sheet's rows, four poisons |
| P1 one composition rule | DONE, two deviations noted below | renderer `e2af3b2` |
| P2 every row from parts | DONE | renderer `e2af3b2`; `mary_os_flipbooks_draw_every_row_from_parts` (Rust) |
| P3 in-engine parity | DONE | `scripts/measure_rigged_parity.py` + `crates/ambition_render/examples/rigged_sprite_parity.rs`; both facings inside A ≤ 1%, B ≤ 10 px |
| P4 interpolation | next | |
| P5 baked retirement | | |

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
| Mary-O quasar star overlay | root image + frame | `game/ambition_demo_mary_o/src/quasar_shader.rs:228-250` |
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
