# Portal parts: exact partitions, similarity transforms and numerical failures

**Status:** MAINTAINER-APPROVED DESIGN, IMPLEMENTATION OPEN (2026-11-06).
**Execution:** [PORTAL-SIMILARITY](../queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals).
**Work state:** THE REQUESTED PLAYABLE 80/40 SLICE IS COMPLETE (`fddf6084a`).
P01–P06 and P08–P10 are repaired. P07 remains open for the full plan.

Station H has two reachable wall openings with widths 80 and 40. Each keeps
its authored aperture. The robot crosses at half scale and keeps its physical
material after it clears the opening. Reflection mode keeps its handedness
and held facing. The return crossing restores scale and parity. A javelin
stands beside the pair. The existing F3 convention setting selects Reflection.

The real app witness checks exact far-side contact while the near-side feet
stay supported, both crossings, persistent material, GGRS rewinds and a second
bit-for-bit replay. The visible-app witness checks the robot's rendered linear
transform against its accepted physical material after crossing. Both pass.
The guards are in `portal_similarity_slice`. Core (872), portal (141), runtime
(266), actor and renderer (523) libraries pass under the workspace feature
union. The sideways-pose fixtures now set physical material; their owner tests
pass. Post-merge app portal tests (38), schema tests (4), NPC tests (2), Smash
tests and workspace policy pass. The assembled app warning check passes.
After `55d5a4292`, the assembled app check, both slice tests and all four
schema tests pass again.

The last full workspace run was not green. It reported Sanic righting
`WorkLimit` refusals without portals; its long soak was stopped. Fighter attack art and
unpublished history-art checks also fail. The capability sentinel reports the
new upstream HTN dependency. The review receipt records the scope and logs.

Full PSIM acceptance remains open: general remaining-time continuation,
complete projectile/hurt contact, including moving targets and other surviving
contact responses, other movement models, swept guard contact and response, attachments, the
bounded numeric law and fault effects, and device/GPU and performance gates. Two simultaneous limbs
and a chained third portal now have native contact and GNU-ton occupancy
guards in `2c4bd3822`. Chained damage, launch scale and replay guards pass in
`99d51bc9f`. Two active fists hit separate bodies in the same tick through
independent scaled paths in `6ae7a653a`.

Station J is live in `0f64b41bf`. Its axe crosses three pairs at quarter scale
while a foot uses another pair. The damage, refusal and replay checks pass.
The impact film shows the body and far foot. Walls or platforms back all J
mouths and both K mouths. I has a Reflection prompt at the existing wall pair.
I's held-javelin film passes two reflections in zero- and ninety-degree views
after `04a1625b1`. GNU-ton's staged simultaneous-limb GPU film passes after
`9d6f9da4f`. Full PSIM acceptance remains open.

The November 10 review corrections include projectile refusals and
dimensionless knockback in `741212079`, and proposed limb admission in
`482dcfc5b`.
Stationary material repair and support now use admitted material in
`5f07fbb17`. Independent captures and depth order are fixed in `ae8f2b07a`.
Room-scoped straddler lookup is fixed in `b499e5090`. Adaptive capture storage
and explicit material-proxy overflow are fixed in `1d551594b`. Runtime and
device performance measurements remain open. See the
[review receipt](../../../dev/reviews/portal-correctness-and-cost-review-2026-11-10.md)
for guards and verification scope.

**Existing owners:** [portals: a body in two places](portals-a-body-in-two-places.md),
[portal pair identity and aperture](portal-pair-identity-and-aperture.md),
[portals are moving frames](portals-are-moving-frames.md), and
[one transit for every mover](portals-one-transit-for-every-mover.md).

## Product rule and scope

A live portal is physically real: bodies, limbs, projectiles, attacks and appropriate
world interactions can pass through it and interact on the far side while still
partly on the near side. This feature is **not** a view-only fake or an instant
whole-body transport. Most portals are local to an `activeArea` / runtime room.
Cross-room live portals and dynamic rollback-island merging are **not** required
by this plan. Loading zones retain their separate transfer semantics;
[island rollback and input compaction](island-rollback-and-compact-peer-input.md)
are selected separately and do not block local similarity portals.

The former rule that both ends of a pair expose the *smaller* aperture was a
shortcut for the isometry implementation, not the desired gameplay policy.
Keep each endpoint's full authored aperture, and derive a similarity mapping
from the pair. A large entry and small exit scale down; the reverse scales up.
Portals may also rotate or reflect. Authored orientation conventions remain
per portal / game policy as already decided; scaling does not introduce a new
global orientation setting.

A powerful mathematician may author unusually large portals or strong scale
ratios, with their cost and constraints expressed as gameplay data. Such a
character is a customer of the general system, never a special simulation path.

## The part invariant

**One logical actor; a partition of its canonical material into N transformed
parts.** Each part occupies exactly one chart with exactly one similarity
transform. No **emitted** part may remain unpartitioned across a *traversable*
portal aperture that its geometry is eligible to cross. A part encountering
another portal is cut again; it is not left straddling that portal with an
extra render mask or a second physical body.

Let `D` be a canonical body primitive / material domain, with parts
`P_i = (D_i, T_i, path_i)`, where `D_i` is a canonical subset and `T_i` maps
canonical coordinates into the part's world chart. Required invariants:

1. **No loss, no duplicate:** the union of `D_i` is `D`, and their interiors
   are disjoint; coincident cut boundaries are counted once. The invariant
   applies per canonical primitive even when multiple *world-space* images
   overlap, scales differ or portal paths cycle.
2. **No uncut active aperture:** a final part has one side/through/beside
   classification for each eligible intersection. A piece is not required to
   avoid the infinite supporting line *outside* an aperture: that is ordinary
   solid/air geometry, not an active portal cut.
3. **One spatial authority:** geometry, visual clip region, hit/hurt volumes,
   structural effects and attachment positions are different projections of
   `D_i` and `T_i`, not separately maintained pieces and independent masks.
   A nonconvex domain may be stored as several convex cells; a circle cut by
   a plane must retain its exact circular intersection test.
4. **One gameplay owner:** health, brain, inventory, authority and logical
   attack identity remain on the actor or owned limb. Parts are derived
   per-tick geometry, not separately simulated ECS actors or extra custody.
5. **One transform per part:** the full portal path composes into `T_i`.
   Provenance/path identifiers remain available for diagnostics, deterministic
   ordering, and rollback-safe contact identity.

In the existing `frame::thread` implementation, `VolumePiece` already holds
`volume`, `bounds`, `to_piece: Iso2`, and `path`. Treat that as the *starting
point*, not proof that all consumers already satisfy the stronger invariant.
The older `ambition_portal2d::pieces::BodyPieces` one-here/optional-through
representation must not remain a parallel authority for mechanics. Audit
existing collision glue, transport, combat, presentation and structural FX
against the canonical part contract, then converge them rather than inserting
specialized exceptions for scaled limbs.

The current `PieceBudget` / worklist includes an escape path that emits a
whole piece if it cannot afford the further cut. That **must not** be used to
let an eligible moving object pass an active aperture while leaving an uncut
part. A budget exhaustion is a *refusal*: prevent the crossing at the solid
portal boundary, preserve the last valid simulation state and emit a bounded
failure reason. For an attack, the forbidden continuation cannot hit through
the opening. If the representation cannot fit even the clipped remainder,
refuse portal activation/entry or conservatively block the attempted motion;
do not discard material, duplicate contacts, or silently output an invalid
part. Define refusal at the source of movement as well as in the pure
partitioner so collision agrees with the emitted pieces.

Keep deterministic `max_hops` and `max_pieces`; add a bound for cumulative
scale. A cyclic arrangement must terminate with valid partition/refusal
semantics, not depend on a floating-point blowup to stop recursion.

## Similarity mathematics and chirality

Replace the *piece mapping* algebra `Iso2` with or extend it to `Sim2`:

```text
T(x) = s * Q * x + t
s > 0 (finite), Q^T Q = I, det(Q) in {-1,+1}, t finite
T2(T1(x)) = (s2*s1) * (Q2*Q1) * x + (t2 + s2*Q2*t1)
T^-1(y) = Q^T * (y - t) / s
```

A portal from entry `(a, half_length h_a)` to exit `(b, half_length h_b)`
uses `s = h_b / h_a` and its selected orthogonal `Q`:
`T(x) = b + s*Q*(x-a)`. Both authored lengths remain unchanged; no
`min(h_a,h_b)` is substituted for either end. The inverse portal must be
the mathematical inverse of the forward portal. Composed scale is a product,
not a new authoring value assigned to each fragment.

`sign(det(T.linear)) = sign(det(Q))` **is the piece's handedness**. It is
not the facing direction. It must be carried by the same transform used for
physics and visuals. During partial traversal, apparent part chirality is
that of the actor's canonical root transform *composed with* the portal
path. When transit rebases the actor's canonical root onto the exit, the
persistent root orientation/parity changes exactly once; the relative part
paths are rebased to preserve the same world appearance. Do not keep a
separately mutable piece-chirality bit that can disagree with the matrix.
A symmetric base character may look unchanged under reflection, but an
asymmetric attachment, handed strike, pose landmark or later skin must still
produce the correct transformed result. Turning, flipping facing and changing
chirality are distinct operations. Camera reflection is not automatically a
physical reflection of the shared world.

All geometric consumers use the same `Sim2`: points scale/rotate/translate;
free vectors rotate and scale; lengths and radii scale by `s`; polygon area
in world coordinates scales by `s*s`; unit normals transform with the
orthogonal part (or inverse-transpose followed by normalization); and
contact/constraint results must be mapped back into the owner's chart
consistently. Reflecting an oriented angular quantity must account for parity.
The invariant conserves **canonical material**, not displayed area or momentum.

