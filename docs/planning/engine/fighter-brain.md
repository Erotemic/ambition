# The Advanced Fighter Brain — a level-9 CPU that doesn't cheat

> **FB6 was UNIMPLEMENTABLE as written; the redesign is §12 (fable,
> 2026-07-30).** The original FB6 named the custom rollback engine ADR 0027
> DELETED, priced its budget in wall-clock milliseconds inside a deterministic
> sim, and proposed seeding a scratch world from a view that cannot reconstruct
> one. §12 replaces all three: rollouts run on a **shadow model built only from
> `Perceived` facts**, under an **exact step budget** that is data, with a
> **deterministic predicted opponent**. FB1–FB5 below remain accurate and
> verified against source. Implement FB6 from §12 only — §5's pinned budget
> contract is superseded and kept for the record.

**Authored by fable, 2026-07-05.** The plan for an opponent that plays like
a top-level Smash CPU while obeying the same constraints as a human: it
reads only what a player could know, acts only through the controller seam,
and its skill comes from *prediction and option quality*, not privileged
state or frame-perfect reflexes. This brain is ALSO infrastructure: it is
the automated playtester the [boss-design pipeline](boss-design.md) scores
fights against, and the DI/tech-skill exerciser the
[combat model](combat-model.md) tests with.

**The no-cheat contract (hard rules):**

1. **Perception:** reads `WorldView` (the one world-out seam) only. If the
   fight needs the brain to know something, the VIEW grows a field a human
   could also perceive (positions, velocities, move phase/animation state,
   damage meters, stage geometry) — never a private query. (This resolves
   the U5 tension by policy: privileged channels are for RL research rigs,
   never for shipped difficulty.)
2. **Action:** writes `ActorControl` through `Brain::tick` like every brain.
   The body enforces (two-port); the brain physically cannot do anything a
   player's controller couldn't.
3. **Human-rate constraints as DATA** (the difficulty ladder, §4): reaction
   latency (a perception delay-buffer — the brain sees the world N ms late),
   an APM/input-rate cap, and execution noise (aim/timing jitter). Level 9
   = small numbers, never zero. Difficulty NEVER scales damage or reads the
   future.

---

## 1. Architecture: three layers over the existing brain seam

Lives in `ambition_characters::brain` beside the existing smash-brawl
template (which becomes this brain's v0/fallback tier).

- **L1 — Situation classifier** (cheap, every tick): derives the tactical
  state from the view — `Neutral`, `Advantage` (opponent in hitstun/landing),
  `Disadvantage` (self in hitstun/shield-broken/cornered), `Recovery`
  (offstage/knocked out of arena), `EdgeGuard` (opponent recovering). Pure
  function of the view; unit-tested per scenario fixture.
- **L2 — Option generator + utility scorer**: per state, enumerate legal
  options from DATA — movement verbs from the body's capability mask, and
  attacks from the **frame-data table** (CM7: startup/active/recovery/
  cancel windows derived from the actual `MoveSpec`s — the brain knows its
  moveset the way a player who read the frame data does, and automatically
  understands any NEW character it's put in, which is what makes it work
  for every roster row and for bosses). Score = authored utility features
  (range vs. option reach, frame advantage, kill potential at victim's
  damage meter, stage position risk) with per-difficulty weights.
- **L3 — Forward-model rollouts (the frontier feature):** because the sim
  is deterministic and headless, the brain can SIMULATE its top-k candidate
  options a short horizon forward (5–20 ticks) against a predicted opponent
  policy (v1: opponent continues current move / repeats last-seen habit),
  and pick by simulated outcome (damage delta, position delta, KO events).
  This is exactly the architecture our engine is uniquely shaped for — the
  RL/headless discipline pays out as an opponent that genuinely
  outreads you. Budgeted: rollouts run on the snapshot seam (netcode N3.1)
  with a per-tick compute cap; below the cap or before N3.1 lands, L2's
  scores act alone (graceful degradation, so L3 is an upgrade, not a
  dependency).

**Opponent modeling (the "reads"):** a small frequency memory over the
opponent's observed choices in bucketed situations (tech direction, ledge
option, approach habit) with decay. Level-9 reads = sampling the model;
lower levels ignore it. Bounded, inspectable, and it's the honest version
of what human top players do.

## 2. Where it runs

One brain, many costumes: the same stack drives (a) SSB demo opponents,
(b) sparring partners in Ambition's duel arenas, (c) the boss pipeline's
playtester (driving the PLAYER side against candidate bosses), (d) RL
baselines/evaluators. Bosses themselves keep `BossPattern` authored
choreography as their spine (fights are authored, not emergent — see
boss-design.md) but may mount L1/L2 for their neutral-game glue via the
existing brain composition.

## 3. The evaluation harness (how we know it's good)

All headless, all CI-able:

- **Scenario suite:** fixture situations (ledge trap, juggle escape,
  recovery from each offstage quadrant, projectile camping opponent) with
  pass metrics (survival %, damage ratio) per difficulty.
- **Ladder self-play:** level N must beat level N-1 in ≥ 60% of headless
  matches (monotonicity gate); level 9 vs. scripted sandbag must exceed a
  damage-efficiency floor.
- **Humanity checks:** input-rate histograms within the APM cap; reaction
  distributions match the configured latency (no accidental cheating via
  same-tick perception — assert the delay buffer is on the ONLY read path).
- **Feel pass:** Jon fights it. BLIND-commit rule applies to weights.

## 4. The difficulty ladder (all data)

`FighterBrainProfile` (RON): `reaction_ms` (L9 ≈ 150, L1 ≈ 500),
`apm_cap`, `execution_noise` (timing/aim jitter σ), `rollout_depth` /
`rollout_k` (0 disables L3), `read_weight` (opponent-model usage),
`utility_weights` (aggression/safety/style). Levels 1–9 are nine authored
rows. Games/demos ship their own rows — it's content.

## 5. Design sketch (pre-solved data structures)

