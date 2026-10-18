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

**Seat term (removed 2026-10-11).** The contested-grab tie-break gave a
same-tick mutual grab to the lower `SimId`, which is seat 0. Measured
2026-10-08 (rung 6, shipped ladder, 40 paired seeds, three fighters): seat 0
took 26 of 45 decided pairs (58%, within spread), and with the comparator
inverted 19 of 45; each fighter's split swapped exactly, so the grab tie was
the whole seat term in this control. A mutual grab now parries: neither grab
holds and both bodies rebound (S0.2), so no rule treats the seats differently.
The ladder rig has not been run again since; the figures are in
`ladder_rig.rs` (`--rungs` docs).

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

### Mirror symmetry is a correctness property (Q49)

Ruling Q49 (2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)):
two CPUs with mirrored starting states and one brain/profile on a symmetric
stage behave identically up to reflection until a modelled asymmetric fact
(geometry, observation, state/history, profile, player input) breaks the
symmetry. For the Emmy Noether case this is required. Random variation added
only so that CPUs look different is not allowed.

**Coverage today (2026-10-04).**

- `game/ambition_app/tests/smash_cpu_cognition.rs`,
  `two_emmys_are_one_fighter_reflected_through_a_mutual_grab` (2026-10-08,
  plan item 1; the grab clause 2026-10-11): each of 1200 frames compares
  position, velocity, facing, the move and its clock, the published control
  and the stream position (lateral values negate about the midline) and
  reports the first frame and field that part. Two Emmys must not part, and
  the window must hold a parried grab. The control: two ordinary fighters part
  on a body field. Before the parry the Emmys parted on frame 883, when the
  lower `SimId`'s grab dash took the other. It replaced an outcome test that
  compared positions only and asked Emmy to hold 1.5x longer than an ordinary
  pair.
- The probe above (`--noise 0` trace) finds defects by hand; it is not a test.
- `two_floors_at_one_height_are_told_apart_by_the_body_and_not_by_the_list`,
  `the_reset_leaves_the_facing_to_its_caller` and
  `an_arrival_does_not_turn_the_body_around` guard defects that the probe found.
- `brain/fighter/decision/tests.rs`,
  `the_decision_of_a_reflected_scene_is_the_reflected_decision` (2026-10-08):
  eight scenes, each stepped as written and reflected about the stage centre,
  for 60 ticks at the immediate profile and rungs 3, 6 and 9 (execution noise
  removed). Lateral fields must negate; vertical fields and buttons must be
  equal. Three scenes put both bodies at one x. Poison: the zero guard in
  `apply_movement`'s `side_toward` removed, so `signum(0.0) = +1` chose the
  side; all three stacked scenes failed on `locomotion.x` at tick 0. No
  defect was found: the decision layer reflects in every scene.
  The raw `signum` sites in `rollout.rs` gave no asymmetry in these scenes:
  the chosen options did not depend on them. The shadow model has its own
  reflection test since 2026-10-08 (see the next item).
- `brain/fighter/rollout/tests.rs`,
  `the_shadow_of_a_reflected_scene_is_the_reflected_shadow` (2026-10-08): two
  stacked bodies and the reflected scene, stepped with an approach, a dash
  and an attack start. It found a defect: with `signum(0.0) = +1` both
  shadows drove to `+side` (x 490 against 490) and both attack starts turned
  to face `+side`. Every lateral sign in the brain now asks one rule,
  `ambition_combat::util::SignumOr` (zero gives no side, and an attack start
  keeps its facing); the two local copies in `brain/smash` are deleted.
  Control: the same verbs with the foe 60 px to the side.

**Known asymmetry sources (not yet triaged; each is a defect in a Noether
scene or an authored rule that must be named):**

- `f32::signum(0.0) = +1` where a zero lateral distance or velocity picks a side
  (`brain/fighter/decision.rs`, `brain/fighter/rollout.rs`). Fixed 2026-10-08
  in both: they ask `util::SignumOr` (see Coverage).
- `brain/smash/mod.rs`, perch side: falls back to `toward` only near zero, else
  an absolute `cross.signum()` from a seed phase.
