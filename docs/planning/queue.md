# The queue — live execution order

This file is the executable current-work ledger. It is intentionally
self-replenishing: when the listed rows are exhausted, verify the highest-value
item in [`tracks.md`](tracks.md) against HEAD, promote it here, and continue.

There is **one row shape**:

```text
- ▢ **ID — current problem.** Current evidence, next action, acceptance.
```

`▢` appears only on executable rows. Completed investigations do not stay here;
use git history. A focused plan owns technical design and should be linked rather
than copied into this file.

Before implementing a row, re-check the named gap against current source/tests.
Direct new maintainer observations outrank this ordering when they are
reproducible.

**Reviewed baseline:** `4e5f59cf753a62105cbc9fd53aa9697d337d0eed`.

## Recent structural receipts

✔ **D-RECONSTITUTION — the same-room replay was a second room constructor.**
`reset_ecs_room_features` mutated twelve families of surviving entity back
toward a presumed spawn state through a hand-kept list. Measured divergence: a
replayed enemy came back facing the wrong way, 34.6px from where a fresh entry
puts it. A replay now records a lifecycle intent for the ACTIVE room and the
room-transition road rebuilds it, so new session / transition / replay /
new-game reset differ only in target room, retire filter and durable-fact
policy. Two defects fell out: a boss's persisted defeat was re-derived from its
corpse every frame (so a retraction was overwritten on the next), and a
harness-staged scenario actor had no reconstruction record (it survived a reset
and not a door). Guarded by `canonical_reconstitution.rs`. Owner:
[`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

✔ **D-SESSION-OWNERSHIP — cross-game rollback health contamination.** Fixed by
`26ec7b19`: rollback authority is owned by `SessionScopeId`, health carries only
across timelines of that same gameplay session, foreign-session confirmation is
`Unavailable`, and session-mirrored resources are re-established on activation.
Guarded by shell lifecycle and session-ownership tests. Durable rule: ADR 0027.

✔ **D-TRAP-HOLD — "any action press" meant Attack or Special, and nothing else.**
`ChargeSustain::UntilPressedAgain`'s comment has long said the Performer ends her
trapdoor beat by *"pressing a non-move action"*, movement excluded. The check
read the RESOLVED ATTACK GESTURE — which carries Attack and Special and nothing
else — so **the component it consulted could not express the rule it was
asserting**, and a grab, taunt, projectile or Interact left the freeze running.
⭐ THE RULE IS NAMED ONCE NOW, as
`ActorControlFrame::action_press_that_is_not_movement` on the body-semantic frame
that CAN express it, so the next customer asks the same question instead of
spelling a second verb list. Six verbs in, seven ways of moving out — and
traversal (blink, pogo, flight toggle) counts as movement, because a player
steering under the stage must not end the beat by steering. ⚠ SHIELD CANNOT
CONTRIBUTE: the frame carries `shield_held` and no shield edge, which is a
vocabulary gap and is recorded where the list is. Five arms: four verbs that must
end it, red under the old attack/special check; and one contrast — steering,
jump, dash, fast-fall, blink, pogo, flight toggle — red under an over-corrected
predicate that counts movement, which is the other direction the fix could have
failed in. `trap_probe`'s stale comments now say what the condition reads and
warn that the probe prints a SUBSET of it.

✔ **D-CONTROL-INTERACT — the press-gated WORLD verbs are per driven body now.**
`open_ecs_chests`, `interact_ecs_actors_and_switches`, `heal_save_shrine_system`
and `regen_player_mana` each resolved one `ControlledSubject`, so a couch's
second seat could stand on a chest, a switch or a shrine and press interact
forever. ⭐ THE GESTURE HALF WAS ALREADY RIGHT — `ActingParticipant` keys the
buffered interact off the body's OWN driving slot — so only the subject was
singular, and the conversion is the same one D-CONTROL-ITEM made nine times.
⛔ THE SWITCH LOOP'S `return` ENDED THE SYSTEM: "once we flip one we stop" is
right PER BODY and wrong for the population, so seat a flipping its switch
stopped seat b flipping a different one. It is a `break` now. ⭐ DIALOGUE KEEPS
ITS `return`, because a conversation is a GLOBAL mode flip and two bodies cannot
both open one on a tick — the right scope, stated where it is. ⚠ THE SHRINE'S
CHECKPOINT IS ONE FACT AND ITS HEAL IS N: every resting body heals, and the
checkpoint is written by the first body in the rewind-stable driven order that
rests, so the value does not depend on query order. Four guards, all red under
the `DrivenBodies` poison. The portal gun is deliberately NOT converted and the
reason is stated in `fire_adapter.rs`: see D-PORTAL-GESTURE-SEAT. Presentation
readers (the portal eye, the drawn gun, sim-view facts, the camera) stay singular
because a view has one viewpoint.

✔ **D199 — the projectile's solid test is swept and policy-aware now.** Of the
row's three asks, two were already closed (victim ordering and wall occlusion are
guarded; the swept-versus-hurt-volume half is deliberately deferred behind an
EXECUTABLE measurement, `projectile_speed_stays_under_the_swept_threshold.rs`).
The first ask is done: `step_projectiles`' anti-tunnelling step cast
`raycast_solids` along the shot's CENTRE LINE, so a box clipping a block's CORNER
— centre line beside it, endpoint clear — was a hit nothing in the road could
see. It is `ae::cast::body_sweep` now, the engine's one body-vs-world swept entry
point, which was already sitting beside the raycast unused by this caller. ⭐ AND
THE CAST WAS POLICY-BLIND: `include_one_way = false` unconditionally, which is
right for a `Bouncing` shot (a fireball crosses a one-way from below by design)
and wrong for `ExpireOnContact`, whose contract is that ANY solid / blink-wall /
one-way contact is expiry — so a fast straight shot flew through a platform its
own policy says should have killed it. The predicate asks the shot. Three arms:
the corner clip and the one-way tunnel, each red under the poison for ITS OWN
half (a point-sized sweep; a policy-blind predicate), plus a contrast case — a
bouncing shot flying ALONG a one-way still passes through — green before and
after, which is what keeps the fix from becoming "one-ways stop everything".
⚠ The snap lands a hair INSIDE the hit block rather than exactly tangent, because
`time_of_impact` puts the box merely touching and every policy below reads
`strict_intersects`, which is false for a touch. ⚠ The guard for the OTHER input
to the deferred inequality — a hurt volume authored below ~11px — still does not
exist and still cannot be written from authored data; that stays recorded in the
deferral test's own header.

✔ **D-RESTORE-SIM-LOCAL — the once-per-session resume memory was two `Local`s on
a SIM system.** `restore_checkpoint_on_session_start` runs in `PlayerSimulation`,
and a `Local` does not rewind: a rollback crossing the frame it routed on would
resimulate with the memory already past the crossing, so one timeline asks for
the crossing and the other believes it already did. Both memories are one
registered resource now, `CheckpointResumeProgress`, probed by WHICH GENERATION
rather than by presence — the coverage oracle refused a presence probe, correctly,
because the value IS the decision. `GGRS_ROLLBACK_SCHEMA_VERSION` 141 → 142.
⚠ MEASURED UNPINNED, AND SAID SO. `a_cross_room_checkpoint_resume_stays_checksum_clean`
is green with the memory poisoned back to `Local`s: a confirmed transition rebases
GGRS onto a new frame zero, so no rewind crosses the routing frame and the
divergence is unreachable today. The move is still right — a correctness that
holds only because some other layer rebases moves when the rebase does — but the
test doc says plainly what it does and does not cover, the same way the Mary-O
room memory's does. What the arm DOES pin is that a cross-room resume under a
live sync-test session stays checksum-clean at all, with a premise assert that it
actually crossed.

✔ **D-RESTORE-LEDGER-SCOPE — the ledger is session-scoped now, and the row was
WRONG about the consequence.** ⛔ THE ROW WAS WRITTEN OFF A SCHEDULE READING AND
NOT MEASURED, and its stated acceptance — "two sessions in one process, the
second with an empty save, and the second does not see the first's rows" —
ALREADY HELD at HEAD. Measured 2026-08-31 by poisoning all four resets: the
post-relaunch arm stayed green, because retirement clears `SaveRestored`, session
B re-runs its restore, and `adopt_rows` REPLACES rather than merges — an empty
file empties all four. `retirement_clears_the_save_applied_latch` documents
exactly that and was right. ⭐ WHAT WAS ACTUALLY WRONG is narrower and is fixed:
between retirement and B's restore, `AuthoredOccurrences`, `OccurrenceBaseline`,
`CustodyBaseline` and `MintedItemBaseline` still described a world that no longer
exists, and none of them was declared session-scoped. All four are now in
`SessionScopedResources`, so the correctness stops resting on the SAVE road
running at all — which matters because
`adopt_the_occurrence_ledger_at_activation` runs `.after` this reset, making the
two an ordered pair rather than two opinions. Guarded at both edges
(`session/teardown/tests.rs`) and at the launcher window
(`session_isolation.rs`), each of the four red when poisoned alone. ⚠ The
post-relaunch assertion is kept for the contract it states and is explicitly
marked as NOT attributable to this change.

✔ **D-RESTORE-COLLISION — the two checkpoint roads collide, and it is benign.**
`restore_checkpoint_on_session_start` records a transition to the checkpoint's
room on the first tick a body exists; `resume_at_checkpoint_on_reset` records one
too, from the `ResetToCheckpoint` the durable chain writes. The slot is
earliest-sticky, so the second gets `AlreadyPending`, writes no
`RoomReplayAdmitted`, and the reset message is drained either way. ⭐ MEASURED:
both roads ask for the SAME destination and arrival, and the only thing the loser
drops is the replay announcement, whose consumer is the attempt-residue sweep —
and a session start has no previous attempt. Poisoning EITHER road alone leaves
the arm green; poisoning BOTH reddens it, which is what makes
`a_save_with_a_checkpoint_and_an_occurrence_lands_both` a guard of the end-to-end
contract rather than a check that cannot fail. ⛔ The first version of that arm
was VACUOUS and all three poisons passed: it relocated an object of the room the
session OPENS in, which the destination never authors, so the suppression was
guaranteed by arithmetic. The row now relocates one of the RESUME room's own
objects. ⚠ NOT closed by this: the two `Local`s — see D-RESTORE-SIM-LOCAL.

✔ **D-RESTORE-INTERIM — a load builds its first room right, instead of building
it wrong and correcting it.** Activation passed `continuity: None` with the
comment *"there is no earlier occurrence of anything to have a disposition
yet"* — true for a fresh session, false for a LOAD, whose file has been on disk
since before the process started. So a room whose object the save says is lying
next door authored it anyway; the durable chain then ran in `Update`, latched,
asked for a checkpoint resume, and the room-transition road rebuilt the room
several ticks later with the ledger in hand. Measured: two live things behind
one identity for two frames of a real startup load — in a window where combat,
pickups and encounters all run ungated on `SaveRestored`, so the body that
picked one of them up wrote its custody over the very row the correction was
about to read. ⭐ THE LEDGER LEG NEEDS NO BODY, which is why it could move: only
the item/wallet leg of the durable chain requires a primary body, so ledger
adoption now runs at `SessionScopeSet::Activate` — the seam whose stated promise
is "before any provider constructs the world these values describe" — and
activation passes the real `OccurrenceContinuity`. The temporary population is
never built. Guarded by
`a_load_never_authors_the_occurrence_it_is_about_to_suppress`, which samples
EVERY frame rather than the ends (both endpoints answered "absent" while the
middle answered "present" — the shape a two-endpoint test cannot see), red under
each leg poisoned separately. ⭐ It needed a harness that boots WITH a save the
way the binary does: `Platformer2dSimHarnessOptions::with_save`, inserting the
file before the first update. Writing a save into a RUNNING session can only
measure the correction road. The save types moved onto the SDK's `session`
surface for it — the durable state a session resumes from is a session concept —
which is the gap `sim-harness-names-only-the-public-sdk` predicts by name.
⚠ STILL OPEN, and now its own row: D-RESTORE-COLLISION.

✔ **D-INPUT-RECORDER — the input recorder was quadratic rollback state.**
`InputStreamRecorder` was `rollback_resource_clone`, so every GGRS save cloned
the WHOLE recorded input history and saving frame N cost N. It was registered
for a reason: `InputStream::push` was append-only, so a resimulated tick
recorded itself a second time and only the restore kept the stream contiguous.
⭐ `push` IS TICK-ADDRESSED NOW — re-recording a tick the stream has already
passed discards the abandoned tail and rewrites from there, which is what a
rewind MEANS. The recorder reproduces its own correct state from the
resimulation, so it is `declare_rollback_derived_resource` and carries no
snapshot bytes at all; the confirmed prefix, which is the part that grows, never
moves. Guarded by `input_stream_under_rollback.rs`: the same script through an
eager host and a `check_distance: 4` sync-test host produces the same recording
(RED by poisoning `push` back to append-only), plus an arm asserting the
registration is `Derived` — because restoring the stream is ALSO a way to keep
it contiguous, just the expensive way, and the behavioural test alone could not
tell them apart. ⚠ The two hosts do not start on the same `SimTick`, so the
comparison is on the recorded SEQUENCE, not absolute tick numbers.
`GGRS_ROLLBACK_SCHEMA_VERSION` 140 → 141: a peer that snapshots the recorder
and one that does not cannot agree, so removing bytes is still a wire change.

✔ **D-CONTROL-ITEM — make held ranged/custom item actions per driven body.**
Nine held-item abilities read `Res<ControlledSubject>` — one entity by
construction — so a couch's second seat and a possessed body never acted:
`ranged/{volley, meteor, beam, vortex}`, `thrown/puppy_slug_gun` and
`traversal/{grapple, dive, blink, mark_recall}`. All nine now loop
`DrivenBodies` (the possessed subject ∪ seated bodies, ordered by `SimId`), and
every per-body exit is a `continue`. The held bolt carries
`ProjectileOwner(firer)` — the same rollback-registered, entity-remapped
component the ECS projectile road uses — so `held_projectile_step` credits a
hit to whoever fired it instead of `Query<Entity, PrimaryPlayerOnly>`. Looping
exposed one new defect and it is fixed: the summon cap counted a query, which
cannot see this tick's `Commands` spawns, so N seats firing together each read
the same pre-tick count and every one of them summoned. Ten guards, each proven
RED by poisoning `DrivenBodies::entities()` back to the single subject (9) and
the bolt's attacker back to the primary slot (1). NOT in scope and NOT changed:
presentation readers (the portal eye, the drawn gun, control prompts, camera)
where one viewpoint is correct — see D-CONTROL-INTERACT for the rest.

✔ **D-PORTAL-POLICY — the portal map convention is a threaded value, not a
process global.** `PORTAL_MAP_ROTATION: AtomicBool` in `shared_tangle::math` is
deleted along with the per-frame system that mirrored `PortalTuning::convention`
into it. Every consumer takes `MapConvention` as a parameter now, resolved from
the tuning at its own system boundary — about forty call sites across the pure
math, the piece layer, view cones, the presentation rigs, the host camera
continuity, projectile transit and the content adapters. ⭐ THE TWO DEFAULTS
DISAGREED: the static defaulted to Reflection and `PortalTuning::default()` says
Rotation, reconciled only by the mirror system — so every fixture that never ran
it silently played under a different convention than production, and one test's
anti-vacuity assertion was resting on exactly that. Guarded by
`two_sessions_in_one_process_keep_their_own_portal_conventions` (the acceptance,
run in both orders) and by `engine.platformer-math-holds-no-process-state`,
which forbids the shape returning and was verified red.

✔ **D-SIM-SELECT — the last two selection sites now break ties on identity.**
The row named three; one was already closed (projectile-victim ties carry a
stable `(distance, x, y)` key). The two live ones were bare
`min_by(total_cmp)` on distance: possession candidates and the pickup magnet.
Both go through `sim_selection::winner_by(metric, SimId)` now — nearest first,
identity last — and both carry a spawn-order arm modelled on the projectile
tie-break's: the same two bodies spawned left-then-right and right-then-left,
with the identities fixed to POSITION rather than to the spawn slot, so "the
same winner" means the same body and not the same index. Each verified red by
poisoning its identity function to `None`, which is the exact shape of having no
tie-break at all.

✔ **D-REPLAY-RESIDUAL — the dead intent variants are gone and one listener was
measured redundant.** `LifecycleIntent` carried `DeathReset`, `ManualReset`,
`Replay` and `FullReset`; nothing recorded any of them and a stray one would
have returned `CommitOutcome::Retry` forever — a silent stall wearing an
exhaustive match's clothes. Deleted, with their codec branches; tags 0/1/2/4 now
refuse to decode. ⛔ THE READABLE SCHEMA DUMP COULD NOT SEE THAT: same stable
name, same encoder type, same projection, so every wire ledger stayed green
while the encoding changed. `GGRS_ROLLBACK_SCHEMA_VERSION` is what that class of
change is for — 139 → 140. Mary-O's `reset_snakes_on_room_reset` was deleted,
but only after a cover test was written: gutting it left all 41 Mary-O tests
green, which is evidence that nothing covered it rather than evidence of
redundancy. Gravity's `reset_gravity_on_room_reset` STAYS — `BaseGravity` is a
session-scoped resource and no room rebuild touches a resource. Two legs remain
unpinned and their tests say so: `return_the_replay_subject_to_spawn`'s use of
the admitted subject, and `follow_the_active_room`'s room memory.

✔ **D-RESTORE-FACTS — measured, and the SHAPE is provisionally kept.** A save
load builds its first room with no occurrence continuity and the file's facts
then correct it. Measured: the correction lands on the same authoritative
population an in-session re-entry produces, both for an empty file and for a
file carrying a relocated occurrence (`canonical_reconstitution.rs` cases 7-8,
both red under a poisoned `outlook_for`). ⚠ That was EVENTUAL convergence, and
the interim population it left unproven is now gone: D-RESTORE-INTERIM moved
ledger adoption to the activation edge, so the first room is built right rather
than built and repaired.
`ResetToCheckpoint`'s contract, which claimed it was "not a save load", is
reconciled.

✔ **D-SIM-LOCAL — the level's departure memory was three Bevy `Local`s.** The
row named Mary-O's `follow_the_active_room` room memory; the localizer named a
different system. `cycle_level_on_flag_tally` accumulated its dwell timer on the
SIM clock in a `Local<f32>`, and a Bevy local does not rewind — so a goal-pole
slide desynced the demo's own sync-test host at frames 92-93 on exactly the two
checksummed types the threshold gates (`MaryOLevelState`,
`PendingLifecycleCommit`). All three memories (dwell, the room the level asked
for, the room the mode last saw) now ride `LevelDeparture` on the mode owner,
registered and value-probed beside `MaryOLevelState`. ⚠ Only the dwell half is
PINNED: putting a `Local` back in `follow_the_active_room` leaves the new test
green, because a confirmed transition rebases GGRS and no rewind crosses the
commit frame. The move is still right — a correctness that holds only because
another layer rebases moves when the rebase does — and the test says so rather
than implying a guard it does not have. The row's quest/room-visit clause was
NOT a defect: it is a documented, checksum-guarded tradeoff
(`ambition_persistence/src/quest/registry.rs`).

✔ **D-POLICY-1 — `ambition_workspace_policy` is green, 35/35, with no bulk
waiver.** Twelve failures in five groups, each triaged to whether the ownership
rule had changed. Four policies described the SHAPE of the dead
`platformer_runtime` compat shim, whose own TODO asked for its deletion once
callers migrated — deleted, and replaced by the rule they stood in for
(`engine.no-generic-runtime-facade-in-the-actor-crate`). One pinned
`run_if(gameplay_allowed)` after the schedule replaced that repeated predicate
with the `GameplayGated` SET — a red for a change that made the gating
stronger. Three runtime dependencies the runtime already schedules
(`ambition_damage`, `ambition_mount`, `ambition_gameplay_trace`) were missing
from its allowlist. Four pose/velocity write sites were the ADR 0024 authority
forms the rule already sanctions elsewhere, in files never listed; one
(`officer_probe`) was a real bare write and now goes through `transit_body` via
a new `probe_stage::place`. `ambition_demo_smash/src/lib.rs` came under the size
limit by moving its 1640 lines of inline tests into sibling `tests.rs` files —
`module_size.toml` still has zero waivers. The velocity rule gained the poison
self-test its pose twin has always had: it had nineteen waivers and nothing
proving it still fired.

The one unresolved developer-policy choice from the session-ownership work is in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §37.

## Current execution order

- ▢ **D-PORTAL-GESTURE-SEAT — the portal fire gesture carries no seat.**
  `FirePortalGun` carries an aim and nothing else, so `resolve_portal_fire_intent`
  cannot be made per-body the way the other press-gated verbs were: a resolver
  looping every driven body would have to guess whose press it was, and would
  fire N shots for one. The change belongs to the GESTURE and to the input
  adapter that writes it (`game/ambition_content/src/portal/input_adapter.rs`),
  not to the resolver. Acceptance: the gesture names the seat that made it, and
  two seats each holding a portal gun place their own portals from one tick's
  presses. Owner:
  [`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md).

