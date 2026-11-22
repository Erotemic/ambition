# Smash feel — current residuals from the October 29–30 audit

**Owner:** this page holds human-play feel, pose, sound and camera acceptance.
Feature/parity implementation belongs to [the Smash inventory](smash-parity-inventory.md);
CPU policy and engagement to [the fighter brain](../engine/fighter-brain.md) and
[BRAIN](../queue.md#brain--finish-truthful-fighter-attack-selection);
combat rules and authored strike reach to [the combat model](../engine/combat-model.md).
The original audit was read from `2cda058a4` on 2026-10-29. Its intermediate
measurements and superseded R/P implementation diary are in Git history. The
status below records the subsequent October 30 work, **not a fresh claim about
HEAD**. Revalidate affected code, data and subjective feel before changing it.

## November 22 review: complete the action before more cast polish

**Baseline:** `34b4ab0b7f4f5a950a52e6031423e76b5e75f6ac`; renderer
`2b52adb65a3235c6b1f4f9fc4954ee31cc7b2bb8`.
This pass reviewed production code, authored motion and plans. Cargo and the
generated visual payload are absent in the audit environment. No current
match, frame-time result, capture or visual acceptance is claimed.

The S0 feature gate is closed. It did not prove every crossing from input to
reaction to pose. The current reports about abrupt motion and the Admiral's
failed recovery lead to the following owner tasks:

| Question | Current source finding | Plan |
| --- | --- | --- |
| May the body start or continue the action? | Movement-only reaction gates do not govern ordinary move starts; native flinch lacks ordinary playback teardown; landing lag settles after Trigger | [ACT1](../engine/body-action-execution.md#act1--accept-and-interrupt-actions-at-the-owning-cut) |
| What did the attempted action actually select? | AI legality tests playback only; native admission can refuse or choose `when_refused`, while own-move observation lacks the definition | [ACT1 offer](../engine/body-action-execution.md#an-offer-and-an-accepted-action-must-name-the-same-move) |
| Which interval did it execute? | Movement, moves, physical pose and reaction expiry disagree under local holds; zero delta also does not stop every flow/cadence operation | [ACT2](../engine/body-action-execution.md#act2--resolve-body-action-time-once-per-step) |
| Which hit owns the reaction? | Overlap dispatches status callbacks before the native defense result; a later callback can replace the current launch without naming the accepted hit | [CR1 reaction data](../engine/combat-reception-and-injury.md#bind-a-hits-reaction-to-its-accepted-reception) |
| What owns a tech/getup until it ends? | Invulnerability and a one-time velocity stand in for the recovery episode | [ACT3](../engine/body-action-execution.md#act3--give-floor-recovery-its-own-episode) |
| Which event and phase does the picture show? | Reaction identity/loop metadata is lost; charge/pin fallback can discard the selected source's clock or mask; some existing art is not selected | [CLIP1/CLIP2](smash-expressive-fighter-poses.md#clip1--carry-the-beat-through-the-existing-clip-request) |
| Does retiming preserve the drawn contact beat? | Windows are retimed piecewise; the clip uses total-duration normalization, which denotes a different source beat | [CLIP3](smash-expressive-fighter-poses.md#clip3--preserve-source-motion-under-ruleset-retiming) |
| Does a selected recovery actually return the body? | The CPU offers a timed ride as scalar carry, emits walking input and presses the ride's bail edge | [REC1–REC3](../engine/fighter-brain.md#rec1--execute-the-recovery-after-the-move-starts) |

These are source-established boundary gaps. The reported Admiral match has
no trace in this review; its exact dismount cause remains unverified.

**Priority:** the main Ambition work remains first. ACT1/ACT2 and the
Ambition landing/recoil/binding parts of CLIP1/CLIP2 are eligible shared work
in their existing engine/presentation lanes. CR1's native reaction and
CLIP3's reusable source-time binding refine those same owners. ACT3, fighter recovery
execution, cast choreography and demo tuning remain later Smash work.
One engine implementation can serve both games without making the entire
Smash polish pass an Ambition dependency.
[Inventory §17](smash-parity-inventory.md#17-current-implementation-order)
is the only Smash execution order.

### What to inspect before calling an animation missing

Trace the accepted action or reaction, its occurrence and clock, the semantic
pose requested, the published row selected and its actual exit frame.
Then decide whether the defect is mechanics, selection, timing or source art.
A contact sheet cannot show that the game cuts a 492 ms landing clip after a
160 ms request. A row-routing test that manually cancels the old move cannot
prove a real hit performs that cancellation. The owner plans link those
current sources and narrow witnesses.

The Admiral's forward smash demonstrates why more drawing alone is not the
next step. Source motion places its active span at 0.34–0.42 s in a 0.80 s
move. The Smash facet selects 0.30–0.35 s in 0.90 s. At first active, current
normalization requests source phase 0.3333 instead of 0.425. The phase error
is established by code and authored data; frame quantization and the art's
intentional lead require a capture before judging the visible error.
Preserve that source coordinate through preparation before retuning the
artist's anticipation to compensate for it.

Retain the completed reaction anchors, root interpolation, limited carriage
blends, Scout's visual rig/planted hands/follow-through, and E2's prepared
clip/track metadata. This review does not select another interpolation
framework, a complete skeleton conversion or more frames for every fighter.

### A bounded demo acceptance pass, after the shared fixes

Use existing tools and actual matches. Start with a small representative
set: Scout for ordinary Ambition/shared fighter motion; the Admiral for
mounted recovery and a different art family; one Polygon for explicit
reaction phases. Add a fighter only for a distinct failure or visual question.

| Sequence | What the reviewer must be able to see and do |
| --- | --- |
| Hit to launch to floor | An active victim move ends when the reaction interrupts it; local freeze, launch, stun, tumble and landing show the same timing; a second hit restarts its own recoil |
| Tech/getup/roll to action | Contact is immediate, the chosen recovery travels coherently, vulnerability and control return on their authored windows, and the picture reaches the intended exit |
| CPU offstage return | The real brain starts, controls and ends recovery; the rider regains support with its stock. A boarding frame or move start is not the result |
| Short local bout | Both players can read startup, contact, recovery, shield and escape; camera and cues support that reading; another fighter's hit does not stop the stage unless the game asks |

The bounded pass must also distinguish a refused hit from its physical
contact effect, and an unavailable primary move from its accepted fallback.
These are mechanics checks within the selected sequences, not new roster
or tournament gates. The optional strike-pulse history improvement remains
in the combat reservoir; no shipped roster failure was established for it.

Trace mechanical discrepancies first. Then review silhouettes and transition
timing at the actual camera scale, with the selected quality tier and both
facings. Keep impacts and active attacks sharp. Use 60/144 Hz comparison only
to test the touched visual clock, not as a new whole-roster gate.

Only after execution is sound should a before/after move-use census guide
CPU scoring, difficulty or shark power. Preserve the paused tactical-intent
experiment and disabled rollout policy. A short real match and a focused
failed return are more relevant here than making an arbitrary tournament
win-rate threshold green.

The acceptance is a readable, controllable short match on the chosen roster,
with any remaining defect named. Headless checks protect mechanics; human
play and timed captures decide whether the result feels polished. More
roster slots, stages, items or cinematic systems are not prerequisites.

## Built by the end of the recorded feel pass

| Packet | Recorded implemented shape | Remaining acceptance |
|---|---|---|
| R1/R2 | Smash-specific `CombatRules` lock action in hitstun and use affine hitlag; hitstun begins after the freeze. `world_freezes_on_hit: false` keeps global slow-motion out of Smash while per-body freeze carries the impact. | Re-measure hit advantage, presentation sync and human feel after further changes. |
| R3 | Per-fighter move-timing facets for the seated roster; polygon key clocks were adjusted. | Sanic's timing facet was absent from the reviewed rollout; confirm/currently fill if needed. |
| R4/R6 | Flick-to-dash vs walk, crouch, coyote/parry choices, 10-frame buffer, tap-jump setting, short-hop binding, authored walk speed and directional air-dodge lag. | Human controller pass and character-specific movement shapes. |
| P1/P2/P3 | Tiered and directed hit sparks, tiered hit/swing audio, victim shake and flash for the hitlag. | Render/audition on current sprites and published sounds; no one-size-fits-all claim. |
| P4/P5 | Anticipation, smear, overshoot, settle and traveling slash arcs for the three polygon test fighters. | The rest of the cast remains to review; the beast's coil was subtle. |
| P6 | Authored jump/landing/skid puffs, start/turn dust and landing/jump squash. | Human visual acceptance; no base-sprite shadows. |
| P7 | Cast camera with pan/zoom rates, floor margin, camera-box clipping, member-count framing, off-screen bubbles and predicted finishing-blow zoom. Human-seated matches use the cast; render interpolation and the camera follow the same presented poses. | Human review of framing, bubble appearance, zoom rates and multiseat transitions. |
| CPU/determinism | Strike-geometry-aware admission, live self vs delayed foe, recovery and charge fixes; `SimStepPeriod` fixed the sampled cross-process tick disagreement. | Remaining F0 ledge losses and perception-lag whiffs; do not treat old seed counts as current. |

These are implementation receipts, not a declaration that the game already feels
like its reference or that all published character art has been reviewed.

## Remaining work — select by evidence

### R5 — Human play and per-fighter movement shapes

Run real two-human/controller matches on a flat stage (at least
robot/admiral, Alice/goblin and a robot mirror) and review movement, hitlag,
hitstun, attack windows, recovery and camera motion. The October 29 motion
measurement placed base locomotion in the reference range, but it suggested
reviewing the robot's relatively heavy gravity/fall speed, some heavy bodies'
falls, and the authored-vs-observed 3/2-frame jump squat. **Do not retune
`base_body` on this dated comparison alone.** Re-measure the relevant per-body
values with `fighter_motion` and `smash_frame_data.py`, then tune individual
character policy where the human test supports it.

### P4/P5 — Poses and attack coverage across the actual cast

The polygon posing pass is not a cast-wide acceptance. Re-render other
fighters' anticipations, smears, contacts, follow-through and charge tells.
Check that the strike's authored visual arc and hittable region agree; attacks
on a tiny body may correctly extend well beyond the body (Q168 ruling). Do not
shrink the robot slash to match a body-width ratio or reclassify the robot as
a long-range duelist. Check whether other fighters' hit/visual coverage is too
short. Resolve the beast's subtle windup in a visual review, not in an arbitrary
numeric gate. See [combat model](../engine/combat-model.md#authored-attack-reach-and-matchup-behavior-q168-2026-11-06)
and [robot combat feel](../game/robot-combat-feel.md).

### P7 — Camera and off-screen bubble acceptance

Play human-vs-human and human-vs-CPU bouts, including two, three and four
fighters, a fast launch, camera-box exit and a last-stock finishing blow.
Check pan/zoom/floor-share rates (the October 30 first-cut values were pan
6 Hz, zoom open 6 Hz, close 2.5 Hz, floor share 0.15), bubble placement,
its 96 px ring, phone-size readability and a body straddling the fall bound.
The camera study used Melee's `src/melee/cm/camera.c` and Ultimate stage-camera
boxes as references, **not byte-compatible behavior**. Facing-dependent front
and back member extents remain unimplemented and need a demonstrable benefit
before becoming a new feature. A red flash/slow motion on the finishing zoom
was also not built; treat it as optional presentation polish until accepted in
play, not a regression in the implemented finishing-blow trigger.

### CPU / F0 — Residual engagement and recovery

REC1–REC3 above are the current source-backed recovery follow-up. The
October counts below remain historical inputs, not the new diagnosis.

The measured October 30 changes reduced whiff rates and corrected several
self-KO sources, but some Sanic and Stargan left-ledge falls remained. On the
lower difficulty rung, 300 ms old foe motion could also cause dash-dance and
charged haymaker misses. Revalidate against current AI before selecting work:
these are [fighter-brain](../engine/fighter-brain.md) F0 and
[queue BRAIN](../queue.md#brain--finish-truthful-fighter-attack-selection)
inputs, not an independent Smash-feel execution campaign. Distinguish true
stale-view decision mistakes from an opponent changing trajectory after the
last perceived frame. Do not enforce a uniform reach floor or specific duel
win rate (Q168).

### Earlier simulation-clock receipt

The October pass found fractional simulation steps tied to frame phase.
Its recorded `SimStepPeriod` fix gave six identical fresh-process runs of
the sampled Polygon pair. Preserve that witness; revisit that old diagnosis
only with a current reproducer.

The authored `world_freezes_on_hit: false` policy selects local hitlag for
Smash. It does not prove that every clock consumer obeys it. The November 22
ACT2 source review found the separate primary-body global-freeze shortcut
and the local movement/playback mismatch. ACT2 is already selected and does
not wait for another selection of the old determinism task. The time owner
remains [simulation authority](../engine/simulation-authority-and-determinism.md).

## Focused evidence and acceptance

The original October comparison found very short aerials, insufficient landing
lag, weak freeze, generic hit feedback, and a camera following the primary
avatar instead of a declared cast. The recorded fixes above supersede those
claims as live outstanding tasks. For every touched behavior, test the
*current* production composition rather than reinstalling a finished packet.

| Question | Instrument or witness |
|---|---|
| Do tilt and aerial connects leave honest advantage? | `ko_envelope feel` at 0%, 50% and 100%, after a representative on-hit trace |
| Do jab/smash hits freeze the fighters for the intended beats? | `moveset_takes` with per-tick hitlag and presented flash |
| Do aerials have meaningful active/recovery/landing windows? | `scripts/measure/smash_frame_data.py`; roster-specific exceptions must be intentional |
| Does a slow push walk, a flick dash and a crouch remain usable? | `fighter_motion` and two controller layouts |
| Are effects, body pose and strike geometry visually aligned? | `moveset_render --overlay art` and actual shipped-game match capture |
| Does the camera follow a declared cast with a human seated? | `match-report --camera`, `camera_scenarios` and `a_declared_cast_is_framed_even_when_a_primary_player_is_seated` |
| Do representative CPU fighters recover and engage? | multi-seed `match-report`, real stock-loss and ledge-specific witnesses; compare F0/BRAIN |

**Human acceptance cannot be inferred from a green headless test.** A good
reference feel is not a byte match to Ultimate. Preserve authored character
differences, player readability and the ability of a hit to register visually,
mechanically and audibly. All numerical observations here are dated starting
points; do not turn them into global acceptance thresholds without play.
