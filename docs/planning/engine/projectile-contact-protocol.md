# Projectile contacts: geometry, ordering and resolved identity

**Status:** target contract for A2, with the A5 destructible dependency stated
below. Not a claim that the present stepper implements continuous target contact.
**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`.
**Scope:** the current single-room, same-build deterministic simulation. The
[combat model](combat-model.md) owns combat policy and
[collision plan](collision-and-ccd.md) owns geometric primitives. This page owns
their projectile integration protocol, including its explicit timing limits.

## Decision

A projectile must select a contact from its actual movement segment and the
published simulation geometry, account for earlier blocking surfaces, and name
the selected recipient once. Flight response, attack reception and encounter or
object consequences remain different owners. A damage refusal does not erase a
physical contact retroactively.

Preserve the existing phase split: immediate projectile interception happens
while stepping; ordinary hit reception happens in Combat Resolve. Making every
reaction synchronous would change guard, damage, move-confirmation and effect
ordering merely to simplify an interface. Conversely, delaying reflection or
absorption until the next frame would allow an already stopped shot to hit again.

This protocol replaces broad feature re-query for **direct** contacts. An area
attack intentionally enumerates several recipients and remains a separate
operation. Do not use an area event as an approximation to one touching projectile.

## Current source anchors

| Source | Relevant behavior |
| --- | --- |
| `crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs` | Ordinary victims are considered separately from boss/breakable predicates; a feature hit can consume the shot before world sweep |
| `crates/ambition_projectiles/src/entity.rs` | ProjectileSeq establishes the current global projectile processing order |
| `crates/ambition_platformer2d_actor_monolith/src/features/ecs/target_volumes.rs` | Publishes authored/fallback damageable volumes, including intentional empty sets |
| `crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage_predicates.rs` | Recomputes feature eligibility/geometry instead of consuming one resolved contact |
| `crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/boss_hit.rs` | Recomputes boss geometry during damage; also owns encounter-specific reception policy |
| `crates/ambition_combat/src/components/features.rs` | DamageableVolumes explicitly distinguishes unpublished, published nonempty and published empty |
| `crates/ambition_combat/src/hitbox/mod.rs` | Existing common strike/victim geometry semantics |
| `crates/ambition_combat/src/events.rs` | Direct body, area, orb-match and unresolved-feature routing coexist |
| `crates/ambition_platformer2d_runtime/src/combat_schedule.rs` | Materialize/step, interception, Resolve, move-contact latches and later content reactions |
| `crates/ambition_platformer2d_core/src/cast.rs` | Existing body sweep and raycast primitives; a center ray is not a finite projectile sweep |

The current ordinary-target guard casts toward a victim center and then tests
end-position overlap. It does not establish swept target time of impact. The
world sweep has a separate collision policy, including projectile-kind-specific
one-way behavior. Replacing only the feature queries leaves those disagreements
intact. A2 must characterize them before relocating code.

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
registry is needed to ask a boss whether a shot arrived. Keep generic flight
helpers in `ambition_projectiles`; keep combat contact orchestration in the
existing monolith projectile region until its inputs are genuinely independent
of feature internals. A2 is not an instruction to put the whole stepper in the
projectile crate or gameplay algorithms in runtime composition.

## One published target geometry

Use the existing DamageableVolumes distinction without collapsing its states:

| Geometry state | Contact meaning |
| --- | --- |
| Explicit coarse target / no authored geometry | Coarse body box is the selected geometry |
| Not yet published | Preserve the established coarse fallback only where the supported construction profile permits it |
| Published nonempty | Use those actual combat shapes; their bounds are only broad-phase candidates |
| Published empty | No hurt contact; never substitute a coarse box |

Preparation/materialization must seed an authored target's correct first-tick
state before admitting it to combat. A not-yet-ready authored publisher cannot be
used as an excuse to expose an unintended coarse hurtbox for one frame. Legacy
scratch bodies can still declare coarse geometry intentionally.

Bodies and bosses use one published authored/fallback geometry in melee,
projectile candidate selection and targeted reception. Boss attack-phase and
animation geometry is computed by its publisher once at the declared sample,
not separately by three consumers. A published empty authored override stays
empty even when a generated boss hull would be nonempty.

Contact eligibility is distinct from shape. A pogo-only object can expose a pogo
volume while refusing projectile damage; a stand-only destructible need not
receive a hit at all. Publish the minimal projectile-contact eligibility fact
from the real owner alongside geometry, updated at the same sample. It is a
derived view with one named writer, not a second mutable copy of breakable rules.
It must not carry an `ignore_key` string that forces a caller to understand boss,
enemy or breakable naming conventions.

Ordinary phase invulnerability and environmental-kill-only reception can produce
contact without HP loss. Do not turn all damage immunity into geometric
intangibility. The receiver owns this distinction; it has different projectile
and feedback consequences from never touching a body.

## Sampling and the first supported sweep model

The first unified implementation samples target and world geometry after the
relevant movement/geometry-publication barrier and before projectile stepping.
Include bosses and destructibles in that barrier, not only ordinary bodies.
Capture a projectile's actual pre-integration position and the segment produced
by the integration road. Do not reconstruct its start as current position minus
current velocity times delta: acceleration, interception or transit can make that
an incorrect history.

For this A2 slice, targets are stationary at that declared sample during a
projectile sweep. This provides swept projectile versus sampled target contact;
it is **not** full moving-target or rotational continuous collision detection.
A moving target crossing a stationary projectile between samples remains a
separate collision-program acceptance case. Tests and documentation must name
that limit instead of claiming complete CCD.

A teleport has no traversed segment through the intervening space. A portal
transit produces separate entry/exit travel legs in their respective spatial
frames. Obtain them from the existing movement/transit operation; never sweep
through the gap. A reflected/bounced shot ends its current tick's travel after
that response in this slice. Residual-time multi-bounce integration is a later
explicit behavior change, not an incidental benefit of a while loop.

The ordered list of travel legs, broad-phase candidates and contact witnesses is
tick-local scratch. It is recomputed after rollback. Persistent return-leg hit
memory, projectile identity, ownership and velocity remain registered simulation
state. Do not serialize transient spatial-query results as another authority.

## Candidate representation and ordering

Proposed names below describe internal data shapes; they are not required public
SDK spellings. <!-- cite-ok: proposed protocol vocabulary -->

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
the existing body/snapshot identity road. Never hash allocation-order Entity
values or replace stable identity with a feature-name string.

Process projectiles by the existing ProjectileSeq order. Within one projectile,
process legs in actual traversal order and contacts by finite time of impact.
For exact ties use the compound-contact rule below, then stable target/collider
identity, then authored part ordinal. Require unique production projectile
sequence and target identities; duplicate or missing required identity is a
construction/verification failure, not a sort fallback to query order.

Validate finite inputs before comparison. Use a total deterministic numeric
ordering, not an epsilon comparator whose pairwise ties can be nontransitive.
Any geometric tolerance belongs to the intersection algorithm, uniformly applied;
it does not become an arbitrary tie-breaking policy. Broad-phase iteration order
and entity spawn order cannot decide which equally placed target is hit.

Projectiles retain their existing oldest-first serial order; A2 does not globally
sort every projectile by subframe impact time. Geometry stays at the declared
sample while guard/reception state evolves in the established phase order. That
separation must be identical under resimulation.

### Blocking surfaces and compound contacts

Use the same finite projectile shape and the same world-hit policy for obstruction
and response. The policy must name solid, blink-wall and one-way treatment and,
for directional one-ways, the admitted approach direction. Do not import a
"solids only" center-ray special case into the target selection branch.

A wall strictly before a target prevents that contact. An independent blocking
surface at exactly the same impact time wins over an unrelated hurt target. This
prevents a tie from granting damage through a wall.

**A destructible's own collision surface is not an unrelated wall.** A solid crate
can contribute both world obstruction and a damageable volume. Carry contributor
identity through the world collision witness. Coalesce matching surface/target
contacts at the same time into one compound contact, resolve the target once,
and also honor the surface's physical response. Without this identity, a generic
"wall first" rule can make every solid destructible immune to projectiles.

Do not infer matching contributors from approximate AABB equality, display names
or adjacency. Tile terrain uses a stable collider/geometry identity; a contributed
object collider additionally identifies its owning occurrence. Several hurt parts
of the same target are one direct recipient, with authored part order resolving
an otherwise equal witness.

If an object's blocking shape is encountered before its inset hurt shape, there
is only a surface hit at that point. Do not pull its hurt shape forward to create
a damage event. A policy that explicitly receives damage on a protective surface
must author/publish that contact surface rather than relying on a dispatch trick.

Collision contribution changes become visible at the declared geometry refresh
barrier. Breaking a crate does not cause half the tick's spatial queries to see
its old wall and the other half to see it gone. With the current deferred
Resolve path, its destruction is settled after the stepping sample. Preserve that
phase rule until a separately tested immediate-geometry policy replaces it.

## Contact admission, interception and damage are separate results

A direct contact fixes its recipient before damage. At contact time:

1. Reject ineligible factions, dead/retired targets, intentional intangible shapes
   and targets already excluded by this projectile's current leg policy.
2. Select the earliest reachable target/surface under the rules above.
3. Resolve a guard's immediate projectile catch through the existing mutable
   interception operation. Its result determines reflection/absorption now.
4. For an ordinary accepted contact, enqueue exactly one **targeted** attack
   request with its witness and attribution. Apply the flight disposition now.
5. In Combat Resolve, the selected receiver applies current reception policy and
   produces the final damage/block/no-damage outcome. No geometry/family rescan.

The current Body-target path is the starting point. Extend the direct-target
protocol to all supported recipients, retaining a stable identity alongside any
transient handle that can survive a queue. A local concrete dispatcher or disjoint
typed receivers may implement delivery. They must prove exactly one receiver
owns a recipient; missing/multiple claims are diagnostics, not a broadcast fallback.

A boss can use ordinary body damage mechanics with its own phase policy. A
breakable can use its own health/broken reducer. Do not move both health models
into a shared bag merely to make dispatch uniform. New content should normally
compose an existing receiver and consume its outcome; a genuinely new receiver
semantics requires an explicit domain adapter and exclusivity test, not a hidden
handler discovered through a universal service locator.

The receiver can reject damage because state changed before Resolve. It cannot
redirect the attack to the next body or enlarge the original volume. A target
that vanished or no longer matches its stable identity produces a retired-target
outcome, not a hit on a replacement. An accepted flight contact is not undone by
that later retirement.

### Flight response table

| Contact result | Ordinary shot | Returning shot | Damage/feedback rule |
| --- | --- | --- | --- |
| Ineligible / intangible / already excluded | Continue candidate search | Continue candidate search | No accepted hit |
| Immediate absorption | Terminate this step and request despawn | Same | No later body, feature, splash-as-hit or world-hit road |
| Immediate reflection | Apply re-ownership/velocity response; end this step | Same | No ordinary damage request for the catching contact |
| Accepted ordinary target contact | Consume after targeted request; landing splash only if its policy calls for it | Survive; add target to current leg's hit memory | Reception can later connect, block or refuse HP loss |
| World surface | Existing expire/bounce policy at selected witness | Existing return/world policy | World cues are not target damage |
| Compound object contact | One targeted request plus physical surface response, combined once | Same combination under returning policy | No duplicate break cue or broad feature hit |

Returning projectiles currently admit at most one ordinary target per step and
record contact before damage resolution. Preserve that limit and per-leg reset
of hit memory; do not accidentally turn the sorted candidate list into unlimited
piercing in the same tick. An eligible ordinary contact counts for this ledger
even if reception later finds invulnerability. Interception follows its own
re-ownership/ledger reset policy. Tests must cover outgoing and return legs.

The current unresolved-feature fallback unconditionally despawns even a returning
shot. The target contract removes that family-specific lifetime decision: an
ordinary contact uses the projectile's flight policy for bodies, bosses and
breakables alike. A contributed world surface can still stop/bounce it under
that policy. This is an intentional behavior correction, not a behavior-preserving
extraction. Land its boss/breakable outbound/return fixtures and semantic change
separately from the adapter/file move; do not preserve a hidden feature-only
consume flag in the new interface.

For compound contacts, terminal responses dominate: absorption/consumption means
no subsequent bounce, retarget or remaining travel. Otherwise apply the one
world response. Each side may contribute facts, but the projectile gets one final
step disposition. Maintain an immediate local terminal flag; deferred despawn
alone does not stop the rest of a system from using the entity.

### Area effects and attribution

A landing splash is an explicit area attack at the selected impact location,
not a second interpretation of the direct contact. Retain the existing inclusion
rule: the directly contacted recipient is not excluded from the splash. The
current emitter uses an empty ignored-target list. Thus the recipient can receive
a direct request and an area request; reception policy, invulnerability and death
can change the second outcome. This does not promise twice the HP reduction.
Only one splash is emitted for one landing. Absorption is not an ordinary landing;
a surviving return shot does not land merely because it contacts a target.

The target order is **direct request, then its landing area request**, within the
existing serial projectile order. Current ordinary-body delivery already writes
that order; the feature fallback writes splash first. Standardizing the latter
can change outcomes when a first hit grants invulnerability or breaks a target.
Treat this as a separate semantic patch with both receiver-family fixtures,
not as an incidental reordering during a file move. Preserve splash inclusion
and one-area-event cardinality while testing the intentionally selected order.

Receiver adapters must preserve each recipient's order in the shared attack
stream, including area and targeted requests. Separate message readers or queues
must not process all direct attacks before all area attacks, nor restore the old
family-dependent order. Use the existing ordered hit stream with explicit routing
rather than a second projectile-only bus. Independent disjoint recipients may
be processed separately only where they cannot reorder shared consequences.

Freeze firing allegiance at materialization as today. Reflection changes owner
and allegiance through the existing interception owner. Preserve attacker
provenance even if the firing body disappears. Presentation emits from the
resolved outcome and original sources, not from both candidate selection and
damage application. Preserve event/cardinality traces as acceptance evidence.

## Schedule contract

Keep these visibility edges in the common runtime composition:

```text
relevant movement + geometry publishers
    -> effect-produced projectile materialization and allegiance stamping
    -> projectile contact stepping / immediate interception / targeted requests
    -> Combat Resolve: receiver mutation and accepted outcomes
    -> move-contact latch updates and domain consequences
    -> next eligible move/flow evaluation
