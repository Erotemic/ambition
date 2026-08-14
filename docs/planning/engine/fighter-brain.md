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

### 2. Calibrate the difficulty ladder with measured outcomes

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
