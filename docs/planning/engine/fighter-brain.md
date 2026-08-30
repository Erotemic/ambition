# Advanced fighter brain — current evaluation and regression work

**State:** implementation exists; current work is evaluation and one confirmed
rollout regression.

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

### Level-6 rollout regression is confirmed

The current controlled `recovery_below` A/B is:

```text
                         l1      l3      l5      l6      l9
rollout ON (shipped)    45/45     0       0     45/45     0
rollout OFF             45/45     0       0      0/45     0
```

Run: `ladder_rig --sweep-below [--no-rollout] --seeds 45`.

The result was reproduced and remained byte-identical to the prior recording.
`RecoveryLens` did **not** change which bouts are fought. Level 6 is completely
rescued by disabling rollout while level 1 is unaffected, so this is a
rollout-caused level-6 recovery regression rather than evidence for a global
recovery rewrite.

Level 1 is a separate case: disabling rollout does not rescue it, and its long
reaction delay relative to the fixture fall time is one of the profile-level
variables the rig can isolate.

## F1 — diagnose one level-6 decision, do not run another broad sweep

The next experiment is a single controlled trace at the rollout authority
boundary. For one failing level-6 `recovery_below` seed, capture per decision:

- L2 movement option order;
- each movement line passed into rollout refinement;
- which verbs enter `suicidal_movement`;
- `RecoveryLens::regains_support` / route verdicts;
- `least_bad_movement`;
- the final selected movement.

Run the identical seed with rollout disabled and diff the L2 choice that
survives. The goal is to identify the specific veto/reordering that converts a
successful L2 decision into the total level-6 failure.

This is queue row `D-FIGHTER-L6`.

Do **not** tune rollout depth, RecoveryLens heuristics, APM, reaction time, or
movement weights before the trace identifies the responsible decision.

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
infrastructure, but the level-6 A/B is evidence that the **decision integration**
remains wrong for this case. It is not evidence that the generic platformer
navigation architecture should be replaced.

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
