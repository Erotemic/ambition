# Awaiting a maintainer decision

This file holds only open maintainer and product questions. Engineering work
that can proceed without a ruling goes in [`queue.md`](queue.md).

- Give each question one unique `Q<number>`. Do not reuse a retired number.
- Use the heading shape `## Q<number> — <question>`. Other documents link to
  these anchors, so do not change the text of a heading.
- Each entry states the question, the context needed to choose, the owner, and
  what changes for each answer.
- A queue row that waits on a question names it in a `**Blocked by:**` field.
- When a question is answered, record the ruling in
  [`maintainer-decisions.md`](maintainer-decisions.md), update the owner plan or
  source, and delete the question here.
- Before you file a question, make sure that no ruling already decides it. A
  decided direction with unfinished implementation is a queue row, not a
  question.

## What actually blocks architecture work today

This is the blocking set, not the important set. A question can matter and
block nothing.

**The blocking set is empty (2026-10-01).** No open P0/P1 queue row states a
`**Blocked by:**` question.

| question | what it blocks | and if it stays open |
|---|---|---|

`scripts/check_blocking_set_names_every_gate.py` reads the queue's
`**Blocked by:**` fields and fails if this table has no row for a gate. Each
table row links a `Q` that has a section on this page with at least two
options, written `* **(a) …`. Derive gates from the field, not from row prose.

One question looks like a blocker and is not:

- `Q142` appears in `ID-PEER` only as history.

## Gameplay and content

## Q33 — how should a recharging ranged weapon communicate that it is unavailable?

Choose the player-facing unavailable/readiness signal. The mechanism should not
invent one presentation independently for every ranged weapon.

## Q36 — what are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

The engine has a canonical-height contract; these remaining authored characters
need product values rather than inferred sprite dimensions.

## Q40 — should a held gun-sword kick the player the way it kicks the pirate?

Choose whether recoil is an authored weapon property applied to every holder or a
pirate-specific behavior.

## Q41 — where should a hand-fired fireball leave the body?

Choose the authored launch landmark/offset contract for hand-fired projectiles.
Do not derive it independently from sprite bounds at runtime.

## Q42 — should the gauntlet fireball keep bespoke art or use the catalog energy ball?

Product/art choice. The runtime should consume one authored presentation identity
once chosen.

## Q43 — does a body hanging on a ledge inside a hazard volume die?

Choose the game rule for ledge custody versus hazard damage. The engine can then
encode one authority rather than special-case the observed overlap.

## Q44 — should `SmashChargeSpec` keep a Smash-specific name?

The mechanism is now broader than one game mode. Rename only if the intended API
is reusable; do not churn names solely for aesthetics.

## Q46 — does Mary-O 1-1 want a fourth question block over floor?

Content-layout choice needed to make the floor-refusal behavior of the fire form
meaningfully playable.

## Q47 — where does TwinTrack's simultaneity limit live while the exhibit is parked?

Choose whether the parked exhibit still consumes a global/participant/world
simultaneity slot.

## Q49 — is near-identical CPU play on a symmetric stage acceptable?

If yes, no diversity mechanism is owed. If no, specify whether the desired
variation is tactical policy, difficulty behavior or presentation/personality.

## Q52 — does a second bark set for one enemy replace the first or conflict?

Choose whether bark sets are single-valued authoring or composable collections.
The content validator should enforce the chosen cardinality.

## Q55 — should authored worlds grow to use all five route-gate families?

The vocabulary exists. Decide whether broader authored coverage is desired now or
the unused families should remain capability surface without demo customers.

## Q58 — does the BODY gate family ask what a body *can do* or what it *is doing*?

Capability and current action are different facts. Pick the authored gate
semantics before extending content usage. The Q54 ruling (2026-10-01) already
says that such a gate is evaluated per actor; this asks which body fact it reads.

## Q67 — does the Limit meter survive stock loss?

Choose stock lifetime for the meter. The earlier free-Limit respawn bug is fixed
independently of this decision.

## Q70 — should the title Settings tab visibly highlight on pointer hover?

