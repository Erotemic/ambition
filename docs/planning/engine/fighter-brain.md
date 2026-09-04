# Advanced fighter brain — current evaluation and regression work

**State:** implementation exists. The 2026-08-31 level-6 rollout regression is
CLOSED — and ⛔ **a SECOND, distinct level-6 rollout defect is OPEN, measured
2026-09-04**: a fighter running the rollout selects `Dodge` and `Shield` **zero
times in 662 decisions**, because `pick_movement`'s unjudged tier is unreachable.
The defensive vocabulary switches off at the rung the difficulty ladder calls
better, and that single rung is the whole of the ladder's apparent inversion.
Half repaired (`Shield` is modelled); `Dodge` needs real motion in the shadow.

⛔⛔ **AND NOTHING HERE HAS MEASURED THE LADDER AMBITION SHIPS.** The demo app the
rig runs installs no `AuthoredFighterLadder`, so every rung carries the engine
floor's `UtilityWeights::default()` — which *is* the level-9 row. Every number on
this page describes the floor's reflex-only ladder. The ownership question is with
Jon in [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md);
**do not tune the brain against these numbers until it is answered.**

⇒ Current work is evaluation, the open level-6 rollout defect above, and the
separate level-1 case.

The historical construction campaign is not active planning. The production
fighter brain already has situation classification, option scoring, delayed
perception, authored difficulty profiles, opponent-memory inputs, scenario
fixtures, rollout/refinement, normal actor-control output and recovery probing.

## Current measured facts

### Difficulty/evaluation rig exists

The scenario/evaluation tools exercise the production brain seam rather than a
separate toy policy. Fixed seeds are deterministic, the scenario set is covered,
and action-rate/profile constraints are measurable.

Earlier ladder measurements also established that headline `apm_cap` values are
not themselves the main rung separator: observed press rates remained well below
the caps while reaction/decision cadence still separated profiles.

Do not infer "higher rung wins more" from one-body scenario metrics. Duel
outcomes, self-KO behavior, damage and recovery are different measurements.

### Level-6 rollout regression — DIAGNOSED AND FIXED 2026-08-31

The A/B that established it, 2026-08-29:

```text
                         l1      l3      l5      l6      l9
rollout ON (shipped)    45/45     0       0     45/45     0
rollout OFF             45/45     0       0      0/45     0
```

**After the fix, rollout ON:** `l1 45/45, l3 0, l5 0, l6 0/45, l9 0` — l6 now
fights (84%/54% peak damage where it was 0%/0%). Recorded in
`dev/ambition_dev_measurements/ladder_recovery_sweep.jsonl`.

⭐ **THE ROLLOUT READ ITS OWN SILENCE AS SAFETY.** `movement_intent` returns
`None` for verbs the shadow cannot simulate (Dodge, Blink), and its header says
outright that a rollout reporting every unknown as safe *"would be lying in one
direction"*. It reported neither — the option was dropped from the rolled set —
so it never appeared in `suicidal_movement`, and the consumer's `find` over
"not vetoed" promoted it above every verb the rollout DID judge. Worse, it
suppressed `least_bad_movement` entirely: that fallback was gated on every
OFFERED verb being vetoed, and an unjudged verb never is.

The trace line, two ticks before l6 died at 0%:

```text
offered=[Approach, Dodge, Jump] vetoed=[Approach, Jump]
  unmodelled=[Dodge] chose=Some(Dodge) least_bad=Some(Approach)
```

`pick_movement` ranks three tiers now — judged-and-unvetoed, then the least-bad
line, then unmodelled — with the third tier also carrying the untouched
no-rollout path.

⚠ **THE TRACE HAD TO GROW FIRST.** `least_bad` and `unmodelled` never left
`refine_by_rollout`, so *"every option was fatal and this one dies latest"* and
*"a verb nobody rolled outranked them"* rendered identically. Both are published
now, on the `AMBITION_FIGHTER_TRACE=1` line and on the `fighter_decision` causal
fact. F1 asked for a decision trace rather than another sweep; the trace was one
field short of being able to answer, and that is worth remembering the next time
an instrument reports a decision without its reason.

Run: `smash_tool ladder-rig --sweep-below [--no-rollout] --seeds 45`.
⚠ Note the seed count: this is the arm the 45-seed claim above was measured
on. The `Reproduce with` block further down runs **1** seed, which shows the
trace format but does NOT reproduce the 45-seed result it sits under.

`RecoveryLens` did **not** change which bouts are fought, and the traced Recovery
decisions were byte-identical between the two arms for their first 22 ticks — the
divergence was never on the recovery road at all. It was in Neutral/Advantage,
where the veto emptied the modelled options and an unjudged one took over.

### Level 1 — CLOSED, and it was the same defect as the platform floor

⛔ **REACTION TIME WAS NEVER THE REASON.** `--reaction-ms` at 500, 300, 150 and
**0** gave byte-identical outcomes; the trace changed at 0 (the per-decision
position jumps that are perception staleness disappeared), so the override landed
and the outcome did not depend on it.

⭐ **THE CAUSE was the respawn platform.** It was rebuilt every tick at the
protected fighter's own position while calling itself stationary, and it reaches
the collision world as a `BlinkWall` — which the three floor filters excluded. So
a fighter standing on it perceived no floor at all, and every ledge question read
through the false one.

⛔⛔ **NEITHER HALF COULD LAND ALONE**, which is the lesson worth keeping. Making
the block standable while the platform still followed the body gave the rollout a
floor defined as *"wherever I am"* — a constant 48px to the edge however far the
body walked — so every verb was judged to walk off and vetoed, every tick, and
level 6 regressed to its exact pre-fix numbers. Measured twice, reverted twice,
before the platform half was found.

⭐ **TOGETHER, at 45 seeds: `unfought` is gone from every rung, and level 1 goes
45/45 → 0/45.** The perceived edge changes as the body walks (44 → 21 → −9 past
the lip → 226 on reaching the stage). Recorded in `dev/ambition_dev_measurements`.

## F1 — DONE. The trace named it; here is what to reuse

The experiment was a single controlled trace at the rollout authority boundary,
one failing seed, rollout on and off, and it worked — see the diagnosis above.
Three things from it are worth carrying to the next one:

- **run the trace, not another sweep, and let it name the tick.** The Recovery
  decisions were byte-identical between arms; the divergence was in
  Neutral/Advantage. A sweep would have said "l6 fails" for another week.
- **the instrument was one field short.** `least_bad_movement` and the unjudged
  set never left `refine_by_rollout`, so the two candidate explanations rendered
  the same. Adding them was cheaper than reasoning without them, and they are
  published now.
- **check what the consumer does with a `None`.** `movement_intent`'s own header
  says an unknown must not read as safe OR as fatal; the consumer read it as
  safe. A refusal to answer is not an answer, and the caller has to be told
  which it got.

Reproduce with:

```text
AMBITION_FIGHTER_TRACE=1 cargo run --release -p ambition_demo_smash_app \
  --bin smash_tool -- ladder-rig --sweep-below --seeds 1 2>&1 | grep '^\[fighter '
```

✔ **RE-RUN 2026-09-03 AND IT STILL REPRODUCES — release build, 1 seed, 6,388
`[fighter …]` lines.** The instrument is intact: `least_bad`, `unmodelled` and
`floor_edge` are present on **every one** of those lines, so the fields F1 added
have not been lost to a refactor.

⭐ **AND THEY ARE LOAD-BEARING, not decoration.** The three tiers of
`pick_movement` are all exercised in a single one-seed sweep:

| field | non-default | of 6,388 |
|---|---|---|
| `least_bad=Some(..)` | **280** — `Recover` 108, `Retreat` 86, `Approach` 86 | 4.4% |
| `unmodelled=[..]` | **983** — `Dodge` 846, `Shield`+ 137 | 15.4% |

⚠ **THIS CENSUS PREDATES THE SHIELD MODEL (2026-09-04) and cannot be re-derived
from HEAD.** `Shield` was unmodelled when it was taken and is modelled now, so a
fresh sweep would move those 137 out of the unmodelled column. The row is left as
measured rather than silently adjusted — it is what the instrument said on the day
— but do not read it as the current split. ⇒ The `Dodge` 846 is the part that
still stands, and it is the part that matters: it is the remaining half of the
repair.

⇒ Tier 2 fires 280 times and tier 3 fires 983 times in ONE seed. The page's
lesson was that the instrument had been one field short; the measurement says
the added fields are populated often enough that a reader who ignores them is
ignoring a sixth of the decisions.

Do **not** tune rollout depth, RecoveryLens heuristics, APM, reaction time, or
movement weights without a trace that identifies the responsible decision first.

The remaining case is level 1, which rollout never explained: queue row
`D-FIGHTER-L1`.

## F2 — keep the evaluation rig representative

A scenario must instantiate the premise it claims to measure. A static opponent
cannot measure reaction delay; an unarmed fighter cannot measure combat choice;
a recovery case must include the real body capabilities relevant to recovery.

Keep deterministic fixed-seed reports and enough instrumentation to show that:

- the brain acts through perceived/delayed facts;
- actions route through normal actor control;
- profile APM/reaction/noise limits are actually enforced;
- CPU cost remains within the intended budget.

⛔ **AND FOUR PROVENANCE FACTS THIS LIST WAS MISSING, added 2026-09-04 after every
one of them turned out to be invisible in a shipped report.** A table that does
not carry these is not comparable with another table, which is the only thing a
rig is for:

- **which LADDER the fighters got** — the rig ran its whole life on the engine
  floor and no output said so;
- **which STAGE** — the layout was a constant nobody had chosen, and it turns out
  to halve the lethality;
- **which DESIGN** — paired or unpaired changes 14 of 36 verdicts, and two runs
  that do not say which cannot be compared;
- **which WEIGHTS** — the scenario table printed none at all, and the ladder
  table's line said *"v1 (profile default)"* while v1 was overriding the profile.

⇒ **The rule under all four: a number leaves this rig with its method or it is
not a measurement.** Each was found the same way — by trying to compare two runs
and discovering they had answered different questions.

Do not build a second permanent telemetry stack around the fighter brain.

⭐ **MEASURED 2026-09-03: the rig reproduced FIVE of its NINE named scenarios
when this section was written, and it says so itself. It is NINE of nine now —
the four reclaims are recorded below, in the order they landed.** `smash_tool ladder-rig --scenarios --seeds 1` opens with
*"PLACEMENT ONLY — 5 of 9 fixture(s) are reproduced by placing two bodies"* and
then skips four by name, each with the setup it cannot perform:

| skipped fixture | what the rig cannot set up |
|---|---|
| `juggle_escape` | velocity, body phase |
| `projectile_camper` | projectiles |
| `edgeguard_window` | velocity |
| `edgeguard_ledge_hang` | ledge hang |

⭐ **THE RIG IS HONEST AND THAT IS THE POINT** — it refuses rather than placing
two bodies and calling the result a juggle. This is F2's own rule enforced by
the instrument: *"a scenario must instantiate the premise it claims to
measure."* ⚠ But the consequence belongs on this page in a number: **the four it
cannot reach are the ones a platform fighter is judged on** — edgeguarding
(twice), juggling, and projectile camping. Reading a green ladder as "the brain
is evaluated" over-reads it by four ninths.

⇒ **The gap is a SETUP capability, not more seeds.** Every skip names velocity,
body phase, projectiles or a ledge hang — states a placement cannot express.

