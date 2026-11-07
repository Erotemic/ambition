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

### Simulation clock — Follow up only with a current reproducer

The old CPU run found that frame-phase-dependent fractional simulation steps
could make identical seeds diverge. The recorded `SimStepPeriod` fix gave six
identical fresh-process runs of the sampled polygon pair. Keep that witness.
`world_freezes_on_hit: false` removed global hit slow-motion in Smash, not in
every other ruleset; other games may still ease a global freeze against wall
time. Revisit that only if a current deterministic/rollback or feel witness
shows a problem. Owner: [simulation authority](../engine/simulation-authority-and-determinism.md).

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
