# Advanced fighter brain — remaining acceptance work

**Status:** implementation exists; the missing work is evaluation/calibration.

The full FB1–FB6 implementation history is archived at
[`../../archive/planning-superseded/2026-08-13/engine/fighter-brain.md`](../../archive/planning-superseded/2026-08-13/engine/fighter-brain.md).

Current source already contains the no-cheat fighter-brain stack under
`ambition_characters::brain::fighter`: situation classification, option scoring,
opponent-habit memory, authored difficulty profiles, scenario fixtures, and the
shadow-model rollout/refinement path. `FighterBrainProfile` now enables L3
rollouts on the upper ladder, so the old FB1–FB6 construction checklist is not
open work.

## Remaining work

### 1. Make the scenario suite an actual evaluation rig — STARTED, and it found its own blocker

`brain::fighter::evaluation` runs every scenario across all nine rungs through
the real `tick_fighter` seam and returns a report. Two properties hold and are
tested: one seed produces one report (108 rows, identical), and the report covers
every scenario the suite names.

⛔ **its first run measured that it could not yet measure the ladder**: with
`BrainSnapshot::idle()` every rung emitted zero presses, because an empty attack
kit leaves `generate_options` offering movement only. Armed with a kit shaped
like the one `build_attack_kit` assembles in the actor tick, the ladder appears:

```text
L1 mean 28.5 apm (cap 120)   L5  66.0 (270)   L9 103.5 (420)
```

**Measured properties, all tested:** press rate rises monotonically across the
nine rungs; every rung stays inside its authored `apm_cap`; one seed produces one
report; the report covers every scenario the suite names. The APM and ladder
checks were probed by un-arming the kit — both go red, so neither can pass
vacuously the way the first draft did.

⭐ **calibration fact worth keeping: the caps are not what separates the levels.**
Every rung presses at roughly a QUARTER of its own cap, so raising a cap alone
would move nothing — reaction and decision cadence are doing the ordering.

⚠ **and the ordering claim is narrower than "stronger levels win".** Winning is a
survival/damage question needing two bodies; this rig has one brain and a
scripted opponent. That is the remaining half, below.

(`apm_cap`'s *"Data today; enforcement is FB4's rig"* comment is stale:
`ApmLedger::may_press` gates every press already. What was missing was the
measurement.)

⚠ and one rig detail worth keeping: **the opponent has to move.** A rung's
headline difference is `reaction_ms`, and a delayed view of a world that never
changes IS the live view — so a static scenario would report the ladder as
degenerate for the wrong reason.

### Original item

`brain/fighter/scenarios.rs` already defines the named starting situations, but
its own source says the metrics half is not there. Build the smallest headless
runner that executes those scenarios through the real fighter-brain/controller
seam and records useful outcomes such as:

- survival / self-KO rate;
- damage dealt vs damage taken;
- recovery success where the scenario actually stages recovery;
- deterministic result identity for a fixed seed/profile.

Do not count a placement-only fixture as a scenario result. The runner must
instantiate the premise the scenario names.

### 2. Calibrate the difficulty ladder with measured outcomes — MEASURED 2026-08-14

The rig for this already existed (`ambition_demo_smash_app --bin ladder_rig`,
seating two CPUs at different rungs). What was missing was that nobody had run
it. Median time-to-elimination, higher rung : lower rung, over the four
registered pairs:

Two earlier passes here were wrong and are not reproduced: a 3-seed default
flipped the win direction on three of four pairs versus a 15-seed run of the
same deterministic build (`DEFAULT_SEEDS` is now 15), and the first pass with an
added engagement column misread `BodyHealth::damage_percent` — a ratio, where
`1.88` means the HUD prints `188%` — as an already-scaled percent, so `0.84%`
was actually **84%** and the "these fighters never hit each other" reading was
wrong by 100×.

⭐ **the corrected table, same build, same fifteen seeds:**

```text
3 vs 1    48.0% :  45.0%      5 vs 3   111.0% :  98.0%
6 vs 5   139.0% : 101.0%      9 vs 6   193.0% : 158.0%
```

⭐⭐ **the ladder discriminates in exactly this column**: peak damage rises
monotonically with the rung and the higher rung out-damages the lower on all four
pairs, while the time column stays within spread everywhere. The engagement
column was the working signal all along.

⚠ **the `ladder_probe` table below is NOT affected** — its `peak% = 0` is real
and by construction, because its opponent cannot attack. Two tables, one column
name, two different reasons for a small number, which is part of how this
survived.