✔ **FIXED THE VELOCITY HALF THE SAME DAY: 5 of 9 → 6 of 9.** `edgeguard_window`
now runs and produces four real rungs, and the higher-skill fighter wins the top
two — which is the verdict an edgeguard fixture exists to produce. The change is
small because the authority already accepted it: `transit_body` takes a
`TransitVelocity`, and the rig was passing `Zero` unconditionally. `Scenario`
gained `starting_velocities()` beside `starting_positions()`, and `place_at`
passes `Set(..)` when a scenario asks for motion. ⛔ Still the transit authority,
not a field write — ADR 0024, and `engine.pose-writes-are-authority-only`
already caught the bare version of that once.

⚠ **The instrument stayed honest and that is the check that it worked:**
`juggle_escape`'s skip line narrowed from *"cannot set up: velocity, body
phase"* to *"cannot set up: body phase"*. It did not start passing because the
rig stopped asking; it names the one thing that is still missing.

✔ **AND THE BODY-PHASE HALF, THE SAME DAY: 6 of 9 → 7 of 9.** `juggle_escape`
now runs. ⭐ **`BodyPhase` is DERIVED, not stored** — the runtime's `body_phase()`
(`features/ecs/perception.rs:250`) computes it from `BodyCombat.hitstun_timer` /
`recoil_lock_timer`, `BodyMelee`'s attack phase and the shield. So a fixture that
starts a body *"in hitstun"* is reproduced by writing the TIMER the phase is
computed from. Assigning the enum would be writing the thermometer.

⛔ **`starting_hitstun()` returns `None` unless every non-`Neutral` phase in the
fixture is `Hitstun`.** The attack phases need a `BodyMelee` mid-swing, which a
timer cannot fake, so a startup/active fixture is still reported unreproduced
rather than staged as something the fixture did not describe. The rig's skip
filter asks the accessor rather than string-matching the phase name.

✔ **AND THE LEDGE HANG: 7 of 9 → 8 of 9.** `edgeguard_ledge_hang` runs. It is
the fixture the premise calls *"the most punishable state in the genre"*, and the
ladder now has a verdict for it at every rung.

⛔ **The rule "a hang is not a position" survives — it is why this took real
geometry rather than a coordinate.** The anchor comes from the actual platform,
`smash_stage().world.blocks[0]`, and the ledge is its top corner on the side the
fixture put the body; `wall_normal_x` is `-1` at the left edge because the wall
is then on the player's RIGHT. Guessing any of that would stage a body hanging in
mid-air — a fixture staging something its premise did not describe, which is the
failure the skip existed to prevent.
⚠ **Order matters and the authority says why:** `transit_body` CLEARS
`ledge_grab` (*"the ledge anchor was a fact of the departure point"*), so the
hang is declared after the transit, never before.

✔ **AND THE THREE RECLAIMS PERTURBED NOTHING ELSE — checked rather than
assumed.** The four fixtures that already ran (`ledge_trap`, `recovery_left`,
`recovery_right`, `recovery_below`) produce **byte-identical rows** before and
after all three changes, compared by hashing their lines from the 5-of-9 run
against the 8-of-9 one. ⇒ The new setup only fires where a fixture asks for it:
`starting_velocities` returns `None` for a still fixture, `starting_hitstun`
`Some((0.0, 0.0))` writes nothing, and `starting_ledge_hangs` skips a body that
does not hang. A reclaim that also moved the existing numbers would have been a
regression wearing a coverage win's clothes.

✔ **AND THE LAST ONE: 8 of 9 → 9 of 9.** `projectile_camper` runs. The rig fires
the volley ability's OWN authored bolt — `abilities::ranged::volley::authored_bolt`,
exported for exactly this — from the foe toward the subject, mapping the
fixture's projectile OFFSET the way `starting_positions_on` maps its positions.

⛔ **The fixture's `damage: 3` is not reproduced, deliberately, and that is the
point.** Its premise is *"an opponent at range with a shot in the air"*; the
damage value describes its own 800x600 stage the way its coordinates do.
Building a `ProjectileSpawn` out of the fixture's numbers would stage a
projectile **no ability authors** — a shot that exists nowhere in the game. The
rig fires a real one instead, on the real spawn road
(`ProjectileSpawnRequest::open` → `ProjectileStart::StepThisTick`).

⭐ **ALL NINE, AND THE OTHER EIGHT DID NOT MOVE.** Every fixture that ran before
each change produces byte-identical rows after it — checked at 5→8 and again at
8→9, by hashing their lines. Setup fires only where a fixture asks: no velocity,
no hitstun seconds, no hang, no shots means no writes. A coverage win that also
moved the existing numbers would be a regression in a win's clothing, and a
rising fixture count would never have shown it.

### The brain CAN see a drop-through platform, end to end (checked 2026-09-04)

⚠ **Recorded because the opposite is the obvious guess and I made it twice
before checking.** When the platformed stage landed, my first assumption was that
the CPU could not perceive the tiers, on the grounds that `StageView` carries a
single field — `bounds`, the room's AABB — and the smash brain's
`TerrainAwareness` carries only `off_stage` and `nearest_ledge_distance`. Both
true, and both irrelevant: the FIGHTER brain reads `WorldView`, which has
`terrain: Vec<PerceivedSolid>`.

The chain resolves, every hop:

| hop | where |
|---|---|
| authored `BlockKind::OneWay` | `smash_platform_stage()` |
| survives perception's filter | `perceived_solid_kind` maps `OneWay` → `SolidKind::OneWay` |
| reaches the view | `WorldView.terrain`, filtered to the viewport |
| is lowered back into a simulable block | `ambition_combat/src/brain/fighter/recovery.rs:130` — `SolidKind::OneWay => ae::Block::one_way("perceived", …)` |

⇒ So a recovery rollout can plan *through* a drop-through tier rather than
treating it as absent or as a wall. That is what makes a flat-versus-platforms
comparison a measurement of DECISIONS and not only of physics accidents.

⚠ It does not follow that the brain uses them WELL — that is what the comparison
below is for — only that a difference, if one appears, has a road to travel down.

### ⛔⛔ EVERY LADDER NUMBER IN THIS DOCUMENT MEASURED A FLATTENED LADDER (found 2026-09-04)

**`ladder_rig` overwrote the authored per-level utility weights on every fighter,
on every run, and said it was doing nothing.**

`weights_from_args()` returned `UtilityWeights::v1()` when no `--weight` was
passed, and `force_utility_weights` assigned it to **every** live fighter brain.
`v1()` is not a neutral default: it is *exactly the LEVEL 9 row* of
`game/ambition_content/assets/data/fighter_brain_ladder.ron` — `frame_advantage`
0.6, `kill_potential` 0.4, `stage_risk` -0.8, `expected_payoff` 0.5.

⇒ **So a "6 versus 5" bout was two fighters with LEVEL 9 PRIORITIES wearing level
6 and level 5 reflexes.** The authored utility ladder — the half that says how
much a rung cares about a kill and how far it will chase one past the ledge — was
erased before the first tick. What actually differed between rungs was only
`reaction_ms`, `apm_cap`, `execution_noise` and `read_weight`; `rollout_depth`
and `rollout_k` are 0 on every row, so they never differed either.

⚠ **And the log line said `weights: v1 (profile default)`**, which is wrong twice.
`v1` is not the profile's default — the profile authors weights PER LEVEL — and
"default" reads as *nothing was changed* at the exact moment something was. The
scenario table did not print even that: `--scenarios` returns before the ladder
mode's announcement, so every scenario table ever quoted into this document
travelled without the scoring configuration that produced it.

⇒ **Fixed**: an override is now `Option`, applied only when `--weight` was
actually passed, and both tables name what they ran under — either
*"each rung's OWN authored row"* or the overridden values. `--weight` still
forces on every fighter, which is the documented reason it exists.

⛔⛔ **AND THE FORCING WAS THE OUTER OF TWO LAYERS — the re-run PROVED IT by
changing nothing.** I expected the authored-weights run to move the numbers. It
did not: **all 36 of 36** fixture×rung cells came back byte-identical in every
column — the complete matrix, not a sample. That is the evidence, not a disappointment — if the authored rungs
had been reaching the fighters, removing an override that overwrote them would
have had to change something.

⇒ **The real mechanism, confirmed twice.** `FighterBrainProfile::for_level` — the
engine FLOOR — sets `utility_weights: UtilityWeights::default()`, and `default()`
*is* `v1()`. So the floor already gives **every** rung level-9 weights. The
authored per-level rows reach a fighter only through
`project_authored_fighter_ladder`, which needs `Res<AuthoredFighterLadder>`, and
that resource is inserted by **`ambition_content`** — which neither
`ambition_demo_smash` nor `ambition_demo_smash_app` depends on (`grep ambition_content
game/ambition_demo_smash*/Cargo.toml` → 0).

⇒ **So the rig has never once had the authored ladder.** Its rungs differ in
`reaction_ms`, `apm_cap`, `execution_noise` and `read_weight` — the floor's own
per-level values — and in nothing else. The rig's forcing was a redundant second
flattening on top of a floor that had already flattened it.

⚠ **The fix is therefore NOT the one I made.** Making the override optional is
correct and stays, but it is not sufficient and on its own it is inert. The rig's
app has to install the authored ladder before any run can measure it, and until
it does, *"the ladder inverts at 5→6"* is a statement about the FLOOR's reflex
ladder and not about the difficulty ladder Ambition ships.

⇒ **What this costs.** Every ladder result this project has recorded measures the
engine floor's reflex-only ladder. They are not wrong about what they measured;
none of them is a measurement of the shipped ladder, and no brain change should
be justified by them until the rig loads it.

### The scoreboard change, and what the matrix said under it

⭐ **MEASURED 2026-09-04, same 15 seeds and 36 fixture×rung cells, with the
verdict changed from survival-until-a-cap to OUTCOME (stocks taken, then
cumulative damage dealt).** Still forced-v1 weights — this run predates the fix
above.

| | survival verdict (old) | outcome verdict (new) |
|---|---|---|
| `both survive` — no information | **18** | **0** |
| favours LOWER | 17 | 24 |
| favours higher | 1 | 12 |

⇒ **The saturation is gone**: 18 cells that could not answer now answer. And the
lopsidedness mostly is too — 1 higher-favouring cell became 12 — which is
evidence the OLD metric was biased toward the passive fighter rather than
reporting a real skill inversion.

⛔ **But a residue survives, and it localises.** Every individual cell is still
`(within spread)`, so no row can claim anything. Aggregating across fixtures —
the correct blocked unit, since the four rungs of one fixture are not independent
— **6 of the 6 decided fixtures favour the LOWER-skill fighter** (3 tie), a sign
test at **p = 0.031**. Per rung it is sharper still:

| rung | LOWER : higher |
|---|---:|
| 3 vs 1 | 3 : 6 — **the correct direction** |
| 5 vs 3 | 7 : 2 |
| **6 vs 5** | **9 : 0** |
| 9 vs 6 | 5 : 4 |

⇒ The ladder points the right way at the bottom and inverts hardest at the 5→6
boundary, where the lower fighter outfought the higher one in **every one of the
nine fixtures**. ⚠ Under forced level-9 weights, so what differs at that boundary
is reaction 300→260ms, APM 200→240, noise 0.20→0.16, read 0.2→0.3 — and none of
those is obviously a way to get worse. That is the question the authored-weight
re-run exists to answer.

