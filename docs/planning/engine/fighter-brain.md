# Advanced fighter brain — evaluation, difficulty and regression contract

**State:** the reactive option architecture remains in use. The 2026-11-22
source review identifies a missing recovery execution path (REC1–REC3 below),
beside the existing evaluation and tuning work. A selected move is not proof
that the body executes the forecast or reaches safety.

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

**Seat-term history:** a same-tick mutual grab formerly preferred a low
`SimId`, creating seat bias. Mutual grabs now parry for both bodies (S0.2);
this is guarded in both spawn and message orders. Older ladder numbers
measured before this fix are not baseline evidence for the current combat
rules.

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

**Existing regression coverage:** `two_emmys_are_one_fighter_reflected_through_a_mutual_grab`
compares per-tick reflected state, pose/control and random-stream position;
`the_decision_of_a_reflected_scene_is_the_reflected_decision` tests decision
symmetry; `the_shadow_of_a_reflected_scene_is_the_reflected_shadow` covers
shadow stepping. Specific tests guard supporting-floor tie ordering,
respawn-facing preservation, zero-side `SignumOr` handling, facing-aware
support selection, and symmetric mutual-grab parry. Keep those focused
witnesses; historical poison counts and frame-by-frame debugging live in
Git/test history.

**Remaining symmetry questions:**

- Ordinary fighters currently have per-seat cognition streams; Emmy opts out
  with `preserves_mirror_symmetry`. Decide whether an ordinary per-seat draw
  is an intentionally modelled profile fact or whether all fighters should
  seed by character. Do not add cosmetic randomness just to break a mirror.
- `--paired` exchanges per-seat noise streams, **not** seats, so it does not
  cancel a seat-keyed term. If paired results are used as unbiased estimates,
  change pairing to exchange seats and publish a null control.
- Review the `brain/smash/mod.rs` perch-side fallback: it uses an absolute
  `cross.signum()` away from zero. Check it against a new reflected scene
  before claiming arbitrary-roster mirror correctness. Extend the decision
  and shadow scenes as concrete failures demand.

**Q49 invariant:** mirrored fighters on a symmetric stage produce mirrored
behavior unless a named asymmetric input, history, profile or world fact
breaks symmetry. Each new asymmetric tie-break needs a witnessed rationale.

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
- **A press the body cannot act on is buffered, never spent.** Under
  `hitstun_locks_actions` the input gate eats every attack edge for the whole
  stun, and the emission is open-loop: it charged an APM token and armed
  `own_attack` for a move that never started. `tick_fighter` holds a matured
  press while `reeling::still_reeling` (perceived hitstun, less the view's
  staleness) and emits it on the first actionable tick — the genre's input
  buffer. A decision made inside the stun replaces the buffered press, so what
  comes out is the latest decision's. The start of a stun is still seen
  `reaction` late, and a press maturing in that window is eaten as a person's
  would be. Found 2026-10-29 while chasing the route regression below; measured
  in that match it was latent (no press matured inside a stun), so the guards
  are the two unit tests in `decision/tests.rs`, not the match.
- **The brain dies where the game dies.** `StageView::bounds` is the ROOM box,
  which is what `Recovery`, `EdgeGuard` and the cornering share were measured
  on. The kernel resets a body past the room PLUS the stage's authored margins
  (`World::edges`; Smash: 400 each side, 240 above and below), and that box is
  `WorldView::death_line`. The recovery lens, survival DI and the shadow's KO
  ask it; nothing else does. Measured 2026-10-29, George mirror, 3600 ticks,
  with the lens killing at the room box: one seat spent 364 ticks offstage,
  made 131 `Recovery` decisions that found no route, and 64 of them began from
  a body outside the room, alive, with the platform in view — every route
  refused from a position the game had not lost. Guards: the lens test
  `a_body_past_the_room_but_short_of_the_blast_line_is_searched_alive` and
  `every_authored_route_gets_pressed` (`the_repertoire_gets_used.rs`).