```rust
/// L2's working set, rebuilt per decision tick (not per frame — decide at
/// ~10–20 Hz gated by reaction latency, hold intents between decisions).
pub struct OptionSet {
    pub movement: Vec<MoveOption>,          // from the capability mask
    pub attacks: Vec<AttackOption>,         // from the CM7 frame-data table
}
pub struct AttackOption {
    pub move_id: String,
    pub frames: MoveFrameData,              // startup/active/recovery/cancels/reach
    pub score: f32,                         // Σ weight_i · feature_i
}
/// The opponent model: bucketed frequency counts with exponential decay.
pub struct HabitModel {
    // key: (SituationBucket, TheirChoice) → decayed count
    counts: HashMap<(u16, u16), f32>,       // NOT iterated in sim order —
                                            // read-only lookups, determinism-safe
    pub decay: f32,
}
/// L3 rollout, budgeted:
/// for opt in top_k(options):
///   let snap = snapshot.take(scratch_sim);        // netcode N3.1 on a scratch world
///   scratch_sim.inject(self_slot, opt.as_controls());
///   scratch_sim.inject(opp_slot, predicted(habit_model));
///   let score = step_n(scratch_sim, horizon).evaluate();  // Δdamage, Δposition, KO
///   snapshot.restore(scratch_sim, &snap);
/// pick argmax; UNDER BUDGET ONLY (wall-clock cap per decision; else L2 score).
```

The perception delay-buffer is a `VecDeque<WorldView>` of length
`reaction_ms / tick_ms` wrapped around the ONE view read — assert in
tests that no L1/L2/L3 code path reads the live view directly.

**FB6 budget contract (pinned 2026-07-06 — SUPERSEDED by §12, kept for the
record of what was wrong):** rollouts run on a SCRATCH
sim world (never the live one), horizon 5–20 ticks, `top_k ≤ 4`, and a
**wall-clock cap of 2 ms per decision tick** (decisions at 10–20 Hz, so
≤ 4% of a 60 Hz frame worst-case); when the cap trips mid-evaluation the
brain uses the best fully-evaluated option, and with `rollout_depth = 0`
or N3.1 absent, L2 scores act alone — L3 is an upgrade, never a
dependency. Rollout outcome score v1 = `Δ(their damage meter) − Δ(own
meter) + KO_bonus − position_risk` with the same per-difficulty
`utility_weights` as L2 (one weight vocabulary, two horizons). Allowed
omniscience inside a rollout: NONE beyond the no-cheat contract — the
scratch world is seeded from the DELAYED view's reconstruction, and the
opponent is driven by the predicted policy, not their real controller.
Scoring weights are NOT divined up front: v1 weights are authored
starting values, then FB4's ladder self-play monotonicity gate is the
calibration instrument (adjust until levels order correctly). Anything
beyond that (learned weights, deeper search) is post-1.0 research, not
this track.

## 6. Slices

| # | Slice | Grade |
|---|---|---|
| FB1 | ~~View audit for the no-cheat contract~~ ✅ **DONE 2026-07-10** — see §7 | [opus] |
| FB2 | ~~Frame-data table consumer (needs CM7) + L2 option generator/scorer~~ ✅ **DONE 2026-07-10** — see §9 | [opus] |
| FB3 | ~~L1 classifier + scenario fixture suite~~ ✅ **DONE 2026-07-10** — see §8 | [opus] |
| FB4 | 🟡 **(1) profiles + (2) the reaction/delay-buffer humanity check DONE 2026-07-10** — see §10. (2)'s APM histogram and (3) the ladder rig need a brain that emits inputs | [opus] |
| FB5 | ~~Opponent-model memory (bucketed frequencies, decay)~~ ✅ **DONE 2026-07-10** — see §11 | [opus] |
| FB6 | L3 rollouts — redesigned in **§12** (shadow model over `Perceived`, exact step budget, deterministic prediction); slices FB6a–FB6e there | **[opus, fable-specced — §12 is the spec]** |

Sequencing: FB1–FB4 need only landed systems + CM7 and deliver a credible
mid-level CPU; FB5 makes it scary; FB6 makes it level 9. SSB demo ships
with FB1–FB4 minimum.

---

## 7. FB1 — the view audit, and what it found (opus, 2026-07-10)

**Answer to the card's question: no, the view did not carry move phase, damage
meters, or stage geometry — and two of the fields it DID carry were wrong.**

### The view now carries

| Field | Where | Why the contract needs it |
|---|---|---|
| `BodyPhase` + `phase_remaining` | `SelfView`, `PerceivedActor` | §1 names *"move phase/animation state"*. `Neutral / Hitstun / AttackStartup / AttackActive / AttackRecovery / Shielding`, with `is_punishable()` — active frames are NOT a punish window, and that distinction is L2's whole game. Derived once, in `body_phase()`, from `BodyCombat` + `BodyMelee` + `BodyShieldState`; hitstun outranks a swing (a body knocked out of its own attack is reeling), a swing outranks a shield. |
| `invulnerable` | both | i-frames. Perceivable: the body flashes. |
| `damage_taken` + `health_max` + `damage_frac()` | both | §1's *"damage meters"*, CM1's smash-percent axis. L2 cannot score kill potential without the victim's meter. |
| `WorldView.stage: StageView` | the view | §1's *"stage geometry"*. **NOT viewport-clipped** — a fighter can see the blastzones. `offstage()` is L1's `Recovery` predicate and `actor_offstage()` is `EdgeGuard`; without them those two states are undecidable. `distance_to_edge()` is L2's corner-pressure feature. Its bounds are the room's world AABB — the same envelope CC3's invariant 3 polices, so "offstage" and "out of bounds" mean the same thing in both places. |

`StageView::default()` is the **empty** box (inverted bounds), so every point
reads offstage. The first draft used a zero-size box at the origin, which made
the origin — and only the origin — read as safe. That is the kind of quiet lie a
perception type must not tell.

### Two bugs the audit surfaced

1. **The 2× half-extent.** Both fill sites (`PerceptionBody` in
   `actors/update.rs`, `PerceptionPeer` in `collect_perception_peers`) passed
   `BodyKinematics::size` — the **full** body size — into a field contracted as a
   **half** extent. Every body perceived itself and everyone else as twice its
   real box, and `WorldView::reachable`, which sweeps `self_view.half_extent`,
   refused corridors the body physically fits through. Fixed, and pinned by
   `the_views_half_extent_is_a_half_extent`, which asserts the observable
   consequence (a real sweep through a real gap) rather than the call sites.
2. **`on_ground` and `shield_raised` were hardcoded `false` for every peer.** The
   old comment said *"no consumer reads them; wire them when a brain needs them."*
   A view that lies until someone reads it is worse than a view that lacks the
   field: FB1's L1 classifier is exactly that reader, and it would have concluded
   nobody is ever grounded or guarding. Now read from `BodyGroundState` /
   `BodyShieldState`.

Self's phase and i-frames come from the **same** per-tick peer snapshot everyone
else's do, so a body cannot read itself more precisely than its opponent reads it.

### The perception delay-buffer

