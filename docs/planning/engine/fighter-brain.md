# Advanced fighter brain — current evaluation and regression work

**State:** implementation exists. The 2026-08-31 level-6 rollout regression is
CLOSED — and ⛔ **a SECOND, distinct level-6 rollout defect is OPEN, measured
2026-09-04**: a fighter running the rollout selects `Dodge` and `Shield` **zero
times in 662 decisions**, because `pick_movement`'s unjudged tier is unreachable.
The defensive vocabulary switches off at the rung the difficulty ladder calls
better, and that single rung is the whole of the ladder's apparent inversion.
Half repaired (`Shield` is modelled); `Dodge` needs real motion in the shadow.

⛔⛔ **READ THIS BEFORE ANY TABLE BELOW: MOST OF THEM MEASURED THE WRONG THING, IN
FIVE SEPARATE WAYS, AND ALL FIVE ARE NOW FIXED.** Every one was the same class —
the rig's configuration differed from the shipped game's, and only the rig was
ever read. In the order they were found on 2026-09-04:

| # | the rig used | the game uses | fixed by |
|---|---|---|---|
| 1 | one weight set on every rung | nine authored rows | dropping the `--weight` override |
| 2 | `FighterBrainProfile::for_level` | `fighter_brain_ladder.ron` | **`--ladder PATH`** |
| 3 | the L3 rollout ON at rung 6+ | rollout disabled on all nine rows | (follows from #2) |
| 4 | two STAND-IN fighters | George is the authored one | `--character`, now printed |
| 5 | a **60-second** bout | a **480-second** match | reading `SMASH_TIME_LIMIT_TICKS` |

⛔ **AND A SIXTH OF A DIFFERENT KIND, found later the same day: `--paired`
cancelled the WRONG TERM.** It swaps the rungs, so a *fighter* comparison at one
rung got a control aimed at something else — and **a control that cancels the
wrong term is worse than no control, because it produces symmetric-looking output
that reads as rigour.** ⇒ Not a wrong configuration; a correct instrument pointed
at the wrong variable. Repaired by swapping the fighters when the rungs are equal,
which immediately produced the George-vs-stand-in result below.

⇒ **#5 is the one that touches everything.** On a 60-second clock no bout could
end, so stocks tied in every cell and **every verdict fell through to the damage
tiebreak** — so any table dated before it says *"dealt more damage in the first
eighth of a match"* wherever it appears to say *"won"*. ⚠ **That includes the
SCENARIO matrices** (the nine fixtures, the Shield-model comparison, flat-vs-
platforms), which have NOT been re-run at the shipped clock and carry the same
caveat.

⛔⛔⛔ **AND THE BIGGEST FINDING ON THIS PAGE IS NOT ABOUT THE LADDER AT ALL —
`D-BRAIN-MENU`.** Measured 2026-09-04 on the shipped fighter and the shipped
ladder: **the brain selects `george_booul_dash_attack` ZERO times and the bodies
start it 43 of 59.** It selects `jab` 40 times; one jab starts. ⇒ Sixteen of
George's twenty-eight authored moves never start once, including **all three
smashes and all three tilts**.

⚠⚠ **AND IT BEARS ON EVERY LADDER NUMBER ON THIS PAGE, so face it here rather
than let a reader find it.** If a running fighter's attack press becomes the dash
attack whatever the brain scored, do the utility weights — which I isolated as the
`5 vs 3` inversion's cause — matter at all?

⇒ **They demonstrably do, and the evidence is the isolation itself.** Changing
`frame_advantage` + `expected_payoff` alone moved `5 vs 3` from significantly
inverted to not, byte-for-byte against a control that changed nothing. ⭐ A knob
that reached nothing could not have done that. And the weights drive the MOVEMENT
options too, which are not press-converted at all.

⚠ **What it does narrow is the CHANNEL.** The ladder's differences are reaching
the fight through movement, through approach, and through whichever attacks
survive the press conversion — **not through the full attack scoring the weights
appear to control.** ⇒ So "the ladder sags at rung 5" stands as measured; "and it
sags because higher rungs pick worse ATTACKS" would be more than the evidence
supports, and I have not claimed it.

⇒ The chain is traced end to end below: the kit is built for the STANDING stance
(`running` is never passed to the builder), the brain emits a BUTTON rather than a
move, and the body re-resolves that press WITH `running` — preferring
`{base}_dash`. ⭐ **So the brain is not preferring the dash attack; it is scoring a
menu it cannot order from.** Filed as a defect in `queue.md`.

⛔⛔ **AND THE OBVIOUS FIX WAS MEASURED AND DOES NOT WORK.** Building the kit with
`running` (so the brain sees what the press will actually produce) closes the
informational gap — the brain does then name the dash attack — and **changes no
behaviour**: 81% → 85% dash attack, still zero tilts, still zero smashes. ⇒ Because
the gap is **temporal**: the kit is built at DECISION time and the press is
buffered and resolves at EMISSION time, by which point the stance may differ. **A
correctly-built kit is still a kit for a stance the body has since left.**

⭐ **The rule that earns:** *a fix that closes the mechanism you diagnosed is not
evidence you diagnosed the cause.* It compiled, it was principled, it closed a
real mismatch, and it would have passed review.

⭐⭐⭐ **AND THE ROOT CAUSE, FOUND ON THE THIRD FRAMING: MOVEMENT AND ATTACK ARE
SCORED INDEPENDENTLY.** `OptionSet` exposes `best_movement()` and `best_attack()`
separately and both are emitted together — and **73 of 81 attack decisions (90%)
are made on a tick where the brain also chose `Approach`.** ⇒ The fighter
approaches and attacks *simultaneously, always*; approaching drives a run; a
neutral press while running is a dash attack.

⛔ **So the tilts and smashes are unreachable IN PRINCIPLE.** Reaching them needs
*stop moving, then commit* — which the brain cannot express, because nothing
scores the PAIR. ⚠ Both earlier fixes die on this: knowing the press will convert
gives the brain no reason to stop running. ⇒ The remedy is a scoring-shape change
(score movement and attack jointly, or let an attack require "hold position"), and
it is a design decision rather than a repair.

⭐ **The definitive ladder table — shipped rows, shipped clock, bouts that
resolve — is the one headed "THE DEFINITIVE RUN".** Prefer it over anything
above it. ⛔ The rest are kept because the contrast between them is the evidence
for the class, not because their numbers stand.

⚠ The ownership question is still with Jon in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md), and
**tuning still waits on it** — but the reason has changed from "we cannot see"
to "we can see one bad rung and its cause".

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


### ⛔ SUPERSEDED — "the 5→6 boundary is rollout-on vs rollout-off"

⚠ **True of the ENGINE FLOOR and of no shipped fighter.** The floor arms the L3
rollout at level 6; the shipped ladder sets `rollout_depth: 0` on all nine rows,
so no player has ever crossed this boundary. ⇒ The finding below is real about the
instrument and is not a fact about the game. Kept because the arms in it are what
established the rollout's behaviour.

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

### ⛔ SUPERSEDED — the paired 36-cell matrix ("one rung is broken and the rest are fine")

⚠ **Its conclusion named the WRONG rung.** This matrix was taken on the engine
floor at a 60-second clock, and under both the shipped rows and a real match
length the broken rung is `5 vs 3`, not the `6 vs 5` this section identifies. See
**THE DEFINITIVE RUN** below. ⇒ Kept for its method — the pairing, the seat-bias
control and the per-bout work in it are all sound and are what made the later runs
readable.

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

⛔⛔ **RE-RUN AT THE SHIPPED CLOCK, AND THE HEADLINE BELOW IS WRONG. THE TIERS DO
NOT CHANGE THE LETHALITY AT ALL — THEY ROUGHLY DOUBLE THE TIME.**

The prediction was stated before the run (below) and the run confirmed it. Nine
fixtures, `5 vs 3`, shipped ladder, **480-second clock**, 6 seeds paired, the two
stages differing only in the three tiers:

| | flat | platforms |
|---|---:|---:|
| mean survival | **91.5s** | **167.7s** |
| mean stocks LEFT | **0.00** | **0.00** |
| fixtures where every bout resolved | **9 / 9** | **9 / 9** |
| mean damage dealt | 280.8% | 242.5% |

⇒ **Stocks left is `0.00` on BOTH stages** — every bout on every fixture ends in
mutual elimination. ⭐ The `1.64` vs `3.78` gap in the 60-second table below is
**entirely** an artifact of the buzzer: platforms were not preventing kills, they
were postponing them past the cut-off.

⇒ **What the tiers actually do is take 1.83× as long to reach the same end**
(91.5s → 167.7s). ⚠ Which is a real and interesting effect — it is most of a
minute of extra fight — but it is a statement about PACE, not about lethality,
and the two want different design responses.