- **"Off the stage" is beside the room or below it, never above it.**
  `StageView::offstage` ignores the room's top: a body launched high over
  the platform is being juggled with its floor still under it. `classify`
  still reads `Recovery` past the death line's rise side (Smash: 240 px
  above the room; a fixture with no margins: the room top — the
  `recovery_above` scenario's premise). Measured
  2026-10-29 (pointed vs pugnacious polygon, 30 s, rung 5): 23% of the
  brawler's 166 `Recovery` decisions and 11% of the pointed polygon's 119
  were above the 480 px room with the platform below, each spending its up
  special to recover onto a stage it was already over. Recovery was 45% and
  32% of ALL decisions in that match; the trace instrument for this is
  `AMBITION_FIGHTER_TRACE=1 smash_cpu_report match-report`.
- **A swing that reaches is shielded; the spot dodge is its mix-up.**
  `options::incoming_attack`: the foe is in a swing's startup or active
  window, faces this body, and `attack_reach` plus 24 px covers the side
  gap. On the ground that offers `Shield` at 0.9 and `Dodge` at 0.85 in
  every situation but `Recovery`; `apply_movement` centres the stick for
  that dodge (a spot dodge), and rolls away for any other swing. A swing
  that cannot reach is nothing to answer — the old "shield when anybody is
  attacking" rule produced two statues and stays only in `Disadvantage`.
- **Neutral spaces to the kit's reach.** `generate_options` hands
  `movement_options` the farthest reach of the ground kit (`Basic`/`Smash`
  candidates). Beyond 1.15× that reach: approach. At it, moving in, with NO
  attack admitted from here: retreat; moving out: approach — the dash-dance,
  read off the body's own velocity rather than a stored phase. The attack
  list is built first and movement asked again with it, because a first cut
  that danced before the list existed stepped out of its own range on the
  decision it could have swung (measured: moves 23/26 → 7/6, damage 51/24 →
  9/0 in the 60 s pair). A kit that reaches nothing keeps the old bias.
  `ShortHop` (a tapped jump; the squat-decided law cuts the rise) is offered
  at 0.45 inside the band and at 0.85/0.7 as the anti-air when the foe is
  airborne above this body; the aerials enter the kit only once airborne.
- **The mix is drawn once per decision from the seat's own stream.**
  `pick_movement_mixed` draws among the eligible options within a margin of
  the best (`0.05 + 0.1 × execution_noise`: rung 9 0.055, rung 1 0.09) —
  the spot dodge beside the shield, the short hop beside the walk-in, which
  are authored 0.05 apart. The margin stays under 0.1 at every rung because
  the scores that are NOT a mix step by 0.1 (Approach over Retreat, Approach
  over the roll); a first cut at 0.14 drew Retreat or the roll half the time
  and cut moves started 23/26 → 7/8. The draw is unconditional (one sample
  every decision), like the jitter's second draw, so the stream is a
  function of the tick count.
- **The kill term is a threshold.** `kill_readiness(damage_frac)` is 0.1
  below 35%, 1 at 70% and a line between (the floor keeps two finishers
  ORDERED by launch against a fresh victim, the launch-law tests' contract,
  and is far too small to pay for a startup); the ladder's `kill_potential` weight
  (0 at rungs 1-2, 0.7 at 5, 1.15 at 9) scales it by rung, which is where
  "smashes at rung 9, none at rung 1" comes from. Measured before it
  (rank trace, rung 5): a smash in the top four three times in 30 s, never
  first, 0.04-0.16 under a tilt, because `damage_frac` from zero never let
  the kill term pay for a 19-frame startup.