⚠ **A hypothesis I tested, dropped, and then found I had tested WRONG** —
recorded at length because the error is a reasoning trap, not a wrong number, and
it is the kind that survives review.

The hypothesis: the fixtures handicap the subject (7 of 9 place SELF offstage or
in hitstun — *"Self is past a blastzone"*) and seat 0 is always the higher rung,
so the skew is placement rather than skill. I tested it by splitting the 36 cells
into the fixtures that handicap SELF and the two that advantage it, and found
**68%** favouring LOWER (n=28) against **62%** (n=8) — same direction, same size
— and concluded placement was not the cause.

⛔ **That comparison cannot detect the thing it was aimed at.** Both groups have
seat 0 = the higher rung. A confound that is UNIFORM across the groups being
compared is invisible to the comparison: if seat 0 is simply a worse seat — for
any reason, including ones no fixture premise mentions — every group skews the
same way and the split shows nothing. I compared whether the fixture's PREMISE
mattered and reported the answer as though it were about the SEAT.

⇒ The control that can see it is not a subgroup split but a swap: play the same
seed with the rungs exchanged between seats, which is what `--paired` was built
for. ⭐ **AND IT DID NOT REPRODUCE: the paired matrix below reads 16 : 19, and 14
of the 36 cells change verdict.** The 24 : 12 skew was the seat, the hypothesis I
"refuted" was right, and my refutation was a comparison that could not see its own
subject. ⇒ **A subgroup comparison is not a control for a variable every subgroup
holds constant** — and the null it produces is indistinguishable from a real one.

### What all nine fixtures actually SAY at the rig's own seed count

⭐ **MEASURED 2026-09-03, `smash_tool ladder-rig --scenarios` at the rig's
`DEFAULT_SEEDS = 15`, 60s cap, 9 fixtures × 4 rungs (1v3, 3v5, 5v6, 6v9) = 36
verdicts.** The rig now reaching 9 of 9 was worth having for this reason: with
every fixture running, the matrix can be read as a whole for the first time.

⛔ **IT DOES NOT DISCRIMINATE SKILL. 35 of the 36 verdicts are inside the seed
spread — the rig's own `(within spread)` qualifier.** The full count:

| verdict | count | reads as |
|---|---:|---|
| `both survive (within spread)` | 17 | 60s cap saturated; no information |
| `LOWER lasts (within spread)` | 16 | inside noise |
| `higher lasts (within spread)` | 1 | inside noise |
| `both survive` | 1 | cap saturated |
| **`LOWER lasts`** | **1** | **the only result outside spread** |

The single outside-spread verdict is `ledge_trap 9 vs 6`, **13.2s : 17.7s — the
LOWER-skill fighter outlasting the higher.**

⚠ **THE HONEST NUMBER HERE IS 1, AND I FIRST WROTE 17.** Grepping the verdict
words alone gives *"LOWER lasts at 17 of 18 discriminating rungs"*, which reads
as a spectacular ladder inversion and is an artifact of dropping four characters:
`(within spread)` is part of the verdict, not decoration on it. A count that
discards a significance qualifier reports noise as a finding — the same instrument
error as counting a substring and calling it a concept. **A verdict travels with
its qualifier or it does not travel.**

⇒ **What this means for the ladder, stated carefully.** This is NOT evidence the
skill ladder is inverted; it is evidence the RIG cannot currently tell. Two
distinct causes are visible in the same table and neither has been separated yet:

1. **The low rungs saturate the cap.** All 9 fixtures at both 1v3 and 3v5 return
   `both survive` — 60 seconds is not long enough for weak CPUs to resolve
   anything, so half the matrix is structurally incapable of an answer.
2. **The high rungs stop scoring.** At 5v6 and 6v9 the stocks-left columns are
   `0 : 0` almost everywhere, while the low rungs show 1–2 stocks taken. Stronger
   CPUs take FEWER stocks, not more. Survival-time-until-timeout is the metric,
   and it rewards passivity: a fighter that never commits cannot be punished.

⇒ **Next measurement, before any brain change is justified by this table:** the
rig needs a scoreboard that is not survival-against-a-cap — damage dealt per
engagement, or stocks taken per minute — and the cap needs to be long enough that
the low rungs resolve. Until then, "the ladder is fine" and "the ladder is
inverted" are BOTH unsupported by this run, and the table above is the reference
point that says so.


### ⭐⭐ THE 5→6 BOUNDARY IS ROLLOUT-ON vs ROLLOUT-OFF (found 2026-09-04)

The matrix kept saying the same thing about one rung and I kept treating it as
part of a general skew. It is not general: it is a **step function**, and the
step is in the engine floor.

`FighterBrainProfile::for_level` interpolates every parameter linearly in
`t = (level - 1) / 8` — reaction, APM, execution noise, read weight — **except
two**:

```rust
rollout_depth: if level >= 6 { 12 } else { 0 },
rollout_k:     if level >= 6 { 4 }  else { 0 },
```

⇒ **Level 6 is the first rung that runs the L3 rollout search at all.** So the
`6 vs 5` cell is not "a slightly better fighter against a slightly worse one" — it
is *rollout on against rollout off*, and every other rung pair compares two
fighters that both search or both do not.

⇒ **And `6 vs 5` is the cell that will not go away.** Unpaired it was **9 of 9**
fixtures favouring the LOWER rung. Under `--paired`, which cancels the seat and
the placement, `5 vs 3` FLIPPED outright (4:1 toward LOWER became 1:4 toward
higher — it was a seat artifact) while **`6 vs 5` did not move**. An effect that
survives the control that killed its neighbour is not the same kind of thing as
its neighbour.

⚠ **This is a hypothesis with a mechanism, not a conclusion.** It says the rung
where rollout switches on is the rung that under-performs; it does not yet say
the rollout causes it. The rig has `--no-rollout`, which turns exactly that
switch off for every fighter, so the arms are:

| arm | expectation if rollout is the cause |
|---|---|
| `--paired` | `6 vs 5` favours LOWER (the standing result) |
| `--paired --no-rollout` | `6 vs 5` stops favouring LOWER |

⭐⭐ **BOTH ARMS RAN, AND THE SWITCH FLIPS THE VERDICT.** `ladder-rig --seeds 10
--paired`, engine floor, flat stage, with and without `--no-rollout`:

| rung | rollout ON (default) | rollout OFF |
|---|---|---|
| 3 vs 1 | higher outfights | higher outfights — **byte-identical row** |
| 5 vs 3 | LOWER outfights | LOWER outfights — **byte-identical row** |
| **6 vs 5** | **LOWER outfights** · 51.8s : 56.3s · stocks **0 : 0** · dealt 147% : 149% | **higher outfights** · >60s : >60s · stocks **2 : 2** · dealt 227% : 222% |
| 9 vs 6 | higher outfights | higher outfights |

⭐ **The control is built into the design and I did not have to arrange it.**
Rollout is already off below level 6, so `--no-rollout` cannot change the two
bottom rungs — and it does not: those rows are identical to the digit between the
arms. Anything that moved at `6 vs 5` is the rollout and nothing else.

⇒ **Turning level 6's rollout off flips the rung the right way up.** At `6 vs 5`
only the HIGHER fighter has rollout by default, so this arm removes exactly one
fighter's search — and the verdict inverts.

⚠ **And the whole match changes character, which is the part I did not predict.**
With level 6's search on, *both* fighters end **fully eliminated** (0 stocks each)
having dealt ~148%. With it off, *both* survive with **2 stocks** and deal ~225%.
Removing one fighter's rollout made the other one survive too. ⇒ A rollout-driven
fighter appears to play in a way that gets BOTH bodies killed — reckless
commitment, or dragging the fight somewhere lethal — rather than simply losing.

### ⭐⭐ THE SHIPPED LADDER TURNS THE ROLLOUT OFF ON EVERY ROW — so none of this reaches a player

**Checked 2026-09-04, and it reframes the whole investigation above.**
`game/ambition_content/assets/data/fighter_brain_ladder.ron` carries
`rollout_depth: 0` on **all nine rows**, and says why in its own header:

> *"Rollout fields remain zero until rollout fidelity is good enough to enable
> them without changing lower-level behavior."*

⇒ **In the shipped game no fighter at any level runs the L3 rollout.** The real
app composes `ambition_content`, gets the authored rows, and every rung has the
search disabled. The rollout runs ONLY where the engine FLOOR supplies the profile
— `for_level`'s `if level >= 6 { 12 }` — and the floor is what the DEMO app falls
back to, because it does not depend on `ambition_content`.

⇒ **So the defect chain resolves like this, and the order matters:**

1. The dodge/shield suppression and the lethality inversion are real and measured.
2. They occur only under the engine floor, i.e. **only in the rig's demo app**.
3. **No player has ever met them**, because the authored ladder disables the
   search everywhere.

⭐ **AND THE AUTHORS WERE RIGHT, WITH EVIDENCE THEY DID NOT HAVE.** The ron's
comment is a precaution — *"until rollout fidelity is good enough to enable them
without changing lower-level behavior"* — and the measurements above are exactly
the change in lower-level behaviour it was guarding against: a fighter that stops
dodging and shielding entirely, and a match that goes from unresolvable in 60s to
both fighters losing every stock. ⇒ **This page's finding is the justification for
a decision already taken on instinct**, which is the most useful thing a
measurement can be.

⚠ **What it does NOT do is lower the priority to zero**, for two reasons:

- **It corrupts the rig.** Every ladder measurement runs on the floor, so every
  cell at rung 6 and above has been measured with a search the shipped game never
  uses. That is the same class of error as the missing `AuthoredFighterLadder`,
  and it is the same cause.
- **It is the blocker on ever enabling rollout.** The ron will not move to a
  non-zero depth until this is fixed, so `Dodge` in the shadow is the work that
  unblocks a capability the ladder has been holding in reserve.

⇒ **And it sharpens the ownership question already with Jon.** Option (a) — the
demo composes `ambition_content` — would turn the rollout OFF in the rig as a side
effect, because the authored rows zero it. That single change removes the defect
from every future measurement without touching the brain at all.

### ⛔⛔ THE ROLLOUT EXPLAINS ABOUT TWO CELLS OF NINE — the full arms, 2026-09-04

**Both 15-seed paired scenarios matrices finished, and they overturn the reading I
took from a 10-seed ladder arm.**

| rung | rollout ON | rollout OFF |
|---|---:|---:|
| 3 vs 1 | 2 : 6 higher | 2 : 6 higher |
| 5 vs 3 | 2 : 7 higher | 2 : 7 higher |
| **6 vs 5** | **9 : 0 LOWER** | **7 : 2 LOWER** |
| 9 vs 6 | 3 : 6 higher | 3 : 6 higher |
| all | 16 : 19 | 14 : 21 |

⇒ **Only `6 vs 5` moves, and it moves by two cells.** Seven of nine fixtures still
favour the lower rung with the rollout switched off entirely. So the rollout is a
CONTRIBUTOR to that cell and nowhere near the whole of it.

⛔ **AND THE PLAIN-MATCH LADDER SAYS LEVEL 6 IS FINE.** Away from the fixtures,
running the rung both ways round at 12 seeds (`--rungs 5,6` and `--rungs 6,5`,
which swaps which level sits in which seat):

| ordering | level 6's seat | dealt (level 6 : level 5) | who won |
|---|---|---:|---|
| `5,6` | seat 0 | **142% : 133%** | level 6 |
| `6,5` | seat 1 | **207% : 150%** | level 6 |
| paired | both | **161% : 133%** | level 6 |