⚠ **"Unfought bouts" PROBABLY goes with it, and I am marking that as unproven
rather than asserting it.** The tempting deduction is: every bout resolved, so
every bout was fought. ⛔ **It does not hold.** This demo's own header says
*"combat damage does not kill a stocks fighter; leaving the stage emits the
knockout signal"* — so a fighter can lose all three stocks by self-destructing,
with nobody landing a hit. A resolved bout is not necessarily a fought one.

✔✔ **SETTLED BY MEASUREMENT — `--per-bout` on the platform stage at the shipped
clock: 109 bouts, and in ZERO of them did neither seat deal damage.** (One bout
was one-sided.) ⇒ So the 41-of-540 "unfought" figure below IS approach time, not
refusal to engage — every platform bout, given a real match length, becomes a
fight.

⭐ **Worth keeping is that the deduction I nearly published was invalid and the
measurement was one command.** "Every bout resolved, so every bout was fought"
fails on this demo's own rule — combat damage does not kill a stocks fighter,
leaving the stage does, so all three stocks can be lost to self-destructs with
nobody landing a hit. ⇒ The aggregate (242.5% mean damage dealt) made the
conclusion likely and could never make it certain, because **an average cannot
rule out a handful of members hiding under it.**

⭐ **The method note is the transferable part.** Both metrics in the original table
— *stocks LEFT* and *unfought bouts* — are "how much has happened by the buzzer"
quantities. ⇒ **A cut-off metric cannot distinguish "less happens" from "the same
happens later", and every such metric will read a slowdown as a reduction.** The
fix is not a better statistic; it is a clock long enough for the thing being
counted to finish.

⇒ Everything below this line is the superseded 60-second measurement, kept because
the contrast is the evidence.

⛔⛔ **THE PREDICTION AS IT WAS WRITTEN BEFORE THE RE-RUN — it is preserved
unedited because a prediction is only worth something if it cannot be adjusted
afterwards.** Every number above was taken at **60 seconds**, and the shipped
match is **480**. ⚠ The two headline metrics are exactly the ones a short clock
distorts: *stocks LEFT* and *unfought bouts* both measure **how much has happened
by the buzzer**, not what the stage does to a fight.

⇒ **The alternative reading the current data cannot exclude: the tiers do not
halve the lethality, they SLOW THE APPROACH.** If platforms make fighters take
longer to close, then at 60 seconds they will show more stocks left and more
unfought bouts *even if a full match ends identically*. ⭐ Those two hypotheses
predict the same 60-second table and different 480-second ones, which is what the
re-run tests.

⇒ Both stages are running again at the shipped clock. Until then, read the table
below as **"what the tiers do to the first eighth of a match"**, which is a real
thing to know and is not what its heading claims.

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
number in this document is substantially a fact about the STAND-INS**, who have
no special button. ⚠ **This sentence also said *"no forward tilt and no dash
attack"* and that is WITHDRAWN** — those were read off unbound verbs, and the
directional chain answers an unbound `attack_forward` with the `jab`. Enumerated
by press rather than by binding, the stand-ins' whole surplus over George is
**eight `special` presses and nothing else**; see item 4 of the instrument list
below. ⇒ The point of this paragraph is unchanged and slightly sharpened: the
stand-ins are George's genre shape **with one button removed**, which is enough
to explain a slower fighter without inventing two more missing moves. ⚠ Not
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

⛔⛔ **THESE FOUR BOLD CELLS ARE THE ONES MOST EXPOSED BY THE INSTRUMENT DEFECT
BELOW, because their meaning is carried by an ABSENCE.** Every cell reading
*"within spread"* states its qualifier out loud; the two bold ones mean what they
mean **only because the qualifier is missing** — and the missing qualifier comes
from a sign test that discards direction, while the words *higher* / *LOWER* come
from a pooled median. ⇒ **A cell whose two authorities disagree prints exactly
like a cell where they agree**, and there is no residue in the table to tell them
apart.

⚠ **Read the two bold cells as UNCONFIRMED until the cells are RE-RUN (the rig itself was fixed 2026-09-04, `36dd9a248`; nothing has been re-measured through it)**, not
as withdrawn: the defect makes them unverified, not wrong, and the mechanism
evidence for `5 vs 3` (stocks level at `2 : 2`, the lower rung dealing 215% against
191%, and the byte-for-byte weight isolation) is independent of the qualifier and
still stands. ⇒ It is the word *significant* that is on hold, not the inversion.


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

1. `read_weight` is authored on all nine shipped rungs, **0.0 rising to 1.0** (`0.0 0.0 0.0 0.1 0.2 0.3 0.5 0.7 1.0`) — it
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
(`crates/ambition_characters/src/snapshot_impls.rs:451`, with its own comment explaining why a rewind must
restore it). ⇒ The shipped game pays per-tick observation and per-snapshot
bandwidth to maintain an opponent model that nothing ever reads.

⚠ **Sized, because "not free" invites an exaggeration.** The state is a `BTreeMap`
over 5 situations × 6 choices = **30 rows at most**, at `u8 + u8 + f32` each plus
a count — **≤184 bytes per fighter per snapshot**. ⇒ That is not a performance
problem and nobody should present it as one. The case for wiring it up or
deleting it is that a five-knob ladder with one silent knob cannot be reasoned
about, and every rung was tuned by someone who believed all five worked.

⭐⭐ **AND THE TEST THAT SEPARATES A DEAD KNOB FROM A RESTRAINED ONE IS WHETHER
ANYONE AUTHORED VALUES FOR IT.** Both look identical to a grep — *published,
nothing reads it* — and they want opposite responses:

| | dead | restrained |
|---|---|---|
| example | `read_weight` | a resource with one producer and no reader, by design |
| the tell | **nine authored values, 0.0 rising to 1.0** | nobody has spent effort on it |

⚠ **That top value was recorded as 0.9 until 2026-09-04 and it is 1.0** — a small
error, kept visible because of HOW it survived. The row was written from the
shape of the ramp rather than from the file, and re-checking it turned up a
second thing: `grep -c read_weight` on the shipped ladder returns **10** against
**nine** rungs. ⇒ The tenth is a COMMENT — line 42, *"Its skill is the read
(`read_weight: 1.0`), not the reflex"* — which is a prose sentence that names the
value it describes, so a count of the field silently includes a mention of it.
⭐ The authored nine, read off the file: `0.0 0.0 0.0 0.1 0.2 0.3 0.5 0.7 1.0`,
with `rollout_depth: 0` and `rollout_k: 0` on **every** rung, which is the half
this argument actually rests on and which is confirmed.
| what it means | somebody tuned a knob believing it worked | the seam exists ahead of its customer |
| what to do | wire it up or delete it | write down WHY and leave it |

⇒ **Effort spent on a thing is the evidence that somebody believed it worked.** A
`.ron` with nine hand-picked values is a claim about behaviour; an unread resource
with one producer is a claim about nothing. ⚠ Which is why "unused" is not by
itself a defect report — and why this one is.

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

### ⭐⭐ IT GENERALISES — the same two weights carry BOTH inverted cells

A decomposition on one cell is a coincidence until it predicts a second one. So:
rung **6** given rung **5**'s `frame_advantage` + `expected_payoff` (0.60 → 0.50,
0.40 → 0.30), 40 seeds, against a matched control.

| arm | cell | dealt (hi : lo) | verdict |
|---|---|---|---|
| control (shipped) | `6 vs 5` | 197% : 214% | ⛔ **LOWER outfights** |
| rung 6 given rung 5's two weights | `6 vs 5` | 204% : 211% | ✔ LOWER outfights **(within spread)** |

⇒ **Stepping those two knobs back one rung removes the significance of the `6 vs 5`
inversion**, exactly as stepping them back two rungs removed the `5 vs 3` one.

⭐ **And the effect SCALES with the step, which is the prediction I would not have
been able to fake.** At rung 5 the revert was a two-rung step and the verdict
flipped direction outright; at rung 6 it was a one-rung step and the verdict only
lost significance. ⇒ Same knobs, proportional response, at two different cells.

### ⭐ THE LADDER'S ENDS ARE CORRECTLY ORDERED — only its middle sags

`9 vs 1`, 40 seeds, shipped ladder: **`256% : 177%`, higher outfights, no
qualifier.**

⇒ Put together with the adjacent-pair matrix, the shipped ladder is:

| comparison | result |
|---|---|
| `9 vs 1` | ✔ higher (significant) |
| `3 vs 1` | ✔ higher (significant) |
| `5 vs 3` | ⛔ **LOWER** (significant) |
| `6 vs 5` | ⛔ **LOWER** (significant) |
| `9 vs 6` | ✔ higher (significant) |