- **A landing is a commitment.** `foe_commitment`: a swinging or reeling foe
  owes its clock; a foe coming down owes the time to the floor plus
  `LANDING_LAG_NOMINAL_S` (0.15 s, the genre's aerial landing lag; the
  facet's own number is not perceived). That is the landing read; a roll is
  not perceived as a phase yet and has no read.
- **A charge is bounded by the opening.** `charge::hold_ticks_within`: in
  `Advantage`/`EdgeGuard` the charge runs no longer than the commitment less
  the hold point and no shorter than the partial. The full 90-tick charge
  into a 20-tick recovery released into a shield.
- **The mix is weighted and reaches the attacks.** The runner-up's share of
  the draw is `0.5 × (1 − gap / margin)`: a coin at a dead heat, nothing at
  the margin. A uniform draw inside the margin took the 0.05-under short hop
  every other decision at spacing (35 hops in 30 s, the neutral spent
  airborne). The attack ranking takes a second, unconditional draw with the
  same rule — the grab or the dash attack beside the jab on an approach.
- **Admission was measured before it was touched, and it was not the lever.**
  `OptionSet::admission` (candidates, admitted, nearest miss and where the
  led foe sat) prints as the rank trace's `[admit]` line. Measured
  2026-10-29 (pointed vs pugnacious, rung 5, 30 s): an attack was admitted
  in 6% of Neutral and 6% of Advantage decisions; the refusals missed by a
  median 174 px (7 of 134 within the 24 px slack, 14 within 48), with the
  led foe 160-190 px off vertically and one or both bodies airborne in two
  thirds of them. The admission already leads the foe by relative velocity
  over staleness + startup; no slack or lead fixes a 174 px miss. The lever
  was ground contact: the dash-dance fired whenever nothing was admitted
  (94% of decisions), the band short hop put the body in the air, and a
  cornered body rolled (0.75 over Retreat 0.7) with nothing incoming — 500
  of 3,600 ticks evading. Now the dance and the band hop only face a
  standing, uncommitted foe, and the un-threatened cornered roll sits under
  the walk. `match-report` prints `whiffs` (moves that ended with no
  overlap): 14 of 17 and 14 of 15 in that pair before the change.
- **The lead is ballistic for a foe coming down onto a grounded body, and
  that swing is admitted at any time it is live.** `lead_of` adds `½·g·t²`
  on the down axis for an airborne foe seen from the ground
  (`NOMINAL_GRAVITY_PX_S2`, 1,450; the facet's gravity is not perceived),
  stopped at the floor (the standing offset); admission then samples the
  led foe when the threat opens and at the middle and end of each active
  span — an anti-air is a foe whose arc enters the region before the hitbox
  closes. Every other pairing keeps the straight lead and the one sample: a
  body coming down onto a grounded foe would "land beside it" inside the
  lead and admit a ground poke it cannot throw from the air (the rig's
  `juggle_escape` pressed a jab, and the rig's press rates stopped ordering
  by rung); that case waits for the airborne kit to be asked. Measured
  before it: 70-90% of thrown moves whiffed and the refusals missed by a
  median 174 px with the foe 160-190 px off vertically.
- **Report timing/variance must be accounted for.** Match-report samples
  use per-live-minute rates and repeated runs; raw fixed frame counts after
  an asynchronous route change do not guarantee identical sim-tick histories.
  The Oct 30 investigation found a per-process split in outcomes tied to
  transition frame cadence, not a demonstrated defect in the fighter brain,
  standard hash maps, or the clock easing ramp (the observed `sim_dt` remained
  0.016666668). One of every fifteen host frames could lack a sim tick,
  shifting the phase of a gravity modifier and yielding the first divergence.
  **Open engine measurement:** isolate the bevy_ggrs pacing and frame/timer
  relationship with identical initial conditions and fixed sim tick sequences;
  do not repair it by retuning AI weights. Preserve the tick trace witness
  `AMBITION_MATCH_REPORT_TRACE_TICKS=1` and control for live-start timing.

- **The poke the foe closes through is refused, and a recovery special is
  an attack only with the foe inside its region as it opens.** Admission
  asks, as each active window OPENS, where the led foe is: short of the
  near edge, or somewhere the region does not lie toward (behind the body,
  a closer that crossed through), at every opening is a refusal with no
  slack on that side (`polygon_point`'s whiffs started at a 55 px gap, its
  hits at 89, nearest approach −10 along: the foe was behind the thrust's
  near edge by the time the box was live). The opening, not the window's
  middle or end: a foe inside the box as it opens is a hit however the
  straight lead would carry it on — bodies stop at each other, and a
  running body's own closing is not a pass-through (the rooted-move
  fixture runs at 530 px/s into a foe 60 px ahead and must be offered the
  poke). `match-report` splits `⇒ recovery` out of the whiffs — a way-home
  move started off the stage misses by design — and prints `attack
  whiffs` per live minute beside the whole. A lift (`recovery_route`
  offers a way home) is admitted only when the unled foe is inside its
  region — no lead, no slack — and in Recovery a lift whose region reaches
  the foe is not offered as a route (the ranking's to admit, or nobody's).
  Tests: `a_poke_the_foe_closes_through_is_refused`,
  `a_recovery_special_is_an_attack_only_with_the_foe_inside_its_region`,
  `a_lift_with_the_foe_in_reach_is_not_a_route`.
- **One authority for where a move hits.** `ambition_combat::authored_coverage`
  derives the kit's `coverage` and `reach` by the strike spawn's own
  per-volume rule — the sprite's authored hit polygon for a vfx-tagged
  volume whose clip the sheet names, the RON box otherwise — from the same
  `AuthoredAttackVolumeResolver`, the same character, size, drawn quad and
  art floor; `tick_actor_brains` applies it to every candidate. Witness:
  `the_brain_admits_by_the_geometry_that_hits` presses every move of the
  pair at its first Active window and compares the region the brain is
  handed with the live `Hitbox` extent, to the pixel. Measured (rung 9,
  three runs, per live minute): pair moves 68.8 → 77.0, whiffs 49.4 →
  49.2, damage 78.5 → 141.2, `inside` whiffs 5 → 1; mirror moves 57.2 →
  54.1, whiffs 42.1 → 42.8, damage 70.2 → 99.5. The RON-box
  `[disagree]` ticks fell from 32 to 3 at the pair because the brain no
  longer admits those swings; the remaining whiffs miss by 40-90 px,
  scattered on every side, with the recovery specials still the largest
  rows.
- **A grab that catches has connected.** The capture box is a sustain
  effect, not a strike volume, so no `LandedBodyHit` was ever written for
  it and nothing set the grab's `landed_hit`: a collar that caught,
  pummelled and threw read as a whiff to the report, to the brain's own
  `landed` (which then REFUSED the grab for `refuse_ticks`) and to every
  on-hit cancel. `acquire_captures` marks the use that asked
  (`CaptureAttemptRequested::move_instance`), as the throw already did.
- **A whiff needs a named cause.** `match-report` distinguishes attacks
  from intentional recovery moves and tracks avoided strikes, near approach,
  and authored-versus-resolved coverage. The historical `along=0/across=0`
  median was misinterpreted as an overlapping hitbox refusing a hit. A
  per-tick overlap/permission audit found no unexplained refused contacts in
  the sampled fights; the genuine issue was disagreement between RON boxes
  and the authored sprite hit polygons. The resolved geometry is now shared
  via `AuthoredAttackVolumeResolver` (see the one-authority rule above).
  Future whiff diagnosis must inspect live 2-D geometry and contact causes,
  not infer a refused hit from two independent coordinate medians.

- **The recovery route is legal-now like every attack.** The Recovery list
  skipped `legality`; the route was re-pressed through its own move and the
  helpless fall after it.
- **A hold decides its throw.** `decision::throw_direction`: at kill percent
  (≥ 0.6) toward the nearer blast line, forward or back as the captor faces;
  at set-up percent (≥ 0.3) back when the edge is behind, up otherwise; low,
  down when there is 240 px of floor both ways (a tech chase), up at an edge.
  Forward was the only throw ever thrown before (census 2026-10-29); all four
  are now reachable and the choice is mirrored by construction (Q49).
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

### REC1 — Execute the recovery after the move starts

**State:** IN PROGRESS. A fighter can guide a leased flight mount after a
move that gives it sustained movement authority. The fighter waits for the
reservation, flies toward support, and uses the native lease to release.
Badely's shark flies from the side of the stage to the saddle catch point
before it gives him control. The recovery forecast still uses the authored
reach estimate. The Smash ride test checks entry, boarding, steering, release,
and landing with all stocks from both sides of the stage.

**Priority:** the next recovery-correctness slice when BRAIN/Smash work is
selected. It does not preempt the main Ambition work. Shared observation and
motor adapters belong to the engine; named ride behavior and tuning remain
authored content. General use by another game does not require a second brain.

#### Source chain

| Boundary | Source and original behavior |
| --- | --- |
| Authored move | [Admiral moveset](../../../game/ambition_content/assets/data/movesets/pirate_admiral.ron): `call_the_shark` summons at 0.18 s, ends at 0.34 s and advertises a 5 s, 650 px sustained route |
| Forecast | [`RecoveryLens::carried`](../../../crates/ambition_combat/src/brain/fighter/recovery.rs): move the rider instantly toward nearby geometry by scalar reach, zero velocity, then run a falling-body probe; do not simulate the ride |
| Input | [`MovementVerb::Recover`](../../../crates/ambition_combat/src/brain/fighter/decision.rs): write walking axes and press/hold jump; no flight velocity target |
| Control mode | [Snapshot construction](../../../crates/ambition_platformer2d_actor/src/body/actors/update.rs): `actor_aerial` sees the mount, but Fighter does not use it and the motion law still comes from the rider |
| Motor | [Mount steering](../../../crates/ambition_mount/src/lib.rs) forwards `velocity_target`; [flight integration](../../../crates/ambition_platformer2d_actor/src/body/movement.rs) uses that target instead of walking axes |
| Release | [Native shark status](../../../game/ambition_demo_smash_modules/src/shark.rs): jump or burst aboard produces `DismountWhy::Bailed` |
| Existing witness | [`two_admirals_ride_their_own_sharks_at_the_same_time`](../../../game/ambition_app/tests/smash_ride.rs) stops when both first board. Its comment permits the CPU to be off again within twenty frames |

Preventing the jump edge alone is incomplete: the fighter would still send
no flight target. Reducing shark speed or lease duration would not repair
this execution path.

The native lease starts after boarding and spends room-scaled simulation time.
It is not the move's 0.34 s proper-time clock. The optional summon receipt in
[`answer_ride_summons`](../../../crates/ambition_abilities/src/ride.rs)
means the mount exists, not that the rider boarded or reached safety.
[AC1](native-action-completion.md) owns that construction receipt; do not
redefine it as a successful recovery.

#### Three small owners

| Fact | Owner | Must not duplicate |
| --- | --- | --- |
| Selected attempt | Fighter state: chosen move/binding, intended support or release region, phase and progress needed to continue or replan | Lease, health, movement model or mount relation |
| Controlled motor | Native observation: actual controlled body's identity, center, velocity, frame, law and effective current routing | Another motor or character-specific controller |
| Physical outcome | Existing move, reservation, mount, pose and contact owners | A brain Boolean that claims boarding or recovery |

The rider's pinned velocity is zero even while the mount moves, and its ground
state is invalidated. A flight controller needs the carrier's velocity to
brake. A voluntary release from the accepted saddle pose currently starts without
inherited carrier velocity. The release forecast must also preserve any
native pending launch; the kernel applies that launch after the pose hold
ends. Do not zero it or invent inherited momentum.

Project these as live own-body/owned-relation facts, following Q167. Keep
terrain and foes under their existing perception policy. Retain the selected
attempt across decision intervals and rollback. It is a short execution
record, not a copy of all physical state or a second utility planner.

Static grant is not effective routing. `steer_mount_from_rider` preserves
`CommandedMove` travel and forwards only rider facing even on a mount with
`ControlGrant::Total`. Expose that current native routing decision. A carried
body whose steering is masked is different from a body whose requested
motion meets a wall. Preserve the script's authority; the recovery follower
must wait or replan under its real remaining lease, not fight the script.

#### Execution

| Phase | Next fact required |
| --- | --- |
| Start and await | Observe the actual accepted move identity/occurrence, then reservation/boarding. No ride before the move's summon beat is not failure |
| Follow | Observe the actual carrier and effective routing; emit a world-space flight target through ordinary `ActorControl` |
| Prepare release | Reach a region from which the real rider can regain support/ledge, with time and clearance remaining |
| Release and regain | Observe relation end and the rider's own intended support or ledge; only then report success and resume ordinary combat |

Use [ACT1's non-spending offer](body-action-execution.md#an-offer-and-an-accepted-action-must-name-the-same-move)
for the candidate. Current `ActionLegality::Now` tests playback cancellation
only, and current `OwnMoveFact` names no move definition. The primary move
can be refused or replaced by its authored fallback. An instance change
alone cannot prove the forecast recovery started. Preserve the actual move
identity in observation, follow the supported contract that exists, and
replan when it differs. A missing or changed offer is not permission to
force a private AI move start.

Use the existing
[`follow_flight` and `FlightLaw`](../../../crates/ambition_platformer2d_core/src/navigation.rs)
for bounded following and stopping-distance control. Convert its axes through
the carrier's frame and scale the normalized axes by `FlightLaw.top_speed`
to form the existing world-space velocity target. Let the existing mount
and movement systems execute it. The low-level follower may continue each
tick; new tactical choices still obey the brain's decision policy. Do not
replace the fighter brain when it boards.

While aboard, only the permitted release policy may emit a jump or burst
release edge. A later dodge/tech overlay must not add one accidentally.
Native launch or control loss still wins. Awaiting boarding retains the
ordinary movement that the active move permits; it is not already a ride.

Adapt semantics, not whole input structs:

- The follower can press jump to descend through a one-way surface. Copying
  that edge to this rider would bail again. The first corridor can avoid
  that descent; a later drop-through customer needs a motor-level drop
  distinct from the rider's release gesture.
- A generic interact press is not currently a CPU dismount. Mount selection
  is computed for `DrivingParticipant`. Expose the mechanic's permitted
  release at its native adapter, or initially hold over safe support until
  its existing lease releases it. Do not assume every flyer leaves on jump.
- The current [`FlightMap`](../../../crates/ambition_platformer2d_world/src/navigation/flight.rs)
  covers the room interior. A live offstage fighter may be outside it but
  inside the blast bounds. Use a bounded observed return corridor for this
  customer; a room-grid `NoRoute` is not proof that recovery is impossible.
- Account for both mount and rider clearance, saddle offset and the accepted
  pose. A single merged box can reject a useful route; a mount-only box can
  promise a route the rider cannot pass. Reuse current mount clearance
  authority rather than a new collision solver.

The first connector needs its own check. `FlightMap::route`/`free_near` can
snap to a free cell; `route_to` then prepends the real start without proving
that connector clear. Enlarging the room grid does not make this safe.
Validate the full connector from the observed pose to the first route node
through the existing sweep/admission operation, using the applicable mount
and rider clearance. The runtime rider-clearance adapter can clamp
or detour the desired velocity; observe the motion it actually accepted and
replan on loss of progress. That adapter is not a certificate for a whole
compound route. Its current `rooms.single()` lookup must use the motor's
live room when this shared adapter is corrected; another live room must not
silently remove rider clearance.

Use the existing `route_into` region and the flight follower's low-speed
arrival condition for a release region. Derive the region from the accepted
saddle/rider placement and the native release law, including a pending
launch. A point reached by the carrier is not yet a recovered rider. If the
selected goal is stage support, touching a temporary carrier that will
depart does not complete it. Other authored goals can permit a moving
platform. Keep this target condition in the selected attempt, and report
the actual contact identity where the current probe returns only
`RegainedSupport`/`RegainedLedge`; do not add a global `safe` Boolean.

End or replan on rejected admission, expired/refused reservation, changed or
lost grant/relation, mount death, launch, leaving play, pose refusal, lease
expiry, lost support target, inadequate remaining time or stalled progress.
Read native refusal/dismount reasons. Advance progress/stall budgets only
when the controlled motor receives simulation time; a paused or held motor
is not stuck. Read the native lease rather than keeping another countdown.
Ordinary target-loss/no-hostile early
returns must not discard an active survival execution.

#### Proof and removal

Reuse the real offstage CPU setup in `smash_ride`. Let the shipped brain
supply all input after setup. Continue through board, steer, release, own
support/ledge, retained stock and resumed ordinary action. Mirror the starting
side. Record dismount cause and emitted control on failure. Preserve the
existing first-boarding test as the narrower reservation/identity proof.

Add one carrier-loss or short-lease interruption case to show that the stale
attempt ends and remaining legal recovery can be selected. Do not require a
whole ladder tournament to prove this path.

Replace competing jump/burst emission while an aboard recovery owns those
release edges. Do not infer completion from move end, summon receipt or
reached flight point.
Keep the native relation, lease, control routing and flight law.

### REC2 — Execute the policy the forecast evaluated

**State:** PLANNED follow-up to REC1; no new planner is selected.

The mismatch is broader than the shark. The
[core recovery probe](../../../crates/ambition_platformer2d_core/src/movement/recovery.rs)
returns successful `steps` and `effort`. Fighter consumes only
`regained()` and the route index. The searched drift/jump effort presses
again at the apex; live Recover presses at each decision. A successful search
is evidence for the searched policy, not for a different input sequence.

Keep the selected binding together with the target and executable effort that
the estimate used. Execute that bounded policy through current control.
Anchor its steps to the native acceptance, boarding or release cut selected
by that recovery family. Candidate selection, reaction delay, input buffering
and startup must not silently consume the effort the forecast evaluated.
Replan when current facts invalidate its assumptions. A scalar reach can
remain a cheap estimate; it must not stand for a completed timed ride.

Distinguish these results in the existing recovery API:

- a supported policy found a route under stated inputs;
- a bounded supported search found none;
- this external motion or current state is not modeled.

The last result must not silently veto a legal recovery or pretend to prove
success. A teleport's resolved destination, a burst's effort, a portal pair
and a timed ride have different execution contracts. Start with the mounted
family, then retain the successful kernel effort for an ordinary burst/jump
customer. Resolve additional families through their current owners only when
needed. Do not interpret arbitrary modules in the planner.

Use full production simulation in an offline scenario as the reference for
the bounded forecast. Runtime following stays cheap; it need not simulate a
five-second ride every steering tick.

Replace the shared scalar `carrying => carried` success path for each
migrated family, and remove synthetic equivalence tests that treat teleport
and a five-second ride as the same motion. Use paired forecast/execution
through the same inputs and current movement law.

### REC3 — Preserve live motion inputs and prepare only demanded queries

**State:** source-backed follow-up, no performance result claimed.
Owner of reusable capability/geometry preparation:
[platformer navigation](platformer-navigation-and-reachability.md).

`RecoveryLens::scratch` currently constructs a fresh axis-swept body from
tuning and size. That loses a surface-momentum/crawler law, current maneuver
state, full frame acceleration and base-size distinction. The core probe
already accepts `BodyClusterScratch` and calls the ordinary kernel.
Supply truthful inputs to that API instead of building another simulator.

Geometry must also preserve the subject's meaning. `RecoveryLens::from_view`
currently lowers every `PerceivedSolid` and ignores `open_for_self`. That
field says a gate is passable for this body while it can still block the
body's projectile. Existing floor queries already respect it. The recovery
forecast can therefore land on a gate the real body passes through. Preserve
the existing subject-specific passability when lowering the permitted
perception snapshot; do not copy projectile collision policy or reveal
unseen terrain to make the estimate succeed. Include that meaning in the
existing query's validity key where it can change.

Completed E4 already keys navigation by the actual law, abilities, canonical
and base size, full frame and geometry/exits stamp, reuses standing motion
measurements and retains current graph demand. Reuse that ownership. A
history-free standing experiment is not a snapshot of a falling or hurt body;
share preparation without confusing those two questions.
`NavMotion::measure` currently requires a standing traversal envelope
before measuring flight. Expose its existing flight measurement through that
owner if needed; do not impose the standing requirement on a permanent flyer
or copy the measurement into Fighter.

The current fighter decision also lowers a recovery world before it knows
whether recovery, a candidate survival check or enabled rollout will use it.
Prepare that query lazily. Reuse perceived geometry for its valid lifetime;
replan active flight on changed goal, observed geometry, law/subject or
stalled progress. Do not rebuild a world and search all efforts on every
steering tick. Do not cache by fighter name.

First measure construction/search counts on a normal grounded decision and
an unchanged active flight. For motion fidelity, use one real non-axis-swept
or changed-frame customer and compare forecast with execution. The target is
less unused work and truthful inputs; no FPS gain is asserted.

Use one perceived gate in open and closed states with ordinary ground as a
control. Forecast and production motion must use the same body-passability
meaning. This is a small additional geometry crossing, not a second world
model or a requirement to rebuild every navigation query each tick.

#### Ranged-action query follow-up

**Source customer at `00bf92c22142`, 2026-11-22; planned continuation of
REC3, not a new priority.** The older
[`tick_smash` state machine](../../../crates/ambition_combat/src/brain/smash/mod.rs)
uses `WorldView::line_of_fire` to keep or reject a substituted ranged attack.
[That query](../../../crates/ambition_characters/src/perception.rs) casts a
0.5-half-extent sight probe from the body's root through perceived block
AABBs. It lacks the selected shot's finite material and contact policy.

[Native projectile contact](../../../crates/ambition_platformer2d_shared_tangle/src/projectile/collision.rs)
uses canonical finite material, native block/chain geometry and
`WorldHitPolicy`. An `ExpireOnContact` shot can stop at a one-way surface
which the sight predicate always ignores. A narrow corridor can pass the
sight probe but stop the actual shot. Sharing source AABBs does not make
these questions equivalent.

Keep sight/point occlusion for its declared readers. For this ranged-action
customer, supply the selected native muzzle, material, contact policy and
supported forecast interval to an owner query. Start with a straight
unpartitioned shot. Compare its answer with native execution for the one-way
and finite-width cases. Preserve the intentional distinction between a gate
open for this body and a gate open for its shot.

Use only the terrain admitted by the brain's perception policy. Where that
projection omits a required chain or topology fact, retain an unsupported/
unknown result or extend the permitted projection. Do not read hidden live
terrain to make a forecast exact. Prepare only a candidate that needs this
answer and keep ordinary equivalent straight-shot queries cheap.

Delete the selected customer's claim that a sight ray proves shot passage.
The native projectile/contact foundation remains closed. This is the
action-specific counterpart of REC3's body-passability correction; it does
not reopen tactical intent, all-family rollout or general Smash polish.

### Tactical intent experiment (deferred, not active BRAIN work)

The first measured gate did not pass. The maintainer paused this experiment;
`tactics` stays off in shipped profiles. A later explicit selection should
restart from the failed punish trace in `fighter-tactical-intent.md`, not
activate a production rollout because groundwork or a punish-window census
exists. This is tracked as deferred work in `../tracks.md` and is **not**
part of the active BRAIN acceptance.

### F1 — the brain on the new fighter body (2026-10-18, open)

Every fighter now moves on Smash's default fighter or its own facet body
(`base_body` in `match_rules.ron`, Ultimate's traction, air control, fall
speed and short hop; `docs/related-work/smash-ultimate-fighter-reference.md`).
The brain was tuned on the old wandering-enemy body. Measured with `ko_envelope
selfko`, 3600 ticks each against a passive seat: Robot v3 5, George 4, Alice 3,
Pirate 1, Ninja 0, Duelist A 0, all at 0%. Two emergent tests are ignored with
this as their reason: `the_cpu_reaches_for_more_than_one_move` (George, 4
distinct moves against a floor of 5) and
`the_fighter_brain_engages_rather_than_standing_still` (a CPU crosses the
stage and runs off it inside two seconds). Jon, 2026-10-18: movement for two
humans first, the brain later. Un-ignore both when this is done.

A third joined them when the air dodges took Ultimate's two timings (neutral 52
frames, directional 87, with landing lag): `two_emmys_are_one_fighter_reflected_through_a_mutual_grab`.
The two Emmys stayed exact reflections for 3000 frames, but they never parried
a grab, so the test's window no longer reaches the tie it is about. With the
old 45-frame dodge it passed. The brain dodges as if a dodge were cheap.

### F0 — passive-opponent self-KO investigation (mostly closed)

`ko_envelope selfko FIGHTER [ticks=N]` tests a CPU against a passive seat.
Root causes included ground rolls near ledges, authored attack momentum,
recovery steering, and stale self-perception. Roll/ledge and recovery fixes
are built. Q167/`SELF-PERCEPTION` shipped 2026-10-30: a fighter sees its own
whole `SelfView` live while foe/world views stay delayed. In the recorded
23-fighter-minute census the change reduced self-KOs from 39 to 5; those five
are **not** proof that all per-fighter self-KO behavior is solved. The same
review found Sanic still capable of left-ledge exits; new reproductions belong
with the fighter-body and grounded-attack-momentum acceptance, not with a
reversed Q167 decision. Do not restore the pre-Q167 partial live-kinematics
probe; it mixed live position with delayed grounded facts.

**Remaining actions:** keep passive-opponent self-KO and recovery diagnostics
in the representative roster matrix; revalidate surviving losses on the
current body rules, with fighter-specific traces. The new-body brain task F1
below still owns the two ignored behavior tests and the Noether test whose
old window fails to reach a grab under the new air-dodge timings.

### F1 — the authored ladder is the authority (Q88, 2026-10-03)

**DONE.** `c1254c8e7` declares Smash's authored ladder by room scope;
`f1719f650` supplies one engine default for rooms with no ladder.
Guards: `the_demo_app_runs_its_fighters_on_smashs_ladder`,
`each_fighter_takes_the_rung_of_its_own_rooms_ladder`, and
`the_engine_default_is_one_profile_at_every_level`. Keep failed authored loads explicit.

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
`ambition_platformer2d_actor`, default off; with the feature off the
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