⇒ **Level 6 beats level 5 in every ordering once you follow the LEVEL rather than
the column heading.** The ladder is not broken at this rung in an ordinary match.

⚠⚠ **So I built a narrative on an underpowered arm and should say so plainly.**
The `--seeds 10` paired ladder run read `6 vs 5` as favouring the lower rung, and
`--no-rollout` flipped it — which I recorded as *"turning level 6's rollout off
flips the rung the right way up"*. At 12 seeds the BASELINE already favours level
6, so what flipped was noise, not the rung. Both cells were `(within spread)` and
I read a direction out of them anyway, which is the exact error the spread
qualifier exists to prevent and which this page had already caught me making once.

⇒ **The resolved picture, and it is narrower and stranger than the one I had:**

1. **The ladder is fine in plain matches at `6 vs 5`.**
2. **The FIXTURE scenarios invert it — 7 of 9 even with rollout disabled.** That is
   a property of the fixtures, not of the difficulty ladder.
3. **Rollout accounts for about 2 of those 9 cells.**
4. ⇒ The remaining question is not *"why is level 6 worse"* but **"why do these
   nine fixtures reverse a rung that a plain match gets right"** — and the
   fixtures place SELF (always the higher rung) in a bad spot in seven of nine,
   which pairing cancels only if the handicap does not INTERACT with skill.

### ⭐⭐ AND THE ROLLOUT'S COST IS VISIBLE, CONSTANT, AND THE VERDICT CANNOT SEE IT

Reading the `6 vs 5` cells by COLUMN rather than by verdict turns the picture
sharp. Survival gap = seat 1 (level 5) minus seat 0 (level 6), paired, 15 seeds:

| fixture | rollout ON | rollout OFF |
|---|---:|---:|
| ledge_trap | **+4.5s** | **+0.0s** |
| juggle_escape | +3.4s | +0.0s |
| projectile_camper | **+4.5s** | **+0.0s** |
| edgeguard_window | **+4.5s** | **+0.0s** |
| edgeguard_ledge_hang | **+4.5s** | **+0.0s** |
| recovery_left | **+4.5s** | **+0.0s** |
| recovery_right | **+4.5s** | **+0.0s** |
| recovery_below | **+4.5s** | **+0.0s** |
| recovery_above | +1.7s | **+0.0s** |
| **median** | **+4.5s** | **+0.0s** |

⇒ **The fighter with the rollout dies about four and a half seconds sooner, in
every fixture — and with the rollout off the two die on the SAME TICK in all
nine.** A row of nine zeroes is as clean a control as this rig has produced, and
it appears nowhere else in the matrix: `3 vs 1` and `5 vs 3` sit at the 60s cap,
and `9 vs 6` (where BOTH fighters search) shows no constant at all.

⭐ **AND IT IS NOT AN ARTIFACT OF THE PAIRING — the UNPAIRED matrix shows the
same constant**: +4.5s in **8 of 9** fixtures, median +4.5, identical to the
paired median. ⇒ The three arms line up as cleanly as this rig has ever managed:

| arm | 6 vs 5 survival gap |
|---|---:|
| unpaired, rollout ON | median **+4.5s** (8 of 9) |
| paired, rollout ON | median **+4.5s** (7 of 9) |
| paired, rollout OFF | **+0.0s in all 9** |

⚠ **Exactly 4.5s in seven of nine is not behaviour.** Behaviour varies with the
scenario; a constant does not. ⇒ The shape says one discrete event — a stock lost
one cycle earlier, most likely — rather than a diffuse "plays worse". The
constant's origin is NOT yet identified: it is not `RESPAWN_PROTECTION_SECONDS`
(2.0), not `RESPAWN_INTERVAL_SECONDS` (1.0), not their sum (3.0), and not
Mary-O's `DEATH_DWELL` (3.2, and another demo's besides).

### ⭐⭐ SETTLED BY PER-BOUT DATA — and the zero meant the opposite of "they died together"

I flagged "+0.0 in all nine" as deserving suspicion rather than celebration, and
it did. `--per-bout` (added for this) prints every bout instead of the median, and
the answer is unambiguous.

**Rollout ON**, `6 vs 5`, five seeds — eliminated ticks, seat 0 : seat 1:

```
1273 : 1543     3132 : 3402     1874 : 2144     2248 : 2518     3600 : 3600
```

⇒ **Exactly 270 ticks apart in every resolved bout** — 4.5s at 60Hz, to the tick,
across damage totals from 23% to 206%. Not a median artifact and not behaviour:
270 is a CONSTANT. ⇒ It is the teardown beat between the loser's elimination and
the winner's body being removed, so the winner's `eliminated` value is **not a
death** — it is the match ending. ⚠ **The rig's survival column conflates "died"
with "the match finished and your body was cleaned up"**, which is worth knowing
before anyone quotes a survival time again.

**Rollout OFF**, same seeds:

```
3600 : 3600     3600 : 3600     3600 : 3600     3600 : 3600     3600 : 3600
```

⇒ **Every bout hits the 60s cap with both fighters alive** (stocks 2:2, 2:1, 1:3,
3:2, 3:2). The "+0.0 gap" was never two deaths on one tick — **it was no deaths at
all.**

⇒ **So the finding inverts into something much bigger than a rung.** With one
fighter running the rollout, a `6 vs 5` match RESOLVES inside 60 seconds and both
fighters lose every stock. With neither running it, the same two fighters cannot
finish each other in 60 seconds and end with stocks in hand.

⇒ **The rollout does not make its holder slightly worse. It makes the fight
lethal — for both — and its holder dies first.** That is consistent with the
dodge/shield suppression measured above: a fighter that cannot dodge and cannot
shield trades hits until somebody runs out, and the one that cannot defend runs
out first.

⚠ **THE VERDICT AND THIS COLUMN ANSWER DIFFERENT QUESTIONS, and I first wrote
that as "the verdict is blind to it", which is too strong.** Removing the rollout
changes **2 of 9 verdicts** at this rung while changing **9 of 9 survival gaps**
from +4.5s to zero. Both fighters end at `0 : 0` stocks either way, so stocks tie
and the verdict is decided on damage dealt — and damage still favours the lower
rung once the survival gap has gone to zero.

⇒ So the verdict is not failing to see something it should: it reports a real
damage difference that OUTLIVES the survival difference. Level 5 both deals more
and lives longer, so the two columns agree here rather than conflicting.

⇒ **What is true, and worth a reader's attention, is narrower:** the rollout's
single most visible effect — four and a half seconds of life, in every fixture —
moves the verdict in only two cells, because the verdict never reaches survival.
⇒ **Read both columns.** A change can be large, consistent, and mechanically
obvious in this table while the verdict beside it barely moves, and neither number
is wrong.

⭐ **The dodge/shield suppression is untouched by this and remains true.** It was
measured directly — zero selections in 662 decisions — not inferred from win
rates. What changes is its consequence: it is a real hole in the shadow's
vocabulary that costs about two fixture cells, not the explanation for a broken
ladder rung.

### ⭐⭐ THE NULL CONTROL: a rung against itself, and the seat bias measured directly

**The prior question this rig had never been able to ask.** Every verdict it has
printed compares two DIFFERENT levels, so nothing answered *"do two IDENTICAL
fighters split evenly?"* — and a tool that cannot measure zero cannot be trusted
about small numbers. `--rungs 6,6` (added 2026-09-04) asks it.

| design | `6 vs 6` — the same level against itself |
|---|---|
| **paired** | `48.2s : 48.2s` · `0 : 0` · dealt **144% : 144%** · peak `95% : 95%` · **even** |
| **unpaired** | `52.4s : 48.2s` · `0 : 0` · dealt **144% : 154%** · peak `90% : 100%` · **LOWER outfights** |

⛔⛔ **THE PAIRED ROW IS AN ALGEBRAIC IDENTITY AND I FIRST RECORDED IT AS
EVIDENCE.** With equal rungs, `bouts_for_seed(6, 6, seed)` calls
`run_bout_at(6, 6, seed)` **twice with identical arguments**, so the pair is
`[B, B.mirrored()]` — a bout averaged with its own transpose. Equal columns are
guaranteed by construction, for any bout, on any instrument, biased or not. ⇒ It
confirms `Bout::mirrored` is symmetric end-to-end, which is a real if small check,
and it says **nothing whatever** about whether pairing removes bias for UNEQUAL
rungs. I wrote *"the instrument has no residual bias once the seat is
controlled"*; that sentence was unsupported by the row under it.

⇒ **The unpaired row is the one that measures something**, and it is the finding.

⛔⛔ **Unpaired, two fighters with NO difference between them produce
`LOWER outfights`** — seat 1 deals 154% against seat 0's 144%, a ~7% edge from
the seat alone. ⇒ **That is the 24 : 12 skew, in one row, with the skill removed.**
Every unpaired verdict whose margin is inside that band was reporting the seat.

⇒ **What it licenses, and what it does not.** It measures the seat term directly
and puts a size on it. ⚠ It does NOT independently verify that `--paired` removes
that term — the degenerate row above cannot, and no equal-rung run can, because
pairing equal rungs is a tautology. What supports the paired matrix is the
argument rather than this control: each rung stands in each seat equally often, so
a uniform seat term cancels in the mean. ⇒ A real check would need a
DELIBERATELY seat-biased fixture and a demonstration that pairing flattens it;
that experiment does not exist yet and should not be assumed. It does NOT license
the unpaired corpus — every number this rig produced before
`--paired` carries a seat term of roughly this size, in the direction that made
the ladder look inverted.

### The paired matrix, all 36 cells: one rung is broken and the rest are fine

⭐⭐ **MEASURED 2026-09-04. Controlling the seat repairs every rung except the one
where rollout switches on.** Same 9 fixtures × 4 rungs × 15 seeds, unpaired
against `--paired` (each seed run twice with the rungs swapped between seats):

| rung | unpaired LOWER : higher | paired LOWER : higher |
|---|---:|---:|
| 3 vs 1 | 3 : 6 | 2 : 6 |
| 5 vs 3 | **7 : 2** | **2 : 7** ← flipped outright |
| **6 vs 5** | **9 : 0** | **9 : 0** ← did not move |
| 9 vs 6 | 5 : 4 | 3 : 6 |
| **all** | **24 : 12** (p = 0.065) | **16 : 19** (p = 0.74) |

⇒ **14 of 36 cells changed verdict when the seat was controlled.** The overall
skew toward the lower rung — the thing I nearly published as "the ladder is
inverted" — evaporates: 24:12 becomes 16:19, and the sign test goes from
suggestive to nothing.

⇒ **And what is left is one rung, unmoved: `6 vs 5`, nine fixtures out of nine,
in BOTH designs.** Its neighbour `5 vs 3` flipped completely, which is what a
seat artifact looks like. An effect that survives the control that destroyed its
neighbour is a different kind of thing, and `6 vs 5` is exactly rollout-on
against rollout-off.

⇒ So the ladder is not inverted. **One rung is** — and the rollout is a large part
of why, but ⚠ **not all of it**, which the 10-seed ladder arm alone would have
told you wrongly.

⛔ **The stronger arm disagrees with the headline the weaker one suggested.** The
`--seeds 10` ladder run flipped `6 vs 5` outright when rollout was disabled, which
reads as "rollout is the cause". The 15-seed PAIRED SCENARIOS run with
`--no-rollout` — nine fixtures instead of one match, the same design as the
matrix above — moves that cell from **9 : 0** toward the lower rung to roughly
**5 : 2**. The uniformity breaks; the lean survives.