⭐⭐ **So the ladder is not broken — it SAGS.** Its endpoints are ordered and
distinguishable; rungs 5 and 6 are a trough between two strong rungs. ⇒ That is a
much better diagnosis than "the ladder is inverted", and a much smaller fix: the
progression is right at the ends and wrong in the middle, on two named knobs whose
mechanism is understood.

⚠ **Still bounded by the same confound** — all five rows are decided on damage,
because stocks tie in every one. A sag in damage-per-minute is not yet a sag in
difficulty. That is the open question in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md), and a
180-second arm is running against it now.

### ⭐⭐ CANDIDATE FIXES, MEASURED — and halving the rise does nothing at all

Three settings for rung 5's `frame_advantage` / `expected_payoff` at the shipped
clock, 16 seeds paired, everything else untouched:

| rung 5's pair | dealt (5 : 3) | verdict |
|---|---|---|
| **0.50 / 0.30** *(shipped)* | 306% : 360% | ⛔ **LOWER outfights** |
| 0.40 / 0.20 *(halve the rise)* | 300% : 362% | ⛔ **LOWER outfights** |
| **0.30 / 0.10** *(rung 3's values)* | **336% : 313%** | ✔ **higher outfights** *(within spread)* |

⇒ **The middle row is the informative one.** Halving the increase leaves the cell
as inverted as the shipped value — `300% : 362%` against `306% : 360%`, which is
no movement at all. ⭐ So this is not a magnitude the ladder overshot and could
dial back; **any** rise in the pair between rungs 3 and 5 appears to cost more
than the reflex improvement buys.

⚠ **Which does not mean "set them flat".** The winning row lands *within spread*,
so it says the inversion is gone, not that rung 5 is now correctly stronger. And
nothing here tests rungs 6–9, which carry the same rise and further.

⭐ **What it does establish is that rung 5 is not flattened by the fix.** With the
pair held at rung 3's values, rung 5 still differs on reaction (300ms vs 400), APM
cap (200 vs 120), execution noise (0.20 vs 0.30), `kill_potential` and
`stage_risk` — and it wins. ⇒ The reflex ladder works; it was being cancelled by
the weight ladder.

### ⭐⭐⭐ AND FLATTENING THE PAIR ACROSS THE WHOLE LADDER FIXES ITS ORDERING

The obvious completion of the rung-5 arm: hold `frame_advantage` at `0.30` and
`expected_payoff` at `0.10` on **every row from level 4 up** — six rows — and
leave everything else alone (`kill_potential` and `stage_risk` still rise, and so
do all the reflex knobs). Shipped clock, 12 seeds paired:

| cell | SHIPPED | PAIR HELD FLAT |
|---|---|---|
| 3 vs 1 | ✔ higher | ✔ higher *(299% : 208%)* |
| 5 vs 3 | ⛔ **LOWER outfights** | ✔ higher *(329% : 313%, within spread)* |
| 6 vs 5 | LOWER *(within spread)* | LOWER *(within spread)* |
| 9 vs 6 | higher *(within spread)* | ✔ **higher outfights** *(448% : 379%)* |

⇒ **No cell is significantly inverted any more.** The one established defect is
gone, and `9 vs 6` — undetermined on the shipped rows — becomes significant in the
CORRECT direction. ⭐ Two cells improve and none regresses.

⭐ **And the survival medians rise further and more cleanly**: 85s → 101s → 116s →
122s, against the shipped ladder's 85s → 98s → 114s → 113s, where the top pair
went backwards. Higher rungs take longer to kill each other, monotonically.

⚠ **This is a MEASUREMENT, not a recommendation I am authorised to make.** It says
what the ladder does under one alternative; it does not say the alternative is
the right design. ⛔ Holding those two flat means higher rungs no longer weight
frame safety or move power more heavily than rung 3 does — that is a statement
about what a harder CPU IS, and it belongs to whoever owns the ladder. The entry
in [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md)
carries it as one option among several, with this table attached.

⚠ **Bounded, too**: 12 seeds; `6 vs 5` stays undetermined and still leans LOWER;
and this tests one alternative, not the space. A gentler-but-nonzero rise was
also measured at rung 5 and did nothing — see the candidate table above — so
"less rise" is not a third option that was skipped.

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

### ⭐⭐⭐ THE DEFINITIVE RUN: the shipped ladder, at the shipped clock, with matches that finish

**Everything below this line supersedes the 60-second tables above.** Same rig,
same sign test, 12 seeds paired — but the clock is now
`ambition_demo_smash::SMASH_TIME_LIMIT_TICKS`, the **shipped eight minutes**, so
every bout runs to a conclusion instead of being cut off at 12.5%.

| cell | survived (hi : lo) | stocks LEFT | dealt (hi : lo) | verdict |
|---|---|---|---|---|
| 3 vs 1 | 85.1s : 80.6s | 0 : 0 | 299% : 208% | ✔ **higher outfights** |
| 5 vs 3 | 97.3s : 98.7s | 0 : 0 | 300% : 360% | ⛔ **LOWER outfights** |
| 6 vs 5 | 103.6s : 104.7s | 0 : 0 | 346% : 361% | LOWER *(within spread)* |
| 9 vs 6 | 112.0s : 116.5s | 0 : 0 | 407% : 390% | higher *(within spread)* |

⇒ **Every bout now RESOLVES** — `0 : 0` stocks throughout, both fighters fully
eliminated, at medians rising cleanly from 85s to 112s as the rungs climb. ⭐ That
rise is itself a sanity check on the ladder: higher rungs take longer to kill each
other, which is what stronger fighters should do.

⭐⭐ **AND THE `5 vs 3` INVERSION SURVIVES THE SHIPPED CLOCK.** Still *LOWER
outfights*, still significant, with matches finishing. ⇒ **This is the result the
whole day was trying to reach, and it is the one that cannot be explained away by
the instrument.**

⭐⭐ **The patience defence is dead, and survival time is what kills it.** The
worry was that a damage-rate verdict cannot see a fighter who wins by refusing bad
commitments. But on a clock long enough to resolve, **rung 5 survives 97.3s
against rung 3's 98.7s** — it does not live longer, and it deals 300% against
360%. ⇒ Two independent signals, both against rung 5. A patient-but-stronger rung
5 would have to beat rung 3 on at least one of them and it beats it on neither.

### ✔ RE-RUN AT THE SHIPPED CLOCK — the full scenario matrix, and it confirms

The nine fixtures × four rung cells, **shipped ladder, shipped 480s clock**,
paired, 8 seeds. This replaces the 60-second scenario runs everywhere on this page:

| cell | fixtures favouring the LOWER rung | individually significant |
|---|---|---|
| `3 vs 1` | **1 / 9** | 1 (`juggle_escape`, *higher*) |
| `5 vs 3` | ⛔ **9 / 9** | 1 (`ledge_trap`, *LOWER*) |
| `6 vs 5` | 6 / 9 | 0 |
| `9 vs 6` | 4 / 9 | 0 |

⇒ **Every rung-matrix conclusion survives the clock fix, across every situation:**
`3 vs 1` orders correctly in 8 of 9 fixtures, `5 vs 3` is **unanimously inverted**,
and `6 vs 5` and `9 vs 6` are unresolved — 6/9 and 4/9 is what no signal looks
like. ⭐ `9 vs 6` splitting almost exactly in half is the cleanest statement of
"undetermined" this page has.

⚠ **This is a confirmatory result and it was predicted before the run.** I said so
in advance and am reporting it at the same volume I would have reported a
reversal, because a result you have pre-committed to reporting cannot be quietly
dropped for being boring. ⇒ What it buys is not novelty: the 60-second scenario
tables are no longer the only evidence for the page's central finding, and the one
worry that mattered — *a null taken through an instrument that could not resolve
its bouts* — no longer applies to any of these cells.

⚠ **What it does NOT rescue is the Shield comparison**, which is a different arm
and is withdrawn above for a reason a longer clock cannot fix: the shipped ladder
disables the rollout the Shield model serves, so there is nothing to measure on
any clock.

### ⭐⭐ AND IT IS NOT SITUATIONAL: nine of nine scenario fixtures agree

The nine scenario fixtures, `5 vs 3`, at the shipped clock, 8 seeds paired:

| fixture | verdict |
|---|---|
| `ledge_trap` | ⛔ **LOWER outfights** (significant) |
| `juggle_escape` | LOWER *(within spread)* |
| `projectile_camper` | LOWER *(within spread)* |
| `edgeguard_window` | LOWER *(within spread)* |
| `edgeguard_ledge_hang` | LOWER *(within spread)* |
| `recovery_left` / `right` / `below` / `above` | LOWER *(within spread)* ×4 |

⇒ **Nine of nine point the same way.** One is individually significant; the other
eight are underpowered at 8 seeds, which is expected — a cell needs near-unanimous
pairs to clear p < 0.05 at that n.

⭐ **The direction is the result, not the individual verdicts.** The rung-5 sag
shows up in a ledge trap, a juggle escape, a projectile camper, two edgeguard
setups and all four recovery angles. ⇒ It is **not** an artifact of one fixture's
geometry, and that matches the mechanism exactly: a fighter that weights frame
safety more heavily is reluctant to commit *everywhere*, not in one situation.

⚠⚠ **DO NOT READ 9-OF-9 AS p = 0.004.** The naive sign test across fixtures gives
`2 × 0.5^9`, and it does not apply: the nine share the same two fighters, the same
ladder rows and the same weight vector, so they are nine views of one system
rather than nine independent trials. ⇒ The defensible claim is
**consistency of direction across structurally different situations**, which is an
argument that the cause is general. It is not a ninefold multiplication of
confidence, and treating it as one would be the same error as the subgroup split
recorded in the instrument note.

### ⭐⭐ REPLICATED AT 28 SEEDS — more than double the evidence, identical picture

The same four cells, same shipped clock, **28 seeds** instead of 12:

| cell | survived (hi : lo) | dealt (hi : lo) | verdict at 28 | verdict at 12 |
|---|---|---|---|---|
| 3 vs 1 | 88.4s : 85.3s | 304% : 218% | ✔ higher | ✔ higher |
| 5 vs 3 | 98.4s : 98.7s | 306% : 360% | ⛔ **LOWER** | ⛔ **LOWER** |
| 6 vs 5 | 114.2s : 114.3s | 373% : 382% | *(within spread)* | *(within spread)* |
| 9 vs 6 | 112.9s : 116.5s | 392% : 382% | *(within spread)* | *(within spread)* |

⇒ **Nothing moved.** Every verdict and every qualifier is the same. ⭐ That is what
a replication is for, and it is worth more than the extra precision: the two
significant cells are significant at both sample sizes, and the two undetermined
ones stay undetermined when the evidence more than doubles.

⭐ **So `6 vs 5` is not a second inversion — it is noise, and now with the power to
say so.** At 60 seconds it looked significant; at the shipped clock it does not,
at 12 seeds or at 28. ⇒ **The shipped ladder has exactly ONE bad rung.**

⚠ **The `5 vs 3` inversion is now the most heavily replicated result on this page**
— significant at 12, 24 and 40 seeds on the short clock and at 12 and 28 on the
shipped one, with survival time agreeing at both clocks. It is not going away.

⚠ **What the full clock TOOK AWAY, and it must be said as loudly.** `6 vs 5` and
`9 vs 6` are now **within spread** at 12 seeds — the 40-second-clock claim that
"every cell is significant" does NOT survive. ⇒ So the honest final statement is
narrower than the one I wrote three hours earlier: **one inversion is established
(`5 vs 3`), the `6 vs 5` sag is suggested but unproven, and the ladder's ends are
ordered.** ⛔ Anyone quoting "the ladder goes backwards through its middle" is
quoting a 60-second table that this one replaces.

### ⛔ SUPERSEDED — the 40-seed 60-second table (kept for the instrument lesson)

⚠ **Read the section above instead.** This table is retained because the
comparison between it and the definitive run is the clearest possible statement of
what a wrong clock does: it manufactured two significant results that a real match
length does not support.

### ⛔⛔ AT 40 SEEDS EVERY CELL IS SIGNIFICANT, AND THE SHIPPED LADDER GOES BACKWARDS THROUGH ITS MIDDLE

The shipped ladder, 40 seeds, paired, sign test — **no cell carries a qualifier**:

| cell | dealt (hi : lo) | verdict |
|---|---|---|
| 3 vs 1 | 215% : 163% | ✔ **higher outfights** |
| 5 vs 3 | 191% : 229% | ⛔ **LOWER outfights** |
| 6 vs 5 | 197% : 214% | ⛔ **LOWER outfights** |
| 9 vs 6 | 231% : 220% | ✔ **higher outfights** |

⇒ **Read as an ordering: `1 < 3 > 5 > 6 < 9`.**

⛔⛔⛔ **AND THE WHOLE TABLE ABOVE MEASURED THE FIRST 12.5% OF A MATCH.** Found
after it was written, by asking what the shipped match clock is:
`apply_smash_match_rules` sets `time_limit_ticks = 8 * 60 * 60` — **eight
minutes**. `ladder_rig`'s clock was **sixty seconds**.

⇒ **That is why every cell has tied stocks.** A 60-second bout cannot end, so the
verdict — *stocks taken, then damage dealt* — falls through to damage in every row
of every ladder table this tool has ever produced. ⚠ Every "rung N is weaker"
result on this page means *"deals less damage in the opening eighth of a match"*.

⭐ **Measured, not inferred:** at 180 seconds the same `5 vs 3` cell RESOLVES —
both fighters eliminated at a median of **~98s**, `0 : 0` stocks, 300% : 360%
damage. So a match takes about 98 seconds and the instrument was stopping it at
60. ⇒ The rig's default now reads `ambition_demo_smash::SMASH_TIME_LIMIT_TICKS`,
and a shortened run prints what fraction of a match it covers.

⭐⭐ **BUT THE INVERSION SURVIVES THE LONGER CLOCK.** At 180s, with matches
actually resolving, `5 vs 3` is **still LOWER outfights and still significant**.
⇒ And the patience reading predicted rung 5 would at least survive longer; it does
not — eliminated at 97.3s against rung 3's 98.6s. **On a clock long enough to
finish, rung 5 neither deals more damage nor lives longer than rung 3.** ⚠ `6 vs 5`
at 180s falls within spread, so only one of the two sagging cells survives.

⇒ **This is the fifth instance today of the class named below**, and the largest:
weights, ladder source, rollout, fighters, and now the clock. Arms at the full
shipped 480s clock are running.

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

⛔⛔ **THIS PARAGRAPH EXPLAINED A DISCREPANCY WITH A FACT THAT IS NOT TRUE, and
the correction is bigger than the paragraph — read it before you use any
`(within spread)` label on this page.**

It said A's `9 vs 6` stocks `2 : 3` under *higher outfights* was fine because
*"the verdict is computed per bout"*. ⛔ **It is not.** `report_row` computes the
verdict from **pooled medians** — a median of stocks taken, then a median of
damage dealt as the tiebreak. Nothing in it is per bout. So the reassurance was
invented to dissolve a discrepancy rather than derived, which is the worst way for
a wrong sentence to enter a document: it makes a reader stop looking.

⛔ **AND LOOKING FINDS A REAL DEFECT IN THE INSTRUMENT — one row, two authors.**
Raised by a review through the sibling session 2026-09-04 and re-derived here from
source:

- the **displayed verdict** comes from pooled medians, stocks first, damage on a
  tie (`ladder_rig.rs`, `report_row`);
- the **`(within spread)` qualifier** comes from a *paired, damage-only* exact
  sign test, and `sign_test_says_within_spread` returns `p >= 0.05` — **`k =
  positives.max(negatives)` discards WHICH direction won.**

⇒ **So a row can print `LOWER outfights` with no qualifier while its own sign test
is significant for HIGHER.** The reviewer's fixture is legal and reproduces it: 16
pairs favouring higher by +100 and 4 favouring lower by −1000 give pooled medians
that say LOWER, and signs 16/4 (two-sided p ≈ 0.0118) that remove the qualifier.
The row then contradicts itself in the direction that reads as most confident.
⚠ **Second mismatch:** when STOCKS decide the verdict, the paired test still tests
DAMAGE — so the comment claiming the spread is measured on the *"DECIDING
quantity"* is false exactly when `hi_took != lo_took`. ⓘ And `median()` is
`values[len / 2]`, the upper-middle order statistic, not a median for even N.

⚠⚠ **WHAT THIS DOES AND DOES NOT INVALIDATE.** The defect can only mislead where
the pooled direction and the paired direction DISAGREE; where they agree the label
is what it claims. ⇒ Which cells those are is **unknown until the cells are
re-run** — so until then, treat every verdict on this page as carrying an
unstated *"direction not cross-checked"*, and lean on the cells' mechanisms rather
than their labels. ⛔ **The `5 vs 3` inversion is the one to re-take first**, since

⛔⛔ **AND THE HOLD IS WIDER THAN THE SIGNIFICANCE LABELS, which I under-stated at
first.** `median()` was corrected in the same pass: it returned
`values[len / 2]`, the **upper-middle** order statistic, and **every `--paired` run
has an even sample by construction** — so it was wrong precisely where this page
takes its numbers. ⇒ **Every DESCRIPTIVE column on this page is pre-fix too**, not
just the verdicts. ⚠ The effect is largest on the stock columns, which are small
integers: a 20-bout sample splitting 10 zeroes / 10 ones reported **1** for both
seats where the median is **0.5** — the more flattering half, for everybody.

it is the finding this document is built around.

⭐ **THE RE-RUN, WRITTEN OUT so it is a paste rather than a reconstruction.** Arm A
is the shipped ladder against the two stand-ins, which is the arm both contested
cells came from:

```text
cargo run --release -p ambition_demo_smash_app --bin smash_tool -- ladder-rig \
  --ladder game/ambition_content/assets/data/fighter_brain_ladder.ron \
  --paired --seeds 12
```

⚠ **Take `--seeds 12` first and compare it against the table above before spending
more**, because the twelve-seed row is what the disputed cells were measured at —
a re-run at a different n answers a different question and cannot confirm or
refute them. ⇒ Then `--seeds 28` and `--seeds 40`, which are the replications
already on record.

ⓘ Three things the rig now prints that the original run did not, and each is worth
reading before the table: `report_which_clock_is_in_play`,
`report_which_fighters_are_in_play` and `report_which_ladder_is_in_play`. ⛔ **If
the clock line does not say the shipped eight minutes, stop** — every verdict this
tool printed before 2026-09-04 fell through a damage tiebreak for exactly that
reason.

⚠ **And this costs a full release build of the composed demo app.** On a box with
disk it is minutes; on this one it is the reason the row is still open.

ⓘ The fix is not *"check the sign against the median"* — that keeps both
authorities and adds a referee. It is to compute **one paired outcome per seed**
(reorient straight and mirrored into logical higher/lower, aggregate, stocks
first, damage only on a stock tie → Higher/Even/Lower) and derive **both** the
displayed direction and the sign test from those same outcomes, leaving pooled
medians as descriptive columns only. ✔ **Landed 2026-09-04 (`36dd9a248`)** by the
sibling session, on a box that could compile and poison-verify it.

⭐ **THE POISON ARMS, WRITTEN DOWN SO THEY DO NOT HAVE TO BE RE-DERIVED.** A fix
to this function is only worth landing if every one of these reddens the CURRENT
code and passes the new one:

1. **The reviewer's 20-pair fixture must not print a significant `LOWER`.** 16
   pairs where higher scores `[1000, 0]` against lower `[400, 400]` (a +100
   difference) and 4 where higher scores `[0, 0]` against lower `[1000, 1000]`
   (−1000). ⇒ Pooled medians give higher 0, lower 400 → verdict `LOWER`; paired
   signs are 16/4, two-sided p ≈ 0.0118 → qualifier removed. **The row contradicts
   itself, and this is the arm that proves the defect rather than describes it.**
2. **A row where stocks and damage disagree must follow STOCKS** — currently the
   qualifier tests damage regardless, so the test and the word describe different
   quantities whenever `hi_took != lo_took`.
3. **Mirror orientation invariance**: reorienting straight and mirrored bouts into
   logical higher/lower must not change the verdict. A rig that answers differently
   depending on which seat the fixture put SELF in is measuring the placement.
4. **Ties still dropped**, as the sign test already requires — padding a run with
   tied pairs must not move `p`.
5. ⛔ **MORE EVIDENCE ONE WAY MUST NOT REVERSE SUPPORT.** This is the general form
   of the bug that already shipped here once: the old `|mid| < 0.5 * (hi - lo)`
   criterion got *harder* to pass as seeds were added, because a range only grows.
   ⇒ Any replacement must be checked for the same property deliberately, since
   that one survived review by looking reasonable.

⚠ **And `median()` wants fixing in the same pass** — `values[len / 2]` is the
upper-middle order statistic, so for even N it is not a median and it is biased
upward. Every column on this page is computed with it.

✔✔ **AND THE PROPOSED DESIGN WAS VALIDATED NUMERICALLY BEFORE ANY RUST WAS
WRITTEN — 2026-09-04, because this box cannot compile and a design can still be
wrong on paper.** Both forms of `report_row` were modelled in Python (the current
two-author form and the proposed single-author one) and the arms above were run
through both. ⇒ The point of the exercise is that **the current code FAILS two of
them**, which turns the review's report from a reading into a reproduction:

| arm | current | proposed |
|---|---|---|
| **1** reviewer's 20-pair fixture | prints **`LOWER`, unqualified** | `higher`, signs **16/4** |
| **2** stocks favour higher, damage favours lower | prints **`higher`, unqualified — while its own damage sign test is 0/10 for `LOWER` at p = 0.00195** | `higher`, signs 10/0, from stocks |
| **4** ties padded onto a run | — | verdict and qualifier unchanged ✔ |
| **5** 4 → 20 unanimous pairs | — | `within spread` at 4, significant from 6 up, **never reverses** ✔ |

⭐ **Arm 2 is the sharper of the two failures and it was not obvious from reading.**
The row does not merely test the wrong quantity — it prints an **unqualified**
`higher` whose only statistical support ran, significantly, the other way. A reader
sees the most confident thing this tool can print.

ⓘ Arm 5 also confirms the replacement has the property the old range criterion
inverted: support strengthens with evidence, and four pairs correctly cannot reach
significance however unanimous (2 × 0.5⁴ = 0.125). ⓘ And `median([1,2,3,4])`
returns **3** where the median is **2.5**, as expected for `values[len / 2]`.

⚠ **This validates the ALGORITHM, not an implementation.** The Rust still has to be

✔✔ **LANDED 2026-09-04 — `36dd9a248`, by the sibling session, compiled and
poison-verified on a box that builds.** My branch was written in parallel and is
deleted; the two derivations were independent and **agree on every ordering
decision** (means where they sum, `i32` signs where they name a `PairedOutcome`
enum — the same reduction), which is worth more than either alone.

⛔⛔ **AND RUNNING IT FOUND WHAT NEITHER DESIGN CAUGHT: FIVE GREEN TESTS COULD NOT
SEE THE DEFECT.** With the paired functions written and all their arms passing,
re-wiring `report_row` back to the broken shape — word from the pool, qualifier
from the pairs — left **every test still passing**. ⇒ Each one called the paired
function *directly*, and that function was never the broken part: **the bug lived
in which authority the row consulted.** ⭐ **A test that constructs its subject
cannot witness that subject being bypassed.** The fix was to extract the row's own
decision (`row_verdict(bouts, properly_paired)`) and assert on it with the 20-pair
fixture — precisely because that is where the two authorities disagree — after
which the same poison reddens one test with the defect's signature:
`left: ("LOWER outfights", false)` against `right: ("higher outfights", false)`.

⛔ **AND MY "POISON ASK 3 HOLDS BY CONSTRUCTION" WAS WRONG — the guard had never
run.** `mirroring_a_bout_swaps_every_per_seat_reading` carried its doc comment and
`#[test]`, then a *second* doc comment and `#[test]` immediately after; both bound
to the following function, and the mirror check became a private `fn` nothing
called. ⇒ That is the guard proving index 0 means the higher rung in **both**
halves of a pair — the assumption `paired_outcomes` rests on entirely. **It held by
luck, not by construction**, and I told the sibling session otherwise. Restored and
passing.

⚠ **The same shape was in two of my own files and is now fixed** (`7bb880ff3`):
inserting a test between an existing test's docs and its body steals the old
`#[test]`, leaving dead code wearing a test's name. `the_side_special_is_a_command_grab_and_not_the_standing_grab_renamed`
had been dead for a day — the guard for the exact `lunge_grab` claim published
above it.

ⓘ Also landed with it: `median()`, and `sign_test_says_within_spread` moved to
`#[cfg(test)]` because production no longer converts differences to signs at all.
⇒ **Re-running the ladder cells is still outstanding** and still needs a disk; the
instrument is trustworthy now, and none of the numbers on this page have been

✔ **REVIEWED BY RE-DERIVATION 2026-09-04, not by reading the diff through**, since
this is the instrument every number on this page came from:

- `paired_outcomes` reduces each pair **stocks first, damage only on a stock
  tie** — the ordering the verdict uses. It SUMS the pair where my parallel
  version took the mean; for a comparison those are the same reduction.
- `paired_verdict` takes the direction from `higher.cmp(&lower)` and the
  qualifier from the **same** split. ⇒ One authority, and the two literally
  cannot disagree because there is nothing left to disagree with.
- ⭐ **`k = positives.max(negatives)` is still there and is now CORRECT.** That
  `max` was only ever the bug because a *second* authority supplied the
  direction; with the split feeding both, discarding sign inside the tail
  calculation is exactly right — a two-sided test does not care which way.
- Edge case checked by hand: all-ties gives `n = 0`, `k = 0`, tail `1.0`,
  `p = 1.0` ⇒ `even (within spread)`. Correct — no evidence either way.

ⓘ Two cosmetic notes, neither a defect: `hi_dealt_all` / `lo_dealt_all` are built
unconditionally though only the unpaired branch reads them, and a corrected
`median` now makes `stocks_taken` fractional on even runs — which is fine, since
those medians are descriptive on a paired row and no longer author anything.
re-taken.

✔✔ **THE ARMS' EXPECTED VALUES WERE CHECKED NUMERICALLY RATHER THAN REASONED**,
before either implementation existed. Every fixture was run
through the Python model of the same arithmetic and reproduces exactly what the
Rust asserts: arm 1 signs **16/4** → `higher outfights`, unqualified; arm 2 all
`+1` from stocks; arm 3 all `−1` from damage on a stock tie; arm 4 six tied pairs
changing nothing; arm 5 clearing the qualifier at **exactly 6** unanimous pairs
(five are 2 × 0.5⁵ = 0.0625 and must not clear 0.05) and never reversing to 20;
arm 6 invariant under exchanging a pair's halves.

⭐ **Which makes the remaining risk NAMEABLE, and that is the point of saying it.**
The logic and the expected values are verified; what is unverified is whether the
Rust compiles and typechecks. ⇒ **If an arm fails on a machine that can build it,
the fixture is not the suspect** — read it as a type or syntax problem, or as the
implementation diverging from the model, not as the arm being wrong.

ⓘ `report_row`'s derivation was extracted into `paired_direction_and_spread` to
make any of this reachable. ⚠ **That the defect required printed output to observe
is not incidental — it is why it survived**: a row's two halves had no seam a test
could get between.
written and these arms still have to be run against it — a model agreeing with a
design says nothing about whether the code matches the model.


### ⛔⛔ WITHDRAWN — the Shield comparison's null is not a result, and re-taking it is not worth doing

**Read this before the section below.** That comparison concluded *"no measurable
effect"* from 14 cells that were all `within spread`. Two later findings dissolve
it:

1. **The instrument could not resolve its bouts.** Every cell was taken at the
   60-second clock against a shipped match of 480, so no bout ended, stocks tied
   everywhere, and every verdict fell through to the damage tiebreak. ⇒ **A null
   from an instrument that cannot resolve its subject is not a weak result — it is
   not a result.** "Confirmation" and "this could never have shown otherwise" are
   indistinguishable in that data.
2. **And re-taking it on the shipped configuration would measure nothing at all.**
   The Shield model exists to give `refine_by_rollout` a score for a verb; the
   shipped ladder sets `rollout_depth: 0` on all nine rows, so the rollout never
   runs. ⇒ Same chain that makes `read_weight` inert makes this model inert.

⇒ **So the honest position is: the effect is UNMEASURED, and measuring it requires
the engine FLOOR (rollout on) at the shipped clock — an arm about a configuration
no player meets.** ⚠ That is why I am not running it. Not "we checked and it is
fine": *we cannot check on anything that ships, and the check that is possible is
about the rig.*

⭐ **The mechanical argument is untouched and is the whole justification now.**
`MovementVerb::Shield => ShadowIntent::Hold` replaces a `None` — a verb the
fighter can actually pick, scored as literally nothing. That is a wrong number
rather than a missing one, and it needs no matrix. ⇒ Keep it for that reason and
for no other; the section below should be read as *what the old instrument said*,
not as evidence.

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

### ⛔⛔ THE SHIPPED FIGHTER THROWS ONE MOVE 81% OF THE TIME, AND NEVER A SMASH OR A TILT

A move census, not a damage number — `smash_tool capture-probe`, George vs
George, 120 seconds, run twice: once on the engine floor and once with
`--ladder` pointing at the shipped rows.

| | floor | **shipped ladder** |
|---|---:|---:|
| `george_booul_dash_attack` starts | 132 / ~158 (**84%**) | 98 / ~121 (**81%**) |
| distinct moves used | 12 of 28 authored | 12 of 28 |

ⓘ **The `~` on those denominators is doing real work and the percentages should be
read as UPPER BOUNDS.** `capture-probe` computes no shares at all — it prints move
start COUNTS, and the 84% / 81% are hand arithmetic over them (132/158 = 0.835,
98/121 = 0.810; both check). ⇒ The denominator is a sum of the starts that were
listed, so any start not in the listing makes the true denominator LARGER and the
share SMALLER. **An under-counted denominator inflates a percentage**, so these
cannot be too low and can be too high. ⚠ Nothing in the argument turns on the
second digit — *"the dash attack is most of what this fighter throws"* survives any
plausible correction — but a two-figure share sitting on a `~` reads more precise
than it is, and that is the same overstatement in miniature that the significance
labels above are on hold for.

⛔ **Sixteen of George's twenty-eight authored moves never started once**, and the
list is not a tail of oddities: **all three smash attacks** (`smash_forward`,
`smash_up`, `smash_down`), **all three tilts** (`tilt_forward`, `tilt_up`,
`tilt_down`), three of five aerials, three of four throws, the standing grab and
the taunt.

⇒ **And it is NOT a floor artifact.** The shipped ladder gives the same picture —
81% against 84% — so this is the fighter a player meets, not an instrument
configuration. ⭐ That is the first finding on this page that survives the
floor/shipped distinction without needing it.

⚠ **THE OBVIOUS CAUSE IS REFUTED.** My first hypothesis was the `frame_advantage`
mechanism: high weight on a signed feature → prefer the fastest move. **The dash
attack's startup is 0.15s; the jab's is 0.05 and the tilts are 0.06–0.07.** It is
not the fastest thing George owns, so "the scorer prefers speed" does not explain
it.

⛔⛔⛔ **AND THE TRACE ANSWERS IT: THE BODY PERFORMS A MOVE THE BRAIN NEVER
SELECTS.** `AMBITION_FIGHTER_TRACE=1` over the same probe, shipped ladder, 40s:

| | brain CHOSE (trace `attack=`) | body PERFORMED (census) |
|---|---:|---:|
| `george_booul_dash_attack` | **0** | **43** |
| `jab` | **40** | 1 |
| `tilt_up` | 8 | 0 |
| `modus_ponens` | 19 | 4 |
| `bivalence` | 6 | 3 |
| `tilt_down` | 1 | 0 |

⇒ **The brain never once named the dash attack, and it is 43 of the 59 moves the
bodies started.** ⭐ This is the failure `options.rs` records having made twice, in
its own words — *"the brain named one maneuver, the model judged a second, the
body performed a third"* — happening **in the shipped fighter on the shipped
ladder**, and it explains the whole census: the smashes and tilts the brain picks
never reach the body.

⚠ **Two measurement caveats, both real and neither of which dissolves it.**
(1) The trace covers **seat1 only** (516 lines, one seat) while the census counts
**both** — so the two columns are not the same population. (2) A trace line is a
per-tick DECISION and a census entry is a move START, so 40 `jab` decisions may be
far fewer than 40 attempted jabs. ⇒ **Neither explains zero-chosen against
43-performed.** Re-deciding cannot manufacture a move the brain never names, and a
seat mismatch cannot turn 0 into 43.

⇒ ✔✔ **MECHANISM CLOSED — traced end to end, five links, each read rather than
assumed:**

1. **The kit is built for the STANDING stance.** `update.rs:1747`'s builder calls
   `move_for_directional_verb(verb, direction, **grounded**)` — three arguments.
   There is no `running` in it.
2. ⇒ **So the brain scores `jab`, the tilts and the smashes** — the standing set —
   and never sees `attack_dash` as a candidate at all. That is why the trace shows
   **zero** dash-attack selections: it was never on the menu.
3. **The brain then emits a BUTTON, not a move.** `PendingAttack` carries an
   `AttackBinding { verb, .. }` — a press and a gesture. The move it scored is
   advisory and does not travel.
4. **The body re-resolves that press with `running` in play.**
   `move_for_flat_verb(base, grounded, running)` tries `{base}_dash` **first**
   whenever `grounded && running`.
5. **George binds `attack_dash`.** ⇒ Every attack pressed while running becomes
   `george_booul_dash_attack`, whatever the brain scored.

⛔⛔ **So the brain evaluates a menu it cannot order from.** It is not choosing the
dash attack over the tilts — it is scoring the tilts carefully and then pressing a
button that means "dash attack" because of a stance the scorer never consulted.

⭐⭐ **AND THE REPO HAS ALREADY SOLVED THIS EXACT PROBLEM ONE VERB OVER.** The
burst press had the identical shape — dodge and dash are one input, resolved by
body state — and the fix was to make perception carry the RESOLVED answer:
`SelfView::burst` is a `BurstManeuver`, and its doc says *"`resolve_burst_maneuver`
is the one rule, and this field is its answer. The brain is handed a fact."*
⇒ **The attack press has no equivalent, and it is the same fix**: either build the
kit with the stance the press will actually be resolved in, or hand the brain the
resolved move as a fact.

⚠ **What this does NOT say.** The body is not misbehaving — converting a running
attack into a dash attack is what a dash attack IS. The defect is entirely on the
scoring side: an option set assembled under an assumption the emission then
violates.

⇒ **Named next step rather than a guess.** `move_for_flat_verb(ATTACK, grounded,
running)` resolves a plain attack press to the DASH attack when the body is
running — so an 81% share may be a statement about how often the CPU is *running
when it attacks*, which is an approach-behaviour question and not a scoring one.
⚠ Untested. The arm that separates them is a per-decision trace of the movement
verb chosen immediately before each attack, which `AMBITION_FIGHTER_TRACE=1`
already emits.

⚠ **Bounded**: CPU-vs-CPU, default stage, the roster's default rungs, one 120s
sample per arm. ⇒ The 81/84% split is stable across two configurations, but a
distinct-move count from one sample is a floor on variety, not a ceiling.

### ⭐⭐⭐ GEORGE SIGNIFICANTLY OUTFIGHTS A STAND-IN AT THE SAME RUNG

The measurement the rig could not take until `--paired` learned to swap fighters.
Rung 5 against rung 5, shipped ladder, shipped clock, paired, 12 seeds — the only
thing that differs is the kit:

| arm | dealt | survival | verdict |
|---|---|---|---|
| **George vs Robot** | **318% : 199%** | 62.7s : 58.2s | ⭐ **higher outfights — SIGNIFICANT** |
| Robot vs Robot *(null control)* | 369% : 389% | 134.3s : 131.5s | *(within spread)* — correctly null |

⇒ **The stand-ins are measurably weaker, not merely plainer.** Difficulty rung,
ladder rows, clock, stage and seed set are all held equal. ⭐ And a George match
RESOLVES in ~63 seconds where two Robots take ~134 — **twice as fast to a
conclusion.**

⭐⭐ **The null control is the load-bearing half.** Two mechanically identical
fighters (both Robots receive `fighter_moveset()`) swapped between seats come back
*within spread*, exactly as a null should. ⇒ So the repaired pairing gained power
on a real effect and manufactured none on a null — which is the validation the
change needed, and had that control gone significant the "repair" would have been
a new bias instead.

⛔ **AND THAT RUN IS THE ONE EASIEST TO TALK YOURSELF OUT OF, which is why it is
named here.** A passing null feels like a wasted run: you already believe it, it
costs the same as a real arm, and it produces no headline. ⇒ But *power on the
real effect and none on the null* is the ONLY thing separating a repaired control
from a new bias — without it, "I fixed the control" is a claim about intent.
⭐ **A control that only ever runs against effects you expect cannot tell you it
has started manufacturing them.**

⚠ **Why nobody had this number**: the unpaired form of the same comparison gives a
`329% : 225%` gap and still reports `(within spread)`, because unpaired seed
variance is precisely what pairing removes. ⇒ **The question could be asked and
could not be answered**, which is the defect recorded immediately below.

⇒ It is the strongest input available to the roster question in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md). ⚠ It
does not decide it: sparring partners being weaker may be exactly right. It
removes "the Robots are fine as they are" from the set of *measured* positions.