For moving portals, retain the existing relative-surface-velocity law, now
using the local similarity map for the relative velocity; check velocities
at the correct entry and exit contact points. The rule for authored motion
speed, mass, force, damage, gravity, and actor scaling after completed transit
must be explicit and testable. Start with geometric/kinematic consistency:
`v_out = v_surface_exit + s*Q*(v_in-v_surface_entry)` for stationary ratio
and locally rigid moving endpoints, with no implied conservation of kinetic
energy. Destination gravity is owned by the destination world, not silently
rescaled by a visual transformation. Do not hide these choices in a rendering
adapter or infer them from sprite pixel dimensions.

## Unified partition and consumer integration

For each canonical primitive or authored articulated region, evaluate the
eligible portal aperture(s), split the canonical region at the finite face,
map the through subset with the portal similarity, and recursively process
that result. Root-to-aperture **movement** and root-to-aperture **reaching**
(a carried body versus a strike extending from its owner) retain their
separate existing admission semantics; both produce the same kind of valid
partitions. An aperture end and host depth must not become an infinite
half-plane teleport. A character too large to enter remains held by the
same collision/snag authority as before; scaling does not excuse a bad fit.

Consumers, all from the common parts rather than independent approximations:

- **Collision/transit:** evaluate each clipped transformed part against the
  correctly scoped room geometry. Return contacts/constraints in canonical
  actor coordinates, including through transformed limbs on moving hosts.
  Resolve multiple simultaneous contacts without multiplying the actor's
  mass, movement input or authority by the number of parts. Preserve portal
  cooldown and swept collision ordering for projectiles and free bodies.
- **Combat:** slice attack and hurt geometry by the same source regions.
  Hit deduplication is per logical attack and victim, even if several parts
  touch. Impact location, normal, knockback and source point are mapped
  through the specific contacting part; authored reach and damage are not
  altered just because a portal added fragments.
- **Presentation:** render each visual, sprite/rig limb, held item and
  structural effect with the *same transformed clipped region*. Masking is
  derived from the material partition, not independently guessed. Portal
  windows/captures honor non-unit scale and determinant sign, including
  near/far occlusion, camera projection, text and asymmetric detail.
- **Persistence/replay:** pieces and portal maps are deterministically
  derived from the authoritative root, portal states and authored geometry.
  No new per-frame network verdict or fixed-width input payload is needed
  to represent a scale ratio. Mechanical portal content and any runtime
  changes follow existing content/rollback admission authority.

A boss extending fists through two portals simultaneously, with one hand
then passing through a third, is the primary stress customer: the torso,
arms and hands receive separate composed transforms while one boss remains
one character. The baseline moving projectile, held item, and player body
are equally important customers. A future cross-room live-portal protocol
is excluded; do not reintroduce global rollback coupling for this plan.

## Deterministic numerical refusal and the IEEE 754 effect

Numerical validation happens **before** any transform, bounds, pose, physics,
portal placement or partition result is admitted. Refuse invalid geometry:
nonfinite input, zero or negative scale, unrepresentable inverse/composition,
underflow to a noninvertible value, overflow, or precision below the chosen
supported geometric resolution. Limits should be based on the numeric
representation and explicit gameplay policy, not accidental `NaN`
propagation. A finite ratio beyond the supported gameplay scale cap is a
*policy refusal*, not falsely reported as an IEEE floating-point overflow.

For an actual numerical scaling/reflection failure, publish one typed,
deterministic **Floating-Point Exception** presentation event for the rejected
portal operation. The portal briefly emits tiny letters spelling **IEEE 754**
and matching symbols: `NaN` for nonfinite input, `+Inf` / `-Inf` for overflow,
`+0` / `-0` and a vanishing mantissa for underflow, and small hexadecimal
bit-pattern fragments for representability/precision failures. A valid but
out-of-policy scale may instead show a separate `EXPONENT BIAS` refusal
accent; keep fault classification truthful. These are presentation effects,
not actual IEEE special values introduced into simulation state.

The effect must be rollback-safe: stable operation identity, deterministic
reason, one event per rejected operation rather than per tick, bounded
particle/glyph count, and a replay-stable presentation seed. Rewinding and
replaying must not spam duplicates or permanently create extra particles.
Place the effect at the offending aperture, not at an unrelated actor's
camera. It must clip and composite through the ordinary portal/structural
FX road wherever that road is applicable. It is not a gameplay exception,
not an engine crash, and not a reason to disable unrelated portals.

## Finite implementation slices and acceptance

1. **PSIM-1 - Pure algebra and partition truth.** Add an invertible bounded
   `Sim2` with deterministic composition, inverse and parity. Make a part a
   canonical subset plus one transform/path; implement tiling and clipping
   assertions in canonical coordinates. Audit the worklist budget fallback
   so it refuses movement instead of emitting an invalid uncut part. Keep
   `Iso2` adapters only as transitional bridges; no second authoritative
   partition. **Accept:** 0.5x, 1x, 2x, rotations, both reflection parities,
   inverse round-trips, linked chains, finite apertures, two simultaneous
   links, cycles at budget and generated tiling tests pass without loss or
   double-counted material.
2. **PSIM-2 - Paired aperture and movement.** Remove the effective-smallest
   aperture limitation; derive the directed scale from the independently
   authored endpoints. Update place/link, fits/snag, transit, moving frames,
   free-body/projectile sweeps, contact response and geometry consumers.
   **Accept:** widths 80/40 and 40/80 behave as inverse mappings; whole
   transit and partial occupancy preserve size, orientation, ownership and
   collision; oversized bodies and refused cuts do not tunnel.
3. **PSIM-3 - Combat and multi-part bodies.** Route strike/hurtbox, hit events,
   held item, articulated reach and GNU-ton multi-link behavior through the
   same canonical partitions. **Accept:** one victim hit once; scaled and
   reflected limb contacts have correct positions/knockback; two portals
   simultaneously and a chained third do not detach or duplicate a limb.
4. **PSIM-4 - Presentation and permanent handedness.** Extend folded meshes,
   portal captures, clipped sprite/rig/particle rendering, collision overlays,
   facing/reference-frame consumers and transient/rebased actor parity.
   **Accept:** an asymmetric authored test prop (and asymmetric equipped
   accessory on an otherwise symmetric player) visibly reflects correctly;
   through-side pieces scale continuously; passage through two reflections
   restores original chirality; near/far compositing and world-local
   coordinate readers agree with collision.
5. **PSIM-5 - Numerical refusal and observed seam lab.** Implement typed
   rejection and the IEEE 754 particles, update the
   [seam lab](portals-one-transit-for-every-mover.md) for scale/reflection
   cases and run representative runtime render/physics/replay witnesses.
   **Accept:** overflow, underflow, invalid/nonfinite parameters, cap
   refusal, and budget exhaustion each leave the previous admitted world
   playable; only actual numerical failures emit the IEEE fault symbols;
   deterministic playback and multiple viewports produce one bounded effect
   per fault, with no material loss or duplicated authoritative entities.

Do not treat a new generalized geometry type as completion until the real
projectile, held-item, character, boss, effect and camera clients exercise it.
Preserve the existing working rigid-portal behavior while migrating each
consumer. No blanket performance optimization without a measured regression;
profile worst-case part counts and capture cost on a target device. Existing
S4 seam/presentation follow-ups remain open until their own acceptance is met.

## Implementation plan

The five PSIM slices above define acceptance. The work packets below define
the transition through the current code. They are implementation proposals;
they do not add maintainer rulings. Finish each packet with its named
behavioral evidence and remove the old authority in the same change.

### Implementation baseline

The table describes the source inspected on 2026-11-08 at `f0e8bb16cd64`,
before the draft. The draft starts at `540cce6c186f2b2dc2c589d0aeb05bc4a06596e4`.
Check the affected owners again after the optimization work ends.

| Concern | Baseline owner and required change |
| --- | --- |
| Pair algebra | `crates/ambition_platformer2d_core/src/frame.rs`: `Iso2` has public matrix fields and uses transpose as inverse. Pair helpers take frames, which have no aperture sizes. Introduce validated `Sim2` and derive portal maps from apertures. |
| Partition | `crates/ambition_platformer2d_core/src/frame/thread.rs`: `VolumePiece` stores a mapped volume, world bounds, an `Iso2`, and a path. Keep canonical source regions as authority. Replace both the whole-piece fallback and the fallback that omits through-material. |
| Domain queries | `crates/ambition_platformer2d_shared_tangle/src/body/geometry.rs`: `pieces_from` logs refusals and returns only pieces. Return an explicit admission result to simulation callers. Keep room and body policy ownership. |
| Pair openings | `crates/ambition_portal2d/src/types.rs` and `crates/ambition_portal2d/src/link.rs`: `opening_half_length` takes the minimum through `partner_opening_half`. Remove that field and the equalizer when all consumers support the directed map. |
| Transit | `crates/ambition_portal2d/src/placement.rs` and `crates/ambition_portal2d/src/transit.rs`: the common machine maps position and velocity. `PortalBodyTransited` carries the admitted full map. Actors and loose items use `BodyKinematics` and `SweepSample`; custody removes stored items from world queries. The root now carries physical material through the admitted map. Projectiles spend the remaining tick after each accepted root crossing. General kernel and other free-body continuation remain open. |
| World collision | `crates/ambition_platformer2d_shared_tangle/src/feature_overlay.rs` and `crates/ambition_platformer2d_world/src/collision.rs`: `PortalGlue` imports far-side geometry into the entry chart. Replace independent pair clipping with projections of canonical parts and the same validated maps. |
| Body contact | `crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs`: `snapshot_body_contact` publishes a whole owner box plus carried boxes, then maps foreign boxes back through one straddled link. Use clipped parts and map each constraint to its one owner. |
| Transit consequences | `crates/ambition_platformer2d_runtime/src/portal_body.rs`: carried momentum and projectile acceleration reconstruct rigid maps from normals. Read the admitted transit map and velocity facts instead. |
| Combat | `crates/ambition_combat/src/hitbox/mod.rs` and `crates/ambition_combat/src/events.rs`: strike and hurt geometry already use parts. Preserve the contacting part through hit resolution; remove the empty-pieces-means-whole ambiguity for refused geometry. |
| Presentation | `crates/ambition_portal2d_presentation/src/carry.rs`, `crates/ambition_portal2d/src/view.rs`, and `crates/ambition_render/src/folded_mesh.rs`: reuse the shared partition. Copy factorization currently keeps rotation and reflection but omits scale. |
| Read models | `crates/ambition_sim_view/src/pose_view.rs` and `crates/ambition_sim_view/src/presented_pose.rs`: publish root scale, orientation, and chart changes from simulation. Preserve the distinction between authoritative and presented poses. |
| Alternate body pieces | `crates/ambition_portal2d/src/pieces.rs`: replace `BodyPieces` and `compute_body_pieces` consumers, including the gun visuals. Delete the alternate decomposition. Keep useful geometric helpers only under the common authority. |
| Articulated customer | `game/ambition_content_modules/src/gnu_ton/conductor.rs`: owned limbs have authored poses and use the portal extension ports. Keep those ownership semantics; remove assumptions that only one pair can carry a picture or limb. |

