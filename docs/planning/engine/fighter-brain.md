# The Advanced Fighter Brain — a level-9 CPU that doesn't cheat

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

**FB6 budget contract (pinned 2026-07-06):** rollouts run on a SCRATCH
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
| FB2 | Frame-data table consumer (needs CM7) + L2 option generator/scorer with authored weights | [opus, fable-specced — this doc §1] |
| FB3 | ~~L1 classifier + scenario fixture suite~~ ✅ **DONE 2026-07-10** — see §8 | [opus] |
| FB4 | Difficulty profiles + humanity checks + ladder self-play rig — concretely: (1) the nine `FighterBrainProfile` RON rows per §4 (reaction_ms interpolates 500→150, apm_cap, noise σ, rollout knobs 0 until FB6); (2) the THREE humanity checks from §3 as headless tests — input-rate histogram ≤ apm_cap, reaction-time distribution matches the configured latency (assert the delay buffer is the ONLY view path — a lint-style grep + a runtime assert), and no same-tick perceive→act; (3) the ladder rig = N headless matches per adjacent-level pair via `SlotControls`, gate: level n beats n−1 in ≥ 60%, plus L9-vs-sandbag damage-efficiency floor. The rig reuses the duel-arena headless harness (`ambition_app/tests/duel_arena.rs` is the seed) | [opus] |
| FB5 | Opponent-model memory (bucketed frequencies, decay) | [opus] |
| FB6 | L3 rollouts on the snapshot seam (after netcode N3.1) with compute budget + degradation | **[fable design, opus execute]** |

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

The buffer exists; **nothing yet forces a brain through it.** §3's humanity check
*"assert the delay buffer is on the ONLY read path"* is a lint plus a runtime
assert, and it belongs with the profiles that give `delay_ticks` a value. FB1
built the instrument; FB4 makes it mandatory. Until then `build_world_view`'s
output still reaches the smash brain live, and that is stated here rather than
quietly true.

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