### ⛔⛔ A SIXTH, AND IT IS A DIFFERENT SHAPE: a control that cancelled the wrong term

The five below are all *the rig read the wrong configuration*. This one is worse
in kind, because the instrument was working exactly as designed.

⇒ **`--paired` swaps the RUNGS between seats.** That is the correct control when
the rungs are what differ. But `--rungs 5,5 --character George --opponent Robot`
is a real question — *is this fighter stronger than that one at one rung* — whose
variable is the **fighter**, and the pairing was swapping something else entirely.

⭐⭐ **A control that cancels the wrong term is worse than no control, because it
produces symmetric-looking output that reads as rigour.** The degenerate arm
returned perfectly equal columns (`304% : 304%`, `142% : 142%`) and a verdict of
`even` — which is precisely what a careful null control is supposed to look like.

⚠ **And the cost is measured.** Run unpaired instead, and the same comparison
gives a **329% : 225%** damage gap that still reports `(within spread)`, because
unpaired seed variance is exactly what pairing exists to remove. ⇒ **The question
could be asked and could not be answered, and nothing in the output said so.**

⭐ Repaired by pairing on the FIGHTER when the rungs are equal and the characters
differ — the same control applied to the actual variable — with the degenerate
warning narrowed to the genuinely variable-free case (same rung *and* same
fighter), where it still fires.

