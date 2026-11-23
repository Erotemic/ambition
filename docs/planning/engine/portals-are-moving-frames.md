# Portals are moving frames: one accepted motion, several charts

**Status:** Initial frame work built 2026-10-15; the motion unification below
is **planned**, following source review at
`b91b29d359358ea417353afce4f472dc7fab15ec` on 2026-11-21.
**Delivery owner:** [portal parts, similarity and chirality](portal-parts-similarity-and-chirality.md),
executed through [PORTAL-SIMILARITY](../queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals).
This page specifies the motion contract and its migration packets. It adds
no independent queue or general-purpose physics framework.

The new findings are source and mathematical counterexamples. No new game
run, build or runtime reproduction was performed for this review. Existing
tests establish their stated cases; they do not prove the stronger composed
cases below. Preserve the implemented material and moving-frame mathematics.

## Implemented foundation to preserve

- The host publishes displacement and turn; its adapter supplies linear
  velocity and spin in simulation seconds. Readers do not infer either from
  differences between observed portal poses.
- `PlacedPortal.step` retains the continuous start and host travel.
  `ApertureStep` samples translation and rotation, including full winding.
  Matching start/end normals do not imply a stationary aperture.
- Explicit placement and host wrapping are discontinuities. They do not
  invent a sweep across the intervening room. The existing closure eviction
  policy remains distinct from ordinary continuous contact.
- Directed links carry both endpoint steps. `MaterialCast` and its prefix
  retain the original world-tick interval, source material and attempted
  curve. Temporal cells retain material present only between endpoint poses.
- Static/translation specializations and curved motion share the canonical
  material contract. Preserve the cheaper static and affine solvers.
- Projectile and loose-item flight already spend remaining time after root
  crossings. They share prepared root transfer and commit primitives.

The source owners are `platformer2d_core::frame::{motion,crossing,material_cast}`,
`portal2d::{placement,transit}`, and
`game/ambition_content/src/portal/host_adapter.rs`. Existing arc, full-winding,
common-boost, hosted-platform, spinning-plank and wrap cases remain useful.
`a_moving_portal_is_carved_where_it_is` establishes the published host pose.

## The distinction that must become explicit

A body has **current root state** and an **accepted path during this tick**.
A portal rebase changes the coordinates used for the current root. It must
preserve the path's physical events and their times. A later contact reader
still needs the portions before and after the rebase.

For an instantaneous map `M` and part maps `P_i`, changing root material
`R` to `M R` and part paths to `P_i M^-1` leaves `P_i R` unchanged.
That identity holds at the crossing instant. It does not authorize mapping
an already completed tick through a map sampled earlier in that tick.

The same distinction separates a **candidate** from its **accepted operation**.
An operation accepted with a pair closed cannot later acquire a crossing
through that pair because another system asks the original room topology.
The accepted trajectory, restriction snapshot and root changes belong to
one operation, with one owner deciding when its writes become authoritative.

## Source evidence and counterexamples

### A moving exit exposes the endpoint-remap error

`crates/ambition_portal2d/src/placement.rs:315–324`,
`PortalTransfer::at_crossing`, samples the entry and exit at crossing time,
then maps its `center` argument through that instantaneous map.
`swept_crossing_step` at 399–419 passes the completed body's center.
Generic `portal_transit` commits that one map after integration
(`crates/ambition_portal2d/src/transit.rs:542–575`).

Let entry be fixed, exit translate at velocity `u`, the map's linear part
be `L`, crossing time be `tau`, and tick end be `T`. With constant root
velocity `v` and no later response, the completed exit trajectory is

`M_tau(x(tau)) + (L v + u) (T - tau)`.

Mapping the completed entry-chart endpoint instead produces

`M_tau(x(T)) = M_tau(x(tau)) + L v (T - tau)`.

