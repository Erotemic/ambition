# Construction and reconstitution

**State:** OPEN — the four rebuild paths agree on the population they produce;
durable restore reaches that agreement by building and then correcting, which is
a shape worth removing rather than a defect.

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

`rooms::reconstitute_the_active_room` names a room; it does not build one. A
replay records a `LifecycleIntent::Transition` naming the ACTIVE room, and the
room-transition road prepares, authorizes, commits and rebases it — the road a
door takes, and the road a checkpoint resume already took. The in-session rebuild
paths differ in three values and in nothing else:

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

### Provenance/lifetime vocabulary exists

`SessionScopeId`, `SessionRoot`, spawn provenance/lifetime components, occurrence
records and room/session scope helpers already provide the vocabulary needed to
decide which populations a lifecycle operation may retire or reconstruct.

## What the hand-kept ledger cost

`reset_ecs_room_features` was a second, incomplete room constructor: sixteen
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

Done. `reset_ecs_room_features` is deleted; `RoomTransitionSet::Reset` now runs
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

### C3 — make durable restore consume facts, not ECS snapshots — MEASURED

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

**Measured 2026-08-30: the correction lands on the same population.** A
fresh-process load of an empty file builds the room a session that never loaded
anything has, and a load carrying a relocated occurrence suppresses it exactly as
walking back into the room does — cases 7 and 8 of the acceptance suite, both red
under a poisoned `outlook_for`. So this is a SHAPE, not a defect: a load
constructs a room it is about to replace.

Worth removing when it buys something concrete (a load currently costs an extra
room construction, and the window between the two is where a fact re-derived
from live state can overwrite a restored one). Not worth removing for tidiness.

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
| `a_replay_reconstructs_the_room_without_retiring_the_body_playing_it` | the body playing the room is not swept with it, and arrives at the room's spawn |
| `an_object_in_your_hands_survives_a_replay_and_is_not_re_authored` | BOTH retention legs: `RoomResident` and the durable-fact input |
| `a_replay_does_not_adopt_what_the_attempt_created` | the attempt-residue leg |
| `loading_a_save_builds_the_room_a_re_entry_builds` | the durable road produces the population the in-session roads produce |
| `a_relocated_occurrence_is_suppressed_by_a_load_and_by_a_re_entry_alike` | a durable fact construction must ACT on reaches both roads alike |
| `running_one_lifecycle_path_then_another_lands_in_the_same_room` | repeated rebuilds do not drift |

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
