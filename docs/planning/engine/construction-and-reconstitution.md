# Construction and reconstitution

**State:** OPEN — the four rebuild paths agree on the population they produce,
and durable restore reaches that agreement by CONSTRUCTION rather than
correction. C1–C4 are closed; C5 waits on a real transport customer.

> **Verified against `ef2c4bd50` (2026-09-04), and this header was WRONG.** It
> still said durable restore "reaches that agreement by building and then
> correcting, which is a shape worth removing" — the state C3 closed on
> 2026-08-31 and recorded in this file on 2026-09-02. A reader arriving at the
> top learned the opposite of what the file's own closed row says, and the
> roadmap's P1 section had already moved. Re-derived rather than re-read:
> `adopt_the_occurrence_ledger_at_activation` is at
> `crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs:85`,
> and its every-frame guard
> `a_load_never_authors_the_occurrence_it_is_about_to_suppress` is at
> `game/ambition_app/tests/canonical_reconstitution.rs:959`.
> ⭐ **The lesson is the one this file's own C3 row already contains: closing a
> row is not finishing it.** A `State:` line is a claim like any other, and a
> receipt written into the body leaves the summary above it stale.

## Goal

A room or authoritative population should be reconstructable from one semantic
model:

```text
prepared immutable content
        + durable occurrence / progression facts
        + explicit lifecycle policy
                    |
                    v
        canonical construction plan
                    |
      +-------------+-------------+-------------+
      |             |             |             |
  new session   room transition   replay     durable restore
```

The lifecycle operations may retain different populations and may require
different authorization barriers, but they should not invent different ways to
build the same authoritative state.

## Current architecture

### Prepared construction is transactional and federated

Room construction is already split into typed domain-owned lanes. The room
adapter translates authored world data into each domain's vocabulary; domains do
not depend upward on the world-spec crate merely to construct themselves.

One room transaction owns:

```text
plan -> preflight -> commit -> verify -> publish
```

The construction schema/fingerprint is metadata. It is not a string/`TypeId`
service locator and it does not select arbitrary constructors.

### Confirmed rollback transitions use the same readiness transaction

The rollback host no longer commits a room transition speculatively. A confirmed
lifecycle intent waits for the authorized prepared construction plan, commits the
room, then rebases GGRS onto a new frame-zero baseline. Earlier rollback frames
cannot restore the pre-transition room.

Therefore a "rollback snapshot that crosses the room boundary" is not an engine
requirement. Persistence/checkpoint state is a separate durable product boundary.

### Same-room replay is a construction, not a repair

A replay is ONE ADMITTED LIFECYCLE OPERATION, from request to reconstruction:

```text
RoomReplayRequested { reason }      the ASK — a reset press, a death, a "try
                                    again" beat. Anybody may write it. It may
                                    be refused, so NOTHING authoritative may
                                    change on it.
        |
        v  admit_room_replay: resolve the CONTROLLED body, name the active
           room, take the one pending-lifecycle slot
        |
RoomReplayAdmitted { reason, subject }   the FACT — one writer, only on
                                         acceptance. Every consequence hangs
                                         off this: the subject back at spawn,
                                         the attempt's residue retired,
                                         content's per-attempt state cleared,
                                         the portal policy chosen by `reason`.
        |
        v  the room-transition road prepares, authorizes, commits and rebases
```

`admit_room_replay` records a `LifecycleIntent::Transition` naming the ACTIVE
room, and the room-transition road rebuilds it — the road a door takes, and the
road a checkpoint resume already took. The in-session rebuild paths differ in
three values and in nothing else:

```text
                    target room      retires                  durable facts
new session         start            (nothing live yet)       as saved
room transition     the next one     RoomResident             as remembered
same-room replay    the active one   RoomResident             as remembered
new-game reset      start            every RoomScopedEntity   forgotten
```

`RoomResident` is "room-scoped and not in anybody's hands". Custody is the
retention class that decides what rides through a rebuild, so an object a body is
carrying survives a replay for exactly the reason it survives a door.

**Consuming the same construction semantics is not enough; the authorization is
part of the road.** An intermediate version of this work prepared and committed
the plan directly from the reset request. It passed every eager-host test and
desynced a sync-test session within three frames of a player death
(`rollback_lifecycle_reset`, checksum mismatch at frames 105-107): a room rebuild
is a structural change to the simulated world, so under a rollback host it may
only commit at a confirmed frame, after which GGRS rebases onto a fresh
frame-zero baseline. A second caller of the plan is a second lifecycle
authority even when it is not a second constructor.