⇒ **The rule it earns is about guards rather than configs:** *a guard's stated
reach is a claim that wants its own poison.* `--paired`'s doc said it "cancels the
seat/placement term", and the widest thing that sentence claims is **any**
comparison. It had only ever been poisoned for the rung case. ⚠ I found this by
tripping it, not by testing it — and the guard that caught me fired twelve times
into an output I was filtering with a `grep -v` that happened to match its text.

### ⭐⭐ THE CLASS BEHIND FIVE SEPARATE FINDINGS: the rig reads its config from anywhere but the shipped file

Five findings in this document have the same shape, and it is worth naming
because a sixth is otherwise inevitable. ⚠ The fourth arrived while this section
was being written and the fifth arrived a few hours later, which is the strongest
evidence that naming the class was worth doing.

1. **The flattened ladder** — the rig overrode every rung with one weight set, so
   36 cells were byte-identical and the "ladder" it measured had one rung.
2. **The floor's reflex ladder** — with the override gone, the rig still read
   `FighterBrainProfile::for_level`, not the authored rows, because the demo app
   does not depend on `ambition_content`.
3. **The rollout** — that same floor switches the L3 search ON at level 6, and the
   shipped ladder switches it OFF on all nine rows.

4. **The fighters themselves** — found the same day, and until the clock turned up
   it was the widest of them. `ladder_rig`'s `fighters()` defaults to `smash_duelist_a` vs
   `smash_duelist_b`. ⛔ **Neither is George.** `register_character` hands George's
   authored table to `smash_george_booul` and `fighter_moveset()` to everybody
   else, and that stand-in contract bound **18 verbs to George's 26** — no
   `special`, no `special_forward`/`_up`/`_down`, no `attack_forward`, no
   `attack_dash`, no `taunt`.
   ⛔ **THE SENTENCE THAT FOLLOWED THAT LIST IS WITHDRAWN (2026-09-04).** It read
   *"two fighters with a dead special button, no forward tilt and no dash
   attack"* — and two thirds of that is an inference from BINDINGS that the
   engine does not honour. `directional_verb_chain` falls back to the base verb,
   so an unbound `attack_forward` is answered by the `jab`; the fighters had a
   forward tilt and a dash attack the whole time, swinging the jab's timeline.
   ⇒ **What survives is the special button, and it survives exactly.** Enumerating
   every `(base, direction, stance)` press instead of counting keys: the stand-ins
   answer nothing on **15**, George on **7**, George's set is a strict SUBSET of
   theirs, and the surplus is **eight — every one a `special`**. ⭐ So the true
   statement is: **every ladder number in this document was taken between two
   fighters that were George's genre shape with the special button removed.** That
   is still an instrument defect worth the entry, and it is a narrower one than
   the withdrawn sentence claimed. Guarded now in `game/ambition_demo_smash/` by
   `the_stand_in_is_george_s_genre_shape_with_the_special_button_removed`. ⚠ And this one is not only an instrument defect: the demo's select
   roster is three characters, **two of them stand-ins**, and the catalog default
   is one of the two — so the dead button was a player's as well as the rig's.