Small UI/product choice; implementation already has the semantic tab state.

## Q71 — how much Limit should a successful block award?

Generic meter policy now permits a block source. Choose the Smash balance value;
keep it out of generic validation.

## Q79 — how far may the camera zoom out before the fight stops being legible?

Choose the product legibility floor. Camera policy can then clamp against a named
limit rather than an arbitrary tuning value.

## Q87 — should the top platform and respawn point continue to overlap?

Stage-layout/product decision. If not, move one authored placement rather than
adding runtime avoidance.

## Q89 — what special should each Robot stand-in have?

The stand-ins currently lack the button vocabulary expected by their match role.
Choose authored moves or explicitly accept the omission.

## Q91 — keep the 10× countdown mode?

This was explicitly requested earlier and remains available. Decide whether it is
still a product/debug affordance worth carrying.

## Q93 — should the demo author a dense melee room?

Product/content call. Do not add engine behavior merely to manufacture a stress
scene unless the room itself is wanted.

## Q81 — what should happen to the mostly-unreferenced bespoke FX rows for Pirate Admiral and George Booul?

The FX-row census finds these sheets as the extreme unreferenced-art case.
Either wire effects that correspond to intended authored moves, deliberately keep
rows as future art, or remove superseded rows. The count is owned by
`scripts/measure_fx_row_reachability.py`; do not copy a stale number into code.

## Q82 — should LDtk editor-preview tilesets remain runtime-packaged when the runtime never draws them?

Confirm whether another tool/runtime consumer needs them before excluding them
from runtime residency/packaging. A concrete proposed retarget is preserved as
`dev/patches/ldtk-player-tileset-retarget-20260902.patch`; it changes the map-assets
submodule and therefore needs an explicit content/pointer decision.

## Q83 — should the 442 MB shared sprite pack remain when one prop is the only current reader?

Choose whether this is intentional shared infrastructure or a packaging mistake
that should be split/deferred.

## Q84 — should portraits have independently authored readable low-resolution tiers?

Current generated tiers preserve existence but not necessarily readability.
Choose the product quality requirement before adding portrait-specific generation.
`dev/patches/portrait-tiers-are-never-baked-20260902.patch` is the existing proposed
implementation for the "full-resolution only" answer. Related: the Q69 ruling
(2026-10-01) says readability is not an acceptance criterion for the potato
tier.

## Q85 — should Hall characters without authored interaction dialogue remain non-interactive?

Content decision: author dialogue or explicitly accept that those cast members are
visual/background only.

## Q88 — who owns the Smash CPU difficulty ladder?

Choose whether ladder tuning is Smash ruleset content, generic fighter-brain
policy, or a combination with an explicit boundary. The current engine should not
infer this from table location. The Q127 ruling (2026-09-19) already says that
difficulty is game policy and that CPU brain levels are separate from
participant handicaps; this asks only where the ladder's tuning lives.

## Q90 — `read_weight` is authored on the ladder and inert: wire it or delete it?

`read_weight` is authored on every rung of `fighter_brain_ladder.ron`, but it is
read only behind `uses_rollouts()`, and every shipped rung sets the rollout
fields to zero. Do not retain an authored difficulty field that never affects a
decision. `read_weight_changes_nothing_while_the_shipped_rows_disable_the_rollout`
(`crates/ambition_combat/src/brain/fighter/rollout/tests.rs`) pins the current
state and must be deleted with either answer.

## Q92 — is the BODY-PROFILE developer experiment still wanted?

If no, delete the experiment and its planning residue. If yes, name the
measurement it must produce before more implementation work.

## Q95 — fast-forward the music renderer's main and repin the superproject so fresh clones refuse General-MIDI fallback?

The refusal exists on the renderer line prepared for this purpose, but the
superproject pin does not contain it. The safe operation is a durable
fast-forward/push of the renderer's main followed by the parent pointer bump; do
**not** repoint the parent at a deletable agent-only commit. In the same change,
flip `GATES` in `scripts/check_pinned_music_renderer_refuses_gm.py` from
reporting to gating.

