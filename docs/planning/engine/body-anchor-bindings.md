# Body anchor bindings

**Status:** Planned. Source review at `9e5e1e06` (2026-11-21); production
baseline `b91b29d3`. The arithmetic below is source-derived, not a runtime
reproduction. This page owns shared body-point evaluation and alignment.
[The queue](../queue.md) selects its delivery work.

Related owners: [semantic landmarks](runtime-rigged-sprite-animation.md),
[named art places](the-art-says-where-a-body-stands.md),
[prepared facets](capability-linking-and-prepared-facets.md),
[motion authority](unified-movement-kernel.md),
[accepted motion](portals-are-moving-frames.md), and
[persistent support](slopes-feet-ride-surfaces.md).

## The missing operation

A rider's soles meet a mount's seat. A captive's root meets a captor's jaw.
Each point belongs to a different body, with its own scale and axes. The
shared operation evaluates those two points and places one body so they
meet. It does not own riding, capture, control, custody or release.

Semantic landmarks and named seats are already selected work. Do not add
another landmark table or introduce a general relationship component. The
new step is to give both endpoints their own map, and to preserve the meaning
of a selected point when its body or prepared definition changes.

For a body point `a`, let `M(a) = t + L a`: `t` is its root position and `L`
is its accepted linear material map. The relation owner first chooses the
child's orientation, facing and material. Then alignment requires:

```text
child_root = parent_map(parent_point) - child_linear(child_point)
```

The points in this equation have already been converted from rig origin and
authored facing to canonical body coordinates. Apply each body's physical
scale, turn and parity once. Do not subtract a scaled child height in the
parent's canonical space, or pass the combined offset through the parent map.

The result is a proposed root position, not a physical permission. The existing
body pose authority admits or refuses it and publishes the accepted motion.
The binding cannot move the parent, resolve a two-body mass constraint, or
make an unsupported collision path valid.

## Current evidence and first customers

Paths below are relative to the repository root. Line ranges identify the
reviewed source, not permanent API contracts.

| Customer | Current operation | Work to remove |
|---|---|---|
| Rider soles to seat | `ambition_mount/src/lib.rs:841–863` subtracts `desired.size().y / 2` from `Mountable::seat`, then uses `saddle_world_offset` and the mount's gravity frame | The combined offset and saddle-specific point mapper (`2203–2228`) |
| Captive root to hold point | `ambition_combat/src/capture/systems.rs:2893–2915` reconstructs rig-feet origin, facing and gravity rotation around a raw attachment slot | This private point conversion and the silent root fallback |
| Held item in hand | `ambition_held_items/src/lib.rs:287–304` already uses the shared landmark material map | Preserve this as a control; do not add another hand-placement path |

The paths in this table are under `crates/`. Mount's seat contract already
says it is a point on the mount, independent of rider size (`lib.rs:121–139`).
The capture owner already refuses an absent authored hold name when it
acquires a hold (`ambition_combat/src/extension.rs:684–697`).

**Arithmetic witness.** Take a prepared seat `(0,-50)`, mount material scale
`0.5`, rider canonical height `40`, and rider material scale `1`. Use root
zero, ordinary facing and no turn. The seat is at `-25`; the rider's sole is
`20` below its root. The required root is `-45`. Current saddle arithmetic
gives `-70`. Mapping that combined offset through the parent gives `-35`.
Both fail because the endpoints have independent material maps. This is an
arithmetic counterexample; no gameplay scene was run during this review.

`BodyKinematics::size` and `material_map` distinguish these inputs in
`ambition_platformer2d_core/src/body_clusters.rs:185–190`.

## One point query, explicit bases

Extend or extract the existing `BodyLandmarks` authority. Its
`rig_point_in_world` already converts rig-feet origin and facing, then applies
physical material (`ambition_combat/src/body_landmarks.rs:260–302`). Keep the
rig/package choice behind that authority. The pure alignment operation can
live with existing body math; it must not make the core depend on character
catalogs, Bevy queries, capture or mount policy.

An endpoint can initially be a direct canonical point. A sampled endpoint
also identifies the selected semantic point and its simulation pose cut.
Both produce a point in a stated chart. A root anchor is a deliberate point,
not the fallback for a required attachment that failed to resolve.

Gravity direction, physical material parity and authored facing are distinct.
An owner can choose to align the child with gravity or with the parent; it
then evaluates the child's point with that chosen material. Facing selects
the authored pose. It must not cancel or duplicate physical reflection.

