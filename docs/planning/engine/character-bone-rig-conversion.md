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

### 2. Animations as clips on the skeleton — NEXT

Translate each `animation_pose` branch (idle/walk/slash/taunt/hurt/death) into
declarative clip channels over the pirate skeleton (const/expr/keys, the
`rigdoc` channel vocabulary — e.g. idle `body_tilt` → `{"expr":"3*sin(tau*t)"}`).
Verify each clip reproduces `animation_pose` sample-for-sample (a numeric parity
test, env-independent), so the motion is now data on bones, editable without
touching paint code. Keep the pirate's own convention; do **not** route through
`humanoid_svg_rig`.

### 3. Minimal SVG paper-doll + assembled animation

Capture the pirate's rigid parts (the `draw.part` scopes) as a **minimal**,
deduped SVG parts library (the auto-capture discovery from the companion doc),
bind each part to its bone, and assemble frames by posing the parts with the
skeleton + clips. Verify against the current sprite with the **alpha-aware,
symmetric fidelity metric** (`equivalence_harness._frame_verified`): every frame
must reproduce today's look. The articulated limbs (legs/arms drawn as lines)
either become thin bone-bound parts or stay procedural on the same bones —
decide by what keeps fidelity.

### 4. Roll to the pirate family, then the next character

The 5 pirate roles share `_pirate_common` (palette + cohort tags), so the rig +
clips cover the whole family by construction — this is the only "variation" in
scope (it maps onto existing sprites). Then pick the next PIL humanoid and give
it its **own** rig by the same recipe.

## Toolkit already in place

- `authoring/skeleton.py` — reusable FK bones + `two_bone_ik` + `Channel/Clip/Rig`
  (used where a character wants the shared convention; the pirate uses its own).
- `authoring/rigdoc.py` — `RigDocument` (bones/parts/ik/clips → rendered frames);
  the data-driven rig format (`*.rig.json`).
- `authoring/humanoid_svg_rig.py` — binds a labelled multiview SVG to the shared
  humanoid skeleton (Oiler/M.LeBlanc). **Not** used by the pirate (different feel).
- `authoring/animation_vocab.py` — shared animation *names/timings* (not motion).
