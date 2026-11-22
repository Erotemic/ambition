# Bounded perception and attention

**Scope:** how much of the world a brain receives each tick, and the attention
budget that bounds it. **Ruling:**
[ADR 0034](../../adr/0034-perception-is-bounded-by-attention.md).
**Related:** [fighter brain](fighter-brain.md),
[world facts](world-facts-observations-and-memory.md).

**Source follow-up:** `00bf92c22142`, 2026-11-22. The rich-view cap is
implemented; its native belief/reporting boundary is not yet complete.
OBS1 below refines that implementation claim without reopening the
earlier allocation and dense-room receipts. No new runtime or timing
result was produced by this audit.

## The rule

> A brain does not receive the room. It receives a **bounded tactical
> representation** of the room, whose size does not grow with room population.

More actors in a region increase the fidelity of the crowd summary. They do not
linearly increase the information handed to every brain. The scarce resource is
representation construction, not cognition: before this work, building each
brain's view was about 89% of decision cost in a 130-body room.

## Current shape

| Increment | State | Where |
| --- | --- | --- |
| 1. A brain declares what it needs | landed | `PerceptionRequirement` (`None` / `TargetBelief` / `TacticalWorld`) in `ambition_characters::perception`; the `None` gate in `body/actors/update.rs` |
| 2. Cheap target belief | representation landed; native dispatch refinement open | `perception::nearest_hostile_peer` borrows the peer slice and allocates nothing. Seven of nine brain templates declare `TargetBelief`; only `Smash` and `Fighter` need a `WorldView`. The native loop nevertheless also takes the cheap branch for `TacticalWorld`. |
| 3. Attention budget | rich-view cap landed; OBS1 open | `build_world_view` keeps at most `TACTICAL_ATTENTION` (16) peers, ranked hostile first, then squared distance, then id. The rest is `WorldView.remainder: AttentionRemainder { actors, hostiles, nearest_unattended_hostile_dist_sq }`. Native exact memory refresh is currently uncapped. |
| 4. Bounded crowd representation | open | see "Target representation" |

Standing facts:

- The cheap road and the full view use one definition of "in view" and one of
  "hostile" (`peer_is_visible_to_body`, `peer_is_hostile_to_body`). Guards:
  `a_cheap_belief_agrees_with_the_view_it_replaces`,
  `a_cheap_belief_sees_a_same_faction_grudge_the_way_the_view_does`.
  These do not establish liveness parity: cheap acquisition does not filter
  dead peers, while `WorldView::hostiles` does. OBS2 owns the correction.
- `WorldMemory` decays everything not seen this tick, so the cheap road still
  feeds the memory a seen set (`update_from_seen`). A cheap road that skipped
  the memory update would break pursuit of a remembered foe.
- A body's senses are derived when it decides (`perception::perception_of`,
  from its seat and the session's one extent). `PerceptionExtentOverride` is a
  per-world development knob, read once at App build.
- The attention cut must be deterministic because admitted evidence feeds
  `WorldMemory`, which is rollback state. The current native wiring below
  bypasses the kept set even though the view builder and ADR describe that
  intended boundary.
- A resolved hit updates perception memory in `CombatSet::Settle`, before the
  snapshot. `ResolvedBodyHit` is cleared on rewind, so its consequence cannot
  wait for the next tick. Guard: `contact_state_survives_real_rewind_and_resimulation`
  in [the contact replay](../../../game/ambition_app/tests/rollback_contact.rs).
- Budget guards: `a_crowd_is_attended_to_hostiles_first_and_the_rest_is_counted`,
  `below_the_attention_cap_the_view_is_what_it_always_was`.
  Both exercise view construction, not the whole native decision/crew path.

## OBS1 — make attention the native evidence admission boundary

### The native bypass

In [`tick_actor_brains`](../../../crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs),
`PerceptionRequirement::needs_target_belief()` is true for both
`TargetBelief` and `TacticalWorld`. The latter first builds the bounded
view, then also builds `cheap`. Its memory fold iterates every visible
`view_peers` row; belief selects the `Some(cheap)` branch. The
`believed_target(&world_view)` branch that would fold the kept set is
unreachable on this path.

With empty memory and 17 visible live hostile peers, a tactical body
therefore builds 16 rich actors and one remainder count, but refreshes
17 exact memories. If it shares sightings, the next native crew pass can
name all 17. The census reports the view's 16; it does not count exact
memory refreshes. This is a source-derived sequence. Below-cap parity and
the existing dense view-count receipt both remain valid.