## Q102 — is a solid breakable represented as `BlinkWall { Hard }`, or is that a temporary borrow?

The current solid-breakable road publishes hard-wall behavior through the
blink-wall vocabulary. Decide whether that is the intended durable
representation or whether breakable solidity needs its own semantic fact. Do not
split the type only for naming; split it only if the gameplay contracts differ.

## Q105 — may an author place a chest that is already open?

Runtime open state is the `Opened` marker. If authored worlds may start with an
open chest, add an authored input and lower it to the marker at construction. If
not, keep the authoring surface closed rather than restoring dead state
vocabulary.

## Q148 — which track should Mode Collapse fight to?

Current default, so this blocks nothing: `crooked_ascent_boss`, an authored boss
track that nothing else uses. The spec's note wants a bespoke "mode collapse"
track (a loop that degenerates); that is an art handoff.

* **(a) Keep `crooked_ascent_boss`.** No change.
* **(b) Commission the bespoke track.** Edit the four `music_*` fields of
  `mode_collapse_boss.ron` when it exists. To change one room only, set that
  room's `fight_music_track` ([room music recipe](../recipes/room-music.md)).

## Q149 — when a room retires and comes back, does a wounded enemy keep its wounds?

Current default, so this blocks nothing: **no, a returned room is built fresh
from its placements.** The save keeps an enemy's death (the `RespawnPolicy` fate
flag) and an encounter's outcome (`PersistedEncounterState`). It keeps no state
of a living enemy (HP, position, aggro) and no wave index of an encounter in
progress. This agrees with `OnRoomReenter`.

The Q38 ruling (2026-10-01) already makes a persistent character's whereabouts
durable, and OW3 keeps them: such a character is rebuilt from its record with
full health. So the open part is only whether its other live state (HP, a fight
in progress) is durable with its whereabouts.

Owner: [open-world runtime and residency](engine/open-world-runtime-and-residency.md), OW3.

* **(a) Fresh on return** (current). No work.
* **(b) Wounds persist.** Add a dormant record for a living actor (OW3's
  dormant mint rows are the road) and a per-placement policy variant such as
  `KeepsWounds`.

## Q150 — while two players are in two rooms (split view), whose HUD, banner and music show?

Current default, so this blocks nothing: **the session keeps one HUD, one
banner and one music stream, and they follow the primary seat's subject.** Each
player in a separate room has a view of their own, and every world drawable is
drawn in its own room (V1-V5 in
[open-world runtime and residency](engine/open-world-runtime-and-residency.md)).
Three surfaces are still one per session: the gameplay HUD (`PlayerHudFacts`),
the gameplay banner (`GameplayBanner`), and the music (there is one audio
output). Since 2026-10-02 the music does follow the primary seat: each live
room keeps its own fight claims, and the music intent plays the primary
seat's room (`PrimaryLiveRoom`). Before, the intent did not run while two
rooms were live, and any room's fight took the one claim. Options (b) and (c)
can build on the per-room claims.

This blocks only the last ◐ of the "separate from another participant" row in
[`game/open-world-roadmap.md`](game/open-world-roadmap.md).

* **(a) One of each, following the primary seat** (current).
* **(b) A HUD panel and a banner per view**, each from its own seat, with one
  music stream that follows the primary seat or the most urgent claim (a fight
  outranks exploration). Needs a per-seat HUD fact and a banner keyed by room.
* **(c) Per-view HUD and banner**, and music that crossfades to the room of the
  view with focus.

## Q151 — in Ambition, how does a second player join, and what does one player's death do while the other plays on?

Current default, so this blocks nothing: **a death resets the dying player's
own room to the checkpoint, and the other player's room goes on untouched.**
Held by `a_death_in_one_room_restarts_that_player_and_leaves_the_other_players_room`
(`game/ambition_app/tests/two_players_two_live_rooms.rs`).

Two facts under that answer are not decisions yet:

