# Character bone-rig conversion

Convert the procedural PIL characters into **simple bone rigs** so animations
become editable on bones (and improvable across a character's own clips),
sourced from a minimal SVG paper-doll assembled by the rig. Companion to
[svg-component-character-migration.md](svg-component-character-migration.md)
(the parts-discovery + fidelity-verifier work that produces the minimal SVG).

## Direction (Jon, 2026-07-24)

- **The cast must NOT feel the same.** Do **not** reuse an existing rig (e.g.
  Oiler's) across characters. Expect *many* distinct bone rigs and animation
  setups; unify later, if ever.
- **Port, don't redesign.** Each character gets its **own** rig that reproduces
  **today's sprites and animations**. Match the current look/motion; this is not
  a redesign like Oiler.
- **Variations only where they map onto animations we are already porting.** No
  new motion/variation systems yet.
- **Check-in gate (standing):** don't commit an SVG until it is *minimal* and the
  animation *assembles* from it + its paper-doll parts. Scene SVGs stay in
  gitignored `tmp/` (regenerable).

## Why the pirate first

The roster is ~91–95 / 122 HumanoidBiped, but per the direction each is its own
rig. The pirate is the ideal first exemplar because it is **already a bone-posed
paper doll in disguise**:

- `targets/characters/_pirate_common.py::animation_pose(anim, i, n)` returns a
  dict of **parametric bone angles** (`root_x`, `bob`, `body_tilt`, `left_leg`,
  `right_leg`, `left_arm`, `right_arm`, `weapon`, `head_tilt`, `hat_tilt`,
  `left/right_foot_lift`, `coat_sway`, …) — one set of formulas per animation.
- `paint_character()` places named rigid parts (`torso`, `coat_tail_*`, `boot`,
  `sword`, `hat`, `face`, `chest_skull`) via `draw.part(name, origin, angle)`
  scopes, and draws the articulated limbs (legs/arms/neck) as lines.

So the pose function ≈ clip channels, and the `draw.part` scopes ≈ paper-doll
parts. The pirate's kinematics are its **own** (not shared humanoid FK): a joint
is `parent + rot(offset, child_world_angle)` — sockets ride the tilted pelvis
while limbs swing in world space. Forcing it into Oiler's relative-FK skeleton
would lean the legs with the body and change the look — so it is modelled as-is.

## Increments

### 1. Extract the explicit skeleton — DONE (`744d9f2`)

`targets/characters/_pirate_rig.py`: 15 bones declared parent-first, each with a
local `offset` and a world `angle` (the pirate convention). `evaluate(pose, kind,
w, h, tilt)` returns `{bone: BonePose(point, angle)}`. `paint_character` now
reads every joint from `evaluate()` instead of recomputing `transform(...)`
inline. **Raster byte-identical** across pirate_raider + pirate_admiral, all six
animations at real frame counts. `tests/test_pirate_rig.py` pins structure, the
socket-rides-tilt + legs-swing-in-world conventions (poison-tested), and a
golden walk pose.

### 2. Minimal SVG paper-doll assembled by the rig — DONE (`e4c615b`)

`_pirate_common.build_scene(kind)` already captures the pirate as a **minimal**
`ComponentScene`: 21–22 deduped rigid paper-doll parts (`torso`, `coat_tail_*`,
`boot`, `sword`, `hat`, `face` + blink/mouth/x-eye variants, `chest_skull`)
placed per frame by the explicit skeleton's joints (via the `draw.part` scopes),
plus the posed limb/neck strokes. No dangling refs.

**Reproduction verified near-perfect.** Rasterizing each `frame_doc` (resvg) and
comparing to the authoritative PIL raster in supersampled paint space with the
alpha-aware symmetric metric (`equivalence_harness._frame_defects`): **median
occupancy ~0.047 across all 5 roles × all 38 frames**, visually indistinguishable
(see `tmp/pirate-rig/proof_raider_pil_vs_svg.png`). The solid-geometry floor is a
uniform ~0.045–0.05 — pure resvg-vs-Pillow stroke-edge AA (sub-2px fringe), not
lost geometry. The only frames above the floor are the six **slash** frames
(~0.075–0.09), and *entirely* because of the translucent swoosh arc: suppressing
just that effect drops slash occupancy back to the ~0.045 floor. That divergence
is the accepted translucent-compositing class (same as mockingbird's glow), not a
reproduction defect. `tests/test_pirate_svg_fidelity.py` guards the envelope and
pins that only slash exceeds the solid floor (so a real dropped part — which
lands occupancy in the 0.2+ range — fails hard).

The articulated limbs stay procedural strokes posed on the same skeleton joints
(making them rigid `<use>` parts would change the stroke look — out of scope
under "reproduce today's sprites exactly").

### 3. The rig is the assembly driver

`build_scene → paint_character → _pirate_rig.evaluate` — every part placement and
limb comes from the skeleton, and the pose channels come from `animation_pose`
(the pirate's parametric clip layer, kept in its own convention, **not** routed
through `humanoid_svg_rig`). Scene SVGs stay regenerable/gitignored per the
campaign rule.

### 4. Adversarial pass + honest fidelity — DONE (`56d6994`)

An independent adversarial review (which re-proved byte-identity across all 5
kinds) drove a fix pass. Outcomes worth carrying forward:

- **Honest claim:** the six **slash** frames do NOT pass the strict
  `_frame_verified` (0.07) gate — entirely the translucent swoosh effect (nulling
  the arc drops slash to the ~0.045 solid floor). That is the accepted
  translucent-compositing divergence class, not lost geometry. Every other frame
  and all solid geometry reproduce at the floor. The docs/tests state this
  plainly rather than implying full raster-equivalence.
- **Drop guard is structural**, not threshold-based: every core part must be
  registered AND placed (its `<use>` referenced by a posed frame). Occupancy is
  the *mislocation* guard; the structural check is the *drop* guard (occupancy
  margins were razor-thin for small parts like a single boot).
- **⚠ Gotcha — the death-branch alpha-0 stroke is an ERASER**, not dead code:
  `blending_draw` carves on zero alpha, clearing a 1px seam below the feet on the
  death settle. It looks like a no-op; removing it shifts the parity hash. Kept
  and commented.
- Golden test pins all 15 bones + root; removed genuinely-unused imports/param.

### 5. Roll to the pirate family, then the cohort — DONE (`81736c9`, `19d9c2c`)

The 5 pirate roles share `_pirate_common`, so the rig covers the whole family by
construction. For the rest of the humanoid cohort, a key structural finding: only
the pirate family uses the clean **rotate-around-joint FK** model. Every other
procedural humanoid (vikings, cutlass_viper, colonial, ghoul, dark_lord…) uses
the **position-shift model** — a single body frame `root + rot(local, body_ang)`
where a pose channel nudges a joint's *position* (`elbow = P(base + arm*k, …)`)
rather than rotating a bone. So per Jon's call these are **faithful
extractions**: lift the joint computation into a declared `_<char>_rig.py::
evaluate(pose, …)`, byte-identical, no look change (re-rigging into true rotate
bones would shift the look and was rejected).

Ported so far (each raster byte-identical across all its animations, verified
pristine-vs-refactored):

- `viking_warrior` (`81736c9`, the worked shift-model template)
- `viking_shieldmaiden`, `viking_heavy_warrior`, `viking_heavy_shieldmaiden`,
  `colonial_statesman`, `ghoul_skulker`, `dark_lord`, `pirate_cutlass_viper`
  (`19d9c2c`) — run as parallel byte-identity-gated extractions off the template.
- `president_portrait` (`be16079`, colonial sibling)
- `pirate_heavy` (`cba7db0`, variant-parameterised — `evaluate(pose, spec, …)`)

That is 10 procedural humanoids on explicit rigs, plus the 5-strong pirate
family = 15 total. Full suite green (only the pre-existing robot_slash reds).
Each render entry differs (`_render_frame` vs per-variant `_draw_variant`); the
existing discovery/smoke tests already import and render every target, so the
ports are covered there. Per-character golden-joint tests (like the pirate's) are
a worthwhile follow-up but were not added in the batch.

**Recipe** (mechanical, delegatable): baseline-hash the module → find the
skeletal anchors in `_render_frame` (body/head frames + hips/shoulders/elbows/
hands, and inline knees/feet; leave `_draw_leg`-style helpers and geometry
polygons in paint) → declare them in a rig dataclass + `evaluate` with the
expressions copied *verbatim* → rewire `_render_frame` to read `J.*` → the raster
hash must be unchanged. Variant-parameterised characters thread their `spec` into
`evaluate`; multi-view characters get one `evaluate_*` per view.

**Skipped:** `neil_ongras_turfson` is already a declarative `RigDocument`
(`.rig.json`) — no inline joints to extract. Non-humanoid pattern-matches
(`trex_enemy`, `flying_spaghetti_monster_boss`, `smart_house`) and the
already-SVG `oiler` are out of scope.

### 6. Follow-on

The extracted rigs make each character's joints/pose editable data (the "boned"
goal). The minimal-SVG paper-doll layer (the `draw.part` seam + fidelity check,
as the pirate has) is the next step per character where wanted.

## Toolkit already in place

- `authoring/skeleton.py` — reusable FK bones + `two_bone_ik` + `Channel/Clip/Rig`
  (used where a character wants the shared convention; the pirate uses its own).
- `authoring/rigdoc.py` — `RigDocument` (bones/parts/ik/clips → rendered frames);
  the data-driven rig format (`*.rig.json`).
- `authoring/humanoid_svg_rig.py` — binds a labelled multiview SVG to the shared
  humanoid skeleton (Oiler/M.LeBlanc). **Not** used by the pirate (different feel).
- `authoring/animation_vocab.py` — shared animation *names/timings* (not motion).