What a replay retires BEYOND the room is the previous attempt's residue —
`SpawnedThisAttempt` loot, post-boss NPCs, in-flight projectiles. Those are
session-scoped by lifetime, so no room sweep reaches them and the policy states
them explicitly rather than by omission.

A replay therefore costs what a door costs: a couple of frames, a clock ease, and
a transition cooldown. `reset_sandbox` still returns the body and its meters
immediately, so the input still answers on the frame it is pressed.

**Admission is a decision, and the slot can refuse.** `PendingLifecycleCommit`
is one earliest-sticky slot, so `record` returns an `Admission` and is
`#[must_use]`: a caller that runs an operation's consequences without checking
has changed the world for an operation that never happened. That was live. On
every player death, `close_death_interlude` wrote BOTH `ResetToCheckpoint` and a
replay request; the checkpoint resume recorded its transition first, the
replay's record silently no-op'd, and the replay's consequences ran anyway. The
death road asks for one operation now — the checkpoint resume owns the room
change and announces the admitted replay, carrying `PlayerDeath` so the player's
placed gun portals survive a death and not a deliberate retry.

**The subject is the body you are DRIVING.** `RoomReplayRequested`'s contract
says "the controlled player"; the implementation queried `PrimaryPlayerOnly`,
which is the home avatar. While possessing an actor, a replay therefore reset
the body the player was not driving and named it as the rebuild's subject, while
the possessed body carried the previous attempt's state through custody. The
subject is resolved once, at admission, from `ControlledSubject`, and travels on
the admitted message rather than being re-derived later — the rule
`RoomTransitionIntent` already states about its own subject.

A composition with no controlled body still replays: `subject` is `None`, the
consequences that are not about a body run, and the transition road is not asked
for a crossing it cannot describe.

### Provenance/lifetime vocabulary exists

`SessionScopeId`, `SessionRoot`, spawn provenance/lifetime components, occurrence
records and room/session scope helpers already provide the vocabulary needed to
decide which populations a lifecycle operation may retire or reconstruct.

## What the hand-kept ledger cost

`reset_ecs_room_features` was a second, incomplete room constructor: sixteen <!-- cite-ok: records the deleted second constructor by name -->
queries at Bevy's parameter ceiling, mutating twelve families of surviving
entity back toward a presumed spawn state. Every row was a fact somebody noticed
was missing, so the list could only grow, and adding an authoritative family to
fresh construction never added it to the replay.

Its measured divergence at `ac633fce2`, on `combat_calibration_lab`: after a
replay every enemy came back **facing the wrong way** and drifted 34.6px from
where a fresh entry puts it, because `ActorMut::reset_to_spawn` hard-set
`facing = -1.0` and no row of the ledger restored the brain's chosen direction.
An enemy that patrols right on entry patrolled left — into the wall behind it —
after every retry.

## Required convergence

### C1 — name retention classes at the lifecycle boundary ✔

Retention is decided by declared lifetime/provenance, not by what a reset
function happened to remember:

- process-only diagnostics/services;
- gameplay-session authority (`SessionScopedEntity`, and the session-mirrored
  resources re-established at `SessionScopeActivated`);
- room-resident authoritative population (`RoomResident`);
- room-scoped but in custody (`RoomScopedEntity` + `InCustodyOf`) — rides
  through a rebuild with whoever holds it;
- attempt residue (`SpawnedThisAttempt`, post-boss NPCs, live projectiles);
- persistent occurrences whose durable facts outlive residency
  (`AuthoredOccurrences` -> `RoomOccurrenceOutlook`);
- rollback-timeline history, discarded/rebased at confirmed room boundaries.

### C2 — make replay consume canonical constructors ✔

Done. `reset_ecs_room_features` is deleted; `RoomTransitionSet::Reset` now runs <!-- cite-ok: the row states this symbol is deleted -->
`reconstitute_the_active_room`, which records a lifecycle intent for the active
room. The only thing that stayed behind is `SpawnedThisAttempt`, which is a
lifetime declaration rather than a reset row.