- Grab contention: a same-tick mutual grab was a tie broken by `SimId`.
  Since 2026-10-11 it parries (S0.2), with no winner:
  `two_bodies_grabbing_each_other_on_one_tick_parry_and_rebound` runs both
  spawn orders and both message orders.
- Target selection breaks an exact-distance tie by `SimId`
  (`ambition_combat/src/targeting.rs`). Triaged 2026-10-08: not reachable
  in a two-fighter mirror, where each body has one foe.
- `movement/recovery.rs`, `DRIFT_SIDES = [0.0, -1.0, 1.0]`: the search tries
  one side first. Triaged 2026-10-08: not a defect for a decision. The order
  chooses only which effort `RecoveryOutlook::Regained { steps, effort }`
  reports, and no production reader reads those two fields: the brain asks
  `regained()` and `bounded_by()`, which do not depend on the order. A
  reader that wants `steps` must take the minimum over the efforts first.
- The measured seat term. Triaged 2026-10-08: it was the grab tie above,
  which is gone (see **Seat term**). The record was 69% of decided pairs
  (40 : 18), then 58% (26 : 19, within spread); what moved it was not
  bisected.

**Per-seat streams.** Ordinary fighters seed cognition per seat
(`fighter_cognition_seed` in `actor_spawn/brain_builders.rs`), and Emmy opts
out with `preserves_mirror_symmetry`. Several tests pin that ordinary seats do
NOT share a stream (`two_participants_of_one_character_do_not_share_a_stream`,
`the_character_is_not_the_ordinary_symmetry_breaker`,
`two_seats_of_an_ordinary_selectable_fighter_do_not_share_a_stream`). Under Q49
a per-seat stream is a modelled fact only if it is an input the design wants
(for example, a personality draw); it must not be the reason two fighters look
different. Decide this when MIRROR-SYMMETRY reaches it: either name the
per-seat draw as a profile fact, or seed by character for every fighter.
Until then the Emmy exception is the Noether scene.

**Regression to add (queue row MIRROR-SYMMETRY).** Strengthen the Emmy test
rather than add a second outcome test:

1. ✅ A per-tick reflection comparison of the full state that can diverge
   (built 2026-10-08, see Coverage):
   position, velocity, facing, action/move id and phase, the decision
   published to `ActorControl`, and each body's random-stream position. Report
   the first tick and the first field that part.
2. A symmetric stage, mirrored spawns, one brain and profile, and no
   symmetry-breaking input, so any divergence is a defect.
3. ✅ A unit-level reflection test of the decision layer (built 2026-10-08,
   see Coverage). Extend its scenes to reach the `rollout.rs` sites.
4. ✅ A same-tick tie fixture (2026-10-08): two bodies grab each other on one
   tick, in both spawn orders. Since S0.2 (`81fd79963`, 2026-10-11) neither
   grab holds and both bodies rebound by the clank's rule, because a lower
   `SimId` win treated mirrored bodies differently (Q49)
   (`two_bodies_grabbing_each_other_on_one_tick_parry_and_rebound`). Two bodies on one ledge are the ledge
   policy's (`CombatRules::ledge_occupancy`, receipt LEDGE-OCCUPANCY).
