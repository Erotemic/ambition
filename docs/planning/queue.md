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

✔ **D-DROP-IDENTITY — the same gauntlet was an occurrence or not, depending on
how you got it.** `drop_held_weapon` spawned a `GroundItem` with provenance and
no `SimId`, and the three durable roads that could give a carried object back —
`capture_minted_item_baseline`, `capture_custody_baseline`,
`TransactionBaseline::capture` — are all keyed by one. So a checkpoint taken
while the player held a boss's signature gauntlet had no description of it and
the death sweep destroyed it, while the identical gauntlet authored as a room
placement carried an identity all along. Fixed by `4a283375b`:
`SimId::death_drop(parent, kind)`, derived from `(parent, kind)` exactly as the
drop's provenance is. Guarded by
`only_the_death_drop_that_becomes_an_object_carries_an_identity`, which pins
both halves — the three drops that grant a QUANTITY stay anonymous, because
`OwnedItems` is their durable record. ⛔ The S4 identity census was green because
its `populate()` spawns no `GroundItem` at all, not because the invariant held:
a census with no waiver list is only as strong as the population it walks.

✔ **D-ROUTE-CONDITION — a route could not ask a question the engine had already
published.** `prepare_question` hardcoded the condition id to `world.flag_set`
and passed the authored `gated_by` as its ARGUMENT, so `inventory.holds` and
`held.is_held` were published and unreachable from a route. Fixed by
`2054291e4`: `gated_by` is an authored condition LINE, parsed by the
`prepare_line` the catalog already had — a bare value still means
`world.flag_set`, so the two shipped rows needed no migration. `body.can(verb)`
lands with it, reading the EFFECTIVE `BodyAbilities` rather than the authored
base. ⛔ The discriminator is SYNTACTIC and must stay so: a misspelt condition
gets the catalog's diagnostic and a standing wall, never a silent demotion to a
flag lookup that would also never be satisfied.

✔ **D-REVIEW-0904 — four review findings, and each guard now fails for the
reason it exists.** (1) The music gitlink pointed at a `cli.py` that did not
parse while its four publisher tests passed, because they import the helper and
never start the program — `fe1b0105c` guards the ENTRY POINT. (2) GPU readiness
recorded "ever prepared" and ignored `Modified`, so a reveal could lift over a
replaced GPU copy — `d462520a0` makes the proof mean the CURRENT contents.
(3) The parallax regression asserted a memo rather than the room, and the no-art
branch its own comment described was unreachable — `a80ba6e31` counts real
`RoomVisual`s and publishes the loader's outcome. (4) The camera calibration
test owned a copy of the height it claimed to guard — `cc8ee48f2` moves the
cross-domain guard to a crate that can read both authorities. ⛔⛔ Verifying (2)
found the wider trap: `cargo test -p ambition_asset_manager --lib` runs NONE of
`image_stages`' tests, because the module is behind the crate's `bevy` feature
that no default build enables. The poison run reported 56 green and proved
nothing; `--features bevy` is required.

✔ **S4-WHEN — a rollback anchor was ANONYMOUS and no fixture width could have
found it.** Landed 2026-09-04, `974e8e97e` / `f715aa544`. S4's rule was *"a
census with no waiver list is only as strong as the population it walks, so
widening `populate()` is the way to strengthen it"*. ⛔ That rule has a second
axis it did not name — **WHEN the census looks** — and the identity census walked
the world only after sixty frames of play, so its real population was *"whatever
survives sixty frames"*. A `PortalShot` is a rollback anchor
(`require_rollback::<PortalShot>`) that carried no `SimId`: it rewound by entity
index while being the entity that decides where a portal opens, and every shot
has fizzled or placed long before frame 60.
  ⭐⭐ **THE PROOF IS A POISON THAT PASSES.** With the shot anonymous AND the
census restored to looking only at the end — the code exactly as it shipped that
morning — the run is **2 passed, 0 failed, GREEN**. A live defect sailing through
an assertion with no waiver list. ⇒ **An anti-vacuity floor is only as honest as
the moment it is asked at**, and incapability is a stronger claim than
insufficiency: "widen it more" has no answer to "no width would have helped".
  ⇒ Fixed on the road the repo already had — `PortalFireIntent` carries the
shot's `SimId`, minted by the FIRER from its own `SimIdCounter`, the same
`Some(mint())` shape `deploy_sentry` / `open_vortex_well` / `drop_hazard` take.
It costs the message its `Copy`.
  ⭐ **AND A THIRD CENSUS, INSIDE THE SIMULATED FRAME**, because both existing
ones walk the world BETWEEN steps and cannot see an anchor that never reaches a
step boundary. ⛔ Its reach is measured rather than claimed, and the FIRST POISON
FAILED, which is the finding: an entity spawned and despawned through `Commands`
with no `ApplyDeferred` is seen by NOTHING, so the honest scope is *"every anchor
that exists at a system boundary inside a step"*. The wording it first shipped
with over-claimed. ⇒ **A guard's stated REACH is a claim like any other and wants
a poison of its own.**
  ⭐⭐ **AND THE SHARPEST RESULT IS A NEGATIVE THAT REVERSED.** A portal shot
fired within one step's travel (~31.7px) of the fizzle line lives and dies inside
one `sim.step()` — the peer session named it by measuring that production's
`.after()` ordering still reaches a sync point. Counted by its own id: **371**
scanned frames in open space, **0** at the fizzle line with the scan unordered,
**1** with the scan `.after(portal_fire_system)`. ⇒ What was unreachable was the
SCAN's position, not the entity's lifetime. **A negative result is a claim about
the instrument until you have varied the instrument** — I had already written the
0 into the owner doc as a standing open gap. The edge is now load-bearing and
poison-verified: drop it and the counter reads zero, naming the edge.
  ⚠ **AND THE MINT-STABILITY MEASUREMENT WAS A RE-DERIVATION**: S1 on the same
page has recorded since 2026-09-02 that a process-global mint desyncs at frame 2.
I poisoned `SimIdCounter::next()` to learn it again, at the cost of a near-total
rebuild and a live poison in a shared tree. **Re-reading an open row beats
building.**

✔ **D-REVIEW-0904B — four AUTHORITY-AND-LIFETIME findings, all four the same
class: a value that outlived the thing it described.** Landed 2026-09-04,
`3b0d5697c` / `d4cb7e0db` / `86ede5f38` / `0d5152f3e`, each poison-verified.
(1) `ResolvedCameraSnapshot`'s `Option` kept its promise only until the first
successful resolve: the give-up arm returned WITHOUT touching the views, so each
kept the frame from the tick before the subject went away, and local views
outlive sessions. (2) `ambition_sprite_fx` was one-way in both halves — the mesh
path wrote `frame_size * scale` into `Transform` and never put it back
(COMPOUNDING 32 → 1024 → 32768 across add/remove cycles), and `Tint` wrote
`Sprite.color` with no record, so a tint was permanent and Tint→HueShift baked it
into the stored original forever. (3) `body.can`/`body.fits` asked the resting
HOME avatar, which possession has explicitly stopped driving. (4)
`GatedLockWallVerdicts` published after a fast return, so a walled room followed
by a wall-less room left the first room's verdicts standing.
  ⭐⭐ **THE MEASUREMENT THAT CHANGED A FIX, and it generalises past the camera:
`must_frame_world` HIDES AN EASE DEFECT.** The obvious regression for (1) —
cast A, gap, cast B — passes with the ease reset deleted, because the cast arm
carries the declared box the view must cover and the clamp adopts B whatever the
ease state says. The reset is observable only on the SINGLE-SUBJECT follow, whose
centre is purely the eased target: poisoned there, the centre resolves to **+284
instead of +900**. ⇒ A guard written against cast framing cannot witness an ease
defect. The test's third phase is a home avatar for exactly that reason.
  ⭐⭐ **AND THAT IS THE GENERAL RULE, not a camera detail: A CLAMP DOWNSTREAM OF
WHAT YOU ARE ASSERTING MAKES THE ASSERTION UNFALSIFIABLE.** Named jointly with
the peer session on 2026-09-04, which hit the same shape the same day in an
unrelated crate — `touchable()` clamping to `MIN_TOUCH_PX` turned a select-screen
size assertion into a tautology. Two crates, two domains, one mechanism: the
clamp supplies the value the test demands whatever the code under test did. ⇒
Before believing a passing guard, ask what sits between the thing you changed and
the thing you read — and poison the change to find out, because a clamp is
invisible in the assertion's own line.
  ⛔ **REPAIRING (3) REDDENED TWO PRE-EXISTING ROUTE TESTS**, which is the
composition-repair shape rather than a regression: both held `PlayerEntity`
alone, and `features/ecs/dormancy.rs:96` already records the same fixture trap
one domain over — *"a fixture that spawned `PlayerEntity` alone would find NO
OBSERVERS AT ALL"*. Both grew a seat.
  ⛔ **AND (4)'s PUBLISH SITE ALREADY CLAIMED THE BEHAVIOUR IT DID NOT HAVE** —
*"a room with no walls publishes an empty map rather than last room's"* sat three
lines below the fast return that made it false. A comment stating a rule is a
specification to check, not a fact to trust.

✔ **D-RECONSTITUTION — the same-room replay was a second room constructor.**
`reset_ecs_room_features` mutated twelve families of surviving entity back <!-- cite-ok: deleted; the row records the removal -->
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
named, `held_projectile_step`, was deleted by the K2 fold on 2026-09-02). Looping <!-- cite-ok: deleted by the K2 fold; the row says so -->
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
measured redundant.** `LifecycleIntent` carried `DeathReset`, `ManualReset`, <!-- cite-ok: deleted variant; the row records the removal -->
`Replay` and `FullReset`; nothing recorded any of them and a stray one would <!-- cite-ok: deleted variant; the row records the removal -->
have returned `CommitOutcome::Retry` forever — a silent stall wearing an
exhaustive match's clothes. Deleted, with their codec branches; tags 0/1/2/4 now
refuse to decode. ⛔ THE READABLE SCHEMA DUMP COULD NOT SEE THAT: same stable
name, same encoder type, same projection, so every wire ledger stayed green
while the encoding changed. `GGRS_ROLLBACK_SCHEMA_VERSION` is what that class of
change is for — 139 → 140. Mary-O's `reset_snakes_on_room_reset` was deleted, <!-- cite-ok: deleted; the row says so -->
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
`platformer_runtime` compat shim, whose own TODO asked for its deletion once <!-- cite-ok: deleted shim; the row records it -->
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