Two defects fell out of the convergence, both invisible while the replay reset
the world in the same frame it was asked to:

- **A boss's persisted defeat was re-derived from its corpse, every frame.**
  `update_boss_encounters` wrote the `Cleared` record whenever a boss sat in
  `Death` with its outro complete, guarded by `if !boss_is_cleared(..)` — which
  looks idempotent and is not. A road that RETRACTS the record, so the boss can
  be re-fought, had its retraction overwritten on the next frame by the body it
  was replaying. The record is now written on the death EDGE.
- **A scenario actor injected by the sim harness had no reconstruction record.**
  `spawn_boss_at` is documented as "the programmatic counterpart to a room
  `BossSpawn`", but it wrote a one-shot `SpawnActorRequest` into a world whose
  room definition said nothing about it — so it survived a reset and not a door.
  It now registers a `RoomContentStagingRegistry` stager for the active room, so
  every construction of that room produces it.

### C3 — make durable restore consume facts, not ECS snapshots — ✔ CLOSED 2026-08-31, recorded here 2026-09-02

The third of the three closings named below is the one that landed:
`adopt_the_occurrence_ledger_at_activation` (commit `758e9df37`, queue row
D-RESTORE-INTERIM) puts the file's occurrence ledger in place at
`SessionScopeSet::Activate`, and activation hands the real
`OccurrenceContinuity` to the first construction — the temporary population is
never built. Pinned every frame, not at the endpoints, by
`canonical_reconstitution::a_load_never_authors_the_occurrence_it_is_about_to_suppress`,
which boots WITH the save (`Platformer2dSimHarnessOptions::with_save`, the road
the binary takes) and is red under either leg poisoned alone. The two collisions
that trace found are their own rows and both closed (D-RESTORE-COLLISION,
D-RESTORE-LEDGER-SCOPE). What still restores after the body exists is the
item/wallet leg (`restore_inventory_from_save`), which needs a primary body and
adopts custody rows idempotently; no interim duplicate population is involved.
The paragraphs below are the 08-30 measurement that this superseded, kept for
the record.


The storage half holds: `AmbitionGameSaveData` is product facts keyed by stable
ids (occurrence whereabouts, custody, encounter/quest/switch/boss records,
inventory quantities), and it explicitly refuses component blobs.

The restore half is a build-then-correct. Session activation prepares its first
room with `None` occurrence continuity — the comment there says "Activation
BUILDS a world; there is no earlier occurrence of anything to have a disposition
yet", which is true of a new game and false of a load. The save's facts are then
adopted into that already-built world, and `complete_durable_restore` emits
`ResetToCheckpoint`, which reaches the canonical plan indirectly:
`shrine::resume_at_checkpoint_on_reset` records a room-transition intent, and
THAT construction does state the continuity.

**Measured 2026-08-30: the correction lands on the same FINAL population.** A
load of an empty file builds the room a session that never loaded anything has,
and a load carrying a relocated occurrence suppresses it exactly as walking back
into the room does — cases 7 and 8 of the acceptance suite, both red under a
poisoned `outlook_for`.

⚠ THAT IS EVENTUAL CONVERGENCE, NOT A CLEAN BILL. Two things it does not
establish, and the earlier language overstated:

- **The fixture is not a real startup load.** It writes the save resource and
  clears `SaveRestored` eight frames in; it does not exercise
  `load_save_at_startup`, the persistence root, or startup ordering. It is a
  focused test of durable-fact ADOPTION, and the module says so.
- **The interim world is unproven, not proven harmless.** The architecture
  currently permits an authoritative population to exist — and run — before the
  saved facts correct it, and no gameplay gate is tied to `SaveRestored`. The
  boss-defeat record is the warning: live state can re-derive and overwrite a
  durable fact inside exactly such a window.

So the build-then-correct shape is kept PROVISIONALLY. Closing C3 needs one of:
a genuine startup save-load test showing gameplay cannot observe or mutate the
pre-correction population; a gate on gameplay authority until restoration
completes; or supplying the saved occurrence facts to initial construction so the
temporary population never exists.

`ResetToCheckpoint`'s contract said it was the death/retry horizon and "not a
save load" while the durable road wrote exactly that; the comment was wrong about
its own producer and now says so. A death and a load differ in where the baseline
came from, not in what happens to the world.