The missing displacement is `u (T - tau)`. A change caused by exit-speed
assistance also changes remaining displacement, even for static apertures.
Rotation requires continuation from the event; a translation correction
added at the final endpoint would not solve the general problem.

The November 12 kernel receipt in the delivery plan measured a static
translation pair. Preserve that result and narrow its completion claim.
`placement/tests.rs::a_moving_portal_scoops_a_stationary_body` checks transfer
and outward velocity, while `a_full_turn_takes_a_still_root_at_the_crossing_pose`
checks the instantaneous crossing pose. Neither establishes residual motion.

### A chart change erases the target trajectory before projectile contact

`platformer2d_core/src/movement/authority.rs:172–202`, `reconcile_transit`,
collapses `SweepSample` to the final pose over `[0,1]`.
The runtime orders PlayerSimulation before Combat
(`platformer2d_runtime/src/schedule.rs:83–95`); portal transit is in the former
and projectile stepping is CombatSet::Materialize
(`portal_schedule.rs:132–144`, `combat_schedule.rs:456–468`).

`combat/src/components/features.rs:309–355`, `swept_body_contact`, accepts
that replacement as the target's path. Thus a crossing target is represented
at its final exit pose for the whole projectile query. Pre-crossing contact
can disappear, and a query can see the target before it physically arrived.
The input substitution is visible in source; a specific hit/miss still needs
an integrated witness. Ordinary moving-target CCD tests do not cover it.

Generic transit also commits only the first root crossing. With aligned
static pairs A: x=0 to 100 and B: x=150 to 300, a small body travelling
from x=-10 to x=70 in its original chart should finish at x=320 after
both crossings. One final map through A produces x=170. Canonical cells
can describe both cuts; generic root state and its collapsed record cannot.
Use a budget admitting two crossings when witnessing this case.

### Refusal recovery can be overridden by later crossing discovery

`platformer2d_core/src/movement/kernel.rs:182–198` retries a refused motion
with only occupied links, or with every link closed. The accepted fallback
retains the original refusal for diagnosis.
`movement/kernel/tests.rs:1775–1806`,
`a_motion_refusal_steps_with_portals_closed_and_does_not_replay_an_old_sweep`,
sets `max_hops=0` and an unbacked aperture. It correctly records `HopLimit`,
then lets the body fall beyond the closed mouth into empty space, with a
fresh sweep and unchanged material. Its comment states this recovery policy.

Generic `portal_transit` later collects all room portals and reads that sweep
(`transit.rs:476–531`). It has no accepted-link context or body budget input.
Its `PreparedPortalTransfer::new` checks finite body/material payloads, while
the stronger `prepare_body_crossing` at 377–409 checks link membership, hop
budget and rebased occupancy for item/projectile owners. The kernel fixture
stops before this generic phase, which can rediscover the refused crossing.
Extend that existing fixture through transit to expose the composed result.

### Exact CCD is followed by an incomplete velocity rule

`platformer2d_core/src/movement/collision.rs:149–152` filters native contact
using `hit.map.vector(delta) - collider.displacement()`. Its surface carry
and response at 203–236 also use inverse-vector maps of displacement.
This omits moving-portal drift and a rotating surface's point velocity.

For mapped root displacement `-1` along x, exit drift `+3`, and a stationary
obstacle with normal `-X`, physical motion is `+2` and approaches the obstacle.
The filter sees `-1` and rejects the native hit as separating. This is an
algebraic sign reversal, not a reported runtime tunnel. Later repair would
still fail the intended contact-at-TOI contract.

The required law already exists in `frame/material_cast/payload.rs:11–31`
and 81–99 (`ContactVelocityFrame`) and `world.rs:1294–1328`
(`MaterialCollider::velocity_in_root`, `velocity_at`). Loose-item response
uses it in `held_items/src/motion.rs:263–288`. Reuse this authority.

## Required motion operation

