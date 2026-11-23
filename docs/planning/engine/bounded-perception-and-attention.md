# Bounded perception and attention

**Scope:** how much of the world a brain receives each tick, and the attention
budget that bounds it. **Ruling:**
[ADR 0034](../../adr/0034-perception-is-bounded-by-attention.md).
**Related:** [fighter brain](fighter-brain.md),
[world facts](world-facts-observations-and-memory.md).

**Source follow-up:** `00bf92c22142`, 2026-11-22. The rich-view cap was
implemented; its native belief/reporting boundary was not. OBS1 closed
that boundary on 2026-11-23 (`6fe56fb12`) without reopening the earlier
allocation and dense-room receipts. No timing result is claimed.

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
| 2. Cheap target belief | landed | `perception::nearest_hostile_peer` borrows the peer slice and allocates nothing. Seven of nine brain templates declare `TargetBelief`; only `Smash` and `Fighter` need a `WorldView`. Native dispatch is by the actual requirement (`perceive_for_decision`). |
| 3. Attention budget | landed for direct evidence (OBS1) | `build_world_view` keeps at most `TACTICAL_ATTENTION` (16) peers, ranked living hostiles first, then squared distance, then id. The rest is `WorldView.remainder: AttentionRemainder { actors, hostiles, nearest_unattended_hostile_dist_sq }`. A `TacticalWorld` body refreshes memory for, acquires from and reports exactly that kept set. |
| 4. Bounded crowd representation | open | see "Target representation" |

Standing facts:

- The cheap road and the full view use one definition of "in view" and one of
  "hostile" (`peer_is_visible_to_body`, `peer_is_hostile_to_body`). Guards:
  `a_cheap_belief_agrees_with_the_view_it_replaces`,
  `a_cheap_belief_sees_a_same_faction_grudge_the_way_the_view_does`.
  Both roads also reject a dead candidate and fold a seen death into memory
  (OBS2: `an_observed_death_disconfirms_pursuit_and_an_unobserved_one_does_not`).
- `WorldMemory` decays everything not seen this tick, so the cheap road still
  feeds the memory a seen set (`update_from_seen`). A cheap road that skipped
  the memory update would break pursuit of a remembered foe.
- A body's senses are derived when it decides (`perception::perception_of`,
  from its seat and the session's one extent). `PerceptionExtentOverride` is a
  per-world development knob, read once at App build.
- The attention cut must be deterministic because admitted evidence feeds
  `WorldMemory`, which is rollback state.
- A resolved hit updates perception memory in `CombatSet::Settle`, before the
  snapshot. `ResolvedBodyHit` is cleared on rewind, so its consequence cannot
  wait for the next tick. Guard: `contact_state_survives_real_rewind_and_resimulation`
  in [the contact replay](../../../game/ambition_app/tests/rollback_contact.rs).
- Budget guards: `a_crowd_is_attended_to_hostiles_first_and_the_rest_is_counted`,
  `below_the_attention_cap_the_view_is_what_it_always_was` (view construction),
  and `the_tactical_attention_cap_bounds_the_evidence_a_body_admits` (the
  native admission and crew path).

## OBS1 — make attention the native evidence admission boundary

**Landed 2026-11-23, `6fe56fb12`.** A `TacticalWorld` body built its
bounded view and then also took the cheap road, folding every visible
peer into memory: with 17 visible foes it built 16 rich actors, refreshed
17 exact memories, and could call all 17 out to its crew.
[`perceive_for_decision`](../../../crates/ambition_platformer2d_actor_monolith/src/body/perception.rs)
now owns per-body admission. `TacticalWorld` folds exactly the attended
view; `TargetBelief` keeps the borrowed cheap road and builds no view;
`Perception::Omniscient` keeps its global target (belief `None`). The
`None` fast path is unchanged.

Guard: `the_tactical_attention_cap_bounds_the_evidence_a_body_admits`.
Seventeen foes: memory and the next crew call name the 16 attended. Below
the cap all are admitted. A peer outside the viewport is never admitted;
another live room's peer is never offered
(`a_viewer_perceives_only_the_bodies_of_its_own_live_room`). A prior
memory of the unattended foe decays and stays a pursuit candidate; it is
not refreshed as seen and not called out. The cheap-road pursuit guard
(`a_target_belief_body_still_pursues_a_foe_it_has_lost_sight_of`) drives
both native roads through the same function.

The census row adds `admitted` (exact rows folded), `rich` (view rows
built), `retained` (memory rows after the fold) and `reports` (eligible
crew calls per view). `offered` and `visible` stay separate.

**Still open:**

- `TargetBelief` still admits every visible peer. Before changing that
  population, specify its direct-evidence budget and retained-target
  policy and prove it; that is a behavior/replay change.
- [OBS3](agentic-character-runtime.md#obs3--resolve-observation-demand-across-controller-providers)
  makes the effective demand include admitted module consumers.
- Retention over time (how many old memories a body keeps) is a separate
  budget.

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
(rich actors after the cut), and since OBS1 `admitted`, `rich`, `retained`
and `reports`. In a sparse room the viewport bounds `kept`
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
rich-view structural clause measured in a dense room; OBS1 added the
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
