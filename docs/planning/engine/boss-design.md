# Boss design pipeline — how mid-tier agents author genuinely good fights

**Authored by fable, 2026-07-05.** The current boss fights work mechanically
and aren't much fun. The gap is TASTE, and taste doesn't transfer to
mid-tier agents by exhortation. This pipeline transfers it three ways:
(1) a **finished vocabulary** so authoring a fight is composing data, not
writing systems; (2) **codified craft rules** (the telegraph grammar and
fairness constraints) that make bad fights hard to express; (3) **measured
quality** — headless metrics scored against the
[fighter brain](fighter-brain.md) as an automated playtester, so an opus
agent iterates against numbers and rules instead of vibes. Jon's manual
taste pass remains the final gate (BLIND-commit discipline), but it should
start from a fight that is already structurally good — and every fight Jon
tunes becomes a labeled example in the seed library for the next agent.

Ambition's frame: *every boss is a failed objective function* — each boss
has a LEGIBLE optimization it pursues and over-commits to; the player wins
by exploiting the over-commitment. This is not flavor; it is the design
device that makes fights READABLE, and §3's rules encode it.

---

## 1. Vocabulary completion (what still blocks pure-data fights)

Post-G-track, a boss is: an actor + `BossConfig` + `BossBehaviorProfile`
(RON: patterns, phases, movement, `limb_routing`, `possessed_verbs`) +
`BossEncounterSpec` (RON numbers) + optional mount pair + limb actors +
moveset `MoveSpec`s. Remaining vocabulary slices:

- **BD1 — pattern control-flow as data:** today's `BossPattern` sequencing
  covers timed/scripted beats; add the three authored-logic atoms fights
  keep wanting: *conditional selection* (weight table per situation bucket:
  player near/far/above/behind, boss HP band), *interrupts* (on-hit,
  on-phase, on-timer), and *stances* (named sub-pattern the interrupt
  vocabulary can enter/leave). No scripting language — three enum arms.
  **Pinned data sketch (fable — extend the EXISTING
  `BossPatternStep`/`BossPattern` in `ambition_characters::brain::
  boss_pattern`, do not mint a parallel format):**
  ```rust
  // Two new BossPatternStep arms (serde, alongside Telegraph/Strike/Rest):
  Select { table: Vec<WeightedArm> },   // roll once when reached
  Stance { id: String },                // jump to the named stance's steps
  pub struct WeightedArm {
      pub weight: f32,
      pub when: Option<SituationBucket>, // None = always eligible
      pub steps: Vec<BossPatternStep>,   // inline sub-sequence
  }
  // The bucket is a CLOSED enum computed from the boss's existing view
  // (distance band, relative vertical, behind/ahead, own HP band):
  pub enum SituationBucket { PlayerNear, PlayerFar, PlayerAbove,
      PlayerBehind, HpBelow(f32) }
  // BossPattern grows named stances + interrupts (both serde-default
  // empty = every existing RON row parses unchanged, byte-parity):
  pub struct BossPattern {
      pub steps: Vec<BossPatternStep>,
      pub stances: HashMap<String, Vec<BossPatternStep>>,  // entered via Stance{id}
      pub interrupts: Vec<InterruptRule>,
  }
  pub struct InterruptRule {
      pub on: InterruptTrigger,          // OnHitTaken{min_damage}, OnPhaseEnter{n},
                                         // OnTimer{every_s}
      pub cooldown_s: f32,
      pub enter: String,                 // stance id to jump to
  }
  ```
  Runtime: the pattern ticker (which already walks `steps` by duration)
  gains a small cursor stack (current stance + return point); `Select`
  rolls with the boss's seeded RNG stream (determinism, netcode N0
  guardrails). Weighted-roll + interrupt-cooldown + bucket-eval are pure
  fns, unit-tested without Bevy.
- **BD2 — arena beats as data:** hazard waves, add/summon spawns, terrain
  changes (the RoomGeometry overlay + encounter script bus both exist)
  authorable from the encounter spec, so set-piece phases don't need Rust.
