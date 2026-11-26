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
  velocity and spin in shared simulation seconds. Some motion callers use
  room-effective seconds instead; the conversion required below is not yet
  established by this adapter. Readers do not infer rates from differences
  between observed portal poses.
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
`crates/ambition_platformer2d_runtime/src/portal_world/hosts.rs`. Existing arc, full-winding,
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

**Landed (2026-11-25): generic transit continues from the crossing.**
`PortalTransfer::at_crossing_completed` maps the root where it crossed
(`M_tau(x_tau)`), sends its remaining travel relative to the entry surface
through the map's linear part, and adds the exit surface point's own travel
for the rest of the tick. Each surface point moves at its published step's
per-tick rate (`shift + turn x arm`), so a translating exit is exact and a
turning aperture releases the body straight rather than along the turn.
`PortalCrossing` carries both steps. A static pair still gives
`M_tau(x(T))`. Probe (deleted): a body crossing at `tau = 0.5` into an exit
that moves 40 px this tick ends where a still exit at the end pose puts it
(x 500, not the old 480).
`a_full_turn_takes_a_still_root_at_the_crossing_pose` now asserts the
released root's straight travel at its mapped velocity, not the exit plane.
Not covered: the kernel still collapses the step at the crossing (M2's
continuation in the destination chart, with its contacts, and two crossings
in one tick); exit-speed assistance changes velocity, not this tick's
travel; the item/projectile road (`prepare_body_crossing`) is unchanged.

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

**Landed (2026-11-25): contact reads the travel before the crossing.**
Generic transit records the tick's travel up to the root crossing
(`ae::CrossedTravel`: the body's sample cut at the crossing point and
world time, in the chart it left). `clear_crossed_travel` runs before
transit every tick, so no reader sees an older tick's, and the record is
declared rollback-derived (`portal.crossed_travel`). `swept_body_contact`
queries that prefix over `[t0, tau]` and the arrival record only over
`[tau, 1]`. Probe (deleted): a shot where the body was before it crossed now
hits (t 0.15, was a miss); a shot at the exit before the crossing no longer
hits (was a hit at t 0); a later one at the exit hits from t 0.5. The
strike-victim road (projectiles) passes the record; the boss feature road,
held items and the pose-held/kernel-collapsed `reconcile_transit` seed are
unchanged, and the next-step seed still restarts at the arrival.

**Landed (2026-11-25): generic transit takes every root crossing the
budget admits.** After a transfer, the rest of the tick is a straight leg in
the exit's chart (from the mapped crossing point at `tau` to the transfer's
end); `continue_after_crossing` searches that leg for the next root crossing
(not the exit just left) and composes the transfers: one map, the last exit
pair, the input warp of each hop, the final position and velocity. The
hop budget is the portal geometry's (`PortalGeometry::budget().max_hops`), so
the default single hop runs no extra search. Witness
`placement/tests.rs::a_body_crosses_two_pairs_in_one_tick`: the pairs above,
under a composed budget, end at x 320 with the entry velocity; under single
hop the same travel ends at 170 through A only. The kernel still integrates
the whole tick in the entry chart (its material collision already threads
both cuts).

**Landed (2026-11-25): contact reads the legs between crossings.** A chained
transfer keeps each leg it searched, cut at the next crossing
(`TransitStep::Transfer::between`), and `CrossedTravel` holds every leg
before the arrival: the one in the chart the body left, then each between
two crossings. `swept_body_contact` queries each over its own interval and
the arrival from the last crossing on. In the two-pair witness the middle
leg runs from x 100 at tau 0.125 to x 150 at tau 0.75, in A's exit chart;
a single crossing records none.

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

**Landed (2026-11-25): transit keeps the accepted restriction.** A probe of
`transit_step_with_tuning` over the fixture's recovered travel (the same
mouth, the body from y 465 to 600) returned `Transfer` to the exit: the
crossing the kernel refused at `HopLimit` was taken by generic transit.
`SweepSample` now carries `restriction` (`PortalRestriction::Open`,
`Straddled` or `Closed`). The kernel writes `Closed` when it recovers with
every portal closed and `Straddled` when it keeps only the straddled
pairs. Transit reads it: closed is no portal, straddled is only the pair of
the body's `PortalTransit`. Under `Closed` the same probe is `Idle`, and the
fixture's sweep records `Closed`. The field rides in `body.sweep_sample`'s
codec (schema 545). This closes the override; M1's accepted completed-step
history apart from the next-step seed is still open.

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