- **Ambition has no join road.** Only the primary seat gets a body in
  production (`avatar/bundles.rs`). The only production code that seats slot 1
  is Smash's match activation.
- **The death rules count only `PlayerEntity` as a participant**
  (`session/death.rs`). A seat-driven body that dies takes the enemy road
  (`actor_hit.rs`): a "defeated" banner, a bounty coin and its authored respawn
  policy.

The join road (which device, which body, which room) is the larger half of the
question. For the death rule:

* **(a) Restart the dying player at the checkpoint at once** (current), and
  give a seat-driven body the same participant death.
* **(b) NSMB co-op:** a dead player waits out of play until the other player
  also dies or rests at a shrine (`LevelReset::WhenNoParticipantRemains`). The
  roster must count seat-driven bodies. Poor fit for players far apart.
* **(c) The dead player restarts at the other player's room**, so the party
  regroups.

## Q152 — which world mechanic should keep time while its room is not live?

Current default, so this blocks nothing: **the breakable respawn keeps time.**
`HazardRespawn::AfterSeconds` due times are on `GameplayElapsed`, kept by
`BreakableRespawnSchedule` when the room retires and read when the room is built
again (OW5 in
[open-world runtime and residency](engine/open-world-runtime-and-residency.md)).
Body respawn policies do not keep time: `RespawnPolicy::InPlace(seconds)` counts
down on the live body and goes with the room (Q149). `OnRest`, `OnRoomReenter`
and `DeadStaysDead` do not wait on time. Encounters persist an outcome, not a
clock.

Also open: whether a world clock must survive a save. `GameplayElapsed` is a
session clock; a rewind restores it, the save does not keep it, and a checkpoint
restore forgets every breakable record.

Each option needs a logical clock that a rewind restores and that runs while a
room is not live, plus a durable record that a built room reads.

* **(a) A respawn that counts world time:** a policy such as `After(seconds)`
  that writes the death time to the save. Smallest: one durable timestamp per
  placement.
* **(b) A persistent character that moves on a schedule** between rooms. Needs
  a route and a reconstruction of where the character is at a given time.
* **(c) Regrowth or restock:** a pickup or a shop stock that refills after an
  amount of world time.

## Architecture and engine policy

## Q34 — should external/launch-owned motion become an explicit cross-game fact?

If more than one game needs it, give it a reusable authority. Otherwise leave the
current local mechanism local.

## Q35 — what owns fighter reach during move startup?

Choose the semantic owner of startup reach so AI, collision and presentation do
not each infer it from different move state. The Q107 ruling (2026-10-01) makes
the moveset the owner of mechanical attack timing; this asks about reach.

## Q37 — should the F9 rollback proof pulse survive a gameplay-session change?

Decide whether the pulse demonstrates one rollback session or is a process-level
debug affordance (`game/ambition_app/src/dev/rollback_observatory.rs`). Its
resource lifetime should follow that answer.

## Q59 — two validation ledgers can be red when no lane ran: hook the lane or accept the state?

The engineering half is done: the verdict-bearing scripts that carry a budget,
a waiver table or an absence contract run in `./run_tests.sh --maintenance`
(`scripts/run_tests.py`). Reports (`measure_*`, `render_*`) exit 0 by design.
The open part is the receipt model.

* **(a) "Not run" is a first-class incomplete receipt**, distinct from pass and
  fail. Ledgers and the lane must record it.
* **(b) No incomplete state.** A ledger that no lane runs leaves the required
  surface. This is acceptable while each check costs seconds; a check that
  costs minutes would need a policy.

## Q62 — keep or discard the epoch-captured 4,741-line `mary_o.ldtk` delta?

This is the explicit history/content decision. Do not modify the retained LDtk
files until the ruling is made.

## Q63 — should interactables gate on facing, chests persist per-chest, pickups carry collected state, breakables author a debris cue?

