# Fighter tactical intent — an experiment (draft v0.2, 2026-10-30)

**Question.** Does one persistent tactical intent — a held spacing objective
plus an armed punish — make the Smash Siblings CPU better to fight than the
reactive utility policy alone, on identical fixtures? This is an experiment
with a gate (§6), not a rewrite. If the gate fails, the reactive policy stays
and this document records why.

Revised from the v0.1 proposal (GPT) after review against the repository;
§2 lists what changed and why.

## 1. What the reactive brain already has (reuse, do not rebuild)

| Capability | Where |
|---|---|
| Situation (Neutral / Advantage / EdgeGuard / Disadvantage / Recovery) | `ambition_characters/src/brain/fighter/situation.rs` |
| Spacing relative to the body's OWN reach | `options.rs`, movement scoring |
| Defence: shield or spot dodge against a swing whose `attack_reach` covers the gap | `options.rs::incoming_attack` |
| Punish read: foe in attack startup/recovery or landing, `phase` + `phase_remaining` perceived | `situation.rs::is_punishable`, `options.rs::foe_commitment` |
| Own-move outcome: `own_attack.landed`, `refused` | `data.rs::FighterState` |
| Recovery authority (lens, helpless rule) | `ambition_combat/src/brain/fighter/recovery.rs`, `situation.rs` |
| Delayed foe / live self perception (Q167), noise, APM | `perception.rs`, `decision.rs` |
| Deterministic per-seed matches and traces | `smash_cpu_report match-report`, `AMBITION_MATCH_REPORT_TRACE_TICKS` / `_TRACE_MOVES` |

**Not** active adaptation: `HabitModel` is filled but read only by the
rollout path (`rollout.rs`), which every shipped rung disables
(`rollout_depth: 0`). Opponent modelling is therefore not an existing
capability.

## 2. Changes from v0.1

1. **One intent, two kinds.** Not six plan kinds with their own executor
   (a second decision system beside the scorer). One `Intent` record in
   `FighterState` — target, kind, objective, expiry, exit reason — with two
   kinds: `Space` and `Punish`. `Recover` and `Defend` stay reactive and
   override any intent. Approach / Pressure wait for the gate.
