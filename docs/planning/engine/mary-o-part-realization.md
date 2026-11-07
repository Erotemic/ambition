# Mary-O drawn entirely from parts

**State:** MOSTLY LANDED; packaging and target-GPU checks remain. Parent:
[`runtime-rigged-sprite-animation.md`](runtime-rigged-sprite-animation.md).
Jon accepted the recommendations below (D1–D6) on 2026-10-02. Mary-O goes first;
every rig-document character follows the same road.

## Current state and remaining work

The October 2026 conversion has completed the offline parity and published
part-flipbook program for Mary-O, the robot, rig-document characters and the
procedural painters. The historical per-character size tables, run progress
and transient before/after measurements live in Git history, not in this
active owner plan. Current authority is the published assets and the
renderer/engine test suites, not the October census.

| Phase | State | Durable evidence or remaining action |
| --- | --- | --- |
| P0 offline parity | DONE | Renderer `tests/test_mary_o_part_flipbook.py`, full row census and poisons |
| P1 composition rule | DONE | Renderer `a23d30620`; snapped parts need transparent `PART_BORDER` |
| P2 all Mary-O rows from parts | DONE | `mary_os_flipbooks_draw_every_row_from_parts` |
| P3 engine parity | DONE | `scripts/measure/measure_rigged_parity.py`; test at every relevant quality tier |
| P4 authored interpolation | DONE | schema 2 track IDs and per-clip `tween`; `RiggedSpriteAsset::tween_into` |
| P5a part-based body effects | DONE | shared impostor atlas; effects must follow the whole body |
| P5b runtime baked residency | DONE | `NO_BAKED_IMAGE` / `parts_only`; room cover waits for part pages |
| **P5c baked-file packaging** | **OPEN** | Exclude parts-only `_spritesheet.png` from shipped packages, but keep the offline oracle |
| P6a robot v3 | DONE | schema 3 placement/draw/opacity; 1,888-frame parity gate |
| P6b rigs and smaller characters | DONE | `publish_rig_flipbook`; check `measure_published_flipbooks.py` |
| **P6c procedural painters** | **PUBLISHED OFFLINE; VERIFY TARGET** | Recorder + `publish_generator_flipbook`; confirm final packaged coverage and measured CPU/GPU draw cost before declaring all characters production-complete |

**Standing rules:** record painter operations at the same supersample/grid as
the frame; merge overlapping reduced edges before publication; keep stable
track IDs and authored tween policy. Pick parts or baked realization by a
measured cost rule (`part_flipbook.realization_by_cost`), not by the desire to
use parts everywhere. No baked payload is required for headless simulation.

**Unfinished validation:** a suitable hardware-GPU view of rotated and
interpolated parts, their effects and room transitions; the current offline
parity gate alone is not that check. Re-test at the target tier before
retiring baked assets from distribution.

**Unresolved compatibility observations from the original parity run:**

- The published `feet_pixel` and `feet_anchor_norm` disagreed by 8–26 px on
  director, officer and medic. The parity harness was fixed to place by the
  anchor, but the *metadata disagreement* was not fixed. Check current sheets
  and readers before choosing whether to normalize the asset data.
- The baked render path showed half-pixel blur and 2–8 px clipping on some
  Mary-O frames that parts drew correctly. These are only a remaining product
  concern if a shipped character still uses the baked fallback. Verify the
  customer before selecting baked-path overscan/placement work.

## Goal and acceptance

- Every row of all three forms (`mary_o_v2`, `_tall`, `_fire`: 31 rows, 87
  frames) is drawn from parts. `baked_clips` is empty.