⇒ **So: rollout accounts for a large share of the `6 vs 5` inversion and does not
account for all of it.** Something else at that boundary is still favouring the
lower rung, and the remaining candidates are the floor's linear terms — reaction
300→260ms, APM 200→240, noise 0.20→0.16 — none of which is obviously a way to get
worse, which is what makes the residue worth chasing rather than assuming.

⚠ **Recorded this way deliberately.** I had the clean flip in hand from the
cheaper arm and could have stopped there; the expensive arm is what says the story
is partial. A measurement that confirms the hypothesis you already like deserves
the same second arm as one that refutes it.

⭐⭐ **A MECHANISM, ASSEMBLED FROM NUMBERS THIS PAGE ALREADY CARRIES — and it
says the 2026-08-31 fix may have traded one bug for its mirror image.**

The regression fixed on 2026-08-31 was that the rollout **PROMOTED** what it
could not simulate: an unjudged verb never appears in `suicidal_movement`, so a
`find` over "not vetoed" ranked it above everything the rollout actually judged.
The fix demoted the unjudged to a third tier, below judged-and-unvetoed and below
`least_bad`.

⇒ Now read that against this page's own trace census, one seed, 6,388 decisions:

| field | count | share |
|---|---:|---:|
| `unmodelled=[..]` | **983** | 15.4% |
| — of which `Dodge` | **846** | 86% of the unmodelled |

⇒ **`Dodge` is the verb the rollout cannot simulate, and it is nearly all of the
unmodelled traffic.** So a fighter WITH rollout has dodge relegated to a tier
reached only when nothing judged survives, while a fighter WITHOUT rollout (every
rung below 6) runs the untouched path where dodge competes on equal terms.

⇒ **The hypothesis, stated so it can be killed:** the rollout does not make level
6 choose badly among the options it judges — it makes level 6 **stop dodging**,
and a platform fighter that will not dodge takes more damage and dies more. That
predicts exactly the shape measured above: rollout-on fighters fully eliminated
(0 stocks each) where rollout-off fighters survive with 2.

⭐⭐ **TESTED, AND IT IS WORSE THAN THE HYPOTHESIS. A ROLLOUT FIGHTER NEVER
DODGES AND NEVER SHIELDS.** `AMBITION_FIGHTER_TRACE=1`, `ladder-rig --seeds 1`,
run twice with and without `--no-rollout`, counting `chose=Some(..)` per rung —
the rig prints its summary row after each rung, so the trace lines between two
rows belong to the rung named by the later one:

| rung | decisions on / off | Dodge% on | Dodge% off | Shield% on | Shield% off |
|---|---|---:|---:|---:|---:|
| 3 vs 1 | 1344 / 1344 | 19.8% | 19.8% | 11.2% | 11.2% |
| 5 vs 3 | 1324 / 1324 | 18.3% | 18.3% | 10.5% | 10.5% |
| **6 vs 5** | 798 / 1300 | **5.0%** | 12.1% | **4.6%** | 10.3% |
| **9 vs 6** | 662 / 1384 | **0.0%** | 14.9% | **0.0%** | 13.9% |

⭐ **The control is exact and free**: rollout is already off below level 6, so the
two bottom rungs must be unchanged — and they are identical to the decision, the
verb and the tenth of a percent. Everything that moved is the rollout.

⇒ **At `9 vs 6`, where BOTH fighters have rollout, dodge and shield are selected
ZERO times in 662 decisions.** At `6 vs 5` only the higher fighter has it, and the
rung's rate falls to roughly the mix of one fighter dodging normally and one not
dodging at all. The verbs are not demoted; they are **unreachable**.

⇒ **Why unreachable and not merely rarer — and the gate is one `.or` earlier than
"nothing judged survives" suggests** (sharpened by toothbrush, re-derived here).
`pick_movement` (`ambition_combat/src/brain/fighter/decision.rs:680`) is:

```rust
find(|o| !vetoed(o) && !unmodelled(o))   // tier 1 — EXCLUDES the unmodelled
    .or(least_bad)                        // tier 2
    .or_else(|| find(|o| !vetoed(o)))     // tier 3 — the ONLY path an unmodelled verb has
```

⇒ Tier 3 runs only when tier 1 finds nothing **and `least_bad` is `None`**. It is
not the veto set that closes it; it is `.or(least_bad)` catching the fall. So the
zero-in-662 follows from `least_bad` being `Some(..)` whenever tier 1 fails, and
tier 1 fails rarely because the rollout always judges `Approach`, `Retreat` and
`Jump`.

⚠ **The ATTACK side does not have this defect, checked so nobody assumes
symmetry.** `refine_by_rollout` does `options.attacks.iter().take(rollout_k)` and
picks the best of those — a bounded RE-RANK of L2's top few, which is the
documented cost control, not an exclusion. An attack beyond `k` keeps L2's
ordering; a movement verb in `unmodelled` loses tier 1 entirely. The asymmetry is
that one path narrows a ranking and the other removes candidates from it.

⭐ **And the same function's doc comment explains why a NO-rollout fighter dodges
normally**: *"TIER 3 IS ALSO THE NO-ROLLOUT PATH. With rollouts off nothing is
judged and nothing is vetoed, so L2's order comes straight through tier 3
unchanged."* ⇒ The asymmetry is exact — running the rollout is what POPULATES
`unmodelled`, and populating it is what removes those verbs from tier 1. The 2026-08-31 fix stopped the rollout PROMOTING what it cannot
simulate; the same ordering now DELETES it. Both unmodelled verbs from this page's
census — `Dodge` (846 of 983) and `Shield` (137) — are exactly the two that vanish.

⇒ **So a platform fighter's entire defensive vocabulary switches off at level 6**,
which is the rung the difficulty ladder calls better. That is the whole 5→6
inversion, and it explains the character change too: a fighter that never dodges
and never shields trades damage until both bodies die.

⇒ **The repair is to make the shadow able to simulate a dodge, not to reorder the
tiers a second time.** Reordering is what produced each of these two bugs in turn.

#### Half the repair landed: `Shield` is modelled now (2026-09-04)

`movement_intent` maps `MovementVerb::Shield => ShadowIntent::Hold`. That is a
MODEL, not a guess: a body raising its guard SETTLES — the movement code brakes
it, and its own comment says *"planting yourself is what makes a raised guard a
decision rather than a free slide"* — and `Hold` coasts to a stop at
`ground_coast_decel`. So the shadow can now answer *"does guarding here kill me"*
instead of declining to answer.

⛔ **`Dodge` and `Blink` stay unmodelled deliberately, and `Hold` would be the
wrong answer for both**: an air dodge SETS a velocity and a blink teleports.
Modelling them as stationary is exactly the *"lying in one direction"* the
function's own header refuses. Dodge is 846 of the 983 unmodelled decisions and
is the larger half of this repair; it needs real motion in the shadow.

⭐ **AND THAT MOTION IS DERIVABLE — the reason Dodge is unmodelled is that nobody
wrote the model, not that the shadow cannot know where a dodge goes.** Checked
2026-09-04, both ends:

- **What a dodge does.** `ambition_platformer2d_core/src/movement/abilities.rs:580` sets
  `vel = (side·aim.x + down·aim.y) * air_dodge_speed`, held for `air_dodge_time` —
  a 2D aimed burst. `ShadowIntent::Dash` is the right SHAPE (velocity set along a
  direction for a duration) and the wrong dimensionality: it is lateral-only.
- **Where it aims.** ⭐ The brain already decides. `decision.rs:1112` says *"all
  this verb decides is the DIRECTION"* and computes it from a `threatened` read —
  away from a swing, toward everything else — over the SAME `view` the rollout
  holds. So a shadow model would recompute a direction, not invent one.

⇒ **The repair is a `ShadowIntent::Evade { dir: Vec2 }`** that sets velocity to
`dir * air_dodge_speed` for `air_dodge_time` and then resumes, with `dir` taken
from the same `threatened` predicate the emitter uses. The shadow already knows
`on_ground`, which is what separates the grounded roll from the air dodge.

⚠ **Invulnerability does NOT need modelling for this.** The rollout's question is
"does this verb kill me", which is a question about where the body ENDS UP; the
i-frames change what happens to it on the way and not where it lands. A motion
model is sufficient for the veto, which is what keeps this repair small.

⭐ **The suppression is measurably fixed for `Shield`** (same instrument, same
seed, controls unchanged to the decimal):

| rung | Shield% before | Shield% after | Dodge% before | Dodge% after |
|---|---:|---:|---:|---:|
| 3 vs 1 | 11.2% | 11.2% | 19.8% | 19.8% |
| 5 vs 3 | 10.5% | 10.5% | 18.3% | 18.3% |
| 6 vs 5 | 4.6% | **12.1%** | 5.0% | 2.5% |
| 9 vs 6 | **0.0%** | **5.7%** | **0.0%** | **1.3%** |

⇒ Dodge reappears at all (0.0% → 1.3%) because a Shield that is now JUDGED can be
VETOED, and a vetoed tier-1 option is what finally lets tier 3 be reached.

⚠⚠ **THE LADDER EFFECT IS NOT SETTLED, AND IT IS NOT ALL GOOD.** Paired ladder,
10 seeds, before → after:

| rung | before | after |
|---|---|---|
| **6 vs 5** | LOWER outfights | **higher outfights** ✔ |
| **9 vs 6** | higher outfights | **LOWER outfights** ✘ |

⇒ The rung the whole investigation was about flips the right way; the rung above
it flips the wrong way. Both cells are `(within spread)` at 10 seeds, so neither
is individually significant, and at `9 vs 6` BOTH fighters gained the shield
model where at `6 vs 5` only one did.

⇒ **Kept on correctness grounds, not on outcome.** A verb the brain offers and can
NEVER select is a defect whatever it does to win rates, and the model is honest.
But the ordering effect is unresolved: a 15-seed paired scenarios comparison
(pre-fix already recorded, post-fix running) is the verdict, and if it shows a net
regression this is one commit to revert.

⛔ **Do not "fix" this by re-promoting unmodelled verbs.** That is precisely the
2026-08-31 bug, and this page records what it cost. A verb the rollout cannot
model is a hole in the model; ranking it is choosing which direction to be wrong
in, and both directions are wrong.

⚠ **Held at hypothesis-with-strong-evidence, not proof.** Both `6 vs 5` cells are
`(within spread)` at 10 seeds in the ladder mode; what carries the weight is the
DIRECTION flip against a byte-identical control, plus the scenarios matrix where
`6 vs 5` was **9 of 9** fixtures unpaired and **5 of 5** paired toward LOWER while
its neighbour flipped. A 15-seed paired scenarios run with `--no-rollout` is the
confirmation and is queued.

⚠ **Prior art, and it is why this needs to be measured rather than announced:**
this document already records a *"Level-6 rollout regression — DIAGNOSED AND
FIXED 2026-08-31"*, and `tracks.md` warns against reinterpreting that regression
as proof the navigation architecture is wrong. ⇒ Either the fix did not fully
land, or this is a second effect at the same boundary. Both arms are running.

### What the tiers did to the fight — flat vs platforms, measured 2026-09-04

⭐ **The first measurement this project has of a stage CHANGING the fight**, and
the reason `super-smash-siblings.md`'s checkpoint 4 asked for more than one
layout. Same 9 fixtures × 4 rungs × 15 seeds × 60s, same outcome verdict, same
engine-floor ladder, unpaired, `--stage flat` against `--stage platforms`.

