# World geometry and spatial query meaning

**State:** focused planning from source `00bf92c22142`, 2026-11-22.
Current material occupancy has two native customers below. Navigation coverage
and partial-replay publication refine their existing owners. A broad surface
or zone unification remains **DEFERRED**.

This is a source audit. No new Rust test, gameplay reproduction, or performance
measurement was run. Preserve the completed material/contact, shared spatial
index, interaction selection and navigation receipts. The new head also
contains the IR1 item recipe and AC1 construction-receipt changes; this packet
does not alter them.

Durable spatial doctrine remains in
[the spatial model](../../architecture/spatial-model.md) and
[a body in two places](portals-a-body-in-two-places.md).

## Decision: preserve the evidence that the next decision needs

Sharing an AABB or an intersection function does not make two spatial questions
equivalent. A body can touch a switch through an admitted part, face that switch
in a mapped frame, and keep its root elsewhere. A route can omit a real support
because the selected planner did not prepare it. A replay can rebuild correct
geometry for one room while erasing the published geometry of another.

These failures discard information before the next owner makes its decision.
Keep the native shape, admitted map, query cut and coverage until the consumer
has applied its own policy. Do not combine the policies into a universal
collider or region service.

| Question | Native fact to share | Meaning retained by the consumer |
| --- | --- | --- |
| Does a body occupy a water region or touch a switch now? | Accepted physical material, exact cuts and maps at the current sample | Strict overlap, region response, facing, target preference |
| Can a body stand or fly here under this navigation profile? | Native support faces and solid occupancy | Finite preparation, hazard margin, one-way traversal, route coverage |
| What geometry is published after a room replay? | The room population and admitted sample that each publication replaces | Preserve foreign rooms, retire actual old occurrences |
| Can a selected shot pass observed terrain? | The shot's finite material and native contact policy | Permitted perception, supported forecast and tactical choice |