The four no-op authoring fields are deleted (`InteractableSpec.requires_facing`,
`PickupSpec.collected`, `ChestSpec.persistent`, `BreakableSpec.debris_cue`).
What remains is the product choice: are facing-gated interaction, per-chest
persistence, per-pickup collected state and per-breakable debris cues intended
capabilities? For each "yes", add the field and its consumer in one change. For
each "no", nothing changes. Until a capability is chosen, unsupported
nondefault values get diagnostics, not an invented runtime meaning. The Q45
ruling (2026-10-01) already lets a unique capability item behave as an
entitlement. See [item custody](engine/item-custody-and-accounting.md).

## Q66 — should the citation checker become a ratcheted gate now that its baseline is zero?

Choose whether citation health is required on every relevant change or remains an
advisory audit.

## Q68 — which settings are the evergreen baseline every game inherits?

Choose the cross-game groups/surface. The full settings menu and shell-level audio
controls already exist; this decides composition, not implementation feasibility.

## Q72 — `EncounterEffect::SetMusic` has no production customer: give it semantic identity or remove/defer it?

If retained, multiple simultaneous scripts must not share one subsystem-wide
claimant. If no authored use is planned, remove/defer the unused capability.

## Q74 — keep or cut the three declared dependency seams that still have no customer?

A seam with a plausible near-term composition use may stay; otherwise remove the
dependency rather than preserving hypothetical architecture. Recheck actual
production call sites and supported profile closure separately. The A9 render
dependency finding concerns a mandatory reachable path, not this older
unused-seam inventory; one is not evidence for the other.

## Q76 — are composite mount-riders actually planned?

`MountedBrainCache` has no production constructor. It is also the only body that
`apply_brain_commands`' source-only arm serves (the arm keys on the mount's
control claim, which only a cached rider files), and nothing resumes the source
that arm records when such a ride ends.

* **(a) Yes.** Keep the capability, and add the resume when a ride ends.
* **(b) No.** Delete the cache and the source-only arm.

## Q78 — how should the divergent/unpushed sprite-renderer submodule state be reconciled?

Before any blind `git submodule update`, decide which line/commit must be kept and
pushed. Tooling warnings should cite this question until the submodule state is
settled.

## Q86 — should cast framing be bidirectional (target) rather than only a floor/minimum?

Choose whether framing owns both minimum and desired composition or only prevents
excessive zoom-in.

## Q94 — what residency-memory limit should the runtime target?

Needs a maintainer/hardware/product value. The residency mechanism can enforce a
budget once the budget exists. Report source, decoded CPU, prepared simulation
content and device residency separately. A8 instance isolation and A9 dependency
closure do not supply a hardware budget.

## Q103 — what should an unprepared character id inherit at wear time?

Prepared characters fold catalog movement tuning and motion model at admission,
but the wear road still has a fallback for ids outside the prepared registry.
The shipped compositions have no orphan prepared ids, so this is a
boundary-policy decision, not a live content defect.

* **(a) Inherit the catalog's authored tuning at wear time.**
* **(b) Inherit engine defaults.**
* **(c) Refuse an unprepared wear.**

## Q109 — should a simulated identity be able to name its room instance?

Deterministic ids identify authored/simulated objects but do not encode a
room-instance dimension, so a second instance of the same authored room can
collide with an already-live identity. This gates A8's two-instance proof.

* **(a) Room-instance identity belongs in canonical `SimId` semantics.**
* **(b) Room instance is a separate deterministic scope** beside `SimId`.

## Q135 — reopened 2026-10-03: the save is applied by the simulation, not before the timeline

The 2026-09-16 ruling (`maintainer-decisions.md`, Q135) says GGRS does not start
before the durable restore finishes, and `maintain_local_session` refused to
create a session while hydration was pending. On 2026-10-03 the restore chain
moved into the simulation schedule and the gate was removed
(BODY-BORN-ON-THE-TIMELINE in [`queue.md`](queue.md)). New evidence:

- Three compositions build their primary body on the timeline (the Sanic and
  Mary-O rollback fixtures). The gate has no body to wait for, so the save was
  applied from `Update` over a live timeline there.
