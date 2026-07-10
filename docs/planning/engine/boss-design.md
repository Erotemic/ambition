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
| BD1 | ~~Pattern control-flow atoms~~ ✅ **DONE 2026-07-10** — see §8 | [opus] |
| BD2 | Arena beats from encounter spec (waves/spawns/terrain via existing buses) | [opus] |
| BD3 | ~~Telegraph event channel (rides CM5)~~ 🟡 **DATA + VALIDATOR half DONE 2026-07-10** — see §10 | [opus] |
| BD4 | ~~Seed library v1~~ ✅ **DONE 2026-07-10** — see §7 | [opus] |
| BD5 | 🟡 **VALIDATOR LANDED; ENFORCEMENT PENDING (audit correction 2026-07-10)** — see §9 | [opus] |
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
3. ~~**Every shipped telegraph clears even the heavy floor** … rule 1 will fire on
   exactly one boss when BD5 lands.~~ **WRONG, and BD5 disproved it (§9).** Rule 1
   fires nowhere. The mockingbird's 26-tick cycle telegraphs belong to `sweep` and
   `dash_through`, both `Medium`, whose floor is 20 ticks — not a heavy's 30. I
   compared a medium's telegraph against a heavy's floor. The `dash_through`
   recipe's advice (0.60 s for a grounded re-author) still stands as *taste*; it
   was never a rule-1 violation.

### What BD5 gets for free

`SeedLibrary::seed_for_move(key)` and `counter_coverage(keys)` — rule 2's
per-fight verb union — are already there, ordered and tested. `MoveSeed::threat`
is rule 1's tier. What BD5 still owes: the per-game calibration RON, the
occurrence-level recovery measurement (finding 1), and the simultaneity integral.

---

## 8. BD1 — the three atoms, landed (opus, 2026-07-10)

`ambition_characters::brain::boss_pattern::{control_flow, …}`. **Byte-parity**:
`stances` and `interrupts` are `#[serde(default)]`, so every row
`boss_profiles.ron` already carries parses unchanged, and BD4's measured seed
bands are bit-identical after the change.

### What was built

- **`BossPatternStep::Select { table: Vec<WeightedArm> }`** — a weighted table
  gated on a CLOSED `SituationBucket` (`PlayerNear`, `PlayerFar`, `PlayerAbove`,
  `PlayerBehind`, `HpBelow(f32)`), each computed from the boss's existing context.
- **`BossPatternStep::Stance { id }`** + `BossPattern::stances` — a named
  sub-sequence, entered as a jump with a saved return point.
- **`BossPattern::interrupts: Vec<InterruptRule>`** — `OnHitTaken { min_damage }`,
  `OnPhaseEnter { phase }`, `OnTimer { every_s }`, each with a `cooldown_s`, each
  entering a stance.

### Four rulings the sketch left open

1. **A `Select` rolls at RESOLUTION, not at the cursor.** The timeline the ticker
   walks (`BossPatternState::timeline`) is resolved on phase change, on stance
   enter/leave, and each time the cursor loops — so for a looping script "roll
   once when reached" and "roll once per pass" are the same statement. Two
   reasons: the cursor advances by step DURATION, and a zero-duration step at the
   cursor is one authoring mistake away from an unbounded advance loop; and BD5's
   validator wants to integrate a pass's total threat, which it cannot do against
   a step meaning *"and then, maybe, some other steps."* The resolved timeline
   contains no `Select` at all, and a test says so.

2. **Ineligible arms leave the DENOMINATOR, not just the draw.** A table that is
   half far-range arms would otherwise silently under-weight its near-range ones
   the moment the player closed in — the bug a "roll then filter" order has. But a
   `Select` still consumes exactly one draw whether or not an arm wins, so two
   bosses that diverge in position stay in lockstep on the RNG stream itself.

3. **`OnHitTaken` needed no damage channel.** The brain remembers its own HP
   (`BossPatternState::last_hp`); a drop since last tick IS a hit, and a heal is
   not one. Inventing a per-tick damage message would have been a second
   representation of a fact the boss already carries.