⭐ **`ladder_probe` confirms it by a different route** (one fighter, opponent
cannot attack, so every loss is a self-KO):

```text
level   first self-KO   survived   stocks lost   peak%
    1        6.0s        12.2s          3          0%
    3       11.1s        13.8s          3          0%
    5       34.0s        36.5s          3          0%
    6        5.6s         8.4s          3          0%
    9        7.4s        17.7s          3          0%
```

Every rung loses all three stocks to itself, at 0% damage, exiting at |v| 760.
⇒ **the ladder cannot be calibrated on outcomes until a duelist stops walking
off the stage**, because suicide latency is the only thing these bouts measure.

⭐⭐ **and the probe's clean A/B is the lead worth pulling.** Same level-9
profile, only `rollout_depth` varied:

```text
9 / depth 0     47.8s to first self-KO,  54.3s survived
9 / depth 12     7.4s to first self-KO,  17.7s survived
```

The L3 rollout — enabled automatically at level ≥ 6 — makes a fighter **more
than six times worse** at staying on the stage. That is a decision-model
finding, not a tuning one, and it is the first thing to investigate: a search
that plans twelve ticks ahead is choosing to leave.

**Four hypotheses tested against HEAD (2026-08-14); two are dead.**

- ✘ *the shadow model cannot see the floor.* The stage is one
  `Block::solid("smash_platform", …)`, `perceived_solid_kind` maps `Solid` →
  `SolidKind::Solid`, and `build_world_view` clips terrain from `world.blocks`.
  Terrain is perceived.
- ✘ *the model cannot see the stage edge.* `ShadowState` carries `stage:
  StageView` and prices leaving it: `outside && !started_offstage` raises
  `ShadowEvent::Ko`. The rollout tests already pin that a bounded floor produces
  a self-KO event and an infinite plane does not.
- ⚠ *the model's death line is in the wrong place, and conservatively so.*
  `StageView.bounds` is `world.size`; the sim kills at `world.size +
  world.edges.fall`. The imagination therefore thinks it dies EARLIER than it does,
  which should make a fighter more cautious, not less. It cannot explain a
  fighter that leaves.
- ▢ **`ground_span` is only known while standing.** `supporting_floor()` requires
  the solid's top within roughly one body-height of the feet, so an airborne
  fighter has `ground_span: None`. That is survivable given the stage bound
  above, but it means the ledge-relative half of the model goes blind exactly
  during recovery.

⭐⭐ **MECHANISM, and it is arithmetic.** The rollout prices a KO when the body
leaves the stage box — and on the Smash stage that consequence lies **beyond the
search horizon**:

```text
platform top   y = 300      world box      640 x 480      gravity 2250 px/s²
fall to the box floor       180 px    ⇒    t = √(2·180/2250) = 0.40s = 24 ticks
rollout_depth (level ≥ 6)                                        12 ticks = 0.20s
free fall within the horizon              ½·2250·0.2²    =        45 px
```

The death is at twenty-four ticks. The imagination stops at twelve. **Stepping
off the ledge costs nothing inside the horizon**, so the option scores clean —
and a deeper search simply finds more ways to leave, every one of them free.
That is why depth 12 is worse than depth 0 rather than better: it is not a
broken model, it is a model that stops looking one step before the cliff bites.
Horizontally the same holds — 112 px from the platform lip to the box edge, more
than a fifth of a second of running.

⇒ **two candidate fixes, and they are not equivalent.**

1. *Lengthen the horizon* past the fall. Honest but expensive: the module's
   contract is exactly `rollout_k × (1 + rollout_depth)` shadow steps with no
   early exit, so reaching 24 ticks doubles the per-decision cost of every
   upper-ladder fighter.
2. *Price the committed fall* — ⛔ **REFUSED (Jon, 2026-08-14).** The rule *a body
   that is airborne, below the platform top and outside `ground_span` is already
   dead* was implemented, measured (depth 12 went from 7.4s to 43.2s, surviving
   past 60s), and then **removed**, because it is not body-generic and it is not
   true. A body may still recover with air movement, a jump it has not spent,
   flight, a wall, a ledge grab, a recovery attack, an impulse, a portal or a
   grapple. It happened to hold for THIS stage and THIS fighter, which is exactly
   what a Smash-specific approximation looks like from inside.

