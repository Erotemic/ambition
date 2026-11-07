# Portal parts: exact partitions, similarity transforms and numerical failures

**Status:** MAINTAINER-APPROVED DESIGN, IMPLEMENTATION OPEN (2026-11-06).
**Execution:** [PORTAL-SIMILARITY](../queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals).
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
