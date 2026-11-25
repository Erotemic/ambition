# Scale-relative gameplay

**State:** SR1, SR2a and SR2b DONE 2026-11-25; SR2c–d (ledge, crawler/momentum, exit assistance) and SR3 PLANNED. Selected by the
maintainer on 2026-11-21. A body made
small or large by a portal should keep its normal movement and action timing
at that scale. This changes the earlier controls-only gravity policy.
Production source reviewed: `f41218979625`. No game, build or timing run was
performed for this review.

**Execution:** [SCALE-FEEL](../queue.md#scale-feel--the-same-body-behavior-at-a-different-scale).
This page owns the capability and unit contract. The existing movement,
frame, combat, item and relation owners implement it. The
[portal delivery plan](portal-parts-similarity-and-chirality.md) still owns
P1–P6: admitted motion intervals, contact witnesses, query cost and views.
Do not create another motion kernel or portal timeline.

## Product law: the same timing in body units

For a body's ordinary actions, scaling by positive material scale `s` means
the same number of body lengths traveled in the same time. A jump keeps its
time to apex and height relative to the body's height. A timed move keeps
its duration and its travel relative to the body. The authored definition
stays canonical. The owning consumer converts its quantities once.

For equivalent scenes under constant map `x_s = b + s Q x`, the intended
comparison is:

```text
x_s(t) = b + s Q x_1(t)
v_s(t) = s Q v_1(t)
a_s(t) = s Q a_1(t)
time, input timeline and action phases stay unchanged
```

The comparison requires matching geometry, initial relative motion, fields
and gameplay contact bands. A small body in the original room has a different
relationship to its doors, wind, platforms and opponents. It can fit through
a smaller gap. Its capabilities retain their scale; identical outcomes
against unchanged external objects are not promised.

Start with positive scale in supported aligned standing frames. World
gravity selects the destination down direction. A portal's rotation or
reflection does not rotate that world direction. Righting, control mapping
and moving-frame transport retain their current owners.

### Why the current jump law differs

For canonical launch speed `j` and gravity magnitude `g`, basic ballistic
motion gives height `h = j²/(2g)` and apex time `t = j/g`. Scaling only `j`
gives height `s² h` and time `s t`. The requested result uses `s j` and `s g`:
height becomes `s h`, and time stays `t`. A tuning change cannot replace this
unit contract across all scales. Variable jump and water modifiers must
retain the same dimensionless phase rules on top of it.

## Give each quantity an owner before converting it

| Quantity | Rule when its owning body changes scale |
| --- | --- |
| Canonical length, reach, clearance, movement probe band or travel | Multiply by `s`; map direction through the quantity's declared frame. |
| Canonical speed, speed cap, velocity change or speed threshold | Multiply by `s`. |
| Canonical acceleration, constant deceleration or ordinary gravity response | Multiply by `s`. |
| Time, count, angle, restitution, coefficient, gain in `1/time` | Keep unchanged. Read the equation: a field named friction might be a deceleration instead of a coefficient. |
| Authored damage, health, resource cost, hit count or hitstop duration | Keep its gameplay policy. Geometry alone does not scale it. |
| World surface velocity, world field or world-authored force | Retain its world meaning. The target body's size does not redefine the source. |
| Already resolved frame, contact-mapped launch or carried acceleration | Consume it in its declared chart; do not apply cumulative material scale again. |
| Mass and actual force/momentum impulse | Use the explicit mechanical law below. A velocity change is not a momentum impulse. |

Use existing `BodyMaterial`, canonical tuning and checked conversions. Do
not infer material scale from the current AABB, texture size or stance.
Do not add a universal units registry or a second scaled copy of every
authored definition. Local helper types can make the input/output distinction
clear where a real consumer needs it.

## SR1 — resolve ordinary gravity and the standing experiment together

**Delivered 2026-11-25.** `BodyMaterial::gravity_response` is the narrow
derivation: finite input checked before the clamp, widened product, identity
fast path. `FrameEnv::resolve_body` applies it in both resolver arms; a fault
publishes `MotionFrame::with_refused_response` (orientation and world forces
kept, no gravity), which `step_motion` refuses as
`Numeric(InvalidFrame)` without a portal retry. `NavGraphKey` now holds the
canonical size and the body's scale-only material, so `probe_body` converts
controls exactly as the runtime kernel does. Witnesses:
`a_scaled_body_falls_and_jumps_in_its_own_units_at_the_same_times` (resolver
→ kernel free fall, the navigation probe's jump envelope, and a world-force
counterexample) and `a_response_that_overflows_is_refused_and_moves_nothing`
in `gravity/resolve.rs`. The kernel fixture is renamed
`material_scale_changes_authored_control_units_and_takes_the_frame_as_given`.

### Existing boundary

[`step_admitted_motion`](../../../crates/ambition_platformer2d_core/src/movement/kernel.rs)
already converts canonical control parameters with `for_material`, keeps an
identity-scale fast path, and restores canonical parameters after the step.
It consumes `MotionStepContext.frame` as a complete world-space value.

[`resolve_body_motion_frames`](../../../crates/ambition_platformer2d_runtime/src/gravity/resolve.rs)
selects each body's canonical gravity response. Players use active movement
tuning. Actors use their authored movement tuning or the configuration
fallback, with the existing live/dead surface factor and externally posed
or captured-body policy.
[`FrameEnv::resolve`](../../../crates/ambition_platformer2d_shared_tangle/src/frame_env.rs)
then selects the room/zone direction and composes gravity, world force zones
and carried acceleration as separate contributions. This is the useful
boundary; keep it.

### Selected change

Let `d` be world-selected down, `g` canonical response, and `r` the existing
body response factor. Resolve ordinary body gravity as:

```text
gravity_acceleration = d * checked(s * max(g * r, 0))
```

Preserve each body's existing held, captured, flight and death policy. Do
not add an actor-only factor to a player that does not have that factor.
Keep a zero response with a valid orientation. Pass the accepted material
scale to the shared response derivation, and publish one resolved frame.
Non-ECS callers use the same narrow derivation. Validate finite inputs before
the clamp; use the existing widened, checked arithmetic for the product.
Do not let `max` hide NaN or an intermediate overflow. An invalid result
joins typed admission/refusal before publication to input, combat or probe
readers, as well as before motion state is committed.

The movement kernel must **not** multiply the full `MotionFrame`. World
force zones are already world accelerations. `CarriedAcceleration` can
already contain a source conversion and a portal map. Reapplying `s` each
tick would compound that value. Held-jump, water and timed gravity modifiers
continue to affect the gravity contribution separately from external force.
The world retains zone selection and direction; the body response now gives
ordinary motion the selected scale-relative magnitude.

Retain the current root-body environment sampling in this packet. P2 owns
any later change to sampling cuts inside an accepted interval. Do not
integrate gravity independently for each spatial image of a partial body.

The separate [present material occupancy](world-geometry-and-spatial-semantics.md#present-material-occupancy)
packet corrects which material touches water/climb regions at today's sample.
It does not change that clock or make SCALE-FEEL wait for continuous region
integration. Preserve one body response and the region owner's metric meaning.

### Navigation is the same physical experiment

The [standing experiment](platformer-navigation-and-reachability.md#define-the-standing-experiment-before-caching-it)
must retain canonical body geometry, admitted material and the complete
resolved frame. Runtime and probe use the same response meaning. Navigation
does not multiply gravity itself or pre-scale tuning before the movement
kernel converts it. Its existing cardinal/aligned start restriction remains
explicit; preserving scale does not certify every transient orientation.

The current
[`material_scale_changes_authored_control_units_and_takes_the_frame_as_given`](../../../crates/ambition_platformer2d_core/src/movement/kernel/tests.rs)
fixture supplies the same explicit world frame at two scales. Equal falling
speed is correct for that low-level input, but it is not the new product
policy. Retain or rename that narrower input contract if useful. Add the
product witness through the actual response resolver; do not seed `s * g`
in a test and claim that the runtime writer was covered.

**Acceptance:** at scales `0.5`, `1` and `2`, compare nonzero freefall and
jump motion in equivalent static scenes. Normalized displacement agrees to
the stated numerical tolerance; apex/phase time stays equal. Include the
standing probe and a fixed world-force counterexample. Keep existing
canonical-parameter, timer and numeric-refusal witnesses.

## SR2 — classify contact bands and exit assistance

Geometry and control conversion alone do not give the same platformer feel.
The current code has fixed world bands that can become large relative to a
small body. Classify their purpose before converting them.

| Existing owner | Concrete quantities to resolve |
| --- | --- |
| [Collision semantics](../../../crates/ambition_platformer2d_core/src/collision_semantics.rs) and [movement collision](../../../crates/ambition_platformer2d_core/src/movement/collision.rs) | Support/landing bands: `CONTACT_SLOP=4`, `ONE_WAY_CROSSING_SLOP=8`, `EDGE_OVERLAP_SLOP=1`, including the support probe. Body affordances use body units; surface thickness remains geometry. |
| [Ledge grab](../../../crates/ambition_platformer2d_core/src/ledge_grab/runtime.rs) | Reach, clearance and latch slack; the fixed fall-snap speed `45`. Separate the body's reach from a tolerance for joining two geometric surfaces. |
| [Crawler](../../../crates/ambition_platformer2d_core/src/movement/adhesive_crawler.rs) and [momentum movement](../../../crates/ambition_platformer2d_core/src/movement/surface_momentum/mod.rs) | Capture slack, route lookahead, contact slack and speed gates. Surface tie/coincidence rules need their own numerical or geometry meaning. |
| [Movement integration](../../../crates/ambition_platformer2d_core/src/movement/integration.rs) | Surface lead and submerged-ground probes. Their length owner must be explicit. |
| [Portal placement](../../../crates/ambition_portal2d/src/placement.rs) | The current raw `min_exit_speed` floor. Ordinary exit assistance uses destination body units and exit-relative velocity. An explicit world launcher may own a world speed. |

**SR2a delivered 2026-11-25 (support and landing bands).** `ContactBands::of`
in `collision_semantics` carries `CONTACT_SLOP`, `ONE_WAY_CROSSING_SLOP` and
`EDGE_OVERLAP_SLOP` at the body's scale (identity fast path). Every kernel
support/landing predicate, the walkable-line probes, the resting-support
probe, teeter/footing/floor-held queries, the footstool and talk spacing
take the stepping body's bands. The line/block seam allowance stays geometry.
`for_material` no longer converts `teeter_margin`, `Balance::teeter_band`
or `Balance::tip_speed`: they are fractions of the body's half-width (and
half-widths per second) that `footing` already multiplies by the physical
half-span, so the conversion applied the scale twice. Witness:
`a_one_way_crossing_band_is_the_bodys_own_at_every_scale` (kernel tests).
Open gap: the fighter brain's shadow rollout keeps the canonical edge sliver
because perception publishes no material scale.

**SR2b delivered 2026-11-25 (ledge reach).** `LedgeReach::of` carries the
reach band (up 28, down 30, across 10), the roll overshoot (36) and the
auto-snap fall speed (45) at the body's scale. The probe, the getup arc and
the fall snap take the body's reach; the abilities ledge query passes its
body's. `LEDGE_LATCH_TOL` and `LEDGE_JOIN_TOL` match one geometric surface
against another and stay geometry. Witness:
`a_ledge_reach_is_the_bodys_own_at_every_scale` (ledge_grab tests).

Do not multiply every epsilon. Gameplay slack, authored world geometry and
floating-point uncertainty are different quantities. Numerical predicates
need representability bounds at the accepted geometry/scale. Keep the
supported scale range finite. A new test matrix is not a substitute for
choosing these owners.

Preserve the existing moving-frame velocity law in
[`Sim2::velocity`](../../../crates/ambition_platformer2d_core/src/frame/similarity.rs):

```text
v_exit = u_exit + lambda * Q * (v_entry - u_entry)
```

Each `u` is the surface velocity at the corresponding contact point and
time, including rotation. A platform's actual world velocity does not
become `s` times that velocity because a small rider stands on it. Resolve
body-relative motion and transport through the accepted contact witness.
P3 already owns removal of static contact-map reconstructions; reuse that
work. Preserve existing guard and projectile relative-velocity response.

**Acceptance:** one support/ledge case near its admission boundary and one
crawler or momentum customer use scaled body and geometry at unchanged
time. Include a moving platform whose world velocity stays unchanged when
only its rider scales. Check exit assistance in destination body units.
Reuse owner tests; do not add a broad Cartesian product of scales and rooms.

## SR3 — finish actions, free movers and body relations

These are concrete consumers of the same contract. Deliver small changes
through their existing owners. A completed SR1 does not close this packet.

| Existing source | Selected conversion and retained meaning |
| --- | --- |
| [Moveset events and starts](../../../crates/ambition_combat/src/moveset/mod.rs) | Timed/start `Impulse` currently maps orientation through `AccelerationFrame` without material scale. These are velocity operations. Convert the authored delta or target once, and share that helper across start/cancel/timed paths. Preserve add/replace/component-retention modes; do not multiply preexisting velocity. |
| [Hit reaction](../../../crates/ambition_combat/src/hit_reaction.rs) and [response law](../../../crates/ambition_platformer2d_core/src/hit_response.rs) | Victim-owned feel speeds and speed thresholds use victim units. Convert the hitstun reference speed too, so a uniform scale does not change `launch/reference` and thus duration. Dimensionless ratios and time limits stay unchanged. |
| [Melee contact](../../../crates/ambition_combat/src/hitbox/mod.rs) | Source `LaunchSpeed` is already mapped through the actual contact into the victim chart. Do not multiply it by victim scale again. Classify autolink `max_speed` with its source/anchor meaning; retain dimensionless gains. |
| [Projectile body](../../../crates/ambition_platformer2d_shared_tangle/src/projectile/body.rs) and [projectile integration](../../../crates/ambition_projectile_combat/src/systems.rs) | Convert the projectile's canonical gravity response once. Preserve lifetime and one age/force advance per declared interval. A portal leg loop must not advance a whole tick again. Keep mapped carried acceleration. |
| [Projectile splashes](../../../crates/ambition_projectile_combat/src/systems.rs) and [damage-box spawn](../../../crates/ambition_combat/src/strike.rs) | Raw `splash_half_extent` currently reaches a world-anchored damage box. A material-owned splash derives its size from the projectile's actual impact image and contact map. Root material alone can miss another map during partial transit. An explicitly world-sized effect remains possible. |
| [Free held-item motion](../../../crates/ambition_held_items/src/motion.rs) | Convert canonical `GROUND_ITEM_GRAVITY` with the item's material. Keep its existing scaled rest threshold and dimensionless bounce/slide factors. The projectile collision owner separately retains its scaled minimum rebound speed. |
| [Mount seat placement](../../../crates/ambition_mount/src/lib.rs) | The current seat combines an unscaled mount-local point with the rider's physical half-height. Map the mount-owned point once, then combine the rider's already physical clearance in the shared frame. Do not give rendering another pose authority. |

Small-to-small combat in equivalent conditions should retain normalized
launch and hitstun duration. Small-to-large combat is a different comparison:
source-owned launch and victim-owned response keep their declared meanings.
Damage, resources, hit identity and hit count do not multiply with scale or
the number of portal images.

The [held-item release](../../../crates/ambition_held_items/src/release.rs)
path already records a holder material baseline and later applies only its
delta to the item's independent material. Preserve it. A pickup must not
normalize a separately scaled item to its holder. A mount/capture relation
that carries another body through a scale change needs the same meaning:
retain the relative material basis, local anchor and one physical pose
authority. It does not require sharing the whole held-item implementation.
Release velocity and staged launch remain with the relation owner.

**Acceptance:** extend actual owner witnesses for a timed self-motion event,
a material-scaled projectile/impact, hitstun normalization and a mount or
capture transition. Keep the existing independent holder/item-scale fixture.
An integrated crossing, continued action, reverse crossing and rewind can
extend [portal_similarity_slice](../../../game/ambition_app/tests/portal_similarity_slice.rs)
and [portal_scale_lab](../../../game/ambition_app/tests/portal_scale_lab.rs).
Do not describe their existing geometry/contact checks as timing evidence.

## Mass and world forces remain explicit choices

[`HeldForce`](../../../crates/ambition_abilities/src/extension.rs) actually
uses `force / mass`, while moveset impulses are velocity changes. The
[force port](../../../crates/ambition_combat_port/src/force.rs) declares
world force/well units. Keep authored `Mass` unchanged in this delivery,
as portal transit currently does. Do not infer a 2D or 3D density law from
the requested movement feel.

If a future mechanic specifies mass factor `mu(s)`, the similarity condition
for an equivalent force experiment is `F_s = mu(s) * s * Q * F` and momentum
impulse `J_s = mu(s) * s * Q * J`. A velocity change instead uses `s * Q`.
This conditional equation selects no mass exponent and promises no global
energy conservation across portals.

A world-authored force still acts through its actual force/mass law. A
body-authored field can convert its source strength and geometry when it is
emitted. It is not multiplied by every recipient's size. Held-force lift
continues to remove its declared fraction of resolved gravity. Body contact
remains its existing kinematic constraint; this task adds no rigid-body
impulse solver.

## Efficiency and completion

Retain the identity-scale arithmetic and allocation path. A room without
portals can still contain a scaled body. Select work from actual material
and consumer inputs, not merely from portal presence.

Resolve the frame once at its actual sample cut. Lower transient policy
parameters once per admitted step/leg as needed. These scalar/vector
conversions must not trigger whole-room portal scans or new per-body scene
copies. Use the existing static/affine and no-intersection query paths.
Cache a derived result only after repeated work is measured and its full
definition/material/sample dependencies are known.

For offline comparison, run the same production law on matched canonical
and scaled inputs and normalize the outputs. Also check elementary nonzero
ballistic quantities so a no-motion result cannot pass an equality. A
reference experiment need not run during gameplay, and it need not become
an independently maintained solver.

Close this work only when the admitted scope is named: ordinary motion,
contact affordances and the listed action/relation consumers. Report
remaining unsupported orientations, partial-body field sampling and
explicit world-force exceptions as their actual scope. P2/P3 retain time
and contact authority; the scale contract supplies their physical units.
