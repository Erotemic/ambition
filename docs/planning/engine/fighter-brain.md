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
| FB1 | View audit for the no-cheat contract: does `WorldView`/`BrainSnapshot` carry move-phase, damage meters, stage geometry? Add missing fields; add the perception delay-buffer wrapper | [opus] |
| FB2 | Frame-data table consumer (needs CM7) + L2 option generator/scorer with authored weights | [opus, fable-specced — this doc §1] |
| FB3 | L1 classifier + scenario fixture suite | [opus] |
| FB4 | Difficulty profiles + humanity checks + ladder self-play rig — concretely: (1) the nine `FighterBrainProfile` RON rows per §4 (reaction_ms interpolates 500→150, apm_cap, noise σ, rollout knobs 0 until FB6); (2) the THREE humanity checks from §3 as headless tests — input-rate histogram ≤ apm_cap, reaction-time distribution matches the configured latency (assert the delay buffer is the ONLY view path — a lint-style grep + a runtime assert), and no same-tick perceive→act; (3) the ladder rig = N headless matches per adjacent-level pair via `SlotControls`, gate: level n beats n−1 in ≥ 60%, plus L9-vs-sandbag damage-efficiency floor. The rig reuses the duel-arena headless harness (`ambition_app/tests/duel_arena.rs` is the seed) | [opus] |
| FB5 | Opponent-model memory (bucketed frequencies, decay) | [opus] |
| FB6 | L3 rollouts on the snapshot seam (after netcode N3.1) with compute budget + degradation | **[fable design, opus execute]** |

Sequencing: FB1–FB4 need only landed systems + CM7 and deliver a credible
mid-level CPU; FB5 makes it scary; FB6 makes it level 9. SSB demo ships
with FB1–FB4 minimum.