Some current hold offsets explicitly mean gravity-relative displacement
(`CapturedBy`, `capture/mod.rs:65–72`). State that basis when migrating them.
An intentional gravity-relative offset can be added in its declared basis;
it must not rotate an already material-mapped jaw point again. A part-local
offset follows the part. Do not silently change one meaning into the other.

Evaluate both endpoints at the same simulation cut and in the same admitted
chart. The pose owner resolves live rig samples before the binding reads
them (`BodyRigPoseResolved`). A named future gesture sample remains a query
of that named pose; it is not the current contact pose. Do not read finished
or interpolated renderer transforms as simulation truth.

This equation is local to a selected material chart. On a body that straddles
a portal, a named point need not belong to the root image. The existing
material/query authority selects its admitted image and path when required;
today's root-chart landmark map alone does not prove that ownership.
Keep this with the portal accepted-material contract. This plan adds no
independent traversal, image selector or global world-space shortcut. The
first slice handles direct points in a common admitted chart. A missing path
or incompatible cut is an explicit failure.

## A selected point has a lifetime

Capture currently stores `hold_attachment: Option<u16>` (`capture/mod.rs:
62–79`). At use, `pose_captives:2905–2908` indexes the current pose and falls
back to the root if the point or pose is absent. The original name is lost.
A replacement rig can assign that index to a different point.

Live rig retraction is supported. The existing
`a_character_that_stops_authoring_a_body_rig_has_it_retracted` test replaces
the prepared character and removes its rig and pose
(`ambition_platformer2d_actor_monolith/src/character_projection/tests.rs:766–797`). It does not test
the capture failure; it establishes that the definition can change in use.

Keep the semantic selector and bind its resolved slot to the prepared owner
and generation that give it meaning. Reuse the prepared-facet generation
receipt; do not create a second version registry. A generation or body-form
change invalidates that binding before it can read a new pose. The relation
owner chooses an explicit rebind to the same selector, a refusal, or release.

Distinguish: optional point absent, required point unavailable, pose not yet
resolved, body retired, and numeric map failure. `BodyLandmarks::in_world`
already separates absence from map failure. Do not turn those outcomes into
a successful root anchor. Resolve names at binding/rebinding, not by scanning
rig names on every tick. Rollback restores the relation's body references
and binding receipt with the selected generation; stale derived samples are
discarded. The owning relation still ends itself when either endpoint
occurrence retires.

## Point motion is reusable; velocity policy is not implicit

When a consumer needs point velocity, evaluate it from the same accepted
continuous trajectory as its point. For `p = t + L a`:

```text
point_velocity = root_velocity + linear_map_rate * local_point
                                + linear_map * local_point_rate
```

This includes rigid turn/scale and an authored moving point. It is a point
and its time derivative, not another body velocity authority. Requested
velocity is insufficient when motion was blocked or a point rotated around
the root. Use the motion owner's accepted path, time and cut.

A flipbook step, facing flip or discrete transfer need not have a finite
derivative at its cut. Report the discontinuity; do not invent a velocity by
dividing a pose jump by frame time. Do not require a smooth animation law
or a new integrator to ship position alignment.

Matching endpoint velocity is a separate domain choice. A saddle can keep
its current pin velocity policy; a capture can preserve its throw rules.
Release can mean inherited point velocity, a prescribed throw, or rest.
`HeldMaterialBasis::present` preserves the item's own material through the
holder's change since pickup (`ambition_held_items/src/release.rs:6–19`).
Keep that bind-time policy. `put_down:53–69` intentionally releases at rest.
Neither material inheritance nor release speed follows from two points meeting.

## Delivery packets and proof

### A1 — direct canonical points, two native owners

Use existing body math and pose admission to align mount seat/rider soles
and capture root/direct hold point. Keep each owner's explicit basis,
orientation and velocity policy. Remove both private conversion paths;
do not leave a new helper beside them. Preserve mounted size, piloting,
dismount/death and capture acquisition/hit/release decisions.

Acceptance: unequal physical scales give the two-endpoint answer; facing,
physical reflection and turned gravity each act once. A refused pose follows
the existing owner's refusal path without partial state or a second write.
Use the existing mount and capture fixtures; add no generic relation harness.