The geometric moving-frame law already exists in `frame/material_cast/payload.rs:11–31`
and 81–99 (`ContactVelocityFrame`) and `world.rs:1294–1328`
(`MaterialCollider::velocity_in_root`, `velocity_at`). Loose-item response
uses it in `held_items/src/motion.rs:263–288`. Reuse this authority, but first
give its input rates the same derivative clock. The existing helpers do
not establish that unit conversion for their callers.

Receipt (M3, axis filter, 2026-11-25): the axis contact filter now measures
the material point's travel with `MotionAdmission::carry_displacement`
(`frame/thread.rs`). For each link on the hit path it subtracts the entry
surface point's per-tick travel, maps the rest through the link, and adds
the exit surface point's per-tick travel (`ApertureStep::point_travel`,
shared with the transfer's surface rates in `portal2d/src/placement.rs`).
Every term is per tick, the same unit as `delta`, so the example above now
reads `+2` and is kept. A hit with no portal on its path still uses
`hit.map.vector(delta)`. The witness is
`a_moving_exit_carries_its_drift_into_the_contact_frame`.

Receipt (M3, surface carry, 2026-11-25): the swept contact's carry and the
reported `Contact::surface_velocity`, at the sweep and at repair, now use
`surface_step_in_body_chart` (`movement/collision.rs`). That function pulls
the obstacle's per-tick step back with `MotionAdmission::uncarry_displacement`,
the inverse of the carry above. When an exit drifts `+3` into a still wall,
the wall appears to approach the body by 3 in the body's chart, so the carry
holds the body's image on the wall face. Under the still map, the step was 0
and the image sank in by the drift. The witness checks that the two carries
round-trip.

The clock question, read at the writer: one system,
`platformer2d_runtime/src/portal_world/hosts.rs:150-159`, writes a hosted
portal's per-second `vel`/`spin` and its per-tick `step`. Both come from
the same host `shift`/`turn`, scaled by `1/dt` of the simulation tick.
`ApertureStep::sample` keeps those rates across the tick, and
`MaterialCollider::velocity_at` divides the same per-tick displacement by
the caller's `dt`. So the per-second law (`carry_velocity`,
`ContactVelocityFrame`) and the per-tick law (`carry_displacement`) agree
whenever the reader's `dt` is the tick's. The contact comparisons against
`delta` now use the per-tick law, so they need no `dt` at all. Impact speed
reads the body's own velocity against the source normal, as it does with no
portal.

Receipt (M3, still body, 2026-11-25): `sweep_player_axis_clusters` skips
the sweep for a zero axis delta only while `geometry_moves_near` finds no
moving source within reach. That means no block with a velocity, no moving
line other than the body's own carrier, and no moving portal link whose
swept bounds reach the body. Otherwise the still body is swept like a moving
one. The witness `a_still_body_is_met_by_a_wall_that_sweeps_across_it_in_one_tick`
uses an 8 px wall that crosses the body inside one tick and carries it to
the far side. The control (the old unconditional skip) leaves the body at
x 195–225 with the wall at 229–237. The skip remains for still geometry,
which is the common case. The gate costs one grid query over the region
repair already reads.

Receipt (M3, spinning obstacle, 2026-11-25): `MaterialCollider::displacement_at`
gave a rigid line only its translation (`chain.velocity`) and dropped its
`turn`. A bar spinning about a still pivot therefore read as still: its
contacts failed the axis filter as separating, and it tunnelled through a
still body. Its point now also goes round the pivot, at the per-tick rate of
the same law `velocity_at` applies per second. The witness is
`a_still_body_is_met_by_a_bar_that_spins_across_it_in_one_tick` (movement
`slopes` tests). Before the turn term, the body stayed wholly on the bar's
start side. The swinging and lifting plank riders are unchanged.

Receipt (M3, whole step, 2026-11-25): through `step_motion` with no gravity,
a body hangs half through a floor mouth while the exit drifts 3 px a tick
toward a still wall beside the half that is through
(`a_drifting_exit_holds_the_material_through_it_at_a_still_wall`, movement
`wall_collision` tests). Each tick the body is admitted, and it still
straddles the mouth. The far half's edge on the exit side reaches the wall
face and stays there for the rest of the run (x 625.00 with the wall at 625);
the root moves 3 px a tick to pay for the drift. In the control without the
wall, the same drift carries that edge more than 20 px past the face.