- At every published frame time, a player cannot tell the part-drawn body from
  the baked one. The only accepted differences are from aliasing and sampling.
  The bounds are under [Validation](#validation-contract).
- Clips authored to tween interpolate smoothly between keyframes.
- The baked sheet is no longer resident for Mary-O, and later no longer shipped.
  Resident texture bytes go below today's baked bytes.

## October 2 part audit (historical input)

The original Mary-O census found 31 rows and 87 frames across three forms.
Every body frame could be reconstructed from part blits; shrink/fire/transform
rows also needed recolor and ordered effects, and edge sampling differences
had to be addressed. That experiment led to the completed parity and schema
work in P0–P4 above. Use the published flipbooks and renderer parity tests for
the current measurements, not the October 2 percent-mismatch tables.

## Composition rules that survived the audit

- The front-facing death, grow/form alternation and every effect-bearing row
  are covered by the published part and overlay road (P2).
- Palette recolor, effects behind/in front of the rig, draw order and
  sub-pixel placement use the shared publish/runtime rule. Effect overlays
  must stay on whole logical pixels when the art requires NEAREST sampling.
- Keep the poison tests for missing effects, displaced parts, order and
  recolor; an aggregate pixel-percentage comparison alone can miss a small
  but important visual layer.

## Asset and distribution contract

Parts-only publication avoids retaining baked textures at runtime (P5b).
**P5c remains open:** shipped products must omit obsolete baked
`_spritesheet.png` files for parts-only characters while preserving the
renderer-side baked oracle. Compare packed distributions and GPU residency,
not the October 2 unconstrained texel estimates. Runtime palette materials
are optional and must be justified by measured savings.

## Interpolation contract

Use stable per-part track IDs and the clip's authored `step`/`linear`
policy (schema 2). Linear pose interpolation places and rotates parts using
the same rule as the renderer oracle. A track that changes raster, presence
or draw order steps at the boundary. The corresponding P4 implementation and
in-between parity were completed; target-GPU review of rotated/tweened parts
remains outstanding.

## Validation contract

The P0–P4 validation harnesses have landed. These checks remain regression
requirements, and their target-GPU execution is an unfinished acceptance step.


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
   flip, draw order and tint. The P3 parity harness landed; repeat its visual
   and effect checks on the actual target GPU before treating offline parity
   as hardware acceptance.
3. **Effects that ride the body:** hit flash, portal pieces, the quasar
   star-power overlay, stance squash, split screen. Compare each drawn over
   parts with the same effect over baked.
4. **Human review:** a page or GIF per row with baked | parts | diff heat map,
   and each tween at 4x slow motion. Jon's eye is the final acceptance.

## Baked-image consumer audit (historical)

The October 2 survey located baked-root dependencies in hit flash, portal
compositing, quasar overlays, effects, sprite loading and room readiness.
P5a/P5b replaced the runtime dependency through the part-based shared
impostor/loader road. Do not treat the old consumer list as a current defect
list. Recheck shipped packages for P5c and target-GPU parity for P6c.

## Original delivery phase map (historical)

The current DONE/OPEN state is recorded at the top. This table names the
original acceptance boundaries, not additional work to re-execute; P5c and
target-device P6c validation remain the live residuals.


| # | Work | Acceptance |
| --- | --- | --- |
| P0 | Validation 1 over the current hybrid, with census and poisons. Commit the measurement scripts. | Gate red on each poison, green on walk |
| P1 | One composition rule in the renderer: per-part recolor (D1) and continuous placement (D2) in the baked render too, so the oracle and the parts use the same maths | Jon reviews the sub-pixel change (≤ 2.6%); goldens re-baselined |
| P2 | The flipbook covers every row: the recorder captures recolored parts and effect overlays in draw order; `PART_ROWS` is deleted | Validation 1 green at A and B on 87 frames; a texel-saving floor test |
| P3 | Validation 2 harness; fix what it finds (sampling, half pixel, flip) | Mary-O app suite green with all rows from parts |
| P4 | Interpolation: track ids and per-clip tween policy (schema 2), keyed slots, phase, in-between oracle | Tween gate green; review GIF accepted |
| P5 | Retire the baked sheet for Mary-O: move hit flash, portal and quasar off the root image (D4), fix the loader gates, add part pages to the room-transition manifest, stop loading the PNG, then stop shipping it. The renderer keeps generating it as the offline oracle. | Resident bytes below baked; Validations 2 and 3 green |
| P6 | Other characters: pirates (already all parts) through P4/P5, then the other rig-document characters | Per character |

## Accepted design decisions (2026-10-02)

These were accepted when the part-realization approach was selected. They are
not outstanding questions unless a measured regression reopens them.


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

  The shared body-impostor road (P5a) landed. Reassess the cost on the target
  GPU; the October 2 per-body render-target estimate was not a measurement.
- **D5 — palette at runtime (option B).** Later, and only if 16% more saving
  matters. It needs a palette material on every slot.
- **D6 — the bounds.** Proposed: A ≤ 1.0%, B ≤ 6 px, at frame times and
  in-betweens.

## Risks

- GPU sampling and effect parity still require final acceptance on target hardware.
- Effect overlays dominate the size.
- Tier tables come from cropping and downsampling parts. Effects must use
  NEAREST, and every tier needs Validation 2.
- Tweens change the feel, not gameplay. Hurtboxes and body metrics stay
  simulation facts.