### Proposed state and physics rules

**Validated algebra.** Put `Sim2` in the core frame module. Keep its fields
private. Store positive scale separately from its orthogonal orientation and
translation. Handedness belongs to that orientation; derive it without taking
the determinant of a matrix whose scale can underflow. Construction,
composition, inverse, and geometric projection return typed failure. Validate
actual transformed coordinates and geometric resolution as well as scale.
Keep frame-only rigid helpers only where the operation is truly rigid.
Portal operations must consume apertures or an already admitted map.

**Canonical regions.** Each primitive has one source domain. A part holds a
source subset, a composed `Sim2`, and a stable directed path. Pull aperture
cuts back into source coordinates. Derive world geometry and bounds from that
record. Keep circles analytic; use convex cells for polygon regions. World
geometry caches may be rebuilt, but must not become independently writable
truth. Preserve double precision clipping and area calculations described in
`dev/benchmark-candidates/portal-clipping-at-world-coordinates.md`.

**Root transform.** Put persistent material scale and orthogonal orientation
with the core body state. Use `BodyKinematics::pos` as the position authority;
do not store a second root translation. Build one canonical-to-world root map
from these facts and the body's resolved pose. Keep authored body dimensions,
stance geometry, rig landmarks, and equipment offsets in their canonical
units. Derive collision size and visual size through this map. Audit writers
of `BodyKinematics::size` and `BodyBaseSize`; a crouch or morph must not erase
portal scale. Remove mutable size copies when the same fact can be derived.

`BodyMaterial` owns physical scale, room-chart axes and parity. Gravity is a
movement reference. The body map does not derive its axes from gravity.
`ActorRoll` and `SurfaceUpright` are removed. Righting emits a recorded source
turn and uses native material admission. A solid can block the turn. A numeric
or representation fault retains the previous accepted body and movement state.
Canonical facing does not reset material parity. Combat and attachment readers
still need the common material-map migration.

**Rebase at the existing root crossing.** Let `R` map canonical material into
the root chart and `P_i` map that chart into part `i`'s chart. The full part
map is `T_i = P_i * R`. For admitted crossing map `M`, set `R' = M * R` and
`P_i' = P_i * inverse(M)`. Thus `P_i' * R' = T_i`: all parts stay in their
world positions at the rebase. Change persistent scale and parity exactly
once at the root crossing, while trailing material still uses the inverse
path. Clear the latch only after the trailing material leaves. A pose-owned
limb keeps its owner's chart and does not independently rebase the boss.

Proposed defaults for the first implementation:

| Quantity | Rule |
| --- | --- |
| Geometry and reach | Scale lengths by cumulative material scale. Keep canonical reach and source material unchanged. |
| Crossing velocity | Use the approved relative-surface law, including scale. Carry the actual entry and exit samples in the transit result. |
| Controller motion | Express authored body speed and body-local acceleration in canonical units. Scale their world lengths through the root map. Keep time in simulation seconds. |
| External motion | Keep world gravity and world force fields in world units. Map acceleration carried by a projectile through the portal linear map. Do not repeatedly scale a world field because the body has more parts. |
| Mass and damage | Keep authored mass, health, and damage on the owner. Geometric scale alone changes none of them. No area-derived mass or momentum conservation is implied. |
| Knockback and constraints | Distinguish body-local authored velocity, world velocity, and physical force. Transform each through its declared frame. Do not apply a displacement map to a force by accident. |
| Normals and angular values | Normalize inverse-transpose normals. Apply parity to signed angular quantities. Do not multiply an angle or a unit direction by scale. |
| Exit-speed assist | Apply the existing minimum exit speed as explicit world-speed policy after the geometric velocity map. Test the pure map without the assist and test the assist separately. |
| Runtime aperture size changes | Treat a size edit as a discrete placement/topology operation and validate affected occupancy before admission. Continuous dilation is outside the first implementation; it would require a scale-rate term in the velocity law. |

Keep numerical support limits separate from gameplay scale limits. In PSIM-1,
define the minimum supported geometric resolution and finite composition and
inverse requirements. In PSIM-2, put cumulative gameplay scale bounds in
simulation tuning and content validation. Choose initial values with the
numeric witnesses; do not choose them from sprite dimensions. Reaching a
gameplay cap must never be reported as hardware overflow.

### Admission and scheduling

A partition query has two outcomes: a complete valid partition, or a typed
refusal with the responsible source operation, path, aperture, and reason.
An incomplete candidate is not a drawable or a collision result. Do not use
an empty vector to mean both refused geometry and an uncut whole body.

Validate a candidate at the source that can change the material or aperture:
body integration, projectile sweep, authored limb extension, placement,
runtime size edit, or a moving host. On failure, retain the last admitted
pose and complete material partition. Stop motion at the boundary, or refuse
entry before opening a carve. A refused attack continuation produces no hit
past the boundary. If an extension has no valid prior pose, refuse that
extension before it produces geometry. Do not keep only the convenient
pieces of an invalid candidate.

Use the existing domain operation owners. Do not add a general transaction
framework. Movement must validate candidate occupancy during its sweep, not
only at its endpoint. If a host carries an aperture into invalid occupancy,
its motion owner must receive the refusal before it commits the motion.
Never leave a portal behind its moving host as a silent fallback. Preserve
the existing explicit portal-close eviction policy.

Make these dependencies explicit in the current schedule:

1. Resolve candidate host poses and topology, then admit valid portal maps.
2. Publish the admitted aperture snapshot before carves and body integration.
3. Validate candidate body/limb/sweep geometry against those maps. Open a
   body's carve only for an admitted entry; constrain refused motion at its
   source. Use the same admission in projectile contact truncation.
4. Commit the root transfer and its material map together. Reconcile motion
   state from this transfer before other consumers read the new body.
5. Publish complete parts for combat and observations from settled state.
6. Presentation reads those facts on its own clock. It never retries an
   authoritative refusal with a larger visual budget.

`PortalSet::Passages` currently publishes after transit. Keep that settled
query role, but supply current admitted aperture inputs to movement before
transit. Do not solve the ordering by making movement use last tick's links.
If contact requires several candidates, sort by stable simulation identity,
source primitive, and directed path. Pair-vector indices and raw Bevy entity
order must not define rollback contact or fault identity.

### Work packets and dependencies

Run these packets in order after the static-planning hold ends. Acceptance
for PSIM-2 through PSIM-4 crosses packets; a helper or a mechanical witness
alone does not close a slice.

1. **PSIM-1A: algebra and source regions.** Replace piece-map `Iso2` with
   validated `Sim2`. Convert the worklist to canonical subsets and derive
   all world projections from them. Define typed numeric and budget refusal
   at this layer. Migrate direct callers together; do not leave a second
   partition authority. **Witness:** inverse and composition, 0.5x/1x/2x,
   arbitrary rotations, both parities, exact circle cuts, translated regions,
   simultaneous links, and source tiling with no overlapping interiors.
   **Deliberate defect:** omit scale from inverse or map a cut normal as an
   unnormalized displacement; the corresponding geometric assertion fails.
2. **PSIM-1B: refusal reaches the source.** Replace both fallback modes in
   the partitioner. Thread admission through `BodyGeometry`, movement,
   carves, sweeps, and pose-owned extension. Add stable operation identity
   and reason before effects exist. Keep the last valid complete geometry
   on rejection. **Witness:** exhaust hops, pieces, and work in real body,
   projectile, and attack paths; the body stops, a shot cannot skip a
   blocked solid, and an attack cannot hit through the refused opening.
   Include cycles and a budget too small for the clipped remainder.
   **Deliberate defect:** ignore the refusal in movement; the no-crossing
   assertion fails. This packet closes PSIM-1 only with production evidence.
3. **PSIM-2A and PSIM-4A: root state and transfer.** Introduce the core
   material transform, construction defaults, snapshot codec, and rollback
   registration. Extend the common transit result with the admitted map,
   stable pair identity, and pre/post velocity facts. Update kernel and free
   bodies, stance/morph writers, carried momentum, projectile acceleration,
   pose readers, and actor righting together. Install body state through the
   normal construction seam; install transit adapters through the portal
   plugin and runtime schedule. **Witness:** a scaled asymmetric body crosses
   and returns; root and trailing-part maps remain continuous across the
   rebase; crouch, control transfer, and two reflections preserve the right
   material scale/parity. Save/restore and rollback recover the same state.
   **Deliberate defect:** apply root parity twice at transfer; the asymmetric
   landmark and rebase assertions fail.
