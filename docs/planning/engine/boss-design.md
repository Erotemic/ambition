# Boss design pipeline — how mid-tier agents author genuinely good fights

**Scope:** how an agent authors a boss fight that is structurally good before
Jon's taste pass. **Mechanism:** [boss system](boss-system.md).
**Playtester brain:** [fighter brain](fighter-brain.md).

The pipeline transfers taste three ways: (1) a finished vocabulary, so a fight
is composed data, not new systems; (2) codified craft rules (the telegraph
grammar and fairness constraints), so bad fights are hard to express; (3)
measured quality, so an agent iterates against numbers and rules. Jon's taste
pass stays the final gate, and every fight Jon tunes becomes a labeled example
in the seed library.

Frame: *every boss is a failed objective function.* Each boss pursues a legible
optimization and over-commits to it; the player wins by exploiting the
over-commitment. §3's rules encode this.

## 1. Vocabulary

A boss is an actor + `BossConfig` + `BossBehaviorProfile` (patterns, phases,
movement, `limb_routing`, `possessed_verbs`) + `BossEncounterSpec` + optional
mount pair and limb actors + moveset `MoveSpec`s + specials as procedural
modules. Profiles and encounters are content-pack data.

**Pattern control flow (BD1, landed)** in
`ambition_boss_encounter::pattern::control_flow`:

- `BossPatternStep::Select { table: Vec<WeightedArm> }`, gated on the closed
  `SituationBucket` (`PlayerNear`, `PlayerFar`, `PlayerAbove`, `PlayerBehind`,
  `HpBelow(f32)`).
- `BossPatternStep::Stance { id }` with `BossPattern::stances` (a `BTreeMap`).
- `BossPattern::interrupts: Vec<InterruptRule>`: `OnHitTaken { min_damage }`,
  `OnPhaseEnter { phase }`, `OnTimer { every_s }`, each with `cooldown_s`, each
  entering a stance.

Runtime rules:

- A `Select` rolls when the timeline is resolved (phase change, stance enter or
  leave, cursor loop), never at the cursor. The resolved timeline contains no
  `Select`.
- Ineligible arms leave the denominator. A `Select` consumes exactly one draw
  whether an arm wins or not, so RNG streams stay in lockstep.
- `OnHitTaken` reads the brain's remembered HP (`BossPatternState::last_hp`).
- An `OnTimer` accumulator resets when its condition holds, not when the
  interrupt may fire, so a timer behind a long cooldown does not bank firings.
- An interrupt resumes the beat it stole, with its elapsed time.
- An unknown stance id is a no-op (the validator reports it). A self-referencing
  `Select` stops at a depth limit.

No shipped boss uses `Select`, `Stance` or interrupts yet.

**Open slices:**

- **BD2 — arena beats as data:** hazard waves, summons and terrain changes
  authored from the encounter spec through the existing geometry overlay and
  encounter-script bus.
- **BD3 — telegraph presentation:** `TelegraphSpec { pose, cue, vfx }` on
  `BossPatternStep::Telegraph` is landed and projected into `BossAttackState`.
  The presentation emitter and its sfx/vfx consumer are owed; they land with the
  first fight that needs them.

## 2. The seed library

`ambition_boss_encounter::pattern::seeds` (`MoveSeed`, `SeedLibrary`) and the
catalog `game/ambition_content/assets/data/boss_seeds.ron`. A seed is a
parameterized attack archetype with a design intent, the skill it tests, its
fair counters, a threat tier, measured telegraph and active bands, its
instances and recipes. **A fight is 4 to 7 seeds + phase escalation + one
signature move.** Every fight that survives Jon's pass contributes its signature
move back as a seed.

Shipped archetypes: `sweep`, `slam`, `body_nova`, `zone_denial`,
`projectile_rain`, `spread_volley`, `beam`, `dash_through`, `summon`.
`counter_stance`, `enrage_repeat` and `grab_command` have no instance and are
not in the file; add each with the fight that first needs it.

`telegraph` and `active` are the exact observed envelope of every occurrence in
`boss_profiles.ron`. `boss_seeds_bands_are_the_measured_envelope`
(`game/ambition_content/tests/boss_seeds.rs`) fails when an occurrence leaves its
band and when a band is wider than its instances. Retuning a boss therefore
updates `boss_seeds.ron`.

No seed carries `recovery`: the punish window is the `Rest` that follows a
`Strike`, a property of the occurrence. The shipped roster never demands a
`Parry` (`the_shipped_roster_does_not_yet_demand_a_parry` deletes itself when a
fight fixes that).

## 3. The telegraph grammar & fairness rules (validated, not advised)

A diagnostic fight validator (`ambition_boss_encounter::pattern::validator`)
checks the authored data. Per-game bands live in one RON file,
`game/ambition_content/assets/data/boss_validator_bands.ron`.

1. **Telegraph proportionality:** telegraph duration scales with threat
   (damage x area). Attacks without a telegraph event fail.
2. **Answer coverage:** each attack's `fair_counters` is non-empty, and across
   the fight every core movement verb (jump, dash, walk-out, shield or parry
   where the game has it) appears in some counter set.
3. **Commitment:** every attack has a punish window (recovery at or above the
   floor) or is tagged `pressure`. No unpunishable heavies.
4. **Simultaneity budget:** at most N concurrent active threat volumes per
   phase.