`ambition_characters::perception::DelayedPerception` — a `VecDeque<WorldView>` of
length `delay_ticks + 1`, `observe()`d by the gameplay layer, `perceive()`d by the
brain. `from_reaction_ms(ms, hz)` converts a `FighterBrainProfile` row (150 ms →
9 ticks at 60 Hz; 500 ms → 30).

**Warm-up is deliberately stale, never fresh.** Before the buffer fills it returns
the *oldest* view it holds, so a brain spawned mid-fight reacts more slowly than
its profile for a few ticks and never gets a same-tick perceive→act at the exact
moment a fight begins — which is the moment FB4's humanity check is watching.
`clear()` (respawn / room change) blinds the brain for a tick rather than
stranding it on a picture of the old room.

`delay_ticks == 0` is a legal profile (RL rigs, regression fixtures) and returns
the live view. Shipped difficulty rows never use it — §1.3: *"Level 9 = small
numbers, never zero."*

### Left for FB4, on purpose

~~The buffer exists; **nothing yet forces a brain through it.**~~ ✅ **FB4a closed
this 2026-07-10, and not with a lint.** `Perceived` has a private field and only
`DelayedPerception::perceive` mints one; L1 and L2 take a `Perceived`. A brain
layer that wanted the live world would have to edit `perception.rs` to name it. See
§10.

`AttackRecovery` carries `phase_remaining = 0.0`: the sim keeps no endlag clock
today. CM7's frame-data table is what gives it one, which is why FB2 depends on
CM7 and this field is already in the struct.

~~`brain::smash::arena::Stage` … `StageView` should subsume it when FB3's fixture
suite lands.~~ **Revisited when FB3 landed: they stay separate, and the prediction
was wrong.** `arena::Stage` is the self-play arena's AUTHORED geometry — a fixture
the harness builds a world from. `StageView` is what a body PERCEIVES of a stage.
They hold the same four numbers and mean different things, and collapsing them
would put a perception type in the arena's constructor. Same shape, different
authority.

---

## 8. FB3 — L1, and the fixtures it is judged by (opus, 2026-07-10)

`ambition_characters::brain::fighter::{situation, scenarios}`. Both pure; neither
touches Bevy.

### L1

`classify(&WorldView) -> Situation`. A pure function of the view and nothing else,
which is the no-cheat contract's first clause — and the reason FB1's audit had to
come first. Before it the view carried no move phase, no damage meter, and no
stage geometry, so **three of L1's five states were not derivable at all.**

**The states are RANKED, and the rank is the design.** Two facts can hold at once —
you can be offstage and in hitstun, or juggling an opponent while cornered — and
L1 answers one question: *what is this tick about?*

1. `Recovery` — self offstage. A stock lost to the blastzone is not repaid by a
   punish.
2. `Disadvantage` — self in hitstun, or cornered.
3. `EdgeGuard` — the opponent is offstage.
4. `Advantage` — the opponent is punishable.
5. `Neutral` — nobody has anything.

**Disadvantage outranks EdgeGuard on purpose:** a player who chases an offstage
opponent while himself in hitstun is not edge-guarding, he is being carried. The
precedence IS the enum's declaration order, and a test says so, so inserting a
variant in the middle fails loudly.

Two thresholds live in the module rather than in a difficulty profile, because
they are facts about the STAGE and the KIT, not about difficulty: `cornered`
(< 120px of stage behind you — you have lost your retreat option, not your life)
and `landing` (airborne, descending faster than 60px/s **along `gravity_down`**,
because a fight under rotated gravity is the same fight). A level-1 CPU and a
level-9 CPU agree about whether they are cornered; they disagree about what to do
next, and that is L2's job.

`Advantage` deliberately excludes an opponent's ACTIVE frames. That is where the
hitbox is, and walking into it is not a punish.

### The scenario suite

Eight fixtures in `scenarios::suite()`, each a named `WorldView` plus the one fact
everyone agrees on before any brain runs: which `Situation` it is. §3's four —
ledge trap, juggle escape, projectile camper, edge-guard window — plus **recovery
from each of the four offstage quadrants**, which §3 asks for and which is four
fixtures, not one: a body knocked off the top has different options from one
knocked off the side, and a classifier that conflates them is not caught by a
single case.

They live in the LIBRARY, not in a `#[cfg(test)]` module, because FB4's ladder rig
scores survival % and damage ratio *over these same eight situations*. A fixture
suite only a test can see gets rebuilt, slightly differently, by the next slice.

**The metrics half is not here**, and cannot be: survival % and damage ratio need a
brain to survive and deal damage, and nothing above L1 exists. FB4 brings the
profiles and the rig; these scenarios are what it will run.

---

## 9. FB2 — L2, the option generator and scorer (opus, 2026-07-10)

`ambition_characters::brain::fighter::options`. Pure: every input is the
`WorldView` the no-cheat contract allows, plus the body's own kit and its
difficulty's `UtilityWeights`.

- **Movement verbs from the capability mask.** `SelfView`'s `can_dash` /
  `can_shield` / `can_blink` gate what L2 may propose, so the brain physically
  cannot ask for what the body would refuse (invariant I3).
- **Attacks from CM7's frame-data table.** `reach` and `startup_s` come from
  `MoveFrameData`, not from a table someone typed — which is precisely what lets
  the brain *understand a character nobody wrote a table for*. At 100px it throws
  the 100px jab; at 400px, the lunge. Nobody said so; the frame data did.
- **`score == Σ weight_i · feature_i`, by construction**, and the four unweighted
  features ride along on every `AttackOption` so a failing ladder run can be READ
  rather than guessed at. Zeroed weights make every attack score zero — the
  ablation that proves nothing is smuggled in outside the dot product.
- **`Recovery` offers no attacks at all.** Not a low-scoring one; none. A body past
  the blastzone has exactly one problem, and it is not a preference.
- **Ties break on the move id**, so `best_attack` is a function of the world and
  not of how a content author sorted a RON file (ADR 0023).

`stage_risk` is a COST (`w = -0.8`): committing near a blastzone is how a level-9
CPU dies to a level-3 one, and a negative weight means it can never be bought back
by kill potential alone.

### A gap in §1's own four features, found by building them