5. **The match clock, and it is the biggest.** The shipped match is **eight
   minutes** (`time_limit_ticks = 8 * 60 * 60`); the rig ran **sixty seconds**.
   ⇒ Every bout was cut off before it could end, so stocks tied in every cell, so
   **every verdict this tool has ever printed fell through to the damage
   tiebreak** — a fact about the instrument that read as a fact about the
   fighters. ⭐ Fixed by reading the demo's own constant instead of choosing a
   number, which is the whole rule in one line.

⭐⭐⭐ **AND THE FIVE ARE ONE ROOT CAUSE, NAMED LATE: `profile_for_level` IS A
FORK — two authorities answering "what does rung N mean", both shipping.**

```rust
pub fn profile_for_level(level: u8, ladder: Option<&FighterBrainLadder>) -> FighterBrainProfile {
    ladder.and_then(|l| l.level(level)).cloned()
        .unwrap_or_else(|| FighterBrainProfile::for_level(level))
}
```

⇒ The authored `.ron` answers it when a composition installs one; the engine floor
answers it when none does. **Both answers ship, and nothing at the call site says
which one you got.** ⭐ Its own doc even records the shape — *"a rule about which
of two sources wins cannot be enforced by the source that loses"* — which is a
statement that this IS the arbitration, not that there is only one authority.