**First slice landed (2026-11-22).** The pure step lives with the body math:
`BodyKinematics::root_placing(point, at)` returns `at - L(point)`, using the
body's own linear material map.
- **Mount.** The saddle pin maps the seat through the mount's map
  (`mount::seat_in_world`: the seat mirrored by the mount's facing, then the
  mount's scale, turn and parity). It maps the rider's soles through the
  rider's map. `saddle_world_offset` and its combined `seat - rider half`
  offset are gone, and so is the pin's read of the mount's gravity frame: the
  mount's material turn carries the seat once.
- **Capture.** The hold anchor goes through `rig_point_in_world` (rig feet,
  facing, the captor's material). The declared gravity-relative hold offset
  is added in its own basis and never goes through the material again. The
  private rig-feet, facing and frame reconstruction is gone.
- **Refusal.** A map that cannot be evaluated refuses through each owner's
  existing path (`DismountReason::PoseRefused`, `release_capture`).
- **Witnesses.** `ambition_mount` `saddle_tests`: the −45 arithmetic and a
  rider at its own scale; facing mirrors only the side; reflection and facing
  each act once; a turned pair turns once. `player_pilots_mount_end_to_end`
  asks the same two maps.

Open from A1: the capture's root fallback for an absent pose (A2), and a
capture witness at unequal scales.

### A2 — sampled named points and invalidation

Move capture's named jaw/hand through the same point query. Replace its naked
slot and root fallback with a binding whose owner/generation is known.
Extend the existing authored seat work through this query, without a second
table. Check a moving jaw, slot reorder, rig removal, missing current pose,
and rollback. No case may silently select a different point or use yesterday's
pose as the current one. Admission remains `BodyPoseAdmission`, not the query.

**Capture slice landed (2026-11-22).** `CapturedBy::hold_point` is now a
`HoldPoint { slot, name }`, where `name` is a stable FNV-1a key of the
attachment name, so the relation stays `Copy` and restores identically on
every peer. It replaces the naked `u16` slot. The capture owner uses the slot
only while it still carries the name (`HoldPoint::in_rig`):
- A rig redrawn with the points in another order rebinds to the name's new
  slot.
- A rig that no longer states the name, or a captor with no rig, ends the
  hold through `release_capture`.
- With no pose of the point this tick, the captive is not written. Before,
  it was put at the captor's root.
- A map failure refuses the hold.

The rig's own slot check stands in for a generation receipt here, because it
also catches a rig replaced in use without a content reload (the T-rex test
inserts one). No codec change: `CapturedBy` is a cloned component.
Witnesses: `a_hold_point_follows_its_name_across_a_replaced_rig`, and
`a_hold_at_a_point_of_a_rig_with_no_pose_leaves_the_captive_where_it_is`
(it replaces the test that asserted the root fallback).

**Pose provenance landed (2026-11-24).** `BodyRigPose::rig` records the rig
the pose was solved from (`SolvedRig`, an `Arc` compared by identity, held so
its allocation cannot be reused while named). `resolve_body_rig_poses` solves
again when the rig was replaced, even where the clip and frame names
coincide; before, it kept the old rig's joints and points under the new rig.
`BodyRigPose::resolved_for(rig)` is the reader's question: the capture hold,
`attachment`, `attachments_from_centre` and `hurt_volumes` read no pose of a
rig the body no longer wears, so a body whose new rig selects no frame this
tick has no point rather than yesterday's. The captive is then left where it
is, as for a missing pose. Witness:
`a_replaced_rig_is_solved_again_and_an_old_pose_is_not_its_pose`.

Open from A2:
- The authored seat work through the same point query.
- A rollback witness through a rig replacement (the pose is rollback-derived:
  a restore drops it and the resolver rebuilds it from the restored rig).

### A3 — point velocity when a consumer needs it

Add derivative evaluation at a declared continuous cut when an actual release
or contact rule consumes it. Compare a turning anchor with its root velocity;
the two differ. Reuse accepted motion and retain discontinuity outcomes.
Do not change existing throw or put-down policy merely to share this operation.

## Boundaries that must remain

Keep capture separate from mount/possession, as `combat-model.md` requires.
Persistent floor, crawler and ledge support belongs to the support plan.
Petting uses a side-axis walk mark and preserves floor height (`body/pet.rs:
92–114,283–300`); point evaluation can help it without turning it into a pin.
The current grapple module is a cast and velocity impulse, not a rope solver.

Physical attachment also does not imply travel custody. `body_custody.rs`
omits capture and makes mount travel follow its rider, opposite to saddle
placement. The [endpoint audit](relationship-endpoints-and-transitions.md)
shows that its fighter-only reason for omitting capture is now stale, and
that current custody is insufficient for arbitrary departure roots. REL3
resolves native relation disposition at accepted transfer; it does not add
automatic capture custody or change A1/A2's completed geometry. Keep these
explicit domain relations. No universal parent
tree, lifetime graph, joint solver, whole-world relation pass, or automatic
inheritance of collision, support, control and material is part of this work.