> ✅ **CLOSED via option (1) — verified 2026-08-07.** Both halves landed:
> `MoveFrameData::max_damage` exists (documented as *"the move's POWER"*), and
> `UtilityWeights::expected_payoff` is the fifth feature, computed as *"(this
> move's `max_damage` ÷ the kit's strongest …)"*. The jab-out-scores-a-smash hole
> this section describes is priced.
>
> ⛔⛔ **AND THE CALIBRATION NEVER REACHED THE GAME, which is the part worth
> keeping.** This section's discipline was followed exactly — the weight was not
> "divined up front", it was built and then CALIBRATED per rung in
> `fighter_brain_ladder.ron`, whose own comment names this hole: *"`expected_payoff`
> is FB6a's fifth feature — it prices a move's POWER on a plausible landing,
> closing the hole FB2 recorded (a jab out-scored a smash on any punish they both
> fit)."* The ladder authors it as `0.00` on the low rungs and `0.50` at level 9,
> because **not noticing which move hits harder IS a difficulty statement**.
>
> ⚠ **the game never read the ladder.** Until 2026-08-07 nothing loaded
> `fighter_brain_ladder.ron`, so every fighter took
> `FighterBrainProfile::for_level`, which hands every rung
> `UtilityWeights::default()` — and `default()` is `v1()`, which is the authored
> LEVEL-9 row. So the feature shipped, the calibration shipped, and a level-1 CPU
> priced a smash exactly as a level-9 did. ⭐ the failure was not in the doctrine
> this section defends; it was one wiring step after it, and no test could see it
> because every test parsed the ladder file itself.


**None of the four reads a move's POWER.** `kill_potential` is the *victim's*
meter; `reach_fit` and `frame_advantage` are geometry and timing; `stage_risk` is
about me. So at ANY weights, given a punish window both a jab and a smash fit, the
jab wins: it is faster, therefore has more frame advantage, and nothing prices the
smash's payoff. **A level-9 CPU that always jabs its punishes is not a level-9
CPU.**

CM7's `MoveFrameData` carries no damage or knockback either, so L2 could not price
it even if a fifth feature existed. Two ways out, and this slice takes neither on
its own authority:

1. Derive `max_damage` / `max_knockback` into `MoveFrameData` — a pure derivation
   over the Active volumes, exactly like `reach` — and add an
   `expected_payoff = damage × landing_chance` feature.
2. Let **FB4's ladder** discover that the weights cannot order the levels, and
   force the question.

§FB6 is explicit that *"scoring weights are NOT divined up front … FB4's ladder
self-play monotonicity gate is the calibration instrument,"* so (2) is the
doctrine's own answer, and (1) is what it will ask for. Recorded here rather than
patched by inventing a fifth weight nobody has calibrated.

**The unit tests reflect that discipline**: they assert FEATURE properties and
structural invariants, never *which move wins a scenario at v1 weights*. That is a
calibration claim, and calibration is FB4's.

### The decision cadence is not here

§5: *"rebuilt per decision tick … decide at ~10–20 Hz gated by reaction latency,
hold intents between decisions."* The latency lives on
`FighterBrainProfile.reaction_ms`, which is FB4's. L2 is a pure function a decision
tick calls.

---

## 10. FB4a — the delay buffer stops being a promise (opus, 2026-07-10)