### C4 — keep transition preparation and commit singular ✔

Eager/headless and rollback hosts authorize commitment differently but consume
the same prepared construction semantics. The rollback host's confirmed-frame
barrier is a lifecycle authorization layer, not a second room constructor.

### C5 — external/P2P lifecycle barrier only with a real transport customer

Local sync testing cannot exercise corrected remote input at a lifecycle barrier.
When external/P2P netplay is real, add a peer-coordinated confirmation/content
barrier around the existing construction/rebase seam. Do not build Matchbox
ceremony merely to satisfy a local planning checkbox.

## Invariants

- A domain owns the constructor for the state it owns.
- Prepared construction is deterministic for equal prepared content and durable
  facts.
- A lifecycle operation cannot publish a partially verified room.
- Replay/restore do not maintain independent semantic constructors.
- A rebuild prepares before it retires, so a failed preflight costs nothing and
  the running room keeps playing.
- Rollback history does not cross a confirmed room boundary; a new baseline is
  installed after the lifecycle commit.
- Durable persistence stores product facts, not allocator-local ECS history.
- A relationship is persisted only when the durable road can restore both sides
  of its authority.
- Presentation is reconstructed from authoritative state rather than preserved as
  hidden authority.

## Acceptance

`game/ambition_app/tests/canonical_reconstitution.rs` censuses the authored
room-scoped authoritative population — position, facing, health, disposition,
breakable state, collected/opened markers, switch position — and compares the
lifecycle paths against a fresh entry:

| case | what it pins |
| --- | --- |
| `a_freshly_entered_room_is_a_population_worth_comparing` | the premise: two empty censuses are equal, so the room must author a real population |
| `leaving_a_room_and_returning_rebuilds_what_entering_it_built` | the reference arm — a transition has always been canonical |
| `replaying_a_room_rebuilds_what_entering_it_builds` | the replay rebuilds rather than repairs |
| `a_replay_leaves_the_home_avatar_standing_at_spawn` | the HOME AVATAR is not retired with the room and returns at the room's spawn |
| `a_replay_follows_the_body_you_are_actually_driving` | ⛔ the separate claim: while possessing, `PrimaryPlayer` is the body you are NOT driving |
| `an_object_in_your_hands_survives_a_replay_and_is_not_re_authored` | BOTH retention legs: `RoomResident` and the durable-fact input |
| `a_replay_does_not_adopt_what_the_attempt_created` | the attempt-residue leg |
| `loading_a_save_builds_the_room_a_re_entry_builds` | the durable road produces the population the in-session roads produce |
| `a_relocated_occurrence_is_suppressed_by_a_load_and_by_a_re_entry_alike` | a durable fact construction must ACT on reaches both roads alike |
| `running_one_lifecycle_path_then_another_lands_in_the_same_room` | repeated rebuilds do not drift |
| `a_replay_keeps_session_progress_and_reports_what_it_touches` | ⛔ the DURABLE half — coins held during an attempt survive it, and which save families a replay touches is reported rather than ratcheted |
| `a_replay_retracts_the_boss_defeat_a_gate_would_have_read` | ✔ the ONE attempt-retraction mechanism, asked through `boss.cleared` rather than the save row |

The rollback host is covered by `rollback_lifecycle_reset.rs`, which drives the
same reset under a forced sync-test session and requires the timeline to stay
checksum-clean across the rebuild.

Each was verified red before green by poisoning one leg of the reconstitution at
a time; every poison failed exactly the case that names it.

The census compares positions within a 2px tolerance and everything else exactly.
The tolerance buys one thing: a boot builds its room before the first frame while
a rebuild commits partway through one, so two identically-constructed populations
get different fractions of a frame of motion. The defect this suite was written
against measured 34.6px plus a flipped facing.

## ⛔⛔ THE ACCEPTANCE COVERS ONE OF TWO AUTHORITIES (found 2026-09-04)

The census above is keyed by `SimId` over `RoomScopedEntity` and compares
position, facing, health, disposition, breakable state, collected/opened
markers and switch position. **Every one of those is ECS state.** The durable
save appears in that file only as a FIXTURE — `boot_with_save` writes one in,
and two tests clone one out to build the next arm — and **no case asserts that
a lifecycle path leaves the durable facts a fresh entry leaves.**