5. **Readability floor:** two distinct attacks never share a `(pose, cue)`
   telegraph identity.

**Calibration v0** (data, per game): the sim steps at 60 Hz. Telegraph bands:
light (at most 8 damage, single volume) at least 12 ticks; medium at least 20;
heavy (one-shot or arena-wide) at least 30. Recovery floors: heavy at least 24
ticks (`MoveSpec::frame_data().recovery_s`), medium at least 12. `pressure`
attacks are exempt but capped at 10% victim HP per touch. No attack's active
volumes cover more than 60% of the arena's walkable width in one tick (default
N = 3). Missing telegraph, empty `fair_counters`, unpunishable heavy and an
exceeded budget are errors; band deviations up to 20% are warnings that need a
`// boss-tuning:` justification; above 20% is an error.

**The unit of judgement is a beat.** The validator walks each phase into
`Beat { move_key, phase, telegraph_s, active_s, recovery_s }`, including
`Select` arms and stance bodies. A phase timeline loops, so a leading `Rest`
counts after a trailing strike.

**Current state:**

- Rules 1, 2, 3 and 5 are implemented. Rule 1 fires nowhere on the roster.
- Rule 4 is blocked: persistent threats (for example `zone_denial` hazards from
  a special) keep their lifetime in module code, not in authored or
  runtime-visible data. A seed-level `persists_s` is one candidate.
- The roster's findings are the constants `EXPECTED_ERRORS` and
  `EXPECTED_WARNINGS` in `game/ambition_content/tests/boss_fight_validator.rs`.
  The errors are rule 3 findings in Enrage, where the authored sequence chains a
  strike into the next telegraph. Whether that is unfair or intended escalation
  is a per-fight taste call (BD7 with Jon).
- GNU-ton is the only boss that authors telegraph identities. For every other
  boss, a missing telegraph is a warning.

**Enforcement is deferred by maintainer decision.** The validator is diagnostic.
It does not gate installation and does not block other work. The engine cannot
yet express everything that makes a fight feel good (recovery, pressure,
overlapping threats, readable telegraphs, intentional exceptions). An install
gate is a separate maintainer decision, taken when bosses are authored for feel
or near shipment.

## 4. Measured quality (the playtester loop)

Headless, deterministic, agent-runnable:

- **Rig:** the fighter brain at several difficulty rows drives the player
  against the boss over N seeded runs, plus a sandbag and a random-input floor.
- **Metrics:** hits-taken distribution, time-to-kill band, threat density per
  phase (escalates with breathing valleys), the winner's verb usage,
  punish conversion and damage-source diversity.
- **Loop:** author from seeds -> validator -> rig metrics in band -> blind
  commit with the metric report in the message -> Jon's taste pass -> feedback
  becomes seed annotations or band changes. Never tune against your own sense
  of fun.
- **Report format:**

  ```ron
  FightReport(
      boss_id: "...", build: "<git sha>", runs: 32, seed0: 1234,
      per_difficulty: { 3: RunBand(...), 6: RunBand(...), 9: RunBand(...) },
  )
  RunBand(
      win_rate: f32,
      time_to_kill_s: (min, med, max),
      hits_taken: (min, med, max),
      threat_density: [f32; N_PHASES],
      verb_usage: { "jump": u32, "dash": u32, ... },
      punish_conversion: f32,
      damage_sources: { "<attack id>": f32 },
  )
  ```

  In-band assertions: win rate rises with difficulty; median hits taken is
  inside the band; no damage source above 0.5; verb usage covers every core
  verb; threat density does not decrease across phases and has a valley.

**First cut (BD6): `fight_discovery`.** It runs a boss in its real arena
headlessly and writes a choreography, a §4-shaped summary per run and named
findings:

```text
cargo run -p ambition_app_tools --release --bin fight_discovery -- \
    --room gnu_ton_arena [--policy all|sandbag|random|aggressor] [--seconds 180] [--seeds 2]
```

Beats come from `BossAttackState`; threat comes from `CombatObservation` rows
for the boss, what it rides and their limbs. Policies are floors (sandbag,
seeded random, scripted aggressor). Finding thresholds are guesses until a
rated fight calibrates them. The next rung is the fighter brain in the player
seat, which `win_rate` needs.

## 5. The ceiling

The pipeline gets structural quality: readable, fair, escalating fights that
exercise the kit. Signature-move invention, humor and dramatic pacing stay
with Jon or a stronger model. Hollow Lite's boss
([`../demos/hollow-lite.md`](../demos/hollow-lite.md)) is the acceptance test: an
agent-authored fight that Jon rates as fun.

## 6. Slices

| # | Slice | State |
|---|---|---|
| BD1 | Pattern control-flow atoms | done |
| BD2 | Arena beats from the encounter spec | open |
| BD3 | Telegraph identity | data and validator done; presentation consumer open |
| BD4 | Seed library | done |
| BD5 | Fight validator | diagnostic; rule 4 blocked; enforcement deferred |
| BD6 | Playtester rig and report | `fight_discovery` first cut; fighter-brain rung needs F1-F4 of [fighter brain](fighter-brain.md) (ladder authority, representative rosters, mid-ladder progression) |
| BD7 | Pilot: re-author one boss (mockingbird or behemoth) through the loop; calibrate bands with Jon | open |
| BD8 | Hollow Lite boss through the pipeline | open |