`ambition_characters::brain::fighter::profile`, plus
`game/ambition_content/assets/data/fighter_brain_ladder.ron` (§4: *"Games/demos
ship their own rows — it's content."*).

### The humanity check that is now a TYPE

§3 asks for a test that *"the delay buffer is on the ONLY read path"* and that
there is *"no same-tick perceive→act"*. FB1 built the buffer and said out loud
that nothing forced a brain through it.

Nothing has to. **`Perceived` has a private field, and only
`DelayedPerception::perceive` constructs one.** L1's `classify` and L2's
`generate_options` take a `Perceived`. A brain layer that wanted to read the live
world would have to edit `perception.rs` to name it. *A test can be forgotten and
a grep lint can be argued with; a type cannot.*

The one door is `Perceived::cheating`, whose name is its documentation — RL rigs,
replay fixtures, and the brain layers' own unit tests. `FighterBrainProfile::delay`
never calls it, and `no_shipped_level_reacts_instantly` is why.

### The ladder

Nine rows, `reaction_ms` 500 → 150, monotone in reaction, APM, and execution noise.
`FighterBrainLadder::problems()` checks all of that **at startup**, and every one of
those checks would otherwise surface as *"the levels do not order correctly"* after
hours of self-play.

**~~`rollout_depth` is zero on every row.~~ ⛔ STALE, corrected 2026-08-02.**
`ed6c55d0e` (2026-07-31) made `profile.rs` ship
`rollout_depth: if level >= 6 { 12 } else { 0 }`, so FB6 already turned rollouts
on for half the ladder. Two facts in this sentence had rotted: the claim above,
and the citation `ambition_platformer2d_runtime::snapshot::restore` — that module
does not exist, having gone with the custom snapshot engine (ADR 0027). The live
home is `ambition_platformer2d_runtime::rollback`.
⚠ `encounter-orchestration.md` cites a test from the same deleted module and is
NOT stale, because it says so: it records the deletion, names where the invariant
lives now, and keeps the old name explicitly as history. That is the difference
between a preserved citation and a rotted one.
§1 promises graceful degradation — *"L3 is an upgrade, not a dependency"* —
so the whole ladder plays on L2's scores alone today, and FB6 turns them on without
touching a difficulty's identity. `the_whole_shipped_ladder_plays_without_l3` pins
that promise.

### What FB4 still owes

- **The APM cap is DATA, not enforcement.** *"Input-rate histograms within the APM
  cap"* needs a brain that emits inputs, and nothing above L2 does.
- **The ladder self-play rig** (level *n* beats *n−1* in ≥ 60% of headless matches)
  needs the same. It is also the instrument that calibrates L2's weights — and FB2
  (§9) already found the hole that will make it say so: none of §1's four features
  reads a move's power.

---

## 11. FB5 — the reads (opus, 2026-07-10)

`ambition_characters::brain::fighter::habit`. A decayed frequency memory over
`(Situation, Choice)`, and nothing else.

**Bounded by construction.** Both keys are closed enums, so the whole model is a
5 × 6 table. There is no history to prune and no unbounded growth to fear —
`the_model_is_bounded_by_the_product_of_two_closed_enums` observes ten thousand
times and the table is still thirty rows.

**Honest by construction.** The model records only what the view already showed:
what the opponent DID, in a situation the brain could name. A brain that reads you
is not a brain that can see your controller.

**Decay is what makes it a read rather than a census.** An opponent who spot-dodged
nine times and then stopped is not a spot-dodger, and a plain count says otherwise
forever. `observe` decays that situation's rows before crediting the choice, so at
`decay = 0.5` three fresh jumps outweigh nine stale shields. Other situations do
not decay: being edge-guarded rarely does not make what they do there less known.

**Ignorance is not knowledge of absence.** An unseen situation returns the UNIFORM
PRIOR, not zero. A model that returned zero would tell a level-9 brain its opponent
will never shield out of a juggle, on the evidence of never having juggled them.
And `read_bonus` is measured against that prior, so an opponent who does the
expected thing exactly as often as chance is worth no read at all.

`read_weight = 0` on levels 1–3 makes the whole model contribute nothing, which is
§1's *"Level-9 reads = sampling the model; lower levels ignore it"* in one
multiplication.

### One deviation from §5's sketch, on purpose

The sketch says `counts: HashMap<(u16, u16), f32>` with the note *"NOT iterated in
sim order — read-only lookups, determinism-safe."* That is true of the LOOKUP and
false of any iteration, and a trace, a rollback snapshot (brain memory rides the
`SnapshotCursor for Brain` in `snapshot_impls.rs`), and FB6's rollouts all
iterate. It is a `BTreeMap` (ADR 0023), and the keys are the enums themselves
rather than opaque `u16`s, so a trace reads as English.

---

## 12. FB6, redesigned — rollouts on a shadow model (fable, 2026-07-30)

### 12.1 The three faults in FB6-as-written, stated precisely

1. **It named a deleted engine.** `snapshot.take/restore` on a scratch sim was
   netcode N3.1's custom rollback engine; ADR 0027 deleted it. GGRS/bevy_ggrs
   is the sole rollback authority, and it snapshots the LIVE world — there is
   no scratch-world seam to run a rollout on, and building one (a second `App`
   with prepared content, stepped mid-frame inside a brain system) would be a
   parallel sim lifecycle nobody polices.
2. **A wall-clock budget inside a deterministic sim is a desync generator.**
   "2 ms per decision, and when the cap trips use the best fully-evaluated
   option" makes the DECISION a function of machine speed and scheduler
   jitter. Brains run inside the simulation; under GGRS a resimulated decision
   tick must reproduce the original decision bit-for-bit, and a replay on a
   slower machine must not play a different fight. ADR 0023 already forbids
   this class; the old contract wrote it down anyway.
3. **"Seed the scratch world from the delayed view's reconstruction" is
   unimplementable in both directions.** The real sim needs full component
   state the view does not carry, so the reconstruction is underdetermined —
   and anything that closed the gap by copying the live world would carry
   unperceived facts (exact cooldowns, hidden internal state, the opponent's
   real controller) into the brain, violating the no-cheat contract by
   construction.

The common root: FB6 wanted THE sim to be the brain's imagination. The sim is
authoritative, stateful, and omniscient; an imagination must be cheap, pure,
and exactly as ignorant as its owner. Those are different machines.

### 12.2 The three decisions

**D1 — The rollout runs on a SHADOW MODEL, not the sim.** A `ShadowState` is
built from a `Perceived` view and stepped by a pure function at the sim tick
rate. The forward model is the frame data plus the kinematics the view already
exposes — which is exactly what a human who "read the frame data" mentally
simulates. FB4a's type-level enforcement carries over unchanged: the
constructor's only world input is `Perceived` (private field, minted only by
`DelayedPerception::perceive`), so the rollout physically cannot contain a
fact the brain could not see. The fidelity gap against the real sim is OWNED
as a measured property (§12.6), not hidden: the model is the brain's read of
the fight, and a human's read is not the sim either. This decision also
deletes the entire scratch-world problem — nothing persists across a
decision, so there is nothing for GGRS to snapshot and no second world to
keep coherent.

**D2 — The budget is an EXACT step count, and it is data.** Work per decision
is exactly `rollout_k × (1 + rollout_depth)` shadow steps (one predicted-
opponent baseline plus k candidate lines, §12.4) — the profile's two existing
fields ARE the budget. No cap, no early exit, no "best so far": the same
decision costs the same steps on every machine and under every resimulation.
Wall-clock becomes an assertion instead of a knob: a shadow step is O(actors
modeled) = O(2), so the worst shipped case (k = 4, depth = 20) is ~100 cheap
struct updates per decision at 10–20 Hz — a benchmark test pins it (§12.6),
and if a platform ever cannot afford it the fix is an authored profile row,
never a runtime clock.

**D3 — The predicted opponent is deterministic. No RNG in L3, ever.** The
predicted policy is a pure function of the view and the `HabitModel`:

* an opponent mid-move COMPLETES it per its frame data (commitment is real);
* otherwise, if the model holds a genuine read for the current
  `(Situation, _)` bucket — its modal choice strictly exceeds the uniform
  prior — the opponent takes that modal choice, ties broken by `Choice` enum
  order;
* otherwise `Continue`: hold current velocity and phase. Ignorance predicts
  inertia, not behavior.

§1's "level-9 reads = sampling the model" becomes *arg-max, gated by
evidence*, and `read_weight = 0` rows never consult the model at all, so low
levels predict pure inertia. Execution noise (§4) stays in FB4's execution
layer with its own seeded stream; it never enters the rollout.

### 12.3 The shadow model, v1 — a closed list, and its stated omissions

`ambition_characters::brain::fighter::rollout` (new module, pure, no Bevy).

```rust
/// Everything the rollout knows. Built ONLY from `Perceived` + frame data.
pub struct ShadowState {
    pub me: ShadowFighter,
    pub foe: ShadowFighter,
    pub stage: StageView,          // copied from the view
    pub gravity_down: Vec2,        // from SelfView
}
pub struct ShadowFighter {
    pub pos: Vec2, pub vel: Vec2, pub facing: f32, pub half_extent: Vec2,
    pub on_ground: bool,
    pub phase: ShadowPhase,        // Idle | Move { frames: MoveFrameData, t: f32 } | Hitstun { remaining: f32 }
    pub damage: i32, pub health_max: i32,
    pub shield_raised: bool, pub invulnerable: bool,
}
/// In-flight projectiles, ballistic. `PerceivedProjectile` carries
/// `pos`/`vel`/`damage`, which is everything this models.
pub struct ShadowProjectile { pub pos: Vec2, pub vel: Vec2, pub damage: i32 }
pub fn shadow_step(s: &mut ShadowState, dt: f32,
                   my_intent: &ShadowIntent, foe_intent: &ShadowIntent)
                   -> Vec<ShadowEvent>;   // Hit{by,damage,kb} | KO{who}
```

