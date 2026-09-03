# Bounded perception and attention

**Status:** direction set by Jon 2026-09-01, grounded in measurement, and
**INCREMENT 1 IS LANDED** — this line said "no code written against it" until
2026-09-02, by which point there was.

* ✔ **Increment 1**: `PerceptionRequirement` declared on the brain
  (`None` / `TargetBelief` / `TacticalWorld`), the `None` gate wired into
  `actors/update.rs`, and the rollback schema baseline moved with it. The
  rollback/replay question that blocked it is settled by
  [ADR 0034](../../adr/0034-perception-is-bounded-by-attention.md).
* ◐ **Increment 2**: a cheap `TargetBelief` provider that answers "do I perceive
  a valid target, and where was the last one" WITHOUT building a full
  `WorldView`, and a bounded `TacticalWorld` representation.
  ✔ **THE PROVIDER EXISTS (2026-09-02): `nearest_hostile_peer`**, which borrows
  the peer slice, applies the two filters and takes a min by squared distance —
  **allocating nothing**. The `String` id clone that dominates `PerceivedActor`
  never happens.
  ⭐ **AND THE CUSTOMER IS SEVEN OF THE NINE BRAIN TEMPLATES**, which is the
  fact that makes this worth doing rather than a capability looking for a user:
  `Patrol`, `Skirmisher`, `Sniper`, `ChargeCrash`, `Aerial`, `MeleeBrute` and
  `BossPattern` all declare `TargetBelief` and read only `target_pos` /
  `target_alive`. Only `Smash` and `Fighter` take a `&WorldView`. Until now every
  one of those paid for the full view to answer one question — at the hall's
  density ~14 `PerceivedActor` constructions to use ONE, and 113 at the density
  the table above reaches.
  ⛔ **IT ANSWERS THROUGH THE SAME TWO AUTHORITIES THE FULL VIEW USES.**
  `peer_is_visible_to_body` and `peer_is_hostile_to_body` were EXTRACTED from
  `build_world_view` rather than restated, so there is one definition of "in
  view" and one of "hostile" — the latter being three inputs and a precedence
  (team relation, faction relation, personal grudge), which is exactly what a
  hand-written cheap check gets subtly wrong.
  ⛔⛔ **AND THE EQUIVALENCE IS PINNED, because a cheap road that can disagree is
  worth nothing.** `a_cheap_belief_agrees_with_the_view_it_replaces` and
  `a_cheap_belief_sees_a_same_faction_grudge_the_way_the_view_does` assert the
  two roads name the same hostile, and the same NOBODY. Poison-verified twice:
  restating hostility as faction-only drops the grudge and goes red; dropping the
  viewport filter goes red.
  ⚠ **THE VIEWPORT ARM ONLY DISCRIMINATES BECAUSE OF ITS GEOMETRY, and the first
  version did not.** Its out-of-view foe was also the FARTHEST, so `min_by`
  rejected it whether or not the filter ran — poisoning the filter away left the
  test GREEN while its comment claimed to catch exactly that. The out-of-view
  peer now sits NEARER (340) than the right answer (450), so its exclusion is
  observable.
  ✔ **THE ROUTING LANDED (`0520d3d12`, "Seven of nine brain templates stop
  building a WorldView to find one foe") — this row said STILL OPEN after the
  change shipped.** `actors/update.rs` now reads *"⭐⭐ ONLY `TacticalWorld`
  BUILDS A VIEW NOW"* at the `build_world_view` call site; `TargetBelief` bodies
  take `perception::nearest_hostile_peer`, and the census's `kept` comes from
  whichever road ran rather than from the view.

  ⭐ **And the two-halves problem this row raises was the thing that had to be
  solved, not avoided.** `WorldMemory::update` decays everything NOT seen this
  tick, so a body on the cheap road still owes the memory a seen set —
  `update_from_seen(sim_time, dt, seen)` takes it as borrowed `SeenActor`s, and
  the memory fold runs on BOTH roads.

  ▢ **STILL OPEN:** the bounded `TacticalWorld` representation, which is the
  larger half and is what the density table above prices.

  ⛔⛔ **AND THE ROUTING IS BIGGER THAN "USE THE CHEAP PROVIDER", because the
  belief has TWO halves and only one of them is the nearest hostile.** Read the
  increment's own words — *"do I perceive a valid target, AND WHERE WAS THE LAST
  ONE"*. `believed_target` touches the view twice:

  ```rust
  mem.0.update(view, dt);                       // <- needs the SEEN SET
  view.nearest_hostile().map(|a| a.pos)         // <- needs one peer
      .or_else(|| memory.last_known_hostile())  //    (the memory answers here)
  ```

  `WorldMemory::update` collects `view.actors.iter().map(|a| a.id.as_str())` to
  decay everything NOT seen this tick. So a `TargetBelief` body cannot simply
  skip the build: drop the view and every remembered foe decays as though it had
  left the viewport, and pursuit (invariant I6) silently stops working. ⇒ **A
  cheap road that answered only the first half would look correct on a body that
  can see its foe and quietly break the one that is chasing.**

  ⇒ **The shape that works is to narrow what `update` DEPENDS ON, not to skip
  it**: take an iterator of `(id, pos, hostile)` borrowed from the peers rather
  than a `&WorldView`. The memory keeps its exact semantics — it already borrows
  ids rather than cloning them — and the `PerceivedActor` construction that costs
  ~152 ns each disappears for seven of the nine templates. That is the same
  "publish the thing downward" move D33 used to get the census out of the kernel,
  applied to a data dependency instead of a crate one.

  ⚠ Found by reading `update` before writing the routing, not after. The
  provider and its equivalence guards are right and land as they are; what would
  have been wrong is the call site that used them.
  ✔ **Routed 2026-09-02 (`0520d3d12`):** seven of nine templates stop building
  a view; `update` takes the narrowed iterator.

* ✔ **Increment 3 (2026-09-03): the ATTENTION BUDGET — the `TacticalWorld`
  view is bounded.** `build_world_view` carries at most
  `TACTICAL_ATTENTION` (= 16) peers exactly: the visible peers ranked
  hostile-first, then by squared distance, then by id (deterministic — the
  kept set feeds `WorldMemory`, which is rollback state), cut at the cap, on
  BORROWED peers so no `PerceivedActor` is built for a peer that is not kept.
  What the cut left out is the aggregate remainder,
  `WorldView.remainder: AttentionRemainder { actors, hostiles,
  nearest_unattended_hostile_dist_sq }` — the pressure a crowd exerts even when
  nothing in it is attended. **16 from the measurement, not taste:** the MEAN
  `kept` saturates at ~14.4 in a 130-body hall at the shipped viewport (n = 65
  and 130 alike), so the mean never reaches the cap there — but the densest
  single viewer in that configuration saw 21 and is cut to 16 (`kept_max` 21 →
  16 at 480x320 in yardrat's re-run below; population 130 re-brained to
  `medium_striker`, NOT the shipped cast, which is authored `stand_still` and
  builds no view at all). It binds hard in the regime the budget is FOR, where
  `visible` reaches 124 at 6x the viewport and `kept` reads 16.0 flat; before
  the cap `Decide` went 0.24 → 1.99 ms/tick there, linear in `kept`.
  Guards: `a_crowd_is_attended_to_hostiles_first_and_the_rest_is_counted` (40
  visible → 16 actors, all hostile; the capped view's `nearest_hostile()` is
  the uncapped scan's; the remainder counts 24/4 and names foe_17's distance;
  input order changes nothing) and
  `below_the_attention_cap_the_view_is_what_it_always_was`. The acceptance
  instrument was renamed to what it now shows —
  `offered_saturates_when_bodies_are_spread_grows_when_dense_and_the_budget_caps_kept`
  — and the census row gained a `visible=` term (density, pre-cut) beside
  `kept=` (post-cut, the cost driver); `scripts/measure_perception_density.sh`
  prints both. ⚠ NOT yet built: salience beyond hostility and distance
  (is-attacking-me, time-to-collision, recent damage, watchlist promotion) and
  the crowd's spatial aggregation (`SpatialCell`); the remainder is counts and
  one distance. ⚠ The rollback schema did not move (no new registered state);
  `WorldMemory` VALUES differ from before only where the cap binds — no shipped
  room — and ADR 0034 covers that class. Measured next: the density sweep on an
  untenanted box (yardrat's queue) — the prediction is `visible` climbing with
  extent and `kept` flat at 16. ⛔ **The first sweep (yardrat, 2026-09-03)
  read `kept = visible` on every row (kept_max 129) and it was the CENSUS, not
  the cap:** `TacticalWorld` also `needs_target_belief`, so the cheap road's
  uncapped `visible_count` was recorded first and the view road's post-cut
  count was unreachable for every brain that builds a view. The census now
  reports from the view when one was built (`update.rs`); the re-run is the
  acceptance, and `visible` at 960x640 reading 56.1 — the old table's `kept` —
  is the control that the instrument still measures what it did.

  ⛔⛔ **RAN 2026-09-03, AND THE PREDICTION IS CURRENTLY UNTESTABLE — the CENSUS
  cannot see the budget for the brains the budget governs.** `kept` equals
  `visible` on every arm and `kept_max` reaches 129:

  ```text
  extent        views   offered  visible    kept  kept_max
  480x320       36249     130.0     14.4    14.4        21
  960x640       26703     130.0     56.1    56.1       104
  1440x960      22446     130.0     93.9    93.9       119
  1920x1280     21027     130.0    113.2   113.2       124
  2880x1920     17157     130.0    124.3   124.3       129
  ```

  ⭐ **THE REPORTING BRANCH IS UNREACHABLE, not the cap.** In
  `features/ecs/actors/update.rs` the census records from the cheap road when
  `needs_target_belief()` holds and only ELSE from the built view — and
  `needs_target_belief()` is true for `TargetBelief` **and** `TacticalWorld`,
  while `needs_world_view()` is true for `TacticalWorld` alone. So a world-view
  brain always takes the first arm and is recorded as its uncapped
  `visible().count()`. The arm reporting the post-cap `world_view.actors.len()`
  cannot be reached by any brain that builds a world view. `medium_striker` is
  `template: Smash`, i.e. `TacticalWorld` — exactly the population the budget
  governs and exactly the one the instrument cannot see.

  ⚠ **THIS IS NOT EVIDENCE THE CAP FAILS.** `attend()` reads correctly and the
  unit guards exercise it directly (40 visible → 16, hostiles first,
  deterministic ties). The narrower true statement is that this sweep measures
  the PRE-CUT quantity, so neither half of the prediction can be confirmed or
  refuted by it. ⭐ The row is self-consistent with that reading: `visible` here
  reproduces the old pre-cap `kept` table exactly where the extents match —
  960x640 reads 56.1 against the 56.1 recorded further down this page.

  ✔ **RE-RUN PAST THE CENSUS FIX (`697abd994`), same script, population,
  brain — THE BUDGET WORKS, and the control holds twice over:**

  ```text
  extent        views   offered  visible    kept  kept_max     (before: kept / kept_max)
  480x320       35862     130.0     14.4    12.8        16      14.4 / 21
  960x640       25542     130.0     56.1    15.2        16      56.1 / 104
  1440x960      22059     130.0     93.9    15.7        16      93.9 / 119
  1920x1280     21414     130.0    113.2    16.0        16     113.2 / 124
  2880x1920     19995     130.0    124.3    16.0        16     124.3 / 129
  ```

  `offered` is flat at 130.0 and `visible` reproduces the pre-fix run to the
  decimal, so the only column that moved is the one the commit touched. Two
  readings: `kept` (a MEAN over viewers) approaches 16 rather than snapping to
  it — a viewer at the crowd's edge sees fewer even when the room mean is 56 —
  while `kept_max` binds at 16 from the first arm. And at the SHIPPED viewport
  the densest viewer saw 21 before the cap and 16 after: the budget binds for
  the densest viewers even at 480x320 in this configuration (130 bodies
  re-brained to `medium_striker`; the shipped cast is `stand_still` and builds
  no view, which is what makes the real hall cheap and unmeasurable at once).
  Timings not reported (doc edits ran alongside); counts are the verdict.

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

Re-measured 2026-09-01 on 6000-tick runs with the census's startup window
excluded (the earlier figures here averaged a `ticks=1` window whose every phase
reads 0.000, understating short runs by up to a third):

```text
term         @65      @130   growth   predicted from loop bounds   share of phase
builds+use  0.1183  0.2314    1.96x     1.97x  = n x kept(n)            66%
scan        0.0100  0.0416    4.16x     4.03x  = n x (n-1)              12%
fixed       0.0437  0.0797    1.82x     2.00x                           23%
```

⭐ **EVERY TERM MATCHES THE SHAPE PREDICTED FROM ITS LOOP BOUNDS**, and per-build
cost is FLAT — 125 ns at 65 bodies, 124 ns at 130. An earlier reading of these
numbers had it rising with population and proposed a gather-locality explanation;
that was the startup window, and it is withdrawn.

And `kept` **saturates** — 5.00, 5.88, 11.61, 14.63, 14.41 at n = 8, 17, 33, 65,
130 — so `builds = n × kept(n)` grows **linearly** once the room fills. That is
also why the phase reads slope 1.27 over 9 → 130 but **1.04 over 65 → 130**: it
is superlinear only where `kept` is still rising.

| piece | verdict today |
|---|---|
| **attention budget (top-K)** | **Worth 10x in the regime it is for.** Measured by driving the tactical extent at fixed population: `kept` 14.4 → 113 takes `Decide` 0.26 → 2.66 ms/tick, at a flat ~23 µs per perceived peer. It wins nothing at the hall's CURRENT sparsity — which is a different statement from "the idea is wrong". |
| **crowd aggregation** | Same regime, same argument. It is what keeps `kept` bounded once a region actually crowds. |
| spatial index | Worth **12% of the phase today, and it COMPOUNDS.** The scan is the ONLY term measured to grow quadratically — ×4.16 against a predicted ×4.03, confirmed by prediction rather than fitting — so its share rises with population while every other term's shrinks: ~6% at 65 bodies, ~12% at 130, ~25% at 260. **It is now the only remaining structural term in `Decide`.** |

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

⚠ Those two density figures, and the `2.66`/`0.26` pair above them, come from
1200-tick runs and are **understated** for the same startup-window reason. The
RATIO between them is what the argument rests on and it is unaffected; the
absolute millisecond values are a floor.

⛔ **AND NOTHING ELSE IN `Decide` IS A LEAD.** Construction is linear at constant
per-unit cost, so a narrower `PerceivedActor`, a cheaper actor identity, or
better gather locality would each buy a proportional slice of a linear term — not
a shape change. The struct-width ratchet in `ambition_characters` is a regression
guard, not a lead.

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

✔ **THE DISCREPANCY THAT "BLOCKS ANY 200-ACTOR DECISION" IS RESOLVED, 2026-09-02,
FROM THE NUMBERS ALREADY ON THIS PAGE — THERE IS NOTHING UNATTRIBUTED.** It read:
*"builds grow 1.97× between 65 and 130 bodies while `Decide` grows 3.17×, and an
8% term growing 4× cannot close that gap. Something superlinear there is
unattributed — possibly cache, possibly the brain-tick term."*

⛔ **NO CONSISTENT PAIR OF READINGS ON THIS PAGE GIVES 3.17×.** Taking the
component table above at both populations:

```text
              @65      @130    growth
builds+use   0.1183   0.2314    1.96x
scan         0.0100   0.0416    4.16x
fixed        0.0437   0.0797    1.82x
TOTAL        0.1720   0.3527    2.05x     <- against 2.03x population: LINEAR
share @130     66%      12%       23%     = 100%  <- nothing is unaccounted for
```

The components sum to **100% of the phase at 130 bodies**, so there is no room
for an unattributed term to hide in. And the sweep further down measures the same
thing independently — `Decide` 0.195 at 64 bodies, 0.419 at 130 — which is
**2.15×**, and that section already states the conclusion in as many words:
*"64 → 130 is 2.15x cost for 2.03x population, which is linear … There is no
quadratic here to remove."* Two consistent comparisons, 2.05× and 2.15×; the
page asserted a third, 3.17×, and blocked a decision on it.

⭐ **AND THE LIKELY ORIGIN IS THE DEFECT THIS PAGE ALREADY FOUND ONCE.** For
`Decide`@130 = 0.419, a ratio of 3.17× needs `Decide`@65 = **0.132 ms** — which
is **1.30× smaller** than the 0.172 the components measure. The note directly
above the table says the earlier figures were taken before the census's startup
window was excluded and were *"understating short runs by up to a third"*. A 1.30×
understatement is exactly that. ⇒ The 3.17× is almost certainly a
**pre-correction @65 divided by a post-correction @130**: the table was
re-measured and the conclusion drawn from the old numbers was left standing
beside it.

⚠ **STATED AS WHAT IT IS: ARITHMETIC ON PUBLISHED NUMBERS, NOT A NEW RUN.** No
measurement was re-taken for this — and none should be taken on the shared box,
where a timing reads whoever else is compiling. What is certain is that the page
contains no pair of numbers exhibiting the gap, and that its own accounting
leaves nothing unattributed; the startup-window origin is the best explanation
for where 3.17 came from, not a proven provenance.

⇒ **The 200-actor decision is no longer blocked by this.** Both consistent
readings say `Decide` is LINEAR in population past saturation, which is what
`cost ≈ n × kept(n)` with flat `kept` predicts. ⛔ That is not permission to
size the design from headcount: the section below is still right that the hall
saturates because it is SPARSE, so density — not population — is the axis that
breaks this, and a dense-room measurement is still owed.

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

### ⭐ CORRECTION 2026-09-01: these are two rollback classes, not one

The section above names `Perception` and `PerceptionMemory` together. They are
not alike, and the difference decides how much of this is actually a wire
problem.

**`Perception` is a policy, not a view.** It is the enum
`Omniscient | Sighted { viewport_half }`, and the ONLY runtime construction of a
value is `perception.rs:449`, always `Sighted { DEFAULT_VIEWPORT_HALF }`.
`Omniscient` is reached exclusively through `Default` on an absent component. So
every `Perception` in the world holds the identical value, and its presence is
*by construction* the same fact as `PerceptionMemory`'s presence —
`ensure_perception` inserts both together and gates on `Without<PerceptionMemory>`,
which its own comment states: *"Missing memory ⟺ missing perception"*.

The doc comment at `perception.rs:22` says *"a per-body override rides in
`Perception::Sighted` for a character that wants"* one. Nothing overrides it.
That sentence describes a capability, not a behaviour — treat it as a TODO.

**`PerceptionMemory` is accumulated history** and genuinely durable: it cannot
be re-derived from the current tick, which is the whole point of a belief store
that survives losing sight of a foe.

⇒ Gating construction changes the VALUE of the second. It changes nothing about
the first, whose value is invariant.

### ⭐ AND THE HOST THE VERDICT WAS MEASURED ON IS NOT THE SHIPPED ONE

The "not urgent" verdict above rests on `Decide` costing 0.234 ms/tick. That
number came from the **direct sandbox**, which installs no rollback host at all
— confirmed by the schedule census, no `GgrsSchedule` among 20 schedules. The
shipped host advances the sim inside `GgrsSchedule` instead.

⛔ It does **not** follow that the wire is paid every frame. I claimed that and
it is wrong: ggrs skips all saving at `check_distance: 0`, which is what a local
session runs at, so ordinary play saves nothing. See
`performance-and-iteration.md` for the refutation.

⇒ `PerceptionMemory`'s size is a **cognition** cost, not a per-frame wire cost.
The replay-compatibility argument at the top of this section stands on its own —
gating construction changes a canonically checksummed VALUE, and that governs
saves and replays whether or not a frame pays for it — but it must not be argued
from a per-frame number, because there isn't one.

## ⭐ MEASURED 2026-09-01: what a cast that NEEDS perception costs, by population

The gate (increment 1) makes `None` cheap. This is the other arm — the hall
re-brained with `AMBITION_ACTOR_BRAIN_PROFILE=ambition::medium_striker`, whose
`template: Smash` is one of the two that consume a `WorldView`. Same room,
headless, 3 reps per point, medians.

```text
bodies                          2       16       64      130
Decide, TacticalWorld       0.009    0.033    0.195    0.419
Decide, None (contrast)     0.003    0.005    0.012    0.023
```

**18x at 130 bodies.** That difference is the whole cost of supplying perception
to a cast that reads it, and it is what an attention budget has to bound.

⭐ **AND THE SHAPE IS THE ARGUMENT.** Growth against population:

```text
 2 ->  16   pop 8x     cost 3.7x     sublinear  (kept set still small)
16 ->  64   pop 4x     cost 5.9x     SUPERLINEAR
64 -> 130   pop 2.03x  cost 2.15x    linear     (kept saturates ~14)
```

The superlinear band is exactly where each viewer's kept-peer set is still
growing with the room. It flattens above 64 only because the hall is SPARSE —
`Perception::Sighted`'s viewport stops admitting more peers once the gallery
spreads them out.

⛔⛔ **A DENSE ROOM WOULD NOT FLATTEN.** In a melee where fighters are packed
inside one another's viewports, `kept` keeps rising with population and the
16→64 behaviour is what continues. So this curve **understates** the problem it
is measuring: the hall's saturation is a property of its geometry, not a law.

⇒ This is the acceptance criterion's real target. With an attention budget,
`Decide` should be linear across the WHOLE range with a smaller constant, and the
superlinear band should disappear rather than merely saturate.

## ⛔⛔ MEASURED 2026-09-01: the hall CANNOT demonstrate why attention is needed

`[census] perception` now reports what every viewer was OFFERED and what it
KEPT, so `kept ≈ 14` stops being a number remembered from a probe nobody can
re-run. Tactical cast, same room, population swept:

```text
bodies   offered    kept   kept_max   Decide (3-rep median)
     2       3.0     2.0          2    0.009
    16      17.0     5.6          7    0.033
    64      65.0    14.7         21    0.195
   130     130.0    14.4         21    0.419
```

⭐ **KEPT SATURATES.** From 64 to 130 the room doubles what it offers and each
viewer keeps the same 14.4 (max 21, unchanged). `Perception::Sighted`'s viewport
is already an attention budget — a crude, geometric one — and in THIS room it
binds at about fourteen.

⇒ **An explicit budget of K≈16 would change almost nothing here**, and that is
the honest verdict on the hall as an acceptance room. It cannot show the problem
because its geometry already solves it.

## ⭐⭐ MEASURED 2026-09-02: the hall CAN reach the dense regime — by EXTENT, not population

The row above says the hall cannot demonstrate why attention is needed. That is
true of its POPULATION axis and not of its density axis, and the difference is
now measurable rather than argued. `scripts/measure_perception_density.sh` pins
population and sweeps `AMBITION_PERCEPTION_VIEWPORT_HALF`; the FULL hall cast,
`ambition::medium_striker`, 600 ticks, hall_bench (the shipped rollback host):

```text
viewport_half        offered    kept   kept_max
480x320   (shipped)    130.0    14.4         21
680x453   (1.4x)       130.0    34.2         54
960x640   (2x)         130.0    56.1        104
1360x907  (2.8x)       130.0    82.8        116
1920x1280 (4x)         130.0   113.2        124
```

⭐ **`offered` IS FLAT AT 130.0 ACROSS EVERY ARM**, which is the control that makes
the rest readable: the scan walks every peer whatever the viewport, so nothing
but `kept` moved and the result is attributable to extent alone. The script
refuses to interpret a run whose `offered` drifts.

⭐ **AND `kept` NEVER REACHES THE POPULATION**, which is what makes these arms a
measurement of EXTENT rather than of the cap. `kept_max` tops out at 124 of a
possible 130, so the viewport is still the binding constraint at every point.
⛔⛔ **A FIRST VERSION OF THIS TABLE WAS CAPPED AT 64 AND ITS TOP TWO ARMS BOTH
READ `kept=64.0`** — a full mesh, every fighter seeing every other. That is a
ceiling I imposed, not a saturation I found, and quoting "4x keeps everything"
off it would have said nothing about cost scaling. **A ceiling you set yourself
is not a saturation you discovered** — the same error as reading `kept` flat
across 65 → 130 bodies and concluding the geometry solves it, one level up.
Caught by 383484 before it was quoted.

⇒ **The shipped viewport is doing real work**: 130 offered → 14.4 kept, a 9x cut.
One 1.4x widening more than doubles it; 4x reaches **113.2**, which is the
`kept`=113 the cost row above was measured at. So the density point that row
prices is reachable in the hall, with the cast and the room unchanged, by a knob
instead of a probe nobody can re-run.

⛔⛔ **THIS IS A COUNT, NOT A COST, AND MUST NOT BE QUOTED AS ONE.** It says how
many `PerceivedActor`s get built per viewer, not what they cost. The 10x cost
claim above still rests on timings, and timings on THIS box are a reading of who
else is compiling — five identical hall runs the same day gave frame-spike totals
of 61, 4, 9, 6, 52 while every count was byte-identical. (Measured 2026-09-02 on
an untenanted VM: the same instrument is stable to 2.2% there. The caveat belongs
to the shared box, not to the instrument.) These counts reproduced exactly across
interleaved reps.

⛔⛔ **AND THE CAST MUST BE RE-BRAINED OR THE MEASUREMENT IS OF AN EMPTY
POPULATION — read this as a PREMISE of every hall perception measurement, not a
footnote.** The hall is authored `stand_still`; increment 1's
`PerceptionRequirement::None` gate means such a brain never builds a `WorldView`;
`note_world_view` is called INSIDE that build, so nothing is recorded and the
census row carries **no `kept=` field at all**. The first sweep printed "NO CENSUS
ROW" for every arm, which is the harness refusing to invent data. ⇒ The tell is
an ABSENT field, not a small number — and an instrument reporting NOTHING looks
exactly like an instrument reporting a little. The gate that made the hall cheap
is the gate that makes it unmeasurable.

⭐ The knob itself is `PerceptionExtentOverride`, a value in
`ambition_characters::perception`, published by `ambition_dev_tools` and read by
`ensure_perception` as a resource — **the same inversion as the population cap,
deliberately**, because D33 removed the actor kernel's three developer reads and
an environment read inside `ensure_perception` would have added a fourth.

⚠ AND IT KILLS A CONCLUSION I WAS ABOUT TO DRAW. Seeing kept flat while `Decide`
still doubled, I reasoned the cost must therefore be the SCAN over offered peers
— n viewers x n offered — and that a spatial index was needed after all. The
3-rep numbers refuse it: 64 → 130 is 2.15x cost for 2.03x population, which is
linear. `cost ≈ n × kept(n)` still fits, kept is flat, so cost is linear. There
is no quadratic here to remove.

## What this means for the attention work

The value of a bounded representation is **not** in this room and not in the
current numbers. It is in a room where the viewport does NOT bound the kept set —
a dense melee, where fighters stand inside one another's viewports and `kept`
rises with population instead of saturating at 14.

⇒ The acceptance room must be built to make `kept` grow. A sparse gallery of 200
would still keep ~14 each and prove nothing. The criterion is not "200 bodies",
it is **"a room where kept tracks population"**, and then showing a budget flattens
it.

⛔ Until such a room exists, do not justify attention work with a hall
measurement. The hall says the current design is adequate for the hall.

### ✔ THE ACCEPTANCE INSTRUMENT EXISTS, and it measured both rooms (2026-09-02)

`offered_saturates_when_bodies_are_spread_grows_when_dense_and_the_budget_caps_kept`
(`features/ecs/perception/tests.rs`; renamed 2026-09-03 when the budget landed
— it now also asserts `kept` flat at the cap while `offered` climbs).
`build_world_view` is a pure function over slices, so the dense room needs no
`App`, no schedule and no fixture of `tick_actor_brains`'s 145-line signature —
one viewer, N peers, count `view.actors` + `view.remainder.actors`.

```text
kept per viewer          16     64    130    200
  SPARSE (120px apart)    9      9      9      9     <- the hall's shape
  DENSE  (4px apart)     16     64    130    200     <- kept tracks population
```

⇒ **The prediction above is confirmed exactly.** A sparse room saturates at 9 and
population stops mattering before 16, which is why a hall measurement cannot
justify attention work. A dense room tracks population 1:1, which is the room a
budget has to flatten.

⛔ It asserts the SHAPE, not the constants: saturation in one arm, growth in the
other, and that the two arms differ at the same population. Pinning 9 or 200
would break on any viewport tuning and would say nothing about either property.
Poison-verified by giving both arms the same spacing — the two rooms collapse
into one and the dense assertion fails naming `[9, 9, 9, 9]`.

⚠ What it does NOT measure: build TIME, watchlist size, group counts or crowd
representation. Those are the acceptance criteria for the bounded representation
ITSELF and want the same instrument extended once there is something to compare
against. This one answers only the prior question — does a room exist where the
budget would matter — and the answer is yes, cheaply.

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

### ⛔⛔ CORRECTION 2026-09-01: THAT CRITERION IS ALREADY MET, AND PROVES NOTHING

Measured with `[census] perception`, tactical cast, population swept:

```text
bodies   offered    kept   kept_max
    64      65.0    14.7         21
   130     130.0    14.4         21
```

**No fighter constructs a 129-actor view today.** `Perception::Sighted`'s
viewport already bounds the kept set at ~14 (max 21), and it stays there when the
room doubles. The quoted criterion passes on the CURRENT code.

And "decision cost grows approximately linearly" passes too: 64 → 130 costs
2.15x for 2.03x population.

⚠ The prose above already knew this — *"HOLD `kept` CONSTANT, NOT THE
POPULATION"* — while this section asked for a population. The population was
never the variable.

**The criterion that actually tests the work:**

```text
a room where kept TRACKS population rather than saturating
  (fighters inside one another's viewports — a melee, not a gallery)

without a budget:  kept rises with population, decide cost rises with it
with a budget:     kept <= K at every population, cost flattens
```

⇒ Building a 200-body sparse room would satisfy the old wording and measure
nothing. The room has to make `kept` grow before a budget can be shown to stop
it.

## What must not happen first

⛔ Do not make dormancy the answer. Distant actors sleeping is a legitimate game
policy later; it does not solve the representation problem, and applied to the
hall it deletes the benchmark that finds these defects.
