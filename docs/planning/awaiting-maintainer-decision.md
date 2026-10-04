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
- Do not file "nothing uses X: keep or delete?". Current usage is not a deletion
  criterion (Q74 ruling, 2026-10-03); ask about meaning, authority or cost.

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

- [`Q142`](#q142--must-every-presence-filtered-component-be-rollback-registered)
  appears in `ID-PEER` only as history.

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

## Q55 — should authored worlds grow to use all five route-gate families?

The vocabulary exists, and the families stay as capability surface whether or
not a demo uses them (usage is not a deletion criterion, Q74 ruling 2026-10-03).
Decide only whether broader authored coverage is wanted now.

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

## Q148 — which track should Mode Collapse fight to?

Current default, so this blocks nothing: `crooked_ascent_boss`, an authored boss
track that nothing else uses. The spec's note wants a bespoke "mode collapse"
track (a loop that degenerates); that is an art handoff.

* **(a) Keep `crooked_ascent_boss`.** No change.
* **(b) Commission the bespoke track.** Edit the four `music_*` fields of
  `mode_collapse_boss.ron` when it exists. To change one room only, set that
  room's `fight_music_track` ([room music recipe](../recipes/room-music.md)).

## Q153 — how does a second player join Ambition?

Split from Q151 when its death rule was decided (2026-10-03, see
[`maintainer-decisions.md`](maintainer-decisions.md)). Ambition has no join
road: only the primary seat gets a body in production (`avatar/bundles.rs`),
and the only production code that seats slot 1 is Smash's match activation.
The death rules count only `PlayerEntity` as a participant
(`session/death.rs`), so a seat-driven body that dies takes the enemy road
(`actor_hit.rs`): a "defeated" banner, a bounty coin and its authored respawn
policy. In the two-player fixtures the second body is a placement of its room,
so a rebuild of that room builds the body again without its seat; a join road
must give the seat's body a home that a rebuild does not replace, as
possession does through custody.

Owner: the join road row in
[open-world runtime and residency](engine/open-world-runtime-and-residency.md).

* **(a) Join at the primary player's room:** the new seat's body is stamped
  into the room the primary player is in.
* **(b) Join at a chosen room or shrine:** the joining player picks where to
  enter, from the rooms the session has visited.
* **(c) Join at the start room** of the current save, independent of where
  the primary player is.

## Q154 — should a pickup that authors no policy be gone for the run once taken?

Filed 2026-10-04. Blocks nothing: the engine reads the default as
`OnRoomReload` today and serves an authored `Never` (open-world "Regrowth/restock").

`HazardRespawn`'s documented default is `Never` ("never respawn inside the
current run/session"), but until 2026-10-04 no pickup kept any record once
taken, so every pickup came back when its room was built again. Now a pickup
authored `Never` is remembered `Consumed` in the occurrence ledger (and in the
save); a death brings back only one taken after the checkpoint. A pickup that
authors nothing reads `OnRoomReload`, which keeps today's behaviour: of the
521 shipped `PickupSpawn`s (intro 9, sandbox 29, Mary-O 25, three Sanic
worlds 458; counted 2026-10-04 by parsing the `.ldtk` files), only the two
`basement_breakables` hearts author a policy, and `PickupSpec::new` /
`Pickup::new` now say `OnRoomReload`.

Owner: the regrowth/restock row in
[open-world runtime and residency](engine/open-world-runtime-and-residency.md).

* **(a) Keep `OnRoomReload` as the pickup default (current).** Hearts and coins
  come back on every visit; a designer writes `Never` for a one-time pickup.
* **(b) Make `Never` the default for authored pickups.** The persistent world
  remembers every pickup taken. Mary-O's level loop and Sanic's act cycle
  would then need `OnRoomReload` on their coins and rings (or a replay that
  restores them), or a second lap finds them gone.
* **(c) Per-world default:** a world declares its pickup default (Ambition
  `Never`, the arcade demos `OnRoomReload`).

## Architecture and engine policy

## Q62 — keep or discard the epoch-captured 4,741-line `mary_o.ldtk` delta?

This is the explicit history/content decision. Do not modify the retained LDtk
files until the ruling is made.

## Q78 — how should the divergent/unpushed sprite-renderer submodule state be reconciled?

Before any blind `git submodule update`, decide which line/commit must be kept and
pushed. Tooling warnings should cite this question until the submodule state is
settled.

## Q94 — what residency-memory limit should the runtime target?

Needs a maintainer/hardware/product value. The residency mechanism can enforce a
budget once the budget exists. Report source, decoded CPU, prepared simulation
content and device residency separately. A8 instance isolation and A9 dependency
closure do not supply a hardware budget.

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

Whether the save belongs in what two peers agree on is decided (Q129,
2026-10-03, [`maintainer-decisions.md`](maintainer-decisions.md)): shared
durable world state is peer state, compared by its canonical semantic form.

The ruling's guarantee holds: no tick is simulated over an unapplied save. The
restore runs at the head of the first tick that has the body, before the core
step. `a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once`,
the test the ruling names, passes without the gate.

* **(a) Keep the restore in the simulation** (current). One road for every
  composition; the session may start before the save is applied.
* **(b) Restore the gate and the `Update` chain**, and give compositions whose
  body is born on the timeline another road. That is two roads for one fact.

## Q155 — is a hold of about 0.6 s at a peer room crossing acceptable, or must the crossing keep its session?

Under a peer session a room crossing now commits (2026-10-04, the "Remote
peers" row of
[`open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md)).
The simulation of each peer is held from the frame after the crossing was
recorded until the first frame of the next peer session. Measured in
`two_peers.rs` at a link latency of 3 updates each way (four runs, two
crossings): 23 to 43 updates, which is 0.38 s to 0.72 s at 60 Hz. 2 to 7 updates
are the wait for the confirmed frame and the readiness of the room. 21 to 36
are the handshake of the next GGRS session, which is five round trips
(`NUM_SYNC_PACKETS`, a constant of the pinned GGRS), so this part grows with
the latency. The hold is of the whole world: the room that nobody leaves stops
too. A local (sync-test) crossing holds nothing.

Owner: netcode ([`netcode.md`](engine/netcode.md)) and online play (A4 in
[`multiplayer.md`](game/multiplayer.md)). No queue row waits for this: the
crossing works, and the question is its cost.

* **(a) Keep it** (current). A door between rooms is a pause of about half a
  second for each online player. No new machinery.
* **(b) Shorten the handshake and keep the new session.** A session that
  follows another between the same peers needs one round trip, not five. That
  is a change in GGRS (a fork or an upstream option). Estimate: the handshake
  goes from 21..36 updates to about 8, the hold to about 10..15 updates.
* **(c) Keep the session (shape B).** Readiness bits ride in the input, the
  inputs of the held frames are a canonical null, the host stalls a few frames
  and runs the operation with no rebase. Estimate: a hold of about 6 frames
  plus the latency. It changes the wire input, it commits with no rebase (the
  old ring slots hold the room before the operation and stay safe only because
  no rollback can reach them), and the freeze of the whole world stays.

None of the three ends the whole-world hold. A hold of only the rooms of the
crossing is a separate change (the simulation gate is one condition for each
live room today).

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