- ▢ **THE COMPILE-COST RATCHET FAILS THE GATE, and its five messages are the
  D33 campaign's own accounting.** Measured 2026-09-03 by running `./run_tests.sh`
  to completion: **9/10 jobs passed in 2023 s**, the tenth being
  `compile-cost ratchet`. Baseline frozen at `11ef33c5b5a5` (2026-08-27),
  headroom 2%.
  * ⛔ **REGRESSED** `worst_edit_cost_lines` (`ambition_geometry`) 540,227 →
    572,035 (+31,808 against a +10,804 budget), and `edit_cost_lines`
    (`ambition_platformer2d_core`) 537,395 → 568,724 (+31,329 against +10,747).
    The tool's own reading: *"Something got bigger or grew a dependency edge."*
  * ⛔ **PATH** `critical_path_crates` 14 → 15. Longer is worse *"even if every
    crate got smaller"*, because parallelism cannot compress a serial chain.
    ⚠ **Re-run 2026-09-03 late: 14 → 16, one longer again** — the abilities and
    encounter-features carves each added a crate to the serial chain, which is
    the price the `ambition_match` footprint row already predicted in writing
    ("a crate between combat and the kernel lengthens the serial chain; that is
    the honest price"). The prediction held; the row records it rather than
    treating the movement as news.
  * ⚠ **UNPRICED** `ambition_body_seed`, `ambition_held_items`,
    `ambition_registry_core`, `ambition_world_items` — all four D33
    destinations — carry NO measured compile cost and are priced at the
    population median 2.9059 ms/line. ⭐ **That is a placeholder, not an
    estimate**: the tool states size predicts compile cost at only R²=0.12, so
    every SECONDS figure above is wrong for these four by an unknown factor.
    `python3 scripts/compile_collect.py` measures them.
    ⚠ **Re-run 2026-09-03 late: it is SEVEN now, not four** —
    `ambition_abilities`, `ambition_encounter_features` and `ambition_match`
    joined as their carves landed. ⛔ Which is the shape to notice rather than
    the number: every D33 carve creates an unpriced destination, so the
    placeholder's blast radius grows with the campaign and the ruling to
    re-freeze the baseline ONCE at the end, with all destinations priced
    together, gets more right the longer it is held.
  ⊙ **RE-RUN 2026-09-03 after five more carves, and the campaign's two best
  numbers are here rather than in any prose:**
  * ⭐ **THE MONOLITH IS UNDER 100,000 LINES.** `largest_unit_lines`
    108,364 → **98,808** (−9,556), which the ratchet flags as OUTSIDE its ±2,167
    budget — in the good direction, and therefore as a baseline that is now
    stale rather than as a failure.
  * ⭐ **AND UNDER HALF THE WORKSPACE'S EDIT COST.** The monolith's
    `edit_cost_lines` share 50.5% → **46.9%** (−3.6 pts). That is the number the
    whole decomposition is for.
  * ⚠ `critical_path_crates` 14 → **16** and `UNPRICED` 4 → **7**
    (`ambition_abilities`, `ambition_body_seed`, `ambition_encounter_features`,
    `ambition_held_items`, `ambition_match`, `ambition_registry_core`,
    `ambition_world_items`) — every carve adds one of each, and the seconds
    columns stay a placeholder until someone spends the release rebuild.
  * the two REGRESSED lines moved a little further out (+32,394 and +31,915
    against ~+10,800 budgets), unchanged in character.
  * ⭐ **CARVED — two real wins the baseline is not holding**:
    `worst_edit_cost_seconds` 1,702.5 → 1,648.9 (−53.6 s) and
    `edit_cost_seconds` (the monolith) 1,264.9 → 1,139.5 (−125.4 s). The
    monolith's share of workspace edit cost fell 50.5% → 48.5%. The tool warns
    that an unfrozen win is slack: *"the guard is holding 125.4 s and the next
    regression that size lands silently."*
  ⇒ **NOT RE-FROZEN HERE, deliberately.** `--update` rewrites every number at
  once, so banking the two wins would bank the two regressions with them — the
  identical trap the doc-link ratchet had (`bf4e6f353`), where the advice to
  bank an improvement printed while the regressions stayed hidden. The
  regressions need a carve owner to say whether +31.8k lines is a deliberate
  landing; the four unpriced crates need a measurement, not a ruling.

  ⇒ ⭐ **RULED 2026-09-03: THE RATCHET STAYS RED UNTIL THE CARVE CAMPAIGN ENDS,
  THEN IS RE-FROZEN ONCE WITH EVERY DESTINATION PRICED.** Re-freezing per carve
  is the tempting move and it is wrong for the reason this row already gives —
  `--update` rewrites every number at once, so each carve would bank its own
  regressions alongside its wins, and after five carves nobody could say which
  of the accumulated numbers was ever examined. ⛔ A ratchet re-frozen at every
  landing is not a ratchet; it is a record of the last landing.
  * **So a red ratchet is the EXPECTED state during D33, not a defect to
    triage.** Five crates left the actor monolith on 2026-09-03 alone. The
    figure to watch meanwhile is `critical_path_crates` (14 → 15 at the last
    reading): per-crate sizes fall as a carve splits them, but a longer SERIAL
    chain cannot be compressed by parallelism, so that is the one that says
    whether the campaign is buying compile time or spending it.
  * **The re-freeze has a prerequisite, not just a date.** All four unpriced
    destinations must carry a measured compile cost first
    (`python3 scripts/compile_collect.py`) — freezing a baseline that contains
    placeholders at the population median would bake R²=0.12 guesses in as
    though they were measurements, which is the same class of error as the
    unreproducible figures `README.md` now warns about.

- ✔ **A ROLLBACK TEST IN THE CAPABILITY DEMO DIED IN BEVY BECAUSE THE BACKEND
  DECLARED TWENTY DOMAINS' STATE. IT PASSES.** `examples/capability_demo`'s
  `rollback_round_trip::the_cooldown_the_profile_and_the_velocity_survive_a_real_rewind`
  panics with `called Result::unwrap() on an Err value:
  ArchetypeExists(ComponentId(49))`, raised inside `bevy_ecs-0.19.1`'s
  `world/mod.rs` — the shape of a component registered twice in the demo's
  rollback setup.
  ⭐ **Reproduced in BOTH exhaustive runs of 2026-09-03**, identically, including
  the first on `7ca535427` — before `ambition_abilities` existed — so it is
  pre-existing and not a carve regression.
  ✔ **PREMISES RE-CHECKED 2026-09-03 late without a build:** `7ca535427` is
  still an ancestor of HEAD; the test still exists, at
  `examples/capability_demo/tests/rollback_round_trip.rs:115`; and `Cargo.lock`
  still pins `bevy_ecs 0.19.1`, so the panic's frame is the same crate version
  it was raised in. ⛔ The panic ITSELF needs a build and was not re-run. ⚠ I first hypothesised mid-suite disk
  exhaustion for this job and that was WRONG: `external consumer: outlander`
  failed for that reason and passes now, while this one fails the same way on a
  tree with room to spare. A plausible cause is not a diagnosis; running it twice
  is what separated them.
  ✔ **REPRODUCED AND THE FIRST CAUSE IS FIXED (2026-09-04). It was not a double
  registration between crates — it was ORDER inside the demo's own `compose`.**
  `insert_session_world_component` SPAWNS, which creates an archetype, and it ran
  BEFORE `AmbitionRollbackPlugin` and `PulsePlugin` were added; a plugin that
  then calls `register_required_components` for a component already in that
  archetype hits Bevy 0.19's `ArchetypeExists` — `bevy_ecs-0.19.1`'s `world/mod.rs` line 407
  is `try_register_required_components(..).unwrap()`. Spawning after the plugins
  removes it, measured: the panic changes rather than persisting.
  ✔ **THE SECOND FAULT IS NAMED AND FIXED (2026-09-04), and it was found by
  READING rather than by naming the system.** *"Parameter `…::messages` failed
  validation: Message not initialized"* is
  `ambition_platformer2d_rollback_ggrs::session::retire_rollback_authority_with_its_scope`,
  which takes `MessageReader<SessionScopeRetired>`. That crate registers NO
  messages at all (`git grep 'add_message::<'` in it returns nothing);
  `SessionScopeRetired` is registered by the shared tangle's session-lifecycle
  plugin, which this composition does not add. ⇒ **The same class as
  `sync_portal_view_cones` and `ambition_sprite_fx`, a third time, and this time
  the prerequisite is a MESSAGE rather than a resource.**
  ⭐ Guarded with `run_if(resource_exists::<Messages<SessionScopeRetired>>)`
  rather than by registering the channel: a host with no session-scope lifecycle
  retires no scopes, so there is nothing for the system to do, and registering
  here would make the rollback backend a second registrar of somebody else's
  vocabulary — two cleanup systems in a host that has both.
  ⭐⭐ **AND THE FIX IS VERIFIED BY THE PANIC MOVING, not by the test passing:**
  it is now *"Parameter `…` failed validation: Resource does not exist"*.
  ⭐⭐⭐ **THE THIRD ONE IS NAMED WITHOUT ANOTHER BUILD, AND IT IS NOT A THIRD
  BUG — IT IS THE ROOT ALL THREE COME FROM.** `AmbitionRollbackPlugin` calls
  `register_engine_rollback_state`
  (`platformer2d_runtime/src/rollback/mod.rs:40`), which declares **twenty
  domains'** rollback state — encounter, combat, the actor monolith, mount,
  characters, the floor, time, boss encounter, conversation, sprite sheet, the
  shared tangle, vfx, items, cutscene, persistence, projectiles, sim_view, world
  rooms and gate portals — with exactly ONE of the twenty behind a `#[cfg]`
  (`ambition_portal2d`, `feature = "portal"`).
  ⇒ **So installing the rollback BACKEND installs save/restore/checksum systems
  for every engine domain, composed or not.** The concrete next panic:
  `rollback_resource_canonical::<MovingPlatformSet>`
  (`rollback/mod.rs:101`) installs `bevy_ggrs`'s `ResourceChecksumPlugin`, whose
  system takes `Res<T>` — and `MovingPlatformSet` is initialised by
  `platformer2d_runtime/src/sim_core_resources.rs:85`, a plugin this demo does
  not add. ⚠ The registrar's OWN doc comment already records this exact failure
  for a different type: *"panics on any frame the resource is absent — `Parameter
  `Res<'_, ActiveMatch>` failed validation: Resource does not exist`"*.
  ⛔⛔ **AND THIS IS A DIRECT VIOLATION OF THE COMPOSABILITY DOCTRINE'S OWN
  PRINCIPLE**, which reads *"domain-owned rollback/content declarations compose
  through backend-neutral registrars"*. The REGISTRAR is backend-neutral; the
  CALL SITE is not domain-conditional — it lives in the backend's plugin and
  names all twenty. ⇒ Fixing the Nth resource with
  `rollback_resource_optional_canonical` (as `ActiveMatch` was) treats a symptom;
  the seam is that a domain should declare its own rollback state when that
  domain is composed. Recorded as a design item in
  [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).
  ⇒ **So the demo's remaining faults are not a list to grind down — they are one
  seam, and the count is SIX** (measured by
  `scripts/rollback_checksum_prerequisites.py`, committed): `MovingPlatformSet`,
  `GatePortalPhases`, `MintedItemBaseline`, `OwnedItemsBaseline`,
  `CustodyBaseline`, `OccurrenceBaseline`. **Four of the six are inserted only by
  `sim_core_resources` or the actor monolith**, neither of which this demo adds,
  so those four are guaranteed frame-one panics waiting in order.
  ✔✔ **CLOSED 2026-09-04, AND THE SEAM IS THE PROOF: `GgrsBackendPlugin` SPLIT
  OUT OF `AmbitionRollbackPlugin`, AND ALL SIX FAULTS WENT AT ONCE.** The demo's
  rollback round trip passes, and its whole suite is **21 passed / 0 failed**.
  Installing the GGRS backend and declaring the ENGINE'S rollback state were two
  jobs fused into one plugin; `AmbitionRollbackPlugin` is now `GgrsBackendPlugin`
  plus `register_engine_rollback_state`, so every existing engine composition is
  unchanged and a capability host composes the backend and declares its OWN
  types.
  ⭐ **That one change removing all remaining faults is what makes "one seam, not
  a list" a measurement rather than a claim** — a list would have needed six
  fixes.
  ⛔ **AND THE PAIRING IS THE CONTRACT, stated on the new plugin:** a host that
  composes engine domains and `GgrsBackendPlugin` instead gets those domains
  WITHOUT their rollback state, which is a desync rather than a smaller game.
  This is not a reduced engine.
  ⚠ The twenty-domain call site still names all twenty unconditionally — the
  design item in
  [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md)
  stands. What changed is that a minimum host no longer has to care.
  ⚠ Only two declaration forms reach the unwrapped `Res<R>` —
  `rollback_resource_canonical` and `rollback_resource_clone_checksum`. The
  `_optional_canonical` form exists to tolerate absence and the plain `_clone`
  forms use `ResourceSnapshotPlugin`, which maps `(Some, None)` to
  `remove_resource`. That is why the population is six and not the whole
  declaration set.
  ✔ **The guard's own crate is green: `ambition_platformer2d_rollback_ggrs` 54
  passed / 0 failed** (the commit that landed it said the suite was still queued
  behind a saturated machine; it has reported since). ⚠ And that green means less
  than it looks: nothing in that crate observes the registration either way —
  `session_ownership_tests` schedules the system DIRECTLY and
  `host_invariant_tests` asserts only schedule and resource facts — so dropping
  the system entirely would also be green. The reading is the verification; the
  suite only rules out collateral damage.
  ⚠ **How to find them without guessing:** `bevy_ecs` is not a direct dependency
  of `capability_demo`, so `--features bevy_ecs/debug` is rejected; adding it as
  a dev-dependency for one run is what names them. And the build must be
  redirected — `CARGO_TARGET_DIR=$PWD/target/capability_demo` — because a nested
  workspace's `target/` is on the SHARED volume, which is what filled it the
  first time.
  ⇒ The superseded next step, kept for its reasoning: that
  name, on a box with headroom.
  ⚠ **The demo builds into `examples/capability_demo/target`, which is NOT the
  bind-mounted `target/`** — it is on the shared virtiofs volume, and one
  feature-flag change cost 14 GB of it. Whoever picks this up should expect a
  second full engine build per feature combination.
- ▢ **~47 GB ON THE SHARED VOLUME IS UNACCOUNTED FOR, AND THE NEW BINDMOUNT
  RULE DESCRIBES EXACTLY THIS SHAPE. REPORTED, NOT ACTED ON.**
  Measured 2026-09-03 late on the calculex box, where `/dev/vda1` reads **278 GB
  used of 290, 12 GB free** — below `check_disk_headroom.py`'s 40 GB floor, so
  no Rust lane can start here at all. Walking the volume accounts for about
  **231 GB**: `/home/agent` 224 (of which `.cache/ambition-targets` is 176, one
  store, `BOUND` and healthy per `target_bindmount.sh --status`), `/usr` 5.4,
  `/var` 1.1, `/tmp` 0.24. The repo itself is NOT on this volume — it is
  virtiofs — so it is not the remainder.
  ⛔ **AGENTS.md's binding rule, added the same day, names this shape:** the
  bind mounts the backing store OVER `<worktree>/target`, so artifacts built
  before the bind are HIDDEN rather than removed and still occupy every byte;
  binding an 81 GB unbound worktree moved a volume 34 → 33 GB free. A shadowed
  copy is invisible to `du`, which can only walk what the mount exposes. That
  makes it the leading candidate for the gap and NOT a demonstrated one.
  ⚠ **WHAT WAS RULED OUT, so nobody repeats it:** deleted-but-open files hold
  **0 bytes** (`lsof +L1`), and the two `vda1[...]` bind mounts point at the
  SAME backing store from two paths, which is one copy and not two.
  ⇒ **The prescribed action is the one taken here: report the size and stop.**
  Confirming a shadowed copy means unmounting, and removing one is `rm -rf`
  under a `target/` — ⛔ **Jon's call, not an agent's, and not less so because
  the bytes may be an agent's own.** The volume is also shared with other
  sessions' worktrees, so anything reclaimed here could be someone's live build.
  ⓘ Practical consequence until it is resolved: this box runs `--tool-tests`,
  `--maintenance` (minus its `cargo doc` job) and every pure-Python checker, and
  cannot run a Rust lane. Route Rust gating to a box with headroom.

- ✔ **CLOSED 2026-09-03 by `scripts/lib/canonical_assets.py`** — the first of
  the two candidate shapes below, and taken by another session rather than by
  the one that filed it, which is the point of filing a decision instead of
  guessing at it. The condition is DETECTED rather than remembered: a checkout
  whose sprite tree holds real files generated them and is canonical; one
  holding symlinks is borrowing another checkout's and its assets are not its
  own to ratchet; an absent tree has nothing to check. ⇒ The gate stays, which
  the row insisted on, and stops depending on anyone exporting a variable.
  Original row kept below for its reasoning.
- ✔ **TEN ASSET RATCHETS WERE GATED BEHIND AN ENVIRONMENT VARIABLE NOTHING SET.**
  Closed 2026-09-04, re-measured rather than taken on report: both files now read
  `ASSETS_ARE_CANONICAL = assets_are_canonical(_REPO)` with
  `NOT_CANONICAL_REASON = why_not(_REPO)`, so the gate answers itself — real
  files, no borrowed symlinks, and tier variants that are FRESH — and
  `AMBITION_ASSETS_ARE_CANONICAL` survives only as a manual override. The first
  evaluation of the ten was 2026-09-03 and it found something on the first try;
  see the row below. ⛔ **Both files' docstrings still told the reader the old
  story** — *"nothing in the repository sets it … run by hand or not at all"* —
  and were repaired in the same commit: a gate that moves leaves its own
  instructions behind, and those instructions are what the next reader acts on.
  The reasoning below is kept because it is the argument for why the gate exists
  at all, which the auto-detector implements rather than replaces.

  ⛔ **HISTORICAL — the state the row was opened on: TEN ASSET RATCHETS ARE
  GATED BEHIND AN ENVIRONMENT VARIABLE NOTHING SETS, AND TWO PLANNING
  PARAGRAPHS CALL THEM RATCHETS ANYWAY.**
  `scripts/tests/test_shipped_sheet_pages_are_claimed.py` (5 assertions) and
  `scripts/tests/test_tier_variants_are_actually_smaller.py` (5) are marked
  `@pytest.mark.skipif(not os.environ.get("AMBITION_ASSETS_ARE_CANONICAL"))`.
  ⛔ **Nothing in the repository sets that variable** — whole-tree grep
  2026-09-03 late, 12 hits and every one of them the skipif, its reason string,
  or the two files' own prose telling you to opt in by hand. So the suite has
  never evaluated them: 13 skips in the `scripts/tests` lane tonight, 10 of them
  these.
  ⭐ **THE GATE IS NOT A MISTAKE, WHICH IS WHY THIS IS NOT A ONE-LINE FIX.** The
  sprite tree is gitignored and machine-local, every PNG under `assets/sprites*`
  is generated, and a worktree SYMLINKS the main checkout's copies. On a box
  that regenerates cleanly the `KNOWN_` lists read as stale and the guards fail
  for the wrong reason — the failure mode the gate was added to prevent. Both
  files say so in their own headers, and both already say nothing sets the
  variable, so this was known at the point of writing and never carried outward.
  ⚠ **What is wrong is the PLANNING claim, not the test design.**
  [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md)
  said *"Ratcheted by ... so the count cannot grow"* and *"Poison-verified in
  every direction"* of guards that no lane runs. Both were poison-verified — by
  hand, at the time, with the variable set — and that is a different statement
  from a standing ratchet. Both paragraphs now carry the qualification; this row
  is the open question they point at.
  ⇒ **The decision is where the canonical box is.** A ratchet that only runs
  when someone remembers a magic variable protects nothing, and deleting the
  gate makes every worktree red. Candidates: a `--maintenance` job that sets it
  only when `assets/sprites/` is a real directory rather than a symlink; or
  publishing a measured census as a checked-in baseline so the guards compare
  against a recorded state instead of a local tree. ⛔ Do not simply remove the
  skipif. ⓘ **CORRECTION (2026-09-03 late): my "not diagnosable on this box" note was
  WRONG, and wrong because I globbed the repo root.** The trees live at
  `crates/ambition_platformer2d_actor_monolith/assets/sprites*`, and this
  checkout has four of them holding **425 real PNGs and zero symlinks** — it is
  canonical by the new detector, which returns True here. ⇒ So the ten
  assertions run on this box now, for the first time, and my reason for not
  diagnosing it was an artifact of looking in the wrong directory rather than a
  fact about the machine.

- ▢ **THE TEN RATCHETS RAN FOR THE FIRST TIME AND FOUND SOMETHING ON THE FIRST
  TRY: FOUR NAMES IN `KNOWN_STRANDED_SHEETS` ARE STALE.**
  With `canonical_assets.py` landed and this checkout detected as canonical
  (four sprite trees, 425 real PNGs, zero symlinks), the ten assertions that
  had never been evaluated by any lane ran here 2026-09-03 late: **23 passed,
  1 failed.** ⭐ **AND THE LANE ARITHMETIC CLOSES EXACTLY**, which is what shows
  the module did what it said rather than something adjacent: the full
  `scripts/tests` run went **803 passed / 13 skipped → 819 passed / 3 skipped**.
  +16 passed and −10 skipped, against ten assertions that stopped skipping and
  sixteen that started executing — the difference being the six that were
  skipping on the sprite-tree check underneath the marker, not on the marker
  itself. No test appeared or vanished unaccounted for. The failure is `test_the_known_list_does_not_rot`:
  `carl_stargan`, `pointed_polygon`, `projectile_polygon` and
  `pugnacious_polygon` **no longer strand pages** and must leave the list, or a
  future regression on them passes silently.
  ⭐ **THIS IS THE RATCHET WORKING, NOT BREAKING.** The list is stale in the
  GOOD direction — four sheets were fixed and nobody pruned their names,
  because nothing was running the check that would have said so. A rot test
  that only ever grows is not a ratchet, which is the reason that assertion
  exists at all.
  ⚠ **AND IT IS NOT THE FAILURE THE ORIGINAL GATE FEARED**, which is worth
  separating carefully. The gate's stated worry was that on a box regenerating
  cleanly the `KNOWN_` lists would "read as stale and the guards fail for the
  WRONG reason". Here the box is genuinely canonical by the new detector's own
  test, so the staleness is REAL and the remedy is to prune the four names —
  not to re-gate, and not to widen the list.
  ⇒ Owner: whoever regenerated those four sheets. The edit is four deletions
  from `KNOWN_STRANDED_SHEETS` in
  `scripts/tests/test_shipped_sheet_pages_are_claimed.py`, and the guard's own
  poison discipline (drop a name → new-orphan test red) means the change is
  self-checking. ⛔ Not done here, and the reason got SHARPER on inspection:
  ⛔⛔ **THE EVIDENCE FOR THIS EDIT IS GITIGNORED GENERATED OUTPUT, SO NO BOX
  CAN ATTRIBUTE IT.** The `.ron` manifests are **not tracked** — 0 by
  `git ls-files`, 471 on disk — so "does this sheet strand pages" is entirely a
  property of one machine's regenerated tree, exactly the class the
  re-measuring recipe warns about.
  ⛔⛔ **AND AS OF 2026-09-04 THE TEN DO NOT RUN HERE AT ALL, so the "23 passed,
  1 failed" reading above is not reproducible on this box today.** The detector
  is explicit about why: *"sprite trees are this checkout's OWN (all 2172 png(s)
  real, no symlinks) but the quality tiers are STALE"* — 5 source manifests have
  NO variant at a tier the game ships (`performer` has no 0.5x, 0.25x or potato;
  `actor_spritesheet` and `medic` have no potato) and 190 published tier files
  are up to 9.8 days behind their sources. ⭐ **That is a KNOWN consequence, not a
  new defect:** `engine/performer-up-b-the-wire.md` records that the
  `actor_* -> performer_*` rename travelled in the catalog and the regen scripts
  but not on disk, and `sprites_potato/` here still holds the pre-rename
  `robot_medic_*`. ⇒ The unblock is `./scripts/regen/quality_variants.sh` (incremental).
  ⭐ **RUN 2026-09-04: 184.5s, 213 sheets x 3 tiers + 36 parallax x 3, 8 orphans
  pruned per tier. The 5 MISSING variants went to ZERO and the 190 stale went to
  FOUR.** `performer`, `actor_spritesheet` and `medic` now have every tier the
  game ships, so the pre-rename gap is closed on this box.
  ⛔⛔ **AND THE LAST FOUR CANNOT BE REGENERATED, WHICH IS A PIPELINE BUG RATHER
  THAN A STALE BOX.** They are `officer_actor.ron` and `officer_portraits.ron` in
  `sprites_0_5x` and `sprites_0_25x`, dated 2026-08-27 against a source from the
  same day ten hours later. A targeted re-run (`--sprites-only --target
  'officer*'`) builds NOTHING in 1.0s, while the full pass that refreshed
  `boss_actor.ron` to today leaves these two alone. The tier trees carry **47
  `_actor.ron` sidecars against 192 in `sprites/`**, so the pass emits a SUBSET —
  and these two are sidecars it no longer produces and the orphan prune does not
  take.
  ⇒ **So `assets_are_canonical` stays false here permanently, and the ten
  ratchets can never evaluate anywhere until this is settled** — a bigger
  consequence than two files. They are either orphans the prune should remove or
  a target the pass stopped covering; the answer belongs to whoever owns the
  sprite renderer's tier pass. Until then, do not treat the known-list red as
  attributable and do not treat a SKIP as a pass.
  ⚠ **AND ON THIS BOX THE FOUR ARE CLEAN FOR THE WRONG REASON.** Each has four
  manifests (one per tier) and **ZERO numbered pages**: they do not strand
  pages because they never spilled past a single page here, not because a
  manifest was corrected to claim them. Whether a sheet spills is a property of
  the atlas packing, which is generated. ⇒ So a box that packs them across
  pages could strand them again, and deleting the names on this evidence would
  retire a guard against a regression that was never fixed — the exact failure
  `test_the_known_list_does_not_rot` exists to prevent, committed by the person
  reading its output.
  ⇒ **Leave it red with a named owner.** The edit is safe only for someone who
  can show the four REGENERATED and claiming their pages, not merely absent.
  ⛔⛔ **AND ON A DIFFERENT BOX THE SAME RATCHETS WENT RED FOR A REASON THAT IS
  NOT CONTENT AT ALL.** Within the hour, another checkout reported
  `..._that_is_not_smaller` naming actor/author/medic/officer and
  `…without_a_reduced_variant` naming `performer` — and
  `check_quality_variants_are_fresh.py` exits 1 there with **170+ stale files**,
  `performer` among them. ⇒ **All five of those reds are regeneration
  staleness.** A size assertion on a stale tier compares a fresh source page
  against an old reduced one and reports a history finding dressed as a content
  one. ✔ **FIXED IN THE DETECTOR, not in the assertions** (`7761e3646`):
  `assets_are_canonical` now also requires the freshness check to be green, and
  a stale box SKIPS with a reason naming the check and its output. ⭐ The
  distinction this preserves is the whole point — THIS box is fresh, so its
  four stale names are a real finding to act on; that box is stale, so its five
  are not findings at all. Widening the assertions would have erased the
  difference.

✔ **RETIRED 2026-09-03 — two rows whose work landed.** The file's own contract at the top says
completed investigations do not stay here; both of these were still carrying `▢`, which made the
queue read as an execution authority for work already done.

* **`why_not()` stating a cause it never checked** — fixed. The message is derived from the same
  counts the predicate uses (`_scan_tree_files`), and a guard pins that the two agree. It has
  since been strengthened twice more: a freshness precondition, and an exhaustive scan replacing
  the 25-file sample that let a regenerated PREFIX classify a borrowing tree as canonical.
* **Nine debug binaries at 99.8% the same program** — done, and measured rather than predicted.
  `smash_tool` is one modal `clap` CLI: **4.79 GB → 0.50 GB, output byte-identical**, with
  `test_documented_bin_targets_exist.py` guarding the documented entry points. ⭐ The row's own
  "try the one-line lever first" was tried and REJECTED on measurement: `split-debuginfo=unpacked`
  shrank the executable 41% but wrote 532 MB of `.dwo`, so total on disk went UP 324 MB — the
  redundancy it exploits is exactly what the collapse had already removed, and harder. Recorded at
  the setting in `Cargo.toml` so nobody re-runs it.


- ✔ **THE DIAGNOSED GATE IS FIXED, AND THE PLACEHOLDER IS GONE — OBSERVED BY A
  TEST THAT REPRODUCES IT WHEN THE GATE COMES BACK.** ⭐ **Re-measured 2026-09-04: this row's own named fix candidate —
  *"scope the early return to the parallax spawn alone rather than to the whole
  function"* — LANDED at `9ac1111af`** (an ancestor of HEAD, re-checked), and
  `2026-09-04` extended the same seam so a theme the loader resolved to nothing
  settles instead of retrying. `sync_session_room_visuals` now calls
  `spawn_room_visuals` BEFORE it consults the parallax memo
  (`crates/ambition_render/src/platformer_presentation.rs:265`), guarded by
  `the_room_presents_even_though_its_parallax_theme_has_not_arrived`, which
  counts actual session-scoped `RoomVisual` entities rather than a memo.
  ✔ **CLOSED 2026-09-04 — THE OBSERVATION IS A TEST NOW, and it confirms the
  mechanism rather than arguing it.**
  `an_authored_room_npc_never_wears_the_unclaimed_placeholder`
  (`crates/ambition_render/src/platformer_presentation.rs`) puts one authored NPC
  placement and one `FeatureViewIndex` row in a session, runs the room spawner
  and `draw_unclaimed_feature_views` together for two frames past the 5-frame
  grace period, and asserts the NPC has no `UnclaimedBodyPlaceholder`.
  ⛔ **Its CONTROL arm is the whole reason it means anything:** a second id with
  a view row and no placement MUST get a stand-in, so a build where the
  placeholder never draws at all fails instead of passing.
  ⭐ **Poison-verified by restoring the pre-`9ac1111af` gate — one `return` when
  the parallax scope has not settled — and it reproduces the reported symptom
  verbatim:** *"an authored room NPC wore the placeholder … Got
  ["a_body_the_room_never_authored", "npc_room_greeter"]"*. ⇒ The gate withheld
  `spawn_room_visuals`, the NPC's view went unclaimed, and at frame 5 the
  stand-in drew — which also explains the tier dependence the row could not:
  Potato was clean because `parallax.enabled` is false there, so the gate never
  engaged.
  ⚠ **The limit, stated rather than left implicit:** it is a hand-built app, not
  the shipped composition. It pins the claim path and the gate's effect; it
  cannot say the shipped game has no OTHER source of the same symptom at Ultra.
  A screenshot would have said that and nothing else — which is why this is the
  better of the two and not a replacement for someone eventually looking.
  ⛔ Everything below is the DIAGNOSIS that produced that fix, kept because the
  discriminator is still the right test and the retraction is only legible beside
  what it retracts. It is not a description of HEAD.

  ⛔ **HISTORICAL DIAGNOSIS — EVERY INTERACTABLE ROOM NPC IS DRAWN BY A
  PLACEHOLDER FOR ITS FIRST FRAMES, BECAUSE ITS BUNDLE CARRIES NONE OF THE FIVE
  MARKERS THE VIEW REBUILD SELECTS ON.** Traced by e7 2026-09-03, ruled a DEFECT and not a
  design call. `rebuild_dynamic_feature_views`
  (`crates/ambition_sim_view/src/facts.rs:517`) selects by MARKER —
  `EncounterMob`, `RuntimeStagedActor`, `PostBossNpc`, the two reward chests
  (`EncounterRewardChest`, `BossRewardChest`), and `SpawnOrigin::Dynamic` loot
  by construction PROVENANCE. The peaceful-NPC bundle in
  `crates/ambition_platformer2d_actor_monolith/src/features/ecs/spawn_actors.rs`
  carries none of them. ⇒ The NPC lands in `FeatureViewIndex` and never in
  `DynamicFeatureViews`, so its `FeatureVisual` is created by
  `draw_unclaimed_feature_views` as the PLACEHOLDER — which
  `upgrade_actor_sprites`' query does not exclude, so **the stand-in entity
  becomes the body** and `UNCLAIMED_STAND_IN_GRACE_FRAMES` becomes a mandatory
  5-frame placeholder on every interactable NPC in every room.
  ⛔⛔ **RETRACTED THE SAME HOUR — THE MARKER READING IS WRONG, AND SO IS MY
  CONFIRMATION OF IT.** `spawn_room_visuals`
  (`crates/ambition_render/src/rendering/world.rs:221-232`) iterates
  `spec.placements` and calls `spawn_authored_interactable` for every
  `Interactable`, which spawns the `FeatureVisual` for exactly the NPC ids —
  the same channel that handles enemies at :187 and bosses at :200. ⇒ **A room
  NPC's visual is supposed to come from the ROOM SPAWNER.** The five dynamic
  markers are the right filter for DYNAMIC bodies, and the NPC bundle carrying
  none of them is EXPECTED, not the defect.
  ⚠ **I verified three links of a chain that is not the operative path**, and
  reported the mechanism as closed. Every check below was individually true;
  the premise underneath them — that the dynamic rebuild is where a room NPC's
  view should come from — was never checked, because it was the assumption the
  trace arrived with. ⭐ **A confirmed mechanism is not a confirmed
  DIAGNOSIS**: the links held and the chain was still the wrong one.
  ⇒ **THE OPEN QUESTION IS TIMING, NOT MARKERS.** Why do the room spawner's
  visuals land more than 5 frames after the bodies' `FeatureViewIndex` rows at
  Ultra and within 5 at Potato? Candidates: a tier-dependent gate on the room
  spawner's session caller (a `theme_loaded` gate was read there and retracted
  for painted_blocks — it may be this), or `spawn_authored_interactable`
  returning early on an unready sprite. ⛔ **The fix shape is NOT a marker on
  the NPC bundle**; it is the room spawner's gate, or the stand-in's grace
  clock. The probe is the frame of `spawn_room_visuals` against the frame each
  row appears — which needs a build.
  ⭐⭐ **AND THE GATE IS FOUND, BY READING, 2026-09-03 late — IT IS THE PARALLAX
  THEME.** `sync_session_room_visuals`
  (`crates/ambition_render/src/platformer_presentation.rs:198`) calls
  `spawn_room_visuals` at :274. At :244-262, BEFORE that call, it computes
  `wants_parallax` from `quality.budget.parallax.enabled` (defaulting TRUE),
  and if parallax is wanted while the room's `ParallaxTheme` has none of its
  layers loaded it **`return`s early** — with the comment *"Leave `presented`
  unset so the next frame retries"*. So no room visual of any kind spawns until
  the PARALLAX ART is resident.
  ⭐ **THAT IS THE TIER DEPENDENCE, EXACTLY AS OBSERVED.**
  `crates/ambition_persistence/src/settings/video/tests.rs:144` asserts
  `!potato.parallax.enabled` — parallax is OFF at Potato, so `wants_parallax` is
  false, the gate never engages, and room visuals spawn immediately, within the
  5-frame grace. At Ultra parallax is on, so every room visual — the NPC's
  included — waits on parallax layer loads, which is asset-bound and trivially
  exceeds 5 frames. ⇒ **A backdrop's residency gates every interactable's
  visual**, which is the actual defect: the two have no reason to share a
  deadline.
  ⭐ **AND IT PREDICTS A DISCRIMINATOR, which is what makes it cheap to test.**
  The gate is in front of ALL of `spawn_room_visuals`, so at Ultra it should
  delay AUTHORED room enemies
  (`crates/ambition_render/src/rendering/world.rs:187`) and bosses (:200) as
  much as NPCs — they are downstream of the same early return. ⇒ But NOT
  encounter-WAVE mobs or runtime-staged actors: those carry `EncounterMob`
  (added at `spawn_actors.rs:1348`) and `RuntimeStagedActor` (:305), so the
  dynamic rebuild gives them a view by a second road the gate does not touch.
  ⛔ **So the shape to look for is authored-vs-dynamic, NOT npc-vs-everything.**
  If NPCs alone are late while authored room enemies are on time, this
  explanation is wrong and something NPC-specific is in play.
  ✔ **OWNED 2026-09-03 late by the session holding the presentation lane**,
  which had read this same gate that evening and retracted it for
  painted_blocks — right mechanism, wrong failure, found twice from two
  directions. ⭐ **The authored-vs-dynamic discriminator is the ACCEPTANCE
  CRITERION and is checked BEFORE the fix is written**: NPCs alone late while
  authored room enemies are on time ⇒ this explanation is wrong and they stop.
  Fix shape as stated: scope the early return to the parallax spawn.
  ⛔ Still needs a build to CONFIRM the frame numbers; the mechanism above is
  read from source and the tier assertion from an existing test. ⇒ The fix
  candidate this points at is scoping that early return to the parallax spawn
  alone rather than to the whole function — not the grace clock, which would
  only widen the window the backdrop is already blowing through.
  ⓘ Kept below rather than deleted, because the individual findings stay true
  and the retraction is only legible beside what it retracts:
  **THE SELECTOR AND THE BUNDLE, re-checked 2026-09-03 late without a build:** `crates/ambition_sim_view/src/facts.rs:517`
  is exactly
  `rebuild_dynamic_feature_views`, its filters are the five named above and no
  others, and a scan of the NPC spawn function's whole body finds **zero**
  occurrences of any of the five. ⭐ **AND THE THIRD LINK CHECKS TOO:**
  `upgrade_actor_sprites` (`crates/ambition_render/src/rendering/actors/mod.rs:639`)
  queries `(Entity, &FeatureVisual, Option<&BoundFeatureKind>,
  Option<&BoundSpriteQuality>)` with **no `Without<>` filter of any kind**, so
  nothing in its signature excludes a placeholder `FeatureVisual` — which is
  precisely the step that lets the stand-in become the body. ⛔ Not checked: the
  runtime consequence itself, that the stand-in is what actually draws, which
  needs a build. ⇒ So every link a grep can reach holds, and what remains
  unverified is the observation, not the mechanism.
  ⛔ **SUPERSEDED — see the retraction at the top of this row.** The original
  prescription was: the NPC bundle carries whatever
  `rebuild_dynamic_feature_views` selects on, or the actor family claims its
  view on spawn. That is aimed at the wrong path; the room spawner already owns
  this visual. ⛔ **THE GUARD SHIPS AFTER THE FIX**, asserting the property the fix
  establishes — an interactable NPC's view is claimed the frame its body
  exists, never by the stand-in — and poisoned by dropping the marker. A guard
  written first would pin TODAY'S behaviour, which is the bug. e7 has a
  throwaway probe and is deliberately not shipping it; e7 writes the real guard
  once the fix lands.
  ⓘ **THE PLACEHOLDER IS ALREADY MARKED, which the guard will want and the fix
  should NOT use.** `crates/ambition_render/src/rendering/features.rs` defines
  `UnclaimedBodyPlaceholder` alongside `UNCLAIMED_STAND_IN_GRACE_FRAMES: u32 = 5`
  (line 354), and its own test queries `(&FeatureVisual, &UnclaimedBodyPlaceholder)`
  — so "is this visual the stand-in?" is answerable in one component today, and
  the guard can assert on it directly instead of inferring from a frame count.
  ⛔ **But adding `Without<UnclaimedBodyPlaceholder>` to `upgrade_actor_sprites`
  is NOT the fix and must not be mistaken for one.** It would stop the stand-in
  being upgraded into the body and leave the NPC with no real view at all —
  still absent from `DynamicFeatureViews`, now with nothing drawing it. The
  defect is that the NPC never enters the rebuild; the query is where the
  symptom becomes visible, not where the cause lives.
  ⓘ Needs an owner who can BUILD. Declined here 2026-09-03: this box is at
  11.8 GB free of 290, below the 40 GB floor, so the Rust lane refuses to start
  and the fix could not be gated.

- ✔ **THE S4 `SimId` CENSUS PASSES BECAUSE ITS POPULATION EXCLUDES THE ONE ROAD
  THAT WOULD FAIL IT.** Found 2026-09-03 with ToothbrushAmbition — they found the
  road, I checked the census against it, and both halves are independently
  derived.

  1. **A boss death drop mints no identity.**
     `features/ecs/damage_drops.rs::drop_held_weapon` spawns `GroundItem`,
     `Name`, `RoomScopedEntity`, a `dynamic_drop_origin` and `SpawnedThisAttempt`
     — and **no `SimId`**. Its `parent: &SimId` argument is consumed for
     provenance only, which its own doc comment states as deliberate.
     ⚠ **CORRECTED WITHIN THE HOUR: I first wrote "no `SimId` and no
     `ItemCustody`", and the `ItemCustody` half is WRONG.** `GroundItem` carries
     `#[require(ItemCustody)]` (`ambition_held_items/src/lib.rs:269`), so Bevy
     0.19 inserts it automatically on every one. Checking the spawn call alone
     shows what the caller lists, not what the entity ends up with — a required
     component is invisible at the call site. ⇒ Of the four components
     `capture_minted_item_baseline` queries, the drop has `GroundItem`,
     `ItemCustody` (required) and `SpawnOrigin` (from `dynamic_drop_origin`, which
     returns `SpawnOrigin::Dynamic`). **Exactly ONE is missing, and it is
     `SimId`.** The finding is unchanged and its cause is narrower than I said.
  2. **A `GroundItem` IS a rollback anchor**, said twice in the tree's own words:
     `ambition_held_items/src/lib.rs:1162` (*"it lives on a `GroundItem`, which is
     a rollback anchor"*) and `crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs:304` (*"which is already
     an anchor"*).
  3. **The census asserts every rollback-anchored entity has a unique `SimId`**
     (`game/ambition_app/tests/rollback_populated_timeline.rs:322`,
     `every_rollback_anchored_entity_has_a_unique_sim_id_on_the_populated_timeline`),
     and [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
     records it as holding *with no waiver list*.
  4. ⛔ **Its `populate()` spawns five things and none of them is a ground item:**
     a sentry, a vortex well, a temporary gravity well, a falling hazard and a
     portal shot. Zero `GroundItem`, zero `drop_held_weapon`.

  ✔ **RUN 2026-09-03, AND IT FIRES.** I added the drop road's SHAPE to
  `populate()` — a `GroundItem` through the facade, `ItemCustody` arriving via
  `#[require]`, no `SimId` — and the census failed:

```text
  of 28 rollback-anchored entities:
    1 carry NO SimId (rewind anonymously): [
      "death-drop gauntlet (no SimId, as the drop road spawns it)",
  ]
    0 SimIds are carried by more than one entity: []
```

  ⇒ **The assertion is sensitive; the population was blind.** 27 entities before,
  28 after, and the one added is the one it catches. So the guard was green
  because the defect sat outside its corpus, not because the invariant held —
  measured, no longer predicted.
  ✔ **CLOSED. `SimId::death_drop` landed (`4a283375b`,
  `crates/ambition_platformer2d_shared_tangle/src/sim_id.rs:93`) and the census
  population now walks the drop class permanently.** The committed arm spawns a
  `GroundItem` carrying the identity the FIXED road mints —
  `death_drop(&subject, "weapon")`, i.e. `{parent}/drop/weapon` — so the fixture
  and the road agree on the shape rather than the fixture inventing one, and the
  census PASSES.
  ⭐ **The polarity flipped between the falsifier and the test, and that is the
  point.** The red above was an anonymous `GroundItem`, proving the guard was
  sensitive; a permanently-red arm is not coverage. What landed is the covered
  case: the class is in the population, and it reddens again only if the mint
  regresses. Poison-verified by removing the identity — 1 entity carries no
  `SimId` and the census names it.
  ⚠ The drop's parent is the SUBJECT, not another minted id. `death_drop`
  derives from the dying body rather than taking a sequence number, so borrowing
  one of the fixture's four pre-allocated ids both exhausted the supply (it did:
  `four ids`) and modelled the road wrongly.
  ⓘ It does NOT call `drop_held_weapon` — `damage_drops` is a private module
  (`features/ecs/mod.rs:61`) and unreachable from an integration test. It
  reproduces the SHAPE the drop road produces, which is the right question for a
  census: does the guard notice the class, not does one caller emit it.
  ⛔⛔ **RETRACTED — "a harness-staged boss gets a profile-less config" is NOT
  established and was my error.** I traced `BossConfig.behavior` correctly to
  `for_authored_boss(catalog, canonical_id)`
  (`crates/ambition_boss_encounter/src/clusters.rs:280`) and `canonical_id` to
  `canonical_boss_id_from(&name, &brain)` (`behavior.rs:174`) — then applied it
  to the wrong string. `gauntlet_boss` is the fixture's RUNTIME entity id, which
  that function never reads; it resolves `PhaseScript { script_id }` to the
  script id, and the fixture passes `mockingbird`, which IS an authored profile.
  ⇒ The profile resolved. ToothbrushAmbition's log proves it independently: the
  boss ran the mockingbird phase script (`None -> Intro`, `Some(Intro) ->
  Phase1`), which a profile-less config could not do. ⚠ **Reading a step right
  and feeding it the wrong input is not caught by re-reading the step.**

  ⛔ **THE REAL COVERAGE FINDING, from their run:** every boss drop is spawned
  inside `apply_boss_hit`'s `killed` branch, and `apply_boss_hit` has exactly one
  call site — `apply_feature_hit_events`
  (`crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/mod.rs:819`).
  Writing HP to zero and calling `phase.kill()` never enters it. ⇒
  **`force_kill_boss` cannot produce a single boss drop**, and its reward-chest
  assertion passes because the chest arrives by a different road.
  ✔ **I confirmed the other road independently:** `sync_boss_reward_chests_ecs`
  (`crates/ambition_boss_encounter/src/rewards.rs:24`) is *"idempotently ensure
  cleared boss encounters have ECS reward chests"* — it is driven by the SAVE's
  cleared record and the encounter registry, not by the kill. So the chest
  appears for a boss killed any way at all, including one whose HP was written
  to zero.
  ⛔ **Which means `defeated_boss_is_recorded_cleared_drops_reward_and_clears_music`
  (`game/ambition_app/tests/boss_lifecycle.rs:177`) covers half of what its name
  says.** *"A defeated boss with a DropChest reward must drop exactly one chest"*
  is true and passes — through the save road. The rewards that need the actual
  kill road (the signature gauntlet, the ability pickup, the dropped weapon, the
  coin bag) are spawned in `apply_boss_hit`'s `killed` branch, which
  `force_kill_boss` never enters. ⇒ **The fighter's boss rewards have no test
  that exercises the road they are spawned on**, and the file that looks like it
  does is measuring the save instead.
  ⭐ **The mapping IS guarded and the ROAD is not.**
  `boss_signature_gauntlets_map_to_real_wielded_held_items`
  (`crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/tests.rs:989`)
  pins every RON gauntlet id against its ability const. What nothing guards is
  that a HARNESS-STAGED boss gets a profile-less config silently — and every
  boss test in the repo goes through `spawn_boss_at`. ⇒ A guarded data table
  plus a spawn road that never reads it: the same shape as the census above,
  where the assertion is right and the population never includes the case.
  ✔ **CLOSED, and it is policy rather than a defect.** A phase-scripted boss
  survived a single broadcast `HitEvent` of 9,999 damage (measured 2026-09-03,
  ToothbrushAmbition) — because it was in **Intro**, and
  `BossEncounterPhase::boss_invulnerable()`
  (`crates/ambition_characters/src/brain/boss_pattern/mod.rs:1206`) is true for
  `Dormant | Intro | Transition | Death`. `boss_hit.rs:39` returns early on
  `invulnerable || amount <= 0`, and the file's own header says so at :24:
  *"Boss phase policy rejects hits while invulnerable."* A second gate,
  `transition_lock > 0.0`, sits beside it
  (`crates/ambition_characters/src/boss_encounter.rs:254`).
  ⇒ **A boss is killable only in `Phase1`, `Phase2`, `Enrage` or `Stagger`.** Any
  test that wants the real kill road must STEP until the phase is attacking
  before it hits — hitting on frame 1 hits an invulnerable boss and reads as
  "9,999 damage did nothing".

  ⭐ **AND IT IS THE FIGHTER'S BEST REWARD.** `damage/boss_hit.rs:305` drops the
  boss's signature gauntlet through this exact function — the comment above it
  says *"the player literally wields the boss's move"* — and
  `damage/actor_hit.rs:661` is a second caller, so ordinary actor deaths use the
  same identity-less road. ⇒ Every gauntlet ALSO exists as an authored ground
  item in the sandbox world, which does carry an identity, so the same object is
  acquirable two ways with and without a `SimId`.

- ✔ **THE FEATURE UNION IS CLEAN: 7,123 PASSED, 0 REAL FAILURES.** ⭐⭐ Measured
  2026-09-04 on `a41f3d920`'s parent — the run reported ONE failure and it was
  `rollback_coverage::every_mutable_ambition_resource_in_the_shipped_composition_is_accounted`,
  which the run predates the fix for: `EngineRollbackStateDeclared` entered the
  shipped composition and had to be accounted, which is that guard working
  exactly as designed. Re-run alone after the waiver landed: **18 passed / 0
  failed.**
  ⭐ **Both residuals this row chased all day PASSED**, and each by a fix, not by
  luck:
  * `the_stage_kills::every_live_fighter_stays_inside_the_frame ... ok` — the
    camera `Option` (`92f2f597b`). The mechanism argument held: under the union a
    snapshot existed at `t3` and was `Default`; it now reports unframed and the
    check skips the tick.
  * `composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps ... ok`
    — no fix of its own; it has now fired once in five union runs and is the one
    thing here still unexplained.
  ⇒ **The day's series: 7,072/40 → 7,112/3 → 7,115/2 → 7,123/1(0 real).** Every
  drop is a named cause, not a re-run: `draw_sprite_effects` (40), the
  workspace-policy allowlist, the camera default, and a composition marker that
  owed an entry.
  ⚠ **The boss-omit probe stays open and the arm to run is still named**: the
  whole `app_it` binary under union features, which is the level that decided the
  framing one. It has never reproduced below a full workspace run.

  ⛔ **HISTORICAL — the reading this row carried while the 40 were live:**
  *THE FEATURE UNION: 7,072 PASSED / 40 FAILED, AND 40 OF 40 ARE ONE NEW CAUSE.* ⭐⭐ **RE-RUN 2026-09-04 late on `06c7ae470` with the exact command
  `run_tests.py --list --run-everything-you-probably-dont-need-this` prints
  (82-entry union, `--no-fail-fast`, `cargo exit: 101`). Every one of the 40
  failing blocks names `ambition_sprite_fx::draw_sprite_effects`** —
  *"Parameter `ResMut<Assets<Mesh>>` failed validation: Resource does not
  exist"*. ⛔ It is the SAME DEFECT as the 37-failure `sync_portal_view_cones`
  class, in a crate that entered the workspace five days later, and it was fixed
  the same way (`52666d1c7`): per-resource `run_if(resource_exists::<..>)` on
  `Assets<Mesh>`, `Assets<TextureAtlasLayout>` and `Assets<Image>`, guarded by
  `the_plugin_steps_in_a_composition_with_no_render_stack_and_still_tints` and
  poison-verified against the production text.
  ⭐ **The plugin's existing `EmbeddedAssetRegistry` check was not a substitute,
  and that is the lesson worth keeping:** it answers *"is there an AssetPlugin"*
  and the demos HAVE one — what they lack is a render stack.
  ⚠ **The two load-signature residuals below did NOT reproduce**, so the row's
  "passes at every isolation level short of full parallelism" reading is
  unconfirmed on this run rather than contradicted: 40 of 40 had one cause, and a
  binary that aborts on frame one never reaches a contention-sensitive test.
  ⭐⭐ **THE CONFIRMING RUN IS 7,112 PASSED / 3 FAILED — 40 → 3, and ZERO
  `draw_sprite_effects` panics.** Re-run after `52666d1c7` with the same command.
  The three are three DIFFERENT things, which is the point of quoting them
  separately:
  1. ⛔ **`engine_policies` — a REAL, DETERMINISTIC failure that is not
     union-specific at all.** `ambition_portal2d_presentation/Cargo.toml`
     depends on `ambition_sprite_fx`, which was not in that crate's
     `dependency-allowlist`
     (`tests/ambition_workspace_policy/policies/engine.toml`). ⚠ **It fails at
     DEFAULT features too — the workspace-policy gate had been red on main since
     `cf3ee3953`**, and the union is simply where somebody finally read the
     output. Fixed by adding the crate to the allowlist: `ambition_sprite_fx`
     depends on `bevy` and nothing else — no `ambition_*` edge at all — so it
     SATISFIES the policy's stated rationale ("must stay host-free") rather than
     being excused from it, and the edge is the consolidation that crate exists
     for (`clip_material.rs` re-exports `SpriteFrameBasis`, which moved down out
     of the portal crate). Poison-verified by removing the line.
     ⇒ **An allowlist goes stale the moment a shared floor crate is extracted
     beneath it, and nothing about extracting one prompts you to look there.**
  2. `the_stage_kills::every_live_fighter_stays_inside_the_frame` — the known
     smash framing residual, reproduced. Fighter side.
  3. `composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`
     — **a THIRD member of the residual class, and a test written the same day,
     which changes what the class can be blamed on.** It **PASSED in the run two
     hours earlier** on the same command.
  ⇒ Eliminated the same way as the other two: with the exact union feature set,
  `cargo test --workspace <union> --test app_it <that test>` → **1 passed**, and
  the whole `composes_through_the_sdk` module → **6 passed, three times running,
  while a second union was loading the machine.**
  ⭐⭐ **NOTE WHAT THAT ARM RULES OUT, because it is the hypothesis the row would
  otherwise reach for next: `--workspace` means feature unification applied, so
  that run HAD the union feature set with only one binary executing. ⇒ The
  feature set alone does not reproduce it.** What is left is "the rest of the
  workspace present" — other binaries running — which is the load story the
  paragraph below argues against. Both halves cannot be true, and neither has
  been watched.
  ⚠ `-p <crate> --all-features` is NOT this arm and never was: it is a different
  feature resolution, which is the distinction the smash row below already
  records.
  ⚠ **And a run with `bevy_ecs/debug` cannot separate the two either** — it
  changes the feature set as well. That run answers WHICH system, not why, and
  quoting it as a mechanism would be the same error as quoting the elimination as
  one.
  ⛔⛔ **AND IT WEAKENS THE "LOAD OR CONTENTION" READING RATHER THAN CONFIRMING
  IT.** That probe has no timing, no asset and no wall-clock dependency: it builds
  the engine group twice — once whole, once `.disable::<BossEncounterSimulationPlugin>()`
  — and steps 8 frames. There is nothing in it for CPU pressure to perturb. And
  the failure is a PANIC INSIDE A SYSTEM (`run_fixed_main` → `run_main`), which
  contention does not produce; contention makes an assertion late, not a
  parameter absent.
  ⇒ **So the honest status is three tests that fail only in a full workspace run,
  pass at every isolation level, and share no subject matter — and NOBODY HAS YET
  WATCHED ONE FAIL UNDER A CONTROLLED REPRODUCTION.** That is what the row said
  with two cases and it is still what it says with three; the third just removes
  "these are old flaky demo tests" as an explanation.
  ⛔⛔ **RUN WITH `bevy_ecs/debug` ON, AND THE ANSWER IS THAT THESE ARE NOT ONE
  CLASS — I had been treating them as one and that was wrong.** 2026-09-04,
  `--workspace --no-fail-fast <union>,bevy_ecs/debug`: **7,115 passed, 2 failed,
  and ZERO "Encountered an error in system" anywhere in the log.** No
  parameter-validation panic fired at all.
  * `the_stage_kills::every_live_fighter_stays_inside_the_frame` reproduced —
    **as a plain ASSERTION, not a panic**: *"a live fighter was drawn OUTSIDE the
    frame on 2 body-frames, worst 132 units past the edge … t3 seat 1 at
    (416,204) is 132 units outside a 568x320 frame centred (0,0)"*. ⇒ It never
    was a missing-parameter failure, so `bevy_ecs/debug` had nothing to name for
    it.
    ⚠⚠ **THE "RED HERRING" VERDICT BELOW IS ITSELF RETRACTED — 2026-09-04, by a
    probe rather than by more reading, and the lead is back in scope.**
    `probe_where_bodies_are_before_the_match_settles` walks every
    `BodyKinematics` for the first twelve ticks: **zero bodies for three ticks,
    then exactly TWO, both already at their spawns — nothing at `(0,0)` at any
    tick.** ⇒ So `follow_world` reports `(0,0)` while nothing in the world is
    there. That is not a camera faithfully framing an unplaced body; it is a
    follow point corresponding to nothing, which puts *"callers must not invent a
    world-origin fallback"* back on the table.
    ⭐ **The lesson is the one this row keeps teaching from both directions:
    reasoning from a branch you have READ to a world state you have not MEASURED
    is a deduction chain with an unchecked premise at the end.** "The
    early-return arm cannot be the source, therefore the other arm, therefore a
    body must be there" — three deductions, one unchecked. The probe cost four
    minutes and was available throughout.
    ⚠ The probe ran at DEFAULT features, where the test is green; only a
    union-features probe can rule out a body the union itself adds.
    ✔✔ **SOLVED 2026-09-04, and the answer was a number neither of us looked at:
    THE FAILING FRAME IS `ResolvedCameraSnapshot::default()`, VERBATIM.**
    `local_view_facts()` puts the default on every view at spawn so *"a reader
    must never see a frame where the view exists and its state does not"*; the
    resolver honours *"callers must not invent a world-origin fallback"* by
    returning without writing when the cast is unresolvable; and
    `CameraSnapshot2d::default()` is `center_world: ZERO` with
    `default_base_view()`, whose own comment says *"the default moved to `Duel`
    (568x320) on 2026-09-03"* — the exact dimensions the failure reports.
    ⛔⛔ **So two individually CORRECT decisions composed into the behaviour the
    contract forbids. Nobody wrote the fallback; it fell out of `Default`.** And
    it retro-explains the 800x450 → 568x320 shift both of us had been reading as
    instability: that was the DEFAULT changing.
    ✔ **Fixed at `92f2f597b`: `ResolvedCameraSnapshot` is
    `Option<ResolvedCameraFrame>`**, so an unframed view says so and the compiler
    asks every reader. The two frame checks use `and_then` and SKIP an unframed
    tick, which is what their `continue` always meant.
    ⚠ **NOT yet confirmed against the union**, and the distinction is the one
    this row keeps having to make: at default features the test already passed
    (it skipped `t3` because no snapshot existed) and it passes now for a
    DIFFERENT reason, so that run proves nothing about the union. A union run is
    in flight.
    ⚠ And the narrowed question given to Jon ("may a match present a tick in
    which the followed body has not been placed?") is withdrawn with it — it
    assumed a body that does not exist. What survives is that this is not a
    TOLERANCE question: 132 units with BOTH fighters outside is not a camera
    lagging a launch.

    ⓘ **The superseded reading, kept because the retraction is only legible
    beside it:** the contract was thought to be honoured, thus —
    `camera_snapshot.rs` says *"Empty or unresolvable casts return `None`;
    callers must not invent a world-origin fallback"*, which matched the symptom
    exactly, in the right file, three lines away. It was not the branch that ran:
    the unresolvable-cast arm `return`s without publishing, and the origin centre
    came from the arm ABOVE it, which follows a real body. ⇒ **The camera was
    faithfully framing a body that WAS at the origin**, and the defect is
    upstream in PLACEMENT — at `t3` the followed body exists and has not been
    moved to its spawn, which the next tick fixes.
    ⭐ **What settled it in one run was making the test print
    `ResolvedCameraSnapshot::follow_world`, a field that already existed:
    *"following (0,0)"*.** ⇒ Reading a contract and reading the branch that
    actually ran are different acts, and a comment naming your symptom is a
    hypothesis whose being in the right file makes it more tempting rather than
    more likely.
  * The boss-omit probe did NOT reproduce — it has now fired once in three union
    runs.
    ⛔⛔ **AND COMPARING METHODS WITH YARDRAT EXPOSED A GAP IN MY OWN
    ELIMINATION.** They isolated theirs by running the WHOLE TEST BINARY under
    the union feature set (`cargo test --workspace <union> --test smash_it`) and
    it REPRODUCED — so for the framing test the feature set alone is sufficient
    and no load is needed. ⇒ **That is the arm I never ran for mine.** What I
    ran was the single test under union features (passed), the
    `composes_through_the_sdk` MODULE under union features (passed, three
    times), and the whole `app_it` binary at DEFAULT features (passed, twice,
    556 green). **The whole `app_it` binary under UNION features is missing**,
    and it is the level that decided theirs.
    ⇒ So the two are not the same shape after all, and mine is not yet
    isolated: it is "not the single test, not the module, not the binary at
    default features", with the one arm between those and a full workspace run
    still unrun. ⚠ It costs a full rebuild (the union's feature resolution
    differs from default), against a failure that has fired once in three — so
    it is named here rather than run, and whoever has a union build warm should
    take it.
  * The second failure was a NEW test of mine failing its own anti-vacuity floor
    — see the receipt below; not a member of anything.
  ⇒ **So the "three tests that share a signature" reading is retracted.** One was
  a parameter panic and is fixed (sprite_fx); one is a parameter panic that has
  fired once in three runs (boss-omit); one is an assertion about a camera
  centre and always was. Lumping them made each look like evidence for the
  others, which is how a load story survived three investigations without a
  mechanism.

  ⓘ **The superseded next action, kept because the reasoning was sound and the
  answer was still worth buying:** Every one of the three dies with `Parameter <Enable the debug feature to
  see the name>` — the failure names neither the system nor the parameter, which
  is why three separate investigations each ended in elimination. With the
  feature on, the next occurrence names itself, and
  [`engine/headless-verification.md`](engine/headless-verification.md) already
  prescribes exactly this. ⚠ It is one feature added to an hour-long run, not an
  investigation.

  ⇒ **The number to beat next run is 3 → 1**: the policy failure is fixed and
  the boss-omit probe is unexplained, so a clean run leaves only the smash
  framing residual. ⚠ Three files in two other crates were edited while this run's
  TESTS were executing (after its compile phase); cargo does not rebuild mid-run
  and none of those edits adds a system, but the honest status of this figure is
  corroborated-not-isolated, and the unit test is what actually proves the
  defect.

  ⛔ **HISTORICAL — the previous reading, kept because its diagnosis is what
  named the class the new failure belongs to.** *ITS DOMINANT CAUSE IS FIXED, AND
  ITS NUMBER IS UNKNOWN.* ⭐ **Re-measured 2026-09-04: the 37-failure cause below is GUARDED
  at `8bac49a59`** — `sync_portal_view_cones` carries
  `.run_if(resource_exists::<Assets<Image>>)`, `<Assets<Mesh>>` and
  `<Assets<ColorMaterial>>`, and `debug_portal_view_zones` carries the
  `GizmoConfigStore` guard that the same landing found hiding behind it
  (`crates/ambition_portal2d_presentation/src/plugin.rs:151-165`). So the row's
  headline number describes a tree that no longer exists, and quoting 48 or 37
  today would be quoting the diagnosis rather than the state.
  ⭐⭐ **AND THE RUN LANDED: 7,060 PASSED, 4 FAILED — not 48.** Measured
  2026-09-04 on `518e7cd33` with the exact command `run_tests.py --list
  --run-everything-you-probably-dont-need-this` prints (82-entry feature union,
  `--no-fail-fast`, `cargo exit: 101`). ⚠ The raw run reported FIVE; the fifth
  was a test of mine that was red in the tree at the time and is green now
  (`a_boss_gauntlet_banked_at_a_checkpoint_returns_to_the_hand_that_banked_it`),
  so 4 is the honest figure and the fifth is named rather than quietly dropped.
  The 37-failure class is GONE — not one `ConeRigAssets` panic in the log.
  The four, with what each says:
  * ✔ **BOTH `ambition_demo_sanic_app` `ov1_draws_the_world` failures — FIXED
    `26ec20772`, and it was neither of the two things this row guessed.** Not
    "fails by construction under all-features" and not a composition fault: the
    16 nodes were all named `Declared HUD ...` and belonged to `sanic_rings` and
    `sanic_results` — the demo's OWN declaration, which the doctrine the test
    quotes explicitly permits. The filter asked `Without<DeclaredHudRoot>` while
    the marker sits on the panel and the portrait/stock-row/pips/count are its
    CHILDREN, so ownership is the SUBTREE. ⭐ Mary-O's copy of the same test had
    been corrected with this exact reasoning on 2026-09-03 and the sanic copy was
    never touched: two copies of a guard drift, and the one that drifts is the
    one nobody ran. ⚠ Naming the nodes in the failure message is what settled it
    in one run; a bare count cannot tell a HUD from a pause menu.
  * `ambition_demo_mary_o_app` `ov1_draws_the_world::a_vfx_message_this_demo_writes_is_drawn_by_this_demo`
    — ⭐ **DOES NOT REPRODUCE OUTSIDE A FULLY PARALLEL WORKSPACE RUN.** Measured
    2026-09-04, three arms, each redirected to a file: this crate's own feature
    set, test alone → **1 passed**; the workspace union, same test alone → **1
    passed**; the workspace union, the whole `mary_o_it` binary → **60 passed, 0
    failed, 18.4s**. The only remaining difference from the run that failed it is
    that `cargo test --workspace` runs test BINARIES concurrently.
    ⇒ **So the union's residue is two failures that pass at every feature set and
    every isolation level short of full parallelism** — this one and the smash
    framing one, which yardrat eliminated the same way. That is a load or
    contention signature, not a composition defect, and it should be chased as
    one: shared filesystem/asset state, adapter contention, or a settle loop that
    is frame-counted under CPU pressure.
    ⚠ Stated as measured-by-elimination, not as a proven mechanism. Nobody has
    yet watched either test fail under a controlled reproduction.
    The assertion now reports whether `HostVfxPresentationPlugin`'s
    `session_world_exists` gate had a session at all, so the next failure says
    whether the subscriber was absent or installed-and-skipped.
  * `ambition_demo_smash_app` `the_stage_kills::every_live_fighter_stays_inside_the_frame`
    — *"t3 seat 0 at (224,204) is 44 units outside a 568x320 frame centred
    (0,0)"*. ⛔ **I FIRST WROTE THAT THIS WOULD FAIL IN AN ORDINARY SMASH RUN AT
    THE DEFAULT PRESET. RETRACTED — yardrat ran it three ways and it is green
    every time:** default features alone (3.7s), `--all-features` alone (22.2s),
    and the whole `smash_it` binary under `--all-features` (40 passed, 139.3s,
    which also rules out test interaction). I had the arithmetic — 568x320 IS the
    new `Duel` default (`dc7d5c953`) and the spawns ARE at those coordinates —
    and inferred the consequence instead of running it.
    ⇒ **What survives is narrower: the failure needs the WORKSPACE-WIDE union**,
    which is not `-p <crate> --all-features`; unifying across the workspace turns
    on features in this crate's dependency graph that the crate never enables.
    ⭐ **And the lead is the reported CENTRE, not the frame size.**
    `camera_snapshot.rs:908` says an empty or unresolvable cast returns `None` and
    *"callers must not invent a world-origin fallback"* — so a snapshot centred at
    (0,0) with live seats in the world means something resolved a cast to nothing.
    Fighter side.
    ⛔⛔ **AND THE FOURTH ARM — the real workspace union — CANNOT RUN THIS TEST ON
    THIS MACHINE AT ALL, which is why the elimination above stops where it does.**
    `cargo test --workspace --all-features --test smash_it` builds and then dies
    before a single test: *"Tracy Profiler initialization failure: CPU doesn't
    support invariant TSC."* The union turns Tracy on, Tracy aborts the binary at
    startup, and the run reports `error: test failed` with **no test line and no
    `test result` line anywhere in the log**. ⇒ So the smash framing failure was
    observed on a machine whose CPU has invariant TSC and cannot currently be
    observed on one that does not; `TRACY_NO_INVARIANT_CHECK=1` is the documented
    bypass and has not yet been used here.
    ⚠ **Worth more than this row: a feature-union run is not portable.** Jon's
    standing ask is that a fresh clone reach a runnable game, and on this class of
    CPU the union suite cannot execute its binaries at all — the failure is a
    profiler precondition, not a test, and it presents as an opaque
    `test failed` with an empty result set.
    ⚠ I also lost the message once by filtering my own background run through
    `grep -E "^test |test result|..."`: the Tracy line matched none of those
    patterns, so the log looked empty and the exit code was the pipeline's. Same
    shape as the `| tail` trap this row already records, one level down.
  ⛔ **`| tail` VOIDED THE FIRST RUN'S VERDICT.** The first attempt piped an
  hour-long job through `tail -120`, which threw away every per-crate result and
  made the pipeline's exit code 0 while cargo's was 101. Redirect to a file.
  ⇒ Do not re-derive the portal cause; the remainder above is the work.
  The diagnosis below is kept because it is what a reader needs to recognise the
  same shape again — a Bevy 0.19 missing system parameter is a hard failure where
  0.18 skipped — and NOT as a description of HEAD.

  ⛔ **HISTORICAL — the state at `dbfb1a2ca`, 2026-09-03: 48 failures against
  6,968 passes, and 37 of them were ONE system.** Measured 2026-09-03 at `dbfb1a2ca` by running the gate's
  own union job standalone (`cargo test --workspace --no-fail-fast --features
  <the 80-entry union>`, the exact command `run_tests.py --list` prints under
  `--run-everything-you-probably-dont-need-this`). ⚠ **The union is 82 entries
  as of 2026-09-03 late, not 80** — `ambition_abilities/test-support` and the
  encounter-features entry joined with their carves, which is the union doing
  exactly what it is for. ⇒ Do not retype it here: the command PRINTS the list,
  and `--list` costs nothing because it plans without building. That is also the
  cheapest way to re-check this row on a box that cannot run the job — the plan
  is 52 jobs and the union's feature set is one `grep` away, while the failures
  below need a build. Four targets failed:
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
  ✔ **EVERY PREMISE RE-CHECKED 2026-09-03 late WITHOUT A BUILD, and all hold:**
  `dbfb1a2ca` is still an ancestor of HEAD; all four failing targets still exist
  (`smash_it.rs`, `sanic_it`, `mary_o_it`, the sanic lib);
  `crates/ambition_portal2d_presentation/src/view_cones.rs` is still there with
  `ConeRigAssets` at line 745; and it is STILL last touched by `09bb065a9`
  (2026-08-31), so nothing has gone near it since the port — which is the row's
  actual argument, and it got stronger by staying true for another three days.
  ⛔ **WHAT COULD NOT BE RE-CHECKED IS THE ONLY THING THAT NEEDS A BUILD: the
  48/6,968 tally itself.** Two of the row's numbers moved (the union 80 → 82,
  the UI-node assertion 40 → 16) and both are recorded above; a third — the
  failure count — is unknown on the current tree and should be assumed stale
  rather than quoted. ⇒ Re-run the union job on a box with headroom before
  treating 48 or 37 as today's figures.
  ⚠ **THE OTHER ~11 ARE A DIFFERENT CLASS AND MAY NOT BE DEFECTS AT ALL.** They
  are mary_o assertions, and at least one fails BY CONSTRUCTION under an
  all-features build: `the_presentation_plugin_adds_no_hud_and_no_menu` asserts
  0 UI nodes and gets 40, which is what enabling every presentation feature at
  once is *supposed* to do. ⚠ **Re-measured 2026-09-03 late: it reports 16 now,
  not 40 — the COMPOSITION changed under this row and the judgement did not.**
  Five crates left the actor monolith that day, so the union resolves a
  different presentation set; the ruling below (feature-scope these tests or
  exclude them from the union, and do NOT widen the assertion) is unaffected. Whether those tests should be feature-scoped, or the
  union should exclude them, is a judgement for whoever owns the demos doctrine
  — do not "fix" them by widening the assertion.
  ⭐ **AND I CHECKED WHETHER THAT RULING ALREADY EXISTS, so nobody repeats the
  grep: it does not.** `docs/planning/demos/` and `engine/` state the doctrine
  the test asserts, and nothing rules on what a demo should assert under an
  ALL-FEATURES build. ⚠ That check is worth doing before filing anything as "a
  judgement call" — the `ConeRigAssets` group above was filed that way for a day
  and `engine/headless-verification.md` had already ruled on it, with a named
  pattern to copy.
  ⚠ The `painted_blocks` pair is a THIRD cause, read far enough to aim the next
  person: the helper looks for an entity matching `(&BlockVisual, &Sprite)` whose
  `geo_id` is the placement's, and panics *"no block visual is drawing GeoId …"*
  when the query finds NONE. So the failure is not "wrong art" but "no block
  visual entity at all" — including in
  `a_painted_block_nobody_dresses_keeps_its_flat_quad`, where the undressed case
  is the subject. ⇒ First question for whoever picks it up: under the union, does
  some other presentation feature take ownership of block drawing, or does the
  room never reach the state that spawns them? Neither is answered here; both are
  a build away. ⊙ **NARROWED 2026-09-03 by two builds, and TWO HYPOTHESES ARE
  DEAD.** (a) It is NOT the ConeRigAssets group — after the three guards no
  missing-parameter panic remains in the union and these three still fail
  identically. (b) It is NOT a settle problem either, which is the one I expected:
  `cavern()` runs a FIXED 90 updates and the union log shows
  `room-loaded mary_o_1_2` only at frame 625, so "the helper photographs too
  early" was the obvious reading — and it is wrong. Run at
  `--features capture,input,visible` the room loads at **frame 1** and all four
  tests still fail with the same message. ⇒ Whatever it is, it is not the room
  being late and not a system dying before the blocks are made.
  ✔✔ **MEASURED 2026-09-03 AND THE `painted_blocks` GROUP IS GONE — 4/4 PASS.**
  `cargo test -p ambition_demo_mary_o_app --features capture --test mary_o_it`
  reads **57 passed / 3 failed / 3 ignored**, against the 5 passed / 6 FAILED
  this row recorded at `f21153b7b`, and **not one of the three survivors is a
  painted_blocks test**:

  ```text
  ov1_draws_the_world::a_vfx_message_this_demo_writes_is_drawn_by_this_demo
  ov1_draws_the_world::the_presentation_plugin_adds_no_hud_and_no_menu
  ov1_draws_the_world::visible_mary_o_presentation_retires_and_relaunches_with_the_session
  ```

  Also 4/4 at `--features visible` and at `--features capture,visible`. ⚠ 404
  commits landed between that measurement and this one and I did not bisect
  which fixed it — the finding is that the group no longer reproduces, not what
  cured it.
  ⛔⛔ **AND I RETRACT THE DIAGNOSIS I PUBLISHED AN HOUR EARLIER IN THIS ROW.** I
  read `spawn_room_visuals`'s session caller, found a `theme_loaded` gate that
  returns before the block loop, matched it to a comment warning that the room's
  visuals must not be *"held hostage to a backdrop that is never coming"*, and
  wrote it up as the mechanism. **It explains a failure that does not occur.**
  The reading may still describe a real latent hazard — a theme whose art never
  arrives is indistinguishable there from one still loading — but it is not the
  cause of these tests, and I labelled it a source reading precisely because
  this was possible. ⇒ **A coherent mechanism that matches a comment's own
  warning is still a hypothesis; one test run outranks it.**
  ⚠ **THE TWO SURVIVING CLASSES ARE ALREADY CHARACTERISED ABOVE** and neither is
  a mary_o defect: the vfx one panics *"`fx::vfx_spawn_messages` is not scheduled
  here, which is the whole of the coin-pop report and not a Mary-O bug"*, and
  `the_presentation_plugin_adds_no_hud_and_no_menu` fails BY CONSTRUCTION under
  an all-presentation build (0 expected, 40 actual). ▢ The genuinely new one is
  `visible_mary_o_presentation_retires_and_relaunches_with_the_session`, which
  this row has never named.

  ⊙ **The superseded reading, kept because the retraction is the lesson:** a
  parallax-art gate withholds the whole room. Not "another feature owns block drawing" and not "the room never
  reaches the state" — the blocks are built by a path that refuses to run.

  `spawn_room_visuals` (which loops `world.blocks` and is the ONLY caller of
  `spawn_block`) has three callers. The session one, in
  `ambition_render/src/platformer_presentation.rs`, is gated:

  ```rust
  if wants_parallax {
      let theme_loaded = assets…any(|layer| a.parallax_layers.get(theme, *layer).is_some());
      if !theme_loaded {
          // Leave `presented` unset so the next frame retries
          return;                      // ← every block goes with it
      }
  }
  ```

  ⛔ **AND THE COMMENT DIRECTLY ABOVE IT DESCRIBES THIS EXACT FAILURE AS THE
  THING THAT MUST NOT HAPPEN:** *"A tier that disables it (or a room whose theme
  legitimately has no art) must present normally, or the whole room's static
  visuals would be held hostage to a backdrop that is never coming."* The guard
  the comment describes checks `wants_parallax` — whether the BUDGET wants
  parallax — and the second condition is `theme_loaded`, which asks whether the
  art has ARRIVED YET. A theme whose art never arrives in that composition is
  indistinguishable from one that is still loading, and the retry never ends.
  ⚠ Themes lazy-load: `[game_assets] loaded 4/4 … for 'hub' (other themes
  lazy-load on room transition)`. A test that builds a room WITHOUT a transition
  is exactly the case where the layers may never load.
  ⭐ It also explains the feature-dependence the row could not place: which
  presentation features are on decides whether `GameAssets` and
  `ResolvedVisualQuality` exist at all, and `wants_parallax` defaults to TRUE
  when the quality resource is absent (`.unwrap_or(true)`) — so a thinner build
  is MORE likely to hit the gate, not less.

  ⛔ **THIS IS A SOURCE READING, NOT A MEASUREMENT — do not close the row on it.**
  ⭐ **But the code ships its own one-run falsification**, which is why it is
  worth writing down: the gate is skipped entirely when the budget disables
  parallax. ⇒ **Run the failing `painted_blocks` tests with a quality budget
  whose `parallax.enabled` is false. If the block visuals appear, the gate is the
  cause; if they still do not, this reading is wrong and the remaining suspect is
  `world.blocks` being empty in that spec** (the loop's other precondition, and
  the only one left).
  ⛔⛔ **THAT BISECTION CANNOT RUN AS WRITTEN, and the row got two manifest facts
  wrong (checked 2026-09-03 against `game/ambition_demo_mary_o_app/Cargo.toml`).**
  (1) `visible` ALREADY INCLUDES `input`
  (`visible = [..., "input", ...]`, line 28), so "run `capture,visible`, then
  again adding `input`" is the SAME RUN TWICE and can distinguish nothing.
  (2) `mary_o_it` does NOT declare `required-features` — the `[[test]]` entry
  (lines 46-48) has none; `required-features = ["capture"]` is on the `[[bin]]`
  `capture_mary_o` (line 56). ⇒ Reading a manifest fact off the neighbouring
  stanza is how a plan gets built on a constraint that is not there.
  ⭐ **AND THE ROW'S OWN LOGIC ALREADY ANSWERS IT.** Because `capture` ⊃
  `visible` ⊃ `input`, the run it reports as already done —
  *"at `--features capture,input,visible` the room loads at frame 1 and all four
  tests still fail"* — IS the minimum set. By the row's stated rule (*"if both
  fail, the cause is in the minimum set and the union is irrelevant to it"*), the
  union is irrelevant: these tests fail whenever they are BUILT, and the union's
  only contribution is building them.
  ⇒ **Which reframes the whole row.** `ov1_draws_the_world` and
  `painted_blocks` are `#![cfg(feature = "visible")]` and the crate's
  `default = []`, so a plain `cargo test -p ambition_demo_mary_o_app` compiles
  them to NOTHING and reports green. They are not union-specific failures; they
  are tests that only exist under `visible`, failing there, invisible everywhere
  else.
  ✔ **CONFIRMED 2026-09-03 BY TWO RUNS, AND THE UNION IS NOT INVOLVED.** Same
  crate, same target, no workspace union, `--test mary_o_it -- ov1_draws_the_world
  painted_blocks`:

```text
--features capture                                          5 passed  6 FAILED
--features capture,ui,bevy_ui_menu,kaleidoscope_menu,        5 passed  6 FAILED
         mobile_touch,dev_tools
```

  **Identical down to the numbers** — `the_presentation_plugin_adds_no_hud_and_no_menu`
  is `left: 40, right: 0` in both, and `painted_blocks` panics on the same
  `GeoId … PlacementId("MaryOBlock-106927")` in both. ⇒ The five extra
  presentation features contribute NOTHING, so "another presentation feature
  owns block drawing under the union" is dead as a hypothesis, and so is the
  union framing: **these six fail whenever they are BUILT.**
  ⇒ **What is left is a plain six-test red in `mary_o_it`, unrelated to
  features**, and it is invisible to `cargo test -p ambition_demo_mary_o_app`
  because the crate's `default = []` compiles both files (`#![cfg(feature =
  "visible")]`) to nothing. ⭐ `python3 scripts/feature_gated_tests.py` already
  reports the population — *"ambition_demo_mary_o_app 48 of 63 run bare
  (visible)"*, 784 tests across 29 crates — so the hiding was instrumented all
  along; nobody had connected the survey to this row.
  ✔ **ANSWERED AND FIXED 2026-09-03 — THE GUARD WAS WRONG, NOT THE ENGINE.**
  Printed the forty instead of counting them: **every one is the demo's OWN
  declared HUD** — five readouts × (panel, portrait, stocks row, four stock pips,
  stock count). `engine_owned_ui_node_count` filtered
  `Without<DeclaredHudRoot>`, which excludes the ROOT and counts its children;
  the nodes beneath carry `DeclaredHudPortrait`, `DeclaredHudStock`,
  `DeclaredHudStockCount` or no marker at all (`hud/declared.rs:240-300`).
  ⛔ **And the doctrine it guards explicitly ALLOWS what it was flagging** — *"A
  demo that wants a HUD declares one — that is what `owns` means in the demos
  doctrine."* ⇒ Ownership is the SUBTREE, so the helper now walks `ChildOf` to a
  declared root rather than checking one marker. ⚠ Walking, not parent-checking:
  the stock pips are grandchildren, so one level would still have counted twenty.
  ⭐ **That single filter turned TWO of the three `ov1` failures green** —
  `the_presentation_plugin_adds_no_hud_and_no_menu` AND
  `visible_mary_o_presentation_retires_and_relaunches_with_the_session`, which
  shares the helper. 6 passed / 1 failed where it was 4 / 3.
  ⇒ **The one that remains names its own cause** and is not a filter artifact:
  *"this composition wrote a `VfxMessage::CoinPop` and drew nothing:
  `fx::vfx_spawn_messages` is not scheduled here, which is the whole of the
  coin-pop report and not a Mary-O bug"* (`game/ambition_demo_mary_o_app/tests/ov1_draws_the_world.rs:364`). A
  missing system registration in this composition, stated by the test itself.
  ⇒ So the mary_o red is **four**, not six: that one plus the three
  `painted_blocks`.
  ⓘ Held out of the tree while `383484`'s `--rust` gate window is open; it is a
  test-only change to `ov1_draws_the_world.rs`. That is the whole remaining question —
  `engine_owned_ui_node_count` filters `Without<DeclaredHudRoot>`, so what to
  learn is which plugin spawns those forty.
  ⊙ **TWO ELIMINATED BY READING, 2026-09-03** (no build; recorded so the next
  person does not repeat them). `drawn_demo` is `build_windowed_demo_app`, which
  adds exactly five things: `install_windowed_foundation` (Bevy's
  `DefaultPlugins` + window), `PlatformerEnginePlugins::fixed_tick()`,
  `PlatformerHostPlugins`, `PlatformerAssetsPlugin::for_experience`, and
  `PlatformerPresentationPlugin`.
  ⛔ **NOT `PlatformerPresentationPlugin`**, which was the obvious suspect and is
  the one the test's message accuses: it installs `VisualQualityPlugin`,
  `spawn_main_camera`, `spawn_initial_room_visuals`, `SessionRoomVisualsPlugin`
  and the two animation plugins — no UI node anywhere
  (`platformer_presentation.rs:88-104`).
  ⛔ **NOT `PlatformerHostPlugins`**: developer hotkeys, `HostCameraPlugin`,
  `HostProjectileVisualsPlugin`, `HostVfxPresentationPlugin`,
  `HostInputBindingsPlugin` (`platformer2d_host/src/lib.rs:25-33`).
  ⚠ **`ambition_render` DOES contain the node spawners** — `hud.rs`,
  `gameplay_surround.rs`, `hud/declared.rs`, `rendering/health.rs`,
  `dialog_ui.rs`, `cutscene/mod.rs` — so the engine crate holds exactly what the
  doctrine says belongs app-side; the open question is which of the remaining
  three installs them here. ⇒ The cheap next step is to print the forty nodes'
  components from the failing assertion rather than to keep reading plugin
  lists. ⚠ I wrote the
  settle fix, ran it, saw it change nothing, and reverted it rather than ship a
  loop whose comment claimed a cause it had not established.
  ⊙ **HALF-ANSWERED: it is NOT the ConeRigAssets group.**
  After the three guards there is no missing-parameter panic left anywhere in
  the union, and these three still fail with the same message — so the blocks
  are absent for a reason of their own, not because a system died before making
  them. That leaves "another presentation feature owns block drawing under the
  union" against "the room never reaches the state that spawns them", and only
  the second is answerable without reading the union's feature set against
  mary_o's composition.
  ✔ **THE 37 ARE FIXED (`8bac49a59`), and my "design choice" framing was wrong.**
  `engine/headless-verification.md` had already ruled on this exact class —
  *"the fix is usually NOT to register the resource. A gizmo or mesh system with
  no render stack should be `run_if(resource_exists::<..>)`-guarded so it
  skips"* — with `avatar::trail.rs` as the named pattern, and that same
  paragraph already names `Assets<Mesh>` as one of three that hid in succession
  on 2026-09-02. All three of `ConeRigAssets`' resources are guarded, not just
  the one that failed first, because that doc records those three surfacing one
  after another.
  ⊙ **MEASURED ACROSS THE UNION, 2026-09-03, and it took THREE rounds because
  each fix exposed the next system in the same chain — which is what
  `headless-verification.md` says happens and what I did not believe until I ran
  it:**

  | round | passed | failed | the layer it exposed |
  |---|---:|---:|---|
  | before | 6,968 | 48 | `ConeRigAssets` (`Assets<Image>/<Mesh>/<ColorMaterial>`) |
  | guard `sync_portal_view_cones` | 6,980 | **49** | `debug_portal_view_zones` on `Res<GizmoConfigStore>` |
  | guard that too | 6,991 | 38 | `attach_hit_flash_overlays` on `ResMut<Assets<Mesh>>` |
  | guard that too | **7,016** | **13** | no system-parameter panic remains |

  ⛔ **NOTE ROUND ONE WENT UP, NOT DOWN.** 48 → 49. A fix that removes 37
  failures and exposes 37 more reads as "no progress" on the count alone, and as
  a regression to anyone watching only the number. The three systems are the
  three that paragraph names — `Assets<TextureAtlasLayout>`, `GizmoConfigStore`,
  `Assets<Mesh>` — in that order, a day later.
  ⇒ **The 13 that remain are the classes below, none of them a missing
  parameter**: 8 in `ov1_draws_the_world` (the doctrine group), 3 in
  `painted_blocks`, and `published_local_sanic_forms_bind_through_game_assets`.
  ✔ **THAT LAST ONE IS FIXED (`3f4dbbcd5`)** — it demanded TWO forms, ran ONE
  `app.update()`, and asserted both `Ready`, while
  `MAX_CHARACTERS_MATERIALIZED_PER_FRAME` is 1. The `None` rather than
  `Some(Failed)` is the whole diagnosis: no load attempted yet, which is what a
  ration looks like from the far side. Replaced with a bounded settle; the same
  command that failed now reads 3 passed.
  ⛔⛔ **A LATER UNION RUN OF MINE IS VOID, AND THE REASON IS THE USEFUL PART
  (2026-09-04, `0275cd1b9`).** It read **7,104 passed / 32 failed** and every one
  of the 32 was my own contamination: recursive greps run beside it exhausted
  file descriptors. ⇒ Do not quote that number; the clean re-run is the verdict.
  ⭐ **What it found is worth more than what it measured.** Thirty of the 32 were
  `ambition_workspace_policy`, and each failed naming a repository fact that is
  FALSE — the loudest being *"no ancestor Cargo.toml declares [workspace]"* of a
  workspace whose root manifest declares it on line one. **Nothing in the output
  said "I could not read a file."** Every rule in that crate reports an ABSENCE,
  so a scanner that treats *unreadable* as *empty* announces exactly its own
  finding whenever the machine, not the code, is what failed. Worst site:
  `migration_matrix.rs` returned an EMPTY set on a read error under the comment
  *"a deleted legacy file has no pending functions"*, so an IO failure read as
  *"the migration is complete"*. Fixed at `0b58767f2` across five sites, poison-
  verified at two; `NotFound` alone is tolerated where deletion is a real answer.
  ⚠ **A non-fatal checker wants the OTHER fix**, which the peer session landed
  the same hour on `check_planning_citations.py` (three `except OSError:
  continue`, 1,537 citations, ~15 runs): COUNT the dropped reads and print the
  count BEFORE the verdict, because it qualifies the verdict. Fatal where the
  path came from your own walk; qualified where tolerance is real.
  ⇒ Recipe: `docs/recipes/checks-that-did-not-run.md`, *"And your OTHER WORK is
  part of that environment"*.
  ✔✔ **RE-CONFIRMED AFTER THE S4 SLICE: 7,139 PASSED, 0 FAILED, `cargo exit: 0`,
  179 test blocks, 42 ignored**, measured 2026-09-04 at `935491c76` — the same
  command, nothing else on the machine, zero `Too many open files`. ⇒ The
  anonymous-anchor fix, the portal `SimId`, the third census and the policy
  scanner repair are all inside this number. Two green unions in a row, at two
  different heads.
  ✔✔ **AND THE FIRST CLEAN RE-RUN: 7,137 PASSED, 0 FAILED, `cargo exit: 0`,
  179 test blocks, 42 ignored.** Measured 2026-09-04 at `5c320ebb5` with the same
  82-entry union the `run_tests.py --list --run-everything-you-probably-dont-need-this`
  command prints, `--no-fail-fast`, nothing else running on the machine, zero
  `Too many open files` in the log. ⭐ **THE FIRST FULLY GREEN UNION THIS ROW HAS
  EVER RECORDED.** The progression is 48 → 49 → 38 → 13 → 12 → 4 → **0**.
  ⇒ It also settles what `awaiting-maintainer-decision.md` #50 could not: that
  entry's `✔ Fixed rather than ruled on` was a MECHANISM ARGUMENT, flagged in
  place as *"do not read `✔ Fixed` as measured"* because the failing run had been
  a union-features run and no union had been taken on the fixed tree. One has
  now, and the framing test is in it. ⛔ The WITHDRAWAL was always independent of
  this — no fighter was ever outside a real frame — so this confirms the fix, not
  the premise.
  ⊙ **CONFIRMED 2026-09-03: the union reads 7,019 passed / 12 failed**, and no
  system-parameter panic remains anywhere in it. The full progression is
  48 → 49 → 38 → 13 → **12**. ⚠ It was stated as an EXPECTATION until the run
  existed, because the previous time I turned a verified single-target fix into
  a union number I was wrong by 38 — in the direction nobody expects. This time
  the expectation held; that is a fact about this fix, not a licence to skip the
  run next time.
  The smash target went from thirty-odd failures to **one**:
  `the_screen_decides` and `the_repertoire_gets_used` are entirely green and
  `the_stage_kills` has a single survivor. ⚠ I wrote "the entire smash target is
  green" when it was not — 30-odd down to 1 is the honest sentence, and the 1 is
  a FIFTH cause.
  ⊙ **THE TWELVE, ENUMERATED 2026-09-03** so the next reader does not re-derive
  them: 8 in `ov1_draws_the_world` (the doctrine group, including
  `the_presentation_plugin_adds_no_hud_and_no_menu` in BOTH the mary_o and sanic
  apps), 3 in `painted_blocks`, and
  `the_stage_kills::every_live_fighter_stays_inside_the_frame` — *"a live
  fighter was drawn OUTSIDE the frame on 1 body-frames, worst 16 units past the
  edge … t3 seat 1 at (416,204) is 16 units outside a 800x450 frame centred
  (0,0)"*. That last is a CAMERA FRAMING assertion, unrelated to the other four
  causes, and 16 units on one body-frame is the kind of margin that may be a
  tuning question rather than a defect. ⇒ **FILED 2026-09-03 as
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) entry 50**,
  because which artefact is wrong — the camera or the assertion's zero tolerance
  — depends on whether a platform fighter's camera may lag a fast body for a
  frame, and that is a feel ruling. ⛔ Do not "fix" it from this row.

  ⚠ **AND THE INFERENCE THIS REPLACES WAS WRONG, kept because it is the lesson.** I first wrote: `cargo test -p
  ambition_demo_sanic_app --lib --features capture,input,visible` now shows ZERO
  `ConeRigAssets` panics where it failed on them before. The full union is a
  ~40-minute rebuild that took the shared volume to 100% last time, so "37" is
  an inference from the class rather than an observed count. ⇒ The next union
  run should read 11-ish, and that target's remaining failure
  (`published_local_sanic_forms_bind_through_game_assets`, an asset
  materialization assertion) is a FOURTH cause, distinct from the doctrine and
  `painted_blocks` groups.
  ⇒ **The original framing, kept because it was the error**: I called this a
  design choice: skip the system when its assets are absent (`If<…>`, or a
  `resource_exists` run condition) versus provision the assets in every
  composition that installs portal presentation. The first says a cone rig with
  nowhere to draw should stand down; the second says the composition is
  incomplete. Bevy's own message suggests the first.
  ⛔ **AND THE REASON NOBODY SAW IT: the union job lives inside `if not only and
  everything`**, so a default gate run never attempts it — the same blindness
  the coverage footer now states as 783 tests across 29 crates (`65f4030b5`).
  ⚠ **Re-measured 2026-09-03 late: 784 across the same 29 crates.** The footer
  already says 784 (`2dc2fcb71`, "one joined today"); this line quotes what it
  said at `65f4030b5` and is left as the quotation it is. ⇒ The figure moves
  whenever anyone adds a feature-gated test, which is the reason
  `test_the_gate_states_how_many_tests_it_skips.py` ratchets the footer rather
  than trusting prose — **read the footer, not this number.**
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

- ⛔ **D-BRAIN-MENU — THE FIGHTER BRAIN SCORES A MENU IT CANNOT ORDER FROM.**
  Found 2026-09-04 by putting two existing instruments side by side; there was no
  guard for it because nothing compares what the brain SELECTED against what the
  body PERFORMED.
  ⭐ **The measurement** (`smash_tool capture-probe --character smash_george_booul
  --ladder <shipped ron>`, 40s, `AMBITION_FIGHTER_TRACE=1`): the brain chose
  `george_booul_dash_attack` **0 times**; the bodies started it **43 of 59**. It
  chose `jab` **40 times**; one jab started. It chose `tilt_up` 8 times; none
  started. ⇒ Over 120s, **16 of George's 28 authored moves never start once** —
  all three smashes, all three tilts, three of five aerials — and the dash attack
  is **81%** of starts on the shipped ladder (84% on the floor, so this is NOT a
  floor artifact).
  ⇒ **THE CHAIN, read rather than assumed:** the kit builder
  (`crates/ambition_platformer2d_actor_monolith/src/features/ecs/actors/update.rs:1747`)
  calls `move_for_directional_verb(verb, direction, grounded)` — **no `running`**
  — so the brain scores the STANDING set and never sees `attack_dash` as a
  candidate. It then emits a BUTTON (`AttackBinding`), not a move. The body
  re-resolves with `move_for_flat_verb(base, grounded, running)`, which tries
  `{base}_dash` FIRST when running. George binds `attack_dash`. ⇒ Every attack
  pressed while running becomes the dash attack, whatever was scored.
  ⛔ **The body is not misbehaving** — that IS what a dash attack is. The defect is
  an option set assembled under an assumption the emission violates.
  ⭐⭐ **AND THE SAME PROBLEM IS ALREADY SOLVED ONE VERB OVER.** The burst press
  (dodge/dash are one input, resolved by body state) was fixed by making
  perception carry the RESOLVED answer — `SelfView::burst`, documented as
  *"`resolve_burst_maneuver` is the one rule, and this field is its answer. The
  brain is handed a fact."* The attack press has no equivalent. ⇒ **Two fixes, both
  engineering:** build the kit in the stance the press will be resolved in, or hand
  the brain the resolved move as a fact. The second matches the existing precedent.
  ⚠ **DO NOT LAND EITHER UNMEASURED.** Both change every fighter in every game that
  uses this brain, and while running the flat-verb menu may collapse to one option
  — whether the brain should then stop running to reach its other moves is
  emergent behaviour nobody has measured. ⇒ The rig can measure it:
  `capture-probe` gives the move census and `ladder-rig --paired` gives the
  outcome, both now taking `--ladder` and `--character`.

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
  without the attribute. ⛔ NEXT CANDIDATES — REWRITTEN 2026-09-04, because
  "pick from the inventory again" now has a measured answer instead of a shrug.
  ⭐ **The inventory's primitive table is 14 of 14 measured and the ABSENT column
  is EMPTY**: ten shipped, three partial with the split named, one shipped a
  named fact short. Not one of the twelve previously-unmarked rows turned out to
  be an unbuilt primitive. ⇒ **The next slice off this page is almost never
  "build a primitive" — it is "author a customer for one that already exists"**,
  which is cheaper than the Class column implies and a different kind of work
  than the row's wording invites.
  ✔ **SLICE TAKEN 2026-09-04 on exactly that rule: the drop-through platform.**
  The engine shipped `BlockKind::OneWay`, `resolve_one_way_hit`, a
  `drop_through_timer` and BOTH gestures (down+jump, and the platform fighter's
  own guard+down whose doc reads *"on a surface that can be left downward"*), and
  the smash demo used **zero** of it — a platform fighter with no platforms.
  `smash_platform_stage()` is the customer, `SmashStageChoice` and a select-screen
  stage button make it reachable, and the tier heights are recomputed from the
  engine's own jump arc because the first ones I chose by eye were **scenery**
  (250px against a 148.3px ceiling).
  ⇒ **Remaining customers, COSTED PER ITEM 2026-09-04 — and only two of seven are
  authoring.** ⛔ This row said "the seam is finished, no plumbing" of `P11`'s four
  capture roads; that is true of ONE of them.
  ✔ **Authoring alone:** ~~`P11`'s **command grab**~~ — ✔ **SLICE TAKEN
  2026-09-04**, and it needed no engine work exactly as this row predicted:
  `lunge_grab` is a `special_forward` whose `Active` window sustains
  `smash.capture_attempt`. `author_standing_grab` never asks which verb it is
  attaching to and the captor branch keys off capture STATE, so it pummels and
  throws through the same four verbs. Guard poison-verified five ways. ⚠ **And
  the first attempt at one poison silently matched nothing** — a regex that
  edited no bytes and printed no output, which reads exactly like a passing arm.
  Hence five arms and not three. ⇒ Still open here: `P06`'s **foxtrot /
  dash-dance** (dash-stance moves `move_for_flat_verb` already selects).
  ⭐⭐ **AND THE SLICE FOUND SOMETHING BIGGER THAN ITSELF: two of the demo's three
  characters had no special button at all.** Counted off the contracts —
  `fighter_moveset()`, which both Robots carry, bound **18 verbs to George's 26**.
  Missing: `special`, `special_forward`, `special_up`, `special_down`,
  `special_air_down`, `attack_forward`, `attack_dash`, `taunt`. A press resolved
  to no move, and the catalog DEFAULT is one of the two. The command grab closes
  one of the eight; ⛔ the other seven are DESIGN and are now a question with Jon
  in `awaiting-maintainer-decision.md` (thin stand-ins / finished characters /
  one shared simpler kit) — **do not author them off this row.**
  ⛔ **Needs a seam first:** the **pivot** moves in both rows — `move_for_flat_verb`
  hardcodes one derived stance (`{base}_dash`) and cannot express a pivot;
  `P11`'s **tether** — a grapple mechanic exists and nothing joins it to capture;
  `P11`'s **hit-grab** — nothing raises `CaptureAttemptRequested` from a landed
  hit; `P10`'s **tech result** — presentation only, because the AI half is
  deliberately absent under the no-cheat rule and publishing it would be a cheat;
  and `P14`'s **finish-zoom eligibility** — the camera has the machinery and drives
  it from zones, and no fact says *this blow is the finishing one*.
  ⛔ **DO NOT tune the fighter brain against ladder-rig numbers yet — and the
  reason has been REPLACED, 2026-09-04.** The rig can now measure the shipped
  ladder (`--ladder PATH`) and has: four matched arms, `--paired --seeds 12`,
  one binary. ⛔⛔ **AND THE HOLD-OFF IS NOW THE OPPOSITE OF "WE CANNOT
  TELL" — REWRITTEN AGAIN, LATE 2026-09-04.** The rig's own significance test was
  found to run BACKWARDS (`|median| < 0.5 * (max - min)`; a range only grows with
  n, so more seeds made results LESS significant) and is now an exact sign test.
  ⛔⛔ **AND THE CLOCK WAS WRONG TOO — the biggest of the five.** The shipped match
  is **eight minutes** (`SMASH_TIME_LIMIT_TICKS`); the rig ran **sixty seconds**, so
  no bout could end, stocks tied in every cell, and every verdict this tool has
  ever printed fell through to its damage tiebreak. Fixed: the default now reads
  the demo's own constant. ⇒ **Re-measured at the shipped clock, where every bout
  RESOLVES (`0 : 0` stocks, medians rising 85s → 97s → 104s → 112s up the rungs):
  `3 vs 1` higher ✔, `5 vs 3` **LOWER** ⛔, and `6 vs 5` / `9 vs 6` are within
  spread.** ⚠ So it is **one bad rung, not a broken progression** — the 40-second
  table's "every cell is significant" was an artifact of the short clock and is
  marked superseded in `fighter-brain.md`.
  ✔✔ **AND THE ONE BAD RUNG IS NOW DIAGNOSED AND ITS FIX MEASURED**, so this row
  no longer blocks on a mystery. The cause is `frame_advantage` +
  `expected_payoff` **jointly** — isolated byte-for-byte (those two reproduce the
  whole effect of swapping all four weights; the other two reproduce the control;
  neither of the pair suffices alone), replicated at 28 seeds, and consistent in
  direction across **all nine** scenario fixtures. ⭐ The mechanism agrees and was
  read afterwards: `frame_advantage` is SIGNED, so raising its weight penalises a
  rung's own slow hard-hitting moves, and `expected_payoff` is gated by the
  positive part of it, so it withholds the power bonus from exactly those. Higher
  rungs jab more and smash less.
  ⭐ **Candidates measured at the shipped clock**: halving the rise does *nothing*
  (300% : 362% against the shipped 306% : 360%), and holding the pair flat from
  level 4 up removes every significant inversion, makes `9 vs 6` significant in
  the CORRECT direction, and rises the survival medians monotonically
  (85 → 101 → 116 → 122s) where the shipped ladder's top pair went backwards.
  ⛔ **DO NOT LAND THAT** — it is with Jon in `awaiting-maintainer-decision.md` as
  one option, because holding those two flat is a statement about what a harder
  CPU IS, not a bug fix. ⚠ So the reason not to tune is no longer "the instrument
  cannot see"; it is that the ladder has a known, reproduced, player-facing
  inversion whose cause is a weight VECTOR (no single weight reverts it — all four
  arms failed, only the full swap works) that nobody has ruled on yet.
  ⭐ Two results worth carrying: the L3 rollout is a **lethality** switch and not a
  strength one (the two floor arms are byte-identical below rung 6, where it is
  not armed, and `0:0` against `2:2` above it), and **George is a substantially
  harder fighter than the Robots the rig had always measured**. See
  [`engine/fighter-brain.md`](engine/fighter-brain.md) and the ownership question
  in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). ⛔ DO NOT PICK: D203's hitbox premise (measured and REFUTED by
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

- ✔ **FIVE WORKSPACE DEPENDENCY DECLARATIONS ARE NEVER NAMED IN THEIR CRATE'S
  SOURCE — graph honesty, NOT a footprint win.** (Measured 2026-09-02;
  re-measured 2026-09-04.) ⭐ **Six became five: the PLAIN edge is already gone.**
  `ambition_platformer2d -> ambition_interaction` appears neither in that crate's
  `Cargo.toml` nor anywhere under its `src/`, so the one unconditional edge — the
  only one this row could describe as a plain redundancy — has been removed since
  the row was written. The five optional ones are all still declared and still
  unnamed in `src/`.
  ⚠ **AND THE REMAINING FIVE NEED A CHECK THIS ROW DOES NOT SPECIFY, because
  "unused in `src/`" is not the same claim for an optional dep.** Each is
  activated by a feature — `ambition_characters`' `causal = ["dep:ambition_causal"]`,
  `game/ambition_app`'s `causal = ["ambition_platformer2d/causal",
  "dep:ambition_causal"]` — so removing the `dep:` entry changes what that
  feature turns on, and a feature that becomes empty may still be a marker
  something above forwards. ⇒ Before deleting any of the five, establish that no
  dependent forwards the feature expecting the crate to be linked; the
  feature-union build is where that would surface, and it is an hour to find out
  the expensive way. Every
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

  ✔ **THE PLAIN ONE IS CUT (2026-09-03).** `ambition_platformer2d` no longer
  declares `ambition_interaction`. Verified rather than assumed at each step:
  nothing in the whole crate directory names it (not `src/`, not tests, not
  `build.rs`), so no feature combination can need it; `cargo check -p
  ambition_platformer2d --all-targets` is clean; the workspace no-warnings gate
  is clean; and the capability footprint is UNCHANGED at 47/20, which is the
  row's own claim about redundancy holding up under measurement.
  ✔ **THE FIVE OPTIONAL EDGES ARE STILL THERE AND STILL UNUSED — re-verified
  2026-09-03 late**, on a tree ~90 commits past the original measurement and
  after five carves, which is exactly when a "never used" claim is most likely
  to have quietly stopped being true. Each crate still DECLARES its dependency
  in `Cargo.toml`, and a search of the whole crate directory (`src/`, tests and
  `build.rs`, not just `src/**`) finds zero files naming it, in all five. So
  the row's remaining work is unchanged and its size claim still holds.
  ⚠ The re-check is a source grep, not a build: it can see a name that is never
  written and cannot see one reached through a macro. That is the same instrument
  the original measurement used, so the two are comparable — which is the point
  — but neither is proof that removing the edge compiles. The PLAIN edge below
  was cut only after `cargo check --all-targets`, and these should be too.
  ⭐ **THE SENTINEL'S LOCKFILE CAME WITH IT, and the guard is what said so.**
  `check_absence_contracts.py` runs `cargo tree --locked` in the sentinel's own
  workspace and threw `CalledProcessError … exit status 101` the moment the
  edge left — which is exactly what its docstring promises: *"a dependency
  change that alters the sentinel's lockfile must arrive WITH that lockfile, or
  this check fails loudly instead of silently rewriting it."* Three lockfiles
  each lost one line (`Cargo.lock`, `fixtures/minimal_game`,
  `examples/capability_demo`).
  ⇒ **The five OPTIONAL ones are still open, and TRIAGED 2026-09-03 so the
  ruling is cheap.** All five are still unnamed in their crate's `.rs`
  — ⭐ **re-derived again 2026-09-03 late, 0 source files each**, because a
  triage table is exactly the artefact that goes stale while reading as settled.
  ✔ **And the two rows that said "needs reading before touching" are now read**,
  so every cell in the table is filled and the ruling needs no further digging. ⛔ **None is mechanical**, and the reason is
  not the compiler — it is INTENT. An optional dep is wired into a feature
  definition, so removing it edits a declared seam and every dependent that
  enables that feature:

  | declaration | the feature it backs | who enables it | shape |
  |---|---|---|---|
  | `ambition_characters` → `ambition_causal` | `causal = ["dep:ambition_causal"]` | the monolith's own `causal` | ⚠ a NO-OP feature: it pulls the dep and does nothing else, and its comment says *"Publish this capability's causal facts (brain decisions, for now)"* — a seam declared ahead of its use |
  | `game/ambition_app` → `ambition_causal` | `causal = ["ambition_platformer2d/causal", "dep:ambition_causal"]` | `ambition_app_tools` | ⭐ the feature does MORE than pull the dep, so dropping the `dep:` alone leaves it meaningful — the smallest safe edit of the five |
  | `ambition_sim_view` → `ambition_portal2d` | `portal = ["dep:ambition_portal2d", "…actor_monolith/portal"]` | nothing in a manifest; the gate's feature UNION does | same shape as above |
  | `ambition_platformer2d` → `ambition_sfx_bank` | **`all_capabilities`** (`crates/ambition_platformer2d/Cargo.toml:79`), which is the crate's `default` | everything, via `default` | ⚠ a ROSTER entry, not a wiring: `all_capabilities` lists 20-odd crates the facade can offer and this is one line of it. Removing it narrows what `default` means, so it is the same class of intent decision as the no-op feature |
  | `ambition_touch_input` → `ambition_cutscene` | **`mobile_touch`** (`crates/ambition_touch_input/Cargo.toml:30`) | `ambition_platformer2d/mobile_touch` (`:126`) | ⚠ sits among nine `dep:` lines that ARE used (`actor_monolith`, `sim_view`, `render`, `ui_nav`, `persistence`, `bevy`, `virtual_joystick`); only `ambition_cutscene` is unnamed in the source, so it reads as a wire that was planned and never run |

  ⇒ **A no-op feature that exists to declare a future seam is not debt, and
  removing it would delete the intent.** That is a maintainer's ruling, not a
  cleanup, which is why this row stays ▢ rather than being finished the way the
  PLAIN one was.

  ⛔⛔ **DO NOT REMOVE BLIND — it needs the compiler on each crate's feature
  combinations.** Dropping an optional dep changes feature RESOLUTION, not just
  a line, and only a build says what that does.
  ✔ **CLOSED AS A ROW 2026-09-04 AND FILED AS QUESTION 53.** Everything an
  engineer can decide is decided above: the measurement is re-derived three
  times, the scan's limits are checked, the size is known to be zero, and the
  plain sixth edge is already cut. What is left is one sentence of intent — is a
  declared-ahead-of-use seam something this project keeps, or deletes until the
  day it is wired? — and all five follow from it in either direction. ⛔ A row
  that cannot move without a ruling should not hold an execution slot; it should
  be the ruling's own row, which it now is.

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

- ▢ **POST-CARVE: SWEEP THE DOCS FOR PROSE LOCATIONS, because the citation
  checker is STRUCTURALLY BLIND TO THEM.** (Found 2026-09-03.)
  ✔ **SWEPT FOR THE `ambition_abilities` CARVE, 2026-09-04, AND NOTHING WAS
  STALE — recorded because "nothing changed" is a result the next sweeper should
  not have to re-derive.** The four planning citations that name a traversal
  ability (`abilities/traversal/{flyline,trapdoor,teleport}` in
  `agentic-character-runtime.md:42`, `performer-up-b-the-wire.md:113`, and
  `possession.rs` in `simulation-authority-and-determinism.md:251`) all name
  Family B, which STAYED in the kernel. ⭐ And the two directories are DISJOINT —
  `ambition_abilities/src/traversal/` holds blink, dive, grapple, mark_recall;
  the kernel's holds flyline, possession, teleport, trapdoor — so none of those
  paths is even ambiguous, which is the failure mode that bit three citations
  elsewhere in this file today.
  `check_planning_citations.py --strict` read **1,222 citations, all resolved —
  before and after five sentences were fixed that were flatly false.** It
  resolves cited SYMBOLS; a sentence that merely says where code lives is prose:

```text
"…is `actor_monolith::items::pickup`, which stayed in the kernel"
"`items::pickup` KEEPS the pressed pickup"
"Guards: `items::pickup::tests` (the request)"
```

  ⇒ A carve updates every citation the tooling can see and leaves every
  DESCRIPTION of a location — and *"which stayed in the kernel"* is exactly what
  a reader trusts without grepping, because it reads as settled architecture
  rather than a dated fact.
  ▢ **The routine, after any carve:** grep the planning tree for each moved
  module's OLD path, then sort the hits into HISTORY (past tense, a quoted
  correction, a checklist's record — KEEP) and STALE (present tense about where
  code is, or a guard cited at a module that no longer exists — FIX).
  ⛔ **RE-TENSE, DO NOT DELETE**, and the ratio is why: of eight hits on
  `items::pickup`, **five were correct history and three were stale**. A regex
  that purged the string would have broken five true sentences to fix three
  false ones. `keeps` → `kept` retires the location and preserves the argument,
  which usually survives the move.
  ⚠ The same blindness covers `//!` module headers in code — a doc comment
  naming where a sibling lives is prose too.

  ⭐ **AND ONE HALF OF THIS IS NOW MECHANISABLE**, added to
  [`engine/pickup-carve-checklist.md`](engine/pickup-carve-checklist.md)
  2026-09-03: `scripts/orphaned_symbols.py` catches the sub-case where the carve
  rerouted around a function and left it standing for the tests. Every doc
  naming it still RESOLVES — the symbol exists — while every sentence about it
  is false, which is precisely the blindness this row describes, in the one
  shape a checker can find. Its motivating case is on main: `6c9fb2b58`
  rerouted onto `retire_realizations`, left `demote_stale_realizations` behind, <!-- cite-ok: naming the retired function IS the example; it is what the reroute left behind -->
  and three planning sites went on calling the dead one live.
  ⛔ **IT DOES NOT REPLACE THE GREP.** It finds names that lost their callers;
  it cannot see a sentence that describes a location correctly-shaped and
  wrong — *"which stayed in the kernel"* names no symbol at all. Run both.
  ⓘ Measured across the abilities carve for calibration: delta of ONE, a
  `test_support.rs` helper, benign. A carve whose delta is several DOMAIN
  functions has left its callers somewhere, and the names say where.

- ▢ **D33 — continue actor-monolith decomposition by coherent ownership.** Pick a
  carve that removes a real authority/dependency edge from the residual actor
  kernel, moves registration/tests with the domain, and improves capability or
  compile/test isolation. Do not carve by LOC and do not promise frame-time
  improvement without a measurement.
  ⇒ **THE EXECUTABLE HANDOFF IS
  [`engine/actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md).**
  When D33 is selected, re-measure HEAD with
  `python3 scripts/measure_kernel_module_graph.py --edges 20` and take the READY
  packet from that page. The deeper evidence and design stay in
  [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

  ⭐ **WHAT EVERY CARVE OWES AFTER IT LANDS (added 2026-09-02, from three rows
  that were stale within a week of the merge that closed them).** None of this is
  new work invented for carves — it is the paperwork a merge does not touch,
  because the code lives in one file and the claim about it lives in another:

  1. **`python scripts/modules_md.py`** — must print *"MODULES.md up to date"*.
     A carve moves modules; the maps are generated and go stale silently.
     (75 crates as of 2026-09-03, after `ambition_abilities` and
     `ambition_encounter_features` landed;
     70 the day before. ⛔ AND IT WAS STALE WHEN THAT CUT LANDED — not from the
     cut: `4ac56a996` added `ambition_encounter/src/mob_seed.rs` and left its
     map at 17 modules. Regenerated in the post-carve pass, which is what this
     item is for.)
  2. **`python scripts/check_planning_citations.py`** — a carve renames or moves
     the very symbols the planning rows cite. ⭐ **AND IT COVERS THE DOCTRINE
     PAGES NOW** — calculex widened the checker to scan `docs/concepts`,
     `docs/systems`, `docs/architecture` and `docs/recipes` as well as
     `docs/planning`, because every module that leaves a crate strands the pages
     that cited its old home, and those are the pages a new agent reads first. ⭐ **AND THEN `--vanished <the
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
     ⚠ **THIS IS NOT THE SAME USE AS THE PERIODIC LANE, and the two must not be
     confused.** `./run_tests.sh --maintenance` runs `--vanished` against a
     FIXED baseline ref, deliberately: it asks *"what has gone stale since the
     last time a person triaged this corpus"*, and a rolling window would make a
     row silently stop being a finding because the window slid past the rename.
     The CARVE use is the opposite — a range, scoped to one landing, asking
     *"what did THIS cut leave behind"*. Same flag, two questions; pass a range
     here and leave the lane's fixed ref alone.
     ⚠ AND PREFER A FRESH WINDOW to a wide one. Measured
     2026-09-03 over a week: 37 hits, and on inspection essentially all were
     rows RECORDING a removal ("Deleted: `FpsOverlayState`", "the view is <!-- cite-ok: a row quoting a removal record -->
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
     | `character_runtime/match_activation.rs` | `a-second-writer-of-a-match-global-must-answer-ownership` |
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
     `probe_what_every_waiver_actually_covers`, an `#[ignore]`d listing meant to
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

  11. **⛔⛔ THE COMPILE-COST RATCHET IS A PER-CARVE LEDGER AND IT IS RED.**
     `python3 scripts/compile_ratchet.py` — 2 s, in the default gate, and it
     fails the gate today. It is the ledger the D33 campaign is accumulating
     debt in, and every one of its five messages is about a carve:
     `REGRESSED` ×2, `PATH` (the serial chain got longer), `UNPRICED` (a new
     crate has no measured cost), `CARVED` (a win whose baseline is now stale).
     ⇒ A carve that adds a crate touches ALL of them at once, which is why it
     belongs on this list rather than in a campaign doc.

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
  ⛔ **WHAT STAYED BEHIND *AT THE TIME*, and the split is by TRIGGER not by
  size:** `items::pickup` — ⚠ which itself became `ambition_held_items` the next
  day, so this reads as history now, not as where the code is
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
  ⚠ MEASURED PRE-CARVE (2026-09-02) and kept as the reading that unblocked the
  cut — two of the paths below are no longer kernel paths at all
  (`abilities` and `ability_cooldown` are `ambition_abilities` since
  2026-09-03), which does not weaken the measurement but does mean it is
  history.
  The plugin named `crate::abilities::{ranged ×10, traversal ×4, thrown ×3}`,
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

  ⛔⛔ **THE PARAGRAPH ABOVE WAS FALSIFIED FIVE TIMES ON 2026-09-03 AND IS KEPT
  ONLY AS THE RECORD OF A WRONG PREDICTION.** The `items/` module it names as the
  edgeless remainder was the FIRST thing to leave, as `ambition_held_items`
  (`bbfa38a3d`) — with a manifest edge, a policy allowlist and a source-purity
  rule at the end of it. Then `ambition_body_seed` (`962dba34d`),
  `ambition_match` (`7e625e5a5`), `ambition_encounter_features` (`b67c1348f`)
  and `ambition_abilities` (`4c31111f9`). The kernel's own source fell
  112,733 → 101,042 lines in that day — a raw `src/` line count from the day's
  start. ⚠ The compile-cost row above reads `108,364 → 98,808` for the same
  crate because 108,364 is the RATCHET'S STORED BASELINE, not that morning's
  value: same ruler, different reference point. Verified by running the ratchet
  2026-09-03 late — it reports `largest_unit_lines 108,364 -> 98,509` and a
  plain `wc -l` over `src/` gives 98,509 too.
  * ⇒ **The prediction failed because it looked for edges rather than for
    OWNERSHIP.** A module "named 79 times by the rest of the kernel" is not
    thereby internal; every one of those names is a candidate boundary, and four
    of the five carves cut exactly through such a module. Ask what a module
    OWNS, not how often the kernel says its name.
  * ⚠ **And the metric this row watched moves the WRONG WAY**, which is why the
    room to cut looked absent: the monolith's `[dependencies]` table went
    29 → 33 across those same carves, because a kernel that stops CONTAINING a
    domain starts DEPENDING on it. Success reads as regression there.
    ✔ **Re-measured 2026-09-03 late: still 33 `ambition_*` entries (52 total).**
    ⭐ The count HOLDING across the abilities and encounter-features carves is
    the more interesting reading, and it is the one this row is about: those
    domains were already depended upon before they were crates, so drawing the
    boundary named an edge that existed rather than adding one. ⇒ Which is why
    the number is diagnostic and not a score — it rises when a carve exposes a
    NEW edge and holds when it only makes an old one visible, and neither is
    good or bad without asking which happened.
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
  ✔ **PREMISES RE-CHECKED 2026-09-03 late without a build:** that test still
  exists — `ambition_platformer2d_actor_monolith/src/construction/tests.rs` —
  and `game/ambition_content/src/duel_arena.rs:67` still cites it by name — the
  bare filename is AMBIGUOUS, two tracked files carry it — so the pin and its citation
  have not drifted apart; `SMASH_FIGHTER_KIT` is still live at 12 references,
  which is the row's point that it stayed an ability GRANT rather than being
  retired; and the three ids the row names as authoring from their own demo
  crates still have no `authored/<id>.rs` anywhere in the tree. ⛔ Not
  re-checked: whether the tests still PASS, which needs a build.
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
  in v140, so 4 would decode an old `FullReset` as a reconstitution instead of <!-- cite-ok: deleted variant; the row records the wire history -->
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

- ▢ **CAPABILITY FOOTPRINT: 51 crates linked, 23 a movement-only game never
  asked for — and the count CANNOT fall by a manifest edit.** (⚠ FIVE pairs are recorded below and none of them is authoritative; `python3 scripts/check_absence_contracts.py | grep footprint` prints the live one. 45/18 as of `479f9d3e4`, when `ambition_registry_core`
  entered the closure; 46/19 as of `bbfa38a3d`, when `ambition_held_items` did; 47/20 as of `83460e3f3`, D33 cut 1, when `ambition_body_seed` did; 48/21 as of `7e625e5a5`, cut 2b, when `ambition_match` did; 50/23 on 2026-09-03 when `ambition_abilities` and `ambition_encounter_features` did; **51/23 on 2026-09-04 when `ambition_sprite_fx` did — the first rise that is NOT a carve, and the first declared entry deliberately kept OUT of `never_asked_for`: a movement-only game draws sprites, so altering a sprite's pixels is rendering rather than a gameplay domain. ⭐ The closure moved and the count did not, and the two numbers separating is the point. ⛔ It was also the crate whose new `fixtures/minimal_game/Cargo.lock` entry made `cargo tree --locked` exit 101 and CRASH the whole checker, so this ratchet — and every contract after it — reported nothing at all until `a6989d76b`.** ⭐ EVERY RISE BEFORE `ambition_sprite_fx` IS A CARVE PAYING ITS DEBT — do not read the series as regression, and do not read it as uniform either: five of the six are a domain LEAVING the monolith, and the sixth is a new capability the renderer composes.) (Scheduled
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
- ✔ **`ambition_registry_core`: R2 + R3 LANDED 2026-09-03 (crate + two pilots:
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
  whose policy differs. ⇒ Four consumers now, not two.
  ✔ **AND R4's SECOND HALF IS ANSWERED TOO — by this row's own design doc, which
  had already ruled on all seven (`triage/ambition-registry-core.md`, "The '7
  silent overwrite' registries: at most 2 are accidents").** Four state the
  replace in place (`ParamSchemaRegistry`, `EncounterRegistry`,
  `MovePrefabRegistry`, `FrontendAudioRegistry`), one is a test hatch
  (`PreparedCharacterRegistry::insert_prepared`). So the row was asking for work
  that had already been done and written down.
  ⭐ **Of the two the doc left open, ONE IS MOOT: `GatePortalRegistry` HAS NO
  PRODUCTION PRODUCER AT ALL.** Measured 2026-09-04 across every road: the only
  call to `register` is in a render test
  (`rendering/deferred_write_safety.rs:179`); nothing constructs a
  `GatePortalConfig` in production; no LDtk world contains a `GatePortal` entity
  (0 in all four `ambition_content` worlds); no lowering names one. The two
  non-code hits are a PROP's art kind in an authoring spec
  (`tools/ambition_ldtk_tools/specs/gate_stack_lower_area.ron:208`, `type:
  "Prop"`) and a sprite-name list in a doc comment — neither reaches the
  registry. A registry nothing registers into cannot overwrite anything.
  ⇒ **`CombatBanterRegistry` is the only live question**, and it does have a
  production producer (`game/ambition_content/src/dialogue/mod.rs:44`): should a
  second bark set for one enemy replace the first or conflict? That is one
  sentence for whoever owns conversation content, not a migration.
  ✔ **CLOSED 2026-09-04 as a ROW: R4 asked for work that was already done and
  written down, and the residual is a content sentence rather than engineering.**
  Both halves are answered above from the code and from the row's own design
  doc; the one open question is filed in `awaiting-maintainer-decision.md` so it
  stops occupying an execution slot. ⛔ Nothing here is deferred — there is no
  migration owed and no abstraction waiting on a ruling. R5+ starts from
  `triage/ambition-registry-core.md` when a fifth consumer wants the protocol,
  not from this row.
  ⚠ The dormant gate-portal cluster the measurement turned up is its own item —
  see `tracks.md`. The
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
  ⛔ ONE of them is, not both: the placeholder count is tier-SCALED (0 at Potato, 129 at Ultra, measured 2026-09-03) because the materialization ration charges Full 16 against Potato 1; `asset_wait_ms` is the tier-independent one. See status.md.
  ⭐ **AND THE TIER TELL IS TESTABLE HEADLESS AFTER ALL — `AMBITION_QUALITY_PROFILE=ultra`.**
  A headless box seeds `potato` from its adapter (*"visual quality seeded to
  `potato` for a Cpu adapter (llvmpipe)"*), which is why this looked host-only;
  the boot override wins over that and `capture_scene --help` says to pair them.
  Measured 2026-09-03 at HEAD with the cap removed —
  `AMBITION_QUALITY_PROFILE=ultra AMBITION_PROFILE_CENSUS=1 capture_scene
  hall_of_characters player 640x360 --warmup 400` — census reports
  `profile=Ultra … parallax_resolution=Full msaa_samples=4`, and **zero "nothing
  demanded it" warnings**: the first tell, re-confirmed at the user's tier
  rather than only at Quarter.
  ⛔ **DO NOT READ THAT LOG'S `sprites_potato/` LINES AS A RULE VIOLATION.** 14
  `[image-drawn]` lines name potato art at t≈3.5 s while the census does not
  report `profile=Ultra` until t≈14.9 s — draws from BEFORE quality convergence,
  and `player_robot_v3` appears once at potato and four times at `sprites/`
  after it. A slow headless boot makes the convergence window wide enough to
  photograph. ⇒ The TIMING tell still needs Jon (llvmpipe milliseconds are not
  his machine's); the tier and count tells no longer do.
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