- **BD3 — telegraph channel:** a `telegraph` presentation event on
  pattern/move rows (pose row, flash, sfx cue — combat-model CM5's event
  channel) so anticipation is AUTHORED per attack, and the validator (§3)
  can SEE it.

## 2. The seed library (attack & fight archetypes as prefabs)

A content-side catalog of parameterized building blocks, each a
`MoveSpec`/pattern prefab with named params and a written *design intent*:

`sweep` (horizontal denial), `slam` (vertical punish, big recovery),
`projectile_rain` (positioning test), `dash_through` (cross-up),
`zone_denial` (persistent area), `summon` (attention split),
`counter_stance` (bait/punish), `enrage_repeat` (phase escalation),
`grab_command` (shield answer). Each entry documents: the player skill it
tests, its fair-counter set (which movement verbs answer it), typical
startup/recovery bands, and 2–3 param recipes. **A fight = 4–7 seeds +
phase escalation + one signature bespoke move.** The library starts from
the existing bosses' moves (extract → generalize → document), and grows
by accretion: every fight that survives Jon's taste pass contributes its
bespoke move back as a documented seed.

## 3. The telegraph grammar & fairness rules (validated, not advised)

Codified as an install-time/CI **fight validator** over the authored data
(same pattern as the content-graph validator):

1. **Telegraph proportionality:** every attack's telegraph duration scales
   with its threat (damage × area): heavies ≥ 30 ticks, lights ≥ 12
   (numbers are data, per-game). Attacks without a telegraph event FAIL.
2. **Answer coverage:** for each attack, the authored `fair_counters`
   (from its seed) must be non-empty, and across the fight every core
   movement verb (jump, dash, walk-out, shield/parry where the game has
   it) must appear in some attack's counter set — fights must exercise the
   kit (*"forced-movement variety"*).
3. **Commitment rule (the failed-objective-function made mechanical):**
   every attack has a punish window (recovery ≥ data-floor) OR is
   explicitly tagged `pressure` (small, chip-level threat). No unpunishable
   heavies.
4. **Simultaneity budget:** ≤ N concurrent active threat volumes
   (per-phase data); rain-type seeds declare density so the validator can
   integrate total screen threat.
5. **Readability floor:** distinct attacks must differ in telegraph
   (pose row OR cue) — no two attacks share an identical telegraph.

**Calibration v0 (pinned 2026-07-06 — starting numbers, all per-game RON
data; BD7's pilot re-calibrates them against Jon's verdict):** the sim
steps at 60 Hz, so ticks below ≈ frames. *Telegraph bands:* light
(≤ 8 dmg, single volume) ≥ 12 ticks; medium ≥ 20; heavy (one-shot-threat
or arena-wide) ≥ 30. *Recovery/punish floors:* heavies ≥ 24 ticks of
recovery (CM7 `frame_data().recovery_s` is the measured value), mediums
≥ 12; `pressure`-tagged attacks exempt but capped at ≤ 10% victim HP per
touch. *Arena assumptions the validator may rely on:* the encounter room
declares its arena AABB + platform set; no single attack's active volumes
may cover > 60% of the arena's walkable width in any tick (integrates
with the simultaneity budget, N=3 concurrent threats default). *Worked
example (acceptable):* `sweep(reach=180px, startup=22t, active=10t,
recovery=26t, telegraph=pose+cue at t−22)` vs (rejected): the same sweep
with `startup=8t` (heavy threat, light telegraph → ERROR). *Hard error
vs warning:* missing telegraph event, empty `fair_counters`, unpunishable
heavy, simultaneity budget exceeded = ERRORS (fight does not install);
band deviations ≤ 20% = WARNINGS requiring an inline `// boss-tuning:`
justification the validator prints; > 20% = ERROR. The bands live in ONE
RON file per game so re-calibration is data, not code.

## 4. Measured quality (the playtester loop)

Headless, deterministic, agent-runnable:

- **The rig:** fighter brain (several difficulty rows) drives the PLAYER
  against the candidate boss N seeded runs; also a no-input sandbag run
  and a random-input run as floors.