4. **PSIM-2B, PSIM-3A, and PSIM-4B: common consumers.** Convert collision,
   body contact, strike/hurt/guard geometry, held-item attachments, folded
   meshes, clipped sprites/rigs/particles, portal captures, and debug geometry
   to the same source regions and full maps. Remove `BodyPieces` and the
   normals-only reconstruction of physical maps. Use bounds only for broad
   phase. Keep exact geometry for contact and clipping. **Witness:** a limb
   collides on its far side; an impact keeps the contacting path for its
   location, normal, and knockback; one victim is hit once; visual and debug
   geometry match the admitted region. **Deliberate defect:** use the owner
   chart for a through-part impact; its impact-location assertion fails.
5. **PSIM-2C and PSIM-4C: unequal apertures in production.** Remove
   `partner_opening_half` and `equalize_pair_apertures`. Each endpoint exposes
   its full authored length. Derive both directions from one validated pair
   map and its inverse. Update placement, fits/snag, carve depth, cooldown,
   ray reach, capture projection, camera continuity, content readers, and
   authored literals in one coordinated change. Do not leave a compatibility
   toggle for the minimum-opening rule. **Witness:** widths 80/40 and 40/80
   exercise body, thrown item, shot, strike, moving host, and capture paths.
   Verify oversized entry refusal and obstruction at the scaled exit.
   **Deliberate defect:** restore the minimum-opening rule; the large-entry
   and directed-size assertions fail. This packet depends on packet 4;
   do not expose scaled mechanics with unit-scale visuals.
6. **PSIM-3B: articulated acceptance.** Drive two simultaneous arm links
   and a chained third link through the general engine path. Keep health,
   attack identity, and custody on the actor or existing owned limb.
   Use GNU-ton's extension ports and authored geometry as the customer.
   **Witness:** both hands can contact different targets, a chained hand
   has the product scale/parity, retracting restores its material, and two
   bosses do not share link identity or damage state. **Deliberate defect:**
   keep only one straddled link for all owned regions; the second-hand
   contact assertion fails.
7. **PSIM-5: fault presentation and complete acceptance.** Consume the typed
   refusals through ordinary semantic effects. Deduplicate by session,
   stable operation identity, and fault occurrence; a continuing rejected
   attempt is one operation until it ends or changes. Clear session-owned
   presentation state on session replacement. Replay recreates the same
   seed and bounded glyph count. Extend the seam lab and run the completed
   production matrix. **Witness:** each numerical reason has the correct
   glyphs; cap/budget refusal has no false IEEE fault; rewind/replay and two
   views produce one semantic effect per operation; unrelated portals still
   work. **Deliberate defect:** use tick alone as the operation identity;
   the continuing-fault count assertion fails.

In packet 4, remove the six-plane truncation in
`crates/ambition_portal2d_presentation/src/clip_material.rs`. The current
material drops extra planes. Generate clipped polygon meshes with source UVs
from the canonical convex cells instead of raising a fixed plane count.
Analytic circle collision remains analytic. Render tessellation is a visual
projection and cannot change physical admission. Captures of several rig
regions may share an image, but each region must use its own material subset
and transform. A union bounding box does not identify limb material.

For rays, charge each world segment against canonical remaining reach:
`canonical_distance = world_distance / cumulative_scale`. Map the remaining
reach as well as direction after each portal. A fixed one-world-unit exit
offset must not create or consume authored reach. Preserve deterministic
portal-versus-solid ordering and bounded path termination.

### Verification and measurement

During the planning hold, use static source and document checks only. After
release, verify the target bind before the first build. Freeze the tree for
each gate and run one Cargo job at a time. Use the owning crate's focused
tests for algebra, partition, collision, and combat. The existing
`a_partition_is_never_larger_than_its_budget` test in
`crates/ambition_platformer2d_core/src/frame/thread/tests.rs` encodes the old
fallback; replace its expected refusal behavior, and retain the work bound.
Keep the translated clipping and moving-pair regressions.

Use these production lanes when their packet changes the named behavior:

```bash
cargo check -p ambition_app
cargo test -p ambition_app --test app_it -- projectile_portal_transit
cargo test -p ambition_app --test app_it -- held_projectile_portal_transit
cargo test -p ambition_app --test app_it -- portal_floor_bounce_no_fallthrough
cargo test -p ambition_app --test app_it -- portal_translation_camera_continuity
cargo test -p ambition_app --test app_it -- rollback_
cargo test -p ambition_demo_smash_app
cargo test -p ambition_workspace_policy
```

The filters cover only their named populations. Add production witnesses for
the new invariants in the narrowest owner; these existing tests do not prove
scaled limbs or persistent parity. For codec, message, and schedule changes,
also run the repository tooling and union-feature jobs required by
[the verification recipe](../../recipes/cheapest-sufficient-check.md).
Before the final handoff, run `cargo test --workspace --lib` for this shared
API and state change. Check the render feature and the composition without
portal captures; visual quality must not change mechanical results. Do not
add source-text guards or tests for this work order.

Extend `scripts/measure/portal_seam_lab.py` to compare coverage in canonical
material coordinates. Its current steady-pixel-count test applies only at
unit scale. For an unobscured part, world area divided by its scale squared
gives canonical area; pixel measurements also need the camera projection.
Track material coverage, landmark positions, parity, and actual crossing
counts. Inspect near/far occlusion and window recursion separately from the
unobscured gauge. Test reflections with asymmetric landmarks and an equipped
accessory, including two views with different camera orientation.

The portal lab must include playable examples of these mechanics (Jon,
2026-11-08). Keep its existing stations and add labeled examples for unequal
apertures, persistent scale, asymmetric handedness, two reflections, two
simultaneous paths and a chained third path. Include bounded refusal examples.
Place each mouth against a wall or platform where possible. Keep its entry
side clear, so the player can identify the usable face.
Use targeted LDtk tool edits. The original creation spec does not include all
current stations. Use these examples for the render and seam-lab witnesses.
Station H is live in `fddf6084a`. Its
[station source](../../../tools/ambition_ldtk_tools/specs/portal_similarity_slice.yaml)
adds seven entities. The
[station source](../../../tools/ambition_ldtk_tools/specs/portal_similarity_lab.yaml)
adds I's Reflection prompt and J's axe station with four pairs. The J receipt
below records its physical contact and render checks. It does not add H again.

Station K is now authored in the live lab. It has a gun-sword beside a wall
opening and a target two pixels beyond the paired exit. A shot must hit the
target before its centre crosses the entry. The production projectile test
uses this authored pair and target. The
[station source](../../../tools/ambition_ldtk_tools/specs/portal_contact_lab.yaml)
adds eight entities, including backing walls, and retains all existing stations.
The same production test also attaches K's exit to a translating host. It
checks the physical impact during that tick, before the root transfers. This
host is part of the test; the authored K station keeps its fixed exit.
Station L adds a wall entry, a translating exit host, a target and a gun-sword.
Fire right from the lower face of the station D wall. The exit moves across
the target, so shot timing changes contact. The production test uses only
these authored objects and hits the target before root transfer. The
[station source](../../../tools/ambition_ldtk_tools/specs/portal_motion_lab.yaml)
adds seven entities and keeps the existing geometry. New portals attach to
their host before it advances; the frame refresh follows host motion.

Retain current clipping scratch reuse and presentation pools when they fit
the new authority. After Jon's optimization work ends, record a fresh baseline
on that source tree. Measure no-portal, equal-size, unequal-size, two-link,
three-link, and refused-cycle cases. Record active primitives, cells, parts,
cut work, capture pixels, allocations, CPU frame cost, and GPU capture cost.
Use target-device results for GPU conclusions. Performance changes must not
reduce physical correctness or add a separate controller path.

### Committed checkpoint

Full ray-band contacts: `7263366ed`. Rays query their complete cross-section
through the native cells. Each leg ends at its root crossing, then rebases
before the remaining reach continues. Point rays use the same path. Shrink,
expand, parity and aperture-edge witnesses pass. Core has 865 passes; combat
has 805; shared has 358; portal has 141. Five library tests remain ignored.
The app portal selection has 37 passes; the eye-laser case passes. Schema has
four passes, content portal has 64, policy has 35 and Smash has 71. Six Smash
cases remain ignored. The assembled app check and all 53 absence contracts
pass. Both baselines use v431. Full PSIM acceptance remains open.