One step, in order: (1) advance both phase clocks (`t += dt`; hitstun counts
down; a move past `total_s` returns to `Idle`); (2) integrate — airborne
bodies accelerate along `gravity_down` by the one authored constant
`SHADOW_GRAVITY` and everyone advances `pos += vel * dt`; grounded bodies
follow their intent's lateral velocity; (3) resolve hits — a fighter whose
move has an open Active span (`active_spans` vs `t`) lands iff the gap along
its facing is ≤ `reach` + the victim's half-extent and the victim is neither
invulnerable nor (shielding while grounded); a landed hit applies
`max_damage`/`max_knockback` (§12.5), puts the victim in
`Hitstun { remaining: hitstun_s(kb, victim.damage) }`, and sets its velocity
to the knockback impulse along the launch direction; (3b) hostile
projectiles advance ballistically, and one overlapping a non-invulnerable,
non-shielding fighter applies its `damage` and is removed — projectiles ARE
v1, because §8's fidelity fixtures include the projectile camper and an
instrument that fails its own v1 scope on day one is a scope bug, not a
finding; (4) test KO — a body in hitstun outside `stage` bounds emits `KO`.

**`hitstun_s` and the knockback response are NOT a free calibration point —
the real formula already exists, and the fork is whether to SHARE it.** The
authoritative response is pure math in
`ambition_platformer2d_actor_monolith::features::ecs::damage_apply`:
`hitstun_timer = feel.{boss|enemy}_hitstun_time ×
knockback_reaction_scale(kb).max(0.35)`, velocity from
`resolved_body_knockback_velocity`, plus the carried-momentum rule. The brain
cannot name any of it: `ambition_platformer2d_actor_monolith` AND `ambition_combat` both depend on
`ambition_characters` (verified in both Cargo.tomls), so the kernel sits
above the crate that wants to speak it — the slice-F shape exactly. Two ways
out, and FB6b must pick one:

1. **Carve the pure hit-response kernel** (the two functions plus the feel
   constants they read) down to `ambition_platformer2d_core`, and have
   `damage_apply` and the shadow model call ONE function. Fidelity on the
   hit-response axis becomes exact by construction instead of calibrated.
   Recommended — the orphan-rule precedent says let the dependency graph
   answer, and it just did.
2. A `ShadowTuning` value passed into `refine_by_rollout` carrying the
   constants, with the three-line formula duplicated. Cheaper today, and a
   drift risk the fidelity instrument would eventually pay for.

**Stated omissions (v1 models NONE of these, on purpose):** FUTURE projectile
fire (in-flight ones are modeled; whether the camper fires again is an
opponent-policy question, D3's, and v1's policy does not spawn), projectile
knockback (damage only), one-way platforms as anything but floor, drop-through,
portals, DI, shield damage/break, move cancels, charge scaling, more than one
hostile. The list is closed so nobody mistakes coverage, and §12.6's fidelity
instrument is what tells us when an omission starts costing decisions.
Extending the model is a new slice with a new fidelity measurement, not a patch.

**2026-07-31 — the instrument spoke, and four omissions came off this list at
once.** `ladder_probe` (`ambition_demo_smash_app`) measured a fighter that lost
every stock to ITSELF at every difficulty, and the depth A/B moved nothing. Each
of these was individually defensible and together they made the model blind to
the commonest death in the game — walk to a ledge, jump, hold a direction:

* **terrain other than the stage box.** The floor a body stood on had a height
  and no EDGE (`ShadowFighter::ground_span`), so a body driven off a platform
  walked on at the same height forever.
* **air control.** `ShadowIntent::Drive` was gated on `on_ground`, so a shadow
  body that jumped went straight up and landed exactly where it took off. This
  is the one that mattered most and was written down nowhere.
* **recovery.** `MovementVerb::Recover` was the one verb `movement_intent`
  refused to judge — and the only verb that can save an airborne body, in the
  only situation where a body has nothing else. It now models an air jump
  against a real budget (`SelfView::air_jumps_left`) with drift after.
* **the dash.** Modelled as `Drive`: a 160 px/s grounded walk against the
  engine's 760 px/s impulse that works mid-air.

Two lessons worth more than the fixes. First, **the KO gate has to distinguish
"offstage and reeling" from "past the point of return"** — requiring `Hitstun`
made every self-inflicted exit free. Second, **a veto that empties the option
list must not fall through to doing nothing**: standing still is a fallback only
where standing still is survivable, which in the air it never is. The first cut
halted instead, the fighter reasoned itself into never moving, and survival went
UP — a paralysis reads exactly like a success on a metric that counts staying
alive.

### 12.4 What L3 does with it

```rust
pub fn refine_by_rollout(
    view: Perceived<'_>, situation: Situation, options: &OptionSet,
    habits: &HabitModel, profile: &FighterBrainProfile,
    tuning: &ShadowTuning, tick_hz: f32, commit_ticks: u32,
) -> Option<RefinedChoice>   // None ⇢ caller uses L2's order unchanged
```

Take L2's top `rollout_k` attacks (already score-sorted, ties already broken
by move id). For each: build `ShadowState` from the view, drive `me` with the
candidate's frame timeline and `foe` with D3's predicted policy, step
`rollout_depth` ticks, and score the terminal state with the SAME
`UtilityWeights` vocabulary — `Δ(foe.damage) − Δ(me.damage)` priced by
`kill_potential`, KO events as the kill bonus, terminal stage position priced
by `stage_risk`. One extra baseline line (both fighters on predicted policy,
no candidate) makes the score a DELTA against doing nothing, so a rollout
cannot credit a move for damage the opponent was going to take anyway.
Re-rank the k candidates by rollout score, ties broken by L2 score then move
id; options below the top k keep L2's order. `rollout_k == 0` or
`rollout_depth == 0` ⇒ return `None` — the existing
`the_whole_shipped_ladder_plays_without_l3` pin keeps meaning what it says.

**L3 also vetoes MOVEMENT, added 2026-07-31, and an empty attack list is not a
reason to skip it.** Each verb L2 offered is rolled as its own line; a line
ending with this body out of the world names the verb in
`RefinedChoice::suicidal_movement`, and the caller takes the best verb NOT in
that list. Two structural facts about it, each of which was a bug first:

* **`OptionSet::attacks` is empty in exactly one situation — `Recovery`** — so
  short-circuiting on it made the veto skip the body with one problem. The
  refined choice is two independent answers: `move_id` (empty when there is
  nothing to swing) and the veto.
