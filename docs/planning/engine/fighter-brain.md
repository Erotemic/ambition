# Advanced fighter brain — current evaluation and regression work

**State:** implementation exists; the confirmed level-6 rollout regression is
CLOSED (see below). Current work is evaluation and the separate level-1 case.

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

⚠ **A hypothesis I tested and had to drop**, recorded because it is the obvious
one and the next reader will have it too: that the fixtures handicap the subject
(7 of 9 place SELF offstage or in hitstun — *"Self is past a blastzone"*) and
seat 0 is always the higher rung, so the skew is placement rather than skill. It
is not: the self-handicapped fixtures favour LOWER at **68%** (n=28) and the two
self-advantaged ones at **62%** (n=8). Same direction, same size.

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
geometry and the number have to move together.

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

## Exit

This plan can close when:

1. the level-6 rollout-caused `recovery_below` regression is diagnosed and fixed
   at the responsible decision seam;
2. the scenario/duel reports give repeatable useful calibration signals across
   the authored ladder;
3. fixed-seed determinism and no-cheat constraints remain covered;
4. remaining fighter behavior work is ordinary product tuning rather than an
   unresolved brain architecture problem.