4. **`stances` is a `BTreeMap`, not the sketch's `HashMap`.** The ticker only
   `get`s by id, but a validator and a trace both WALK it, and ADR 0023 bans
   std-hash iteration anywhere the sim can observe the order.

### Two traps, each with a test named after it

- **A timer behind a long cooldown must not bank its firings.** The `OnTimer`
  accumulator resets when the trigger CONDITION holds, not when the interrupt is
  *allowed* to fire. Otherwise a 1s timer behind a 5s cooldown fires five times in
  a row at t=5.
- **An interrupt resumes the beat it stole, elapsed and all.** A boss yanked out
  of a telegraph comes back to that telegraph rather than restarting it, so the
  punish window the player was already reading stays where it was.

An unknown stance id is a no-op, not a panic: BD5 rejects it at install time, and
a fight already running must not die of a typo. A self-referencing `Select` bottoms
out at a depth limit rather than hanging the sim.

### What BD1 did NOT do

**No shipped boss uses an atom yet.** BD1 is vocabulary + runtime; re-authoring a
fight through it is BD7's pilot, which is where the numbers get a taste pass. The
atoms are tested through the real ticker (7 integration tests) and as pure
functions (21 unit tests), so BD7 starts from machinery that works rather than
machinery that compiles.

BD4's seed-library oracle now walks `Select` arms and stance bodies, so the
catalog cannot go partial the moment a fight uses one.

---

## 9. BD5 — validator landed; enforcement pending (opus + audit correction, 2026-07-10)

