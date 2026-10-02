# Bounded perception and attention

**Scope:** how much of the world a brain receives each tick, and the attention
budget that bounds it. **Ruling:**
[ADR 0034](../../adr/0034-perception-is-bounded-by-attention.md).
**Related:** [fighter brain](fighter-brain.md),
[world facts](world-facts-observations-and-memory.md).

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
| 1. A brain declares what it needs | landed | `PerceptionRequirement` (`None` / `TargetBelief` / `TacticalWorld`) in `ambition_characters::perception`; the `None` gate in `features/ecs/actors/update.rs` |
| 2. Cheap target belief | landed | `perception::nearest_hostile_peer` borrows the peer slice and allocates nothing. Seven of nine brain templates declare `TargetBelief`; only `Smash` and `Fighter` build a `WorldView` |
| 3. Attention budget | landed | `build_world_view` keeps at most `TACTICAL_ATTENTION` (16) peers, ranked hostile first, then squared distance, then id. The rest is `WorldView.remainder: AttentionRemainder { actors, hostiles, nearest_unattended_hostile_dist_sq }` |
| 4. Bounded crowd representation | open | see "Target representation" |

Standing facts:

- The cheap road and the full view use one definition of "in view" and one of
  "hostile" (`peer_is_visible_to_body`, `peer_is_hostile_to_body`). Guards:
  `a_cheap_belief_agrees_with_the_view_it_replaces`,
  `a_cheap_belief_sees_a_same_faction_grudge_the_way_the_view_does`.
- `WorldMemory` decays everything not seen this tick, so the cheap road still
  feeds the memory a seen set (`update_from_seen`). A cheap road that skipped
  the memory update would break pursuit of a remembered foe.
- A body's senses are derived when it decides (`perception::perception_of`,
  from its seat and the session's one extent). `PerceptionExtentOverride` is a
  per-world development knob, read once at App build.
- The attention cut is deterministic because the kept set feeds
  `WorldMemory`, which is rollback state.
- Budget guards: `a_crowd_is_attended_to_hostiles_first_and_the_rest_is_counted`,
  `below_the_attention_cap_the_view_is_what_it_always_was`.

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

**Aggregate once per tick, O(n).** Build a fixed deterministic grid of cells
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
scripts/measure_perception_density.sh
```

The census row reports `offered`, `visible` (density before the cut) and `kept`
(after the cut, the cost driver). In a sparse room the viewport bounds `kept`
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