Departure material record: `1293c397f`. `SweepSample` retains the material at
departure. Kernel admission, external pose admission and root crossing build
the same cast from that record. A later root rebase cannot change its source.
Snapshot and invalid-clock tests cover this contract. The portal-only record
and adapter are removed. Both schema baselines use v430. The app portal
selection has 37 passes; schema has four; policy has 35. Both resource-census
tests pass after host-state classification. The assembled app check and all 53
absence contracts pass. The [review receipt](../../../dev/reviews/portal-capability-and-motion-admission-review-2026-11-09.md#departure-material-record-packet)
records the owner batch and exact limits. Production root rebasing, physical
righting and full PSIM acceptance remain open.

Canonical block contact and static-cell repair: `f634f1870`. Block contact
positions now keep canonical material outside the face. Exact translating
constraints prevent an empty static cell from spending the work budget when a
distant pair turns. Both walking/camera regressions now pass with their original
assertions. Core has 860 passes and one ignored; shared has 358 passes. All 37
app portal tests pass. Smash has 71 passes and six ignored; policy has 35 passes.
The assembled app check passes. Standalone Mary-O has 140 integration passes
and three ignored, including the repaired spark/wall case. The
[review receipt](../../../dev/reviews/portal-capability-and-motion-admission-review-2026-11-09.md)
records commands and limits. This closes the reported regression gates; P07
full acceptance and all full PSIM slices remain open.


Initial-overlap contact repair: `b2e8f750a`. A projectile born in a wall now
reports contact before it can leave. Native casts have an explicit overlap
policy; body escape queries still permit separation. Core has 858 passes and
one ignored; shared has 357 passes plus the added scaled-part witness. The
real Mary-O spark/wall test and all 14 app projectile transit tests pass. The
assembled app check passes. The later repair in `f634f1870` closes the two
reported walking regressions. P07 and full PSIM acceptance remain open.


Capability and publication review repair: `9e626f08b`. The body domain samples
material at two distinct epochs. Portal publishers add room-scoped restrictions
and clear disabled-room state. Runtime selects simulation separately from the
gun; construction uses the generation's schema catalog. The generic renderer
reads authored zone presentation. Mary-O's standalone native, test, desktop and
web dependency trees exclude the portal crate; its wasm web check passes.
The app check, four schema tests, 35 policy tests and 17 mixed-game tests pass.
Portal has 143 passes; runtime has 264. The receipt records the publication cost
and all remaining failed gates. Do not combine the two material epochs without
preserving their distinct input states. P07 and full PSIM acceptance remain open.


Motion-admission review, first packet: `4afbf6100`. The body domain owns
external pose admission and prepared group poses. Sequence progress, relationship
release and boss holds now follow the admission result. A missing body material
map returns a typed refusal. The old portal-named pose API is removed.
Core motion reports generic refusal. See the [review](../../../dev/reviews/portal-capability-and-motion-admission-review-2026-11-09.md)
for the remaining composition, presentation, isolation and acceptance work.

The owning suites pass: core 856, shared 356, boss 246, combat 827, Mary O 205,
mounts 16 and actors 589. Five tests remain ignored across these libraries.
The app check and 35 workspace policy tests pass. App petting, grab and GNU-ton
selections pass. Smash passes 71 tests with six ignored. This packet does not
close the two walking/camera failures or complete PSIM acceptance.

Closed-end contact checkpoint: `627255e43`. A shortened native cast retains
first contact at its closed end. Interior topology boundaries keep their
existing contact ownership. The new witness fails before the fix and passes
for four prefix lengths and two world-time intervals after it. All 856 core
tests pass; one measurement remains ignored. The assembled app check passes.
Logs use `target/portal-closed-end-`. This query fix does not close native
actor collision or the two existing walking/camera failures below.

External pose admission checkpoint: `9f2a8fa1a`. Boss and limb pins,
mount seats, captive poses, pet stops, rescue poses and scripted end scenes
use the same atomic admission path. A refusal retains the body's physical
state and prior accepted motion record. An instant pin keeps its recorded
time; a new body is placed at tick end. The query uses each body's policy
and carried acceleration. Part geometry does not gain separate controller
velocity state.

Portal placement now accepts a finite nonzero direction and normalizes it
before publication. Invalid position, direction or size returns a typed
fault. An invalid held-pair edit retains both live ends. This fixes rounded
authored directions that caused native pose admission to reject GNU-ton's
portal geometry.

The core and shared libraries pass 855 and 354 tests. One core measurement
remains ignored. The portal library passes 142 tests. All 39 GNU-ton fight
tests pass, including rewind identity. The assembled app check passes.
The constructor client test targets compile. Boss encounters, combat, Mary O,
mounts and actors pass 246, 826, 204, 15 and 588 tests. Three combat tests and
one Mary O test remain ignored. The broader app `portal` selection passes 34
tests and fails two: `c141_to_c140_preserves_screen_position_and_continues_right`
and `thin_wall_walk_keeps_apparent_player_position_smooth`. The first reports
a discontinuity; the second reports no crossing. Both failures reproduce with identical assertions against the Rust sources
of `f7ad6e564`. The baseline run passes one case and fails these same two.
Do not report the broader selection as green. Baseline evidence is in
`target/portal-camera-baseline-status.json` and its log. Other logs use
`target/portal-pose-final-` and `target/portal-pose-clients-`.
These checks cover external linear pins. They do not close moving-host
admission, physical righting, native actor contacts or any full PSIM slice.

Recorded source-rotation checkpoint: `b7e11ad40`. One native source curve
controls part maps, interval bounds, root paths, contact normals and payload
velocity. It keeps full turns about a translating pivot and derives angular
velocity from the original duration. A closed prefix retains the original
curve and world clock. A later winding can fit after the first pose fails.
Small turns about distant pivots retain their radial displacement. A cast
without a gameplay policy does not acquire a portal scale cap.

All 849 core tests and 139 portal tests pass. One core measurement remains
ignored. The assembled app check passes. Fourteen new tests cover source
rotation, orbiting circles, moving-frame velocity composition, numerical
refusal, contact covariance and closed prefixes. Logs use the
`target/portal-source-rotation-final-` prefix. These tests exercise the query;
physical righting and the remaining pose producers do not yet emit these
curves. No PSIM acceptance slice is closed by this packet.

Source refusal checkpoint: `46058c220`. A rejected operation records only the
retained pose at tick end. The real-app G test covers refusal and later transfer
when its budget is restored.

Native transit checkpoint: `0aeb65a81`. The packet replaces its box envelope with the canonical body
and material map. The opening fit uses physical material axes. A long turned
body can pass when its material fits even if its envelope is too wide. The
witness covers two scales and both parities. All 139 portal tests, 64 content
portal tests and 14 app portal tests pass. The assembled app check passes.
These checks do not close actor collision, root rebasing or PSIM acceptance.

Shared item-body checkpoint: `821ce15c3` (2026-11-09). Loose items now use the shared
body and sweep state. Custody removes stored bodies from spatial queries. The
separate free-body portal path is removed. Item impact response belongs to the
item motion owner. Rollback schema 429 records the new storage contract.

The item suite passes 47 tests. The portal, ability and content selections pass
138, 57 and 69 tests. Smash passes 71 tests with six ignored. The assembled app
check passes. The app batch passes 109 tests; its target-fixture and schema
failures pass in a seven-test follow-up. The five coconut tests and the two-host
physical-state check pass. The 34 passing policy tests and repaired engine
policy check cover all 35 policy tests.

The two prior resource-classification failures remain. Repository tooling
reached 542 passes and 15 skips before it was stopped after the existing
consolidation-ledger failure. Doc anchors pass. The prior doc-link and citation
failures remain. These results do not claim a green full-repository run.

The continuous-admission packet now uses the native material cells and their
recorded time intervals. Payload validation covers each occupied interval,
including intermediate aperture turns. A closed cast prefix keeps the original
line and world clock. Projectile world and damage contacts use that parameter.
A returning shot also resolves a later wall after its feature hit.

This packet passes 835 core tests, 138 portal tests, 19 shared projectile tests,
60 runtime projectile tests and 13 assembled-app portal tests. One core test
remains ignored. The assembled app check passes. Logs use the
`target/portal-native-prefix-` prefix. A real-app follow-up exposed a refused actor that still crossed through the
full-turn G aperture. Refusal now records only the retained pose at tick end;
it does not invent an earlier stationary path. The actor remains in place with
zero hops and crosses after its budget is restored. Both G app tests pass.
The focused kernel refusal test, 47 item tests and 29 projectile-combat tests
also pass. No complete PSIM acceptance slice is closed.


Code checkpoint: `e3ebef90f` (2026-11-09). Lab map checkpoint: `16018b5` in
the map submodule. The broader portal goal is resumed and incomplete.

The root query follows recorded aperture turns and returns the crossing time
and sampled poses. Transit and projectile contact limits use this query.
An overlap without a recorded path cannot transfer a root. A shortened
projectile leg keeps the original line and its closed end parameter, so
projection of its endpoint does not change its crossing time. Rollback schema
428 and its baselines include these motion fields.

The fit, carve and snag rules now use the authored opening width. The former
two-pixel allowance could open a hole for a body that the root query could not
transfer. It is removed. The real-app oversized-body regression passes and
checks support against the actual host face.

Verified for this packet:

- Core library: 827 passed, one ignored. Portal library: 138 passed.
- Runtime projectile tests: 59 passed. Assembled projectile selection:
  19 passed, including the full-turn G spinner and the oversized-body case.
- Content portal selection: 64 passed. Schema-baseline selection: four passed.
  The codec-shape check passes for 30 files.
- Workspace policy: 35 passed. Smash: 71 passed, six ignored.
- Assembled app check and core/portal doctest gates passed.

The earlier full rollback selection passed 82 tests with six ignored and three
failures. Its schema-header failure is fixed and the four schema tests pass.
The two existing resource-classification failures remain:
`SmashAutoplayDriver` and `SpriteSheetWatchPolicy`. The broad rollback selection
was not repeated after the focused fix. A broader content run also found the
unpublished `history_afonso_letter` sheet. These results do not claim a green
full-repository run.

Final affected-check results are in `target/portal-checkpoint-gates.json`,
with portal and Smash logs under the same prefix. Earlier core and policy
results are in the `target/portal-prefix-*.log` files.

K and L are live lab examples. The F/G label updates are committed; their
semantic diff changes two text fields and no geometry. Both labels are visible
in the reviewed `/tmp/portal-lab-moving-entries-wide.png` capture. The map tool
also reports the existing `scroll_lab_door` validation error. The later H and J
receipts below replace the draft status for scale, chirality
and multi-link acceptance examples.

Continue persistent root material rebasing, remaining motion in the
same tick, complete source admission and canonical consumer integration.
Unequal apertures, GNU-ton acceptance and numerical-fault presentation remain
open. Union-feature, full library, replay, rendering and performance acceptance
remain open. Do not switch the minimum-opening policy or mark
PORTAL-SIMILARITY complete from this checkpoint. Preserve the unrelated dirty
submodules, science.ldtk, profiling measurements and the untracked related-work
ideas file.

### Draft status

The tested foundation is merged into main with the Android profiling and asset
loading work. No complete PSIM acceptance slice is closed.

- Loose items use `BodyKinematics` and `SweepSample`. `GroundItem` owns item
  metadata. Custody derives `BodyInStorage` before spatial reads. One portal
  path handles carves, snag, eviction and transit. The separate free-body
  trait and its systems are removed. Pickup and release retain material state.

- `Sim2` keeps scale, orthogonal composition and translation in double
  precision. It validates projected coordinates, feature resolution and moving
  surface velocity. Numerical failure is typed.
- Parts retain canonical primitives and double-precision cut covectors. World
  volumes and planes are private projections. Root parity is part of the full
  transform. The rebase operation preserves world geometry.
- Part count, path length and cumulative scale have explicit limits. A limit
  refuses the operation and returns no partial geometry. The default scale
  range is `1/4096..=4096`; a body or room can set its exponent limit.
- Body dimensions remain canonical. Snapshots retain material scale and axes.
  Restore validates the saved axes without normalizing them again. A byte
  round-trip test guards signed zero and rounding in the peer checksum.
  Each movement policy derives world units for its step and then restores the
  authored parameters. World gravity and body time do not change with scale.
- The body kernel validates a private candidate, its sweep and mapped part
  velocities before commit. Rejection retains body state and clears stale
  travelled paths. Both controller adapters use the same admission input.
- The common transit result includes the validated similarity, both apertures,
  stable directed channels and actual entry, mapped and assisted exit velocities.
  The common body path validates carried acceleration before changing body
  state. The same transfer commits carried run. The separate runtime adapters
  that reconstructed rigid maps from normals are removed.
- Carried acceleration is a shared body capability. Projectile construction,
  stepping, snapshots and restoration use that value. World gravity and force
  zones keep their own contributions. The rollback schema is 429.
- Source admission checks carried acceleration along each path. It does not
  map world gravity or world force fields. Projectile flight validates a
  candidate before it commits motion or contacts. Refusal clears stale travel
  and keeps the prior pose; lifetime continues. The same shot resumes after
  its budget is restored.
- An approaching body prepares both host openings for its own use. A fast
  crossing can skip the begin latch and still advance from the exit without
  meeting that exit's uncut host. This applies to actors and loose items.
- The material cast query partitions a swept spatial domain and clips each
  physical obstacle in that domain. It casts the actual primitive, keeps
  circles analytic and returns the source collider and directed path.
  Projectiles use this query for world contacts. Actor kernels still use
  their prior contact paths. Translating blocks use their full swept bounds.
  The contact query keeps the primitive fixed in canonical coordinates.
  Motion changes the cell maps and cut offsets. One analytic solver handles
  fixed and translating boundaries. Broad phase bounds clip the source box
  in source position and time, so an intermediate cell is not missed when
  both endpoint poses are empty. Contact still uses the actual primitive.
  A cast carries its interval within the world tick; a projectile leg that
  stops at an entry does not test later obstacle motion. A live app witness
  hits a moving wall that misses the far part at both endpoint poses, before
  the root transits. Core witnesses check independent cell motion, a shared
  moving host boundary, exact circle corners, scale, parity and frame boosts.
  The temporal constructor now consumes recorded translation. It tracks the
  intervals in which material exists, splits at entry-rank and clamp changes,
  and counts simultaneous cells rather than disjoint time windows. Contact
  uses the recorded world time for the inverse moving-frame velocity law.
  Translating lines use their swept bounds and published displacement.
  Rotating colliders use bounds over their recorded curves. The query keeps
  full windings and the actual primitive. A rejected contact component does
  not hide a later approach in the same tick. Surface velocity, one-way
  normals and edge identity use the physical contact time. A live app test
  uses a spinning platform that hits far material between matching endpoint
  poses. Rotating apertures now use convex source cells with curved maps,
  moving cuts and conditional link order. Interval bounds exclude empty
  regions; contacts use the actual primitive and sampled cuts. Both directions
  use the same host curve, including full windings. The temporary
  refusal for curved motion is removed. Tests cover two simultaneous curved paths
  and a third link under both orientation conventions. A live app test attaches
  K's exit to a spinning plank and hits a target outside both endpoint poses.
  Line openings now follow the same recorded pose. Their finite ends remain
  solid. The separate end-pose line subtraction is removed. A sliding opening
  test checks box and analytic circle contact times. Contact-only source
  strata do not consume material piece slots. Root queries now follow the
  recorded aperture turn and return both endpoint poses at the crossing time.
  Transit and projectile contact limits use this scan. An overlap without a
  recorded path cannot transfer a root. A shortened projectile leg retains
  the original line, its closed end parameter and its world-tick interval.
  Transit uses that record instead of a line through the projected endpoint.
  Motion samples use rollback schema 428. Physical righting,
  persistent root material rebasing, motion after a crossing and complete
  motion-source admission remain open. The generic transit adapter and projectile scan both supply canonical
  material and its physical axes. The root scan does not close angular transit acceptance.
  A host wrap closes the
  old plane; its aperture does not sweep between the two locations.
- Projectile world contacts use the original collider and the selected part.
  The response uses its normal and physical contact point. Surface velocity
  returns through the inverse path with the moving-frame law. Flight and
  response are prepared before commit. A bounce changes the body once and
  spends one bounce. The motion record ends at contact and retains the
  incoming velocity.
- Projectiles query uncarved room geometry. A solid that only touches a
  spatial-cell boundary does not occupy that cell. Line hosts lose only
  their finite opening intervals. The old carve box removed obstacles up to
  16 pixels in front of an exit. A live portal-lab test now stops a shot on
  an obstacle two pixels outside the exit, before its root transits.
  Projectiles now use the shared physical room composition and its cache.
  This includes live moving platforms and contributed lines. The separate
  projectile composition that omitted these surfaces is removed.
  Closed contact strata also retain the first touch at the complete cast end,
  before any area crosses. The opening excludes its host, while finite ends
  and far obstacles keep their contacts. These boundaries do not consume
  material piece slots.
- The block-only endpoint scan, overlap snap, one-pixel bounce lift and
  projectile crossing nudge are removed. The same material query orders and
  resolves each world contact. Body and feature hit queries still need to
  use the shared cast. Oblique line-host alignment also needs verification.
- Rays use directed links and the common similarity. Reach is measured in
  the initial chart, with each segment divided by cumulative scale. They
  start at the exit face without a fixed offset. Hop, scale and numerical
  refusals return no partial ray. The full strike band is admitted before
  publication. Short rays keep their authored length and width.
- `PartSource` separates body material from world material with an optional
  policy owner. A world strike can use its author's budget without using the
  author's body chart or active transit. Ray admission, hit resolution and
  the folded line mesh retain that policy owner.
- Apertures publish before integration. Body material maps refresh after
  transit. Combat and presentation consume fallible partitions.
- `BodyPieces` is removed. The held gun uses the shared attachment carry path.
  Pooled textured meshes clip against every boundary; the six-plane shader
  limit is removed. Folded meshes clip in the source chart.

`cargo check -p ambition_app --tests` passes on the temporal contact change.
Focused library tests pass: 819 core, 351 shared-state, 169 world, 140 portal,
46 projectile and 29 projectile-combat tests. Two library measurements remain
ignored. The content library passes 64 portal tests. The assembled app passes
all 17 tests selected by `projectile`. This includes station K's first boundary
contact at tick end and the spinning-exit case. The runtime
library passes 59 projectile tests. The curved aperture change also passes
71 Smash host tests and 35 workspace policy tests. Six Smash tests remain
ignored. The preceding ray work passed combat,
portal and renderer libraries and the assembled eye-laser and portal cases.
These results do not close replay, visual or performance acceptance.

External linear pose pins use the shared source admission. Finish source
admission for the remaining free-body sweeps, physical pose turns and moving
aperture hosts. Finish projectile body and feature contact queries,
and the remaining attack updates.
Body-attached ray origins still need the common material pose pass. Ray wall
contact uses its full cross-section in `7263366ed`; point rays use that same
native query. Each source leg ends at its root crossing before rebasing.
Check retained support facts, curved paths, control-phase displacement,
topology changes after the aperture snapshot and candidate-copy cost.
The existing gates do not close PSIM-1B.

Complete root transit and righting, common exact collision/contact projections
and the unequal-aperture switch remain open. The root scan has tests for full
windings, finite circle and polygon openings, common boosts and world-time
subintervals. A large world origin cannot widen an opening. The live station G
regression uses its authored spinner and pair with a full turn in one tick.
Station F and G labels identify moving entry and stationary-body pickup.
GNU-ton acceptance and numerical-fault presentation remain open. Keep the
minimum-opening policy in production until the coordinated switch. Measure
allocation and frame cost on the completed production paths.


### Fast posed limbs and multiple paths

`2c4bd3822` removes the distance shortcut that skipped solid contact for fast
conductor poses. The shared native cast stops two attached limbs at independent
0.5x and 2x exits in one tick. It also stops a fast limb at a solid after three
pairs. The real phase-two GNU-ton witness keeps two fist entities and their
owner while both occupy separate pairs and one reaches a third cut.

Runtime libraries (252), the assembled app, the new GNU-ton witness, the
existing far-side punch and rewind witnesses, and Smash pass. Logs use
`target/portal-limbs-`. This packet does not prove simultaneous-path capture,
chained hit/knockback, chained replay, or the remaining lab stations. Full
PSIM-3 and P07 acceptance remain open.


### Chained strike launch and replay

`99d51bc9f` keeps launch speed in the contacting material's units. Direction
normalization no longer discards scale. A strike keeps canonical material when
no portal snapshot is present. The same strike therefore gives the same launch
through three cuts at a total scale of 0.5 and after its source rebases.

The guards are `a_chained_half_scale_strike_keeps_its_launch_after_the_source_rebases`
and `a_gnu_ton_fist_hits_once_after_three_cuts_and_replays_its_scaled_launch`.
The GNU-ton witness runs under the rollback host. Its fist deals seven damage
once, produces a reflected launch at half speed, and moves the player left and
up. A second run repeats the body, health and hit-ledger trace bit for bit.
Combat libraries (819, three ignored), the assembled app, both GNU-ton guards,
all 38 app portal tests and Smash pass. Logs use `target/portal-launch-`; Smash
results are in `.agent/portal-launch-smash-artifacts.jsonl`. These checks do not
prove visual capture or two simultaneous damaging limbs. Full PSIM acceptance
remains open.


### Offscreen 80/40 image witness

`3e443d731` adds `--portal-convention rotation|reflection` to `capture_scene`.
The choice applies before simulation starts. Frame snapshots now include body
material and canonical facing, plus presented facing and sprite flip.

The tool builds and all seven existing tool tests pass. Real offscreen runs
produce 80 frames each for Rotation and Reflection at station H. Their body
snapshots retain canonical size and give positive and negative parity,
respectively. The controlled Reflection run keeps canonical facing +1, clears
the exit at half scale, and visibly puts the robot's antenna and face on the
opposite sides. No renderer change was needed.

The captures use the low quality profile, a fixed view, and windows off. The
images and frame snapshots are under `target/portal-visual-h/`. The comparison
is `heading-comparison.png`. Normalized material coverage, GNU-ton multi-path
capture, other quality tiers, and device/performance acceptance remain open.

Reproduce the controlled Reflection film after building `capture_scene`:

```bash
AMBITION_QUALITY_PROFILE=low target/debug/capture_scene portal_lab 2176,850 \
  target/portal-visual-h/reflection-heading/f.png 1280x720 \
  --player-at 1860,875@60 --player-holds right@60 \
  --player-holds left@72 --player-holds none@82 \
  --warmup 55 --frames 80 --stride 1 --tick-per-frame --camera-zoom debug \
  --portal-effect off --portal-convention reflection --art-snapshot
```


### Requested vertical slice checkpoint — 2026-11-10

Station H completes the requested 80/40 slice. The final main-tree run passes
all 32 app tests selected by `portal_`, including both slice witnesses, and
all four rollback-schema tests. The commands are:

```bash
cargo test -p ambition_app --test app_it -- portal_ --test-threads=6
cargo test -p ambition_app --test app_it -- rollback_schema_baseline --test-threads=6
```

The logs are `target/portal-slice-final-app.log` and
`target/portal-slice-final-schema.log`. The offscreen image receipt above
records the visible asymmetric body at half scale with reflected landmarks.
The later J receipt below records its live station. Full-plan acceptance remains open.


### Backed lab and simultaneous paths — 2026-11-10

`0f64b41bf` adds station J and backs its eight mouths and both K mouths with
walls or platforms. The map commit is `7e3ea79` in `ambition_map_assets`.
The semantic diff has 25 added entities, no tile or definition changes, and
no changes to the other 79 levels. The existing `scroll_lab_door` validation
error remains.

J has a real axe pickup and an infinite target. The strike crosses three
pairs at quarter scale with reflected material while a foot uses another
pair. GGRS 4/10 replays the contact. A two-hop limit refuses the strike, keeps
the body supported, and causes no target damage. F3 and `capture_scene` now
expose the composed geometry budget.

Body rendering now starts from physical occupancy before centre transit.
Reaching swings keep their own cut rule. Cleared carry targets have no CPU
pixels, so a new target cannot spend the upload budget on zeroes and defer
changed clip meshes. The clean impact frame at tick 88 shows the body and the
far foot. The film is under `target/portal-visual-j/final/`.

This packet passes 34 portal app tests, 128 presentation tests, 25 host tests,
seven capture-tool tests, 73 Smash tests and 35 policy tests. Six Smash probes
remain ignored. The assembled app check passes. Logs use the prefix
`target/portal-lab-final-`. Station I is a prompt at an existing wall pair;
its held-accessory film and full PSIM acceptance remain open.


The first upstream merge passes both J witnesses, all four schema checks and
the assembled app check again. Planning links, anchors and the changed plan's
strict citations pass. The global strict citation scan reports 26 unresolved
references outside this plan on `71435aa51`; it is not a green repository-wide
documentation gate.


The final merge passes both J tests and the assembled app check.
`8b278ec6b` aligns both recorded baseline headers with upstream runtime schema
v442; their peer row set is unchanged. All four schema checks then pass.
The final logs are `target/portal-lab-final-main-chain.log`,
`target/portal-lab-v442-schema.log` and `target/portal-lab-final-main-check.log`.


Review follow-up `670e4683c` fixes P1 recipient scope and P2 translation
admission. Recipient sweeps check conservative bounds before
partition admission. An early solid stop can pass even when the unreachable
motion suffix refuses. Guards cover a remote invalid recipient, a thin
contact after chart cancellation, and a wall before a refused portal path.
After the upstream merge, the assembled app check, all 15 projectile app
cases and three GNU-ton punch cases pass. The punch cases include rewind.
See the [review receipt](../../../dev/reviews/portal-correctness-and-cost-review-2026-11-10.md).
R3 is fixed in `5f07fbb17`. Axis repair, initial support, line settling and
platform support use the same admitted material as movement. The guards
cover stationary far-side support, a ground-line opening at rest and during
transit, source-chart retention and canonical rounding. The H slice pauses
a partial body before reverse transit and deterministic rewind.

The packet passes 878 core, 595 actor, 178 world, 35 portal app, 73 Smash and
35 policy tests, plus the assembled app check. One core probe and six Smash
probes are ignored. Do not restore a root-AABB correction pass after native
material queries. R5 and R6 remain open. Full PSIM acceptance remains open,
including other movement models and general remaining-time continuation.

After upstream merge `a40680fd4`, both 80/40 slice cases and both chained-lab
cases pass again. The app and core all-target warning check passes under
their default features. See `target/portal-r3-merged-gates.json`.

### Independent captures and room identity — 2026-11-10

`ae8f2b07a` gives each drawable family its own root, capture and depth order.
Its two-body guard compares the same policy and link with isolated geometry
in both spawn orders. `b499e5090` resolves straddler links in the owning room;
identical aperture poses in another room cannot select its key or motion law.

All 1,483 active owner cases, 35 portal app cases and 35 policy cases pass.
Four owner probes are ignored. The assembled app check and capture tools build
pass. The close Vulkan llvmpipe film in `target/portal-r4-visible/close/` shows
a robot and javelin in the same 80/40 pair on separate captures. The robot
clears at persistent half scale and reflected parity. Evidence is
`target/portal-r4-visible/witness.json` and `target/portal-r4-r8-gates.json`.

Do not merge independent roots or drawing depths to share a capture.
Do not resolve a straddler by searching other rooms. R5 still needs runtime
measurements; R6 still needs adaptive capture resolution, resource limits
and an explicit overflow policy. Device/GPU and full PSIM acceptance remain
open.

The app, portal presentation and host all-target warning check passes under
their default features. Its log is `target/portal-r4-r8-warnings.log`.

After the upstream day-clock save merge, the assembled app check and all four
schema guards pass with v443. Evidence is
`target/portal-r4-r8-merged-gates.json`.

### Adaptive capture storage — 2026-11-10

R6 is fixed in `1d551594b`. Viewport density sets each target resolution.
A 32 MiB default budget reduces resolution before selecting clipped canonical
material proxies. All admitted families keep their geometry, scale, reflection
and depth order. Idle targets and their hidden material references retire.
Do not omit a family or change physical admission to meet a texture budget.

The twenty-family, viewport, idle-release and reflected-proxy guards pass.
Validation passes 985 owner cases, 35 portal app cases, 35 policy cases and
seven capture-tool cases, plus the app check and capture-tool build.
The llvmpipe films reduce peak target storage from 64 to 2.3125 MiB at
1280 by 720 and from 16 to 0.578125 MiB at 540 by 960. Idle storage is zero.
Matched-tick player and item physics are equal before and after the change,
across viewport sizes and with zero-byte proxies. See the
[review receipt](../../../dev/reviews/portal-correctness-and-cost-review-2026-11-10.md)
for evidence and limits. R5, hardware/device performance and full PSIM
acceptance remain open.

After upstream room-rollback merge `dfdfa5f3b`, all four schema guards pass
with v445. The app, portal presentation and renderer all-target warning check
passes under default features. Evidence is `target/portal-r6-merged-gates.json`
and its warning and schema logs. Non-default feature-only code is not covered.

R5 has a simulation baseline: two mirror matches and one
mixed four-fighter match each complete 1,800 ticks with 100% sampled live-cast
coverage. The warmed native sample attributes 2.14% inclusive user-CPU samples
to material-cell construction. This is an unoptimized workspace build with
the windowless assembled host. It does not establish a regression or justify
a second collision authority. The historical control and optimized/displayed
host costs remain open. See the
[cost receipt](../../../dev/ambition_dev_measurements/summaries/portal-free-material-cost-20261110.md).

### Held accessory through two reflections — 2026-11-11

`04a1625b1` fixes a held javelin tip that drew inside the source wall while its
holder was in front. Both held-picture paths publish the holder's reach root
and ownership. The common carry respects that owner's material budget. The
guard checks a refused hop limit and exact near and far mesh bounds. Do not
partition a held picture from its own centre or substitute the room budget
for its owner's policy.

Station I now has a visible held-javelin witness through two reflections,
including full clearance and return. `db953edef` adds fixed-view camera roll
and sprite transform evidence. Zero- and ninety-degree films match physics
and held world transforms at 138 ticks. Before/after physics match at 136
ticks. The item stays held at all 111 common ticks after pickup. Both root
crossings and the clipped tip were inspected in both views.

All 791 active owner cases and 35 policy cases pass. Two view probes remain
ignored. Tool tests, tool build, app check and the default-feature all-target
warning check pass. The app check passes after the upstream observer merge.
See the [accessory receipt](../../../dev/ambition_dev_measurements/summaries/portal-held-accessory-20261111.md)
for commands and limits. Full PSIM acceptance, physical attachment scale and
release behavior remain open.


### GNU-ton independent limb pictures — 2026-11-11

`9d6f9da4f` gives generated limb pictures their own physical roots and declares
image demand for every animated root in a portal room. Cached body images
wait for their own source images and GPU passes before they rest. Guards cover
an unclassified picture and delayed GPU readiness. Do not use a host root for
a limb cut or treat CPU pose equality as proof of a finished image.

The staged 40-frame GPU film retains the giant and both carried fists. The
left path has three cuts at half scale; the right has one at double scale.
All 524 renderer, 25 host, seven tool, three native GNU-ton and 35 policy
cases pass. App and default all-target warning checks pass after the merge.
The [render receipt](../../../dev/ambition_dev_measurements/summaries/portal-gnu-limb-render-20261111.md)
records commands and limits. This is a diagnostic scene. Full PSIM acceptance,
hardware GPU, mobile and performance requirements remain open.


### Carried projectile contact and payload units — 2026-11-11

`a91231348` replaces the full root-box damage sweep with admitted source cells.
`4825f1f1c` prepares launch units before commit and checks body eligibility
before geometry. Source bounds stay separate; an invalid body in the space
between portal ends cannot refuse the shot. World and recipient queries reuse
admitted bounds. Do not convert a contact refusal to a miss or scale `FeelScale`.

The real actor loses one HP before root transit through both 80/40 directions.
Launch speed follows the chart ratio. A spinning exit reaches sampled hurt
material between matching poses. All 1,730 owner, 17 projectile app, two slice
and 35 policy cases pass. The slice includes reverse and rewind/replay. App and
default all-target warning checks pass. The merged app and projectile cases
pass again. See the [contact receipt](../../../dev/ambition_dev_measurements/summaries/portal-carried-projectile-contact-20261111.md)
for commands, staged validation and limits. Full PSIM acceptance remains open.

### Carried projectile interception — 2026-11-11

`d795c9268` fixes parry position, velocity and cue placement for a carried tip.
The response uses the accepted contact prefix and the existing material
payload law before one commit. The H app guard checks reflection and absorption
at both 80/40 ratios before root transit. The moving-surface core guard checks
translation, spin and response overflow. Keep cues at the physical witness and
admit the response before guard rewards or ownership writes. The
[interception receipt](../../../dev/ambition_dev_measurements/summaries/portal-carried-projectile-interception-20261111.md)
records the passed gates and audio merge `fec049833`. Moving targets, other
responses, general continuation and full PSIM acceptance remain open.

### World response admission — 2026-11-11

`10f8eaf95` rejects a rebound that fits the root chart but overflows carried
material. Contact and response have separate typed results. An earlier terminal
target outcome retires the later rebound; a reached refusal keeps body and
combat writes uncommitted while lifetime advances. Guards cover 2x overflow,
the 0.5x control, an earlier hit or parry, and an H bounce through both 80/40
directions that clears contact on the next tick. Keep response admission before
pose, budget and combat writes. The
[world response receipt](../../../dev/ambition_dev_measurements/summaries/portal-world-response-admission-20261111.md)
records the passed gates and merge `eff565e6d`. General continuation, moving
targets, the remaining response acceptance and full PSIM acceptance stay open.


After timing merge `a0568d91b`, 434 owner, 38 runtime projectile and 19 projectile
app cases pass. Both H slice cases pass, including reverse travel and
rewind/replay. Both room-freeze cases, the assembled app and default warning
checks pass. The world response receipt records the stage boundaries. Full
PSIM acceptance remains open.

### Projectile root continuation — 2026-11-11

`cad13e2eb` continues projectile motion after each accepted root crossing.
Generic transit and projectile motion use one prepared transfer and commit.
Age and the force kick run once. Each leg keeps its world-time interval and
departure material. A returning shot keeps one ordinary contact budget for
the whole tick. A refused leg retains the previous admitted root.

The guards cover both H ratios, L's moving exit, J's three crossings and
two-hop refusal, a recipient beyond the exit, and one returning-shot contact
across roots. After room timing merge `b0e5bd404`, owner, runtime, projectile
app, both H slice, room timing, app compile and warning checks pass. The H
slice includes reverse crossing and rewind/replay. Keep the clock and contact
budget outside the leg loop. The
[continuation receipt](../../../dev/ambition_dev_measurements/summaries/portal-projectile-root-continuation-20261111.md)
records commands and scope. General kernel and other free-body continuation,
moving-target hurt CCD and full PSIM acceptance remain open.

### Authored guard material — 2026-11-11

`065676df3` replaces gravity-only guard placement and world AABB contact with
a canonical region under the accepted body material. Contact and front/top
coverage use that same region through portal cuts. Numerical refusals prevent
recipient writes. The local guard checks both aperture ratios, root parities
and an empty bound corner. The real Warden checks a half-scale reflected
guard and a double-scale turned guard through the publisher and resolver.
Keep bounds in the broad phase. The
[guard receipt](../../../dev/ambition_dev_measurements/summaries/portal-guard-material-20261111.md)
records all passed gates and a deliberate AABB failure. Deflect response,
autonomous facing under material reflection, held-item placement/release,
general movement continuation and full PSIM acceptance remain open.

### Control facing and crawler direction — 2026-11-11

`e5a0e4ac1` maps facing between control axes and canonical material in the
shared kernel. The actor override after the kernel is removed. Neutral input
sends no turn request. Axis requests use the existing turn permissions, and
concrete melee requests use the accepted body pose. Crawler route control and
travel use material parity. A tighter signed-plane bound in the existing
curved contact query removes the angled-chain righting `WorkLimit` found by
the crawler witness. The work limit and typed refusals remain in force.

The core guards cover turn, scale, parity, neutral input, air locking and
reflected crawler travel on blocks and an angled chain. The transformed
Warden guard case now runs with its brain active. Portal reverse/replay,
projectile and GNU-ton path cases pass. After room-state merge `09f0afbc4`,
owners, the assembled app, Warden, reverse/replay, schemas, room cutscenes,
conversation rewinds, control publication and warnings pass. The focused
Sanic ramp/loop/runout case also passes; the full workspace was not repeated.
Do not restore the actor facing override after the kernel. The
[facing receipt](../../../dev/ambition_dev_measurements/summaries/portal-material-facing-20261111.md)
records commands and scope. General movement continuation, complete physical
integration of other motion models, moving-target hurt CCD, guard responses,
held-item placement/release, numeric fault identity/effects and device and
performance gates remain open. Full PSIM acceptance remains open.


`42fe3ccfd` also projects wall-cling and ledge-grab facing from the control
frame into canonical material axes. Acquisition and continued hanging use
that rule. A perpendicular request keeps the current facing. Core (885),
actor (600), Smash (73 active) and the assembled app check pass. The
[facing receipt](../../../dev/ambition_dev_measurements/summaries/portal-material-facing-20261111.md)
records scope. This closes these contact-facing writes; the open physical
integration and acceptance work listed above remains open.


### Guard response charts — 2026-11-11

`16d3d2ff6` retains the strike and guard charts for deflect response. Both
launches use the guard's contact-frame velocity and canonical speed units,
then return to their own root frames. Durations do not scale. Both launches
are prepared before body, move or guard state changes. Do not infer a carried
guard response from root positions in different charts. Source owner, app,
reverse/replay, Smash, policy and warning checks pass. After merge `10b07aef7`,
the app, Warden and schema checks pass. The
[guard receipt](../../../dev/ambition_dev_measurements/summaries/portal-guard-material-20261111.md)
records scope. The overlap midpoint estimate stays in place; swept contact
time/point and root-epoch reception remain open. General motion continuation,
other motion models, attachments, numeric fault effects, device/performance
and full PSIM acceptance remain open.


### Item release material — 2026-11-11

`a66c9209d` records the pickup material basis and admits release before hand,
custody, stock, mint or participant retirement changes. The retained item
keeps its material through the holder's later transform. Native partition
selects the released root chart; its own policy and portal surface velocity
apply. `03a9efeaf` keeps the shared-load sling anchor. Do not treat a refused
object release as a quantity mint. The authored H javelin passes scaled
release, real rewind and a second bit-identical replay. Owner, app, schema,
Smash, policy and warning checks pass in the scopes in the
[release receipt](../../../dev/ambition_dev_measurements/summaries/portal-item-release-20261111.md).
The combined cutscene/item schema is 458 (`dbc086852`); app, schema, javelin
and absence checks pass. The later presentation merge `df6e93285` passes the
app check. Held pose/render, durable material history, free-item motion and
collision, and the other full PSIM and device/performance gates remain open.


### Material landmarks — 2026-11-11

`5db057506` places rig and package landmarks with canonical feet, facing and
physical material. A map fault cannot select the missing-hand fallback. The
held renderer reads the simulation hand when the art has none. Hand muzzles
resolve the hand before firing state changes; other muzzles do not query it.
The [landmark receipt](../../../dev/ambition_dev_measurements/summaries/portal-material-landmarks-20261111.md)
records owner, app, pet, slice, Smash, policy and warning checks. Merge
`f0af28d7c` passes the app check. Portal-path attachment admission, full held
shape/scale/orientation, charge-shot placement and the other PSIM gates remain
open. Do not restore a gravity/AABB landmark placement path.


### Physical charge births — 2026-11-11

`b8788d097` maps charge births through physical hands and canonical box
geometry. Neutral aim follows physical facing. Hand and charge shots share
physical feet-plane clearance. A refused birth spends no meter or cooldown
and emits no shot. The [birth receipt](../../../dev/ambition_dev_measurements/summaries/portal-charge-birth-20261111.md)
records owner, real-app, runtime, Smash, policy and warning checks. Merge
`ef56f8599` passes the app check. Aperture-path birth admission, full held
geometry, free-item collision and the other PSIM gates remain open. Do not
use gravity or a root AABB to reconstruct the physical firing hand.
