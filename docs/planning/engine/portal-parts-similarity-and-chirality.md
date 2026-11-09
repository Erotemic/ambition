# Portal parts: exact partitions, similarity transforms and numerical failures

**Status:** MAINTAINER-APPROVED DESIGN, IMPLEMENTATION OPEN (2026-11-06).
**Execution:** [PORTAL-SIMILARITY](../queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals).
**Work state:** IMPLEMENTATION AND TESTING ON MAIN (Jon, 2026-11-08).
Jon released main and asked for the tested slot 1 work to be merged. Continue
on main with all VM CPUs available. Preserve `portal-similarity-draft` in slot 1
as the merged checkpoint. Feature acceptance remains open. See
[Implementation plan](#implementation-plan) and [Draft status](#draft-status).
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
| Domain queries | `crates/ambition_platformer2d_shared_tangle/src/portal_passages.rs`: `pieces_from` logs refusals and returns only pieces. Return an explicit admission result to simulation callers. Keep room and body policy ownership. |
| Pair openings | `crates/ambition_portal2d/src/types.rs` and `crates/ambition_portal2d/src/link.rs`: `opening_half_length` takes the minimum through `partner_opening_half`. Remove that field and the equalizer when all consumers support the directed map. |
| Transit | `crates/ambition_portal2d/src/placement.rs` and `crates/ambition_portal2d/src/transit.rs`: the common machine maps position and velocity. `PortalBodyTransited` carries normals but no full map; `FreePortalBody::emerge` accepts only position and velocity. Extend the common result and both adapters together. |
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

The root map must state how physical orientation, gravity alignment, facing,
and pose compose. `ActorRoll` currently owns visual righting. It must not be
an independent writer of the orientation used for physical material. Keep
pure visual motion as a presentation offset; if righting moves physical
material, submit it through the same pose admission as other body motion.
Turning the controller's facing must not reset material parity.

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
   the partitioner. Thread admission through `PortalPassages`, movement,
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
Use targeted LDtk tool edits. The original creation spec does not include all
current stations. Use these examples for the render and seam-lab witnesses.
The [station draft](../../../tools/ambition_ldtk_tools/specs/portal_similarity_lab.yaml)
adds an 80/40 pair, an asymmetric javelin, a two-reflection prompt and a
simultaneous/three-link arrangement. Its authoring preview adds 17 entities
and changes no existing geometry or station. Apply it with the production
switch; the current minimum-aperture rule cannot demonstrate its scale labels.

Retain current clipping scratch reuse and presentation pools when they fit
the new authority. After Jon's optimization work ends, record a fresh baseline
on that source tree. Measure no-portal, equal-size, unequal-size, two-link,
three-link, and refused-cycle cases. Record active primitives, cells, parts,
cut work, capture pixels, allocations, CPU frame cost, and GPU capture cost.
Use target-device results for GPU conclusions. Performance changes must not
reduce physical correctness or add a separate controller path.

### Draft status

The tested foundation is merged into main with the Android profiling and asset
loading work. No complete PSIM acceptance slice is closed.

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
  Each movement policy derives world units for its step and then restores the
  authored parameters. World gravity and body time do not change with scale.
- The body kernel validates a private candidate, its sweep and mapped part
  velocities before commit. Rejection retains body state and clears stale
  travelled paths. Both controller adapters use the same admission input.
- The common transit result includes the validated similarity, both apertures,
  stable directed channels and actual entry, mapped and assisted exit velocities.
  Both body storage adapters validate carried acceleration before changing body
  state. The same transfer commits carried run. The separate runtime adapters
  that reconstructed rigid maps from normals are removed.
- Carried acceleration is a shared body capability. Projectile construction,
  stepping, snapshots and restoration use that value. World gravity and force
  zones keep their own contributions. The rollback schema is 426.
- Source admission checks carried acceleration along each path. It does not
  map world gravity or world force fields. Projectile flight validates a
  candidate before it commits motion or contacts. Refusal clears stale travel
  and keeps the prior pose; lifetime continues. The same shot resumes after
  its budget is restored.
- An approaching body prepares both host openings for its own use. A fast
  crossing can skip the begin latch and still advance from the exit without
  meeting that exit's uncut host. This applies to both body storage types.
- The material cast query partitions a swept spatial domain and clips each
  physical obstacle in that domain. It casts the actual primitive, keeps
  circles analytic and returns the source collider and directed path.
  The query covers translation against a room snapshot. It is not yet the
  live body solver. Moving obstacles and physical righting remain open.
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

`cargo check -p ambition_app` passes on the ray change.
Library tests pass: 755 core, 815 combat, 350 shared-state, 139 portal and
517 renderer tests. Five tests remain ignored across those crates.
The portal content ray test passes with the `portal` feature. Removing the
ray's scale conversion makes the nearby-exit target assertion fail.
The ray integration passes the assembled eye-laser test and the 26 tests
selected by `portal`.
The Smash host passes 71 tests, with six tests ignored. The 35 workspace
policy tests pass. An angled exit-host regression fails before the exit point
is constrained to its aperture plane. The final correction passes the core
library, portal content ray test and app compile check.
The preceding transfer work also passed geometry, projectile, runtime and
presentation tests and the workspace all-targets check. These results do not
close the full replay, visual or performance acceptance checks.

Finish source admission for free-body sweeps, externally owned poses and
moving aperture hosts. Audit projectile contact responses and the remaining attack updates.
Body-attached ray origins still need the common material pose pass. Ray wall
clipping still uses its centre line; finite-width contact remains open.
Check retained support facts, curved paths, control-phase displacement,
topology changes after the aperture snapshot and candidate-copy cost.
The existing gates do not close PSIM-1B.

Live root transit and righting, common exact collision/contact projections
and the unequal-aperture switch remain open.
GNU-ton acceptance and numerical-fault presentation remain open. Keep the
minimum-opening policy in production until the coordinated switch. Measure
allocation and frame cost on the completed production paths.
