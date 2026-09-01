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

## ⭐ MEASURED 2026-09-01: what the hall actually costs

Each hall actor is **offered 129 peers and keeps 14.4**, and that number does not
move when the cast goes from 65 to 130 (`kept_frac` 0.225 → 0.112; no body in the
hall is `Omniscient`, so the viewport filter is live for all of them).

```text
kept per actor   ~14.4   CONSTANT in population   the work is already O(n)
offered          n - 1    GROWS                    the scan is O(n²)
```

**The view is already bounded. The scan is not** — 89% of every actor's scan at
130 bodies is discarded. So:

⛔⛔ **AND THE FIRST VERSION OF THIS SECTION WAS WRONG.** It read those two counts
and concluded "the spatial index is the whole win" without measuring the TIME. A
count that grows quadratically is not automatically the expensive one. Measured
at 130 bodies:

```text
scan over discarded candidates   0.032 ms/tick    8%
building and using the ~14 kept  0.285           69%
brain, self view, projectiles    0.098           24%
```

And `kept` **saturates** — 5.00, 5.88, 11.61, 14.63, 14.41 at n = 8, 17, 33, 65,
130 — so `builds = n × kept(n)` grows **linearly** once the room fills.

| piece | verdict today |
|---|---|
| **attention budget (top-K)** | **Worth 10x in the regime it is for.** Measured by driving the tactical extent at fixed population: `kept` 14.4 → 113 takes `Decide` 0.26 → 2.66 ms/tick, at a flat ~23 µs per perceived peer. It wins nothing at the hall's CURRENT sparsity — which is a different statement from "the idea is wrong". |
| **crowd aggregation** | Same regime, same argument. It is what keeps `kept` bounded once a region actually crowds. |
| spatial index | Worth **8-10% today, and it COMPOUNDS.** The scan is the only term measured to grow quadratically — ×4.18 against a predicted ×4.03 — so its share rises with population: ~7% at 65 bodies, ~10% at 130, ~25% at 260. Not urgent; the one piece whose value grows. |

⚠ **THE TWO PIECES DO NOT COMPETE FOR THE SAME MILLISECONDS.** The attention
budget caps a term that is linear in `kept`; the spatial index caps the term that
is quadratic in POPULATION. Density and headcount are different axes and each has
its own defect.

⚠ **CORRECTED 2026-09-01: the slope-1.40 term was `WorldMemory`, not
construction.** Its quadratic membership test sat inside the same probe bucket as
the builds. Fixed, the density curve is now **slope 1.03 in `kept`** — linear —
and `Decide` at `kept`=113 fell 2.66 → 1.99 ms/tick.

⇒ A budget still caps the cost; it caps a **linear** term rather than a
superlinear one. The 10x remains: `kept` 14 → 113 is 0.24 → 1.99 ms/tick.

⭐ **AND CROWDING IS CONSTRUCTION, NOT COGNITION.** The peer-independent brain
term grows ×1.7 across the same range and is **4%** of the phase at 4x extent.
Whatever a brain does with a bigger view, it is not what costs.

⭐ **AND THE SCAN IS FLAT** (×0.91) — it walks the same 129 peers whatever the
extent. Second independent confirmation that the scan is not the expensive half.

⚠ **HOLD `kept` CONSTANT, NOT THE POPULATION.** A room of 200 where each fighter
attends to 16 costs about what 130 attending to 14 costs today. A room of 200
where everyone sees everyone is the 10x case — and it arrives through DENSITY
long before it arrives through headcount.

⇒ **The cost is 1,873 `PerceivedActor` constructions per tick at ~152 ns each.**
A design that bounds the count without making construction cheaper moves 8%.

⚠ **AN OPEN DISCREPANCY BLOCKS ANY 200-ACTOR DECISION.** Past saturation, builds
grow 1.97× between 65 and 130 bodies while `Decide` grows 3.17×, and an 8% term
growing 4× cannot close that gap. Something superlinear there is unattributed —
possibly cache, possibly the brain-tick term. The saturation result says the
shape should be linear; the slope says it is not. Until one of them is shown
wrong, no number here sizes this design.

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

## ⛔⛔ THE FIRST INCREMENT TOUCHES CHECKSUMMED ROLLBACK STATE

The obvious first step — let a brain **declare what it needs** (`None` /
`TargetBelief` / `TacticalWorld`) and build only that — is not a local
optimisation, because two of the things it would skip are rollback state:

```text
PerceptionMemory   rollback_component_canonical, "actor.perception_memory"
Perception         rollback_component_canonical, "actor.perception"
```

A `StandStill` brain that stops building a view also stops calling
`WorldMemory::update`, so its remembered set stays empty instead of tracking the
peers it can see. That is **deterministic** — the gate would read
`StateMachineCfg`, which is itself rollback state — so it does not desync. But it
changes the VALUE of a canonically checksummed component, which means:

- every save and replay made before the change decodes to a different world than
  one made after it;
- the schema baseline moves, and `snapshot_impls.rs` already records what that
  costs: *"a rewind that leaves [`GameplayElapsed`] running makes every memory
  look older than it is — which is exactly how `gnu_ton_arena` diverged on
  `perception_memory` and nothing else."* This component has desynced a match
  before, on a subtler change than deleting its updates.

⇒ **Do not build the declaration as a performance tweak.** It is a wire-format
and replay-compatibility decision that happens to be fast, and it wants the
treatment `ADR 0023` gives that class rather than a benchmark and a merge.

⭐ **AND THE MEASUREMENT SAYS IT IS NOT URGENT.** With the three fixes landed
2026-09-01, `Decide` at the hall's real density is **0.234 ms/tick**, and the
peer-independent remainder — what a `None` brain would still pay — is 0.039. The
whole declaration is worth ~0.2 ms of a 1.78 ms headless frame **today**. Its
value is the seam it establishes for the attention budget, not the milliseconds,
and that is an argument for building it carefully rather than quickly.

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