✔ **AND THE FORK'S BLAST RADIUS IS BOUNDED — re-derived from source 2026-09-04,
because *"two authorities"* reads as more dangerous than it is and an
overstated defect is as unfixable as a hidden one.** The worry the shape invites
is **mixing**: `unwrap_or_else` fires per CALL, so a composition that installs an
authored ladder could still be handed floor rows for whichever rungs that ladder
happens not to author — two authorities inside a single match, silently. ⇒ **That
cannot happen for levels 1–9**, and three things have to hold for it not to,
all of which do:

- `FighterBrainLadder::problems()` requires **exactly nine rungs** and that rung
  `i` is **labelled level `i+1`**, so a validated ladder covers 1..=9 with no
  holes.
- ⭐ **It is CALLED on the production load path** —
  `crates/ambition_combat/src/brain/fighter/content_schema.rs:73`, not
  under `#[cfg(test)]` — and reports every fault at load as a diagnostic.
- The shipped ladder is asserted clean by
  `game/ambition_content/tests/fighter_brain_ladder.rs`.

⇒ **So the fork bites in exactly one place: whether a ladder is installed at
all.** Not which rung, not partway through a match. That is a *composition*
question with a single production installer, which is why the remedy is small and
why the four symptoms below all trace to one condition rather than to nine.
⚠ The out-of-range case (`profile_for_level(200, Some(&ladder))` → floor) is
deliberate and pinned by `a_shipped_ladder_beats_the_engine_floor`.

