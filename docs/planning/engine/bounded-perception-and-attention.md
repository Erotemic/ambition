# Bounded perception and attention

**Status:** direction set by Jon 2026-09-01, grounded in measurement. Not yet
scheduled; no code written against it.

## The rule

> A brain does not receive the room. It receives a **bounded tactical
> representation** of the room, whose size does not grow with room population.

Increasing the number of actors in a region must increase the **fidelity of the
crowd summary**, not linearly increase the amount of information handed to every
brain.

## Why now: the measurement

`hall_of_characters`, 130 bodies, headless, no Tracy. Two probes decomposed the
per-tick cost of `ActorDecisionSet::Decide`:

```text
130 brains deciding                    0.098 ms/tick
building what they decide about        0.760 ms/tick
  peers_seen_by clone                    ~0.38
  build_world_view walk                  ~0.38
```

**The brains are already free. The construction is 89% of the cost.** Every
actor receives a `WorldView` containing all 129 others, each `PerceptionPeer`
cloned with an owned `String id`, then cloned again into a `PerceivedActor` —
~16.8k heap allocations per tick, twice. The hall's cast is authored
`stand_still` and discards every one of them unread.

⭐ **THAT REFRAMES THE DESIGN.** The scarce resource to budget is not cognition,
it is **representation construction**. A rollout-searching fighter will make
cognition expensive too, but today none of the cost is thinking.

⛔ And the fix is NOT `if StandStill { continue }`. That makes the benchmark
cheap and leaves the architecture unchanged for the case we actually want: a room
of genuinely tactical fighters.

## ⭐ MEASURED 2026-09-01: the three pieces are not equally urgent

Each hall actor is **offered 129 peers and keeps 14.4**, and that number does not
move when the cast goes from 65 to 130 (`kept_frac` 0.225 → 0.112; no body in the
hall is `Omniscient`, so the viewport filter is live for all of them).

```text
kept per actor   ~14.4   CONSTANT in population   the work is already O(n)
offered          n - 1    GROWS                    the scan is O(n²)
```

**The view is already bounded. The scan is not** — 89% of every actor's scan at
130 bodies is discarded. So:

| piece | verdict today |
|---|---|
| **spatial index** | **THE WHOLE WIN.** Take the scan from 129 to nearby-cell occupants; output unchanged at ~14. |
| attention budget (top-K) | The viewport already delivers ~14, which is the K this doc was going to impose. Nothing to win in the hall yet. |
| crowd aggregation | Earns nothing until `kept` itself grows — i.e. real crowding, not the hall. |

⇒ **Build the index first.** The other two are correct and stay in this document;
they are what makes the design survive density, and the tell that their time has
come is `kept_frac` climbing back toward 1.0.

⚠ This also confirms the doc's central claim from the other side: population
doubled and `kept` did not move. What sets the work is how many bodies share a
viewport, and the hall's answer is fourteen.

## The three resolutions

Each fighter perceives at three fidelities, each with a **semantic budget**:

```text
EXACT ATTENTION          bounded, ~8-16 actors
  who is attacking me, who I am attacking, nearest threat,
  the projectile about to land, the vulnerable one beside me

PERSISTENT WATCHLIST     bounded, ~4
  boss, rival, escort target, objective carrier, the one I just
  launched and expect to retaliate — important REGARDLESS of distance,
  reached by stable SimId through an index, never by scanning the room

AGGREGATED WORLD         bounded, ~8 groups + one summary
  "7 hostiles approaching from the right", "dense group 18m northeast",
  "nothing southwest"
```

A group is not expanded into members unless one earns promotion.

## Being mobbed is a percept, not 25 percepts

25 surrounding enemies should not become 25 evaluations. They become one fact:

```text
CrowdPressure { count, nearest_distance, centre_of_mass, approach_velocity,
                angular_coverage, combined_threat, escape_gap }
```

plus a handful of exact actors — the one swinging, the one blocking the escape
gap. The brain then decides "I am surrounded, disengage southwest" without
reasoning about 25 fighters. Only after choosing does a particular member get
promoted to exact.

**Density changes the representation, not its size.** This is the invariant.

## Attention is an explicit subsystem

Salience is scored — distance, is-attacking-me, time-to-collision, recent damage
either way, objective importance, power disparity, line of sight, watchlist
membership — and perception says *these are the K things you are attending to*,
rather than the brain scanning the universe.

Behaviour falls out for free: a sudden attack displaces an idle neighbour from
attention; a distant sniper is promoted to the watchlist; an actor untouched for
ten seconds decays from exact into group-level memory.

## Aggregate once, globally

The aggregation is built **once per tick, O(n)**, not rediscovered per actor:

```text
SpatialCell { actor ids, per-faction counts, centroid, mean velocity,
              threat mass, health mass }
```

with one or two coarser levels above it — a small perception pyramid. Start with
a **fixed deterministic grid**, not a quadtree.

```text
today          130 actors x scan 130
target         build index once:            130
               per fighter: ~10 candidates + ~8 summaries + ~4 watchlist
```

⚠ **A grid alone does not solve the mob case.** If 100 actors share adjacent
cells, "query nearby cells" is 100 actors again. Local perception needs its own
budget: salience-select the top K exact, aggregate the remainder.

## The overflow rule is what a grid alone cannot give

⚠ Restating, because the measurement above makes it concrete: the index removes
the scan **while the cast is spread out**. Put 100 actors in adjacent cells and
"query nearby cells" is 100 actors again, `kept` rises with population, and the
O(n²) term returns wearing a different name. The salience budget plus an
aggregate remainder is what stops that, and it is not optional — it is the second
half of the same fix, deferred only because the hall is not yet dense enough to
need it.

## This subsumes targeting

`select_actor_targets` measures 0.059 ms/tick and is **not** worth optimizing as
a loop — but its premise is the same one. Target acquisition should run over the
tactical perception result (attention set, watchlist, promoted group
representatives): 8-20 candidates, not the room. If the brain decides "engage
that group", perception promotes a representative — frontmost, weakest,
highest-threat.

## Cognition budgets replace per-brain hacks

```text
StandStill        self only
AmbientNPC        self + hazards + player proximity
SimpleEnemy       self + current target + local hostile summary + a few exact
TacticalFighter   self + attention + watchlist + groups + objectives + pressure
SmashBot          the same TacticalView, plus expensive rollout over it
```

The rich fighter still gets rich information. It does not get **unbounded**
information proportional to room population.

## Determinism

Fixed cells; the sim's existing coordinate representation; stable `SimId`
ordering; deterministic tie-breakers and salience comparisons; fixed capacities;
no hash iteration deciding anything; no async AI result arriving later.

The spatial snapshot is derived from rollback-authoritative state every tick, so
it should **not** itself be rollback state — that keeps the wire format clean.
See ADR 0023.

## Acceptance

Structural:

```text
room population 200
per fighter: exact <= K, watchlist <= W, groups <= G
```

Empirical, and the test room uses **tactical brains, not `stand_still`** — the
hall's authored cast cannot demonstrate this:

```text
2 -> 16 -> 64 -> 130 -> 200 tactical fighters
decision cost grows approximately linearly
```

> 130 tactical fighter brains must not each construct a 129-actor exact world
> view.

## What must not happen first

⛔ Do not make dormancy the answer. Distant actors sleeping is a legitimate game
policy later; it does not solve the representation problem, and applied to the
hall it deletes the benchmark that finds these defects.