- ▢ **D-SHRINE-CHECKPOINT-OWNER — a comment states a rule the code does not
  follow.** `heal_save_shrine_system` says the checkpoint is written "for the
  PRIMARY player's session, not the possessed subject's body" and then writes the
  RESTING body's `kin.pos`. Its consumer, `restore_checkpoint_on_session_start`,
  places the PRIMARY avatar there — so a checkpoint taken while possessing
  resumes the avatar somewhere it never stood. Read from the source, NOT driven;
  no test covers the possession case either way. Acceptance: an arm that rests
  while possessing and says where the next session starts, then whichever of the
  two rules is chosen, with the other's comment deleted. ⚠ It is a
  save-compatibility ruling, which is why the multi-seat conversion preserved
  today's behaviour rather than deciding it. Owner:
  [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

- ▢ **D-FIGHTER-L6 — diagnose the confirmed rollout regression with a decision
  trace, not another sweep.** The controlled A/B already established the signal:
  rollout-enabled level 6 fails the cited recovery scenario 45/45 while disabled
  succeeds 45/45; level 1 is unaffected and RecoveryLens did not fix it. Trace
  one fixed seed through option generation, rollout vetoes, suicidal-movement /
  support recovery reasoning, least-bad selection and final choice. Fix the
  first wrong authority/decision exposed by the trace. Owner:
  [`engine/fighter-brain.md`](engine/fighter-brain.md).

- ▢ **D72 — continue Super Smash Siblings as a product/engine customer from the
  current parity inventory.** Do not resurrect the historical fun-push campaign.
  Re-read [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)
  and current maintainer observations before choosing the next slice. Prefer
  mechanics/readability/control defects that expose reusable engine seams over
  broad polish. Explicitly keep already-settled genre decisions and shipped
  mechanics from being reimplemented.

- ▢ **D-RASTER-3 — split the weak-GPU improvement between framebuffer scale and
  MSAA.** The valid matched result is **51.045 ms -> 20.101 ms p50, about 2.54x**;
  both DPI/framebuffer cap and MSAA changed together. Run an interleaved A/B on
  real weak GPU hardware with the independent `AMBITION_MAX_SCALE_FACTOR` and
  `AMBITION_MSAA` knobs, multiple reps per arm, holding build/features/profile
  constant. Do not substitute lavapipe/software rendering. Owner:
  [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

- ▢ **D33 — continue actor-monolith decomposition by coherent ownership.** Pick a
  carve that removes a real authority/dependency edge from the residual actor
  kernel, moves registration/tests with the domain, and improves capability or
  compile/test isolation. Do not carve by LOC and do not promise frame-time
  improvement without a measurement. Owner:
  [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

- ▢ **D166 — make the character-authoring boundary load-bearing where a real
  character still bypasses it.** Prepared character definitions are already
  immutable and the first Smash fighter facet exists. Re-measure the current
  residuals before migrating another field. The startup-reach proxy is a
  maintainer decision (§35), not an excuse to widen generic character data.
  Owner:
  [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

- ▢ **D129 — repair player-visible sprite clipping through authored geometry,
  using the existing build-time guard.** The historical "52 of 196" count is a
  stale snapshot; the later full render census also moved as art was repaired.
  Re-run the current target, start with player-visible/selectable characters that
  still fail, and fix the authored canvas/pose/geometry rather than weakening the
  guard. Do not infer a roster-wide scale rule from one character's repair.

## External measurements / human-gated work

These are live but should not cause an autonomous agent to invent data or a
product ruling.

- **Switch Pro outer range:** run `Shift+F6` on both machines, push the controller
  to each extreme/corner and compare peak axis magnitude. Only then decide
  whether shared outer saturation is needed.
- **Character/product decisions:** see
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), including
  the proof-pulse lifetime, character heights, fighter reach/tumble policy,
  ranged-recharge presentation, persistent foreign-room actor placement and
  dormant windbox/armor customers.
- **Rendered external-consumer/platform checks:** keep in
  [`tracks.md`](tracks.md) until the necessary GPU/toolchain is available; do not
  report host-prerequisite absence as an engine defect.

## Replenishment rule

When these rows thin out:

1. inspect [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
   for direct untriaged reports;
2. inspect [`tracks.md`](tracks.md) in its stated replenishment order;
3. re-measure the candidate against HEAD;
4. promote one concrete executable slice here with a focused owner and
   acceptance criterion;
5. keep going.

Do not recreate a staffing table or a second review-status ledger. If a task is
not executable now, it belongs in tracks or the maintainer-decision file rather
than hidden inside a closed queue narrative.