2. **The intent CONSTRAINS, the scorer still chooses.** `Space` supplies a
   range band (outside the foe's threat, inside my answer) and forbids
   offensive commitments that leave the band, except the armed punish. 
   `Punish` arms one move against a perceived opening and fires only when
   the opening is still feasible from now (§3). Legality, noise and
   emission stay the existing scorer's and emitter's.
3. **Windows are measured from now.** Done (below): `foe_commitment` now
   subtracts the observation's age, as `counter_value` already did.
4. **Feasibility first, at rung 9.** The census (§3) shows rung 5 can
   punish only slow moves; the prototype is established at rung 9 and rung
   5's behaviour is a separate difficulty question.
5. **Whiff conversion before baiting.** First: hold the band and convert a
   scripted whiff. Only then: an opponent that attacks conditionally on
   range and sometimes refuses — a predictable dummy missing is not bait.

## 3. Feasibility (MEASURED, `scripts/smash_punish_windows.py`)

A whiff at the attacker's maximum reach, answered by a defender standing
just outside it: reaction delay + one decision interval + travel at run
speed + the defender's fastest grounded `hit_s`, against the attacker's
`total − last active` (facet timing). All 9 polygon pairs, 117 cases:

| rung (reaction) | 1 (500 ms) | 5 (300 ms) | 7 (220 ms) | 9 (150 ms) |
|---|---|---|---|---|
| punishable | 3% | 39% | 58% | 82% |

At rung 5 the smashes and slow aerials leave windows; jabs and tilts do not
(safe on whiff, as in the genre). REASONED limits of the census: reach from
authored RON without facet scaling, run speed from rest, aerial landing lag
not added — it decides feasibility, not policy.

## 4. Groundwork done before the intent (2026-10-30)

- **Target consistency.** Movement scoring asked the ledge question toward
  the FIRST hostile in the actor list; it now uses `nearest_hostile()`, the
  target every other layer uses.
- **Stale commitment.** `foe_commitment` returned `phase_remaining` as seen
  300 ms ago at rung 5; it now returns what is left now (witness
  `a_commitment_is_measured_from_now_not_from_the_observation`).

## 5. Missing pieces the prototype needs

1. **The foe's threat range in neutral.** `incoming_attack` reads the reach
   of the swing in progress; a `Space` band needs the reach of the foe's kit
   before it swings (its authored coverage — what a player knows of the
   character, so fair). Today that is the constant `assumed_foe_reach`.
2. **A scripted attacker fixture.** Done: `whiff_drill` (§5a). A
   range-conditional attacker (for baiting) is the next fixture, after the
   gate.
3. **Intent state in rollback.** `Brain` is a rollback cursor; the intent's
   fields join `FighterState`'s snapshot (`snapshot_impls.rs`, "every field
   gates the next decision"). Policy in `ambition_characters`, emission in
   `ambition_combat::brain::fighter::decision`.
4. **Fidelity of armed moves from a run** (standing vs running bindings) —
   only if the armed punish fires out of a dash.

## 5a. Fixture A exists: `whiff_drill` (baseline MEASURED)

`cargo run -p ambition_app_tools --bin whiff_drill -- --whiffer W --verb V
--cpu C --level N --seconds S`: seat 0 throws V uncharged on a cadence
through ordinary control frames and turns to face the CPU; seat 1 is the
CPU at rung N. Swings are classed hit / stuffed / punished / unanswered by
what the engine resolved.

Baseline, reactive policy, pugnacious forward smash (reach 54 px; census
margin +14 f at rung 9, +5 f at rung 5) against pointed, 60 s:

| rung | swings | hit (CPU walked in) | whiffs | punished | unanswered: median gap |
|---|---|---|---|---|---|
| 9 | 28 | 5 | 20 | 5 (25%) | 182 px |
| 5 | 33 | 4 | 27 | 6 (22%) | 143 px |

The two failures are the intent's two kinds: the CPU stands ~3× the reach
away (no `Space` band), and converts a quarter of windows that exist (no
armed `Punish`).

## 5b. First gate run (2026-10-31): NOT PASSED

Built (66537d545): `intent.rs` (Space, Punish) behind `tactics`, perception's
`threat_reach`, `whiff_drill --tactics`, `AMBITION_INTENT_TRACE`. The trace
found a perception defect that blinded every punish read, reactive and
intent alike: a swing in recovery reported 0 s remaining (fixed, 8a2031c8f).

Fixture A after that fix, one 60 s run per arm (MEASURED):

| rung | policy | punished whiffs | CPU hit by the swing | CPU damage taken |
|---|---|---|---|---|
| 9 | reactive | 2 / 14 (14%) | 5 | 56 |
| 9 | tactics | 2 / 33 (6%) | 1 | 15 |
| 5 | reactive | 7 / 29 (24%) | 3 | 52 |
| 5 | tactics | 2 / 30 (6%) | 3 | 61 |

`Space` works as defence at rung 9 (hit 5 → 1, damage 56 → 15). `Punish`
converts fewer windows than the reactive ranking. Not yet traced: whether
it arms on the window, closes at the run speed it assumed, and is then let
down by admission (the ranking's lead) or by the decision cadence and the
press jitter. That trace is the next step if this work resumes; the flag
stays off on every shipped rung until a run passes the gate. Jon paused the
work here (2026-10-31) after a playtest: "The CPU and smash feel is MUCH
better than it was yesterday."

## 6. The gate

Identical seeds, fighters, stage and rung, `tactics` off vs on (a per-rung
ladder flag), at rung 9 first:

- **Fixture A (scripted whiffer).** Pass: the CPU holds the band without
  being hit and lands a hit during the whiffer's recovery in at least N of
  M seeds, where the census says the window exists.
- **Fixture B (passive foe).** No stalling: the intent must not make the
  CPU hold a band against a body that never comes.
- **Matches.** The four polygon cells, seeds 0–2.

Measured: punish conversions (hits landed during the foe's attack
recovery), intent completions and interruption reasons, steering flips
(oscillation), engagement against a passive foe, self-KO census, damage,
and whiffs split into "meant to hit" vs spacing pokes. Then Jon plays both.

Pass the gate → aerial approach, advantage follow-ups, multiplayer target
switching, difficulty calibration. Fail → record why here and stop.

## 7. Deferred

Pressure / Approach / Defend as intent kinds; aerial and platform
engagement; target switching beyond the nearest-hostile fix; opponent
modelling (HabitModel into the live path); enabling rollout; difficulty
calibration below rung 9; campaign (Gnuton) integration.
