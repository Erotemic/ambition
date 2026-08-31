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

Run: `ladder_rig --sweep-below [--no-rollout] --seeds 45`.

`RecoveryLens` did **not** change which bouts are fought, and the traced Recovery
decisions were byte-identical between the two arms for their first 22 ticks — the
divergence was never on the recovery road at all. It was in Neutral/Advantage,
where the veto emptied the modelled options and an unjudged one took over.

Level 1 is a separate case, and its reaction delay is NOT the reason.

⛔ **MEASURED AND FALSIFIED 2026-08-31.** `--reaction-ms` at 500, 300, 150 and
**0** gives byte-identical outcomes on seed 0 (`5.9s : 10.4s`, 0%/0%, unfought).
The trace changes at 0 — the 100-200px per-decision position jumps that are
perception staleness disappear — so the override lands and the outcome simply
does not depend on it.

⭐ **TRACED TO A CONTRADICTION BETWEEN TWO AUTHORITIES.** On 6 of 6 grounded
decisions the l1 body reports `ground=true terrain=1..2 supported=false
floor_edge=None`. Terrain REACHES the body; none of it passes
`WorldView::supporting_floor`'s hand-written y-band around the feet. `on_ground`
is kernel truth and `supporting_floor` re-derives the same fact from a band, so a
body the kernel says is standing on something perceives no floor at all — and
every ledge question in the brain reads through there.

`terrain=N supported=bool` are on the trace now. They are what separated *"no
terrain reached me"* from *"terrain reached me and none of it is under my feet"*,
which want opposite fixes — the same instrument gap the l6 diagnosis hit one
field over.

⛔⛔ **AND THE FIX THAT FOLLOWS FROM THAT READING IS MEASURED WRONG.** The solid
under the body is a `BlinkWall`, whose own doc says *"full collision ... a brain
without the blink-through upgrade treats it as `Solid`"* — so adding it to the
three floor filters is the obvious move. It **regresses l6 back to `unfought
1/1`, its exact pre-fix numbers**, and does not change l1. Reverted; do not
re-derive it. A coherent reading of the source is not a measurement.

⭐ **THE BLINK WALL IS THE RESPAWN PLATFORM**, identified by probing the
perceived block's name. `ambition_demo_smash` publishes it as a
`MovingPlatformState` (`lib.rs:1662`), and `platforms/mod.rs` inserts platforms
as blink-passable blocks — *"solid for normal collision, but blink-passable for
upgraded blink pathing"*.

⛔⛔ **AND IT IS REBUILT EVERY TICK AT THE PROTECTED FIGHTER'S OWN POSITION**
(`Vec2::new(kin.pos.x, kin.pos.y + DROP_PX)`). It calls itself stationary — zero
sweep, zero speed — and from the outside it tracks the body exactly. That is why
`floor_edge` comes out a **CONSTANT 48.0 across 200px of travel** the moment the
block becomes standable: the body is always in the middle of its own floor.

⭐ **A FLOOR DEFINED AS "WHEREVER I AM" MAKES EVERY LEDGE QUESTION CIRCULAR** —
the answer cannot change whatever the body does. That is why it poisons the
ROLLOUT specifically, the one consumer whose whole job is asking *"where will I
BE"*: every verb is judged to walk off, everything is vetoed every tick, and
level 6 falls back to the least-bad line on every decision.

So the fix is at the PLATFORM, not at the filter. Queue row
`D-BRAIN-PLATFORM-FLOOR`.

Queue row `D-FIGHTER-L1`.

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
  --bin ladder_rig -- --sweep-below --seeds 1 2>&1 | grep '^\[fighter '
```

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

## Exit

This plan can close when:

1. the level-6 rollout-caused `recovery_below` regression is diagnosed and fixed
   at the responsible decision seam;
2. the scenario/duel reports give repeatable useful calibration signals across
   the authored ladder;
3. fixed-seed determinism and no-cheat constraints remain covered;
4. remaining fighter behavior work is ordinary product tuning rather than an
   unresolved brain architecture problem.