- `AmbitionGameSave`, `SaveRestored` and every value the chain writes are
  rollback state, so a restore inside the simulation is deterministic: a rewind
  past it applies it again on the same tick.
- With the chain in the simulation the gate would deadlock: under the rollback
  host the simulation does not run until a session starts.

Whether the save belongs in what two peers agree on stays with
[Q129](#q129--must-the-save-file-be-part-of-what-two-peers-agree-on).

The ruling's guarantee holds: no tick is simulated over an unapplied save. The
restore runs at the head of the first tick that has the body, before the core
step. `a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once`,
the test the ruling names, passes without the gate.

* **(a) Keep the restore in the simulation** (current). One road for every
  composition; the session may start before the save is applied.
* **(b) Restore the gate and the `Update` chain**, and give compositions whose
  body is born on the timeline another road. That is two roads for one fact.

## Q129 — must the save file be part of what two peers agree on?

`AmbitionGameSave` is registered `resource-clone-custom-checksum`, and its
checksum projection serializes the whole save. The `Update` mirror desync is
repaired: the three `persist_*_to_save` mirrors run in the sim schedule. Almost
every writer of the save runs in the rewinding schedule (quest, boss encounter,
switch, shrine, cutscene, map visit, dialogue visit, the mirrors). The only
outside writer is `load_save_at_startup` (`Startup`). To re-derive the writer
set, find each production `ResMut<AmbitionGameSave>` parameter and read the
schedule of the `add_systems` call that registers its system.

A second asymmetry has no recorded reason: `OwnedItems` is `resource-clone`
(not hashed), but `OwnedItemsBaseline` is `resource-clone-custom-checksum`, so
each checkpoint commit carries the bag into the peer checksum.

Owner: `DURABLE-HORIZON-CHECKSUM` and the deferred authority/representation
split of `ROLLBACK-BAG-DESYNC`, both in [`queue.md`](queue.md).

* **(a) The save is peer state** (current). Peers compare every simulation
  write to it, and the mirrors stay in the sim schedule. Make `OwnedItems` and
  its baseline agree.
* **(b) The save is a local artifact** and leaves the peer checksum. This stops
  comparing every sim-side writer above. The 2026-09-16 merged-state review
  rejected this option.
* **(c) Keep both and run the mirrors only on confirmed frames.** Needs a
  confirmed-frame hook that does not exist.

## Q134 — is a dialog visit count something two peers must agree on?

The rewind defect is closed:
`count_the_dialogue_visit_when_a_conversation_opens`
(`crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs`)
counts a visit in the sim schedule when an `ActiveConversation` opens. Held by
`a_conversation_opening_counts_exactly_one_visit_across_a_rewound_window`. What
remains is product: `dialog_visits` is inside the save's checksum projection.

Owner: `DURABLE-HORIZON-CHECKSUM` in [`queue.md`](queue.md). Related:
[Q129](#q129--must-the-save-file-be-part-of-what-two-peers-agree-on).

* **(a) A visit is shared world state** that peers agree on (current).
* **(b) A visit is per-player progress.** Move it out of the shared save or out
  of the checksum projection. Narrowing the checksum changes only what peers
  compare; the resource is still snapshotted and restored.

## Q142 — must every presence-filtered component be rollback-registered?

Resolved by engineering. Kept only because
`scripts/check_presence_filtered_state_is_rollback_registered.py` (its
`ACKNOWLEDGED` table) still owes `PostBossNpc` and `SmirkingBehemothVictoryNpc`
to this section and fails if the section does not name them. Both are now
registered (`marker.post_boss_npc` and `content.cut_rope_victory_npc` in
`game/ambition_app/tests/rollback_schema_baseline.txt`), and the checker
reports 0 owed. Delete this section in the change that removes those two
`ACKNOWLEDGED` entries.

The rule is owned by
[simulation authority](engine/simulation-authority-and-determinism.md): a
component whose presence a query filter reads is authoritative, even when its
value is derived. To witness a presence latch across a rewind, census the latch
on each resimulated pass. A visible consequence that is not itself rollback
state cannot witness the defect.