⚠ **The two stages are identical except for the three tiers.** Same
`STAGE_SIZE`, same blast margins, same solid floor — and the fixtures' starting
positions are *literally the same coordinates*, because
`starting_positions_on` maps proportionally onto the room's AABB and
`stage_bounds` derives that from `world.size`, which both stages share.

| | flat | platforms |
|---|---:|---:|
| mean peak% carried, per fighter | **107.7** | **81.5** |
| mean stocks LEFT (both seats) | **1.64** | **3.78** |
| cells containing an unfought bout | 1 / 36 | **11 / 36** |
| unfought bouts (of 540) | 3 | **41** |

⇒ **The tiers roughly halve the lethality.** Fighters end with more than twice
the stocks and carry a quarter less damage, and the rate at which a 60-second
bout ends with NEITHER fighter landing a hit goes from 3 in 540 to 41.

⇒ The verdict split barely moves (flat 24:12 toward LOWER, platforms 20:14, every
cell still `(within spread)`), so the stage changes HOW MUCH happens far more
than it changes WHO wins.

⭐ **A mechanism, traced in code rather than inferred from the numbers.**
`WorldView::is_standable` counts `SolidKind::OneWay`, so `ground_below()` finds a
tier; and `situation.rs:85` classifies `Recovery` from
`!terrain.is_empty() && !on_ground && ground_below().is_none()`. ⇒ On the
platformed stage a fighter above a tier **has ground below and is therefore not
Recovering** — correct behaviour, and a large one: situations that were recovery
scrambles on the flat stage are neutral here, and a fighter that would have been
edgeguarded or self-destructed instead lands and plays.

⛔ **Two caveats a reader needs before generalising this to "platforms".**

1. **A fixture is a pair of coordinates, not a situation.** `recovery_above`
   drops SELF at (320, −32); this layout's top tier occupies y 180–196 across
   x 236–404, so the fall is INTERCEPTED. The fixture's premise — *"Self is past
   a blastzone"* — is simply less true here, and four of the nine fixtures are
   recovery fixtures.
