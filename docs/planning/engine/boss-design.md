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
| BD4 | Seed library v1: extract + document existing boss moves as prefab seeds | [opus/sonnet — extraction is mechanical, intent-writing is opus] |
| BD5 | Fight validator (the §3 rules over authored data) | [opus] |
| BD6 | Playtester rig + metrics + report format | [opus; needs FB1–FB4] |
| BD7 | Pilot: re-author ONE existing boss (mockingbird or behemoth) through the full loop; calibrate bands against Jon's verdict | [opus + Jon] |
| BD8 | Hollow Lite boss through the pipeline (the acceptance) | [opus + Jon] |