5. Poisons: a `signum(0) = +1` site, an unmirrored random draw, a list-order
   tie-break and a left-first query must each turn the test red.
   2026-10-08, the `signum(0)` poison (`SignumOr` returns `signum()` at zero):
   the per-tick test stays green with the first part on frame 883, so the
   Emmy match reaches no zero lateral distance before the grab. The poison is
   effective: `the_shadow_of_a_reflected_scene_is_the_reflected_shadow` is
   red under it. That unit test is the witness for this source; the match
   population does not reach it. A placement poison (seat 1 two pixels out in
   `ambition_match::prepared::seat_placement`) is red at frame 0 through the
   per-tick claim. The mirror line is the stage's centre, not the first
   frame's mean, which would move with the poison. An unmirrored stream
   (Emmy seeded per seat) is red through the shared-stream precondition, not
   the per-tick claim. A list-order poison (the `supporting_floor` tie by
   terrain order, the historical mirror break) is null in the match: no body
   stands over the twin respawn platforms before the grab, because a respawn
   follows a knockout. The poison is effective:
   `two_floors_at_one_height_are_told_apart_by_the_body_and_not_by_the_list`
   is red under it, and it stays that source's witness. A left-first query
   (2026-10-08): `brain/fighter/recovery.rs` `nearest_support` (the support a
   carry route aims at) told two supports at one distance apart by `(x, y)`
   only, so the left one won for both mirrored bodies. The support on the side
   the body faces wins now; `(x, y)` decides only two points at one offset
   along the side, which a reflection does not swap. Witness:
   `two_supports_at_one_distance_are_told_apart_by_the_facing_and_not_by_x`
   (poison: the facing comparison removed; red at the reflection claim, both
   facings aim at the left shelf). The Emmy match does not reach it: two
   mirrored supports are at one distance only from a body on the centre line,
   and only a carry route asks.
   The `Recover` verb steered to the centre of the room (2026-10-11): from
   below the top of the stage that is under the floor, and the floor stopped
   the jumps (George mirror, seat 0 at x = 571, y = 353, fell out of the room;
   the S0.2 parry made the bout reach it). It steers by
   `WorldView::recovery_ledge` now: out from under the floor, hold the side
   below the top, go in above it; twin floors are told apart by the facing.
   Witnesses: `a_recovering_body_below_the_top_does_not_steer_under_the_floor`
   (poison: the old rule; red at "beside and below") and
   `twin_floors_are_told_apart_by_the_facing_and_not_by_the_list` (poison: the
   facing tie removed; red at "facing right").

The current requirement that two ordinary fighters MUST break the mirror stays
only while per-seat streams are policy; it is a fact about that policy, not a
goal.

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

- **A brain remembers its own last move.** With no memory of it, a brain that
  whiffed derives the same ranking from the same world and throws the same
  move until the world changes, which in a mirror it never does. The brain
  keeps its last press (`FighterState::own_attack`) and declines a binding
  that whiffed for as long as the move plays (`refused`). A hit is the foe
  seen in hitstun within the move's length plus the perception delay. Both
  fields are rollback state. Measured on the grid 2026-10-12: queue row BRAIN,
  road 1.
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
- **Startup is the first live hit, not the first Active window.** A counter's
  stance is an Active window (the parry authority reads that tag) and hits
  nobody. `MoveSpec::frame_data` counts a window as live when it has a volume,
  a capture or a sustained hazard. Read from the first Active window,
  GNU-ton's `third_law` (stance from 0.0 s, sweep at 0.18 s) was a frame-0
  strike, the fastest move of his kit. Witness
  `frame_data_startup_is_the_first_live_hit_not_a_counters_stance`.
- **A counter is priced by the swing it meets.** `Features::counter_value` is 1
  when the foe is committed to a swing (`AttackStartup` or `AttackActive`)
  that faces me, reaches me (its `attack_reach`) and goes live while the
  stance (`MoveFrameData::counter_stance`) is open, read through the view's
  delay; else 0. Only a pure counter (a stance that offers nothing else,
  `pure_counter_stance`) is priced so, and it is offered only while it is 1.
  Its frame advantage reads the stance opening, not its startup (a stance is
  not a live hit). A move that also strikes is priced as its strike: priced
  as a stance too, two patent clerks in a mirror answered each other's windup
  with the same move, 160 presses for 0%. The weight (0.8) is the middle of the band on the Officer's real kit:
  from 0.49 the stance beats his gust against a swing that outranges him, and
  from 1.17 it beats an uppercut that lands first. Witness
  `a_counter_is_chosen_against_a_swing_that_meets_its_stance`.
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

Since 2026-10-16, `ambition_demo_smash_app` plays the rows of Smash's pack
(`data/fighter_brain_ladder.ron`, the file the shipped game's pack names
too); `the_ladder_the_demo_runs.rs` pins it. A room that no game gives a
ladder plays `FighterBrainProfile::engine_default`: one profile at every
level, with no rollout. A rig record from before 2026-10-16 taken without
`--ladder` measured the old level curve (`for_level`, deleted), which armed
the rollout at levels 6 to 9.

