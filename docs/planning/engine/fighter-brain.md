# Advanced fighter brain — evaluation, difficulty and regression contract

**State:** architecturally stable; product calibration open. The major brain
architecture regressions have been diagnosed. Remaining work is representative
evaluation, explicit difficulty/roster decisions, and product tuning backed by a
trace that names the responsible decision.

This page is the current brain/evaluation contract. The large fixed-seed tables,
null-control runs, superseded ladder hypotheses and investigation chronology are
preserved by git history and measurement artifacts rather than in the live plan.

## Scope

This page owns:

- fighter difficulty/profile semantics;
- evaluation-rig representativeness and provenance;
- deterministic regression methodology;
- no-cheat constraints;
- rollout/shadow-model requirements specific to fighter decision quality;
- current brain calibration decisions.

Generic navigation/recovery capability is owned by
[`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md).
Performance cost outside fighter decision architecture belongs in
[`performance-and-iteration.md`](performance-and-iteration.md).

## Current architecture

### The brain acts through normal perception and actor control

Higher difficulty may improve what the brain decides and how reliably it commits.
It may not bypass actor control, use privileged future state, ignore physical
constraints, or directly scale combat outcome merely to make a rung win.

The brain receives semantic resolved facts rather than reimplementing core body
rules from lower-level state wherever possible.

### Difficulty and player-facing gameplay difficulty are different systems

Brain difficulty/profile parameters may change:

- reaction delay;
- decision cadence/APM;
- commit probability;
- accuracy/noise;
- policy/scoring weights;
- planning breadth/depth where enabled.

`ambition_persistence`'s player-facing gameplay `Difficulty` may legitimately
change damage multipliers. Do not confuse that product setting with the no-cheat
rule for fighter AI profiles.

### Recovery uses the shared body capability

The level-6 recovery regression was an integration/decision bug, not evidence
that fighter AI needs a private navigation engine. Keep the reusable recovery
probe/`RecoveryLens` authority and integrate fighter decisions against it.

### Rollout/shadow simulation must call canonical rules

A shadow state may not guess a future maneuver when the real body resolves that
maneuver from cooldown/budget/grounding state.

The `Dodge` lesson is the durable rule: if a semantic verb can resolve to roll,
air-dodge, dash, etc., the shadow path should call the same resolver on
shadow-stepped state or decline to model it. Do not expose hidden inputs merely
so a second implementation can re-derive the rule.

## Evaluation-rig contract

A table is not evidence about the shipped fighter unless the run states the
inputs that define the fighter and scenario.

Every report must print/record:

- exact Git HEAD;
- fighter ladder/profile source path;
- fighter/roster identities and movesets;
- stage/fixture;
- paired/unpaired design;
- difficulty/rung;
- decision/scoring weights or source thereof;
- seed set/count;
- bout clock/time limit;
- relevant feature/profile toggles.

Defaults must appear in the output even when the caller did not pass them.

A parse/load failure for an explicitly requested shipped ladder must fail the
run rather than falling back to a different ladder.

## Regression methodology

### Trace before tuning

Do not tune rollout depth, heuristics, APM, reaction, accuracy or utility weights
until a trace identifies the responsible decision/path.

Required workflow:

```text
reproduce
-> trace decisions/options/facts
-> identify the decision seam
-> make one targeted change
-> run matched control + changed arm
-> re-run representative scenarios
```

### Use null controls

When a rig change, seat assignment or comparison design is under question, run a
mechanically identical/null pairing. Seat or fixture bias that survives the null
must be separated from fighter-quality claims.

Equal rungs do not make equal fighters. Without `--character`/`--opponent`, a
`--rungs 6,6` row seats the demo's two default ids, and those are different
bodies (`smash_duelist_a` wears `player_robot_v3`, `smash_duelist_b` wears
`player_robot_v2`). The null control is one fighter in both seats at one rung:

```bash
cargo run --release -p ambition_demo_smash_app --bin smash_tool -- ladder-rig \
  --rungs 6,6 --paired --seeds 40 \
  --character smash_duelist_a --opponent smash_duelist_a \
  --ladder game/ambition_content/assets/data/fighter_brain_ladder.ron
```

**Known seat term.** Seat 0 takes about two thirds of decided pairs in this
control. The cause is the contested-grab tie-break: `ambition_combat::capture::systems`
awards a same-tick mutual grab to the lower `SimId`, which is seat 0. Inverting
that comparator flips the sign of the term. The tie-break itself is correct: a
mirror is a fixed point, and granting nobody the grab was measured at zero
captures. Other terms may also contribute; only one cell was inverted.

**Open rig change.** `--paired` exchanges the two seats' noise streams, which
cannot cancel a term keyed on seat. To cancel it, the pairing must exchange the
seats. Every recorded paired number was measured under noise-only pairing, and
every unpaired number (the default) carries the full seat term; discount old
ladder numbers accordingly.

Seat placement and decision publication are symmetric:
`ambition_demo_smash::respawn_placement` and
`ambition_match::prepared::seat_placement` place seats symmetrically about the
stage centre, and a later phase is the only writer of `ActorControl`.

### The mirror-divergence probe

`--noise 0` zeroes `execution_noise`, the brain's only per-seat stream, so one
fighter against itself at one rung becomes two deterministic policies in a
mirror-symmetric world. Every divergence is a defect or an adjudicated tie,
visible in one bout at a named decision:

```bash
AMBITION_FIGHTER_TRACE=1 cargo run --release -p ambition_demo_smash_app \
  --bin smash_tool -- ladder-rig --rungs 6,6 --seeds 1 --seconds 54 \
  --character smash_george_booul --opponent smash_george_booul --noise 0
```

The trace emits one line per decision per seat. Reflect seat 1's `x` about the
stage centre, negate its `vx` and `emit_x`, and compare field by field; the
first mismatch is the defect. If a line cannot explain a mismatch, add the
missing field (for example, `facing`) before you guess.

Mirror drift is an event, not a rounding floor: the simulation snaps bodies to
surfaces, and the reflected positions agree exactly on almost every decision. Ask
what happened on the decision where they part. A fighter whose bout contains no
knockout never reaches the respawn road; do not read a clean mirror as evidence
about a kit when the population is missing.

Defects this probe found are fixed and guarded:
`two_floors_at_one_height_are_told_apart_by_the_body_and_not_by_the_list`
(`supporting_floor` tie by list order), and `the_reset_leaves_the_facing_to_its_caller`
with `an_arrival_does_not_turn_the_body_around` (respawn facing is a call-site
answer, `ResetFacing::{Keep, Toward}`).

### Use enough clock for the outcome being measured

A short clock that leaves many unresolved bouts measures pace/partial damage,
not final match strength. When the product question is “which fighter wins,” use
the shipped/appropriate match clock and report unresolved bouts explicitly.

### Scenario premises must be real

A static opponent does not measure reaction delay. An unarmed stand-in does not
measure special-move choice. A recovery fixture must include the relevant body
capabilities.

If the premise is missing, fix the fixture rather than interpreting the table.

### A mirror match can lock into a limit cycle

Two copies of one brain at one rung rank the same menu the same way. Once the
menu narrows to one or two moves, the pair can repeat a period exactly, so a move
lands every time or never. A `0%` row in a mirror is a question, not a verdict:
seat a different opponent (`AMBITION_GRID_FOE`) before believing it. A zero in
that table can also mean a fighter that cannot be seated or has no repertoire.
`AMBITION_GRID_ONLY`, `AMBITION_GRID_FOE` and `AMBITION_GRID_TRACE` narrow the
grid; the trace's `landed` tally separates "chose badly" from "chose well and
missed".

### A near-miss must be measured on both axes

The sweep's `gap` column is `|x0 − x1|`, so a juggle 300px above reads as
point-blank. The column keeps that meaning because recorded sweeps depend on it;
read the trace, which carries both axes.

## Modeling rules for the option layer

These rules come from defects in the option and scoring layers. Each one is a
general shape; check new code against it.

- **Perception is delayed on purpose.** `DelayedPerception` implements the
  no-cheat reaction delay, and `Perceived::staleness_s()` reports how old a view
  is. Attack admission carries the foe forward over `staleness + time to
  connect`. Scoring is deliberately not led: `reaction_ms` is the shipped
  difficulty axis, and a brain that predicts perfectly everywhere flattens the
  ladder. If you widen the lead, state at which rung the prediction should be
  wrong, and by how much.
- **A field derived with a fallback answers two questions.** `startup_s` falls
  back to the whole duration for a move with no Active window, which is right for
  "how long am I committed" and wrong for "when does this become dangerous". Use
  `MoveFrameData::threat_live_at_s` for the second. The payoff gate asks when the
  move connects (`arrival_of` for a hazard, `threat_live_at_s` otherwise), and
  the closures that answer it are shared by admission and scoring.
- **A hazard carries its travel law.** `ThreatTravel::{Straight, Boomerang,
  Placed}` answers `travel_to(distance)`, and `None` for a distance it never
  covers. A new travel shape is a new variant, never a new scalar. A boomerang's
  reach is `v0 · out_s / 2`, not speed × time. Aiming (`travel_to`) and fusing
  (`live_at_s`) are separate questions; nothing prices a fuse yet, and that is
  recorded on the type rather than patched into the lead.
- **A placeholder is a request.** `MoveHazard::OwnersRangedAction` states that the
  move fires the body's ranged action. `attack_kit_of` resolves it with the
  runtime's precedence (what the move equips, then the body's kit). If no weapon
  can answer it, the move keeps its candidate and loses the hazard offer.
- **Staling is part of the price.** `AttackCandidate::wear` carries
  `MoveWear { damage, launch_growth }`, resolved from the body's
  `BodyStaleMoves` and the stage's `ResolvedCombatTuning`. It is not part of
  `MoveFrameData`, because two bodies with one moveset wear moves differently.
  The damage factor applies whole; the launch factor applies only to the percent
  term, never to `base`.
- **Recovery authority is a travel number.** `SustainedAuthority` and teleport
  distances answer movement questions and are read through `RecoveryRoute::carry`
  on the motion road, not as hazard reach. A teleport goes where the move aims,
  not toward the opponent. The motion score is a tent over `travelled / gap`.
- **Read the owner's sentence before reusing a number.** A published scalar
  answers the question its owner wrote. If a change that should be a tuning knob
  gives the same failure at both ends of its range, stop tuning and ask what the
  number means.
- **A common factor can still reorder candidates** if it multiplies only part of
  each expression. Launch factors multiply the percent term and not `base`, so
  they move where two moves' lines cross. `ambition_entity_catalog::launch` owns
  the one copy of the launch law; `WorldView::launch_law` delivers the stage's
  half.
- **An `Option` resolved by a fallback belongs to the law, not the callers.**
  `launch_speed(base, growth: Option<f32>, conditions)` takes the `Option`, and
  `LaunchConditions` carries `ruleset_growth`, so `None` becomes a number in
  exactly one place. `LaunchEnvelope::grows_under(conditions)` answers per world.

## Settled findings that still constrain work

### A press rate read off the evaluation rig is a reading of `apm_cap`

`ScenarioOutcome::apm` counts attack presses, and the option layer offers an
attack only where the move's region touches the opponent. The rig's opponent now
walks in to `RIG_ARMS_LENGTH`, `suite()` derives facing from the nearest hostile,
and the rig's `Up` candidate has its own coverage.

The authored `apm_cap` is what orders the rungs. Null controls on noise, rollouts
and `read_weight` move the curve by at most 0.7 APM; removing the cap turns it
into a saw, because decision cadence quantises the press rate.
`the_ladder_is_ordered_by_press_rate` claims a resolution of two rungs;
`probe_what_separates_the_rungs` holds the controls.

`ApmLedger::may_press` averages from the brain's first tick, not over a window, so
a CPU is most constrained at the start of a bout. This is shipped difficulty
behaviour; changing it moves every APM number.

### Level-6 recovery integration regression is closed

The trace identified the fighter decision integration as the defect; the shared
recovery capability itself was not the culprit. Preserve that split.

### Level-1 platform-floor regression is closed

Do not reopen it as a generic “low difficulty is broken” item without a new
reproduction.

### The shipped ladder and the engine fallback are distinct authorities

The repository has carried both an authored ladder and an engine/default floor.
Measurements can describe the wrong one unless the run names the source.

`ambition_demo_smash_app` does not install the authored ladder, so its rig
measures the engine floor, where every rung has the same utility weights (v1).
`the_ladder_the_demo_runs.rs` pins both facts; update this section when either
changes.

The maintainer decision about which authority should remain lives in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).
Until resolved, every calibration receipt must name the ladder source explicitly.

### Authored `read_weight` is not a normal shipped-axis today

The authored values and the current rollout configuration make its effect
unreachable in the shipped ladder, while the engine floor can make it live.

Whether to wire it into the non-rollout scorer or remove it is a product/design
decision coupled to the ladder-authority decision. Do not tune around it as if it
were already a working shipped axis.

### The middle-rung inversion was traced to utility progression, not reflexes

Matched experiments isolated the problematic ordering to the relevant utility
weights rather than reaction/APM/noise. Candidate progression changes should be
measured as progression changes; do not restart the search over every difficulty
parameter.

### Rollout-specific Dodge/Shield shadow work is low priority while rollout is off

Do not invest in richer rollout shadow modelling merely to improve code that the
shipped ladder currently disables. Reopen when a chosen ladder/profile makes the
rollout a player-visible capability.

### The thin Robot stand-ins are not a representative final roster

They lack the special-kit shape present in authored fighters. Rig conclusions
about match pace or move use must state whether the compared fighters have real
kits.

The actual Robot special identities are a maintainer/content decision, not an AI
architecture question.

## Current work

### F1 — resolve the ladder authority decision

Choose whether the authored content ladder is the required game authority or
whether a reusable engine-floor ladder remains a supported production policy.

Whichever survives:

- every game composition must have one unambiguous source;
- tools print that source;
- fallback behavior cannot turn a failed authored load into a different silent
  experiment.

### F2 — resolve `read_weight`

After F1, choose one:

- integrate it into a scorer that is actually active at the authored rungs and
  add behavior acceptance; or
- remove the field/rollback/config surface if the chosen ladder leaves it inert.

Do not keep an authored difficulty knob that looks live but has no reachable
reader.

### F3 — make evaluation rosters representative

For product calibration, use fighters whose kits represent the game being
shipped. If stand-ins remain useful as controls, label them as controls.

Once Robot kits are decided, add them to the representative ladder matrix rather
than retroactively treating old stand-in tables as roster evidence.

### F4 — finish the utility progression decision

Measure candidate `frame_advantage` / `expected_payoff` progressions against the
representative roster/scenarios.

Acceptance is not merely “rung N beats rung N-1.” Check that:

- ordering improves across representative fixtures;
- play does not collapse into one option;
- pace remains acceptable;
- no-cheat constraints remain unchanged.

A launcher has `coverage: None`, so its `reach_fit` is zero at every range while
its payoff is paid in full. Hazard coverage is the missing term. Per-move
staleness is now priced (`MoveWear`).

### F5 — placement race is correctness, not AI tuning

The `t3` question—whether a match can present a tick in which the followed body
has not been placed—belongs to runtime/presentation correctness. Keep its
reproduction and owner in the relevant planning/queue item; do not compensate in
fighter scoring.

### F6 — move-distribution readability

**Found:** `attack_kit_of` resolves presses with `move_for_directional_verb`,
while the press road calls `move_for_attack(.., RUNNING)`. So while a body runs,
the brain scores one move's frame data and the body performs another. The fix
(the "truthful kit") is behind `--features truthful_attack_kit` on
`ambition_platformer2d_actor_monolith`, default off; with the feature off the
resolver is today's.

**Measured, and not yet explained.** On the pirate admiral duel the truthful kit
helps at rung 3, is roughly neutral at rung 6, and fails the damage gate at
rung 9, where the brain stops choosing `jab` entirely (jab was over half of the
damage at HEAD). This is one fight per rung: a direction, not a curve.
`npc_emmy_noether` already fails the same rung-9 gate at HEAD with the kit change
a no-op, so the gate is calibrated to one fighter. Refuted explanations: the CPU
does not run constantly, the attack-kit-to-movement coupling
(`lifting_candidates`) is inert on the shipped roster
(`no_shipped_fighter_changes_its_lift_availability_with_stance`), and the CPU
does not spam the dash attack. Do not adopt a new story without an instrument.

**Next instrument:** log what `generate_options` was given and what it picked, on
ticks where the body stands, to tell "jab is absent from the standing kit" from
"jab is present and out-scored". Check first that `power` is normalised by
`kit_max_damage` over the current kit, so a change in kit membership re-prices
every candidate. Hypothesis to test after that: an opportunity term on movement,
where the value of standing still is the best standing attack it unlocks.

The instrument is the app acceptance test
`smash_cpus_damage_each_other::two_cpus…` with `FIGHTER`/`RUNG`/`TICKS`, not
`ladder-rig` (its default duelists bind no `attack_dash`, and the demo app cannot
seat authored movesets) and not the evaluation rig (synthetic kit, no damage).
Damage attribution joins on `ResolvedBodyHit::attacker_move_instance`.

Before adding randomness or per-move caps:

1. trace the scored movement + attack pair;
2. identify the missing opportunity/commitment term;
3. add one explicit policy term with clear zero/nonzero cases;
4. measure move distribution and match quality.

## No-cheat acceptance

Brain difficulty/profile code must remain unable to:

- multiply damage directly;
- read privileged future state;
- teleport/skip physical execution;
- bypass actor-control budgets/cooldowns;
- change deterministic physics merely because the opponent difficulty is high.

Prefer structural API boundaries that make these operations unavailable over a
long list of tests that merely hopes nobody adds one.

## Exit

This plan can leave active architecture status when:

1. one ladder authority is selected and reported by every rig/tool;
2. every authored difficulty axis has a reachable, tested meaning or is removed;
3. representative fighter kits/scenarios are used for product calibration;
4. the mid-ladder utility progression is acceptable across the representative
   matrix;
5. fixed-seed determinism and no-cheat boundaries remain intact;
6. remaining changes are ordinary fighter/product tuning rather than unresolved
   AI architecture.

Use git history for the removed 2026-08-31 through 2026-09-04 matrices,
statistical arms, rejected hypotheses and investigation chronology.

## Brain policy stays outside combat ownership

The [responsibility map](architecture-responsibility-map.md) treats combat-adjacent
brain code as decision policy over an authored capability menu. A scoring function
can consume combat/action facts without owning damage, capture or live actor
mutation. Do not absorb the fighter brain into a generic combat context to reduce
imports.

A4 preserves the normal accepted-control/action road for both human and CPU input.
A11/A12 ensure the authored techniques and flows that the menu advertises are
actually installed and valid. Selection tests must distinguish a legal but poorly
scored move from content that cannot execute; difficulty should not conceal either
with character-specific scripts.
