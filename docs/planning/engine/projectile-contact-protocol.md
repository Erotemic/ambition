# Projectile contacts: geometry, ordering and resolved identity

**State:** A2 is built. A projectile selects one contact from its swept travel
leg, compares targets and walls by one time of impact, and names its recipient
once. Compound contacts (ruling Q96) are built. The open items are the
moving-target limit of this slice and an optional file move (see
[Open work](#open-work)).

**Scope:** the same-build deterministic simulation, in every live room. A shot
flies in its own live room (`ProjectileCollisionWorld::shot_room`: its own
stamp, else its firer's room). That room supplies its collision world
(`solids_in`) and its candidate targets. A shot does not reach a body or a
portal in another live room. The [combat model](combat-model.md) owns combat
policy and the [collision plan](collision-and-ccd.md) owns geometric
primitives. This page owns how they join for projectiles, including the timing
limits.

## Decision

A projectile must select a contact from its actual movement segment and the
published simulation geometry, account for earlier blocking surfaces, and name
the selected recipient once. Flight response, attack reception and encounter or
object consequences remain different owners. A damage refusal does not erase a
physical contact retroactively.

Preserve the phase split: immediate projectile interception happens while
stepping; ordinary hit reception happens in Combat Resolve. Making every
reaction synchronous would change guard, damage, move-confirmation and effect
ordering merely to simplify an interface. Delaying reflection or absorption
until the next frame would allow an already stopped shot to hit again.

Direct contacts do not re-query features. An area attack intentionally
enumerates several recipients and remains a separate operation. Do not use an
area event as an approximation to one touching projectile.

## Owners and allowed knowledge

| Owner | Owns | Does not own |
| --- | --- | --- |
| Projectile flight | Integration, travel segments, return-leg state, per-leg hit memory, interception response and termination | Boss catalogs, rewards, breakable trigger rules, health reduction |
| Spatial geometry | Shape/segment intersection, blocking surfaces, stable contributor identity and contact ordering inputs | Damage, factions, guards, whether a boss can lose HP |
| Target publisher | Current hurt geometry and source-specific contact eligibility derived from its real state | Projectile lifetime or duplicate damage application |
| Interception | Atomic read/mutation of the victim's current guard catch and the immediate consume/reflect result | Later broad victim discovery |
| Hit reception | Selected recipient's health/guard/armor/phase policy, damage and knockback outcome | Another scan to choose a different recipient |
| Object/encounter owner | Broken/respawn/death-outro transitions and policy-specific consequences | Re-running projectile collision |
| Presentation and game rules | Cues, score/rewards from accepted outcomes | A second authoritative damage or destruction path |

An integration module can depend directly on these owners. No function-pointer
registry is needed to ask a boss whether a shot arrived. Generic flight helpers
live in `ambition_projectiles`. Combat contact orchestration stays in the
monolith projectile region (`projectile/systems.rs`) until its inputs are
independent of feature internals.

## Current shape

| Concern | Where | Witness |
| --- | --- | --- |
| Boss hurt geometry is computed once, by its publisher | Bosses join the damage-facing barrier `DamageFacingVolumesPublished` (after `Playback`, before `Resolve`). `apply_boss_hit` and the projectile stepper do not derive geometry and do not read `BossCatalog` | `a_boss_is_reached_only_through_its_published_volumes` (four states, including an anti-vacuity row) |
| Breakable geometry | `CenteredAabb` plus broken state, both settled in `WorldPrep`. Breakables are not in the barrier, because a second publication would change nothing | — |
| One travel leg | The pre-integration position is captured once. Nothing derives the start as `pos - vel * dt` | — |
| One obstruction model | One `body_sweep` over the leg, with the shot's own box and its own `WorldHitPolicy` | `a_one_way_blocks_the_shot_whose_policy_says_it_should_and_no_other`, `a_shot_clipping_a_block_corner_along_its_path_hits_it` |
| Swept body contact | `ambition_combat::hitbox::swept_strike_reaches_victim`. Candidates are ordered by time of impact, not by distance to a centre | `a_fast_shot_hits_a_thin_body_it_crosses_within_one_tick`, `a_wall_inside_a_wide_body_past_its_near_face_does_not_stop_the_hit` |
| Swept feature contact | `projectile_reaches_breakable` and `projectile_reaches_boss` return a `FeatureContact` (time, centre, entity). The boss sweep reuses `swept_strike_reaches_victim` | `a_fast_shot_breaks_a_thin_crate_it_crosses_within_one_tick` |
| Bodies, features and walls compared together | The feature candidate is resolved beside the bodies. The earliest contact wins, whatever its family | `the_earliest_contact_wins_whether_it_is_a_body_or_a_crate`, `a_crate_behind_a_wall_is_not_broken_and_one_in_front_of_it_is`, `a_shot_does_not_damage_a_victim_standing_behind_a_wall` |
| Total order | `FeatureContact::order_for_caller`: time, then position, then authored identity | `a_shot_reaching_two_bodies_hits_the_nearer_one_whichever_was_spawned_first`, `the_same_two_victims_with_identities_do_not` |
| Contact box, not endpoint box | A targeted `HitEvent` carries the box at contact, nudged a hair inside. Knockback and impact point are measured from the contact | `the_landing_splash_detonates_at_the_contact_not_at_the_tick_endpoint` |
| Targeted feature delivery | `HitTarget::Feature(Entity)` names the boss or breakable the sweep chose. No projectile road constructs `UnresolvedFeatures`; melee and area callers keep it | `a_direct_shot_breaks_only_one_of_two_crates_inside_its_contact_box`, `a_shot_swallowed_by_an_absorber_never_reaches_the_body_behind_it` |
| Receiver-neutral return lifetime | A returning shot survives a boss or breakable it hits, as it survives a body, and keeps the same per-leg ledger | `a_returning_shot_survives_the_crate_it_breaks_and_an_ordinary_one_does_not` |
| Direct before splash | The direct request is emitted before its landing splash for every receiver family | `the_direct_request_precedes_its_landing_splash_for_a_feature_target` |
| Compound contacts (Q96) | Contributed object surfaces are in the projectile world (`world_with_contributed_solids_and_carves` over `overlay.blocks`). The overlay gives each surface `GeoId::placement(PlacementId(FeatureId), ordinal)`. `wall_is_the_targets_own_surface` compares that identity, never a display name | `a_shot_breaks_a_solid_crate_rather_than_stopping_on_its_own_wall`, `a_crate_no_shot_can_break_still_stops_the_shot` |
| Live-room scope | The shot's room selects its world, portals and targets | `a_shot_does_not_reach_a_body_in_another_live_room`, `a_shot_threads_only_the_portals_of_its_own_live_room` |

The deleted family predicates (`ecs_hit_event_hits_actor`, `_boss`,
`_breakable`) <!-- cite-ok: deleted predicates, named so they are not re-added -->
answered "does this volume overlap something now". That is correct for a melee
hitbox and incorrect for a shot that crosses its target between two samples. Do
not add them back. The `DamageableVolumes` claim is held on
`strike_reaches_victim`, which every consumer calls.

Swept contact is exact for boxes. Every publisher emits `CombatVolume::Aabb`. A
rotated box, circle or hull has no swept primitive here, so a shaped part is
tested at the endpoint only. It is under-swept, never over-reported. The limit
is stated at the function.

## One published target geometry

Use the `DamageableVolumes` distinction without collapsing its states:

| Geometry state | Contact meaning |
| --- | --- |
| Explicit coarse target / no authored geometry | Coarse body box is the selected geometry |
| Not yet published | Preserve the established coarse fallback only where the supported construction profile permits it |
| Published nonempty | Use those actual combat shapes; their bounds are only broad-phase candidates |
| Published empty | No hurt contact; never substitute a coarse box |

Preparation/materialization must seed an authored target's correct first-tick
state before admitting it to combat. A not-yet-ready authored publisher cannot
expose an unintended coarse hurtbox for one frame. Legacy scratch bodies can
still declare coarse geometry intentionally.

Bodies and bosses use one published authored/fallback geometry in melee,
projectile candidate selection and targeted reception. A boss has no fallback
hull. A published empty authored override stays empty even when a generated
boss hull would be nonempty.

Contact eligibility is distinct from shape. A pogo-only object can expose a pogo
volume while refusing projectile damage; a stand-only destructible need not
receive a hit at all. Publish the minimal projectile-contact eligibility fact
from the real owner alongside geometry, updated at the same sample. It is a
derived view with one named writer, not a second mutable copy of breakable
rules. It must not carry an `ignore_key` string that forces a caller to
understand boss, enemy or breakable naming conventions.

Ordinary phase invulnerability and environmental-kill-only reception can
produce contact without HP loss. Do not turn all damage immunity into geometric
intangibility. The receiver owns this distinction.

## Sampling and the supported sweep model

Target and world geometry are sampled after the movement/geometry-publication
barrier and before projectile stepping. A projectile's segment is its captured
pre-integration position and the position the integration road produced.

Targets are stationary at that sample during a projectile sweep. This gives
swept projectile versus sampled target contact. It is **not** moving-target or
rotational continuous collision detection. A moving target that crosses a
stationary projectile between samples is a separate collision-program case.
Tests and documentation must name that limit, not claim complete CCD.

A teleport has no traversed segment through the intervening space. A portal
transit produces separate entry/exit travel legs in their own spatial frames.
Get them from the movement/transit operation; never sweep through the gap. A
reflected or bounced shot ends its current tick's travel after that response.
Residual-time multi-bounce integration is a later explicit behavior change.

Travel legs, broad-phase candidates and contact witnesses are tick-local
scratch, recomputed after rollback. Return-leg hit memory, projectile identity,
ownership and velocity are registered simulation state. Do not serialize
transient spatial-query results as another authority.

## Candidate representation and ordering

The names below describe internal data shapes, not public SDK spellings.
<!-- cite-ok: proposed protocol vocabulary -->

```text
ContactWitness
    projectile sequence and movement-leg ordinal
    normalized finite time of impact in [0, 1]
    target stable simulation identity + current transient entity, when a target
    world collider's stable contributor identity, when a blocking surface
    authored shape-part ordinal / stable collider ordinal
    contact point and normal in the declared spatial frame
    contact geometry sample
```

A current entity handle permits direct access in this schedule. Stable identity
supplies deterministic ordering and guards against referring to another entity
or occurrence. Cross-frame queues must carry stable identities and remap through
the body/snapshot identity road. Never hash allocation-order `Entity` values or
replace stable identity with a feature-name string.

Process projectiles by `ProjectileSeq` order (oldest first). Within one
projectile, process legs in traversal order and contacts by finite time of
impact. For exact ties use the compound-contact rule below, then stable
target/collider identity, then authored part ordinal. Production projectile
sequence and target identities must be unique; a duplicate or missing identity
is a construction/verification failure, not a sort fallback to query order.

Validate finite inputs before comparison. Use a total deterministic numeric
ordering, not an epsilon comparator whose pairwise ties can be nontransitive.
Any geometric tolerance belongs to the intersection algorithm, applied
uniformly; it is not a tie-breaking policy. Broad-phase iteration order and
entity spawn order cannot decide which equally placed target is hit.

Projectiles keep their oldest-first serial order; contacts are not sorted
globally across projectiles. Geometry stays at the declared sample while
guard/reception state changes in the established phase order. That separation
must be identical under resimulation.

### Blocking surfaces and compound contacts

Use the same finite projectile shape and the same world-hit policy for
obstruction and response. The policy names solid, blink-wall and one-way
treatment and, for directional one-ways, the admitted approach direction. Do not
import a "solids only" centre-ray special case into target selection.

A wall strictly before a target prevents that contact. An independent blocking
surface at exactly the same impact time wins over an unrelated hurt target. This
prevents a tie from granting damage through a wall.

**A destructible's own collision surface is not an unrelated wall** (ruling Q96,
[maintainer decisions](../maintainer-decisions.md)). A solid crate contributes
both world obstruction and a damageable volume. The surface and the hurt
contact at the same time are one compound contact: it damages once and also
applies the surface's physical response. A bouncing shot damages a solid crate
and bounces. "Wall wins, so the crate is invulnerable" is rejected. Exemptions
are a projectile's policy against a collision class, never a per-target
carve-out.

Match contributors only by identity. Do not infer them from approximate AABB
equality, display names or adjacency. Tile terrain uses a stable
collider/geometry identity; a contributed object collider also identifies its
owning occurrence. Several hurt parts of the same target are one direct
recipient, with authored part order resolving an otherwise equal witness.

If an object's blocking shape is reached before its inset hurt shape, there is
only a surface hit at that point. Do not pull the hurt shape forward to create a
damage event. A policy that receives damage on a protective surface must
author/publish that contact surface.

Collision contribution changes become visible at the declared geometry refresh
barrier. Breaking a crate does not cause half the tick's spatial queries to see
its old wall and the other half to see it gone. Its destruction is settled in
Resolve, after the stepping sample. Keep that phase rule until a separately
tested immediate-geometry policy replaces it.

## Contact admission, interception and damage are separate results

A direct contact fixes its recipient before damage. At contact time:

1. Reject ineligible factions, dead/retired targets, intentional intangible
   shapes and targets already excluded by this projectile's current leg policy.
2. Select the earliest reachable target/surface under the rules above.
3. Resolve a guard's immediate projectile catch through the mutable
   interception operation. Its result determines reflection/absorption now.
4. For an ordinary accepted contact, enqueue exactly one **targeted** attack
   request with its witness and attribution. Apply the flight disposition now.
5. In Combat Resolve, the selected receiver applies current reception policy and
   produces the final damage/block/no-damage outcome. No geometry/family rescan.

Exactly one receiver owns a recipient. A missing or multiple claim is a
diagnostic, not a broadcast fallback.

A boss uses body damage mechanics with its own phase policy. A breakable uses
its own health/broken reducer. Do not move both health models into a shared bag
to make dispatch uniform. New content should compose an existing receiver and
consume its outcome. A new receiver semantics needs an explicit domain adapter
and an exclusivity test, not a handler found through a service locator.

The receiver can reject damage because state changed before Resolve. It cannot
redirect the attack to the next body or enlarge the original volume. A target
that vanished or no longer matches its stable identity produces a retired-target
outcome, not a hit on a replacement. Later retirement does not undo an accepted
flight contact.

### Flight response table

| Contact result | Ordinary shot | Returning shot | Damage/feedback rule |
| --- | --- | --- | --- |
| Ineligible / intangible / already excluded | Continue candidate search | Continue candidate search | No accepted hit |
| Immediate absorption | Terminate this step and request despawn | Same | No later body, feature, splash-as-hit or world-hit road |
| Immediate reflection | Apply re-ownership/velocity response; end this step | Same | No ordinary damage request for the catching contact |
| Accepted ordinary target contact | Consume after targeted request; landing splash only if its policy calls for it | Survive; add target to current leg's hit memory | Reception can later connect, block or refuse HP loss |
| World surface | Expire/bounce policy at selected witness | Return/world policy | World cues are not target damage |
| Compound object contact | One targeted request plus physical surface response, combined once | Same combination under returning policy | No duplicate break cue or broad feature hit |

A returning projectile admits at most one ordinary target per step and records
contact before damage resolution. Keep that limit and the per-leg reset of hit
memory; a sorted candidate list must not become unlimited piercing in one tick.
An eligible ordinary contact counts for this ledger even if reception later
finds invulnerability. Interception follows its own re-ownership/ledger reset
policy. Bodies, bosses and breakables get the same flight policy; a contributed
world surface can still stop or bounce the shot under that policy.

For compound contacts, terminal responses dominate: absorption/consumption means
no later bounce, retarget or remaining travel. Otherwise apply the one world
response. The projectile gets one final step disposition. Keep an immediate
local terminal flag; deferred despawn alone does not stop the rest of a system
from using the entity.

### Area effects and attribution

A landing splash is an explicit area attack at the selected impact location,
not a second reading of the direct contact. The directly contacted recipient is
not excluded from the splash, so it can receive a direct request and an area
request; reception policy, invulnerability and death can change the second
outcome. This does not promise twice the HP reduction. One landing emits one
splash. Absorption is not a landing; a surviving return shot does not land
because it touches a target.

The order is **direct request, then its landing area request**, within the
serial projectile order, for every receiver family. Receiver adapters must keep
each recipient's order in the shared attack stream. Separate readers or queues
must not process all direct attacks before all area attacks. Use the ordered hit
stream with explicit routing, not a second projectile-only bus.

Firing allegiance is frozen at materialization. Reflection changes owner and
allegiance through the interception owner. Attacker provenance survives the
firing body's removal. Presentation emits from the resolved outcome and original
sources, not from both candidate selection and damage application.

## Schedule contract

Keep these visibility edges in the common runtime composition:

```text
relevant movement + geometry publishers (DamageFacingVolumesPublished)
    -> effect-produced projectile materialization and allegiance stamping
    -> projectile contact stepping / immediate interception / targeted requests
    -> Combat Resolve: receiver mutation and accepted outcomes
    -> move-contact latch updates and domain consequences
    -> next eligible move/flow evaluation
```

Input-produced late projectiles keep their next-tick eligibility; stamping
happens before an owner can disappear in settlement. Do not process a new
projectile once in each materialization lane. The move interpreter does not
rerun within Resolve to react to a fact it just acquired. The
[authored-technique contract](authored-technique-admission.md) defines that
latency.

A delayed targeted hit must not re-test present geometry: the projectile may
have stopped or vanished. It carries the contact box. Reception still reads the
receiver's state in its declared phase.

## Open work

| Item | Work | Trigger |
| --- | --- | --- |
| Moving-target CCD | A target that moves across a stationary shot between samples. Outside this slice | A content case that needs it, through the [collision plan](collision-and-ccd.md) |
| Swept shaped parts | A swept primitive for rotated boxes, circles or hulls | The first publisher that emits a non-`Aabb` `CombatVolume` |
| Contact module move | A `projectile/contacts.rs` module <!-- cite-ok: proposed module path --> in the same crate removes no authority and no dependency edge, so it is not taken | Do it in the same commit as a deletion it enables: a `pub(crate)` boundary, or flight state leaving for `ambition_projectiles` without the victim queries |
| A5 destructible owner | One owner for intact/broken/respawn state, hit and stand triggers, damage reduction, collision publication and one destruction transition, extracted from interaction/combat/features. A5 does not turn every world object into a breakable or merge falling chests/switches under a generic feature enum | See the [actor monolith frontier](actor-monolith-work-frontier.md) |

## Forbidden regressions

- A projectile road that constructs `UnresolvedFeatures` or calls a family
  overlap predicate.
- A damage-side site that derives boss geometry instead of reading the
  published `DamageableVolumes`.
- A segment start derived as `pos - vel * dt`.
- A centre ray as the obstruction test, or a second obstruction model beside the
  shot's `body_sweep`.
- A "feature contact, else world" or "body, else feature" branch order in place
  of the time-of-impact comparison.
- An epsilon tie comparator, or a tie that goes to an independent target over
  an independent wall.
- Contributor matching by display name (`Block.name`) instead of
  `GeoSource::Placement`.
- A family-specific lifetime for returning shots, or a splash emitted before its
  direct request.
- A compound-contact witness built on a non-solid crate. `Breakable::new`
  defaults `collision` to `None`, so such a fixture cannot see the rule. Use a
  crate that no shot can damage (`BreakableTrigger::OnStand`) as the
  discriminating case, and poison the comparison before trusting a green.
- A "two targets inside one contact box" fixture with the targets ahead of the
  shot. The overlap a single contact box spans is behind the contact point.

## Acceptance fixtures

| Fixture | Assertion |
| --- | --- |
| Boss authored hurt shape differs from generated hull | Melee and projectile admission/application agree on the published shape |
| Present-empty hurt shape / new authored target | No fallback hit, including first eligible simulation tick |
| Thin target crossed at speed | Swept projectile hits despite absent end-position overlap, within the sampled-target model |
| Target behind thin wall / corner | Earlier finite-shape obstruction prevents damage; no centre-ray shortcut |
| One-way from both sides / each world-hit policy | Candidate obstruction and world response use the same explicit policy |
| Solid destructible at contact | One compound contact can damage it and block/consume the shot; no self-wall immunity |
| Destructible collider larger than hurt region | Earlier surface blocks without invented hurt contact |
| Two targets / two parts at equal time | Stable target/part order independent of spawn/query order |
| Absorber before body and boss | One terminal absorption; no delayed fallthrough despite deferred despawn |
| Reflector before another target | Shot survives re-owned; no second target hit in that step |
| Returning shot outbound and inbound, body/boss/breakable | At most one ordinary contact per step; receiver-neutral survival and correct per-leg re-hit policy; own solid surface still honored |
| Pogo-only / stand-only object | Pogo/stand support does not imply projectile reception |
| Phase-invulnerable / environmental-only boss | Explicit contact without HP loss, with correct flight and feedback policy |
| Target retired before Resolve | No redirection, replacement hit or broad area fallback |
| Splash plus direct target, including invulnerability/destruction | Recipient included in area; direct-before-area order; one area event at impact; no guaranteed double HP damage |
| Several shots break one object | One broken transition, no duplicate rewards/cues, declared geometry-refresh timing |
| Portal transit / teleport / zero-length segment | No sweep through discontinuity; overlap/zero-time response terminates deterministically |
| Shot and target in different live rooms | No contact; the shot uses only its own room's world and portals |
| Rollback and different entity allocation | Same selected stable identities, interception, hit memory and final outcomes |

Tests must run the real publisher, projectile stepper and receiver schedule for
cross-phase assertions. Pure intersection tests alone cannot prove them. The
oldest-first projectile policy, deferred hit reception and next-tick flow
feedback are deliberate constraints of this slice. Remove them only through
separate behavior changes with new acceptance cases.