⭐ **That gap was cheap yesterday and is load-bearing today, which is why it is
written here rather than in a backlog.** As of 2026-09-04 the engine publishes
NINE authored conditions and eight of them read the save or the live state it
mirrors — `world.flag_set`, `world.switch_on`, `inventory.holds`,
`custody.is_held`, `encounter.cleared`, `boss.cleared`, `quest.active`,
`wallet.can_afford`. ⇒ A replay that leaves a stale durable fact produces a room
whose ECS population is **identical by construction** and whose DOORS and
DIALOGUE differ: a `gated_by` opens that should be shut, an `<<if>>` branch
takes the other leg. The population census cannot see it, and the 2px tolerance
and exact-match rules say nothing about it.

⛔ **AND THE FIX IS NOT "COMPARE THE WHOLE SAVE", which is the obvious test and
the wrong one.** A replay must NOT restore the wallet, the quest ledger or the
dialogue visit counts — those are session progress, not attempt state, and
asserting the two saves equal would pin a policy nobody has decided and would
red the moment a replay correctly kept something. ⇒ The decidable claim is
narrower: **an attempt-scoped fact recorded DURING the attempt must not survive
the replay.**

⚠ **There is already one hand-kept list doing exactly this, and it is the shape
this page's own history warns about.** `reset_cut_rope_attempt_on_replay`
(`game/ambition_content/src/bosses/cut_rope/mod.rs`) clears three facts by hand
on `RoomReplayAdmitted` — the persisted cleared record for every cut-rope
placement in the room and the intro music claim. ⚠ **It cleared a third, a
`smirking_behemoth_victory_npc_seen` flag, until 2026-09-05: that flag had ONE
write (this one), ZERO readers anywhere, and nothing that ever set it true.** The
behaviour its comment claimed — the post-boss conversation waiting for the next
kill — is delivered by the placement record two lines above, which
`spawn_cut_rope_victory_npc` gates on. Two facts by hand now, not three. It is well-placed (the content half of the replay, in the
engine's `ContentRoomReplayResetSet` slot, so the host consumer never names
cut-rope) and it is still a list that grows only when somebody notices — the
same property that made `reset_ecs_room_features` a second constructor.
⇒ **The difference is scale, not kind:** that ledger had sixteen queries and
twelve families; this has three facts. Which is an argument for a guard now,
while it is three.

✔ **THE DURABLE ARM IS LANDED, AND THE MEASUREMENT CHANGED WHAT IT COULD
CLAIM.** `a_replay_keeps_session_progress_and_reports_what_it_touches`
(`canonical_reconstitution.rs`).

⭐ **MEASURED 2026-09-04: a room replay of `combat_calibration_lab` changes
ZERO durable families.** Two families were disturbed by two DIFFERENT roads
before the replay, so an empty result is a claim about the replay rather than
about one field's plumbing:
- `wallet` — a live `BodyWallet` component that `items::persist` mirrors INTO
  the save (`data.wallet = wallet.balance`);
- `flags` — no live component at all; `set_flag` writes the save, and the save
  IS the authority.

⇒ **So the replay path has NO durable-fact policy of its own.** Every
attempt-scoped fact that must be retracted is retracted by a CONTENT system
that names it — today, exactly one: `reset_cut_rope_attempt_on_replay`. That
sharpens the risk above rather than relieving it: the hand-kept list is not one
consumer's shortcut, it is the only mechanism there is.

⛔ **THE ATTEMPT SIDE IS DELIBERATELY NOT ASSERTED, and the reason is scoping
rather than caution.** The one production consumer clears the persisted record
only for cut-rope placements PRESENT IN THE ROOM, so a test that recorded a
synthetic boss id and demanded the replay clear it would fail against correct
code and would pin a policy nobody has written. ⇒ The arm asserts the SESSION
side (coins held during an attempt survive it) and REPORTS the families a
replay touches, as an assertion-free line: a number nobody has ruled on must
not become a ratchet by accident.

⭐⭐ **The hand-kept-list problem is solved for the COMPARISON, by the
compiler.** `durable_families_that_differ` DESTRUCTURES `AmbitionGameSaveData`
rather than reading fields, so adding a fifteenth durable family fails to
compile until somebody classifies it. Poison-verified: removing one binding
gives `E0027: pattern does not mention field 'minted_items'`. ⚠ That fixes the
comparison, not the RESET — `reset_cut_rope_attempt_on_replay` still grows only
when somebody notices.