Receipt (M3, paced rooms, 2026-11-25): `MaterialCollider::velocity_in_root`,
the surface rate that loose items, projectiles and surface momentum compare
with their own stored velocity, now uses the per-tick law of the kernel's
contact. It takes `step_in_root` (the material point's travel in the tick,
with each mouth's travel taken out by `uncarry_displacement`) and divides it
by the caller's own `dt`. A step has no clock, so dividing by the room's
effective `dt` gives the surface's speed on that room's clock. The old body
mixed two clocks: the surface step over the room's `dt`, plus the mouths'
per-shared-second `velocity`. The witness is
`a_co_moving_mouth_and_body_have_no_relative_speed_at_any_room_pace`. A mouth
hosted at full pace drifts 0.5 a shared tick, and a body in a room at pace 1
or 0.5 keeps up with it. On the body's clock the still block seen through the
mouth moves at exactly the body's stored velocity at both paces, while the
per-second law reads the shared-clock rate (half the body's speed at half
pace). At pace 0 there is no clock to state a rate on: `velocity_in_root`
refuses and divides by no epsilon, and the step still reports the mouth's
travel. Held items keep their frozen-body policy (a frozen item skips its
tick). At pace 1 the two laws agree, as the clock reading above showed.

The guard's deflect chart (`ambition_combat::guard::DeflectChart`) mixed
the clocks the same way. It subtracted the per-shared-second
`uncarry_velocity` from a body's stored velocity. It now keeps the
stationary root step (`uncarry_displacement`) and reads it on each body's own
clock, the `dt` that `tick_guard_states` already uses (proper time scale,
dilation, room freeze). Two tests had modelled a moving mouth by its
per-second fields alone (frame `velocity`/`spin` with fixed steps), a pair
no host publishes. They now give the matching steps, as
`portal_world::hosts` does, and keep their expected values, because the laws
agree at pace 1: the far-side projectile bounce still gives −26.5, and the
guard's portal rebound tests pass unchanged.

Still open in M3:
- SUP1's changing-support input through a portal. Without portals the
  kernel's contact already reads SUP1's per-point travel (`displacement_at`
  on a deforming line, landed with SUP1). A cell cut by a portal still reads
  the published rigid fit, because the deforming cast does not admit portal
  cells. That is the portal case SUP1's page leaves open under P2/M1/M2.

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

`crates/ambition_platformer2d_runtime/src/portal_world/hosts.rs::refresh_hosted_portal_frames`
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

For a changing support admitted as a portal host, the proposal must also
name the same material place. Current `PortalHost::Line` stores world arc
distance: `attribute_line` records it and `point_on_line` reuses it on the
new chain, clamping at the end. On a line stretching from `[0,100]` to
`[0,200]`, an attachment at material `u = 0.75` must move from x=75 to x=150;
the current distance lookup stays at x=75. Atomic admission alone cannot
repair that input.

Bind that host center to SUP1's stable surface occurrence and source place,
and derive its position, normal and point-motion interval from that place.
The point rate must include local shape motion; a midpoint rate is not
enough. Keep the proven rigid host path. Until the changing-host binding
and trajectory exist, do not admit that host capability through a rigid
fallback. Material attachment does not decide whether opening width or
lift scales: keep the selected portal sizing and lift policy explicit.
If the opening no longer fits, use admission/refusal or a separately
authorized close, never slide it to another material place. This is a
conditional SUP1/P2 obligation; local SUP1 can land first.

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

### One chronology does not give rates the same unit

Static review at `6d100f9d9e0d` exposes a present boundary:
[platforms advance](../../../crates/ambition_platformer2d_world/src/platforms/simulation.rs)
with `WorldTime::sim_dt() * RoomFreeze::scale_in(room)`, while the
[host adapter](../../../crates/ambition_platformer2d_runtime/src/portal_world/hosts.rs)
divides their displacement by unscaled `sim_dt()` for aperture velocity.
[Loose-item flight](../../../crates/ambition_held_items/src/motion.rs)
integrates stored velocity with room-effective `dt` and also passes that
duration to `MaterialCollider::velocity_in_root`. The latter divides surface
displacement by its caller's duration before combining it with aperture
drift. A normalized contact time in `[0,1]` does not make these rates
compatible. This boundary matters with several live rooms, where
[`RoomFreeze`](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/room_pace.rs)
keeps room pace separate from the shared clock.

For one shared second at room pace `0.5`, a stored native body speed of
100 produces travel of 50. A co-moving hosted aperture also travels 50,
but publishes speed 50 per shared second. Their common-clock relative
speed is zero; subtracting the raw stored speed and aperture rate invents
a relative speed of 50. This is an algebraic contact witness, not a claim
that co-moving roots spontaneously cross.

Retain each native phase/effective-time mapping on the accepted interval.
Before combining body, support, imposed-turn and aperture rates, express
each as a derivative with respect to one named comparison clock `T`.
For a constant native pace `alpha = dt_native/dT`, its local contribution
is `v_T = alpha * v_native`. Use the accepted trajectory's derivative for
the full point motion: external carry and changing support motion remain
separate contributions. Do not multiply an already resolved shared-clock
rate by room pace again. Normalized TOI remains an interval coordinate.

The native owner converts an accepted response back to its stored units
only where that conversion is defined. At zero native pace, no epsilon
division can recover a native rate from external carry. Preserve the
owner's frozen-body policy and stored continuation; a stopped local clock
does not by itself forbid its support from carrying the body. A zero-duration
shared interval is a stationary sample or an admitted discontinuity, not
nonzero continuous travel. If root transfer changes the native clock, its
owner must state whether native control speed or common-clock motion is
preserved; a coordinate rebase must not silently choose that product rule.

This correction preserves native action clocks, room pace and integration
policy. It needs no clock registry or global scheduler. Keep an identity-pace
direct path and the existing static/affine geometry specializations.

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
the existing moving-frame law with rates in the same comparison clock.
Witness an exit driving material into a stationary wall and a spinning
obstacle with a stationary pivot. Preserve
one-way rejection followed by a later valid approach. Delete local velocity
reconstructions. The delivery plan owns canonical rim contact and removal
of the separate AABB snag position writer.

Include the co-moving body/aperture witness above at pace one and half pace
with two live rooms. Common-clock relative velocity must stay zero, while
native stored speed retains its declared meaning. Exercise zero pace under
the existing frozen-body policy, not an invented divide-by-epsilon rule.

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

**Landed (2026-11-25): an interpolated view crosses at the crossing time.**
An interpolated view (`CastFramingTuning::interpolate_poses`, Smash's)
drew a body that a pair carried at its exit for the whole presented tick,
so the chart change showed before the body had crossed. The host now
declares a single crossing in the last tick with its projection
(`PresentedCrossing`), built from the body's `CrossedTravel` leg and the
transfer's map:
- the leg up to the crossing, in the chart the body left, over
  `[start, tau]`;
- the crossing point carried by the map (`M_tau(x_tau)`), from which the body
  travelled straight to its end pose over `[tau, 1]`, as generic transit
  moved it.

`PresentedPose` draws the entry side before `tau` and the exit side from
`tau`. The witness
(`interpolated_draws_a_crossing_on_its_side_of_the_crossing_time`) checks
x 5 at a quarter of the tick, the mapped point at `tau`, and x 505 after it.
Its control, a plain warp, parks at the exit.

A chain is drawn leg by leg too (same day): `PortalTransfer` carries where
its last straight leg begins (`arrival_point`, `arrival_time`: the last
crossing point carried by that hop's map), and `PresentedCrossing` holds up
to `MAX_CROSSING_LEGS` (4) legs. The two-pair placement witness now asserts
the chain's arrival (x 300 at 0.75) and the single hop's (x 100 at 0.125).
`interpolated_draws_a_chain_of_crossings_leg_by_leg` draws x −5, x 125 (in
A's exit chart) and x 310 at those quarter-tick times.

What still parks at the exit:
- a crossing in an earlier tick of a multi-tick frame (it left no
  `CrossedTravel`);
- a push that spans more than one tick;
- a chain longer than four legs.

Extrapolated views draw past the tick's end, so the crossing is behind
them. Only position follows the crossing; the body's turn and material still
change with the tick's pose. No game run was made: the projection is unit
tested, and the host builds it from fields the M1/M2 receipts measured.
Still open in M4:
- an ordered crossing projection for views spanning several ticks, with
  per-observer cursors and rollback invalidation;
- cost measurement on single-leg and chained traffic.