Q88 (2026-10-03) decided which authority remains: the brain owns the knobs,
and Smash content owns the ladder (see F1). Every calibration receipt names
its ladder source; the rig prints it.

### `read_weight` is deleted (Q90, 2026-10-08)

It changed no shipped decision: the only reader of the habit model is the L3
rollout, which every shipped rung disables. It had a second, hidden meaning: it
was the habit model's decay. The decay is now one constant
(`habit::HABIT_DECAY`, 0.9), and the rollout reads the habit whenever the read
is genuine, because it runs only for a profile that pays for it
(`uses_rollouts`). The decay change moves `for_level` brains at levels 6 to 9
(which roll out), such as the dismounted rider: their decay was `t × 0.6`. A
rung that should read less is a new, correctly named parameter with its own
witness.

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

A stand-in may keep an incomplete kit on purpose (Q89, 2026-10-04); do not
add specials to fill input slots. The real Robot's moves are authored when it
becomes game content. Neither is an AI architecture question.

## Current work

### F0 — CPUs self-destruct against an opponent that does nothing (2026-10-18)

`ko_envelope selfko FIGHTER [ticks=N]` seats FIGHTER as the default-level CPU
against a seat 0 that presses nothing and prints every stock it loses with the
position the tick before. Every loss is a self-KO. Over 3600 ticks, before the
fix below: Sanic 14, Alice 4, Robot v3 3 (in 1584 ticks), George 3, Pirate 3,
Bob 3, Ninja 2, all at 0%, into the pit beside a ledge.

Two paths, read from `AMBITION_FIGHTER_TRACE=1`:

1. A ground roll toward the ledge. `Dodge` had no ledge penalty, unlike
   Approach/Dash/Retreat; a Robot rolled off from 33 px out at 530 px/s. Fixed in
   `movement_options` (`roll_walks_off`). After: Pirate 1, Alice 2, Bob 2, Ninja
   1, George 2, Robot 3 in 3265 ticks. Sanic is unchanged.
2. A grounded move's own travel. Smash declares `edge_cancel_recovery`, so a
   move slides off a lip by design. Sanic chose `sonic_boom` (rooted) on a view
   showing 164 px of floor at −182 px/s. The brain perceives through the
   `reaction_ms` delay, including its OWN body (300 ms at level 5), and
   Approach kept accelerating it, so the move started at −478 px/s with about
   145 px left and coasted off. Pricing the move's travel against the floor
   ahead at decision time changed nothing (tried and reverted 2026-10-18): the
   input is stale, not the formula. Open question for Jon: should a CPU perceive
   its own body without the reaction delay?

3. `Recover` held the side whenever the feet were below the top. Just after
   stepping off, that left the Robot 48 px out, and its teleport (aimed by the
   stick, 44 px ledge assist) went straight up past the ledge. Level with the
   floor's side face (`RecoveryLedge::beside`: head above the floor's bottom,
   not under it), `Recover` now steers in. Below the underside it still holds.
   Same seeds, excluding Sanic: 16 losses → 12 (Mary-O 5→1, George 2→1, Robot
   3→2, Bob 2→4, the rest unchanged). One seed each, so noisy.
4. Advantage and EdgeGuard offered nothing that brakes a run: with Approach
   priced off the ledge, GNU-ton chose Jump at 530 px/s with 48 px of floor
   and carried the run off. They now offer Retreat when Approach would walk off
   or overshoot. Goblin 3→0, GNU-ton 4→2, Oiler 3→2, Pugnacious 4→1, Ramen 4→5.
   The first eight fighters are within noise (12→13 excluding Sanic).