```

Input-produced late projectiles retain their existing next-tick eligibility;
stamping happens before an owner can disappear in settlement. Do not process a
new projectile once in each materialization lane. The move interpreter does not
rerun within Resolve to react to a fact it just acquired. The
[authored-technique contract](authored-technique-admission.md) defines that latency.

A delayed targeted hit must not re-test present geometry: the projectile may have
already stopped or vanished. It carries the admission witness. Reception still
reads the receiver's state in its declared phase. This preserves geometric truth and the declared same-tick reception phase without
a global event bus. The two explicit policy corrections above (return lifetime
and direct-before-splash) have their own fixtures rather than a false claim of
bit-for-bit preservation for the old feature branch.

## Implementation packets and deletion gates

### A2a: geometry agreement before a package move

Update the boss publisher/preflight/application in the source table to consume
the same DamageableVolumes semantics. Add authored-disjoint, authored-empty,
first-frame and invulnerable-phase fixtures. Preserve environmental-only feedback.
Locate all publishers' actual schedule ancestry and establish the common sample.
Do not add a second projectile-only boss shape.

### A2b: one segment/obstruction decision

Extract actual travel-leg capture and shape sweep helpers from the existing
stepper into the projectile/spatial owners. Add stable collider-contributor
identity where world objects publish geometry. Introduce compound contact
coalescing in the current integration module. Replace center-distance ordering
and the feature-before-world shortcut with the specified candidate ordering.

This changes collision behavior and must be committed separately from mechanical
file moves. First support stationary sampled targets; include a fast projectile
crossing a thin victim without endpoint overlap. Do not call that a complete
moving-body CCD implementation. Record current policies for one-way, return,
splash and bounce with tests before replacing their branches.

### A2 receipts, 2026-09-09

**A2a — LANDED.** Boss hurt geometry is computed once by its publisher. It was
derived at three damage-side sites (the projectile preflight and `apply_boss_hit`
twice) and none consulted `refresh_boss_damageable_volumes`, so a boss's authored
hurtboxes governed nothing on the damage road and an authored EMPTY override
still offered a target. The publication ran in `WorldPrep`, a phase ahead of the
boss brain, its move projection and its animator, and therefore described the
previous frame — which is why the consumers re-derived. Bosses now join the
damage-facing barrier (`DamageFacingVolumesPublished`, after `Playback`, before
`Resolve`), the barrier has a name because ordering against a twice-registered
system's type set is impossible, and `drive_boss_animators` states that it writes
one of its inputs. The catalog, the attack state and the animation sample left
with the derivation: `apply_boss_hit` can no longer derive geometry at all, and
the projectile stepper no longer depends on `BossCatalog`. Witness:
`a_boss_is_reached_only_through_its_published_volumes`, four states
including the anti-vacuity row, poison-verified against a reintroduced coarse
fallback. ⚠ Breakables were NOT added to the barrier: their geometry is
`CenteredAabb` + broken state, both settled in `WorldPrep`, so a second
publication would cost a pass and change nothing. Say so before adding one.

**A2b — reopened by review #7 and re-closed the same day; the ordering rule is
now one comparison, not three.**

⛔⛔ THE THREE-WAY SPLIT REVIEW #7 FOUND, and it survived every correction below
because each of them made ONE branch swept without making the branches agree:

- the boss/breakable branch compared NOTHING. It computed a swept
  `FeatureContact` and, on finding one, emitted the targeted hit and the splash
  and `continue`d — so `resolve_world_collision` ran only when no feature was
  reached. The effective rule was *"feature contact, else world collision"*, and
  a crate or a boss behind an unrelated solid was struck through it;
- the body branch asked the right question of the wrong geometry: it swept from
  the muzzle to the victim's CENTRE. That answers *"is a wall before the victim's
  middle?"*, and on a wide body the shot reaches the near face well before the
  centre — so a wall between the face and the centre refused a hit that
  physically happened. Wrong in the direction that COSTS a legitimate hit, which
  is why no existing test saw it;
- and the world branch swept a third time for its own pull-back.

⇒ ONE `body_sweep` over the shot's actual travel leg, with the shot's own box and
the shot's own `WorldHitPolicy`, its finite `time_of_impact` kept in the leg's own
[0, 1] parameter and compared directly against every body's and every feature's
contact time. A blocker strictly earlier wins. A TIE goes to the target, which
restates the strict comparison the body branch already used rather than inventing
a policy: nothing here can yet tell a destructible's own surface from an
independent blocker, and that distinction is A5's contributor identity. The
world pull-back reuses the same result (the block's centre rides along with the
time, because deriving it there would be the sweep asked a second way).

⛔⛔ AND A THIRD HOLE THE REVIEW NAMED: BODIES AND FEATURES WERE NOT COMPARED TO
EACH OTHER EITHER. The body loop ran first and, on a hit, `continue`d past the
whole step — so the boss/breakable candidate was not even COMPUTED. "Body" beat
"boss or breakable" by being the earlier code branch: a crate at 0.2 of the leg
lost to a body at 0.8. The feature candidate is resolved beside the bodies now
and the body loop stops as soon as a feature is strictly earlier.

Witnesses, each poison-verified against the exact code it replaced:
`a_crate_behind_a_wall_is_not_broken_and_one_in_front_of_it_is` — its
anti-vacuity floor moves the WALL rather than the crate, so an arm that broke
nothing would fail it — `a_wall_inside_a_wide_body_past_its_near_face_does_not_stop_the_hit`,
and `the_earliest_contact_wins_whether_it_is_a_body_or_a_crate`, whose two arms
are each other's floor. With the pre-existing
`a_shot_does_not_damage_a_victim_standing_behind_a_wall` that is wall-before-body,
body-before-wall, wall-before-feature, feature-before-wall, the large-body near
face, and both directions of body-versus-feature.

⭐⭐ THE COMPOUND SOLID OBJECT ROW IS RULED (Q96, 2026-09-10) AND NOT YET BUILT.
The ruling selects **this document's compound contact**: a published surface
participates in projectile collision, and a contributor supplying both a surface
and a damageable volume at the same time of impact yields ONE contact that damages
once and also applies the physical response. A bouncing shot damages a solid crate
AND bounces. *"Wall wins, therefore the crate is invulnerable"* is rejected. Where
the surface lies before an INSET hurt volume, only the surface was reached — no
damage yet. Exemptions are a projectile's POLICY against a collision CLASS, never a
per-target carve-out. See
[`maintainer-decisions.md`](../maintainer-decisions.md).

⚠ IT IS STILL NOT REACHABLE ON THIS ROAD, WHICH IS WHAT THE WORK HAS TO CHANGE.
A `BreakableCollision::Solid` feature publishes a `BlinkWall` into
`FeatureEcsWorldOverlay::blocks`, but `ProjectileCollisionWorld::solids()`
(`ambition_projectiles/src/collision_world.rs:53`, re-read 2026-09-10) composites
only `gate_solids`, `portal_carves` and `removed_block_names` — so a destructible's
own surface is not in the world a projectile sweeps, and the tie rule has no
compound case to get wrong yet.

⛔⛔ TWO WARNINGS FOR WHOEVER BUILDS IT, BOTH PAID FOR ALREADY.
* **A witness test here has passed under a deliberately broken comparison TWICE** —
  the fixture never published the surface, and the landing splash broke the crate
  regardless of the direct hit. It was deleted rather than kept. ⇒ Poison the
  comparison before trusting a green.
* **A one-word paraphrase selected the REJECTED model twice** while Q96 was being
  relayed: *"a solid breakable stops a bolt"*, and *"a solid surface always stops a
  shot"*. Both read as "wall wins". ⇒ The rejected model is the one that fits in a
  short sentence, so short sentences about this row are where it comes back.

**A2b — the obstruction half landed; the swept-target half has not.**

*One travel leg, captured.* Two sites derived the segment as `kin.pos - kin.vel *
dt`. That is EXACT for today's integrator — `tick` accelerates and then
integrates with the new velocity — and exact only because of three unrelated
facts nothing stated together: that integration order, the interception
short-circuit that stops a re-owned shot before either site, and the portal
transit that `continue`s past both. The pre-integration position is captured once
now, so the segment cannot silently describe space the shot never crossed.

*One obstruction model.* The world branch swept the shot's BOX against the blocks
its own `WorldHitPolicy` names; the victim branch cast `raycast_solids` at the
victim's CENTRE with `include_one_way = false` hard-coded. Two answers to "is
something in the way", disagreeing on shape and on policy — so a wall covering a
victim's body but not its centre did not block, a wall the shot's box clips at a
corner did not block, and an `ExpireOnContact` shot damaged straight through a
one-way its own contract says ends it. Both branches use `body_sweep` with the
shot's box and the shot's policy now. Witness:
`a_one_way_blocks_the_shot_whose_policy_says_it_should_and_no_other`, two arms
(the `Bouncing` arm is the anti-vacuity floor — a fireball crosses a one-way by
design and must still land), poison-verified against a solids-only predicate.

*Swept target contact, and the witness it carries.* Contact was ENDPOINT overlap:
the stepper moved the shot to its new position and asked whether the box THERE
reached the victim. At 4000 px/s a tick covers 64 px, so a body thinner than that
was behind the endpoint before anything looked — the bolt passed through and
nobody was touched, while the obstruction test beside it was already swept. The
shot's box now travels the captured leg
(`ambition_combat::hitbox::swept_strike_reaches_victim`), and candidates are
ordered by TIME OF IMPACT rather than by distance from the muzzle to a victim's
centre: once contact has a finite time, "whose centre is nearer" and "whom it
reached first" are different answers for two bodies of different size.

⛔ **AND THE DELAYED APPLICATION HAD TO STOP RE-TESTING PRESENT GEOMETRY**, which
the fixture found rather than the reading. The targeted `HitEvent` named its
victim and carried `kin.aabb()` — the ENDPOINT box — and the applier tests that
volume against the victim again, so a shot that genuinely crossed its target
failed its own re-test and landed nothing. The event carries the box AT CONTACT
now, nudged a hair inside for the same reason the world sweep nudges
(`time_of_impact` leaves the box tangent and every downstream test is
`strict_intersects`). Knockback direction and the impact point are measured from
the contact rather than from the endpoint, which for a fast shot is past the body
it hit. Witness: `a_fast_shot_hits_a_thin_body_it_crosses_within_one_tick`, with
the ordinary-speed arm as its anti-vacuity floor and parity check;
poison-verified against an endpoint-only sweep.

⚠ SWEPT FOR BOXES, ENDPOINT-ONLY FOR A SHAPED PART. Every publisher in the tree
emits `CombatVolume::Aabb`, so the swept answer is exact for all of them; a
rotated box, circle or hull has no swept primitive here and answering from its
BOUNDS would reintroduce the dead-corner hit `strike_reaches_victim` refuses. A
shaped part is under-swept, never over-reported, and the limit is stated at the
function.

*The feature branch, swept and given a lifetime of its own.* The boss/breakable
road is separate code with a separate geometry source, and making the body branch
swept left it untouched — a shot crossing a thin crate between samples touched
nothing, and the branch handed the applier the same endpoint box to re-test.
`projectile_reaches_breakable` / `projectile_reaches_boss` answer with a
`FeatureContact` (time, target centre, target entity); the boss sweep reuses
`swept_strike_reaches_victim`, so a boss and an ordinary body answer the same way
about published parts, coarse fallback (a boss has none) and intangibility. The
breakable eligibility rule is one function instead of the same four conditions
spelled in the discrete predicate and again in the swept one.

**The receiver-neutral returning-shot lifetime landed with it**, as its own
semantic change with its own fixture, which is what this document asked for. The
branch despawned EVERY shot that reached a boss or a breakable, so the same
boomerang came back from a body and vanished into a crate — one projectile, two
lifetimes, decided by the family of what it happened to touch. It could not ask
`game.returns()` before, because a surviving shot overlaps its target for as many
ticks as it takes to pass through and this road had no per-leg ledger; the
contact carries the target's identity now, so it keeps the same ledger the body
branch keeps. Witness:
`a_returning_shot_survives_the_crate_it_breaks_and_an_ordinary_one_does_not`,
both arms, poison-verified.

⛔ **STILL OPEN IN A2b/A2c.**
- Compound contacts and contributor identity. Measured 2026-09-09: a solid
  breakable contributes its world block with `GeoId::anon()` and a formatted
  `"ecs-breakable {name}"` display string, which is exactly the identity this
  document forbids inferring from. No live defect rides on it today — the feature
  branch resolves before the world sweep, and a breakable's blocking shape and
  hurt shape are the SAME `CenteredAabb`, so "collider larger than hurt region"
  is not reachable with current content — so this is A5 infrastructure rather
  than a repair.
- ~~Direct-before-splash order~~ **LANDED 2026-09-09** as its own semantic patch.
  The feature branch wrote the landing splash BEFORE the direct request, so for a
  boss or a breakable the AREA event was applied before the hit that caused it —
  and wherever the first application changes what the second finds (an
  invulnerability window, a one-hit break) the splash was credited and the shot
  refused. Witness:
  `the_direct_request_precedes_its_landing_splash_for_a_feature_target`, which
  asserts the EMITTED order and says why: the consequence needs content that
  reacts differently to the two orders, and a fixture that could only see the
  consequence would be green for either order against a target with none.
  Poison-verified.
- ~~Targeted delivery for the projectile road~~ **LANDED 2026-09-09.**
  `HitTarget::Feature(Entity)` names the boss or breakable the swept contact
  chose, and both folds narrow to it. The defect it removes is real and was
  measured: `UnresolvedFeatures` hands the applier a VOLUME, and the breakable
  fold has no `break`, so one shot whose contact box covered two crates destroyed
  both — and for a multi-part boss, which part was credited came from query
  order, which a rewind need not reproduce. `UnresolvedFeatures` stays for melee
  and area callers, whose scope legitimately hunts for everything they overlap.
  Schema 176 → 177: a targeted feature hit must not compare equal to a broadcast
  remainder. Witness:
  `a_direct_shot_breaks_only_one_of_two_crates_inside_its_contact_box`,
  poison-verified.

  ⚠ **THE FIRST VERSION OF THAT FIXTURE COULD NOT SEE THE DEFECT AND ITS POISON
  PASSED.** It put the crates AHEAD of the shot, where a contact box — whose
  leading edge sits on the near target's near face — can never reach a second
  one. The overlap a single contact box spans is BEHIND the contact point, so the
  two targets must already be inside the shot's own box. The finding is about the
  fixture, and it is recorded because the same mistake fits every "two targets at
  once" row in the matrix below.

  ⭐ It also closed an ordering gap the sweep introduced: `min_by` on time alone
  hands an exact tie back to query order. `FeatureContact::order_for_caller` is
  the protocol's total order — time, then position, then authored identity —
  matching what the body branch already did.

- ~~The family-predicate deletion gate~~ **CLOSED 2026-09-09.**
  `ecs_hit_event_hits_actor`, `_boss` and `_breakable` <!-- cite-ok: deleted by this row --> answered "does this strike
  volume overlap something right now" — the right question for a melee hitbox
  that exists for a window of frames, the wrong one for a projectile that crosses
  its target between two samples. Once contact became swept the stepper stopped
  calling them, `git grep` found no other production caller for any of the three,
  and this document's own rule — delete a predicate only after every caller has
  migrated — was satisfied. Deleted.

  ⚠ `ecs_hit_event_hits_actor` <!-- cite-ok: deleted by this row --> had no production caller even BEFORE that. It was
  reachable, tested, and unreached, so its four-state `DamageableVolumes` claim
  was pinned against a function nobody ran. The claim moved onto
  `strike_reaches_victim` — what every consumer of the rule actually calls —
  rather than leaving with the dead predicate, and the boss claim moved onto the
  swept sibling with a new arm proving it is swept rather than an endpoint test
  wearing a swept signature.

- **The file relocation is deliberately NOT taken, and the reason is a standing
  rule rather than a shortage of time.** A cohesive `projectile/contacts.rs`
  <!-- cite-ok: proposed module path --> inside the same crate removes no
  authority and no dependency edge: the contact code would import exactly what it
  imports now and be reachable by exactly the same callers. The elegance test this
  repository works to is *"name the authority or the dependency edge the change
  removes; if neither, it is churn"*, and a same-crate move names neither.

  ⇒ It becomes worth doing when it enables something — a `pub(crate)` boundary
  that lets a helper stop being public, or a split that lets the flight state
  leave for `ambition_projectiles` without the victim queries following it. Take
  it then, with that deletion in the same commit. Every semantic correction and
  every deletion gate that had to precede A2c has landed.
Moving-target CCD remains out of scope by the protocol's own slice.

### A2c: direct delivery and smaller flight authority

Replace unresolved-feature projectile events with targeted delivery. Identity-
filtered receiver adapters may stay at current feature integration owners while
their private code is reduced. Move integration helpers to a cohesive proposed
`projectile/contacts.rs` module inside the monolith; move only pure flight state/
response helpers to `ambition_projectiles`. <!-- cite-ok: proposed module path -->
Do not move live victim query authority into a projectile spawn/materialization
crate. Runtime retains installation/order, not contact algorithms.

Delete projectile imports of boss catalogs and the family predicates when no
projectile caller remains. Delete a predicate itself only after all its other
callers have migrated. Remove the projectile use of UnresolvedFeatures; melee
and area callers have their own scope and cannot be declared migrated by this
packet. Keep a direct dependency on genuine shared damage/spatial vocabulary
rather than inventing adapters solely to improve the graph.

Before the final adapter/file relocation, land and test the two explicit contact
policy corrections in separate semantic commits: receiver-neutral returning-shot
lifetime and direct-before-splash order. Keep the geometry/ordering patch distinct
as well. A2c completion requires all of them; graph cleanup cannot certify a
partially unified policy.

### A5 prerequisite made concrete

A destructible owner must eventually own intact/broken/respawn state, its hit and
stand triggers, damage reduction, collision publication and one destruction
transition. The receipt below must pass before extracting that owner from
interaction/combat/features. A5 is not authorization to turn every world object
into a breakable or merge falling chests/switches under a generic feature enum.

## Acceptance fixtures

| Fixture | Assertion |
| --- | --- |
| Boss authored hurt shape differs from generated hull | Melee and projectile admission/application agree on the published shape |
| Present-empty hurt shape / new authored target | No fallback hit, including first eligible simulation tick |
| Thin target crossed at speed | Swept projectile hits despite absent end-position overlap, within the sampled-target model |
| Target behind thin wall / corner | Earlier finite-shape obstruction prevents damage; no center-ray shortcut |
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
| Rollback and different entity allocation | Same selected stable identities, interception, hit memory and final outcomes |

Tests must run the real publisher, projectile stepper and receiver schedule for
cross-phase assertions. Pure intersection tests alone cannot prove them. The
oldest-first projectile policy, deferred hit reception and next-tick flow feedback
are deliberate compatibility constraints of this slice. Remove them only through
separate behavior changes with new acceptance cases.
