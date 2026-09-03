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

✔ **D-SHRINE-CHECKPOINT-OWNER — the code's rule is real, the comment's was not.**
`heal_save_shrine_system` said the checkpoint was written *"for the PRIMARY
player's session, not the possessed subject's body"* and wrote `kin.pos` — the
RESTING body's. The two disagree only under possession, and no test covered it,
so neither claim was measured. ⭐ MEASURED NOW, and the CODE's rule is kept: a
vessel resting at a shrine records the vessel's position, and *"I rested here, I
come back here"* is what a player means by a checkpoint — the body they were
wearing at the time is not part of the promise. Implementing the comment would
have meant a shrine touched while possessing silently records a position the
player never stood at. Guarded by
`the_checkpoint_records_where_the_resting_body_stood` (an avatar at (50,900) that
never touches the shrine, a driven vessel resting at (700,100)), red the moment
the write reads the avatar. Both stale comments are replaced with what the code
does and why.

✔ **D-FIGHTER-L1 + D-BRAIN-PLATFORM-FLOOR — one defect, and neither half could
land alone.** The smash demo's respawn platform was REBUILT EVERY TICK at the
protected fighter's own position (`hold_the_respawn_platforms`), while its own
comment called it *"Stationary: a sweep of zero width at zero speed"* — the sweep
was zero and the centre was not. And it reaches the collision world as a
`BlinkWall`, which `WorldView::supporting_floor`, `floor_below` and
`ground_below` each excluded by spelling `Solid | OneWay` inline. So a fighter
standing on it perceived NO floor (`ground=true supported=false
floor_edge=None`), and every ledge question in the brain — `floor_ahead`, the
walks-off penalty, the recovery route search — read through the false one.
⛔⛔ EITHER FIX ALONE MAKES IT WORSE. Making the block standable while the
platform still followed the body handed the rollout a floor defined as *"wherever
I am"*, whose perceived edge is a **constant 48.0 across 200px of travel** — every
verb judged to walk off, everything vetoed every tick, and l6 back to `unfought
1/1`, its exact pre-fix numbers (measured twice, reverted twice). ⭐ TOGETHER:
**every rung of `--sweep-below` now fights at 45 seeds, `unfought` gone from all
five, and l1 goes 45/45 → 0/45** — which nothing had ever moved, not rollout and
not reaction time. The perceived edge now CHANGES as the body walks (44 → 21 →
−9 past the lip → 226 on reaching the stage), which is the acceptance the row
asked for. Recorded in `dev/ambition_dev_measurements` (`8165029`). Guards:
`the_respawn_platform_stays_where_it_was_placed`, red on the follow. ⚠ A second
smell, unchased: `terrain` is 1-2 solids on a stage that has more.

✔ **D-PORTAL-GESTURE-SEAT — every gun gesture names its body now.**
`FirePortalGun` carried an aim and nothing else, so the adapter resolved one
`ControlledSubject` and the resolver re-derived the firer the same way: a second
seat holding a portal gun made a press that reached nothing, and a resolver that
had simply looped driven bodies would have had to GUESS whose press it was and
fired one shot per body for one press. ⭐ THE FIX IS IN THE GESTURE, as the row
said: all four intents — fire, toggle, drop, pickup — carry `body: Entity`, the
adapter emits one per driven body ordered by `SimId`, and each consumer acts on
the body named rather than on a subject it re-derives. ⛔ `portal_toggle_system`
was `guns.single_mut()`, which is a claim that exactly one gun exists in the
world — true for one seat and silently refusing to toggle either the moment a
second body holds one. ⚠ An `Entity` in a message is safe HERE and only here:
all four are `clear_message_on_rollback`, so they are produced and consumed
inside one tick and never cross a rollback boundary. Guarded by
`two_driven_bodies_each_fire_their_own_portal_gun`, which asserts two intents
AND their origins (two shots from one body would satisfy a count), red under
each half poisoned separately.

✔ **D-FIGHTER-L6 — the rollout read its own silence as safety.** Reproduced at
HEAD on seed 0 before touching anything (l6 `unfought 1/1` with rollout on,
`both survive` with it off), then traced. ⭐ THE TRACE SAID IT IN ONE LINE, two
ticks before the body died at 0%: `offered=[Approach, Dodge, Jump]
vetoed=[Approach, Jump] unmodelled=[Dodge] chose=Some(Dodge)
least_bad=Some(Approach)`. `movement_intent` returns `None` for the verbs the
shadow cannot simulate — Dodge, Blink — and its own header says a rollout
reporting every unknown as safe *"would be lying in one direction"*. It reports
neither: the option is dropped from the rolled set, so it never appears in
`vetoed`, and a `find` over "not vetoed" promoted it above every verb the rollout
DID judge — while also suppressing `least_bad_movement`, which was gated on every
OFFERED verb being vetoed and an unjudged one never is. ⭐ `pick_movement` ranks
three tiers now: judged-and-unvetoed, then the least-bad line, then unmodelled
(which is also the untouched no-rollout path). ⚠ THE TRACE HAD TO GROW FIRST —
`least_bad` and `unmodelled` never left `refine_by_rollout`, so "every option was
fatal and this one dies latest" and "a verb nobody rolled outranked them"
rendered identically. Both are published now, on the stderr line and on the
causal fact. **Measured after: l6 `unfought` 45/45 → 0/45 at 45 seeds with
rollout ON**, peak damage 84%/54% where it was 0%/0%; recorded in
`dev/ambition_dev_measurements/ladder_recovery_sweep.jsonl`. Guards: four arms on
`pick_movement`'s tiers (two red under the old rule) and one on the rollout
publishing its unjudged set. ⚠ l1 is still `unfought 45/45` and was 45/45 in BOTH
arms of the original A/B, so it is a SEPARATE defect — see D-FIGHTER-L1.

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
component the ECS projectile road uses — so a hit is credited to whoever fired
it instead of `Query<Entity, PrimaryPlayerOnly>` (the crediting stepper it
named, `held_projectile_step`, was deleted by the K2 fold on 2026-09-02). Looping
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

- ▢ **THE FEATURE UNION IS RED: 48 failures against 6,968 passes, and 37 of
  them are ONE system.** Measured 2026-09-03 at `dbfb1a2ca` by running the gate's
  own union job standalone (`cargo test --workspace --no-fail-fast --features
  <the 80-entry union>`, the exact command `run_tests.py --list` prints under
  `--run-everything-you-probably-dont-need-this`). Four targets failed:
  `ambition_demo_smash_app --test smash_it`, `ambition_demo_sanic_app --lib` and
  `--test sanic_it`, `ambition_demo_mary_o_app --test mary_o_it`.
  ⛔ **THE DOMINANT CAUSE IS A SINGLE PARAMETER.** 37 of the 48 are the same
  panic: *"Encountered an error in system
  `ambition_portal2d_presentation::view_cones::sync_portal_view_cones`:
  Parameter `ConeRigAssets<'_>::meshes` failed validation: Resource does not
  exist"*. `ConeRigAssets` takes `ResMut<Assets<Image>>`, `ResMut<Assets<Mesh>>`
  and `ResMut<Assets<ColorMaterial>>`; the union puts `portal_render` into demo
  compositions that never provision mesh assets, and Bevy 0.19 turns a missing
  system parameter into a hard failure where 0.18 skipped. `view_cones.rs` was
  last touched by the 0.19 port (`09bb065a9`), which is consistent with the port
  having created this and nothing having run the combination since.
  ⚠ **THE OTHER ~11 ARE A DIFFERENT CLASS AND MAY NOT BE DEFECTS AT ALL.** They
  are mary_o assertions, and at least one fails BY CONSTRUCTION under an
  all-features build: `the_presentation_plugin_adds_no_hud_and_no_menu` asserts
  0 UI nodes and gets 40, which is what enabling every presentation feature at
  once is *supposed* to do. Whether those tests should be feature-scoped, or the
  union should exclude them, is a judgement for whoever owns the demos doctrine
  — do not "fix" them by widening the assertion.
  ⚠ The `painted_blocks` pair is a THIRD cause, read far enough to aim the next
  person: the helper looks for an entity matching `(&BlockVisual, &Sprite)` whose
  `geo_id` is the placement's, and panics *"no block visual is drawing GeoId …"*
  when the query finds NONE. So the failure is not "wrong art" but "no block
  visual entity at all" — including in
  `a_painted_block_nobody_dresses_keeps_its_flat_quad`, where the undressed case
  is the subject. ⇒ First question for whoever picks it up: under the union, does
  some other presentation feature take ownership of block drawing, or does the
  room never reach the state that spawns them? Neither is answered here; both are
  a build away, and the answer decides whether this joins the doctrine group above
  or the `ConeRigAssets` group.
  ⇒ **The fix for the 37 is a design choice, which is why this is a row and not
  a commit**: skip the system when its assets are absent (`If<…>`, or a
  `resource_exists` run condition) versus provision the assets in every
  composition that installs portal presentation. The first says a cone rig with
  nowhere to draw should stand down; the second says the composition is
  incomplete. Bevy's own message suggests the first.
  ⛔ **AND THE REASON NOBODY SAW IT: the union job lives inside `if not only and
  everything`**, so a default gate run never attempts it — the same blindness
  the coverage footer now states as 783 tests across 29 crates (`65f4030b5`).
  A per-crate green says nothing here, exactly as the jab-string row records.

- ✔ **`hall_transition_cover` WAS RED IN PARALLEL, a SECOND-ORDER COST OF THE
  COMPOSITION FIX — CLOSED 2026-09-02 evening.** The fixture waits for FACTS now
  (`wait_for_a_session_room_set`, `settle_resident_pages` — every resident
  realization's USED pages loaded and the table quiet, 180 s backstops that
  say the harness gave up; `70f6a8494`, and the lap test's frame-count sample
  replaced the same day), the three discarded `settle_cast` verdicts became
  an asserting `settle_launcher`, and the population it is about certified
  it: an unfiltered `cargo test -p ambition_app --test app_it` read **548
  passed / 0 failed** with all six hall tests inside it (agent 383484,
  b0df66fc6+). ⚠ One green of a schedule-dependent flake proves the fixture no
  longer loses to this machine at this load, not that no schedule exists that
  beats a 180 s backstop. The original evidence, kept: measured at `0955bd888`:

  ```text
  cargo test -p ambition_app --test app_it hall_transition_cover
    parallel (the normal invocation)   3 passed, 3 FAILED
    --test-threads=1                   6 passed, 0 failed   (106.8s, ~18s/test)
  ```

  All three failures are one panic — `hall_transition_cover.rs:74`, `.expect("a
  session room set")` — i.e. the app never reached a `RoomSet` before the harness
  gave up.
  ⭐ **`settle_cast(app, secs)` IS A WALL-CLOCK DEADLINE, NOT A SETTLE.** It
  returns whether or not the thing arrived, and the CALLER CANNOT TELL WHICH, so a
  timeout masquerades as "settled" and surfaces three lines later as an unrelated
  `.expect`. Callers pass 10s and 20s.
  ⇒ It was adequate while headless decoded nothing. The composition fix took the
  same hall entry from **22 resident images to 226** (re-measured at this commit:
  226 resident / 201 routed / 0 re-decodes), so boot is roughly an order of
  magnitude heavier and the budgets expire under parallel CPU contention.
  ⛔ **THE FIX IS THE SHAPE, NOT THE NUMBER** — raising 20 to 60 buys time until
  the next thing gets heavier. Wait for the FACT (a room set exists / the cast is
  staged) with a generous cap, and panic naming the cap when it is hit, so a
  timeout can never again be mistaken for progress.
  ⚠ And the general point, which is why this is a row and not a bug report: the
  composition fix was correct and its cost was not free. **10× the decode work at
  boot surfaced first as a test-harness timeout, not as a number anyone
  recorded** — a "captures byte-identical, RSS −141 MB" A/B does not show it.