5. Open: an AIRBORNE Approach is never priced off the ledge (`walks_off` and
   `overshoots` need a supporting floor). Ramen, jumping at a foe near the
   left ledge, drifted past it at −230 px/s and the lens found no route home
   with `divergent_series`. Mary-O's first loss is the same: Approach from a
   jump at x 218, still accelerating at x 129 (−236 px/s), Recovery at x 68
   (−300). A fix needs the floor's span on the side axis for an airborne body
   (`floor_ahead` reads only the supporting floor) and an airborne stopping
   distance from the body's own law, as `stop_distance` is for the ground.
   Tried and reverted 2026-10-18: `walks_off` answered in the air from the floor
   under the body and an `air_stop_distance` law. Roster 46 self-KOs in 20.0
   fighter-minutes → 50 in 21.2. Mary-O and Ramen were unchanged; Robot 2→5,
   Bob 3→6. Like item 2, the view is about 300 ms old: at 200–300 px/s that is
   60–90 px, more than the margin being priced. The stale self-view question
   comes first.

⭐ MEASURED 2026-10-18, the answer to items 2 and 5: a probe (not shipped) in
`PerceptionBuffer::observe` gave every buffered view the LIVE `self_view`, so
the brain sees the world 300 ms late but its own body now. Roster self-KOs
went from 46 in 20.0 fighter-minutes to 11 in 20.7: Sanic 12→0, Ramen 5→0,
Mary-O 3→0, Bob 3→0. It changes the humanity contract (the delay buffer is
the only read path), so it is Jon's decision. A human knows where their own
hands are without the screen; the probe is that rule. Offense was unchanged by the probe: the CPU eliminated the passive dummy
within the minute in 10 of 23 runs before and 9 of 23 with it. The blunt probe is not shippable as written: `smash_it` was 50/51 under it,
and `a_fighter_over_a_floor_open_for_it_plays_as_over_the_void` failed its
control (a CPU over a closed gate floor played exactly as over the bare void).
`SelfView` mixes the body's own state with facts read against that tick's
world, so a live self with a delayed world disagrees with itself. Kinematics only (live `pos` and `vel`, everything else delayed) is
as strong: Sanic 12→0, Ramen 5→0, Robot 2→0, Bob 3→0, Mary-O 3→1. It fails the
same control. Not yet known: whether the fixture's premise (the closed-floor
CPU diverges within 60 ticks) relied on the stale self-view, or the probe
loses a real read of the floor.

Totals over one minute each, one seed. The eight fighters with a true baseline:
32 → 25 (Sanic 14 → 12, item 2). The other fifteen were first measured after
items 1 and 3: 27 → about 19.

### F1 — the authored ladder is the authority (Q88, 2026-10-03)

Q88 decided it: the generic brain exposes the knobs, and Smash rules/content own
the mapping from "CPU level N" to them. Two facts still disagree with it
(measured 2026-10-08):

- `FighterBrainProfile::for_level` is a level curve in the brain crate, and 13
  production sites fall back to it when no ladder is installed. It is the "Smash
  difficulty curve" that Q88 says another game must not inherit.
- The Smash demo installs no ladder (`the_ladder_the_demo_runs.rs`), so the demo
  and its rig play that curve, not Smash content.

The move: the Smash demo pack lists the ladder file and the demo installs it
from its pack; the shipped game's pack names the same file (a pack source may
leave its root, as George's facet does), so there is one file; then the
engine floor becomes one level-free default profile. Each step changes what a
CPU does, so each re-measures the rig with the ladder named.