✔ **AND THE ONE RETRACTION MECHANISM WORKS END TO END — verified, not
assumed.** `a_replay_retracts_the_boss_defeat_a_gate_would_have_read` enters
`you_have_to_cut_the_rope`, reads the placement id the room actually authored
(`BossSpawn-105805`, LDtk-generated — a synthetic id would test nothing,
because the production reset is scoped to placements PRESENT in the room),
records the defeat the victory beat records, replays, and finds the gate's
question answering NO again.
⭐ **Asked through `boss.cleared` in the catalog, not through `data.bosses`.**
Reading the save row proves the row changed; asking the condition proves the
question A DOOR ASKS changes with it — the property that matters, and the one a
future refactor of the save shape could silently break. Poison-verified: making
`reset_cut_rope_attempt_on_replay` never run reddens it with
*"the replay rebuilt the boss but left `boss.cleared` answering YES"*.
⇒ **So the mechanism is sound and the RISK is its reach, not its correctness.**
It works for the one content domain that wrote it; nothing generic retracts
anything, and a second boss family that forgets to write its own reset gets no
warning from anywhere.

⭐⭐ **AND THE REACH IS WIDER THAN "A SECOND BOSS FAMILY" — measured 2026-09-05.**
Exactly ONE production system in the whole tree both reads a `RoomReplayAdmitted`
and retracts a durable fact: `game/ambition_content/src/bosses/cut_rope/mod.rs`,
family `boss`.
⭐ **RE-DERIVED with a repaired instrument and it came out CLEANER.** The first
pass matched TWO files and I dismissed the second by hand
(`crates/ambition_combat/src/rollback_registration.rs`, whose hits are message
NAMES inside string literals rather than writes). `durable_fact_writers.py` now
strips string literals and skips `#[cfg(test)]` items structurally, so that false
positive disappears on its own and the count is 1 with no caveat attached.
⚠ The same crude lens was ALSO wrong in the other direction — it truncated each
file at its first test module and under-counted `switch` writers 2 against 5. A
hand-dismissed false positive and a silent false negative came from one instrument.

```text
BossSpawn   11 authored placements   1 retracted on replay
Switch      14 authored placements   0 retracted on replay
encounters   generic writer          0 retracted on replay
flags        generic writer          0 retracted on replay
```

⇒ **A switch flipped during an attempt stays flipped after the replay that undid
the attempt**, so a puzzle room replays already solved. Nobody chose that; it is
the same absence one family over, and it means the ruling
[question 56](../awaiting-maintainer-decision.md) asks for should be stated for
durable ATTEMPT STATE generally. Answered only for bosses, it gets re-asked for
switches by whoever is holding that room rather than by whoever decided the
policy.
ⓘ This is the first open question below — *"which persistent occurrence states
are terminal, resettable, or recoverable?"* — reached from the replay end with a
measurement attached.

⚠ **AND THE DURABLE ARM'S FIRST RUN REPORTED A DEFECT THAT WAS NOT THERE**, which is
worth keeping because this file exists to catch exactly it. The disturbance
originally wrote `save.data_mut().wallet += 137` — a PROJECTION — which the
mirror overwrote from the live component on the next step. The test printed
`0 vs 137` and read precisely like a replay confiscating the player's coins.
⇒ Disturb the authority, not the projection; the instrument committed the
error the suite is written against.

## Open design questions — deliberately unresolved

- Which persistent occurrence states are terminal, resettable, or recoverable?
- How should persistent actor relocation outside authored home rooms be expressed?
- Which durable relationships require stable IDs across a fresh process?
- What exact peer barrier authorizes an external/P2P lifecycle commit once real
  transport exists?

## Related durable/current authorities

- ADR 0027 — GGRS and gameplay-session rollback authority.
- ADR 0030 — spawn provenance is data.
- [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md) —
  existence/residency/simulation/visibility policy.
- [`item-custody-and-accounting.md`](item-custody-and-accounting.md) — physical
  item occurrence/custody semantics.
- [`netcode.md`](netcode.md) — confirmed external effects and eventual P2P
  lifecycle coordination.