* **the verb is sustained for `commit_ticks`, not for the horizon.** A brain
  re-deciding every 5 ticks never walks for 3.2 s; asking what if it did
  condemns every direction from every position. The horizon
  (`rollout_depth × MOVEMENT_HORIZON_MULTIPLE`) is for the CONSEQUENCE — the
  fall, the body leaving the world — which needs seconds. The input does not.

When every verb is vetoed the caller takes the longest-lived
(`least_bad_movement`) rather than standing still, because standing still is
only a fallback where standing still is survivable.

**Rollback obligations: L3 adds NOTHING.** The rollout is a pure function of
`(Perceived, HabitModel, profile)`; no state survives the call. What must be
in the envelope is what FB4's decision cadence already owes when a fighter
`Brain` variant lands: the `DelayedPerception` buffer, the `HabitModel`, the
held intent, and the decision-phase counter all join `SnapshotCursor for
Brain` (`snapshot_impls.rs`) — a resimulated tick that re-decides from
un-rewound memory is the "already applied" derive-memo class of bug: a brain
memory gates behaviour, so it is rollback state, not a cache.

### 12.5 Prerequisite slice — price the payoff (closes §9's recorded gap)

§9 found that no L2 feature reads a move's POWER, could not price it
(`MoveFrameData` carries no damage), and recorded two ways out. FB6 is the
customer that forces route (1): extend the CM7 derivation with

```rust
pub max_damage: i32,      // max over Active volumes' `damage`
pub max_knockback: f32,   // max over Active volumes' `knockback` (flat part)
```

— a pure derivation over `HitVolume { damage, knockback, .. }` exactly like
`reach`, no storage, no new state. The rollout needs it to apply hits
(§12.3); L2 gets its fifth feature `expected_payoff` in the same slice so
both horizons price power through one vocabulary. Adding the FEATURE is
structural; its WEIGHT starts at a v1 value and FB4's ladder calibrates it —
the discipline §9 already pinned.

### 12.6 What §3's harness gains

* **`l3_decides_identically_twice`** — same `Perceived` + model + profile ⇒
  bit-identical `RefinedChoice`, asserted across two calls and (once FB4's
  rig exists) across a GGRS resimulation of the deciding tick.
* **`l3_earns_its_depth`** — the ladder rig plays level N with L3 on against
  the same row with `rollout_k = 0`; L3 ships only if it wins ≥ 60%. "An
  upgrade, not a dependency" becomes a measured claim instead of a promise.
  **First evidence 2026-07-31** — `ladder_probe`, which is NOT the rig (one
  scenario, one opponent, no repeats) but is the first thing in this repository
  to hold a whole profile fixed and move `rollout_depth` alone:

  ```text
    level 9, depth  0   5.2s to first self-KO,  11.4s survived
    level 9, depth 12   2.7s to first self-KO,  23.5s survived
  ```

  Read both columns. The rollout does not stop the brain dying early; it stops
  it dying repeatedly. **Reporting only survival is how a paralysis passed as a
  3× improvement earlier the same day** — a veto with too long a commitment
  window condemned every direction, the fighter stood still, and every number
  said success.

  ⚠ **this measures the VETO, not attack refinement.** Survival on a stage with
  edges is dominated by movement, so a survival win credits the half of L3 that
  says "not that way" and is silent about re-ranking attacks. Authoring a
  nonzero `rollout_depth` on the strength of an ATTACK claim is still blocked;
  authoring one at all is not.
* **The fidelity instrument** — shadow prediction versus the REAL sim.
  As landed (2026-07-30): the shipped versus stage seats its two fighters,
  the attacker's swing is captured from the sim's own `MovePlayback` (so the
  shadow predicts the move the fighter actually throws), and at four authored
  gaps — touching, in reach, out, far out — the shadow model and a real swing
  each answer "does this land?"; floor: agreement on ≥ 3 of 4, the reach-edge
  case being allowed to disagree because two hit tests never share a boundary
  pixel. **GREEN (2026-07-30), and its first red was the instrument doing its
  job**: run 2 disagreed on 2 of 4 gaps, and the diagnosis split three ways —
  a REAL model omission (a move's authored `start_impulse`; the model now
  applies it exactly as `trigger_moveset_moves` does, `MoveFrameData` carries
  it, and `a_lunge_reaches_past_its_static_reach_because_the_body_travels`
  pins it) plus two fixture bugs (the comparison ran while the attacker was
  still sliding out of its walk, so the real swing carried momentum the view
  denied having; and a knockout mid-walk froze the controls while the walker
  burned its budget pressing into the freeze). The comparison now happens at
  TRUE rest and the walker is phase-guarded. ⚠ The original sketch said
  "over §8's eight scenario fixtures" —
  wrong instrument shape: the fixtures are abstract views with no real-sim
  counterpart room, and fidelity is a claim about agreement WITH THE REAL
  SIM, so it must be measured where the real sim plays. This is the
  boring-baseline discipline from `motion_quality`: the instrument that
  tells us when a §12.3 omission starts lying hard enough that argmax is
  noise.
* **The bench pin** — worst shipped `rollout_k × depth` decision costs
  < 100 µs on the CI floor, asserted in a benchmark test, so D2's "wall-clock
  is an assertion, not a knob" is enforced rather than hoped.

### 12.7 Slices

| # | Slice | Grade |
|---|---|---|
| FB6a | ~~`MoveFrameData.max_damage`/`max_knockback` derivation + L2's `expected_payoff` feature (§12.5)~~ ✅ **LANDED 2026-07-30** — `the_smash_outbids_the_jab_on_a_punish_it_fits` pins the recorded scenario both ways | [opus] |
| FB6b | ~~`ShadowState`/`shadow_step` + unit properties~~ ✅ **LANDED 2026-07-30**, route 1 taken: the hit-response kernel (types + `di_adjust` + `knockback_velocity` + `hitstun_duration`) carved to `ambition_platformer2d_core::hit_response`, `damage_apply` and the shadow model call ONE formula, and `the_hit_response_is_the_authoritative_kernel_not_an_imitation` goes red if a private copy reappears. Ballistic projectiles are in v1 | [opus] |
| FB6c | ~~D3's predicted-opponent policy~~ ✅ **LANDED 2026-07-30** — `predicted_foe_intent`: modal habit only when it strictly beats the uniform prior AND `read_weight > 0`; otherwise inertia; no RNG | [opus] |
| FB6d | ~~`refine_by_rollout`~~ ✅ **LANDED 2026-07-30** — re-ranks L2's top k against a do-nothing baseline; `None` at zero k/depth; the marquee test re-ranks a whiffing jab under a connecting lunge | [opus] |
| FB6e | §12.6's instruments. Landed 2026-07-30: `l3_decides_identically_twice`, the bench pin (`the_worst_shipped_budget_is_cheap_enough_to_be_a_non_event`, 100 worst-case decisions < 100 ms), and the fidelity instrument (`the_shadow_model_agrees_with_the_real_sim_about_what_lands` in `app_it` — shadow prediction vs a REAL versus-stage swing at four gaps, frame data captured from the sim's own `MovePlayback`, floor 3 of 4). **Still owed to FB4's rig:** the GGRS-resimulation half of determinism. `l3_earns_its_depth` has its FIRST measurement (2026-07-31, `ladder_probe` — see §12.6), which unblocks authoring a nonzero `rollout_depth` on a survival claim but not on an attack one; the real rig is still owed | [opus] |