⛔ The install cannot be a second `insert_resource`. `ambition_app` composes
`ambition_demo_smash`, so the shipped game would have two installers of the
one App-global `AuthoredFighterLadder` (`ambition_content`'s plugin and reload,
and Smash's plugin), and the later one would win. The ladder is a rule of the
rooms it governs. Smash already declares its combat rules with
`declare_rules(RulesScope::Mode(SMASH_MODE), ..)`, and `RulesOf<T>::of(entity)`
answers per room. So the step is: the ladder becomes a scoped rule;
`project_authored_fighter_ladder` reads it for each brain's room; Smash
declares its pack's rows for its mode; the content reload republishes a
scoped declaration, not the resource. Measured 2026-10-08.

✅ Step 1 built (2026-10-13): `AuthoredFighterLadder` is deleted. The ladder is
`DeclaredRules<FighterBrainLadder>`. `ambition_content` declares the shipped
rows for `RulesScope::EveryRoom` (the hosted Smash stage too, as the global
resource did), and its reload restates that one scope
(`DeclaredRules::restate`). `project_authored_fighter_ladder` reads
`RulesOf::room_of` for each brain. Witness:
`each_fighter_takes_the_rung_of_its_own_rooms_ladder` (two sessions, two
ladders). No CPU changes: the mirror grid is identical.

⛔ Step 2 is blocked by a brain defect, not by the plumbing. With the authored
rows declared for `Mode(SMASH_MODE)`, the George mirror in the Smash demo throws
no move: at rung 5 (reaction 300 ms, 18 ticks) the two CPUs approach, cross,
and turn in phase, so no attack is admitted. The same rows with a 325 ms
reaction fight. Fix the resonance first (a reaction that does not lock to the
other CPU's turn), then declare Smash's rows and re-measure the rig.

Traced 2026-10-13 (`AMBITION_FIGHTER_TRACE=1`, the repertoire arm with the
rows declared): no attack is ever admitted. The two seats walk through each
other at the centre, walk about 100 px on (the view is 18 ticks old), turn,
and repeat the same orbit. Admission leads the foe by the relative velocity
over staleness plus startup (about 216 px at 540 px/s). At the turn, the
view's own velocity goes from −97 to +270 px/s between two decisions 5 ticks
apart, so the predicted gap goes from 276 px to −42 px and skips the reach
band. The orbit repeats exactly, so it skips it on each pass; at 20 ticks
the phase moves and a decision lands in the band. Testing the move's whole
active window does not help: the band falls between decisions, not inside a
move. The cause is that the brain leads with its OWN velocity from the
stale view, though it knows what it has pressed since. The fix is an
efference-copy lead (the body's own commands since the view, through the
shadow model), not a retuned rung.

Built and measured 2026-10-15, then reverted: the own-motion lead is not
enough. The perception buffer already holds the live view, so `Perceived`
was given the live self (`own_now`) and `lead_of` carried this body from
where it is now over the startup only, and the foe over staleness plus
startup. With Smash's rows declared for its mode, the George mirror still
started 0 moves in each repertoire arm (900 to 3600 ticks; the same four
tests red as without it). A probe of every admission showed why. At
t=1.483 s the live gap is 65 px and closing at 540 px/s, inside
`excluded_middle`'s band (0 to 43 px plus slack), but the prediction is
131 px: the view still shows the FOE walking away, because it turned less
than 300 ms ago. Five ticks later the view shows its turn and the
prediction is -81 px: they have crossed. In the mirror both bodies turn
together, and the time from a turn to the crossing (about 0.37 s) is about
the reaction time (0.30 s), so the foe's turn is unseen for the whole
approach. The self term was one error; the other is a turn nobody can see
in time. A fix must change what the brain does about a foe it cannot yet
see turning (for example, not overrunning by a reaction's worth of walk
before it turns), which changes every CPU; it is a policy change to
measure on the rig, not a correction of the lead.

✅ Unblocked 2026-10-16: a reaction is a mean, not a constant. At each
decision the brain draws a sample from its own noise stream and
`DelayedPerception::react` sets the view's age to the delay plus up to one
`REACTION_SPREAD`th (one sixth) of it in each direction. The two seats of a
mirror draw from different streams, so the orbit breaks. Witness: with the
shipped rows declared for Smash's mode, the four repertoire tests in
`the_repertoire_gets_used.rs` pass; with the spread poisoned off
(`REACTION_SPREAD = 1000`) the same four fail. Rig, George mirror, 16 paired
seeds, shipped ladder, before (no spread, the stream shifted by the same
draw) and after:

| rung pair | before: bout length, dealt% | after: bout length, dealt% |
|---|---|---|
| 5 v 5 | >480 s on every seed, 0% | 54.5 s, 184% |
| 9 v 9 | >480 s median, 210% | 50.0 s, 254% |
| 1 v 1 | 51.7 s, 105% | 56.8 s, 80% |
| 3 > 1 | 12:4 (p 0.077) | 16:0 (p 3e-5) |
| 5 > 3 | 15:1, >480 s median | 16:0, 45 s |
| 6 > 5 | 15:1 | 14:2 (p 0.004) |
| 9 > 6 | 16:0 | 16:0 |

The stand-in duelist (`smash_duelist_a`, which had no resonance) is the
control, and it pays a cost: its ladder stays ordered in each pair but
separates less, 15:1, 14:2, 11:5, 13:3 before and 14:2, 12:4, 9:7, 12:4
after (53 against 47 of 64 seeds won by the higher rung). Its mirrors kept
their pace (5 v 5: 63 s to 81 s; 9 v 9: 74 s to 90 s). A spread of one or
two ticks at rung 5 was not measured; it could keep more of the order.

⚠ The single-bout mirror grid (`every_fighter_on_the_grid_can_fight_its_mirror`)
cannot measure this change. A control that only shifts the noise stream by one
draw per decision moved single bouts by two to four times (George 279% to
71%, Alice 145% to 199%), as far as the spread did. Read the seeded rig.

⚠ Where the spread is wider than half the decision interval (rungs 1 to 4),
one decision's view can be older than the previous decision's view. No
witness shows harm from this; a monotone variant was not measured.

✅ Step 2 built (2026-10-16): Smash's pack names the ladder file
(`../../ambition_content/assets/data/fighter_brain_ladder.ron`, so the two
packs read one file). The demo declares its rows for `Mode(SMASH_MODE)`, and
a save restates them between sessions
(`EmbeddedPackReloadPlugin::fighter_ladder`). The tools' `--ladder FILE`
replaces those rows (a second declaration of one scope panics). Witnesses:
`the_demo_app_runs_its_fighters_on_smashs_ladder` (compared with the file)
and `the_ladder_restater_states_the_scopes_ladder_again`.

✅ Step 3 built (2026-10-16): `FighterBrainProfile::for_level`, the level
curve, is deleted. A room with no ladder plays
`FighterBrainProfile::engine_default(level)`: one profile at every level (the
old curve's middle: 325 ms, 270 APM, noise 0.275, no rollout); the level is a
label, so a ladder declared later finds the rung. The tests that make a claim
about the rungs a player meets (press-rate order, jitter order, the
evaluation report) read the shipped ladder file; `evaluation::report` takes
the ladder it walks. Witness: `the_engine_default_is_one_profile_at_every_level`.
No shipped composition played the curve: each declares a ladder for its
fighters' rooms. F1 is complete.

Whichever survives:

- every game composition must have one unambiguous source;
- tools print that source;
- fallback behavior cannot turn a failed authored load into a different silent
  experiment.

### F2 — ✅ `read_weight` removed (Q90, 2026-10-08)

See the finding above. Do not keep an authored difficulty knob that looks live
but has no reachable reader.

### F3 — make evaluation rosters representative

For product calibration, use fighters whose kits represent the game being
shipped. If stand-ins remain useful as controls, label them as controls.

Stand-ins may keep incomplete kits (Q89), so they stay controls. When the real
Robot is authored as game content, add it to the representative ladder matrix;
do not treat old stand-in tables as roster evidence.

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
`npc_emmy_noether` already fails the same rung-9 gate at HEAD with the kit change <!-- cite-ok: a character id in the catalog data, not a Rust item -->
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

## Brain policy stays outside combat ownership

The [target logical authorities](architecture.md#target-logical-authorities) treat combat-adjacent
brain code as decision policy over an authored capability menu. A scoring function
can consume combat/action facts without owning damage, capture or live actor
mutation. Do not absorb the fighter brain into a generic combat context to reduce
imports.

A4 preserves the normal accepted-control/action road for both human and CPU input.
A11/A12 ensure the authored techniques and flows that the menu advertises are
actually installed and valid. Selection tests must distinguish a legal but poorly
scored move from content that cannot execute; difficulty should not conceal either
with character-specific scripts.