- ✔ **THREE `app_it` TESTS ASSERT OVER A PROCESS-GLOBAL LEDGER AND ARE
  PARALLEL-FLAKY — CLOSED 2026-09-02, both halves.** Found 2026-09-02 while
  checking for reds.
  ⭐ **VERIFIED BY THE FULL PARALLEL TARGET, which is the only run that could
  say so**: `cargo test -p ambition_app --test app_it` reads **546 passed / 1
  failed / 22 ignored** in 484s, against 543/2 when this row was written. All
  three ledger readers are green.
  ⛔ **THE ONE RED IS NOT THIS ROW, and the distinction is the useful part.**
  `two_round_trips_through_the_gallery_return_the_same_working_set` passes alone
  (25.7s) and fails in the target — but `residency_snapshot` reads
  `common::resident_character_pages(app)`, the per-App helper, so a sibling App
  CANNOT inflate its answer. It samples with `for _ in 0..300 { step }` and then
  reads; a fixed FRAME count is not a settle, and paced asset work converges
  further in 300 frames alone than under contention. **Fixing the contamination
  is what made that visible** — it is the settle-shape defect in the row above,
  not a ledger one. ⚠ Its message also says PAGES "grew" for a 16 → 15 SHRINK,
  which sends a reader hunting a retention leak; the sibling megapixel assertion
  says "moved" and is the right wording. `app_it` runs its
  tests as parallel threads; `image_stages::ledger()` is a `static`. Exactly three
  files read it:

  ```text
  hall_redecode_census.rs                       #[ignore] + script   PASSES
  hall_transition_cover.rs               (2)    no guard             FLAKY
  quality_change_keeps_each_character.rs (1)    no guard             FLAKY
  ```

  ⭐ **The one with the guard is the one that passes** — same subject, same
  global, and the only difference is whether it runs alone. Measured: full
  `--test app_it` gives 543 passed / 2 failed; each failing test passes under
  `--test-threads=1 --exact`; `leaving_the_gallery…` failed one run and passed
  the next on identical source; and the counts move with the schedule (15→14 on
  one base, 20→19 on another), where a real leak would be stable.
  ⛔⛔ **THE FAILURE TEXT IS WHY THIS IS NOT MERELY NOISE**: *"1 character page(s)
  are resident in the hub with no realization owning them:
  `sprites_0_25x/goblin_spritesheet.png`"*. That reads as an asset-residency leak
  in exactly the subsystem the hall-entry campaign is about, so it will cost
  somebody real hours. The goblin page belongs to a sibling test's App.
  ✔ **FIXED 2026-09-02 at `8fe723cf9`, the per-App way — and this row said that
  was IMPOSSIBLE.** The correction is worth more than the fix:
  `common::resident_character_pages` answers residency from **THIS App's**
  `Assets<Image>` and **this App's** `AssetServer` path, and uses the ledger ONLY
  as a classifier (was this path demanded on the `character-sheet` road),
  requiring `row.path == server.get_path(id)`. A sibling's row can share an arena
  index; it cannot make a page resident here that is not, and the path equality
  rejects a colliding index carrying a different file.
  ⛔ **WHAT I GOT WRONG, because the shape recurs**: I checked whether each key
  discriminates ALONE — path collides (two Apps load the same file), id collides
  (`AssetId::Index` (cite-ok: bevy_asset's, not ours) is a per-arena
  `{generation, index}`) — and concluded neither
  could work. **"Neither A nor B is sufficient" does not imply "A ∧ B is
  insufficient."** Residency from the App plus classification from the ledger is
  exactly that conjunction, and it is sound.
  ⭐ **THE LEDGER HAD THIS SHAPE OF PROBLEM TWICE, AND THE SECOND HALF IS NOW
  CLOSED (2026-09-02).** `render_world_present` was also a process-global `bool`
  standing in for a per-App fact — set true by whichever App installed a render
  plugin, read for every App thereafter, and `is_awaiting_gpu` gated on it.
  ⚠ It was **LATENT, NOT LIVE — measured**: all 97 `VisibleRenderMode` uses
  across `app_it` are `NoWindow`, so nothing in that process ever set it and it
  could not give a wrong answer yet. It would have become live the day one test
  built a render world beside one that did not.
  ✔ **FIXED BY DELETING THE FIELD, not by guarding it.** `RenderWorldPresent` is
  a Bevy `Resource` in `image_stages`, inserted on the MAIN world by the App that
  installs the census's render systems; `is_awaiting_gpu(id, render_world)` and
  the census's never-drawn row take the fact as an argument, so the process
  ledger no longer holds an opinion about which App is asking. Threaded through
  `inspect_room_asset_manifest` to its five production callers — the three
  systems that had a spare param slot took one, and the two that were at Bevy's
  arity limit joined an existing tuple beside `AssetServer`/`Assets<Image>`,
  which is where the readiness authorities already travel together.
  ⭐ **THE TESTS THAT SET-AND-RESTORED THE GLOBAL WERE THE TELL, and they are
  the guard now.** `room_readiness_waits_for_the_gpu_copy_only_while_a_render_world_owes_it`
  used to set the flag for its span and clear it before returning, with a doc
  paragraph explaining why; that dance is gone, and both it and the ledger's own
  `the_gpu_readiness_term_wants_the_gpu_stamp_while_a_render_world_is_present`
  now assert the thing the field made unsayable: **one ledger, one id, one
  breath, two different answers** for a headless App and a rendering one.
  ⛔⛔ **AND BE PRECISE ABOUT WHAT THE POISON PROVED.** Making `is_awaiting_gpu`
  ignore its argument turns both tests red, at both layers, with messages naming
  the defect — but the arm that catches it is one that existed BEFORE this
  change, and the panic arrives before the new same-breath assertion even runs.
  ⇒ The real guard here is STRUCTURAL: the field is deleted, so an App cannot
  inherit a sibling's answer no matter what a test asserts. The new assertions
  DOCUMENT that — they state a claim the old design made unsayable — and the
  only poison that defeats them is reintroducing the global, i.e. reverting the
  change. A test cannot guard a shape it is not able to express; do not credit
  this row with a guard stronger than that.

- ✔ **THE POPULATION CAP ADMITTED AFTER THE PLAN WAS FROZEN — CLOSED 2026-09-02
  by `2ea4ef21ac`, five of six claims verified against the code.** A finding
  under D33, which is still open; ⛔ deliberately NOT titled with that id, because
  a `✔ D33 …` row beside the open `▢ D33 …` row makes the ledger say both things
  about one id — which `test_no_ledger_row_is_marked_both_open_and_closed` caught
  the moment I wrote it.
  `RoomFeatureConstructionPlan::prepare` now spends the quota over
  `room.placements` in AUTHORED order and hands only `admitted_records` to
  `plan_room` (`spawn/mod.rs`), so: a refused NPC has no plan row, no root and no
  id; the predicted roster is honest; the cohort follows the order the API always
  claimed; and `construction_plan_id` differs between capped and uncapped runs
  because it hashes `construction_deterministic_dump()`, which is built from the
  admitted set. The fake-Hazard acceptance is replaced by
  `the_population_cap_is_spent_at_plan_time_and_each_plan_gets_its_own_quota`,
  which uses real `InteractionKindSpec::Npc` placements.
  ⚠ **THE HALL NUMBERS STILL WANT RE-READING**, and closing this row does not do
  it: capped runs taken BEFORE this fix selected their cohort in canonical
  `SimId` order while the API said authored order, and kept over-cap identity
  shells — so `EcsPopulation::scene_entities()` did not fall with body count.
  Body-count curves stand; any explanation that leaned on the cohort being an
  authored-order PREFIX was about a set nobody measured.
  ✔ **AND THE SIXTH IS CLOSED TOO, BY RENAMING RATHER THAN CORRECTING.**
  `ActorAdmission::admitted()` was documented as the number admitted and was
  wrong in two directions: a refusal COUNTS (`fetch_add` precedes the compare, so
  a cap of 2 reports 3) and an uncapped run counts NOTHING (the uncapped path
  returns before the counter, deliberately — it is called once per placement in
  every room). It is now `admission_attempts()`, with both directions in its doc.
  ⛔ Not corrected, because making it truthful means an atomic on the uncapped
  road forever and **nothing in the tree reads it** — the other `.admitted()`
  calls belong to a portal route, a sandbox reset and a shrine. A name that
  cannot mislead beats a number nobody consumes.

<details><summary>The original finding, kept because the sequence is the lesson</summary>

- **THE POPULATION CAP ADMITTED AFTER THE PLAN WAS FROZEN, so a refused NPC
  still got an authoritative root.** Raised by review 2026-09-02
  and verified against the code. ⛔ **NOT A REGRESSION FROM TONIGHT'S WORK** —
  the composition-input inversion (the cap as an engine value, published by
  `ambition_dev_tools`, handed to construction) is right and stays. The process
  -global counter it replaced refused at the same place, so this is the older
  defect the inversion did not reach.

  The sequence, each step checked:

  ```text
  RoomFeatureConstructionPlan::prepare   builds ALL placement requests
  ConstructionPlan::prepare              requests.sort_by(|a,b| a.sim_id.cmp(&b.sim_id))
                                         construction/mod.rs — canonical, before any cap
  spawn/mod.rs                           the frozen set becomes expected_authoritative_ids
  ActorAdmission                         attached AFTER that, as execution context
  commit_entity                          `spawn_empty()`, then SimId + provenance +
                                         transaction ownership on EVERY planned row
  lower_interactable_placement           `if ... !admission.admit_actor() { return; }`
                                         spawn_static.rs — the FIRST place the cap acts
  ```

  ⇒ Consequences, in order of how much they matter to the measurement campaign:
  - an over-cap NPC is a live identity/provenance/transaction **shell** with no
    actor or body — so `EcsPopulation::scene_entities()` did NOT fall by the
    same amount as body count, and any entity-count scaling conclusion from a
    capped run needs re-reading;
  - the cohort is chosen in **canonical `SimId` order**, while `ActorAdmission`
    documents "the first n authored placements". ⚠ **The hall's body-count
    curves are not thereby void** — the cap really did reduce the body
    population and the reports name actual body counts — but any explanation
    that leaned on the selected set being an authored-order PREFIX (the
    visibility/density wobble story) was reasoning about a cohort it did not
    measure;
  - `verify_committed_roster` accepts the shells, because the planned identity
    IS on the expected root with the expected stamps;
  - `RoomConstructionPlanId` hashes the `RoomSpec` plus the deterministic dump,
    and the cap enters neither — so **capped and uncapped runs can share one
    immutable-plan identity while producing different populations**.

  ⇒ **The fix is to admit on the REQUEST side, before `ConstructionPlan::prepare`
  freezes and canonicalises**: take up to N qualifying NPC requests in authored
  placement order, omit the rest entirely, then let the plan sort what remains.
  A refused NPC then has no row, no root, and no place in the predicted roster,
  and the plan id differs when the admitted world differs. ⛔ Do NOT fix it by
  despawning shells after commit, by adding the cap to the plan hash, or by
  keeping hidden mutable policy behind a frozen plan. Relations need an explicit
  rule; refusing preparation when a cap would sever a required one is acceptable
  for the hall workload.

  ⛔ **THE EXISTING TEST CANNOT SEE ANY OF THIS.**
  `the_population_cap_rides_the_plan_and_each_plan_gets_its_own_quota` uses four
  Hazard rows and a fake lowering that spawns an `Admitted` marker, then counts
  markers. No `Interactable::Npc`, no planned ids, no check that refused roots
  are absent. The acceptance wants real NPC placements asserting the refused ids
  are absent from `planned_ids`, from `expected_authoritative_ids`, from the
  receipt and from live `SimId`s; that the admitted cohort follows AUTHORED
  order; and that capped and uncapped plan ids differ.

  ⚠ Minor, same file: `ActorAdmission::admitted()` is documented as the number
  admitted, but a cap of two followed by one refusal returns three, and the unit
  test locks that in as "the refusal was counted as an attempt". Pick one meaning
  and let the name say it.

</details>

- ✔ **DEV GRAVITY CYCLE PUBLISHES A REQUEST; THE SIM APPLIES IT (2026-09-03).**
  `AmbientGravityRequest::Cycle` in `shared_tangle::gravity`; the hotkey and
  the developer menu's Gravity row both WRITE it (the menu through
  `Messages<_>` so a fixture without the gravity plugin renders the row as a
  no-op rather than failing the menu system's parameter validation); the
  kernel's gravity plugin applies it in the sim, chained before
  `resolve_active_gravity`, so a request made this frame is felt this tick.
  The waiver is gone and the guard reads 0 unwaived without it; unit guard
  `a_cycle_request_is_applied_by_the_sim_and_steps_one_cardinal`. The row as
  filed: `cycle_dev_gravity` (`game/ambition_app/src/menu/kaleidoscope_app.rs`) wrote
  `BaseGravity` — a canonically rollback-registered resource — directly from
  `Update`. In a rollback session that is a real desync source: the value is
  restored on every rewind, the toggle is not replayed with it, and the peer
  never saw the toggle at all. Nothing crashes; the runs simply drift.
  ⭐ **The fix has a precedent rather than needing a design**: publish a request
  the sim consumes, the `ClockScaleRequest` / D33 shape. Engine work, no product
  judgement, small. (It was waived meanwhile in
  `scripts/check_rollback_mutators_run_in_sim.py::WAIVERS` with this row named;
  the waiver left with the fix.)
  ▢ **And the same row owes a second question it must not assert.**
  `restore_inventory_from_save` writes `BodyWallet` from `Update` while applying
  a save. At session activation no sim tick has advanced, so there is nothing to
  diverge from — but `session/durable_horizon.rs` states plainly that *"THE
  `Update` ADOPTER STAYS. A file can also arrive after activation (a mid-session
  load), and adoption is idempotent"*. ⇒ The pre-first-tick condition is **not
  provable from the code** and is not claimed. Idempotence makes the write
  consistent with itself, which is not the same as consistent with a peer that
  never applied it. Someone who knows whether a mid-session load can happen
  inside a live GGRS session should answer it; until then the waiver says so.

- ▢ **D72 — continue Super Smash Siblings as a product/engine customer from the
  current parity inventory.** Do not resurrect the historical fun-push campaign.
  Re-read [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)
  and [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
  before choosing the next slice. Prefer mechanics/readability/control defects
  that expose reusable engine seams over broad polish. Explicitly keep
  already-settled genre decisions and shipped mechanics from being
  reimplemented. ✔ SLICE TAKEN 2026-08-31: the walk gait had no road on a
  digital input (inventory §4). ⭐ SURVEYED AND RE-MEASURED — three inventory
  rows were STALE and are corrected in place: wall-tech jump shipped 2026-08-23
  and is re-marked `✔`; the combat action buffer's *"nothing writes them"* prose
  is wrong (four writers, and tests assert them); and the inventory's
  `options.rs:752` citation is dead <!-- cite-ok --> — the file moved crate to
  `ambition_characters` and the line is `:707`. ⛔ NEXT CANDIDATES, in order:
  `CancelCondition::OnBlock` (inventory §1 — its stated blocker, "shield contact
  lands with CM6", is itself stale: shieldstun shipped, and the deferral note at
  `entity_catalog/src/lib.rs:212` — refreshed 2026-08-31, and D-CANCEL-ONBLOCK's
  plumbing is now closed, leaving only its FEEL ruling), and ✔ SLICE TAKEN
  2026-08-31: the `Auto`/`Tilt`/`Smash` strength hint (§9) — a full deflection
  could never be a tilt, because `strong_hint || recent_matches` could only ever
  ADD a smash. `AttackStrengthHint` replaces the bool from `ControlFrame` to the
  gesture resolver (schema 142→143), and ✔ THE DEVICE HALF LANDED THE SAME DAY:
  `ControlSettings::right_stick_mode` (`Aim` / `TiltAttack` / `SmashAttack`,
  default `Aim`, surfaced as a Controls row), a hysteretic flick that presses
  attack once per push, and `ControlFrame::attack_from_aim_stick` so a C-stick
  attack points where the RIGHT stick went while the left keeps meaning walk.
  ⛔ THE NEW SETTING NEEDED `#[serde(default)]` AND ALMOST DID NOT GET IT:
  `ControlSettings` has no container default, so a key missing from an older save
  is a parse error, and `load_settings` answers that by discarding the ENTIRE
  settings file. Now guarded by
  `a_settings_file_predating_a_knob_still_loads_everything_else`, which is red
  without the attribute. ⛔ NEXT CANDIDATES: pick from the inventory again — most
  remaining `▢` rows are deliberately-deferred FEATURES ("add when a move needs
  it"), and §9's own row says a capability wants a CUSTOMER before it is built. ⛔ DO NOT PICK: D203's hitbox premise (measured and REFUTED by
  `fda65a386`), D204/D205 (shipped), or anything in
  `awaiting-maintainer-decision.md`.

- ✔ **D-CANCEL-ONBLOCK — CLOSED 2026-08-31.** A blocked move confirmed an
  `OnHit` cancel because `mark_move_playback_landed_hits` read `LandedBodyHit`,
  whose own doc says it *"MEANS OVERLAP"*, and the block is decided later in the
  damage resolver.
  ⭐ THE FEEL RULING, made rather than deferred, and it is NEITHER of the two
  shapes this row named. Not *"delay the marker"* (that loses the connect-frame
  cancel window on every hit) and not *"retract on block"* (that would have to
  unwind the stale entry it already recorded). Instead the OVERLAP keeps its
  early slot and its staling — a move that hits a shield has been used — and the
  damage road's VERDICT arrives on its own channels afterwards, so nothing is
  written and taken back. `MoveContact { overlapped, connected, blocked }`
  replaces the one bool; `OnHit` reads `connected`, `OnWhiff` reads
  `!overlapped` (a blocked move touched something and is not a whiff), and
  `CancelCondition::OnBlock` exists and reads `blocked`.
  ⚠ THE COST, stated: against an ACTOR victim — the road every match fighter
  takes — the resolution lands on the overlap frame, so the connect-frame window
  is unchanged. Against a PLAYER victim it lands the next frame, so an `OnHit`
  cancel opens one frame later than it used to.
  ⭐ `BlockedBodyHit` is a POSITIVE fact from the resolver's own `Blocked` arm,
  not an inference from a missing connect: the two roads resolve on different
  frames, so absence means both "blocked" and "not decided yet". The trap this
  row named — a consumer reading the victim's shield state — is still a trap and
  is still avoided.
  Schema 143→144 (`MovePlayback`'s projection gained both facts; they are state,
  because two peers disagreeing about them take different cancels out of one
  recovery). The stale `entity_catalog` deferral note is replaced by the ruling.
  Pinned by `a_blocked_strike_is_an_overlap_and_not_a_connection` (inverted, as
  this row asked), `the_three_contact_outcomes_permit_three_different_cancels`
  and `a_playback_learns_connect_and_block_from_the_resolvers_own_channels`.
  ✔ AND THE AUTHORING LANDED 2026-08-31, the same day: George's jab nominates
  `grab` on block. A blocked jab is the moment the defender has committed to
  holding shield, which is exactly when a grab beats them — and until now a
  shielded jab bought George nothing, because the `OnHit` route into
  smash/special is closed by definition when nothing connected. ⛔ authored THIRD
  in his cancel list: the chain takes the first successor it can resolve BY MOVE
  ID, so `jab2` has to keep that slot or every held button would answer with a
  grab. ⛔ and `grab` is a VERB, not a move in his table, so the test asserts
  `cancel_targets` actually RESOLVES it through the contract's verb map rather
  than trusting the string — a window naming nothing would open onto nothing and
  no value test would notice. Red first (`0 != 1` on-block windows). Owner:
  [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

- ▢ **D-RASTER-3 — split the weak-GPU improvement between framebuffer scale and
  MSAA.** ⭐ **The coverage attribution this row wanted first is DONE (2026-09-01):
  four backdrop sprites hold 96% of all drawn sprite AREA and fifty-seven gameplay
  sprites hold 4% — the lever is the backdrop's layer count and blending, not the
  actors.** ⚠ World units, not pixels: `report_draw_census` cannot convert to
  screen coverage without the per-view projections, so this is a ratio and not a
  fill multiple. That half needed no hardware, because drawn area is a COUNT and
  a count is the same on any rasteriser; see
  `journal/2026-09-02-the-overdraw-is-the-backdrop.md`. The timing split below
  still needs the real device. The valid matched result is **51.045 ms -> 20.101 ms p50, about 2.54x**;
  both DPI/framebuffer cap and MSAA changed together. Run an interleaved A/B on
  real weak GPU hardware with the independent `AMBITION_MAX_SCALE_FACTOR` and
  `AMBITION_MSAA` knobs, multiple reps per arm, holding build/features/profile
  constant. Do not substitute lavapipe/software rendering. Owner:
  [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

- ✔ **D-CUBE-CHURN — CLOSED 2026-09-02, by count.** `cache_system_menu` ran
  every visible frame on every face and, on the System face, BUILT THE WHOLE
  MODEL every frame (the settings IR plus a string per row); `DevSnapshot` was
  21 `String`s a frame; `kaleidoscope_sync_focus_visuals` rebuilt a
  `Vec<SystemRow>` per control per frame. Now: the model rebuilds only when an
  input's VALUE moves (settings, radio, dev, pending quality, drill state —
  compared, never tick-checked; a `set_changed()` on `UserSettings` every frame
  for thirty frames builds it once, `an_idle_system_face_builds_its_model_once`,
  red at 30 with the gate removed); `DevSnapshot` borrows its labels
  (`0fb8c50e1`); the focus sync reads `CachedSystemMenu.rows` through the one
  `focus_for_action(action, page, rows)`.
  ✔ **AND THE BIGGER THING THAT SURVIVED IT IS CLOSED TOO, 2026-09-02.**
  `pointer.rs` rebuilt the WHOLE settings IR — a `String` per label, description
  and value, plus both snapshots — ON EVERY HOVER EVENT over a System control, to
  resolve one row index; larger than either allocation this row named, and never
  in the survey. The hover now reads the rows `cache_system_menu` already built.
  ⛔ NOT A STRAIGHT CACHE SWAP, exactly as this row warned: `cache.rows` is empty
  off the System face while a System action stays reachable there, so an
  unguarded substitution resolves every such hover to `MenuFocus::System(0)`.
  ⭐ AND THE GUARD IS ONE COMPARISON WIDER THAN "IS THE FACE ACTIVE", because
  `kaleidoscope_pointer_move` is an OBSERVER: it fires whenever a `Pointer<Move>`
  arrives, which may precede this frame's `cache_system_menu`. Of the six inputs
  the rows are built from, five cannot move without a frame passing; the
  DRILL-DOWN can, because a press opens it. `CachedSystemMenu::rows_are_current_for`
  is both halves. Guarded by
  `the_cached_rows_resolve_a_system_action_exactly_as_a_fresh_model_does` —
  ⛔ which picks a row PAST THE FIRST on purpose, since `focus_for_action` falls
  back to `System(0)` and an action at index zero cannot tell a correct answer
  from the fallback. Poison-verified: make the guard always say yes and the
  off-face arm names it. ✔ **BOTH "also open" items CLOSED 2026-09-02** (`63cceae0a`),
  and re-verified against the code 2026-09-02: `focus_for_action_in_rows` no
  longer exists — there is ONE `focus_for_action` taking `rows` as a parameter,
  so the "same lookup wearing different names" drift `character/assets.rs:261`
  records is undone — and `grid_backend::cursor_focus_key` builds the row list
  ONCE, outside its loop over the page's nodes.
  ⛔ THE NAME IN THIS ROW WAS INVENTED. It read
  `grid_backend::focus_key_for_cursor`, which `git log -S` says has never existed  <!-- cite-ok: quoted mistake -->
  in this repository; the function is `cursor_focus_key`, and the same fabricated
  name had also reached a doc comment in `kaleidoscope_app.rs`. A citation nobody
  greps is a citation nobody can check — this one survived a commit message, a
  planning row and a code comment.
  Still per frame and still allocating:
  `RadioSnapshot`'s `String` per station and the `RebuildKey` clone of it —
  both small, both a count away if they matter. Read the map for what is
  FALSIFIED (Lunex is `Changed<>`-filtered; the `UserSettings` clone is a memcpy;
  `Msaa::Off` recovers nothing):
  [`../../dev/journals/kaleidoscope-what-churns-and-what-does-not-2026-08-31.md`](../../dev/journals/kaleidoscope-what-churns-and-what-does-not-2026-08-31.md).

- ✔ **THE `features` FACADE LAUNDERED 34 LOWER-CRATE NAMES, so 27% of the
  kernel's apparent coupling to it was not coupling at all — CLOSED 2026-09-02,
  `RE-EXPORT 45 (27%) → 0 (0%)`.** (Measured by
  `scripts/measure_facade_reexport_coupling.py`, which ships with it.)
  ⭐ **HOW IT WAS FOUND:** a per-file sizing of `items/conditions.rs` reported
  SEVEN references into the kernel. Six were one type — `crate::features::HeldItem`
  — and `HeldItem` is `ambition_combat`'s, reached through
  `features/ecs/mod.rs:27` (`use ambition_combat::{… held_items …}`) and then
  re-exported twice. The file was not coupled to the kernel; it was naming a
  kernel path to another crate's type.

  ```text
  facade re-exports 129 names:  RE-EXPORT 34 | LOCAL 95 | UNRESOLVED 0
  `crate::features::X` uses:    164 total
     RE-EXPORT  45  (27%)   <- naming a type defined OUTSIDE the monolith
     LOCAL     119  (73%)
  by real owner:  ambition_combat 42, ambition_boss_encounter 3
  ```

  ⛔⛔ **THE CONSEQUENCE IS FOR SIZING, NOT TIDINESS.** Every carve estimate in
  this file that counted `crate::features` references overstates kernel coupling
  by up to a quarter, and those estimates are what decides which carve is "an
  evening's" and which is "multi-day". The `items/` row above was sized that way
  and was wrong for a related reason (it quoted the UNION of per-file imports).
  ⇒ **a sizing grep must resolve where a name is DEFINED, not where it is
  spelled.**

  ✔ **CLOSED THE SAME NIGHT — all 45 re-pointed, `RE-EXPORT 45 (27%) → 0 (0%)`**
  (`a91e7d614`; the 119 LOCAL uses are untouched, which is what the facade is
  for). 42 were `ambition_combat` (`held_items::HeldItem`, `hitbox::*`,
  `targeting::*`, `hazards::*`, `banner::*`, `falling_chest::*`) and 3 were
  `ambition_boss_encounter::ecs::*`.
  ⛔ **THE PATHS WERE DERIVED, NOT GUESSED**, and that mattered: a script read
  `features/ecs/mod.rs` for which submodules come from which crate and which
  names each `pub use <mod>::{…}` republishes, so each rewrite is the path the
  kernel itself imports through. Writing `ambition_combat::HeldItem` from memory
  would have been wrong — it is `ambition_combat::held_items::HeldItem`.
  ⚠ I had written "not worth a merge conflict, pick it up when the tree is
  quiet" and then did it anyway, which was right for a reason worth keeping: the
  conflict risk is not a property of the CRATE, it is a property of the FILES.
  All 15 were checked against `git status` first and none was open in another
  session. ⇒ "the tree is busy" is too coarse a reason to defer a mechanical
  change; "these files are open" is the real question.
  ⚠ The kernel reads 1204 tests after this, against 1207 before — the three are
  a peer's uncommitted room-tier-cap deletion (`character_sprites/assets.rs`,
  11 `#[test]` → 8), checked per file rather than assumed.
  ⭐ **AND THE SWEEP WAS GENERALISED TO EVERY KERNEL MODULE, WHICH IS HOW THE
  MEASUREMENT CAUGHT ITSELF OVER-REPORTING.** Running the same "is this name
  defined here?" test over `crate::<module>::X` for all 32 kernel modules first
  reported `character_runtime` 36/140 (25%), `control` 8/32 (25%) and `session`
  8/58 (13%) — headline numbers that would have justified three more sweeps.
  ⛔ SPOT-CHECKED BEFORE ACTING, and most of it dissolved: `crate::session::reset`
  and `crate::session::data` are MODULE paths, not names, and the script was
  matching a same-named `pub fn` in another crate; `character_runtime::presentation`
  likewise. ⇒ **the second path segment is not necessarily a type**, and the
  `features` result only held because it was cross-checked against that facade's
  actual re-export list.
  ✔ What survived the check was small and is now done: `control::DrivingParticipant`
  (8 uses, really `ambition_characters::control::DrivingParticipant`).
  ✔ **AND THE ONE CASE I LEFT "ON PURPOSE" TURNS OUT NOT TO BE A CASE.**
  `character_runtime::prepare_and_finalize_for_test` (33 uses, really
  `ambition_characters::prepared`) looked like the same laundering, and the row
  said it was deferred only because its hub file was open in another session.
  Read afterwards: the re-export is `#[cfg(test)] pub(crate) use`. It is
  test-only AND crate-private, so no consumer can reach it and nothing is being
  laundered — it is an ordinary local alias for a test helper, which is what
  those are for. ⇒ **Do not re-point it.** The `features` case was a problem
  because it was `pub`: a name a consumer could learn the wrong path to.
  ⇒ **The `features` facade was the real hub; the rest of the kernel is close to
  clean.** Do not re-run the generalised form and act on its raw numbers without
  the per-module re-export cross-check — it over-reports by design.

- ▢ **SIX WORKSPACE DEPENDENCY DECLARATIONS ARE NEVER NAMED IN THEIR CRATE'S
  SOURCE — graph honesty, NOT a footprint win.** (Measured 2026-09-02.) Every
  crate's `src/**/*.rs` was searched for each `ambition_*` dependency its
  `Cargo.toml` declares:

```text
PLAIN (unconditional) edge, never used:
  ambition_platformer2d        -> ambition_interaction
OPTIONAL dep + feature, never used:
  ambition_characters          -> ambition_causal        (feature `causal`)
  ambition_platformer2d        -> ambition_sfx_bank
  ambition_sim_view            -> ambition_portal2d
  ambition_touch_input         -> ambition_cutscene
  game/ambition_app            -> ambition_causal
```

  ⛔ **SIZE IT HONESTLY: REMOVING THESE CUTS NOTHING FROM THE CLOSURE.**
  `ambition_interaction` is declared by six crates including
  `ambition_platformer2d_actor_monolith`, so the facade's edge is redundant, not
  load-bearing — the same fact the capability row above already states about all
  sixteen never-asked-for crates ("gating a facade edge cuts nothing"). The
  value is that the graph stops claiming an edge nobody uses.

  ⇒ **The PLAIN one is worth cutting first**: the facade is what every game
  links, and a declared-unused edge there is what the next reader will justify
  rather than question.

  ⛔⛔ **DO NOT REMOVE BLIND — it needs the compiler on each crate's feature
  combinations.** Dropping an optional dep changes feature RESOLUTION, not just
  a line, and only a build says what that does. ⇒ Left for the abilities carve
  to pick up rather than done here.

  ⚠ **What the scan can and cannot see, because the obvious caveat is the wrong
  one.** A `cfg(feature)`-gated `use` IS visible to a text scan — control:
  `ambition_content_pack` is an optional dep of `ambition_audio`,
  `ambition_boss_encounter`, `ambition_characters` and `ambition_combat`, all of
  which have `cfg(feature)` blocks, and the scan correctly did not flag any of
  them. What it would miss is a macro-generated path or a build script; checked,
  and only `ambition_app` has a `build.rs`, which does not mention `causal`. No
  crate here renames a dependency with `package = "…"`, so the Cargo name is the
  code name. Five further candidates were excluded because they ARE used, from
  `tests/` rather than `src/` (`ambition_platformer2d_host` ×2,
  `examples/capability_demo`, `ambition_app` → `ambition_demo_pocket`,
  `ambition_content` → `ambition_content_cli`) — those are dev-dependency
  questions, not unused ones.

- ▢ **D33 — continue actor-monolith decomposition by coherent ownership.** Pick a
  carve that removes a real authority/dependency edge from the residual actor
  kernel, moves registration/tests with the domain, and improves capability or
  compile/test isolation. Do not carve by LOC and do not promise frame-time
  improvement without a measurement.

  ⭐ **WHAT EVERY CARVE OWES AFTER IT LANDS (added 2026-09-02, from three rows
  that were stale within a week of the merge that closed them).** None of this is
  new work invented for carves — it is the paperwork a merge does not touch,
  because the code lives in one file and the claim about it lives in another:

  1. **`python scripts/modules_md.py`** — must print *"MODULES.md up to date"*.
     A carve moves modules; the maps are generated and go stale silently.
     (72 crates as of 2026-09-03, after D33 cut 1 added `ambition_body_seed`;
     70 the day before. ⛔ AND IT WAS STALE WHEN THAT CUT LANDED — not from the
     cut: `4ac56a996` added `ambition_encounter/src/mob_seed.rs` and left its
     map at 17 modules. Regenerated in the post-carve pass, which is what this
     item is for.)
  2. **`python scripts/check_planning_citations.py`** — a carve renames or moves
     the very symbols the planning rows cite. ⭐ **AND THEN `--vanished <the
     carve's parent SHA>`**, which catches what the default run cannot see:
     `SYMBOL` needs a `::`, so a BARE backticked name — the commonest form in
     these docs — is never checked, and a carve's removals are usually spelled
     bare. It reports a bare name that WAS a definition at that SHA and is not
     one now; the name's own history supplies the precision, so nothing has to
     guess what is "code-shaped". This is the removed/renamed half of item 4,
     without item 4's `head -1` or its `length >= 8` heuristic.
     ⭐ **PASS THE RANGE, `--vanished <parent>..<carve>`** — that is the form
     that attributes correctly. A bare ref compares to the WORKING TREE, so once
     HEAD moves past the carve it sweeps up every later removal and blames the
     carve for all of them: the three carves of 2026-09-02 returned 10, 1 and 0
     hits when run as bare refs on 09-03, and the 10 were mostly the tier-cap
     revert, nothing to do with the carve named. Cut 1 ran as
     `c761a9d80..83460e3f3` and correctly returned 0 — its symbols MOVED rather
     than vanished, which is item 4's job, not this one's.
     ⚠ AND PREFER A FRESH WINDOW to a wide one. Measured
     2026-09-03 over a week: 37 hits, and on inspection essentially all were
     rows RECORDING a removal ("Deleted: `FpsOverlayState`", "the view is
     DELETED") rather than rows made stale by one — docs/planning is clean on
     this axis, and a wide window is archaeology. A fresh window is not, because
     the rows have not been rewritten in past tense yet.
  3. **`python scripts/check_doc_links.py`**.
  4. **The ▢ rows that NAME A SYMBOL the carve touched.** Three were found stale
     in one sweep on 2026-09-02 (`bounded-perception`'s routing row, `queue.md`'s
     `string_id!` row, and the capability-footprint count) — from three different
     authors, none careless: a merge lands code and the row lives in a file the
     merge never touches. Run this over your own range:

     ```bash
     git diff -U0 <base>..<head> -- '*.rs' | grep -E '^[+-][^+-]' \
       | grep -oE '\b(fn|struct|enum|trait) [A-Za-z_][A-Za-z0-9_]*' | awk '{print $NF}' \
       | awk 'length($0) >= 8 || /_/' | sort -u \
       | while read s; do
           grep -rn "\`[^\`]*\b$s\b[^\`]*\`" docs/planning --include=*.md | head -1
         done
     ```

     ⚠ The two filters are load-bearing. Without `length >= 8 || /_/` the symbol
     list is `and`, `id`, `str` and the output is thousands of English words;
     without the BACKTICK requirement it matches prose rather than citations.
     ⛔ And it only finds rows that NAME something — a row describing the carve
     without naming a symbol ("the encounter adapter's seams still cross") stays
     invisible and still has to be read.
  5. **The capability footprint, if the carve ADDS a crate.** `closure_size` and
     `never_asked_for_count` move in `capability-footprint-baseline.json`, and the
     row above that quotes them must move in the same commit. It lagged twice.
  6. **⛔ The debt ledger is not laundered**: the destination joins in the SAME
     commit. A carve that only re-exports has moved nothing, and the source
     crate's line count falling is not evidence on its own.
  7. **⛔⛔ IF AN ABSENCE CONTRACT GOES RED, MOVE ITS EXCLUSION — DO NOT WIDEN
     IT.** **Eleven of the 25** contracts in `check_absence_contracts.py` pin
     *"this belongs to ONE file"* by EXCLUDING that file by path. A carve that
     moves the owner makes the contract flag the NEW location — it reads as "my
     carve broke an architectural rule" when it only moved the rule's home.
     ⇒ Point the exclusion at the new path IN THE SAME COMMIT; widening the
     paths or deleting the contract launders the rule the carve was meant to
     preserve, and looks like a clean carve in the diff. ⚠ Such a contract is
     invisible when you grep for the file it protects — it names that file in a
     `:!` path rather than in its rule, which is why this is a table:

     | If your carve moves… | it trips |
     |---|---|
     | `character_runtime/prepared_match.rs` | `a-second-writer-of-a-match-global-must-answer-ownership` |
     | `character_runtime/presentation.rs` | `the-provider-resolver-is-confined-to-one-file` |
     | `character_runtime/definition.rs` | `registration-does-not-demand-art` |
     | `ambition_combat/src/moveset/mod.rs` | `ending-a-move-goes-through-the-one-teardown-path` |
     | `avatar/starting_character.rs` | `the-motion-model-resolver-is-confined-to-one-file`, `the-movement-tuning-resolver-is-confined-to-one-file`, `the-catalog-axis-tuning-is-confined-to-one-file`, `the-catalog-default-action-set-is-confined-to-one-file` |
     | `avatar/mod.rs` | `the-movement-tuning-resolver-is-confined-to-one-file` |
     | `characters/src/prepared.rs` | `the-catalog-axis-tuning-is-confined-to-one-file`, `the-catalog-default-action-set-is-confined-to-one-file`, `registration-does-not-demand-art` |
     | `characters/src/actor/character_catalog/mod.rs` | `the-catalog-axis-tuning-is-confined-to-one-file`, `the-catalog-default-action-set-is-confined-to-one-file` |
     | `characters/src/brain/{fighter, state_machine/mod.rs, mod.rs}` | `the-generic-brain-does-not-grow-new-platform-fighter-edges` |
     | `characters/src/snapshot_impls.rs` — ⛔ NOT under `brain/`, and it trips TWO | `the-generic-brain-does-not-grow-new-platform-fighter-edges` (excluded) and `the-brain-codec-names-the-fighter-only-through-the-enum-variant` (its whole subject) |
     | `app/versus.rs`, `demo_smash/src/lib.rs` | `the-global-roster-is-retired-only-by-its-owner` |
     | `schedule/input_systems.rs` | `the-seat-topology-has-one-engine-side-creator` |
     | `ldtk_tools/ldtk/paths.py` | `the-worlds-path-is-confined-to-ldtk-paths` |

     ⚠ And `the-character-domain-is-not-named-after-a-character` guards ALL of
     `crates/ambition_characters/` by PATTERN rather than by exclusion, so it is
     live for any carve landing code there whichever file moves.
     ✔ Since 2026-09-02 a parametrized test asserts every path a contract names
     still EXISTS, so the quiet half of this rots loudly now: an exclusion left
     behind guards a ghost, and an include root that vanishes makes the contract
     scan nothing and pass forever.
  8. **⚠ IF THE CARVE MOVES A CRATE, re-read its `WAIVED` prefix in
     `rollback_coverage.rs`.** Twenty-two of that file's 31 waivers are
     NAMESPACE-WIDE (`ambition_render::`, `ambition_input::`, …), so a crate
     that moves or splits can leave a prefix covering nothing — or, worse,
     covering types its reason never described, which the file itself warns
     about. ⛔ There is no assertion for this on purpose: the audit is
     `list_what_every_waiver_actually_covers`, an `#[ignore]`d listing meant to
     be READ against each waiver's rationale. Run it after a crate move.
     ⛔ **AND DO NOT TRY TO CHECK IT STATICALLY — I did, on 2026-09-03, and the
     check is meaningless.** `waiver()` matches with `type_name.contains(needle)`,
     a SUBSTRING, so 49 of the 149 entries deliberately begin `::`
     (`"::intro::plugin::IntroPropSpritesInstalled"`) and name no crate at all —
     that is what makes them survive a crate move, and it is the design, not
     drift. Two more name upstream crates (`bevy_asset::`, `bevy_state::`).
     "Does this prefix cover anything?" can only be asked of the LIVE registry,
     which is why the audit is a test and not a script.

  9. **⛔⛔ AND THEN RUN THE REPO-TOOLING LANE. THIS LIST IS NOT A SUBSTITUTE
     FOR IT.** `python3 -m pytest scripts/tests -q -m "not detached_tool"` —
     about 60 s. Added 2026-09-03 because the cut-1 pass walked items 1–8, found
     two real things, reported clean — and the lane was RED the whole time, on
     two ledgers no item names: the rollback CODEC-SHAPE baseline
     (`scripts/tests/rollback_codec_shape.txt`) and a sub-workspace lockfile
     (`examples/capability_demo/Cargo.lock`). ⭐ The codec one is the sharp
     lesson: **a codec can leave the ledger by MOVING.** `CODEC_MARKER` did not
     know `SnapshotCursor`, so the carved-out impl's new crate was not in the
     population at all — a shape change there would have been invisible.
     Fixed at `36706b667`; 20 codec files → 22, and one of the two had been
     unwatched since long before any carve.
     ⇒ Items 1–8 are ledgers a MERGE does not touch. The lane is how you find
     the ones nobody has thought to list yet.

  9b. **⛔⛔ A NEW `Resource` OR `Message` IN A SIMULATED WORLD OWES ITS ROLLBACK
     DECLARATION, and only the app's oracles say so.**

     ```bash
     cargo test -p ambition_app --test app_it -- \
       rollback_coverage rollback_schema rollback_exit_oracle   # ~36 s
     ```

     ⛔ NO PER-CRATE RUN REACHES THEM. They live in `app_it`, against the
     composed app, so a crate's own suite is green while the declaration is
     missing. Contributed by ambition-df 2026-09-03 after those oracles caught
     TWO undeclared encounter resources from calculex's switch-loop split —
     declared derived at `2eaa0f479`'s parent. ⚠ A carve is not the only thing
     that trips this: any commit that adds simulated state does.

  10. **⛔ IF THE CARVE'S DESTINATION HOLDS DOC COMMENTS, add it to
     `check_doc_link_ratchet.py`'s `CRATES` — IN THE CARVE'S OWN COMMIT.** That
     list carries the instruction already ("when architecture moves out of a
     tracked crate, add its destination in the same change so the ratchet does
     not mistake reduced coverage for improvement") and cut 1 did not follow it.
     ⭐ The failure mode is the opposite of a red: the monolith read as
     **improved**, 63 → 59, because four broken doc links LEFT with the carved
     code and one arrived in an untracked crate. A ratchet that only ever falls
     is measuring its own shrinking population. Added after the fact at
     `bf4e6f353`; it is one line in the carve if you remember.
     ⚠ Reachable locally only since `3e85e4071` — it is a cold `cargo doc` over
     nine crates and lives in `./run_tests.sh --maintenance`, not the gate.

  ⛔⛔ RE-MEASURED 2026-08-31: the owner doc's
  "only four dependencies are single-path" list is STALE — `ambition_dev_tools`
  and `ambition_mount` have 6 dependents, `ambition_items` 5, `ambition_damage`
  3, and the FACADE every game depends on names all four directly. ✔ ALL FOUR
  RE-MEASURED 2026-09-02 late and still exact; ⚠ but `ambition_dev_tools`'s six
  is now 5 PRODUCTION + 1 dev-only, the dev-only one being the kernel itself. So removing
  the monolith's edge to any of them cannot shrink a product's closure, and
  footprint is retired as a rationale for these four; carve them for ownership
  and compile isolation or not at all. ⭐ SLICE TAKEN 2026-08-31 on
  `ambition_dev_tools`, the doc's own frontier (developer tools are a poor reason
  for the simulation kernel to depend upward). Two authorities moved:
  **construction now takes an engine `AbilitySet` instead of
  `&EditableAbilitySet`** — a LIVE-EDITABLE DEVELOPER TYPE was sitting in the
  production construction path, and who edits the set is the caller's business
  (four call sites convert with `.as_engine()`); and the **world-source
  hot-reload watcher moved to `DevToolsSimPlugin`**, resource and system
  together, carrying the reasons it runs in `Update` rather than the sim.
  ⛔ THE EDGE REMAINS, reported rather than claimed gone: four production
  references are left — `DeveloperRuntimeState` in `time/time_control` and
  `control/input_systems`, and `profiling::phase_mark` ×2 in `audio/plugin`. The
  phase marks are instrumentation and can go anywhere; the
  `DeveloperRuntimeState` pair are the kernel reading and writing dev state, and
  moving them means the dev crate owns those systems.
  ✔ THE WRITE IS GONE 2026-08-31: `cleanup_timers_system` decayed
  `dev_state.preset_flash`, a developer HUD timer, and that ONE LINE was the only
  reason the kernel's control module held a `ResMut<DeveloperRuntimeState>`.
  `ambition_dev_tools::decay_developer_presentation_flash` owns it, in the SAME
  schedule — its old home ran in `PresentationSync` so presentation timers decay
  while gameplay is suspended, and `Update` counts a different clock under a
  rollback host. Registration is guarded in the shipped app
  (`the_developer_hud_flash_still_winds_down`), red when the system is
  unregistered; a crate-local test cannot prove registration because
  `DevToolsSimPlugin`'s siblings need resources `ambition_dev_tools` does not
  depend on.
  ✔ AND THE READ IS GONE TOO, same day — the decision taken and stated rather
  than deferred. `update_time_scale_requests` reads `dev_state.slowmo` at rung 4 of
  a 5-rung priority ladder (twice: the main ladder and the no-primary-player
  path), so moving the SYSTEM would move the whole ladder out of the kernel,
  which is wrong. ⭐ THE INVERSION IS ALREADY BUILT: `ClockScaleRequest` carries a
  `ClockRequester::DevTool` variant that `RegimePolicy` already grants in `Solo`
  and denies in `RLDeterministic`/`Cinematic`, and `apply_clock_scale_requests`
  reduces by `min` — order-independent. So the dev crate publishes its own
  request and the kernel dropped both rungs. ⛔ TWO CHOICES MADE, both recorded
  where they bite: (a) `min` IS NOT THE LADDER — bullet-time's 0.5 used to
  outrank slow-motion's 0.25 by sitting at rung 2, and now the stronger slowdown
  wins, which is the right reading for a debugging override; and (b)
  `engine.ambition_dev_tools-manifest-allow` gained `ambition_time`, with the
  reason in its own rationale — the dep exists so the SIMULATION stops reading
  developer state, and `ambition_time` depends only on
  `ambition_platformer2d_core`. `debug_slowmo_scale` moved with the rung onto
  `DeveloperRuntimeState`. ⚠ WHAT IS LEFT is the two `profiling::phase_mark`
  calls in `audio/plugin` — instrumentation, not an authority — and ⛔ THE ROW'S
  OWN CLAIM THAT THEY *"CAN GO ANYWHERE"* IS FALSE, tested 2026-08-31. They
  BRACKET `load_music_cues` inside the audio plugin's `Startup` chain, so moving
  them means the dev crate ordering systems around a named system in the actor
  kernel — and `engine.ambition_dev_tools-source-purity` FORBIDS
  `ambition_platformer2d_actor_monolith` in that crate's source. Bracketing a
  SystemSet instead has the same problem: the set would live in the monolith.
  ⇒ THE ONLY WAYS TO REMOVE THIS EDGE ARE TO DELETE THE MEASUREMENT OR TO MOVE
  `phase_mark` DOWN, and both are carving for a NUMBER, which this row's first
  paragraph forbids.
  ✔ THE PHASE MARKS WENT ANYWAY, 2026-09-02 (`09f34b8d2`) — not by moving
  `phase_mark` down but by inverting the ordering: the plugin publishes
  `AudioInitSet` (in `ambition_platformer2d_shared_tangle`) and the HOST brackets
  that set beside every other mark, under the same two names so existing startup
  profiles still compare. Nothing in the dev crate names the monolith.
  ⛔⛔ **BUT "THE KERNEL HOLDS NO DEVELOPER AUTHORITY" IS FALSE, and this row said
  it while the owner doc had already been corrected — the same item in two files,
  fixed in one.** Counted on `main` 2026-09-02, three PRODUCTION reads remain and
  none is instrumentation:
  * ✔ **`features/npcs.rs` — CLOSED 2026-09-02.** `brain_override::forced_profile()`
    / `forced_preset()` were read while BUILDING A LIVE BRAIN; the value is a
    session-owned `AuthoredBrainOverride` now (in `ambition_characters::brain`,
    a crate the dev tools already depend on, so no manifest-allow entry was
    needed). `DevToolsSimPlugin` publishes it from the environment once at
    build; lowering carries it on `ActorPlacementContext` beside the cast and
    the policies. ⛔ THE TWO PROCESS-GLOBAL `OnceLock`s ARE DELETED, not made
    private — `runtime_census` was the only other reader and it takes the
    resource too, so the census now reports what was IN FORCE rather than what
    the environment said. ⛔ AND THE KNOB IS STILL LIVE ON ALL SIX ROADS:
    `for_room_construction` takes it as a PARAMETER, which is that function's
    own rule ("a road that forgot one link did not fail to compile"), so
    startup, transition, reset, provider activation, hot reload and the plan
    prefetch each had to state an answer. Guarded twice, both poison-verified:
    the value steers the cast and its absence does not
    (`the_developer_override_steers_the_cast_and_its_absence_does_not`), and the
    plugin publishes one at all
    (`the_dev_tool_publishes_the_brain_override`, app-level because the dev
    crate cannot boot its own plugin);
  * ✔ `features/ecs/spawn_static.rs` — CLOSED 2026-09-02 evening, the same
    inversion for the quota: the developer plugin publishes
    `AuthoredPopulationCap` (env parsed once in `ambition_dev_tools::
    population_cap::from_env`), `for_room_construction` takes it as a
    PARAMETER (all six roads state it), and the quota (`ActorAdmission`) is
    spent AT PLAN TIME in `RoomFeatureConstructionPlan::prepare`, before
    `plan_room` — so a refused NPC has no row, no authoritative root and no
    id, and a capped build of a room is a smaller plan rather than the same
    plan with lowerings that decline (the review's follow-up, closed the same
    evening; the lowering-time refusal and `begin_room_lowering` are gone with
    both statics). Guards: the quota's own arithmetic (`ambition_characters`),
    the cap planning the FIRST n NPCs with a fresh quota per plan and
    furniture not counting (`the_population_cap_is_spent_at_plan_time_and_
    each_plan_gets_its_own_quota`, poison: every record kept), and the plugin
    publishing it (app-level). The census row reports the cap IN FORCE (the
    resource): headless hall with `AMBITION_ACTOR_POPULATION_CAP=5` reads
    `actor_cap=5`, 6 bodies;
  * ✔ **`features/mod.rs:350` — `runtime_census` — CLOSED 2026-09-02, AND THE
    ROW'S OWN COUNT WAS WRONG.** It said *"the ONE production read left"*;
    counted on main, non-comment and non-test, there were **TWO**, and the row's
    *"none is instrumentation"* was wrong in the other direction — BOTH survivors
    were instruments. The missing one was `features/ecs/actors/update.rs:606`,
    `ambition_dev_tools::perception_census::note_world_view`, inside the
    `build_world_view` loop.
    ⭐ **AND NEITHER MEASUREMENT WAS DELETED TO REMOVE THEM**, which is the trap
    this row named twelve lines up (*"the only ways to remove this edge are to
    delete the measurement or to move `phase_mark` down, and both are carving for
    a NUMBER"*). Each went by publishing the thing DOWNWARD — the third way
    `AudioInitSet` had already taken:
    * the census marks needed to name `ActorDecisionSet`, which was `pub(crate)`
      in the kernel, so *"only this crate can name the sets"* was true. The enum
      moved to `shared_tangle` beside `WorldPrepSet` and `PlayerInputSet` (which
      the same function already imported from there), and
      `runtime_census::install_sim_phase_boundaries` installs all seven marks
      itself, beside the other twenty. The kernel still CONFIGURES the chain —
      where the sets sit is its business and moved nowhere;
    * `perception_census` is counted in a hot loop, so the number cannot be
      recovered from outside; the 91-line COUNTER moved down to
      `ambition_characters::perception::census` (the crate that defines
      `WorldView`, which is what it counts) and the developer crate still owns
      `enable`, `drain` and the report.
    ⭐⭐ **SO `ambition_dev_tools` IS OFF THE KERNEL'S `[dependencies]` — it is a
    `[dev-dependency]` now**, which the `#[cfg(test)]` live-refresh and reset
    tests still need. ⛔ AND THE MOVE IS ITS OWN GUARD, with no test to rot:
    a production `use ambition_dev_tools::…` in that crate's `src/` no longer
    COMPILES. Poison-verified by adding one — `error[E0433]: cannot find module
    or crate `ambition_dev_tools` in this scope`.
  The first two were the simulation reading developer state to decide what the
  world contains, which is precisely the authority this carve exists to remove;
  the last two were instruments, and the inversion kept both.
  ⇒ **THE `Cargo.toml` PRODUCTION DEPENDENCY IS GONE and the dev_tools slice IS
  CLOSED.** ⚠ What it did NOT buy is a smaller product closure: this row already
  re-measured that `ambition_dev_tools` has 6 dependents and the facade names it
  directly, so nothing downstream drops it. The payoff is ownership — the
  simulation kernel no longer names the developer crate, and cannot again
  without a manifest change.
  ▢ **The next slice's shape, agreed 2026-09-02 and deliberately not started
  without review**: *the sim reads a SESSION-owned override that the dev tool
  WRITES, never the dev crate itself* — the same inversion `ClockScaleRequest`
  already demonstrates for slow-motion.
  ⚠ **What each actually read, checked rather than assumed**: `forced_profile()`
  WAS a `OnceLock` over an env var — a hidden process-global INPUT, constant once
  resolved, and gone as of 2026-09-02. `admit_actor()` WAS more than a read: an
  `AtomicUsize::fetch_add(Relaxed)` that decided whether a placement spawns —
  gone the same evening (see above).
  ⭐ AND IT IS INERT BY DEFAULT — `cap()` resolves to `usize::MAX` with the env
  knob unset and `admit_actor` returns `true` before touching the counter, so
  the mutable global is live only in a developer scenario. ⇒ This is an
  OWNERSHIP problem first. The determinism question (a process-global counter
  that no rollback rewinds, deciding which actors exist) is real but scoped to
  runs with the cap set, and should be stated that way rather than as a shipped
  desync risk.
  ⭐ AND THE OTHER HALF OF THAT FRONTIER WAS MEASURED BEFORE PICKING IT UP —
  *"presentation dependencies"* is mostly a mirage. The kernel's production refs
  are `sfx` 160, `vfx` 103, `audio` 70, `conversation` 33, `cutscene` 16, and
  ⛔ NONE of those crates pulls `bevy_render`, `bevy_audio`, `bevy_ui`,
  `bevy_sprite` or `bevy_text`: they are foundation VOCABULARY crates consumed
  DOWNWARD, and a body emitting its own hit cue is the semantic fact rather than
  a presentation reach. ⚠ `ambition_dialog` and `ambition_sim_view` are
  DEV-dependencies, so a manifest count that ignored section headings would
  report two production edges that do not exist. ⛔ and `cutscene.rs` already
  carries its own defence — gameplay-coupled to rooms, save and schedule, above
  a crate that must stay gameplay-free — so moving it needs a THIRD crate.
  ⇒ THE NEXT D33 SLICE SHOULD COME FROM THE DOMAIN FRONTIERS (items, mounts,
  encounter/conversation orchestration, character preparation), not from
  developer/presentation. Recorded in the owner doc.
  Owner:
  [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

  ⭐ SIZED 2026-09-02 evening, for the NEXT slice (not started): the kernel's
  domain-frontier edges by reference are `ambition_encounter` 66 refs / 24
  files, `ambition_mount` 57 / 17, `ambition_conversation` 45 / 13,
  `ambition_items` 34 / 16 (e7). The items edge is not a leaf: the kernel's own
  `items/` module is ~6,000 lines (pickup, minted horizon, narrative, persist,
  conditions) importing `abilities::ranged` ×10, `features`, `durable_horizon`,
  `traversal`, `character_runtime`, `shrine` and `construction`, and the rest
  of the kernel names `items::` 79 times — a multi-day carve, not an evening's.
  The evening's D33 slice was the brain-override inversion (agent 383484,
  `AuthoredBrainOverride` in `ambition_characters::brain`; see its commits).
  ⭐⭐ **AND A FIRST `items/` SLICE LANDED 2026-09-02 LATE:
  `ambition_world_items`.** The row above calls `items/` *"~6,000 lines … a
  multi-day carve, not an evening's"*, which is the UNION of every file's
  imports and describes the worst file. Counted PER FILE — non-`crate::items`
  references into the rest of the kernel — `pickup/mod.rs` holds **27 of 51**
  and `item_motion.rs` holds **none**. So the multi-day claim is true of
  `pickup/` and false of the rest.

  ✔ **WHAT MOVED:** `world_item.rs` + `item_motion.rs` and their tests — the
  PHYSICAL life of a touched collectible: where it is, whether it is moving, and
  that walking into it collects it. **14 tests moved with it and all 14 pass in
  the new crate**; the monolith went 1221 → 1207, which is exactly them.
  ⛔ **WHAT STAYED, and the split is by TRIGGER not by size:** `items::pickup`
  owns the PRESSED pickup — a held weapon taken with `Attack` — and still
  reaches `abilities`, `ability_cooldown`, `construction` and `shrine`. That is
  the line the pickup module's own `AMBITION_REVIEW(discrete_ok)` note had
  already drawn.

  ⭐ **THE COUPLING THAT LOOKED LIKE A BLOCKER WAS THE CARVE'S OWN LEVER.** The
  collect pass named `features::ecs::pickups::TouchCollectorFilter`, which is a
  type alias composed of nothing but `PlayerEntity` and `TemporaryControl` —
  both already in `shared_tangle`. It moved down beside them, and so did its
  VALUE twin `body_collects_on_touch`; `pickups.rs` re-exports both under the
  short names its three passes read. ⛔ ONE DEFINITION, not a copy: the filter
  decides who a query RETURNS and the value check decides whether a returned
  body collects, so a second copy is how the halves come to disagree.
  ⚠ **AND MY OWN PER-FILE COUNT UNDERCOUNTED, which is worth more than the
  slice.** I sized `world_item.rs` at TWO kernel references using
  `grep -o "crate::[a-z_]*"`. That grep cannot see `super::` paths or a
  fully-qualified call, and the file also reached `super::item_motion` four
  times and called `pickups::body_collects_on_touch`. ⇒ **the same
  one-form-grep error the frontier row above records, made by the person who
  wrote that row, four hours later.** Every reference resolved downward so the
  slice held, but the number was produced by an instrument blind to half its
  subject.

  ⛔ **NOTHING IS RE-EXPORTED FROM `items/mod.rs`.** Keeping
  `pub use ambition_world_items::{world_item, item_motion};` there would have
  meant ZERO consumer churn; it was refused because a re-export keeps the kernel
  as the discovery path for code it no longer owns, and then the boundary is not
  greppable. For the same reason games reach it through the facade's new
  `ambition_platformer2d::world_items` rather than `actors::items` — `actors`
  IS the monolith.
  ⭐ The runtime composes `WorldItemSimulationPlugin` beside
  `ItemPickupSimulationPlugin` (its sibling across the split), so no
  registration for the domain lands back in the kernel — the same shape as
  `ambition_mount` and `ambition_damage`.

  ✔ **GUARDED, both poison-verified:**
  `engine.ambition_world_items-source-purity` (adding the monolith to the
  crate's source trips it) and `engine.ambition_world_items-manifest-allow`
  (adding `ambition_dev_tools` to its manifest trips it).
  ✔ **AND THE LEDGER FEAR WAS CHECKABLE AND FALSE.** Both types are
  rollback-registered, so a crate move looks like it should churn a path-keyed
  baseline. `rollback_schema_baseline.txt` keys rows by the registrar's OWNER
  STRING and short type name — `entity:world_item`, `item.motion`,
  `item.world_item` — so the file is UNCHANGED and only the `use` paths in
  `rollback_registration.rs` moved.
  ⛔⛔ **THE FOOTPRINT RATCHET FIRED, AND IT COUNTS CRATES, NOT BYTES.**
  `capability-footprint-baseline.json` went 43 → 44 and the growth is declared
  in the same idiom as the `mount`/`damage` rows. ⚠ THE SAME CODE WAS LINKED
  BEFORE, inside the monolith — the linked code went slightly DOWN. A reader
  taking `closure_size` as a size metric reads a carve as a regression, which is
  the one thing that row shape must never be allowed to say.

  ⭐⭐ **AND THE SIBLING IS NOT A MULTI-DAY CARVE EITHER — the diagnosis was
  wrong twice, measured 2026-09-02 late.** `items/pickup/mod.rs` is the file that
  holds 27 of the module's 51 references into the rest of the kernel, and that is
  why it was called *"a multi-day carve, not an evening's"*. Split the file at
  its `impl Plugin for ItemPickupSimulationPlugin` block:

  ```text
  items/pickup/mod.rs           1,860 lines
    the plugin block (53-253)     201 lines   21 of 23 cross-module refs
    everything else             1,659 lines    2 of 23
  ```

  ⇒ **THE ENTANGLEMENT IS 201 LINES OF SCHEDULING, NOT 1,659 LINES OF LOGIC.**
  The plugin names `crate::abilities::{ranged ×10, traversal ×4, thrown ×3}`,
  `crate::shrine` ×3, `crate::construction` ×2 and `crate::ability_cooldown` —
  every one of them a SYSTEM NAME being placed in a schedule, not a call the
  pickup logic makes. The pickup domain itself reaches out exactly TWICE, both at
  one seam: `crate::construction::authored_occurrence_request` and
  `ActorConstructionParams::GroundItem`, spawning a ground item from an authored
  occurrence (lines 993 and 1000).

  ⛔ **SO THE "27 REFERENCES" NUMBER MEASURED THE PLUGIN AND WAS READ AS THE
  DOMAIN.** That is the third sizing error in this row's history and they share
  one shape — a count taken at the wrong granularity: the UNION of a module's
  imports read as one file's, a re-exported name read as kernel coupling, and now
  a plugin's system list read as its domain's dependencies.
  ⚠ It is also exactly what the `ambition_world_items` slice hit in miniature and
  solved: the two systems it moved were registered inside the entangled file, so
  the split was "move two files AND the registrations that belong to them". The
  same move is available here at ten times the size.
  ⭐⭐ **AND THE THIRD CUT MAKES IT EXACT.** The two seam references are both
  inside ONE function, `restore_custody_to_checkpoint` (lines 775-1089) — which
  is checkpoint restore reading authored construction records, arguably not the
  pickup domain at all. Bound the plugin AND that function and measure what is
  left:

  ```text
  items/pickup/mod.rs                        1,858 lines
    impl Plugin block          (51-251)        201 lines   21 refs
    restore_custody_to_checkpoint (773-1087)   315 lines    2 refs
    everything else                          1,342 lines    0 refs
  ```

  ⛔ **CHECKED IN ALL THREE PATH FORMS, because this row has already been wrong
  twice by grepping only one.** The 1,342-line remainder contains exactly ONE
  `crate::` reference — `crate::items`, its own parent module — and ZERO
  `super::` paths. It names nothing outside itself.
  ⚠ **RE-VERIFIED AFTER THE DAY'S OTHER COMMITS and the figures moved by two
  lines** (1,860 → 1,858; 1,344 → 1,342), because `ItemPickupSet` left this file
  for `shared_tangle` and a shorter re-export replaced the enum. The SHAPE — 201
  plugin lines holding 21 of 23 refs, the checkpoint function holding the other
  2, the remainder holding none — is unchanged. ⇒ Re-derive the line numbers
  before cutting; they are the one part of this row that moves under ordinary
  work.

  ⇒ **THE CARVE IS: move 1,344 lines, leave 516.** Ground-item physics, custody,
  pickup, throw, held-item specs and aim are free-standing; the entanglement is a
  plugin that schedules its neighbours' systems and one checkpoint function that
  reads authored records. Neither has to move for the domain to leave, and
  neither needs an inversion designed first — which is what "one construction
  seam to resolve" (written an hour earlier, on the coarser cut) got wrong.
  ⚠ The tests are a separate 1,789 lines (`pickup/tests.rs`) and are NOT included <!-- cite-ok: the pre-cut path (moved to ambition_held_items 2026-09-03), kept as the record -->
  in that count; they move with the code and are most of the remaining work.
  ⚠ And the consumer surface is real: games name `actors::items::pickup::` about
  twenty times across `ambition_app`'s tests and the smash demo, so the carve ends
  with a facade export and a re-point, exactly as `ambition_world_items` did.

  ✔ **CUT 2026-09-03: `ambition_held_items`.** The partition held as
  measured (the plugin block and `restore_custody_to_checkpoint` stay; the
  domain, `conditions.rs` and the tests move; `minted_horizon.rs` stays) with
  one addition the checklist did not have: the `CoreHeldItems` chain was
  thirteen links with the kernel's shrine, gun and match spawn INTERLEAVED
  between the domain's steps, so `HeldItemStep {Release, Pickup, Use, Throw,
  Settle, Physics, Residency}` became `shared_tangle` vocabulary first
  (`4aabf8259`, guard by shape in the kernel) and the domain moved with its
  chain intact. `HeldItemSimulationPlugin` configures `CoreHeldItems` end to
  end (phase, custody edge, the seven steps) and registers its ten systems;
  the kernel's `ItemPickupSimulationPlugin` keeps the two sibling variants,
  the three-variant chain and its three attachments (`.before(Release)`;
  `after(Use).before(Throw)`; `after(Throw).before(Settle)`). Guards:
  `ambition_held_items::schedule_tests` (the plugin alone: phase, custody
  edge, chain, one system per step, gating) and the kernel's
  `held_item_steps` (its attachments, beside the domain's plugin). Rollback
  baseline byte-identical; kernel 1,182 tests (31 moved with the domain, all
  green there); footprint 45 → 46 declared. The name-based enumeration test
  that moved could only pass by another crate's `bevy_ecs/debug` and was
  replaced by the count guard. The execution order below is kept as the
  record of what it predicted.
  ▢ **THE EXECUTION ORDER, so this is startable rather than merely sized.**
  Written after `ambition_world_items`, whose one surprise was step 2 — the
  systems' `configure_sets` rules, which live far from the `add_systems` line and
  which that carve dropped (see the regression in the owner doc).
  1. **Read the plugin's `configure_sets` FIRST and write down every set the
     moving systems join, what each set is nested `in_set`, and every
     `.before`/`.after` edge it carries.** ⛔ This is step one and not step four
     because skipping it is how `world_items` lost `.in_set(PlayerSimulation)`
     and `.after(BodyCustodySettled)` while its comment claimed otherwise.
  2. The new crate takes everything in `items/pickup/mod.rs` EXCEPT the plugin
     block and `restore_custody_to_checkpoint` — ⚠ bound them by NAME, not by
     the line numbers above, which have already moved once, plus
     `pickup/conditions.rs` (80 lines, zero `crate::` references) and <!-- cite-ok: the pre-cut path (moved to ambition_held_items 2026-09-03), kept as the record -->
     `pickup/tests.rs`. ⚠ `pickup/minted_horizon.rs` STAYS for now: its single <!-- cite-ok: the pre-cut path (moved to ambition_held_items 2026-09-03), kept as the record -->
     kernel reference is `session::durable_horizon::SaveRestored`, a one-field
     bool with 40 references across 13 files, which is its own move and not this
     one's.
  3. The kernel keeps a file holding the plugin and the checkpoint function,
     importing the moved types. ✔ Verified feasible: neither calls a
     file-private helper — the only non-`pub` functions in the file are `build`
     (the `Plugin` trait method) and a `map_entities` trait impl.
  4. `ItemPickupSet` is already in `shared_tangle::schedule` (`b80598c01`), so
     the kernel's plugin and the new crate can both name it without either naming
     the other.
  5. Facade export + re-point the ~20 game references, as `world_items` did.
  6. The usual tail, all of which `world_items` needed and none of which its
     first pass remembered: two policy rows (manifest-allow + source-purity,
     both poison-verified), `scripts/modules_md.py --write`, the two
     sub-workspace lockfiles, and a declared entry in
     `capability-footprint-baseline.json` — ⚠ the ratchet WILL fire, because it
     counts crates.
  ⚠ **AND THE ONE THING THAT IS NOT MECHANICAL**: `restore_custody_to_checkpoint`
  stays in the kernel while the types it operates on leave, so it becomes a
  kernel system reading a foreign crate's components. That is legitimate — it is
  checkpoint policy, not item policy — but it should be stated in its doc
  comment, or the next reader will "fix" it by dragging it after the domain.

  ✔ **THE SCHEDULE-OWNERSHIP FORK IS SETTLED, AND THE CUT IS SPEC'D END TO END:**
  [`engine/pickup-carve-checklist.md`](engine/pickup-carve-checklist.md).
  Starting the cut and stopping is what found the fork — the partition is clean
  (511 lines stay, 1,347 move, verified to reconstruct the file) and the blocker
  was never code: the moved tests build a plugin that STAYS, and every way of
  splitting a set's rules from its systems loses something, one of them
  silently. ⇒ Answered by the D33 rule in
  [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
  — the carved crate's plugin does BOTH its `configure_sets` and its
  `add_systems`; a set two crates order on is `shared_tangle` vocabulary with
  exactly one configuring owner — plus the one thing that rule does not cover,
  which the checklist carries: the three `ItemPickupSet` variants are `.chain()`ed
  to each other and that INTER-VARIANT edge stays with the kernel.
  ⭐ Worked precedent, the sibling carve now closed: `d220accee` (the fix) and
  `dbec94824` (the guard — phase MEMBERSHIP by set-member COUNT, because Bevy
  0.19 strips system names without `bevy_ecs`'s `debug` feature and a name lookup
  is green or red depending on who else is in the build).
  ⇒ **And it WAS mechanical once the fork was settled: cut 2026-09-03 as
  `ambition_held_items` — the receipt is above.** ⚠ This paragraph predicted
  "what remains is a CPU job rather than a design one" and that prediction is
  spent; it is kept only because the sequence is the transferable part — the
  cut was blocked by ONE design question, the question was answerable in
  minutes once someone stated the three branches and their costs, and the 3,454
  lines that followed needed no judgement at all.

  ⛔⛔ **AND THE "SIZED … BY REFERENCE" LINE ABOVE POINTS AT NOTHING, MEASURED
  2026-09-02 LATE.** Those counts (`ambition_encounter` 66, `ambition_mount` 57,
  `ambition_conversation` 45, `ambition_items` 34) invite a carve chosen by
  reference count, which THIS ROW'S FIRST PARAGRAPH FORBIDS — and re-counting
  non-comment references and then reading what each one IS says the row's own
  "presentation dependencies are mostly a mirage" finding applies to the domain
  frontiers too:

  | edge | non-comment refs / files | what they are |
  |---|---|---|
  | `ambition_encounter` | 74 / 24 | `SwitchActivation`, `EncounterSpec`, `EncounterWaveBook` — types and helpers |
  | `ambition_mount` | 69 / 17 | `MountSlot`, `RidingOn`, `Mounted`, `CanPilot`, `rider_of` — components and helpers |
  | `ambition_conversation` | 48 / 13 | `ActiveConversation`, `NarrativeMusicRequest`, plus registrations |
  | `ambition_items` | 40 / 17 | types and helpers |

  ⭐ **NONE OF THE FOUR IS AN AUTHORITY EDGE.** Every reference is the kernel
  CONSUMING a downward vocabulary crate — components it stores, types it matches,
  helpers it calls — which is the same shape as `sfx`/`vfx`/`audio` above and is
  correct by this doc's own reasoning. ⛔ A reference count cannot tell that from
  an authority read, which is exactly how it produced a four-item worklist with
  nothing on it.
  ⭐ **THE SWEEP THAT DOES DISCRIMINATE: who does the kernel register plugins
  FOR?** That is the anti-god-rule smell the dev-tools and audio slices both
  turned out to be. Across the whole kernel there are **FIVE** foreign plugin
  registrations, and every one is already accounted for.
  ⚠ COUNTED TWICE, because the first sweep was wrong: grepping
  `add_plugins(ambition_x::…)` misses any plugin brought in by `use`, and
  re-running over every `*Plugin` name inside an `add_plugins(…)` turned up two
  more candidates. Both dissolved on inspection — `CharacterCatalogPlugin`
  appears only in `character_roster/tests.rs`, and `PhysicsPlugin` was a
  substring hit on avian's `PhysicsPlugins::default()` inside the kernel's own
  `AmbitionPhysicsPlugin`. A one-form grep would have published four or six:
  * `ambition_conversation::ConversationPlugin` and five
    `NarrativeInputPlugin::<T>` — deliberate, and `FeatureInteractionSchedulePlugin`
    states why in place: *"A payload belongs to whoever CONSUMES it: three of
    these are `features` types that a carved-out conversation crate could not
    name at all."* The conversation domain already installs itself;
  * `ambition_characters::brain::BrainPlugin` — brains are the kernel's subject,
    not a foreign facility;
  * three `ambition_audio` plugins, all inside the monolith's OWN
    `Platformer2dAudioPlugin`, which the host adds through
    `add_presentation_plugins` and which the `audio` feature removes wholesale.
  ⇒ **There is no misplaced foreign registration left in the kernel.** The
  dev-tools slice was the last one, which is why it closed.
  ⇒ **SO THE NEXT D33 SLICE IS NOT AN EDGE REMOVAL AT ALL.** What is actually
  left is INTERNAL: this row already names it — the kernel's own `items/` module
  at ~6,000 lines, named 79 times by the rest of the kernel, importing
  `abilities::ranged` ×10, `features`, `durable_horizon`, `traversal`,
  `character_runtime`, `shrine` and `construction`. That is a decomposition of
  the monolith's INSIDE, with no manifest edge to delete at the end of it and no
  compiler-held guard like the dev-tools one. ⚠ Whoever takes it should know that
  going in, because every previous slice ended with an edge to point at.
- ▢ **D166 — make the character-authoring boundary load-bearing where a real
  character still bypasses it.** Prepared character definitions are already
  immutable and the first Smash fighter facet exists. Re-measure the current
  residuals before migrating another field. The startup-reach proxy is a
  maintainer decision (§35), not an excuse to widen generic character data.
  ⭐ CENSUSED 2026-08-31 against the owner doc's five-part test, ten sites; most
  are two LEGITIMATE authors (a demo mechanic keyed on identity, a match rule
  composed through `MatchRules`) and are not targets. ✔ FIRST SLICE CLOSED: the
  Smash demo's `smash_reading_of_character` — a `match definition.id` writing
  `Vitals::knockback_weight` for a character it does not own — is deleted, and
  George states his own 1.35 in `smash_fighter.ron`. ✔ SECOND SLICE CLOSED
  2026-08-31: Mary-O's `sheet_target` — a `match character_id` restating a
  pairing the demo had already authored three other ways — is deleted;
  `clip_seconds` asks `sheet_for_character_id_from_data`, the join the engine
  already owns. ⚠ its resources are `Option<Res<_>>` so a narrow fixture can
  still swap her form, and an app-level guard
  (`every_mary_o_form_resolves_a_real_sheet_in_the_shipped_demo`) is what stops
  that option becoming a silent veto. ⛔ AND THE THIRD CANDIDATE IS RETRACTED, same day it was filed:
  the `canonical_height` / `standing_height` pair is NOT two live truths for one
  fact — their populations are DISJOINT (18 catalog rows author a standing
  height; neither caller of `with_canonical_height` is among them). What is true
  is smaller: `canonical_height` is read by nothing in gameplay, because the
  scaling its doc claimed happens at authoring time and the OUTPUT is what gets
  stored. Its doc now says so; the field stays, because `moveset_export` reports
  it and that is what a record is for.
  ⭐⭐ AND THE HEADLINE ANSWER, MEASURED 2026-08-31: **the generic repertoire
  floor has ZERO adopters.** `select.rs` said two seated fighters author no
  repertoire and that seating them raised the floor's count "from three to five".
  Both author one now, and so does every other id on the roster — the three with
  no `authored/<id>.rs` (`player_robot_v3`, `mary_o_tall`, `sanic`) author theirs
  from their own demo crates. So on the Smash roster the character-authoring
  boundary IS load-bearing, which is what this row was asking. ⚠ this does not
  retire `SMASH_FIGHTER_KIT`: it is an `AbilitySet` and stays live as an ability
  GRANT — a different job from the repertoire fallback the comments described,
  and the two were being conflated by name.
  ✔ ALSO CLOSED, a second false contract found by the same census: `duel_arena`
  told maintainers its two fighters' `name` MUST track the catalog `display_name`
  because the sheet and the authored hitbox both resolved from it. Measured, and
  false twice — `new_character_in` takes `display_name` off the CHARACTER's
  blueprint and is never handed the request's string, while the sheet binder and
  `authored_attack_volume_resolver` both key on `sprite_character_id` (the hitbox
  resolver has no name road at all; its `None` arm falls to the PLAYER's hitbox).
  ⇒ `SpawnActorRequest.name` is INERT for any request naming a character. Pinned
  by `a_staged_actor_naming_a_character_takes_the_characters_label_not_its_requests`,
  which is attributable because its catalog is EMPTY and could join nothing.
  Seven comments cited two identifiers that do not exist (`smash_fighter_kit()`,
  `character_id_for_display_name`); all seven now name what is really there.
  ✔ RE-CENSUSED 2026-09-02 AND STILL NO CANDIDATE — the sweep is recorded in the owner doc rather than left as an instruction. Three populations searched (id-keyed branches; writes to `definition.vitals`/`.locomotion`/`.movement_tuning` outside `authored/`; demo writes to facts a demo does not own) and every hit is a character authoring its OWN facts. ⚠ ONE ASYMMETRY, explicitly NOT a slice: `CharacterDefinition` has 22 `with_*` builders and none for `vitals`, so every character assigns the public field — an ergonomic gap with no second road to delete, and the five-part test needs one. ⛔ NOT a residual: eleven grid fighters on the
  actor baseline is a MISSING author, not a duplicate one.
  Owner:
  [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

- ▢ **D129 — repair player-visible sprite clipping through authored geometry,
  using the existing build-time guard.** The historical "52 of 196" count is a
  stale snapshot; the later full render census also moved as art was repaired.
  Re-run the current target, start with player-visible/selectable characters that
  still fail, and fix the authored canvas/pose/geometry rather than weakening the
  guard. Do not infer a roster-wide scale rule from one character's repair.
  ⭐ RE-SWEPT 2026-08-31, POPULATION-COMPLETE — every one of the 209 discovered
  targets rendered through its OWN road (`Target.render_sheet`), reading the
  build-time guard's own warning: **38 targets, 405 frames**, and every one is a
  `module` target — all 100 config-driven targets are clean. ⛔⛔ A FIRST PASS
  MEASURED 42 OF 209 and looked plausible: it reimplemented the loop over
  `module.ROWS` + `module.render_frame`, which 116 targets do not export. A
  sample is not the population.
  ✔ `perfect_cellular_automaton` — the 2026-08-26 note's "the one that matters
  (53 frames)" — is CLEAN, 0 of 913. That claim is spent.
  ✔ `carl_stargan` (8 frames) FIXED, and it was a real cut: the shared scientist
  `roll` drops the body 38-42px below `ground_y` at the tuck and only his canvas
  had no room, because he restated the rig frame as his published frame while
  `patent_clerk` and `noether` both add `RIG_RENDER_PADDING`.
  ⚠ AND THE GUARD CRIES WOLF ON FLUSH-STANDING PIXEL ART — measured, not
  assumed. The Mary-O family (41 frames across six forms of the playable
  protagonist) reports `bottom` because `bottom_center_canvas` seats a 24x32
  logical sprite flush on the frame's last row and a flat pixel-art foot is not
  a taper; re-rendering with logical headroom finds at most TWO raster rows of
  ink beyond the boundary. ⛔ do NOT inflate those canvases — it moves every
  Mary-O sprite's ground contact to silence a false positive. `pirate_admiral`
  (2 frames) is the same class at the top edge: his plume touches row 0 on ALL
  six idle frames and the guard fires on the two where it is 7px wide instead of
  0. ⇒ the remaining targets are a POPULATION, not a defect list; the open
  work is an instrument that separates "drawn flush to a fitted frame" from
  "severed", not a stack of canvas edits.
  ⚠ **THE "35" THIS SENTENCE CARRIED IS SUPERSEDED** — it was 38 minus the two
  false-positive classes, over a sweep that could not render 22 SVG targets. The
  measured population is **34 flagged / 465 edges over all 209**, below, and the
  membership differs (`hunny_horror_boss` alone is 59 edges nobody had counted).
  Left as prose rather than a number so it cannot go stale again.
  ⭐ THE INSTRUMENT LANDED 2026-09-02, and it RANKS because the separation it was
  asked for CANNOT BE MADE. `tools/ambition_sprite2d_renderer/scripts/measure_clip_population.py` <!-- cite-ok: on an unmerged submodule branch, deliberately -->
  (the renderer submodule's script, not this repo's `scripts/`), branch `d129-composited-frames` @ `c6a9712` — ⚠ NOT in the
  checked-in submodule pointer (`125adf8`), deliberately: the pointer is Jon's to
  move. ⛔⛔ **The drawing canvas IS the logical frame, so the ink beyond it was
  never rendered.** The obvious classifier — "seated flush" vs "still moving when
  cut" — was written first and does not survive its own data: `pirate_admiral`'s
  plume (a TIP) and `super_sanic`'s historical spike (a CUT) have the same
  profile and both land exactly on the guard's 0.5 threshold.
      pirate_admiral top   7  9  9 10 10 11 14   ratio 0.50  <- a tip
      super_sanic idle    12 14 17 18 20 22 25   ratio 0.50  <- a cut
      mary_o idle         24 24 24 24 24 24 24   ratio 1.00  <- a flat sole
  A classifier that answered anyway would be INVENTING the distinction, and here
  an invented distinction means padding canvases and moving ground contact on the
  strength of it. So the instrument reports the stake (ink at the boundary over
  the edge's length) and the guard's own ratio, ordered by their product. It is a
  WORKLIST ORDER; no row of it is evidence that a frame is wrong.
  ⭐ MEASURED 2026-09-02, 209 targets in 452s: **27 flagged, 362 edges, 22 NOT
  MEASURED.** Four findings:
  * **`smirking_behemoth_boss` (101 edges, severity 1.000) is where a human
    should look first** — every edge fully opaque on `rest#0-5`, `bottom
    [208,208,208,208,208,208,208]` on a 208-wide frame. ⚠ AND IT IS THE
    RANKING'S OWN CAVEAT: that is either a full-bleed sprite by design or
    something badly cropped, and the silhouette cannot say which.
  * **THE FIVE PIRATES ARE ONE AUTHORED PLUME, NOT FIVE DEFECTS.**
    `pirate_admiral`, `pirate_lookout`, `pirate_navigator`,
    `pirate_quartermaster` and `pirate_raider` all report the IDENTICAL profile
    `[7,9,9,10,10,11,14]` on `top`, `idle#1` and `idle#2`. Five rows of the
    guard's report, one authoring decision — they come off the list together or
    not at all.
  * **A SECOND FALSE-POSITIVE CLASS THE COMPOSITED-WHOLE MARKER DOES NOT COVER:
    the `super_mary_o` TILE PROPS.** `pipe_body` / `pipe_top` / `flag_pole_body`
    / `flag_pole_top` are 32x32 and 64x32 tiles with dead-constant profiles
    (`53 53 53…`, `9 9 9…`). ⛔ A pipe body is SUPPOSED to be flush top and
    bottom — that is what tiling means. They are DRAWN rather than composited, so
    the marker never sees them. Do not pad a tile to silence this.
  * ⛔ **A KILLED HYPOTHESIS, recorded so the next reader does not re-derive it.**
    `davy_hylbert`, `paul_diracula` and `pipi_tau` show the fingerprint
    `carl_stargan` showed before its fix (`crouch#0-5` bottom, `roll#2` left,
    `roll#5` right, `death` bottom/left), and `RIG_RENDER_PADDING` has exactly
    four adopters (`carl_stargan`, `noether`, `noether_gameplay`,
    `patent_clerk`) with zero in those three. It reads like the same unfixed
    defect. IT IS NOT: none of the three imports `canonical_scientist_rig`; they
    are independently hand-authored 128x128 targets calling `build_sheet`
    directly. What they share is the MOVESET VOCABULARY — the extreme poses
    (crouch low, roll wide, death sprawl) exceed a canvas sized for the idle.
    That is a more general pattern than a shared rig, and it does NOT license
    `carl_stargan`'s fix as a template.
  ✔ **THE 22 ARE MEASURED NOW, and the population was 7 targets short.**
  Re-run 2026-09-02 with `resvg-py` — which is a DECLARED dependency of the
  renderer (`pyproject.toml: "resvg-py>=0.3"`) that was simply absent from the
  interpreter being used; `tools/ambition_sprite2d_renderer/.venv` already had
  it. Nothing installed. **209 targets in 1007s, 0 would not render: 34 flagged,
  465 edges** (was 27 / 362 over the 187 that rendered without it).
  ⭐ **`hunny_horror_boss` is the biggest thing the gap was hiding** — 59 edges,
  severity 0.402, which places it THIRD on the worklist behind
  `smirking_behemoth_boss` and the pipe props. It had never appeared in any D129
  count. `paradox_barber` (9), `data_lovelace` (14) and `charley_beagle_svg` (1)
  are also new.
  ⚠ **AND THE `mary_o_v2` SVG LINEAGE IS IN THE POPULATION**: `mary_o_v2` (12
  edges), `mary_o_v2_tall` (5), `mary_o_v2_fire` (3). ⛔ These are NOT the
  composited pixel-art Mary-O forms whose flush-standing was established as a
  FALSE POSITIVE — different targets, a different authoring road, and their
  profiles are tapering rather than flat. They want the same care before anyone
  pads a canvas, and the same conclusion should not be assumed to carry.
  ✔ Confirmed clean in the full sweep, so their earlier "not measured" is now a
  real result: `perfect_cellular_automaton`, `noether`, `patent_clerk`,
  `carl_stargan`, `player_robot_v3`, `officer`, `medic`, `oiler`, `performer`,
  `m_leblanc`, `neil_ongras_turfson`, `author` and the three polygon targets.
  ⇒ The instrument's caveat is discharged: there is no longer a "of the N that
  render here" qualifier on this row, and the worklist is the whole roster.
  ⭐ **THE RESULT IS ON THE RECORD, not just in this row**:
  `ambition_dev_measurements` branch `sprite-clip-census-20260902` @ `c0e3889`,
  `summaries/sprite-clip-census-20260902.md` <!-- cite-ok: a path in ambition_dev_measurements @ c0e3889, not this repo --> — the run, its provenance, and the
  two caveats a reader must not lose. ⚠ The 228K per-edge JSON is deliberately
  NOT committed: that repo ignores `profiles/` by design and tracks only the
  readable half. ⛔ Neither submodule pointer is bumped — the instrument
  (`d129-composited-frames` @ `c6a9712`) and the census (`c0e3889`) are both on
  pushed branches awaiting the maintainer's call.

- ✔ **D-SFX-RESET-RED — CLOSED 2026-08-31, and the fixture pressed one frame too
  early.** `ambition_app`'s own lib suite had a long-red test that no gate in
  this rhythm covered: every gate runs `--test app_it`, the INTEGRATION target,
  which does not build `src/`'s `#[cfg(test)]` modules. `--lib` was 191/192 and
  is 192/192.
  ⭐ THE CAUSE: `settle_until_session_world` stops as soon as a session WORLD
  exists, which is one frame before the session is DRIVABLE —
  `resolve_controlled_subject` has not yet copied the seat's
  `DrivingParticipant` into `ControlledSubject`. The reset resolves its subject
  from that resource, so a press on the gap frame found `None`, took the
  replay's *"no body, no crossing to describe"* arm and reset nothing. One
  `app.update()` before the press fixes it.
  ⛔⛔ AND THE OBVIOUS REPAIR WAS THE WRONG ONE, which is why this took a TRUTH
  TABLE rather than a fix: staging a `DrivingParticipant` by hand does NOT work
  without the extra frame (the resolver still has not run), and the extra frame
  works WITHOUT the staging (a body was already seated). The missing thing was a
  FRAME, not a SEAT — and a plausible fix that passes for the wrong reason would
  have hidden that.
  ⛔ FOUR WRONG HYPOTHESES BEFORE IT, all recorded rather than tidied away: the
  timing window (the test read ONE frame and its doc claimed the cue was
  "synchronous"); the reset specifically (ZERO cues of any kind); the wrong
  resource (a direct write IS visible); a buffer-swap artifact (`len()` counts
  both buffers and was 0). ⛔⛔ plus one bad INFERENCE of mine — I read the
  absence of the resolver's *"no controlled body"* `info!` as evidence a subject
  existed. It could not have printed: this fixture installs no `LogPlugin`, and
  the `world_log` line beside it prints unconditionally on BOTH branches. **The
  absence of a log is not evidence when the logger is not installed.**
  ⚠ THE GENERAL HAZARD, worth carrying: a settle helper whose stopping condition
  is WEAKER than "the thing you are about to drive is drivable" hands back an app
  in a gap frame, and everything driven on that frame silently does nothing.
  ⚠ AND THE 191 OTHER TESTS in that target still only run under `--lib`.

- ✔ **D-OILER-CONFIG — CLOSED 2026-08-31. The suite is green and no art was
  wrong.** Found 2026-08-31 while sweeping D129:
  `tools/ambition_sprite2d_renderer/tests/test_cli_batch_publish.py::
  test_default_adapter_configs_reference_registered_generators` fails on a CLEAN
  tree — `configs/review/oiler.yaml` publishes as `oiler`, which is a module
  target, but renders with the `toon` generator, so two renderers would write the
  same sheet and the last to run wins. The checker's own message names the fix
  (delete the config, publish by target name), and the decision it seemed to need
  — *"is the review config wanted"* — was answered by comparing the two: the
  MODULE already carries the config's `authoring_description`, its
  `gameplay_description` and all three barks, VERBATIM. It superseded the concept
  sheet on 2026-08-22 and the config outlived that by a month. Deleted; nothing
  was lost.
  ⭐ AND NO ART WAS WRONG, measured rather than assumed: regenerating `oiler`
  with `--force` gives a byte-identical sheet (`md5 84462285d041` before and
  after), so the module's render was already the shipped one on this checkout.
  The collision was a LATENT hazard — one that resolves differently depending on
  which target a full run publishes last — not an active corruption. ⚠ so
  `oiler_vfx`'s 1 clipped frame in the D129 population is unrelated to it.
  Renderer suite 693 passed, 0 failed (was 692/1).

- ✔ **GPT REVIEW #2 (2026-08-31, HEAD `861cd3d95`) — every finding addressed.**
  ⚠ ITS P1 #1 WAS ALREADY FIXED when the review was written: it snapshots one
  commit before `dfdc5112f`, which closed D-REPLAY-NOSUBJECT exactly as
  recommended. Nothing to do; noted so a later reader does not reopen it.
  ✔ **P1 — the C-stick lost its DIRECTION across the latch.** Mine, from the same
  day's device work. `attack_pressed` / `attack_strength_hint` /
  `attack_from_aim_stick` are EDGES and survive a sub-tick flick; the direction
  rode `aim_x`/`aim_y`, which are LEVELS — newest sample wins. A stick already
  back at rest by the next device frame delivered an armed C-stick attack aimed
  nowhere, and `attack_axis` then fell through to the MOVEMENT axis: flick right
  while running left, swing LEFT. `ControlFrame::attack_aim_x/y` is the same
  value latched WITH its edge, resolved through the body's aim frame. ⭐ NO WIRE
  COST — `ControlFrame` is not snapshot state, and the sim-visible `attack_axis`
  it feeds already was. Pinned at the latch AND on the real adapter→latch path
  (`a_flick_that_recenters_before_the_tick_still_attacks_where_it_pointed`), with
  the left stick held the OTHER WAY so the failure reads as a wrong direction
  rather than a missing one. Poisoned red.
  ✔ **P1 — a CPU mirror scenario rendered as a passive one.** Reproduced first:
  `scenario_key("george","jab","george",40,"passive")` and the `cpu` one both
  returned `george__jab__at40_000`. THREE layers used omission to mean two
  different things — the browser dropped `target` when it equalled the subject,
  the server gated `--target-behavior` on that target, and `moveset_render` reads
  an absent target as a MIRROR, not as "no opponent". All three now say the
  scenario literally; only a genuinely absent target is absent.
  ✔ **P1 — a chain take no longer gets a single-move render.** `moveset_render`
  has no `--chain`, so the panel REFUSES and names the missing capability instead
  of staging move B from neutral beside an A→B take.
  ✔ **P2 — `PortalAimHint` was written by every seat.** It is a SINGLETON, drawn
  for the one `PortalAffordanceBody` sourced from `ControlledSubject`, and the
  per-body gameplay loop wrote it N times: the last seat won, so seat zero's gun
  pointed where seat one aimed. Now written only for the presented subject
  (falling back to the single startup body). The gameplay above stays per-body —
  it was correct.
  ✔ **P2 — the report called exact geometry an AABB measurement.** The committed
  test asserted `aabb_overlap_ticks == 0` for a fixture whose BOXES overlap,
  which is a field denying its own name. Renamed to `target_overlap_ticks` /
  `first_target_overlap_tick` with a `target_overlap_source`
  (`runtime_exact` / `aabb_fallback` / `mixed`); `aabb_reach_bound_px` keeps its
  name because it really is a bounds measurement. Both facts and the provenance
  are asserted now.
  ✔ **P2 — `trace_replay` collapsed `Tilt` to `Auto`.** And `Auto` at full
  deflection resolves back to `Smash`, so the replay played a DIFFERENT MOVE than
  the trace recorded. The note excusing it said *"no device produces the hint"* —
  false since `RightStickMode::TiltAttack` shipped hours earlier. `AgentAction`
  carries the three-valued hint plus the C-stick source and direction; the trace
  model records them (serde-defaulted, so archived traces still read).
  ✔ **P3 — the GPU shutter now measures the WORLD, not only the clock.**
  `capture.rs` says *"frozen time is not a frozen WORLD"* and the guard checked
  `SimTick` alone. It now compares the multiset of drawn positions across the
  pumps. ⛔ TWO INSTRUMENT LESSONS: keying by `Entity` went red on IDENTITY, not
  appearance — four entities despawn and respawn at byte-identical positions
  every pump — and a poison in `Update` is INERT, because presentation re-syncs a
  seat's transform from `SimView` before propagation. Poisoned in `Last`, where
  it reddens.
  ▢ **P3 — portal interaction authority still lives in the view layer**, which
  this queue already tracks as D-PORTAL-INTERACT-SEAT. Unchanged: the adapter
  runs in `PlayerSimulation` and the interaction road spends the press in
  `FeatureInteraction`, so the claim is anticipated rather than read. Both sides
  use the same reach, and the review found no behavioural disagreement. Moving
  the authority is a schedule question, not a patch.
  Gates: app_it 533/533; core 536; input 139 (`--features input`); characters
  395; content 302; sim_harness 36; gameplay_trace 13; app_tools 8; python 30;
  takes discovery 19/19; doc links 271/864.

- ✔ **D-REPLAY-NOSUBJECT — CLOSED 2026-08-31 (rollback schema v146). A
  subjectless replay now OWNS the slot it admits on.** ⭐ GPT review 2026-08-31
  (baseline `9aec2f04`, HEAD `04dff7366`), and VERIFIED HERE against the source:
  `RoomReplayAdmitted`'s doc says *"the lifecycle operation owns the one
  pending-commit slot, and the room WILL be rebuilt"* and that `None` is *"a room
  rebuild with nobody in it"*. `admit_room_replay`'s `None` arm instead
  constructs `Admission::Admitted` LOCALLY — no `pending.record` — logs
  *"clearing the attempt, not rebuilding"*, and writes the message anyway. Every
  consequence that hangs off it (attempt-residue retirement, gravity reset,
  portal policy) then runs for an operation that never acquired ownership.
  ⛔ THE CAUSE IS VOCABULARY: `LifecycleIntent` has exactly ONE variant and
  `RoomTransitionIntent::subject` is a required `SimId`, so a bodyless replay
  cannot express an intent at all.
  ⭐ AND IT IS REACHABLE IN AN ORDINARY COMPOSITION, measured here the same day
  while closing D-SFX-RESET-RED: `ControlledSubject` is `None` for one frame
  after `settle_until_session_world` returns, so any headless or tooling
  composition pressing reset in that window takes this arm.
  ▢ The review proposes a `ReconstituteRoom { room, subject: Option<SimId> }`
  intent so every replay follows one sequence. ⚠ TREAT THAT AS A HYPOTHESIS —
  the reviewer could not run cargo — and ⭐ COSTED HERE 2026-08-31 before anyone
  starts it, because the shape is right and the price is not where it looks.
  The variant is the cheap part. `RoomTransitionApplication::apply` threads
  `subject: Entity` through ~8 sites — a component-presence check that returns
  `SubjectCannotTransit`, the gravity direction it reads off the body, the carry
  body, the motion model, the clusters, `BodyCombat`, and the presentation pair
  — so a rebuild with nobody in it needs every one of those to become optional,
  and the room half separated from the body half. Add the rollback cost:
  `PendingLifecycleCommit` is snapshot state, so a new variant is a schema bump
  plus `snapshot_impls`' encode/decode (it tags variants by byte — `3 =>
  Transition`).
  ⛔ AND READ `loading.rs`'s COMMENT FIRST: *"ONE VARIANT. The four in-place
  reset variants were deleted in v140: nothing recorded them, and a same-room
  replay is a transition to the room you are standing in."* This would
  re-introduce a variant that road deliberately collapsed — justified only
  because, unlike those four, something WOULD record it. ⚠ the cheaper-looking
  alternative (make `RoomTransitionIntent::subject` an `Option`) is worse: it
  makes a bodyless DOOR CROSSING representable, and *"a body without stable
  identity cannot produce a transition"* is a rule that is right for a crossing. Acceptance: a bodyless replay acquires a
  commit, retires attempt residue, REBUILDS the room, and admits with
  `subject: None`; poison the pending slot and none of it happens.
  ⭐ MEASURED AND PINNED 2026-08-31, so the fix has an acceptance test before it
  is written: `a_subjectless_replay_is_admitted_without_recording_an_intent`
  drives the real `admit_room_replay` with no `ControlledSubject` and asserts
  what happens TODAY — one `RoomReplayAdmitted` with `subject: None`, and
  `PendingLifecycleCommit::peek()` empty. Its second assertion inverts when this
  row lands, exactly as `a_blocked_strike_is_still_recorded_as_a_connection` did
  for D-CANCEL-ONBLOCK. ⇒ this is no longer a source reading.
  ✔ AND THE TYPE NO LONGER LIES. `RoomReplayAdmitted`'s doc described only the
  `Some` transaction; it now names both and says plainly that `None` means
  nothing was recorded and nothing will be rebuilt, so a consumer added before
  the fix knows which of the two it is handling. ⛔ it also records the trap:
  the `None` arm's `info!` needs a `LogPlugin` the reaching compositions do not
  install, while the `world_event` line beside it prints on BOTH arms — reading
  the absence of the first as "a subject existed" is a mistake already made once.
  ✔✔ **DONE.** `LifecycleIntent::ReconstituteRoom(RoomReconstitutionIntent {
  target_room })`, recorded by `admit_room_replay`'s bodyless arm through
  `pending.record` like any other operation — so a replay that LOSES the race to
  another lifecycle op is now refused outright instead of half-running.
  ⭐ **THE SHAPE IS NOT THE REVIEW'S HYPOTHESIS, and this row said why before I
  started.** The review proposed `subject: Option<SimId>`. The variant carries NO
  subject field at all, which makes a bodyless DOOR CROSSING unrepresentable
  rather than merely discouraged — the objection this row already raised against
  the `Option` shape, applied to the new variant too.
  ⛔ **AND THE COST WAS NOT WHERE THIS ROW COSTED IT.** The estimate was "~8
  sites in `RoomTransitionApplication::apply`". Those were real and all eight are
  now optional, but the load side was never counted: `ActiveRoomTransitionLoad`
  stores the intent for its staleness comparison, so it had to hold a
  `LifecycleIntent` and read it through accessors (`target_room`, `subject`,
  `arrival`, `edge_exit`, `zone_sfx`) — and there is a SECOND EXECUTOR, the eager
  host in `room_transition/commit.rs`, which the row's "one variant, one
  consumer" framing never mentioned. Both executors now keep TWO `None`s apart:
  an intent naming no subject is a bodyless rebuild, while `subject_entity`
  missing on a subject that WAS named is a void crossing that still cancels.
  Collapsing them would silently rebuild the destination room for a dead body's
  crossing.
  ⚠ codec tag **5, not 4** — tags 0-4 all belonged to the four variants deleted
  in v140, so 4 would decode an old `FullReset` as a reconstitution instead of
  refusing it. `scripts/rollback_codec_shape.py` caught the byte change on its
  own; the registration dump moved only its version line, which is correct for a
  variant added inside an already-registered type.
  ⛔ **ONE TEST WAS DELETED FOR BEING UNFALSIFIABLE.** A guard that a bodyless
  rebuild cannot commit under a CROSSING's authorized plan passed with the
  matching crossing intent too: that fixture has no content epoch and no
  construction plan, so `authorized_plan` waits before it ever compares intents.
  The discrimination is pinned one seam earlier instead, in `same_destination`
  (`repeated_zone_detection_is_one_destination`), whose fixture can tell the two
  shapes apart — poisoned both ways to prove it.
  Gates: app_it 533/533; monolith --lib 1204; runtime 50+5; rollback_ggrs 54;
  `--all-targets` clean on all six crates naming the changed types.

- ◐ **D-SCENARIO-IDENTITY — the REPORT half is closed; the transport, cache and
  render halves are not.** ⭐ GPT review
  2026-08-31, and this half was REPRODUCED by the reviewer running the committed
  Python: reports from takes at **40px and 80px requested spacing return
  `comparable = True`**. Four independent leaks: the HTTP server
  (`server.py:118-125`) accepts only `character/verb/frames/stride/target/spacing`
  and never forwards `--target-behavior`, which the renderer defaults to passive;
  chain state is recorded in the take and passed to nothing; the cache key
  truncates spacing (`__at{int(spacing)}`, so 40.1 and 40.9 collide) and never
  validates the stored manifest against the request; and
  `moveset_report.compare()` builds identity from subject/target/behavior/verb/
  label only.
  ✔ THE REPRODUCED HALF IS FIXED 2026-08-31: the report's `scenario` now carries
  `requested_spacing` and `chain`, so `compare()` — which decides comparability
  by whole-dict equality on that object — stops calling two different
  experiments the same one. ⚠ `requested_spacing`, NOT `spacing_at_press`: the
  REQUEST is what makes two recordings the same experiment, and what the bodies
  actually reached is a measurement, or a rig that settled a pixel differently
  would declare every pair incomparable. Guarded by
  `two_spacings_are_two_scenarios` — the 40/80 case, a chained-vs-single case,
  the premise that identical takes still compare, and the drifted-settle case.
  23/23.
  ✔ AND THE TRANSPORT/CACHE HALF, same day. The server accepts and forwards
  `--target-behavior` (it defaulted to PASSIVE, so a take against a live CPU was
  shown beside a passive render); the cache key is a testable `scenario_key`
  with spacing at three decimals instead of `int(spacing)`, and the behaviour
  rides with the target it describes so a solo render is not split in two; and a
  cached manifest is now validated against the REQUEST's scenario, not only its
  frame count and binary mtime. The browser sends the take's own
  `target_behavior` and keys its in-page cache on it. Guarded by
  `tools/ambition_moveset_inspector/tests/test_server_scenario.py` (3), red when
  the truncation is restored; `check_takes_discovery.mjs` still 19/19.
  ▢ WHAT REMAINS: CHAIN is still not renderable, so a chained A→B take beside a
  single-move render is the one surviving mismatch — and the honest answer there
  is *"this scenario is not renderable yet"* rather than rendering a different
  one, which needs a chain argument on `moveset_render` first. The wider
  `MoveScenario` value object is still the right end state; what is closed is
  every place the CURRENT fields leaked.

- ◐ **D-PORTAL-INTERACT-SEAT — the arbitration is per-body now; the LAYERING is
  what remains.** ⭐ GPT review 2026-08-31, source-read. `portal/
  input_adapter.rs` loops over driven bodies but asks
  `Option<Res<NearestInteractable>>` — one resource computed from ONE
  `ControlledSubject` in `ambition_sim_view` — whether an ordinary interaction
  claimed the button. So seat zero standing near a chest suppresses seat one's
  portal toggle, and seat zero standing clear lets seat one both toggle AND
  interact. ⚠ ALSO THE WRONG DIRECTION: a gameplay input decision consulting a
  presentation read-model. ⭐ this is the same class as the nine abilities
  converted to `DrivenBodies` on 2026-08-31 — the conversion has a layer left
  under it.
  ✔ CLOSED 2026-08-31. `NearestInteractable` carries a per-body map beside seat
  zero's answer, the producer classifies every DRIVEN body through one extracted
  `variant_in_reach`, and the adapter asks `for_body(subject)`. Guarded by
  `one_seats_surroundings_do_not_decide_another_seats_toggle` — all four
  combinations, including the premise that with nobody near anything BOTH seats
  toggle — red when the singleton read is restored.
  ⛔⛔ AND THE FIRST TWO DESIGNS WERE WRONG, both ruled out by measuring the
  SCHEDULE rather than the code. (a) *"Read the per-body interact BUFFER"*: the
  gameplay road spends the press with `consume_interact(subject)`, which is the
  claim itself — but it runs in `FeatureInteraction` and this adapter runs in
  `PlayerSimulation`, so the claim does not exist yet. (b) *"Publish the claim
  from `interact.rs`"*: same phase problem, plus that road only searches when a
  press is already buffered. ⇒ the adapter's answer is necessarily a PREDICTION,
  and the fix is to make the prediction per-body rather than to replace it. Both
  sides use the same `strict_intersects` reach, which is what keeps the
  anticipation honest.
  ⚠ THE LAYERING CONCERN THE REVIEW RAISED IS UNFIXED and is now the whole of
  what is left here: a gameplay input decision still reads an
  `ambition_sim_view` resource. Moving it needs the claim to exist in
  `PlayerSimulation`, which is a schedule question, not a rename.
  ✔ THE SMALLER SIBLING IS CLOSED 2026-08-31. `inventory_adapter`'s drop and
  pickup read `.next()`, so two seats acting on one tick were serialized across
  updates — the second landing in a world the first had changed. Both iterate
  every body-tagged intent now.
  ⛔⛔ AND SERVING EVERY INTENT EXPOSED A DUPLICATION `.next()` HAD BEEN HIDING:
  `commands.entity(..).despawn()` is DEFERRED, so a second intent served in the
  SAME run still saw the pickup in the query and **two bodies came away with a
  gun that exists once in the world.** A claimed set makes the winner definite
  within the run as well as across it, and the winner is message order — the
  producer's stable body order, not this system's entity iteration. Guarded by
  `two_seats_dropping_on_one_tick_both_drop` and
  `two_seats_grabbing_one_gun_produce_exactly_one_gun`, red under both poisons
  (serve one intent; drop the claimed set).

- ✔ **D-MOVE-INSTANCE — CLOSED 2026-08-31, and it was two defects.** ⭐ GPT review 2026-08-31, REPRODUCED: `moveset_report.py` discovers
  instances with `move and move != previous`, so a self-cancel that replaces
  `MovePlayback("jab")` with a fresh one in the SAME update reports
  `accepted: false`. The runtime supports exactly that
  (`moveset/tests.rs` pins the clock reset), and the report's own fixture forces
  `jab → None → jab`, so it never tests the real case.
  ⛔⛔ AND THE SAME DEFECT IS ONE LAYER DOWN, WHICH THE REVIEW DID NOT REACH —
  found here 2026-08-31 while costing the fix. `ticks_of(move, start)` scopes an
  instance by walking until the id CHANGES, so a gapless `jab → jab` gives ONE
  window spanning both, and the first instance's contact is credited to the
  second. Its own comment claims otherwise in as many words: *"NOT 'every tick
  with this id'. A jab chained into a jab shares an id, and scoping by id alone
  would credit the first instance's contact to the second."* It only avoids that
  when there is a gap. ⇒ `live_ticks`, `contact_ticks` and `reaches` all inherit
  it, so `first_contact_tick` and `aabb_bounds_reached_target` for B are wrong in
  the same case, not just `accepted`.
  ✔ ONE FIX SERVED BOTH. `MovePlayback::instance` is set from the playback it
  REPLACES — no counter component, because the site that inserts the new playback
  is the only one that can see both uses — `CombatMoveView` exposes it, the
  recorder records it, and the report both DISCOVERS on `(id, instance)` and
  SCOPES `ticks_of` by it. Schema 144→145; the instance is checksummed because
  two peers agreeing on the id and the clock can still disagree about which use
  this is. A take from before the field falls back to the id, with the old hole
  and a test that says so.
  ⛔⛔ AND THREE VERSIONS OF THE SCOPING ASSERTION PASSED WITHOUT THE FIX, which
  is recorded in the test because it is the reusable part: `ticks_of` walks
  FORWARD, so the second instance is already protected by its own start tick, and
  a first use that ALSO has a contact hides the merge behind "the earlier one
  wins". The arm only bites when the first use WHIFFS — a jab that missed,
  cancelled into a jab that hit.
  ⛔ ruled out in writing: do NOT derive the instance from `elapsed_s` going
  backwards. A looping move's clock does that every lap and would split one
  instance in half instead of joining two.

- ✔ **D-REPORT-GEOMETRY — CLOSED 2026-08-31. The engine publishes the exact
  answer and the report prefers it.** ⭐ GPT review 2026-08-31, REPRODUCED: two boxes whose
  edges meet exactly report `overlap = True` in `moveset_report.py`, while
  `CombatVolume`'s runtime `strict_intersects` returns false. The raw
  observation layer is correct — it preserves exact circles/OBBs/convex — and
  `moveset_report.py:58-72` flattens every volume to bounds before computing
  `overlap_ticks`, `max_reach_px` and `geometry_reached_target`, whose names
  claimed more than a broad phase supports.
  ✔ TWO HALVES CLOSED 2026-08-31, both cheap and both real. (1) `_overlaps` is
  STRICT now, matching `CombatVolume::intersects`'s own documented contract —
  *"edge-touching is NOT an overlap"* — so the report and the game agree about
  tangency. ⛔ the old `<=` was wrong in ONE DIRECTION: it claimed contact the
  engine denied, under a field called `overlap_ticks`, which reads as *"the
  strike was on the target and the engine ignored it"*. (2) The derived fields
  are `aabb_overlap_ticks`, `first_aabb_overlap_tick`, `aabb_reach_bound_px` and
  `aabb_bounds_reached_target`; schema `v2`. `_bounds` already carried the rule —
  *"every field derived from this says `_aabb`"* — and not one of them did.
  Guarded by `touching_boxes_are_not_an_overlap_because_the_runtime_says_so`,
  which pins the tangent case, its premise, and the absence of the old names.
  22/22.
  ✔ AND THE EXPENSIVE HALF, same day, the way the review asked: NOT by porting
  Parry. Each strike row in the observation carries `overlaps` — whose hurtboxes
  that volume is inside RIGHT NOW, computed in Rust with
  `CombatVolume::intersects`, the call gameplay resolves hits with. A circle, an
  OBB or a convex shape answers exactly; two boxes whose edges touch answer NO.
  The report prefers it and falls back to bounds only for a take recorded before
  the field existed.
  ⚠ AND ONLY WHEN THE TARGET HAS AN ID TO MATCH. `overlaps` names victims by
  `SimId`, which is `None` for a body without one — so with a nameless target a
  `None` entry would match ANY nameless body, and the report falls back to
  geometry rather than answering about somebody else.
  ⭐ `overlaps` IS NOT `hit`, and publishing both is the point: this is where
  things ARE, `hit` is what the runtime RESOLVED, and they differ whenever the
  victim was intangible, shielded, already struck by that volume, or on the same
  team. Guarded by
  `the_engines_exact_overlap_outranks_the_bounds_approximation`, whose fixture is
  the DISAGREEING case — bounds that overlap, an engine answer of "no" — plus
  both premises. 26/26.

- ⊙ **GPT review 2026-08-31, TWO ITEMS ANSWERED BY COMMITS IT COULD NOT SEE.**
  Its HEAD was `04dff7366`; the following landed after it. (1) Its P3 *"explicit
  `Tilt` exists through the wire vocabulary but no real device adapter emits
  it"* — the device half shipped in `66bccd961`:
  `ControlSettings::right_stick_mode`, a hysteretic flick, and a Controls row.
  ⚠ its sibling observation stands and is recorded there: the harness `Action`
  and `trace_replay` are still strong-or-not, so a recorded tilt collapses to
  `Auto`. (2) Its P3 GPU-shutter gap is unchanged and correctly graded — it says
  itself it reproduced no mismatched PNG.
  ⚠ AND THE RUST FINDINGS ABOVE ARE SOURCE-READ: the reviewer had no `cargo`.
  Its Python/JS findings ARE reproduced and are the stronger evidence.

- ✔ **D-CAUSAL-CAPABILITY — CLOSED 2026-08-31, and the report was giving one
  of the two audiences the wrong advice.** ⭐ GPT
  review 2026-08-31, source-read. `ambition_app_tools` defaults the `causal`
  feature OFF and `moveset_takes` always writes `"causal": []`, so "the recorder
  was not built" is indistinguishable from "the recorder ran and saw nothing" —
  and `moveset_report.py` read the empty array as the former.
  ⭐ THE HARM IS THE ADVICE, which is what makes this worth fixing rather than
  documenting: a reader whose build ALREADY has the feature was told to
  *"re-record with `--features causal`"* and would get the same empty array
  again. The two absences want opposite next actions — enable the feature, or go
  find why the JOIN missed (seat vs `SimId`, or the tick).
  ✔ The take now writes `capabilities: { causal_resolution: bool }` beside the
  array, from a `const AVAILABLE` in each of the two `causal_trace` modules, so
  the flag cannot drift from the build that produced it. The report carries it
  into the doc and prints THREE messages: unavailable, present-but-matched-
  nothing, and *"this take predates the field and cannot say"* — a legacy take is
  not guessed on. Guarded by
  `an_empty_causal_array_says_which_kind_of_nothing_it_is`, all three arms, red
  when the capability read is pinned to one answer. 21/21 report tests.
  ⛔ AND THE CODE'S OWN DOC WAS WRONG TOO: the no-feature module claimed the take
  *"carries no `causal` array at all"*. It always carried `[]`.

- ▢ **CAPABILITY FOOTPRINT: 47 crates linked, 20 a movement-only game never
  asked for — and the count CANNOT fall by a manifest edit.** (⚠ this number has drifted FIVE times; `python3 scripts/check_absence_contracts.py | grep footprint` prints the live pair. 45/18 as of `479f9d3e4`, when `ambition_registry_core`
  entered the closure; 46/19 as of `bbfa38a3d`, when `ambition_held_items` did; 47/20 as of `83460e3f3`, D33 cut 1, when `ambition_body_seed` did. ⭐ EVERY ONE OF THOSE RISES IS A CARVE PAYING ITS DEBT — do not read the series as regression.) (Scheduled
  2026-09-02 from ambition-da's docs pass; re-worded the same night after
  ambition-da re-derived it, `2068bcd31`.) The instrument is installed:
  `capability-footprint-may-not-grow` in `scripts/check_absence_contracts.py`
  ratchets the closure (`scripts/baselines/capability-footprint-baseline.json`).
  ⛔ Slice H (2026-07-30) already took every facade cut (41 → 38); all 16 that
  remain ALSO arrive through `ambition_platformer2d_actor_monolith`'s own
  manifest, so gating a facade edge cuts nothing — a game that needs actors
  needs the monolith and the monolith brings them. The baseline's own
  `facade_only_four_are_not_a_quick_win` note counts 170 in-repo `ambition::`
  call sites (`render` alone 90) behind the four facade-only edges. ⇒ Honest
  acceptance: the count falls only through a D33 CARVE (a domain leaves the
  monolith AND its edge leaves the monolith's manifest) or a facade-optionality
  migration over those 170 call sites; neither is mechanical. ⚠ A carve that
  adds a crate RAISES the count (`ambition_world_items`, 43 → 44: crates, not
  bytes) — the two lines of work must not be scored against each other, which
  `engine/capability-and-runtime-composition.md` now says. ✔ The baseline's sub-lists are repaired
  (2026-09-03). The owed item named only the ENTERING half (`ambition_damage`,
  `ambition_mount`, done at `f1445c142`); measuring found the other direction
  was worse — FIVE crates that had left the closure were still listed as
  reachable, so `reachable_only_through_the_facade` fell from four to one and
  the 170 call sites above are really 90, `ambition_render` alone. ⭐ The two
  lists are now equal: all 19 arrive through the monolith alone, so no facade
  cut removes any of them, which makes the honest acceptance above stronger
  rather than weaker. Guarded by
  `scripts/tests/test_capability_footprint_baseline_is_coherent.py` — the
  ratchet only ever looked for crates ENTERING.
- ✔ **`string_id!` was defined THREE times; it is written once now.** Fixed by
  `02a796d2c`, exactly as this row specified: `#[macro_export]` on
  `ambition_load`'s copy (`crates/ambition_load/src/id.rs:19`), the other two
  deleted, no new crate or edge. Re-verified 2026-09-03 —
  `git grep "macro_rules! string_id"` returns ONE hit, and the three consumers
  (`ambition_load`, `ambition_game_shell`, `ambition_load_presentation`) all read
  it. ⛔ The `::core::fmt` requirement this row flagged is not only met but
  written down where it can survive: the macro's doc comment says *"an exported
  macro expands at the CALL SITE, where a `use std::fmt;` may not exist — relying
  on one is the difference between a macro that moves and a macro that only
  appears to"*, which is the one sentence a future edit would otherwise
  rediscover. Design record: `triage/stable-identifier-centralization.md`.
- ▢ **`ambition_registry_core`: R2 + R3 LANDED 2026-09-03 (crate + two pilots:
  construction, rollback; rollback baseline byte-identical); next is R4 — decide
  which of `PlacementLoweringRegistry` / `RoomContentStagingRegistry` migrate
  and which of the seven silent-overwrite registries must say "replace" in
  place. Design and evaluation in `triage/ambition-registry-core.md`.**
  ✔ **R4's FIRST HALF IS ANSWERED IN CODE — re-read 2026-09-03: BOTH migrated.**
  `PlacementLoweringRegistry` (`platformer2d_world/src/placements.rs:198`) takes
  `RegistrationMeta` and `classify`. `RoomContentStagingRegistry`
  (`actor_monolith/src/features/ecs/spawn/content_staging.rs:57`) takes `RegistrationMeta` and
  `require_non_empty` and **deliberately does not take `classify`**, saying why
  in place — *"NO `PartialEq`, AND THEREFORE NO `ambition_registry_core::classify`"*
  — which is exactly the opt-out the crate's own docs prescribe for a registry
  whose policy differs. ⇒ Four consumers now, not two. ⚠ The SECOND half of R4 is
  untouched: the seven silent-overwrite registries still have to say "replace" in
  place, and that is what remains of this row. The
  inventory row that preceded it, kept for the record:
  **INVENTORY THE 31 REGISTRIES BEFORE DESIGNING `ambition_registry_core`.**
  (Scheduled 2026-09-02.) 27 became 31 in six weeks; the only registry-shaped
  trait is `RollbackRegistrar`, one domain's hook. The doc's argument is semantic
  drift, so the first deliverable is a table of how all 31 answer the four
  protocol questions (identity; do fn addresses join equality; what enters a
  fingerprint; does a conflict leave the old registry unchanged), each cell a
  citation the planning citation checker resolves. ⛔ No abstraction until that
  table exists. Design: `triage/ambition-registry-core.md`. Assigned to ambition-da.
- ✔ **THE GENERATED-DIALOGUE TRIAGE IS RE-SCOPED (2026-09-02).** 149 catalog
  rows, 124 with a `hall_dialogue_id`, 131 authored `hall_*` Yarn nodes: the
  Hall was solved by hand-authoring, the escape its 2026-07-26 decision left
  open, and `triage/character-dialogue-from-suggestions.md` now says so at the
  top. What is left (~25 rows with no hall id, every room that is not the Hall)
  is a content call — generate for the remainder or keep authoring — recorded
  in `awaiting-maintainer-decision.md`; no engine work until it is made.
- ✔ **Two rows corrected, no work owed (2026-09-02, ambition-da):**
  `engine/participant-action-system.md` §P1 is DONE (pinned by
  `dialogue_claims_the_talker_while_a_pause_still_stops_everybody`);
  `engine/kinematic-world-objects.md` K2/K4 status lines now agree with the
  header that closed K2–K6.

## External measurements / human-gated work

These are live but should not cause an autonomous agent to invent data or a
product ruling.

- ⚠ **THE WEB REVEAL DOES NOT WAIT FOR THE GPU — fix written, ON ITS OWN BRANCH
  `web-gpu-wait` (`2d623308f`), DELIBERATELY NOT MERGED.** Found by review
  2026-09-02.

  **What is wrong on the web TODAY.** The native barrier holds a decoded image
  as pending until the render world is seen to prepare it. On wasm the whole of
  `ImageStagePlugin::build` was `#[cfg(not(target_arch = "wasm32"))]`, so
  `RenderWorldPresent` was never inserted, `is_gpu_prepared` was a stub
  returning `false`, `is_awaiting_gpu` was therefore always `false`, and
  `inspect_room_asset_manifest` read that as SETTLED. **The browser lifts its
  cover when pixels reach `Assets<Image>`, not when the GPU has them** — exactly
  the post-reveal upload the barrier exists to move under the cover.

  **What the branch changes.** One conflation: `Instant` is native-only, so
  gating the TIMING gated the READINESS with it. `ImageStages` gains
  `gpu_prepared: bool` (the fact, every target) beside the native-only
  `gpu_prepared_at` (telemetry); `is_gpu_prepared` has one definition again;
  `gpu_prepared()` takes an optional instant. The GPU stamper and
  `RenderWorldPresent` install on every target. The clock, the `[image-gpu]`
  line and the FIRST-DRAW stamp stay native — correctly, for first draw: it is
  pure telemetry no readiness decision reads.

  ⛔ **THE ONE BROWSER CHECK JON RUNS TO ACCEPT IT** — enter the hall on the web
  persona and watch the cover: it must visibly HOLD through the upload and then
  LIFT. **If it never lifts, revert the branch.** That is the failure mode this
  is held back for, and it is worse for a web player than today's early lift,
  which is why it is not going in unverified.

  ⛔⛔ **AND IT NAMES TWO MORE CONDITIONALLY-BLIND CHECKS** (the family in
  `reviewer-guide` terms: correct, unrun, reported as though run):
  - the **wasm CHECK is TYPE-ONLY**. Every branch here type-checks; a compile
    check can never see a `#[cfg]` that removes BEHAVIOUR rather than code that
    fails to build.
  - the **"web persona BOOTS" job runs the web composition NATIVELY**
    (`--features visible_web_base`, native target), so it compiles the
    `not(wasm32)` branch. It proves the web persona composes; it cannot execute
    the web branch of anything.
    ⛔ AND IT IS BLIND A SECOND WAY: its `if not only and everything:` gate puts
    it in the EXHAUSTIVE plan only, so a default run never executes it at all
    and an exhaustive run executes the wrong branch. One job, two independent
    blindnesses — each question was asked of it separately and each got a
    locally reassuring answer.
  ⇒ Between them the web path had ZERO behavioural coverage, which is how this
  survived. The family now has a page of its own —
  [`../recipes/checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md),
  seven members, the four questions that find them, and ⛔ the rule that the
  catalogue itself rots (member #3 was fixed by `234bcc686` within an hour of
  being written down). Native-side it is guarded and poison-verified — passing `None` for
  the timestamp is what a clockless target does, so the web case is reachable
  from a native test — but no test on this machine executes wasm.

- **Switch Pro outer range:** run `Shift+F6` on both machines, push the controller
  to each extreme/corner and compare peak axis magnitude. Only then decide
  whether shared outer saturation is needed.
- **Character/product decisions:** see
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), including
  the proof-pulse lifetime, character heights, fighter reach/tumble policy,
  ranged-recharge presentation, persistent foreign-room actor placement and
  dormant windbox/armor customers.
- ✔ **The first honest measurement of the shipped program in the hall** — taken
  2026-09-02 (`desktop-timeline-run-20260902T015909Z`): 250-310 fps in every
  room, V-Sync Off, no Tracy. No frame-rate campaign remains; see the top of
  [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).
  The user-visible problem is the hall-entry hitch, which is the asset campaign.
- **A flash in the kaleidoscope menu since Bevy 0.19** (Jon, 2026-09-02, not
  yet observed by an agent). ⛔ STILL THERE after `2e7819419` (Jon, 2026-09-02
  evening, same profile session as the hall run) — whatever that commit fixed
  was not it. DEFERRED by Jon to a session where he can iterate interactively
  with an agent; no more blind fixes. Facts needed before anyone searches: one frame at
  launch or on EVERY menu open; the colour (black = an uncleared/uninitialised
  target, magenta = a Bevy diagnostic texture, the world = a missed overlay).
  Shape suspects, in order: the cube's `Camera3d` activates with
  `ClearColorConfig::None` over the 2D camera and relies on
  `msaa_writeback` for its first frame; the direct-entry window is created
  `visible: false` and shown after startup; the ceiling app logs
  `Couldn't get swap chain texture after configuring. Cause: 'Outdated'` on
  its first frames under 0.19. Read the census `camera` rows: three active
  cameras on the window (main order 0, "Cube scrim display" order 7 with no
  layers, "Cube pause" order 8) — a full-screen camera with nothing to draw
  is a clear per frame if its clear is not `None`. ⭐ PHOTOGRAPHED HEADLESS
  2026-09-02 (`capture_scene --route ambition_gameplay --press Escape,wait:10
  --press-during 1 --frames 10 --include-ui`, llvmpipe): frame 0 the world,
  frame 1 the world dimmed under the scrim, frame 2 the cube with its faces
  drawn — **no black and no magenta frame** in the open sequence, and the
  direct-sandbox host (`<ROOM_ID>` form) does not open the menu at all (the
  world pauses, nothing is drawn — the kaleidoscope is shell-route
  presentation). So under a software renderer the open is clean; what is left
  is host-side (swapchain `Outdated` on the first frames, MSAA writeback on a
  real adapter) or launch-only — which is exactly the fact only you can give.
- **The reveal-barrier fix: HALF-CONFIRMED on the host 2026-09-02, and the
  other half is void.** `desktop-timeline-run-20260902T215256Z` (Ultra, 3090,
  hub → hall): placeholder rectangles 0 / 0 / 0, `wait_ms 292` covered, **0
  frames over 33 ms after the transition** — the reveal no longer hitches.
  ⛔ But that run drew the hall from `sprites_0_25x/` because of the room tier
  cap `dc3cd0d91`, which Jon rejected on sight ("no lower quality tier for
  gallery previews") and which is removed; so the 292 ms and the "expect ~40
  MP" tell below describe a program that no longer exists. The next walk
  re-measures at the user's tier: the cover will hold LONGER (434 MP at Full
  decodes under it) and the tells are the same ones — zero "nothing demanded
  it" warnings, no >33 ms frame after the cover lifts, and every `[image]`
  line in the hall window reading `sprites/` (Full), none `sprites_0_25x/`.
  ⭐ **AND THE PREDICTION, WRITTEN DOWN BEFORE THE CAPTURE so it cannot be
  rationalised after it: if the reveal BARRIER is what fixed the hitch, the
  >33.4 ms count after the cover lifts stays 0 at Full; if the room tier CAP was
  doing the work, it comes back.** That is a real risk rather than a formality —
  the cap was cutting the decode load the barrier holds the cover for, so the
  two fixes were tested together and only one of them survives. ⚠ Until that
  walk exists, the two COUNT tells (placeholders, cover held) are confirmed and
  tier-independent; the TIMING tell is not confirmed for the shipped program.
  Original tells, still the checklist: zero "nothing demanded it" warnings at
  the hall reveal, `asset_wait_ms` in the seconds (the cover visibly holding),
  no >33 ms frames after the cover lifts. In the same run: `image_arrivals`
  megapixels in the hall window (434 at Full) and `resident_mb` (2153 before
  render-world-only). Also worth a look: the kaleidoscope
  System page under a scroll — no flash (`2e7819419`). NEW since `2b31329c6`:
  the `[image-gpu]` lines say how long the UPLOAD half took — on the VM's
  llvmpipe every sheet of the reveal was prepared in ONE render frame
  (`insert→gpu 493ms` for all seventeen). On the 3090 read the `insert→gpu`
  max in the hall window; if it is a frame, a second walk with
  `AMBITION_RENDER_ASSET_MB_PER_FRAME=64` shows whether pacing trades it for
  `awaiting gpu` frames of late-arriving art. AND since the readiness term
  landed, `asset_wait_ms` INCLUDES the upload and the reveal should show no
  `[image-gpu]` line after it — if one appears after the cover lifts, name the
  file; it came by a road the manifest does not list. ⭐ And the DECODE half
  (2026-09-03): at Full the hold is predicted ~3.5 s decode-bound on four IO
  threads; a second walk with `AMBITION_IO_THREADS=8` (desktop host knob,
  unset = Bevy's default of 4) beside the plain one says whether the cover's
  `wait_ms` is the IO pool's — if it halves, the lever is threads; if it does
  not, the lever is the sheet format.
  **Tells added 2026-09-02 evening, same walk:** (a) GAME START — the load
  screen shows a "Load the first room's art" row, and the `[image] … f NNN …
  sprites/player_robot_v3_spritesheet.png` line carries a FRAME STAMP ≤ the
  first `room-loaded`'s (`[image]` lines print `f` since `fdd2019c1`;
  compare frames, not seconds — the census prints in `Last`, after the
  activation frame's work, so its clock reads later than a `room-loaded` the
  insertion actually preceded, and headless the two land in the SAME frame);
  and no 67-79 ms frame ~0.1 s after `room-loaded` (`aca57e636`; the pre-fix
  captures `015511Z`/`015909Z`/`020529Z` all have it). The player must never
  draw as a rectangle at start. ⭐ The bundle summary prints this verdict
  itself since `b41044206` ("✔/⚠ FIRST ROOM: …", by frame); a `⚠` naming only
  `ai_slop` is the basement prefetch, by design. (b) The census row reads
  `images_render_world_only=true` and `resident_mb` is roughly HALF the
  pre-`68d38076e` figure for the same room and tier (the CPU copy is gone);
  the history label carries `+render-world-only`. (c) HALL EXIT — no
  `[image]` line with `live=1` for a hall-only character after `room-loaded
  central_hub_complex`, and the `[image-census]` after it lists
  `character-sheet` at roughly the hub's cast, not the gallery's
  (`124684f56`, 0 orphan pages headless). (d) `[image-census]` prints on
  a 5 s window boundary AND once at `AppExit` (`d10fbf0e3`), so even a run that
  ends inside one window closes with its resident-by-road line. ⭐ And the
  bundle summary states the hall-reveal verdict itself since `01ca0b006`
  (placeholder rectangles by diagnosis, `asset_wait_ms` per transition,
  spikes after the cover) — read the summary first, the raw log second.
  ⭐⭐ **AND THE HOST NOW SAYS THIS ROW IS THE LAST USER-VISIBLE HITCH IN THE RUN
  (2026-09-02, `desktop-timeline-run-20260902T215256Z`, 3090, windowed).** That
  capture has exactly THREE frames over 33.4 ms and all three are BEFORE the
  transition; `asset_activity.csv` puts two of them inside the startup decode
  burst, and the hall's larger burst costs none:

  ```text
    wall_s  images   MP    MB     spikes in that window
     2.262      0   0.0     0
     2.989     98  20.7  82.8     125.3 ms @2.386, 203.3 ms @2.589
     8.002    250  71.2 284.7     none — the hub → hall entry, cover held 292 ms
  ```

  ⇒ **The hall decodes MORE (128 images / 116 MB) than startup (98 / 83) and
  costs ZERO spikes, because a cover holds for it.** ⚠ Conservative, too: the
  hall leg ran under the room tier cap while startup art did not, so the real
  Full-tier gap is wider. ⇒ This is not a polish row — the mechanism that fixed
  the hall exists and the first room does not use it, and the host has now put a
  203 ms frame on what that costs.
  ⛔⛔ **AND THAT READING WAS WRONG — CORRECTED THE SAME NIGHT, by df's question
  rather than by my measurement.** *"A frame spike under a cover is cover time,
  not a hitch"* is this campaign's own rule, applied to the hall six paragraphs
  up, and I did not apply it to my own finding. The run's route lines settle it:

  ```text
  [2.246s] [game-mode]  0.930s f    0  initial playing
           spikes at wall 2.386 (125.3 ms) and 2.589 (203.3 ms)
  [5.179s] [world-event] 3.863s f 1131  room-loaded central_hub_complex
  ```

  ⇒ **Both spikes land between `initial playing` and `room-loaded`** — during the
  first room's load, which is exactly when the load screen with its "Load the
  first room's art" row is up. ⚠ `[game-mode] initial playing` at FRAME 0 is not
  "the player is in the world"; it is the same trap that made an offscreen
  capture report eighteen pops earlier the same day.
  ⇒ **So the honest statement is: three spikes, all before the first room
  finished loading, and the bundle cannot say whether a curtain covered them.**
  The claim that this is "the last user-visible hitch" is withdrawn — it needs
  the route's presentation state at 2.4-2.6 s, which nobody has yet read.
  ⭐ What survives untouched is the COMPARISON, because both halves are measured
  the same way: the hall decodes more and spikes zero WITH a cover, and the
  startup burst spikes twice at whatever cover it has. That still says the cover
  mechanism works; it no longer says a player sees the startup one.
- **FIVE LDtk preview tilesets decode the FULL player sheet on every boot**
  (7.6 MP, `../sprites/player_robot_v3_spritesheet.png`, declared as
  `sprite_player_robot_v3` for editor entity previews; `bevy_ecs_ldtk` loads
  every project tileset). Never drawn by the runtime. ⚠ **Recounted in the
  submodule 2026-09-02: five, not four** — `ambition_content`'s
  `hall_of_characters`, `intro`, `sandbox`, `you_have_to_cut_the_rope` AND
  `ambition_demo_sanic/worlds/sanic_speedway.ldtk`; the earlier count read the
  worlds `ambition_content` exposes rather than the worlds carrying the
  declaration. `mary_o` does not reference the sheet. All five still point at
  full resolution. The fix is retargeting each declaration at
  `../sprites_0_25x/player_robot_v3_spritesheet.png` (0.5 MP, preview survives)
  — in the map submodule, so it waits for Jon's yes. Measured via the image
  stage ledger (`demand=unknown`, asset index 4). ⚠ It is also the player's
  own sheet decoded TWICE per session — `game://sprites/…` by the LDtk spine
  at boot, `sprites/…` by the realization after the first `room-loaded` (host
  captures `015511Z`/`015909Z`) — a pair the re-decode census cannot see
  because two sources are two asset ids.
  ⭐ **SIZED ON THE NO-GPU VM, 2026-09-02 (`4a52f1903`): it is 32% of the hall's
  resident megapixels at Potato and 76× the copy actually on screen** (7.6 MP
  undrawn against 0.1 MP drawn). ⛔ AND IT IS BYTE-IDENTICAL AT POTATO, HIGH AND
  ULTRA — a tileset declaration carries its own `relPath`, so no quality tier can
  reach it. That makes it WORSE the lower the setting: it is a fixed 7.6 MP on
  top of a total that shrinks from 29.9 MP at Ultra to 24.1 MP at Potato, which
  is most of why the hall's never-drawn headroom rises from 8.8× to 26.8× as the
  tier drops. A residency ratio taken at a low tier is measuring this row.
- **Why the capture runs on for minutes after the window closes:** reproduces
  nowhere headless (0.4 s drain for 4.2M zones on the VM). One capture with
  `TRACY_NO_SYS_TRACE=1 scripts/profile_desktop.sh` decides whether it is Tracy's
  Linux sampling/context-switch capture on the host; if the exit is instant,
  make it the script's default.
- **One asset-campaign knob to try in the hall on the 3090** (recorded on the
  census row and in the ledger label): `AMBITION_RENDER_ASSET_MB_PER_FRAME=64`.
  Render-world-only sheets became the default on 2026-09-02 (captures
  byte-identical, peak RSS −141 MB in the hall at Quarter on llvmpipe);
  `AMBITION_IMAGES_RENDER_WORLD_ONLY=0` is the A/B. Read the hall-entry spike
  list, `resident_mb`, and whether any sprite draws blank. See
  [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).
- **Whether to author a DENSE melee room** — a product call, not an engine one.
  Bounded attention (ADR 0034 increment 2) cannot be validated without it:
  `Perception::Sighted`'s viewport already caps kept peers at ~14 (max 21) and
  holds there when the room doubles, so the written acceptance criterion passes
  on current code. A sparse room of 200 would satisfy the old wording and measure
  nothing. See
  [`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md).
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