The existing motion owner prepares a bounded sequence of accepted legs.
Each leg retains its interval, departure material, original source curve,
accepted restriction snapshot and contact/crossing outcome. Existing
`SweepSample` and `LinearTravel::Prefix` supply the single-leg primitives.
Keep the no-crossing single-leg case cheap.

At a crossing, end the current prefix at its accepted point and time,
prepare the checked root rebase and its material/payload changes, and
continue the remaining interval in the destination chart. Multiple roots
use the same time and budget accounting. A pose-owned limb remains in its
owner's chart; its carried pieces do not independently transfer the owner.

Keep the completed tick's full path until its simulation contact/publication
readers finish; retain its compact crossing projection separately for views
(M4). Next-step initialization must preserve that history. Explicit relocation
records a discontinuity and the real travel preceding it; it must not draw
a physical segment across the relocation gap.

Full path means compact source/interval/relation provenance and accepted
outcomes, not stored copies of every solver cell. Keep a single leg inline;
share heavy prepared query values only through their exact reader cut. World,
recipient and payload consumers reuse one prepared leg where their meanings
agree, including the root-search result when no crossing was found. Prefix
reuse must preserve the original clock and admit an earlier valid prefix
without requiring a refused suffix. The delivery plan owns the
[preparation and specialization contract](portal-parts-similarity-and-chirality.md#reuse-preparation-and-specialize-execution).

Bind history to the body occurrence, simulation tick/rollback generation and
accepted aperture snapshot. Store the maps or stable directed identities
needed to interpret it without resolving against a later topology. The
delivery plan's topology owner defines those identities and edit lifetime.
Do not persist array positions as cross-tick or rollback identity.
An aperture born or edited after integration has an explicit effective cut.
Earlier legs and full-tick contact queries use the topology valid during
their intervals; a new aperture cannot retroactively exist for the whole tick.

Commit root state, material, accepted path and crossing publication together
at the motion owner's declared commit cut. Refused candidate legs publish
no crossing or unaccepted contact consequence. Previously accepted prefixes
may remain where the owner's existing policy permits; a refusal must not
silently undo their results or publish a successful later leg. Lifetime/age
advancement follows its explicit owner clock.

The later portal phase projects accepted overlap, input and cue facts. It
does not discover a new physical trajectory from completed endpoints or
reopen links excluded by the accepted fallback. Keep the existing playable
refusal recovery and explicit portal-close eviction policy. A numerical
failure does not itself prove a physical wall or authorize a shared topology
edit. A named narrowed recovery is per operation, retained with its accepted
history; its cost containment and meaning survive the migration.

## Driven aperture motion is admitted before the host commits

Body motion is not the only source that changes material occupancy. A moving
host can translate or turn a portal into an invalid placement for material
already through it. Keeping the same connection incarnation does not prove
that its new trajectory is admissible.

`game/ambition_content/src/portal/host_adapter.rs::refresh_hosted_portal_frames`
currently consumes published host motion and directly writes the aperture
pose, velocity and step. It does not return material admission to the host.
Move the proposal/admission boundary before that host writer commits. The
material owner checks the proposed host/aperture trajectory and affected
occupancy; the host's motion law consumes its accepted result or refusal.
Commit the accepted host, aperture and material together.

On refusal, retain a valid combined state or accepted prefix, or use an
explicitly admitted close/eviction policy. Never move the host and silently
freeze or detach just its portal. Ordinary obstacle response and portal-close
eviction remain distinct. This constraint applies to the existing host
motion owner; it does not require a new universal force solver.

## Shared kinematics, distinct movement policy

At a contact point and time, velocity obeys
`v_physical = L v_source + drift(point,time)`.
Entry/exit surface motion and imposed source turn contribute to drift.
Use the same checked contact witness for approach eligibility, impact speed,
surface carry and response. Unit directions, normals and carried vectors
retain their own transformation laws; drift is not added to a direction.

World-selected gravity direction and explicit world fields retain their
meaning. The [scale-relative gameplay contract](scale-relative-gameplay.md)
converts canonical body gravity response once before publishing a resolved
world frame. Body-carried acceleration is converted at creation or chart
transfer, not multiplied by cumulative scale on each read. Preserve each
movement owner's declared
integration law: the current projectile age and force kick happen once per
tick. A leg loop must not apply the full force kick, timers or ability ticks
again. If a policy samples forces between legs, it states those sample cuts
and integrates only their actual remaining duration. No global force solver
or analytic-trajectory mandate is introduced by this plan.

Axis stopping, crawler attachment, surface momentum, item bounce and
projectile interception keep distinct response policy and private state.
They share event chronology, material validity and contact-frame kinematics.
Keep `step_motion` as the kernel gateway. Do not add player/AI dispatch or
force all bodies through one integrator law.

## Bounded delivery packets

### M1 — Preserve accepted history and restriction context

Expose the completed-step history separately from the next-step seed, using
the existing record/prefix primitives. Bind narrowed refusal recovery to
that accepted operation. Carry source-frame contact facts through readers.
Extend the max-hops-zero fixture through real transit: the recovered body
must not transfer or emit a crossing. Its admitted control still crosses.
Delete generic transit's authority to override the accepted restriction set.

### M2 — Continue the kernel at its actual chart-change events

Use the prepared prefix/rebase primitives inside the kernel operation.
Adopt the same history in established item/projectile motion while keeping
their response, force and contact-budget rules. Witness a mid-tick moving
exit and two static root crossings, with exact final position and velocity.
Delete completed-endpoint remapping and chart-change history collapse.
Include a hosted aperture whose proposed motion makes carried material
overlap a solid. Admission must reach the host before commit; the refused
control preserves the combined state and never leaves the aperture behind.

### M3 — Resolve contacts through the complete velocity witness

Migrate the axis contact filter, surface carry and impact calculation to
the existing moving-frame law. Witness an exit driving material into a
stationary wall and a spinning obstacle with a stationary pivot. Preserve
one-way rejection followed by a later valid approach. Delete local velocity
reconstructions. The delivery plan owns canonical rim contact and removal
of the separate AABB snag position writer.

Include a stationary body and an admitted moving obstacle. A zero source
axis delta cannot select endpoint-only repair unless the relative geometry
is static under the declared policy. The current `collision.rs` early
return loses that distinction. Keep frozen-body/hitlag clock policy separate.

[SUP1](slopes-feet-ride-surfaces.md#sup1--keep-a-changing-supports-material-motion)
supplies the changing support's source motion. A complete chart-velocity
formula cannot recover scale or local material motion discarded by the
producer. Consume SUP1's point/time/source-location witness at the same
accepted interval; preserve ordinary rigid contact work in parallel.

### M4 — Consume one path in contact and presentation

Use retained target legs for projectile contact before and after a target's
root crossing. Publish a compact ordered crossing projection for views that
may span several simulation ticks. Each observer consumes events through its
own cursor/cut, selected through the same presented body pose, time and branch;
interpolation must not apply a chart change before or after that pose crosses.
Rollback/branch changes and observer retirement explicitly invalidate cursors.
Bound projection retention with explicit history cuts; a lagging observer
rebases at a declared compatible cut. Full collision-query history need not
remain resident for views. Keep gameplay budgets in simulation and exercise
these composed cases with the existing replay tools.

Measure cost on unchanged single-leg traffic and moving/chained portal
traffic. Reuse prepared aperture/scene and contact data within an operation;
avoid reconstructing searches merely to recover their velocity witnesses.
Keep static/affine dispatch and conservative candidate selection. No speedup
is asserted here. CPU simulation and GPU capture cost remain separate.

Completion means root state, material, accepted path, restriction decisions
and physical contact agree on one tick's events for every mover policy.
Passing isolated map or crossing tests alone does not establish that result.