The bounded direct-observation set must govern exact sight, direct target
acquisition and direct memory refresh together. It need not contain every
actor the body can pursue: a prior memory, a valid report or an attributed
hit can legitimately name an actor outside current attention. Those are
separate evidence inputs with their own meaning.

### Smallest delivery

1. Dispatch by the actual requirement. `TacticalWorld` must stop taking
   the unrestricted cheap fold. Its existing bounded view can feed memory
   and direct acquisition immediately; select borrowed peers once if the
   native extraction can avoid duplicating work. Do not make a temporary
   world view merely to support the cheap requirement.
2. Keep `TargetBelief`'s borrowed, no-rich-record path and pursuit receipt.
   Its current broad seen fold is not proof of a bounded admission contract.
   Before changing that population, specify its small direct-evidence budget
   and retained-target policy, then prove the intended behavior. That is an
   explicit behavior/replay change, not an incidental result of reusing the
   tactical helper. The first repair may retain today's cheap semantics
   while clearly limiting its claim to tactical admission.
3. Carry observed liveness into the small memory input. Direct acquisition
   rejects a dead candidate, but an attended death still reaches memory to
   disconfirm a prior living target. [OBS2](world-facts-observations-and-memory.md#obs2--preserve-evidence-meaning-through-memory-and-communication)
   defines the sight/report/hit merge rules and source cut. Absence from
   attention is not evidence of death or absence from the room.
4. Preserve `Perception::Omniscient` and its declared global-target policy
   for seated/summoned bodies. Scope and representation demand are distinct.
   A bounded tactical view must not silently narrow intentional omniscience.
5. Keep the zero-consumer fast path. [OBS3](agentic-character-runtime.md#obs3--resolve-observation-demand-across-controller-providers)
   makes the effective demand include admitted module consumers. A native
   fallback's `None` is sufficient to clear memory only when no other
   installed consumer requires it.

The same pure actor data and native adapter remain the owners. There is no
new sensor registry, per-observer ECS hierarchy or all-world snapshot. The
short-lived admitted set is derived; `PerceptionMemory` remains canonical.
Keep the existing room and viewport guards and the same source predicates.

### Proof at the crossing that currently fails

Use the real decision path with a sighted tactical body, empty memory and
17 distinguishable live foes. Assert the admitted exact rows, refreshed
memory and next crew report subjects, not only `WorldView.actors.len()`.
A below-cap arm must retain its existing behavior; a peer outside the
viewport or in a different live room remains excluded.

Also seed a legitimate prior memory that is outside today's attention.
It may decay and remain a pursuit candidate; the cap must not erase it as
if forgotten, refresh it as if seen, or report it as a new firsthand sight.
This separates per-tick evidence admission from retention over time.
Use the existing pursuit guard for the cheap road. OBS2 owns the liveness
and report controls rather than duplicating them here.

Extend the existing optional census to expose the counts that decide this
contract: exact direct rows admitted/refreshed, rich actors constructed,
retained memories and eligible direct reports. Keep `offered` and
`visible` separate. A capped view alone cannot certify bounded native
memory work. Do not add a shipping trace or an event log per peer.

### Cost model and the useful local optimization

Removing the duplicate tactical cheap scan/fold removes work by source
inspection. It does not prove that total room cost becomes flat.
`attend` currently collects and sorts all visible candidates before
truncating to 16. Shared peer collection, candidate acquisition, retained
memory decay, crew delivery and every body's decision still cost work.
An exact-output cap is not a bound on all these costs.

After the native admission witness passes, an owner-local streaming top-K
selector is a reasonable measured optimization. Keep at most K borrowed
candidates and fold discarded/evicted rows into the existing remainder
counts and nearest-unattended distance. Preserve input order below the cap,
the current dense ranking and deterministic ties above it. The simple
full-sort implementation is a useful reference oracle for sparse, dense
and tied inputs; gameplay need not allocate an N-row temporary to obtain K.
Keep this within the current owner until another real consumer shares the
same selection/remainder contract. It is not evidence for a generic
interchangeable-data-structures package.

Measure this only after separating selection from memory/report costs.
No timing result or required speedup is claimed. The bounded crowd/grid
increment below remains separate; do not make OBS1 wait for it.

## Rollback classes

`PerceptionMemory` (`actor.perception_memory`) is canonical rollback state: it is
accumulated history that cannot be re-derived from one tick. A change to what
builds or updates it changes a checksummed value, so it is a replay and
save-compatibility decision under ADR 0023 and ADR 0034, not a performance
tweak. Senses are derived, not stored.

A local session saves nothing per frame (`check_distance: 0`), so
`PerceptionMemory` size is a cognition cost, not a per-frame wire cost.

## Target representation

Each fighter perceives at three fidelities, each with a semantic budget:

```text
EXACT ATTENTION          bounded, ~8-16 actors
  who is attacking me, who I am attacking, nearest threat,
  the projectile about to land, the vulnerable one beside me

PERSISTENT WATCHLIST     bounded, ~4
  boss, rival, escort target, objective carrier — important regardless of
  distance, reached by stable SimId through an index, never by a room scan

AGGREGATED WORLD         bounded, ~8 groups + one summary
  "7 hostiles approaching from the right", "nothing southwest"
```

**Being mobbed is one percept.** Twenty-five surrounding enemies become

```text
CrowdPressure { count, nearest_distance, centre_of_mass, approach_velocity,
                angular_coverage, combined_threat, escape_gap }
```

plus a few exact actors. A member is promoted to exact only after the brain
chooses it. Density changes the representation, not its size.

**Attention is an explicit subsystem.** Salience scores distance,
is-attacking-me, time to collision, recent damage, objective importance, power
disparity, line of sight and watchlist membership. Today the ranking uses
hostility and distance only.

**Aggregate once per tick, O(n), when the crowd consumer is selected.** A
candidate representation is a fixed deterministic grid of cells
(`{ actor ids, per-faction counts, centroid, mean velocity, threat mass,
health mass }`) with one or two coarser levels. A grid alone does not solve the
mob case: adjacent cells can hold 100 actors. The salience budget plus an
aggregate remainder is the second half of the fix.

**Targeting runs over the tactical result.** Target acquisition uses the
attention set, the watchlist and promoted group representatives, not the room.

**Cognition budgets by brain class:**

```text
StandStill        self only
AmbientNPC        self + hazards + player proximity
SimpleEnemy       self + current target + local hostile summary + a few exact
TacticalFighter   self + attention + watchlist + groups + objectives + pressure
SmashBot          the same tactical view, plus rollout over it
```

## Determinism

Fixed cells, the sim's coordinate representation, stable `SimId` order,
deterministic tie-breakers and salience comparisons, fixed capacities, no hash
iteration deciding anything, no late asynchronous AI result. The spatial index
is derived from rollback state every tick and is not itself rollback state.

## Measurement

```bash
scripts/measure/measure_perception_density.sh
```

The census row reports `offered`, `visible` (density before the cut) and `kept`
(rich actors after the cut). It does not currently count native memory
refreshes, retained history or report fanout. In a sparse room the viewport bounds `kept`
near 14, so a sparse room cannot show why attention is needed. In a dense room
`kept` tracks population until the budget caps it at 16. Guard:
`offered_saturates_when_bodies_are_spread_grows_when_dense_and_the_budget_caps_kept`
(a pure test over slices; it asserts the shape, not the constants).

## Acceptance for the open increment

- Structural: at a population of 200, every fighter has exact <= K,
  watchlist <= W and groups <= G.
- Empirical, with tactical brains (not the `stand_still` hall cast) in a room
  where `kept` tracks population: without a budget, decision cost rises with
  population; with the budget, it flattens.
- A 200-body sparse room does not count: it keeps about 14 per viewer and
  proves nothing.

The room for the empirical clause is a deliberate dense-melee development room
(Q93, 2026-10-04; queue row DENSE-MELEE-ROOM). Dense melee is a capability the
engine must support, not a customer made up to justify this architecture. The
room is a development/stress room, not polished game content. It exercises
crowd interaction, targeting, collision, AI, VFX, camera framing, presentation
capacity, rig/impostor scaling and combat readability, so each owner can
measure there. `scripts/measure/measure_perception_density.sh` widens the viewport over
the Hall instead; keep it as a control, not as the dense room.

**The room exists (2026-11-19): `dense_melee_pit`.** At the shipped viewport,
`kept` equals `visible` up to 12 bodies and stops at 16 from 25 to 200, while
`visible` climbs to 141.7 (`--room dense_melee_pit --brain "" --extents
shipped --populations ...`; the table is on the queue row). That is the
rich-view structural clause measured in a dense room; OBS1 adds the missing
native admission witness. The empirical clause — decision
cost flat with the budget and rising without it — still needs interleaved
timing arms on a quiet machine.

## Forbidden regressions

- Do not answer cost with `if StandStill { continue }` or with dormancy. Distant
  actors sleeping is a later game policy; it does not solve the representation
  problem.
- Do not restate visibility or hostility for a cheap road; reuse the two
  shared predicates.
- Perception reads threat facts such as `ProjectileAllegiance` directly. It does
  not own projectile stepping, hit admission or control arbitration, and it
  needs no generic threat-provider registry.
- When several live instances exist, scope perception so a target in one
  instance cannot influence another.
