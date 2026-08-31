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
  `options.rs:752` citation is dead — the file moved crate to
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
  improvement without a measurement. ⛔⛔ RE-MEASURED 2026-08-31: the owner doc's
  "only four dependencies are single-path" list is STALE — `ambition_dev_tools`
  and `ambition_mount` have 6 dependents, `ambition_items` 5, `ambition_damage`
  3, and the FACADE every game depends on names all four directly. So removing
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
  paragraph forbids. ✔ SO THE DEV_TOOLS SLICE IS DONE: the kernel holds no
  developer AUTHORITY, and a plugin asking a profiler to time its own startup is
  a legitimate use of a developer tool.
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
  ▢ NO NEXT CANDIDATE NAMED — re-census before migrating another field. ⛔ NOT a residual: eleven grid fighters on the
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
  0. ⇒ the remaining 35 targets are a POPULATION, not a defect list; the open
  work is an instrument that separates "drawn flush to a fitted frame" from
  "severed", not 35 canvas edits.

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