FB6a–FB6d are pure-module work implementable today. FB6e's `l3_earns_its_depth`
and the resimulation half of the determinism test require FB4's owed decision
rig (the brain that emits inputs); the fidelity instrument and bench pin do
not. **Stop-at-mismatch facts** each slice was specced against: `HitVolume`
carries `damage`/`knockback` (`ambition_entity_catalog/src/lib.rs`);
`Perceived` is a private-field wrapper minted only by
`DelayedPerception::perceive`; `PerceivedProjectile` carries
`pos`/`vel`/`damage`; the hit-response kernel lives in
`ambition_platformer2d_actor_monolith::features::ecs::damage_apply` and is unreachable from
`ambition_characters` (both `ambition_platformer2d_actor_monolith` and `ambition_combat` depend on
it); `AttackOption` carries `MoveFrameData` and score-sorted, id-tie-broken
order; every shipped ladder row has `rollout_depth = 0` today. If any of
those is no longer true, surface the mismatch instead of adapting silently.

---

## 13. FB4b — the decision rig (fable spec, 2026-07-30; opus executes)

The one thing between FB1–FB6 and a fighter that PLAYS: a brain variant that
emits inputs. Everything below it exists and is pure; the rig is plumbing plus
three genuinely careful pieces (cadence, APM, noise), each of which is
rollback state. Design decisions, made here so execution is mechanical:

**1. It is a `StateMachineCfg` variant, not a new `Brain` arm.**
`StateMachineCfg::Fighter { cfg: FighterCfg, state: FighterState }`, beside
`Smash` and `BossPattern` — ticked from
`tick_state_machine_with_actions`, which already threads
`Option<&WorldView>`, and snapshot into the existing
`SnapshotCursor for Brain` arm set (`snapshot_impls.rs`), which is the
derive-memo rule applied in advance: EVERY field of `FighterState` gates
behaviour, so every field is rollback state, not cache.

- `FighterCfg`: the `FighterBrainProfile` row, a `ShadowTuning`, and
  `decision_interval_ticks` (§5's 10–20 Hz; default 5 at 60 Hz).
- `FighterState`: the `DelayedPerception` buffer; the `HabitModel`; the HELD
  `ActorControlFrame` intent; ticks-until-next-decision; the APM ledger (press
  count + elapsed ticks, two integers — a rate, not a log); the pending
  jittered press (`Option<u32>` ticks-until-press); the noise stream (one
  `u64`, SplitMix64-stepped, advanced ONLY when consumed — `BossPatternState.
  rng_seed` is the precedent and the cursor already rewinds that one).

**2. The kit rides the snapshot.** L2 needs `Vec<AttackCandidate>` and the
brain cannot see the body's moveset (`ambition_combat` depends on
`ambition_characters`, not the reverse — same wall §12.3 hit).
`BrainSnapshot` gains `attack_kit: Vec<AttackCandidate>` filled by the
actors-side snapshot builder from the body's real `ActorMoveset` (the
component `trigger_moveset_moves` reads; `spec.frame_data()` per attack row)
— body-derived truth in the world-in port, exactly like `actor_aerial`. ⚠ Build it ONCE per moveset change if
profiling complains, but correctness first: the builder fills it every tick
like every other snapshot field.

**3. The tick pipeline.** Every tick: `observe()` the fresh view into the
delay buffer (the integration layer already holds the view — it is the
`perception` argument); emit the HELD intent; decrement clocks; if a pending
jittered press matures, stamp it onto the emitted frame and spend an APM
token. On a DECISION tick (`ticks_until_decision == 0`): `perceive()` →
`classify` → `generate_options` → `refine_by_rollout` (profile-gated, §12) →
translate the winner into a new held intent. Movement verbs map to
`locomotion`/jump/shield fields; an attack becomes a PENDING press, delayed
by `round(|noise| × execution_noise × decision_interval)` ticks where
`noise` is the next stream sample in `[-1, 1)` — timing jitter only, no aim
noise in v1 (the moveset aims the melee).

**4. APM is enforced at the ONE emission point.** A press with no token in
the ledger (`presses × 60 × 60 / elapsed_ticks ≥ apm_cap`) is DROPPED and the
held movement stays — the humanity histogram then measures what the brain
DID, not what it wanted. §3's check becomes: run any fixture N ticks, assert
`presses / minutes ≤ apm_cap`, and assert the reaction distribution matches
the delay-buffer depth (both readable from `FighterState`).

**5. Habit observation is part of the decision tick.** `FighterState` keeps
the previous perceived foe sample; at each decision the foe's observable
choice since last time (phase went Startup/Active → `Attack`; left the
ground → `Jump`; `shield_raised` → `Shield`; else velocity sign toward/away
→ `Approach`/`Retreat`/`Wait`) is `observe()`d into the `HabitModel` under
the CURRENT `Situation`. This closes FB5's open loop — the model finally has
a writer that is not a test.

**What this unblocks, in order:** the ladder self-play rig (level N vs N−1
over §8's scenarios — survival % and damage ratio were always waiting on a
brain that emits inputs); the APM/reaction humanity checks (§3); and FB6e's
`l3_earns_its_depth` (same row, `rollout_k` 0 vs 4, ≥60%) — after which, and
only after which, ladder rows author nonzero rollout fields.

**Stop-at-mismatch facts:** `tick_state_machine_with_actions` threads
`Option<&WorldView>`; `SnapshotCursor for Brain` exists in
`ambition_characters/src/snapshot_impls.rs` with `BossPattern` and `Smash`
arms; `BossPatternState` carries a rewound `rng_seed`; `ActorControlFrame`
has `melee_pressed`/`melee_held`/`melee_released`. If any is no longer true,
surface it.