2. **This layout's top tier sits ten pixels under the respawn platforms**
   (`smash_platform_stage`'s doc has the arithmetic). A fighter whose respawn
   grace expires drops onto the tier rather than returning to the stage, so part
   of the measured gentleness is that specific geometry rather than tiers in
   general.

⇒ So the honest claim is **"this layout halves the lethality"**, not "platforms
do". Fixing the respawn overlap and re-running is the next measurement, and the
geometry and the number have to move together — the respawn/tier collision is with
Jon in [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md)
because all three ways out are his.

⚠ **A diagnostic exists for the unfought bouts and its first run was abandoned
on purpose.** `Bout::closest_approach` now records how near the two seats ever
came, and an unfought row prints the median of it — *"neither landed a hit"* has
two causes fixed in different places: they never reached each other (navigation)
or they reached each other and declined (scoring). A one-seed probe on this layout
read `unfought 1/1, closest 130px` — approached to about three body widths and
never into range. ⛔ The full platforms re-run was started and then STOPPED,
because it had begun after the rollout shield model landed and would therefore
have compared its unfought count against the 41 above across a behaviour change.
A number is worth less than the confidence that it is comparable.

### ⭐⭐ FOUR MATCHED ARMS: the shipped ladder measured at last, and George measured for the first time

**Method, because every number below depends on it.** One binary
(`target/release/smash_tool`, built from `365be9a53`), four arms launched from it
within seconds of each other, `--paired --seeds 12`, default stage, medians over
the 12 seeds × 2 seat orders. The arms differ in exactly one input each:

| arm | ladder | rollout | fighters |
|---|---|---|---|
| **A** | `--ladder game/ambition_content/assets/data/fighter_brain_ladder.ron` | off (the rows zero it) | the Robots |
| **B** | engine floor | ON at rung ≥6 | the Robots |
| **C** | engine floor | forced off (`--no-rollout`) | the Robots |
| **D** | the shipped `.ron` | off | **George vs George** |

⭐ **Stocks LEFT of three, at 60 seconds:**

| cell | A shipped | B floor+rollout | C floor−rollout | D **George** |
|---|---|---|---|---|
| 3 vs 1 | 2 : 1 | 2 : 2 | 2 : 2 | **0 : 0** |
| 5 vs 3 | 2 : 2 | 2 : 2 | 2 : 2 | **1 : 1** |
| 6 vs 5 | 2 : 2 | **0 : 0** | 2 : 2 | **1 : 1** |
| 9 vs 6 | 2 : 3 | **0 : 0** | 2 : 2 | **1 : 1** |

⇒ **Four things fall out, in descending order of how sure I am.**

**1. The rollout is the lethality switch, and this is the clean control.** B and C
differ in nothing but `--no-rollout`. Below rung 6 they are **byte-identical**
(3v1 and 5v3 agree to the digit), which is exactly right — the floor only arms the
rollout at level 6 — and that agreement is the arms' own validity check. At 6v5
and 9v6 they diverge completely: `0 : 0` against `2 : 2`. ⭐ **The rollout does
not make a fighter win. It makes both fighters die.**

**2. The shipped ladder behaves like the floor with the rollout off.** A tracks C
in structure (`2 : 2` throughout) rather than B. ⇒ Which is what the authored
`rollout_depth: 0` predicts, now confirmed from the other direction. A is not
identical to C — the authored rows differ in utility weights too — but the
*lethality* is governed by the rollout field alone.

**3. George is a substantially harder fighter than the Robots, and this is new.**
Nobody had ever pointed the rig at him. D's stocks-left are lower than A's in
**every** cell, and his damage dealt runs 297%/331% at the top cells against the
Robots' ~200%. ⇒ **So the "matches never resolve" character of every ladder
number in this document is substantially a fact about the STAND-INS**, who until
2026-09-04 had no special button, no forward tilt and no dash attack. ⚠ Not
entirely: George's own 5v3, 6v5 and 9v6 still end `1 : 1` rather than resolving.
He is faster, not fast enough.

**4. ⛔ THE SHIPPED LADDER INVERTS AT 5 vs 3, AND THAT IS SIGNIFICANT.**
Re-measured 2026-09-04 under the exact sign test (`bdd4af264`) after the previous
criterion was found to run backwards — see the section below, and note that only
the QUALIFIERS changed, never the columns. The 12-seed matrix, sixteen cells:

| cell | A shipped (Robots) | B floor+rollout | C floor−rollout | D George |
|---|---|---|---|---|
| 3 vs 1 | ⭐ **higher outfights** | within spread | within spread | within spread |
| 5 vs 3 | ⛔ **LOWER outfights** | within spread | within spread | within spread |
| 6 vs 5 | within spread | within spread | within spread | within spread |
| 9 vs 6 | within spread | within spread | within spread | within spread |

⇒ **Two significant cells, both on the shipped ladder, and they point opposite
ways.** Rung 3 beats rung 1 — the ladder working. Rung 3 also beats **rung 5** —
the ladder inverted.

⭐ **The inversion is not a near miss and it is not a stocks artifact.** Stocks are
level at `2 : 2`; the verdict falls to damage dealt, and the LOWER rung deals
more: **215% against 191%**. Ten or more of the twelve seed-pairs have to agree
for a cell to clear p < 0.05 at this n, so this is a consistent direction across
pairs, not one lopsided bout.

⚠ **And rung 5 is strictly "better" on every authored axis**, which is what makes
it a defect rather than a tuning preference. From the shipped `.ron`: reaction
400ms → 300ms, APM cap 120 → 200, execution noise 0.30 → 0.20, read weight
0.0 → 0.2, frame advantage 0.30 → 0.50, kill potential 0.10 → 0.30. ⇒ Every knob
moves toward "stronger", and the fighter deals *less* damage.

⛔⛔ **THE READ-WEIGHT HYPOTHESIS IS REFUTED, AND THE REFUTATION IS BIGGER THAN THE
HYPOTHESIS.** I guessed rung 5's non-zero `read_weight` was making it patient and
so weaker on a 60-second clock, and ran the arm: rung 5 with `read_weight` alone
reverted to `0.0`, 24 seeds, against a matched control.

⇒ **The two runs are BYTE-IDENTICAL.** `196% : 231%` dealt, identical survival
strings, identical verdict. ⚠ Not "the change did not help" — the change did not
*land*. That is the same signature as the flattened-ladder finding, and it means
the knob never reached the fighter.

⭐⭐ **IT NEVER REACHES ANY FIGHTER. `read_weight` IS INERT IN THE SHIPPED GAME.**
The chain, each link checked:

1. `read_weight` is authored on all nine shipped rungs, **0.0 rising to 0.9** — it
   reads like one of the ladder's main difficulty axes.
2. It has exactly two consumers. `HabitModel::read_bonus` has **no production
   callers at all** — `git grep read_bonus` finds its definition and four lines in
   its own test file. The other is `habits.read(situation)` at
   `rollout.rs:802`, behind `if read_weight > 0.0`.
3. That call is inside `refine_by_rollout`, which begins
   `if !profile.uses_rollouts() { return None }` (`rollout.rs:995`).
4. `uses_rollouts()` is `rollout_depth > 0 && rollout_k > 0`.
5. The shipped ladder sets `rollout_depth: 0, rollout_k: 0` on **all nine rows**.

⇒ So `read_weight` is read only through the rollout, and the shipped ladder
disables the rollout everywhere. **Nine authored values, one of the ladder's five
knobs, with no effect on the game.** ⭐ Proven three independent ways: the source
chain above, the absence of any production caller, and a byte-identical
measurement.

⛔ **AND IT IS NOT FREE.** `state.habits.observe(...)` runs on every decision
(`decision.rs:250`), and the model is serialized into **every rollback snapshot**
(`snapshot_impls.rs:451`, with its own comment explaining why a rewind must
restore it). ⇒ The shipped game pays per-tick observation and per-snapshot
bandwidth to maintain an opponent model that nothing ever reads.

⭐⭐ **AND `read_weight` IS THE ONLY DEAD KNOB — I checked all nine, which is the
reassuring half and it has to be said explicitly.** A finding like the one above
invites the reading that the ladder is riddled with inert fields, and it is not.
Every other knob's production consumers sit outside the rollout gate:

| knob | reached by | live? |
|---|---|---|
| `reaction_ms` | `perception.rs` | ✔ |
| `apm_cap` | `decision.rs`, `evaluation.rs` | ✔ |
| `execution_noise` | `decision.rs` | ✔ |
| `reach_fit`, `frame_advantage`, `expected_payoff`, `capture_value` | `options.rs` (the L2 scorer) | ✔ |
| `kill_potential`, `stage_risk` | `options.rs` **and** `rollout.rs` | ✔ via L2 — but see below: reverting either changed NOTHING measurable at `5 vs 3`, so reachable in source is not the same as active in a matchup |
| **`read_weight`** | `rollout.rs` only | ⛔ **dead** |

⇒ `options.rs` is the L2 scorer and runs at every rung, so the utility weights are
live — which the swap arm independently confirms, since changing them changed the
output. ⚠ The two weights that ALSO appear in `rollout.rs` are not affected: they
have a live L2 path as well, so losing the rollout costs them a refinement rather
than their existence.

⇒ **The distinguishing property is not "mentioned in `rollout.rs`" but "mentioned
NOWHERE ELSE".** That is the check worth repeating whenever a gate is disabled:
not which fields the gated code names, but which fields nothing outside it names.

⚠ **This also costs the `.ron`'s own precaution its justification.** Its comment
says the rollout fields stay zero *"until rollout fidelity is good enough to
enable them **without changing lower-level behavior**"* — but zeroing them already
changed lower-level behaviour, by silently switching off the entire read/habit
system. The comment was guarding against exactly the thing it caused, and nobody
could see it because the knob it disabled is still authored, still populated, and
still saved.

### ⭐⭐ AND THE INVERSION IS THE UTILITY WEIGHTS, NOT THE REFLEXES (measured 2026-09-04)

Two arms, 24 seeds each, paired, against a matched control at the same seed
count. Each swaps **one group** of rung 5's profile for rung 3's and changes
nothing else:

| arm | rung 5 is given | dealt (hi : lo) | verdict at `5 vs 3` |
|---|---|---|---|
| control | *(shipped, unmodified)* | 196% : 231% | ⛔ **LOWER outfights** |
| **weights** | rung 3's `utility_weights` | 212% : 204% | ✔ **higher outfights** *(within spread)* |
| **reflexes** | rung 3's reaction / APM / noise | 192% : 222% | ⛔ **LOWER outfights** |
| *(read_weight)* | `read_weight: 0.0` | 196% : 231% | ⛔ byte-identical to control |

⇒ **Giving rung 5 the WEIGHTS of rung 3 removes the inversion.** The cell goes
from significantly inverted to not-significant and the direction flips to the
correct one. ⇒ **Giving it the REFLEXES changes nothing** — still significantly
inverted, with the damage gap essentially intact.

⭐ **So the ladder's reflex progression is fine and its VALUE progression is
what makes rung 5 worse than rung 3.** The four weights that move between those
rungs: `frame_advantage` 0.30 → 0.50, `kill_potential` 0.10 → 0.30, `stage_risk`
−0.30 → −0.50, `expected_payoff` 0.10 → 0.30.

⚠ **Stated at the strength the evidence supports.** The weight arm lands *within
spread*, so what is established is that the weights are **necessary** for the
inversion — remove them and the significant inversion goes away — not that
rung 3's weights are better.

### ⭐⭐ NO SINGLE WEIGHT CARRIES IT — four more arms, and all four failed to fix it

I said separating the four weights was four more arms and that I would start with
`stage_risk`. I ran all four instead, each reverting **one** weight of rung 5 to
rung 3's value, 24 seeds, same control:

| arm | rung 5's weight reverted | dealt (hi : lo) | verdict |
|---|---|---|---|
| control | *(none)* | 196% : 231% | ⛔ LOWER outfights |
| `frame_advantage` | 0.50 → 0.30 | 196% : 222% | ⛔ LOWER outfights |
| `expected_payoff` | 0.30 → 0.10 | 196% : 221% | ⛔ LOWER outfights |
| `kill_potential` | 0.30 → 0.10 | 196% : 231% | ⛔ **byte-identical to control** |
| `stage_risk` | −0.50 → −0.30 | 196% : 231% | ⛔ **byte-identical to control** |
| **all four** | *(the earlier arm)* | 212% : 204% | ✔ higher outfights *(within spread)* |

⇒ **Every single-knob revert leaves the inversion significant. Only all four
together remove it.** ⭐ So it is not any one weight — which is the answer I would
not have guessed, and it is why running all four beat running the one I fancied.

⭐⭐ **AND THE DECOMPOSITION IS EXACT — it is a PAIR, and the pair is named.** Two
more arms, both predictions stated before running:

| arm | rung 5's weights reverted | dealt | verdict | vs |
|---|---|---|---|---|
| `frame_advantage` + `expected_payoff` | 2 of 4 | 212% : 204% | ✔ higher *(within spread)* | **byte-identical to the ALL-FOUR arm** |
| `kill_potential` + `stage_risk` | the other 2 | 196% : 231% | ⛔ LOWER outfights | **byte-identical to the CONTROL** |

⇒ **`frame_advantage` and `expected_payoff` carry the entire effect of swapping
all four; `kill_potential` and `stage_risk` carry none of it.** Not "mostly" —
byte-identical in both directions, over 48 bouts each. ⚠ And neither of the pair
suffices alone: both single arms stayed significantly inverted. So the inversion
needs *both* of those two, and is untouched by the other two.

### ⭐⭐ AND THE MECHANISM IS IN THE SOURCE, arrived at independently of the measurement

I found the pair by measurement and only then read what those two features are.
They agree, which is the strongest evidence either could have.

⛔ **`frame_advantage` is a SIGNED feature**, `-1..=1`:

```rust
pub fn frame_advantage(startup_s, their_commitment_s, slowest_startup_s) -> f32 {
    ((their_commitment_s - startup_s) / slowest_startup_s.max(0.01)).clamp(-1.0, 1.0)
}
```

⇒ **The slower a move is, the more NEGATIVE it scores** — and the kit's slowest
moves are its hardest-hitting ones. So the weight on this feature is not "how
well does this rung understand frame data"; it is **how heavily this rung
PENALISES committing to a slow move**. Rung 6 at `0.60` prices a smash's frame
risk twice as hard as rung 3 at `0.30`.

⭐ **And `expected_payoff` does not compensate — it compounds.** Its own doc:
*"the move's power … **gated by the positive part of `frame_advantage`** — payoff
only counts when the move plausibly lands. Zero across the board in neutral."*
⇒ So the reward for power applies **only to moves that already win the frame
trade**, which are the fast weak ones. Raising `expected_payoff` therefore
sharpens the same preference rather than offsetting it.

⇒ **The two knobs the measurement isolated are exactly the two that make a
fighter refuse to commit** — one penalising slow moves, the other withholding the
power bonus from them. And the shipped ladder raises both monotonically:
`0.30/0.10` at rung 3, `0.50/0.30` at rung 5, `0.60/0.40` at rung 6. ⇒ **Higher
rungs jab more and smash less, and on a 60-second clock that is less damage.**

⚠ **Where this stops being established.** The mechanism explains the DIRECTION and
predicts the effect should grow with the weights, which matches `3 > 5 > 6`. It
does not prove the causal path inside a match — nobody has traced a rung-6 fighter
declining a smash it would have thrown at rung 3. `AMBITION_FIGHTER_TRACE=1` and a
per-verb count would show that, and the standing rule below says to get it before
tuning anything.

⚠ **And note what the mechanism does NOT say: that the weights are wrong.** A
fighter that refuses bad commitments is playing better in a real sense; it loses
on *damage dealt in 60 seconds*, which is the rig's verdict metric and not
obviously the same as being harder to beat. ⛔ That is a genuine confound in the
measurement, it belongs to whoever rules on the ladder, and it is now recorded in
`awaiting-maintainer-decision.md` rather than resolved here.

⭐ **The second arm was a control on my own interpretation rather than a search
for an effect**, and it is the one that makes the first arm mean something: had
`kill_potential` + `stage_risk` moved the numbers at all, "the pair carries it"
would have been a coincidence of magnitudes instead of a decomposition.

⚠ **And two of the four contributed nothing measurable at this cell**:
`kill_potential` and `stage_risk` reverts came back byte-identical. ⚠⚠ That does
NOT make them dead knobs in the sense `read_weight` is — both are read by
`options.rs`, outside any rollout gate. The honest statement is narrower: **in
this matchup they never changed a decision.** A plausible reason is that their
terms are conditional on situations these fighters do not reach — nobody is ever
kill-confirmed here (matches end 2:2 at 60s) and the two may simply never
threaten a ledge. ⛔ Untested, and it would be tested by a scenario fixture that
forces those situations rather than by another rung arm.

⇒ **Running now**: `frame_advantage` + `expected_payoff` reverted TOGETHER — the
two that individually moved the numbers — against `kill_potential` + `stage_risk`
together, which should reproduce the control byte for byte if the reading above
is right. ⭐ That second arm is a control on my own interpretation, not a search
for an effect, and it is worth the run precisely because it can embarrass it.

⇒ **What this means for the ladder as a design.** Every knob in the shipped rungs
moves monotonically toward "stronger", and one whole group of them makes the
fighter measurably weaker on a 60-second clock. ⛔ That is not a tuning nit: it
means the ladder's authors had no way to tell, because the rig had never measured
the shipped rows and one of the five knobs does nothing at all.

⭐ **Method note worth more than the result.** Three of the four arms above were
run to *falsify* a hypothesis of mine, and all three did. The `read_weight` arm
returned a byte-identical file — and that non-result was the largest finding of
the day. ⇒ An arm that changes nothing is data about the wiring, not a wasted
run, and it is only legible if you diff the whole output rather than reading the
verdict.

⭐ **The interaction with the roster finding is the interesting part.** D (George)
shows the SAME direction at 5v3 — *LOWER outfights* — but within spread. ⇒ The
inversion is not an artifact of the stand-ins; it shows up in the authored fighter
too, just not distinguishably at 12 seeds. But it is *worse* for the Robots, which
fits the hypothesis: patience costs most when you have the fewest ways to convert
it, and until 2026-09-04 the Robots had no special button at all.

⚠ **What is NOT significant is as important.** Twelve of the sixteen cells carry
the qualifier, and `6 vs 5` and `9 vs 6` are undetermined in every arm. ⇒ The top
of the ladder is not measured — it is unmeasured. Nobody should read "the rungs
above 5 are fine" out of this table.

### ⛔⛔ AT 40 SEEDS EVERY CELL IS SIGNIFICANT, AND THE SHIPPED LADDER GOES BACKWARDS THROUGH ITS MIDDLE

The shipped ladder, 40 seeds, paired, sign test — **no cell carries a qualifier**:

| cell | dealt (hi : lo) | verdict |
|---|---|---|
| 3 vs 1 | 215% : 163% | ✔ **higher outfights** |
| 5 vs 3 | 191% : 229% | ⛔ **LOWER outfights** |
| 6 vs 5 | 197% : 214% | ⛔ **LOWER outfights** |
| 9 vs 6 | 231% : 220% | ✔ **higher outfights** |

⇒ **Read as an ordering: `1 < 3 > 5 > 6 < 9`.**

⛔⛔ **BUT READ THE VERDICT'S OWN DEFINITION BEFORE READING THAT SENTENCE.** The
verdict is *"who OUTFOUGHT: stocks taken, then damage dealt"* — and in **every
inverted cell the stocks are tied at `2 : 2`**, so the verdict falls through to
damage. ⇒ What is significant here is that the higher rung **deals less damage in
60 seconds**, NOT that it loses more often. Nobody has shown the higher rungs lose
matches.

⚠ **That is a real confound and it is mine to flag, not to resolve.** A fighter
that refuses bad commitments deals less damage per minute and may well be *harder
to beat*; "damage dealt on a 60-second clock" is the rig's tiebreak, not a
definition of difficulty. ⇒ The measurement is solid and its INTERPRETATION as
"the ladder goes backwards" is one reading of it. The reading that would settle it
is a longer clock or a stock-decided verdict, and it is queued below.

⭐⭐ **Rung 3 is a local MAXIMUM and rung 6 is a local MINIMUM.** A player climbing
the ladder gets a harder opponent from 1 to 3, then **two successive steps
BACKWARDS** — 3 → 5 → 6 each make the CPU measurably weaker — and only recovers
by 9. ⛔ That is the difficulty curve a player actually meets, and it has never
been measured before today because the rig had never read the shipped rows.

⚠ **The 6v5 cell only became significant with more seeds**, which is worth
stating plainly: at 12 seeds it carried the qualifier and I wrote *"the top of the
ladder is unmeasured, not fine."* That was the right thing to write and it was
also too cautious — at 40 seeds it is measured, and it is not fine.

⭐ **And this is the sign test's second self-validation.** Two significant cells at
12 seeds became four at 40. ⇒ Power rising with evidence is the property the old
range criterion had inverted, and it is now visible twice: once in a result it
would have destroyed (`3 vs 1`) and once in results it would never have found at
all (`6 vs 5`, `9 vs 6`).

⚠ **What it does not establish.** These are the Robots, not George; rungs 2, 4, 7
and 8 are unmeasured entirely; and the four cells are adjacent-pair comparisons,
so `1 < 3 > 5 > 6 < 9` is a chain of local comparisons rather than a global
ranking. A rung-1-vs-rung-9 arm would test whether the ladder's ENDS are ordered
even though its middle is not.

⭐⭐ **AND THE INSTRUMENT FIX PAID FOR ITSELF IMMEDIATELY, which is the cleanest
validation available.** The old range criterion called `3 vs 1` significant at 12
seeds and *lost* it at 40. The sign test calls it significant at **both** 12 and
40 seeds — the 40-seed arm reports `higher outfights` with no qualifier. ⇒ A
result that survives a 3.3× increase in evidence is what significance is supposed
to mean, and the old test had it exactly backwards. The fix did not manufacture
the finding; it stopped the instrument from destroying it.

⚠ **One reading artifact, so the next reader does not file it as a bug.** A's
`9 vs 6` shows stocks `2 : 3` under a verdict of *higher outfights*. Each column
is an independent median over the bouts, and the verdict is computed per bout —
so a median stock column and a verdict can disagree without either being wrong.
Read the columns as summaries, never as a single representative match.

### The Shield shadow model earns its place mechanically, not statistically (2026-09-04)

`MovementVerb::Shield => ShadowIntent::Hold` in `rollout.rs` was added so the
rollout stops returning `None` for a verb the fighter can actually pick. The
question was whether it changes outcomes, and the answer is **no, not measurably.**

⇒ Method: the paired scenario matrix before and after, compared cell by cell on
the **14 cells present in both runs** — the post-fix run covers 14 of the 36, so
this is a comparison on the intersection and not on the full matrix. One verdict
moved (`projectile_camper 6 vs 5`, LOWER → HIGHER). ⚠ **And every one of the 14
cells is `within spread` in both runs, including that one.** A single flip among
14 within-spread cells is what noise looks like; it is not evidence the model
helped, and it is equally not evidence it hurt.

⇒ So the model stays, on the mechanical argument rather than this one: the
alternative is a search that scores Shield as literally nothing, which is a
*wrong* number rather than a missing one. ⭐ But note what the measurement costs
it — the earlier plan to model `Dodge` next was resting on the Shield model
proving its worth first, and it has not. Model `Dodge` because `None` is wrong
for it too, or leave both; do not model it expecting a matrix to move.

⚠ **And this whole comparison is floor-only**, like everything else at rung 6+ in
this document: the rollout is the only consumer of `ShadowIntent`, and the section
above shows the shipped ladder disables the rollout on all nine rows. The Shield
model changes nothing a player meets. That is an argument for keeping it (it
cannot regress the game) and against prioritising `Dodge`.

### ⭐⭐ THE CLASS BEHIND THREE SEPARATE FINDINGS: the rig reads its config from anywhere but the shipped file

Four findings in this document have the same shape, and it is worth naming
because a fifth is otherwise inevitable. ⚠ The fourth arrived while this section
was being written, which is the strongest evidence that naming the class was
worth doing.

1. **The flattened ladder** — the rig overrode every rung with one weight set, so
   36 cells were byte-identical and the "ladder" it measured had one rung.
2. **The floor's reflex ladder** — with the override gone, the rig still read
   `FighterBrainProfile::for_level`, not the authored rows, because the demo app
   does not depend on `ambition_content`.
3. **The rollout** — that same floor switches the L3 search ON at level 6, and the
   shipped ladder switches it OFF on all nine rows.

4. **The fighters themselves** — found the same day, and it is the widest of the
   four. `ladder_rig`'s `fighters()` defaults to `smash_duelist_a` vs
   `smash_duelist_b`. ⛔ **Neither is George.** `register_character` hands George's
   authored table to `smash_george_booul` and `fighter_moveset()` to everybody
   else, and that stand-in contract bound **18 verbs to George's 26** — no
   `special`, no `special_forward`/`_up`/`_down`, no `attack_forward`, no
   `attack_dash`, no `taunt`. ⇒ **Every ladder number in this document was taken
   between two fighters with a dead special button, no forward tilt and no dash
   attack.** ⚠ And this one is not only an instrument defect: the demo's select
   roster is three characters, **two of them stand-ins**, and the catalog default
   is one of the two — so the dead button was a player's as well as the rig's.

⇒ **The class: the instrument took its configuration from a DIFFERENT SOURCE than
the shipped game, and only the instrument was ever read.** Every one of the four
was invisible from inside the rig's own output, because the rig faithfully
reported the thing it was actually running. Nothing was broken; the wrong subject
was measured, confidently, for as long as anyone looked.

⚠ **Why it kept recurring is the interesting part.** Each fix looked like it
closed the question. Removing the override was supposed to make the rig measure
the ladder — and it made the rig measure a *different* ladder. Every layer
underneath the one just fixed still defaults, and a default is exactly what does
not announce itself.

⭐ **The general rule this earns:** *a measurement is not of the shipped system
until it names the shipped file it read.* Not "we removed the override", not "the
demo uses the same code" — the actual path, printed in the run's own header, so
the claim and the evidence travel together.

⇒ Which is what `--ladder PATH` now does. `report_which_ladder_is_in_play` prints
whether `AuthoredFighterLadder` is installed, and a parse failure exits rather
than falling back to the floor, so a run whose header says "the AUTHORED rows"
cannot be a run that measured something else. ⚠ **That closes the reporting hole,
not the class.** The rig still reads the stage, the fixture roster and the seat
assignment from its own code; any of those can diverge from the shipped game the
same way, and none of them prints a source path yet.

⇒ **And the fourth instance shows the rule needs a second half.** `--character`
and `--opponent` already existed, so the fighter WAS nameable — the run just
defaulted, and the header never said to what. ⭐ So: *print every input the run
resolved, including the ones nobody passed.* A default that appears in the output
is a default somebody can notice is wrong; a default that appears only in the
source is the one that survives four investigations.

## Relationship to navigation/recovery architecture

The reusable recovery probe and `RecoveryLens` are legitimate body-capability
infrastructure, and the level-6 A/B was evidence that the **decision
integration** was wrong — which it was, and it is fixed. It was never evidence
that the generic platformer navigation architecture should be replaced, and the
trace confirmed that directly: the recovery road's own decisions matched between
the two arms.

Broader reusable navigation/reachability remains owned by
[`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md).

## No-cheat rule

Difficulty may change perception latency, decision cadence, execution noise,
planning breadth/depth and policy quality. It should not scale damage, read
privileged future state, bypass normal actor control, or remove physical
constraints merely to make a higher rung win.

### Re-measured 2026-09-03 — the rule HOLDS, by construction rather than by test

✔ **`apply_difficulty` cannot cheat, and you can see why in one function.**
`crates/ambition_combat/src/brain/smash/difficulty.rs:24` does exactly two
things: it DROPS an action when a roll exceeds `profile.commit_probability`
(movement excepted, so the actor never visibly freezes mid-step), and it JITTERS
aim direction by `profile.accuracy`. No damage term, no future state, no path
around actor control. The rule above is satisfied by what the function is, not
by vigilance.

⚠ **But "remain covered" in exit criterion 3 is generous.** The two tests beside
it — `movement_actions_skip_filter` and
`hard_difficulty_commits_attacks_almost_always` — pin what difficulty DOES. None
asserts what it must NOT do, so a future rung that reached for a damage
multiplier would pass them all. The constraint is currently a property of the
code's shape, which is a good place for it to be and a bad place to leave
unstated.

⛔ **AND "DIFFICULTY" NAMES TWO SYSTEMS HERE, one of which legitimately scales
damage.** `ambition_persistence::settings::gameplay::Difficulty` — the player's
Easy/Medium/Hard — has `damage_taken_multiplier()`, and a test asserting the
three are distinct. That is the player-facing setting and scaling damage is its
job. The rule on this page governs the BRAIN's authored profiles
(`reaction_delay_s`, `commit_probability`, `accuracy`).
⇒ An auditor checking this rule will find `damage_taken_multiplier` first and
must not read it as a violation. Two systems, one word.

## What to do next, in order (rewritten 2026-09-04, late)

⚠ **This list was rewritten because the day answered two of its four items and
demoted a third.** The previous ordering led with the ladder-ownership question
and put `Dodge` second; ownership is now partly settled by `--ladder` and `Dodge`
has lost its justification. Kept visible rather than silently replaced, because
the reordering IS the finding.

1. ⛔ **Answer `read_weight`: wire it into L2, or delete it?**
   ([`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).)
   One of the ladder's five knobs is authored on all nine rungs and read by
   nothing, while still costing per-decision observation and rollback snapshot
   space. ⇒ It leads because both answers are cheap and everything about "how
   hard is rung N" is measured against a ladder that currently has four working
   knobs, not five.