⇒ **the diagnosis stands, and as of 2026-08-15 the fix is option 3 — the one the
refusal named.** A committed-fall verdict now comes from actual recoverability
under the body's own capabilities:
`ambition_characters::brain::fighter::recovery::RecoveryLens` lowers the perceived
view into a real `ae::World` and the body's own kit into an
`ae::BodyClusterScratch`, and `refine_by_rollout` asks
`ae::movement::recovery::probe_recovery` **once per movement line that left the
ground** — never per node, never per tick. The kernel's answer overrules the
shadow's in BOTH directions, which matters as much as the condemnation: the
shadow's movement line HOLDS after `commit_ticks`, so near a ledge it condemned
every verb, the veto emptied, and the choice fell through to
`least_bad_movement` — the line that dies latest rather than the one that lives.

⭐⭐ **and as of 2026-08-15 the lens is armed with the body's own recovery move.**
The module's standing warning — *"a body that recovers by … a recovery attack is
not explored … the first thing to re-check the day a fighter gains one"* — came
due. `BodyKit` now carries an optional `RecoveryLift`, derived from the kit by
`options::lifting_candidates` (a filter on `MoveFrameData::lift_speed`, which is
non-zero only for an `ImpulseMode::Set` impulse pointing against gravity), and
`RecoveryLens::from_view` turns it into `RecoveryPolicy::drift_jump_and_burst`.
So the veto is taken against a body that can throw its Up-B rather than one that
cannot, and the negative it produces names the burst it spent.

⛔ the burst does **not** ride `pending_launch`, and the reason is a trap worth
recording: `accept_external_launch` feeds `launch_into_tumble`, so any burst over
the body's `tumble_speed` would knock the PROBE body down and report "no
support" about a body the real runtime never tumbles. It mirrors the
authored-impulse seam instead — a direct `vel` write — inheriting that seam's
surface-momentum approximation so the two are wrong together or right together.

⭐ the same affordance closed the other half: L2 returned an EMPTY attack list in
`Situation::Recovery`, so a fighter carrying a real Up-B drifted at a stage it
could not reach while holding the thing that would have saved it. It now offers
the kit's lifting moves, scored on lift alone. ⛔ no character conditional
anywhere: a body that authors no rising move plays exactly as before.

⛔ **and it is NOT measured.** No bench prices a decision with the lens attached
(`the_worst_shipped_budget_is_cheap_enough_to_be_a_non_event` rolls no movement
lines), and `ladder_rig --scenarios` has not re-run. The depth-0-vs-12 survival
A/B above is the instrument that would close this, and until it does the fix is a
structural argument with unit evidence, not a measured improvement.

⭐ **one half of the removed change was KEPT, because it is fidelity rather than
heuristic.** `ShadowFighter::ground_level` was `view.on_ground.then(…)` — `None`
for any airborne body — so the shadow of a falling body fell through the world
forever and never landed. `WorldView::floor_below` was added beside
`supporting_floor`: the supporting one answers *"what am I standing on"* and is
right to give up when nobody is standing; a rollout asks *"what will I land on"*,
which has an answer at any height. `ShadowState::from_perceived` seeds both
`ground_span` and `ground_level` from it, and `advance_phase` lands the body on
the floor that is actually there.

⛔⛔ **and the measurement is the lesson: the terminal value alone changed
NOTHING.** The first run with the predicate and without `floor_below` measured
identical numbers to the baseline, because an unrecoverable-trajectory test
cannot fire when the model has no floor to be unrecoverable from. Whatever
replaces the refused rule needs the floor knowledge too.

(The other two suspects are cleared: `started_offstage` resets the moment a body
re-enters the box, so it cannot suppress a death for a fighter that walks out
from inside; and `ground_span` feeds only `on_ground`, so it makes the body fall
without ever raising a KO of its own.)

Use the evaluation rig to determine whether the authored level ladder is
meaningfully ordered. The historical target was that stronger levels beat weaker
ones more often and that the upper ladder clears a useful damage/survival floor;
re-measure before freezing exact thresholds.

If monotonicity fails, tune authored profile data or the decision model based on
the measured failure. Do not make difficulty cheat by reading privileged state,
removing reaction latency, scaling damage, or bypassing `ActorControl`.

### 3. Keep humanity/no-cheat evidence cheap and explicit

The existing perception-delay/APM/profile constraints should remain observable
from the same evaluation run. Add only the instrumentation necessary to show the
brain is still acting through delayed/perceived facts and the normal controller
seam. Do not build a second permanent telemetry framework.

## Exit

This plan is complete when the named scenarios execute through the production
brain seam, upper/lower ladder behavior is measured and reasonably ordered, and
there is a compact repeatable report that can be used both for CPU calibration
and boss-playtester work.