- **Metrics:** hit-taken distribution (a fight that never hits L3 or
  always hits L7 is mis-tuned), time-to-kill band, threat-density curve
  over the fight (should ESCALATE by phase, with breathing valleys),
  verb-usage histogram of the winning brain (did the fight force
  movement variety?), punish-conversion rate (are the §3.3 windows real?),
  and damage-source diversity (no single attack > X% of all damage dealt).
- **The loop an opus agent runs:** author from seeds → validator green →
  rig metrics in band → BLIND commit with the metric report in the commit
  message → Jon's taste pass → feedback becomes new seed annotations or
  band adjustments. The agent NEVER ships on vibes and never tunes
  against its own judgment of fun — only against the validator, the
  bands, and Jon's recorded feedback.
- **The report format (pinned — BD6 emits exactly this, one RON per run
  batch, committed next to the fight's RON + summarized in the commit
  message):**
  ```ron
  FightReport(
      boss_id: "...", build: "<git sha>", runs: 32, seed0: 1234,
      per_difficulty: { 3: RunBand(...), 6: RunBand(...), 9: RunBand(...) },
  )
  // RunBand per difficulty:
  RunBand(
      win_rate: f32,                  // brain beats boss
      time_to_kill_s: (min, med, max),
      hits_taken: (min, med, max),    // band target lives in the same RON
                                      // file as §3's validator bands
      threat_density: [f32; N_PHASES],// mean active-volume area/arena area
      verb_usage: { "jump": u32, "dash": u32, ... },  // winning runs only
      punish_conversion: f32,         // punishes landed / punish windows seen
      damage_sources: { "<attack id>": f32 },  // fraction of all damage
  )
  ```
  In-band assertions (BD6 ships them as a test the pilot calibrates):
  win_rate rises with difficulty; hits_taken median inside the authored
  band; no `damage_sources` entry > 0.5; `verb_usage` covers every core
  movement verb; `threat_density` non-decreasing across phases with at
  least one valley (the breathing rule).

## 5. Honesty about the ceiling

This pipeline gets structural quality — readable, fair, escalating,
kit-exercising fights — from mid-tier agents reliably. The last 10%
(signature-move invention, humor, dramatic pacing) stays human/frontier:
Jon's pass, or a future model. The pipeline's job is to make that pass
START from "structurally excellent" and to bank every taste correction as
data. Hollow Lite's boss ([`../demos/hollow-lite.md`](../demos/hollow-lite.md))
is the acceptance test: an opus-authored fight through this pipeline that
Jon rates as *actually fun*.

## 6. Slices

| # | Slice | Grade |
|---|---|---|
| BD1 | Pattern control-flow atoms (weighted selection buckets, interrupts, stances) | [opus, fable-specced — §1] |
| BD2 | Arena beats from encounter spec (waves/spawns/terrain via existing buses) | [opus] |
| BD3 | Telegraph event channel (rides CM5) | [opus] |
| BD4 | ~~Seed library v1~~ ✅ **DONE 2026-07-10** — see §7 | [opus] |
| BD5 | Fight validator (the §3 rules over authored data) | [opus] |
| BD6 | Playtester rig + metrics + report format | [opus; needs FB1–FB4] |
| BD7 | Pilot: re-author ONE existing boss (mockingbird or behemoth) through the full loop; calibrate bands against Jon's verdict | [opus + Jon] |
| BD8 | Hollow Lite boss through the pipeline (the acceptance) | [opus + Jon] |

---

## 7. BD4 — the seed library, extracted (opus, 2026-07-10)

**Nine seeds, twenty-two attacks, zero uncatalogued moves.**