2. ⭐ **Finish isolating the `5 vs 3` inversion.** The utility weights are
   established as necessary; four single-knob arms separate which of
   `frame_advantage`, `kill_potential`, `stage_risk`, `expected_payoff` carries
   it. ⇒ This is the only *significant* defect measured in the shipped ladder, so
   it outranks everything speculative.
3. **Answer the roster question** — do the two Robots get kits? Every ladder
   number ever taken was measured between two fighters that had no special
   button, and George resolves matches they cannot. ⇒ Until this is answered the
   rig is measuring fighters nobody intends to ship as final.
4. **The `t3` placement race** (question 50). Narrowed to *"may a match present a
   tick in which the followed body has not been placed?"* — now a correctness
   question rather than a feel judgement, with a one-line reproduction.
5. ⬇ **`Dodge` shadow model — DEMOTED from #2, and the reason is measured.** It
   only matters under the rollout, the shipped ladder disables the rollout on all
   nine rows, and the `Shield` model that was supposed to prove the approach
   landed entirely within spread. ⇒ Do it because `None` is the wrong score for a
   verb a fighter can pick, not because a matrix will move. It reaches no player
   either way.
6. **Re-run flat-vs-platforms** once the respawn/tier collision is ruled on, using
   `closest_approach` on the unfought rows to separate "cannot navigate" from
   "declines to commit".

⚠ **And the standing rule, now with a worked example behind it:** do not tune
rollout depth, heuristics, APM, reaction or weights without a trace naming the
responsible decision. The dodge finding came from `AMBITION_FIGHTER_TRACE=1` and
a per-rung count; every reading of it before that trace — including two of mine —
was wrong about the cause.

## Exit

This plan can close when:

1. the level-6 rollout-caused `recovery_below` regression is diagnosed and fixed
   at the responsible decision seam;
2. the scenario/duel reports give repeatable useful calibration signals across
   the authored ladder;
3. fixed-seed determinism and no-cheat constraints remain covered;
4. remaining fighter behavior work is ordinary product tuning rather than an
   unresolved brain architecture problem.