`ambition_characters::brain::boss_pattern::validator`, with the per-game bands in
`game/ambition_content/assets/data/boss_validator_bands.ron` (§3: *"the bands live
in ONE RON file per game so re-calibration is data, not code"*).

### THE MEASUREMENT: the shipped roster vs §3

**8 errors, 1 warning.** Every error is rule 3, every one is in **Enrage**, and
every one is the same shape:

```
  clockwork_warden              minima_trap       Enrage: punish window 0 ticks (floor 12)
  clockwork_warden              saddle_point      Enrage: punish window 0 ticks (floor 12)
  exploding_gradient_boss       overfit_volley    Enrage: punish window 0 ticks (floor 6)
  exploding_gradient_boss       saddle_point      Enrage: punish window 0 ticks (floor 12)
  flying_spaghetti_monster_boss overfit_volley    Enrage: punish window 0 ticks (floor 6)
  gnu_ton_rider                 hand_slam         Enrage: punish window 0 ticks (floor 24)
  overflow_boss                 full_body_pulse   Enrage: punish window 0 ticks (floor 12)
  trex_boss                     full_body_pulse   Enrage: punish window 0 ticks (floor 12)
  smirking_behemoth_boss  [warn] the fight never demands WalkOut
```

The tightened enrage combos chain a `Strike` straight into the next `Telegraph`.
§3 calls that an unpunishable attack; the authors called it escalation. **Which of
them is right is BD7's pilot to settle with Jon** — and making that argument
legible, per attack, per phase, is the entire point of the pipeline.

The warning is its own small design fact: the smirking behemoth's kit is a beam, a
sweep, a slam and a nova, every one answered by jumping or dashing. A player never
has to simply step out of the way.

**Rule 1 fires nowhere**, which disproves §7's finding 3. I had compared a
`Medium` attack's telegraph against a `Heavy`'s floor.

### The unit of judgement is a BEAT

BD4 found that no per-attack `recovery` exists and none can — the punish window is
the `Rest` that FOLLOWS a strike, a property of the OCCURRENCE. So the validator
walks each phase into `Beat { move_key, phase, telegraph_s, active_s, recovery_s }`.
`floor_slam` is fair in phase 1 and unpunishable in enrage, and only a per-beat
rule can say that.

**A phase's timeline loops**, so a strike that ends the list is followed by
whatever begins it. Crediting a leading `Rest` removed three findings that were
facts about the walker rather than about the fight (the smirking behemoth's
`eye_beam`, in all three phases). A validator that cannot be wrong about a fight
cannot be trusted about one.

`Select` arms are walked — a fight cannot hide an unpunishable heavy inside a
table — and so are stance bodies.

### Two of §3's five rules are NOT implemented, and the reason is in the code

- **Rule 4, simultaneity budget.** A scripted timeline is sequential, so its
  body-mounted volumes never overlap. The threats that DO overlap are the
  `zone_denial` hazards a `Special` spawns, whose lifetime lives in the content
  technique's private consts (`MINIMA_TRAP_HAZARD_DURATION_S = 5.0`), not in any
  authored row. This rule needs a `persists_s` on the seed, fed by the technique.
- ~~**Rule 5, readability floor.**~~ ✅ **LANDED with BD3 (§10).** Two distinct
  attacks may not share a `(pose, cue)` telegraph identity. **Nine of nine shipped
  bosses author no telegraph at all**, so rule 5's other half — §3's *"attacks
  without a telegraph event FAIL"* — is a warning today and an error after BD7.

Both are named in the module docs rather than approximated by a rule that checks
something adjacent and reports green.

### Why it is not an install-time gate yet

§3's endgame is *"fight does not install."* That contract is not implemented:
`install_boss_roster` does not call the validator, and the current test deliberately
accepts eight hard errors through `EXPECTED_ERRORS`. The validator therefore
**measures debt; it does not enforce the design doctrine.** Calibration v0 still
needs BD7's pilot before zero becomes the correct threshold, but until installation
rejects hard errors the honest status is “validator landed, enforcement pending.”

BD7 must recalibrate the bands against Jon's verdict, reduce accepted hard errors
to zero (or replace them with explicit per-fight reviewed waivers), and wire the
validator into roster installation. Poison-test that a hard-invalid fight cannot
install before restoring DONE.

---

## 10. BD3 — the telegraph gets an identity (opus, 2026-07-10)

`TelegraphSpec { pose, cue, vfx }`, `#[serde(default)]` on
`BossPatternStep::Telegraph`, projected into `BossAttackState::telegraph_spec` so
presentation reads ONE read-model instead of re-walking the script. Every pre-BD3
row parses unchanged.

**A duration is not an identity.** The wind-up's length says how long the player
has; the pose and the cue say what they are looking at. §3 rule 5 — *"distinct
attacks must differ in telegraph (pose row OR cue)"* — is a statement about this
type and cannot be made about a number: two attacks that both wind up for 1.2 s
are not thereby distinguishable, and a fight in which everything looks the same is
unreadable however generous its timings. That is why BD5 could not implement rule
5 until now, and why it can now.

### THE MEASUREMENT

**Nine of nine shipped bosses author NO telegraph identity.** Every attack in the
game telegraphs by duration alone. §3's *"attacks without a telegraph event FAIL"*
is therefore a WARNING today (one per fight, listing the attacks) and an ERROR
after BD7's pilot. It is the single largest readability gap the pipeline has found,
and it was invisible before there was a field to be empty.

The validator's error half is live: two distinct attacks sharing a `(pose, cue)`
identity is an ERROR, and an all-`None` spec reads as ABSENT rather than as an
identity every attack could collide on.

### What BD3 did NOT do

**No boss authors a telegraph, and none was invented.** Writing `pose: "rear_up"`
for a row that has no such animation, or a `cue` id no sfx registry carries, would
have made the warning go away without making a single fight more readable — the
"generic by accident" failure the boss-sprite bug taught. Authoring them is BD7's
pilot, where the numbers get a taste pass.

**The presentation consumer is likewise owed.** `BossAttackState.telegraph_spec`
is the read-model a CM5-style emitter would fire from on the telegraph's rising
edge; the emitter and its sfx/vfx consumer land with the first boss that authors
one. The doc's two purposes for BD3 were *"anticipation is AUTHORED per attack"*
and *"the validator can SEE it"*. The second is done; the first has a place to go.