⭐⭐⭐ **AND THE FORK'S COST IS LARGER THAN FOUR DEFECTS: AN ENTIRE SUBSYSTEM IS
REACHABLE ONLY THROUGH THE AUTHORITY THAT LOSES.** Traced 2026-09-04, from the two
sources:

| | `rollout_depth` |
|---|---|
| the engine floor (`FighterBrainProfile::for_level`) | `if level >= 6 { 12 } else { 0 }` — **ON at rungs 6–9** |
| the shipped `fighter_brain_ladder.ron` | `0` on **all nine** rungs |

⇒ `refine_by_rollout` returns immediately on `!profile.uses_rollouts()`
(`rollout_depth > 0 && rollout_k > 0`), so **for every fighter a player has ever
met, the L3 search never runs.** And behind that one gate sit:

- the **shadow integrator** and its `TODO(compat-remove)` — a note proposing to
  replace it with the real movement kernel *"once its decision cost is budgeted"*.
  ⚠ Its cost is currently **zero on the shipped ladder**, because it never
  executes; the budget question is about a path no shipped fighter takes.
- **`read_weight`**, authored `0.0 → 1.0` across nine hand-tuned rungs and read
  only through this gate — the *dead* worked example elsewhere on this page.
- the **`Dodge` / `Shield` suppression**, for the same reason.

⭐ **So "which of the two authorities should exist" is not only about difficulty
numbers.** One of them switches an entire search subsystem on; the other switches
it off everywhere, and the one that ships is the one that switches it off. ⇒ Any
effort budgeted against the rollout — including that TODO — is effort spent on the
floor's behaviour, and the floor is the authority nobody plays against. ⚠ Worth
knowing before it is prioritised, not after.


⇒ **Divergences 1 through 4 below are not four accidents; they are four symptoms
of that fork.** Flattened weights, floor-not-ron, rollout-on-at-6, and the
`UtilityWeights::default()` that IS the level-9 row — every one is *the losing
authority answered and looked plausible*. ⚠ Only #5, the clock, is independent.

⭐ **Which reframes the ownership question already with Jon** (`awaiting-maintainer-decision.md`):
it is not really *"who owns the difficulty ladder"* but **"which of these two
authorities should exist"** — and the repo's own principle elsewhere is *one
authority per question*. ⇒ A fallback that silently substitutes a different
answer is the thing that made five findings possible; a composition that must
supply a ladder, or a floor that refuses rather than substitutes, would have made
all four impossible rather than findable.

⚠ **Not proposing the removal here.** The floor exists so a demo with no authored
content still runs, which is a real requirement — the point is that the cost of
that convenience is now measured, and it is four defects that each took a day's
instrument work to see.

⇒ **The class: the instrument took its configuration from a DIFFERENT SOURCE than
the shipped game, and only the instrument was ever read.** Every one of the five
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
2. ✔ **DONE — the `5 vs 3` inversion is isolated to `frame_advantage` +
   `expected_payoff`, jointly.** Byte-for-byte: those two reproduce the whole
   effect of swapping all four weights, and the other two reproduce the control.
   Neither of the pair suffices alone. Replicated at 28 seeds on the shipped
   clock and consistent in direction across all nine scenario fixtures. ⇒ **What
   replaces it: measure candidate PROGRESSIONS**, so the answer to "what should
   rung 5 be" arrives with evidence instead of as a shrug. Two are running (a
   gentler rise, and holding the pair flat); the arm that matters is whichever
   removes the inversion *without* flattening rung 5 into rung 3.
3. **Answer the roster question** — do the two Robots get kits? Every ladder
   number ever taken was measured between two fighters that had no special
   button, and George resolves matches they cannot. ⇒ Until this is answered the
   rig is measuring fighters nobody intends to ship as final.
4. **The `t3` placement race** (question 50). Narrowed to *"may a match present a
   tick in which the followed body has not been placed?"* — now a correctness
   question rather than a feel judgement, with a one-line reproduction.
5. ⬇ **`Dodge` shadow model — DEMOTED, and its blocker is now NAMED rather than
   vague.** It only matters under the rollout, which the shipped ladder disables
   on all nine rows, so it reaches no player either way.
   ⭐⭐ **And the blocker is STATE, not geometry** — traced 2026-09-04 through
   `available_dodge`. `MovementVerb::Dodge` does not name one maneuver across a
   rollout: grounded with cooldown clear it is a **roll**; airborne with budget it
   is an **air dodge**; airborne with the budget **spent it falls through to a
   DASH**. ⇒ `ShadowFighter` carries `on_ground` but not `dodge_cooldown` or
   `air_dodge_spent`, so from step 2 onward the shadow cannot tell which. At step
   1 it need not — `options.rs` reads the body's already-resolved `BurstManeuver`
   — but a rollout is precisely the thing that asks about later steps.
   ⛔ **Modelling it as any one of the three would repeat an error that module
   records making TWICE**: *"the brain named one maneuver, the model judged a
   second, the body performed a third."* ⚠ **AND IT IS NOT SIMPLY TWO FIELDS ON
   `ShadowFighter`** — I wrote that first and checked it an hour later.
   `SelfView` carries `burst` (the RESOLVED answer for this tick) and
   deliberately NOT the cooldown, air-dodge budget, endlag or dash charges that
   produce it, because `resolve_burst_maneuver` is *"the one rule, and this field
   is its answer. The brain is handed a fact."* ⇒ Perception hides the inputs
   precisely so a brain cannot re-derive the rule — the failure this module made
   twice. ⭐ **So the principled fix is to let the shadow CALL that resolver on
   shadow-stepped state**, keeping one authority, rather than exposing the inputs
   (which invites the re-derivation) or guessing. Until then `None` is the honest
   answer and its cost is recorded.
6. ✔ **DONE — flat-vs-platforms re-run at the shipped clock**, and its headline
   reversed: the tiers do not halve the lethality (stocks left is `0.00` on BOTH
   stages), they take **1.83× as long** to reach the same end. Pace, not
   lethality. ⚠ The `closest_approach` half is moot — there are no unfought rows
   at the shipped clock (109 bouts, zero where neither seat dealt damage).

⇒ **WHAT IS ACTUALLY LEFT, and it is short.** Items 1, 3 and 4 are with Jon.
Item 2 and 6 are done. Item 5 (`Dodge`) is real but reaches no player. ⇒ So the
next MEASUREMENT worth taking is none of these — it is the one the roster answer
unblocks, and until that answer arrives the honest state of this page is *the
fighter is measured and the open questions are design*.

⚠ **One engineering item nobody has claimed**, recorded so it is not lost: the
scenario matrices on this page were taken at the 60-second clock and only the
`5 vs 3` slice has been re-run at the shipped one. The Shield-model comparison in
particular still rests on short-clock numbers. ⇒ It changes no conclusion I have
drawn (that comparison found *no effect*, and a longer clock cannot turn no effect
into evidence for keeping it), but a reader quoting those tables should know.

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