The first three rows have bounded work below. The last row is a smaller
[REC3 customer](fighter-brain.md#ranged-action-query-follow-up), not another
spatial architecture program.

## Source evidence and existing owners

| Source | Established source fact | Owner of the change |
| --- | --- | --- |
| [Body integration](../../../crates/ambition_platformer2d_core/src/movement/mod.rs), [physical composition](../../../crates/ambition_platformer2d_world/src/collision.rs) | Both body adapters pass native uncarved physical geometry. Water membership, chest/switch touch and climbing read the admitted material parts (landed 2026-11-27). | Present material occupancy below |
| [Interaction reach](../../../crates/ambition_platformer2d_interaction/src/lib.rs) | Touch, facing and preference are shared by selection and consumption, but use root-box/root-chart facts. | [Physical touch](interaction-opportunities-and-activation.md#physical-touch-retains-its-admitted-material-map) |
| [Standing surfaces](../../../crates/ambition_platformer2d_world/src/navigation/surfaces.rs), [flight map](../../../crates/ambition_platformer2d_world/src/navigation/flight.rs) | Painted fill admits support; polygon bounds remove standing spans; flight occupancy omits chains. | [Static geometry coverage](platformer-navigation-and-reachability.md#static-geometry-coverage--native-support-before-finite-preparation) |
| [Room replay](../../../crates/ambition_hosted_world/src/room_rollback.rs), [body publication](../../../crates/ambition_platformer2d_shared_tangle/src/body/geometry.rs), [portal publication](../../../crates/ambition_portal2d/src/passages.rs) | Replay hides foreign rooms. The portal publishers replace only the replayed room and keep the hidden rooms' entries (T3, landed 2026-11-27). | [T3 publication coverage](portal-pair-identity-and-aperture.md#t3-refinement--a-publication-replaces-only-its-declared-population) |

The common law is small: a result is valid for its stated native subject,
source geometry, policy, sample and covered population. Existing typed arguments
and borrowed query results can express this. A single new context object,
global revision counter, service registry or generic cache is not required.

## Present material occupancy

### The existing sample asks about the wrong occupied set

`update_bodies` selects `ComposedRooms::physical_for` for both ordinary actor
and controlled-body integration. That composition supplies native hosts without
portal holes or glued copies. Material casts apply the portal law to the body;
carving those hosts again would give that law two owners.

At the environment sample, after any ledge carry,
`update_body_simulation_inner` calls `water_at(kinematics.aabb())` and
`climbable_at(kinematics.aabb())`. These helpers select the first native region
whose box strictly overlaps the complete root box. They do not read accepted
body parts.

Use a stationary body that straddles a supported portal pair:

- A far-side part can be inside a pool while the root box is outside it. The
  current water query misses that contact.
- A pool behind the entry can overlap the part of the root box that was cut
  away. No body material is there, but the current query admits water and can
  issue `Drowned` for a body without swimming.

The same construction applies to a physical switch. These cases need no
continuous sweep, changed motion chronology or cross-room portal. Both portal
ends remain in one live simulation room.

The existing `a_body_in_a_pair_is_in_the_water_and_on_the_ladder_in_front_of_its_far_end`
test in world collision exercises `as_met_by` with an explicit glue. It does
not exercise the current native body input above. Keep its historical scope
clear when replacing that adapter's customers.

### One native overlap operation, with a useful result

Use the accepted canonical physical body domain and current material map.
Apply the existing admission, cuts and traversal budget at the consumer's
sample. Test those occupied parts against native domain regions. Keep the
region in its source coordinates; do not first clone or back-map the world.

[Core material parts](../../../crates/ambition_platformer2d_core/src/frame/thread.rs)
already retain source cuts, the part map and its path.
`BodyGeometry::physical_body_pieces` is the ECS adapter for a body's physical
domain. Core integration must use the core material/admission seam; it must
not import the higher ECS owner. Pass current kinematics at the actual
environment cut, including ledge carry. A previously published root pose is
not automatically the same sample.

The useful overlap result retains:

- the native region or target and its live source;
- the admitted physical part and map that established overlap;
- the sample and admission under which that map is valid;
- an explicit refusal when geometry or its budget did not permit an answer.

A borrowed source reference or snapshot-local address is sufficient within
one query. Do not invent a persistent region ID for a result that never leaves
that query. A retained result must use the existing occurrence/snapshot
contract and be rebound or rechecked after restore, motion or topology edits.
Do not retain an unqualified dense link index across publications.

Preserve **strict interior overlap** for these existing region/touch callers.
A contact predicate that also accepts a shared face is not an equivalent
substitute. Existing `VolumePiece::meets` does not supply this checked strict
contract for every shape: its circle branch admits tangency, and its convex
path uses the general boolean intersection helper. Preserve that operation
for its contact customers. Add or reuse a checked strict query for the current
physical polygon-versus-box domain. Use a rotated face-only contact whose
bounds overlap, so a simple bounds rejection cannot stand in for this proof.
Numerical failure or budget refusal is not an empty occupied set. Route it
through the existing typed geometry refusal boundary; do not use a whole
root box or a false miss as silent fallback.

### A contact chooses a response; an image does not integrate a second body

Water, climbing and physical interaction use the same overlap evidence, then
apply different native rules. Keep one body's environment response and one
integration. This work does not add buoyancy, weighted fluids or independent
force integration for each image.

Water keeps its first-source-region precedence for the ordinary case. If
several parts reach one region, deduplicate that region before selection.
If several regions qualify, use their native stable order, not part traversal,
entity allocation or worker completion order. Define the selected part's
metric rule at that owner. Equivalent part enumeration or root recharting
must not change it. A new physical contact may change the selected response.

`WaterContact.submersion` is a depth measure read by audio and the harness;
it is not a fraction of material volume. Preserve the ordinary result. For a
new mapped case, state which source water-surface direction and mapped body
extent define depth. Do not silently put overlap area in the old field.
Exact membership can be established before a richer water-response model.

Climbing has an additional authority boundary.
[The collision policy](../../../crates/ambition_platformer2d_core/src/movement/collision.rs)
uses `ClimbableContact.region_aabb` to permit nonhazard blocks within the
active native climb region. A rectangle mapped back to the body's root chart
would permit the wrong native blocks. Retain source region/chart separately
from the body's response frame. Resolve supported climb direction through
the admitted map at the existing
[climb integration seam](../../../crates/ambition_platformer2d_core/src/movement/integration.rs).
Keep source-coordinate bounds and axes as truthful contact metadata.
The current integrator does not implement top-exit or alignment behavior
from those fields; this packet does not require adding those mechanics.

The first response migration may admit only its supported axis cases. If a
contact has an unsupported response orientation, report that fact separately
from geometric absence. Do not assert that a turned ladder is not touched
because a response adapter cannot yet express it. Native passability must stay
bound to the actual source region in every admitted case.
Only an admitted response enters the mechanical climb-contact/state path.
Inspection can retain an unsupported contact witness, but that witness must
not populate the old `climbable.is_some()` path and grant climb passability.

The interaction owner uses the same evidence for facing and preference;
[its follow-up](interaction-opportunities-and-activation.md#physical-touch-retains-its-admitted-material-map)
keeps target selection and consume-time revalidation together. Talk reach,
door occupancy, boarding and manual pickup retain their own declared domains.

### Keep the existing clock and the fast cases

Water and climb reads remain discrete at their current integration cut.
Preserve the zero-step/early-return rules and thin-region authoring warnings.
P2 owns later changes to samples inside an accepted motion interval.
[SCALE-FEEL](scale-relative-gameplay.md) keeps its bounded response work;
this packet does not enlarge it.

| Workload | Required execution shape |
| --- | --- |
| No relevant regions or physical-touch demand | Skip the part query |
| Uncut, aligned physical box against native boxes | Retain the direct strict-AABB path with the same result |
| Cut or rotated material | Use native exact parts only for relevant candidates |
| Several predicates over the same body at the same cut | Reuse one prepared part query where its inputs agree |
| Different read/consume cuts | Recheck current geometry; do not reuse an answer only because the tick number matches |
| Complex or refused geometry | Retain the existing bounded exact/refusal contract; no approximate whole-box authority |

Do not rebuild glued collision worlds to make environment contact work.
`glue_portal_pair` currently bounds a mapped clipped region by a new AABB
and filters ladders by mapped upright direction. Reconnecting that copy would
restore approximation and would not solve the native passability problem.
Retire it for these consumers when the native query replaces them; inspect
remaining callers before deleting the helper.

The shared bounds index can select candidates, using conservative bounds of
the admitted images. Rejecting a region against the root box before those
images are considered would reproduce the far-side miss. An empty region list
can exit early; a root-box miss alone cannot prove irrelevance for cut material.
Preparation and partition results must remain native-owner data. Add a retained
cache only for a measured repeated operation whose full validity inputs are
known.

## Bounded delivery and evidence

1. **Current water membership.** Use the two stationary straddler cases through
   the real body integration road, with an ordinary no-portal control. Reuse
   current core material admission. Both body adapters must reach that same
   owner. Observe contact and the resulting drowning/swimming decision.
   **Landed 2026-11-27.** `MotionAdmission::occupied` partitions the canonical
   body box under its material map, the admission's links, budget and first
   link. Moving links are posed at the start of the window. The result is
   each part's world polygon, or `None` when nothing cuts the body.
   `World::water_touched` takes the first region in native order that any
   part overlaps strictly. A separating-axis test treats a shared face as
   apart. Depth is measured down from that region's top to the highest point
   of the parts in it, over the root box height. One whole box part answers
   exactly as `water_at` does. An uncut body, a room with no links and a
   room with no water keep `water_at` of the step box. A refusal leaves the
   last contact in place and is reported as the tick's `motion_refusal`.
   Both adapters reach this through `update_body_simulation_inner`.
   Witness: `movement/tests/water_through_openings.rs`. It has the
   far-pool and behind-the-entry cases, a no-portal control, and a turned
   face-only touch whose bounds overlap.
2. **A second consumer.** Extend physical switch touch through the same
   material query. Keep strict overlap, mapped facing/preference, stable
   target selection and revalidation. This proves shared geometry without
   moving interaction policy into the water owner.
   **Landed 2026-11-27**
   ([receipt](interaction-opportunities-and-activation.md#physical-touch-retains-its-admitted-material-map)).
3. **Region response provenance.** Admit climbing through a supported mapped
   case. Check the native blocks made passable and climb direction.
   Include a turned case to establish the supported response or its explicit
   refusal. Retain source provenance in inspection.
   **Landed 2026-11-27, response side.** The same `occupied` call feeds water
   and climbing (`World::climbable_touched`). A region counts when a part
   overlaps it strictly through a map that keeps the region's up as the
   body's up (`to_root.direction(-Y)·-Y > 0.999`). Root-chart parts come
   first, and the first region in native order wins. A region met only
   through a map that turns or flips up is touched, but admits no climb: it
   does not enter `climbable`, so it grants no passability.

   The contact's box and metrics stay in the region's own chart, so climb
   passability is bound to the native blocks inside it. An uncut body keeps
   `climbable_at` of the step box. A refusal keeps the last contact, as water
   does.

   Witness: in `water_through_openings.rs`, the same pair under its two map
   conventions gives one map that keeps up and one that flips it.
   - Up kept: the far ladder is climbed.
   - Up flipped: it is refused.
   - Behind the entry: no ladder.
   - Root-box control: the reverse.

   Still open: inspection does not yet retain the unsupported touch
   (step 4).
4. **Owner observations and cost.** Extend the existing F1/headless observers
   with the actual query result and cut. Count part preparations, candidate
   tests and allocations in ordinary, straddling and no-region scenes. Use
   those counts before proposing persistent caching or claiming a speed gain.

The first shared query needs no complete redesign of portal connection IDs,
navigation, fluids or rendering. Existing snapshot-local native admission can
support it. T3 separately establishes correct publication across partial
replay; its two-room witness must hold before those publications are relied
on after a replay.

**Delete with the migrated callers:** root-box membership as final authority
for admitted cut material, copied environment geometry used for that purpose,
and view-side reconstructions of the new query. Preserve ordinary direct
queries where their preconditions make them exact.

## Work that remains deferred

`WorldEdgeMargins` already expresses axis roles. `BlockKind` having several
semantic axes is still insufficient reason for a new surface hierarchy.
A complete Mary-O room was authored without a combinatorial surface species.
The findings above concern information lost at specific queries, not a need
for one universal runtime zone type.

Reopen broader work only when a concrete mechanic requires crossing surface
axes, a second moving-geometry customer needs a missing kinematic contract,
several domains repeat stable query plumbing, or an external provider is
blocked by a closed reusable-engine switch. Solve that shared substrate and
preserve typed domain meaning.

## Ownership and order

- [PORTAL-SIMILARITY](../queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals)
  owns this bounded material-consumer continuation and T3. Keep P2/P3 motion
  and contact chronology separate.
- [Navigation E4](platformer-navigation-and-reachability.md#static-geometry-coverage--native-support-before-finite-preparation)
  owns native static support/occupancy and finite preparation coverage under
  EXECUTION-COST. It can proceed without the water or portal changes.
- [ISL](island-rollback-and-compact-peer-input.md#derived-publication-during-partial-replay)
  integrates T3's coverage boundary with room replay. It does not need HP1–HP5
  or cross-island live portals.
- [Interaction](interaction-opportunities-and-activation.md) owns selection and
  activation; [collision/CCD](collision-and-ccd.md) owns contact policy;
  [geometry observation](geometry-observability-and-f1.md) reads those owners.

Keep Ambition's selected product order and the existing completed receipts.
No new data-structure package, universal scene graph or global spatial
revision bus is selected by this audit.