- **Vocabulary:** `ambition_characters::brain::boss_pattern::seeds` —
  `MoveSeed` (archetype, intent, `skill_tested`, `fair_counters`, `threat`,
  measured `telegraph`/`active` bands, `instances`, `recipes`) and `SeedLibrary`
  (a `BTreeMap`, so a validator's error list never depends on hash seed).
- **Catalog:** `game/ambition_content/assets/data/boss_seeds.ron`. Content, not
  engine — the engine names no boss and no archetype instance.
- **Oracle:** `game/ambition_content/tests/boss_seeds.rs`, five tests.

### The seeds

| Seed | Threat | Instances | Answered by |
|---|---|---|---|
| `sweep` | Medium | side_sweep, hand_sweep, wing_sweep, broadside, converging_shockwave | Jump, Blink |
| `slam` | Heavy | floor_slam, hand_slam, head_descent, seismic_stomp | WalkOut, Dash |
| `body_nova` | Medium | full_body_pulse, gradient_nova | WalkOut, Dash, Blink |
| `zone_denial` | Medium | hazard_column, minima_trap, saddle_point | Jump, WalkOut, Dash |
| `projectile_rain` | Light | apple_rain, overfit_volley, overflow_flood | Jump, Dash, WalkOut, Blink |
| `spread_volley` | Light | mode_collapse_converge, echo_fan | Dash, WalkOut, Blink, Shield |
| `beam` | Light | eye_beam | Jump, Descend, Dash |
| `dash_through` | Medium | dive_lane | Jump, Dash, WalkOut |
| `summon` | Pressure | gradient_cascade | Dash, WalkOut, Jump |

`body_nova` and `spread_volley` are **new archetypes**, not in §2's list. They
fell out of the data: a proximity burst and an instantaneous ring/cone each have
their own answer-shape, and neither is a sweep, a slam, or a rain.

§2's `counter_stance`, `enrage_repeat`, and `grab_command` have **no instance in
the roster** and are therefore not in the file. An archetype with no example
teaches an authoring agent nothing. They arrive with the fight that first needs
one; `counter_stance` will be built on BD1's `Stance` arm.

### The bands are a measurement, and the test keeps them one

`telegraph` and `active` are the **exact observed envelope** of every occurrence
in `boss_profiles.ron` — no padding. `boss_seeds_bands_are_the_measured_envelope`
re-derives them from the same bytes the game loads and fails on **both** sides:
an occurrence outside a band, *and* a band wider than its own instances. So
retuning a boss updates `boss_seeds.ron`. That is the accretion discipline §2 asks
for, made mechanical rather than requested.

Walking the roster meant handling two shapes. `Scripted` bosses carry per-step
`Telegraph`/`Strike` durations across five phases; `Cycle` bosses carry none and
rotate their `attacks` on the profile's flat `attack_windup`/`attack_active` —
which is why all four of the mockingbird's moves read 0.44 s / 0.28 s. A band
that ignored the `Cycle` bosses would have been a lie about half the roster.

### Three findings the extraction produced

1. **No seed carries `recovery`, and none can.** §3's commitment rule wants a
   punish window per attack. `BossPatternStep` has none: the punish window is the
   `Rest` beat that *follows* a `Strike`, a property of the **occurrence**, not of
   the move. BD5 must measure it per beat. Inventing a `recovery` field nothing
   could fill would have been worse than naming the gap — and the clockwork
   warden's enrage phase already chains `minima_trap → overfit_volley` with **no
   Rest between them**, which is precisely the beat BD5 will have to judge.
2. **The shipped roster never demands a `Parry`,** and demands `Shield` only
   through `spread_volley`. That is §3 rule 2's "forced-movement variety" gap,
   measured. `the_shipped_roster_does_not_yet_demand_a_parry` pins it, and says in
   its own assertion message to delete itself when a fight fixes it.
3. **Every shipped telegraph clears even the heavy floor.** The shortest is
   0.44 s = 26 ticks (the mockingbird's cycle), against §3's `heavy ≥ 30 ticks`.
   So rule 1 will fire on exactly one boss when BD5 lands, and it will be right to:
   `dive_lane` and `broadside` are not light attacks. The `dash_through` recipe
   says so and recommends 0.60 s for a grounded re-author.

### What BD5 gets for free

`SeedLibrary::seed_for_move(key)` and `counter_coverage(keys)` — rule 2's
per-fight verb union — are already there, ordered and tested. `MoveSeed::threat`
is rule 1's tier. What BD5 still owes: the per-game calibration RON, the
occurrence-level recovery measurement (finding 1), and the simultaneity integral.
