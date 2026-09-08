# Checkpoint restoration: admission, preparation and commit

**Status:** target contract for A1; not implemented by this planning overlay.
**Source baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`.
**Scope:** restoration of one session's checkpoint into its single active room.
This does not choose multiplayer save ownership, add concurrent resident rooms,
or define arbitrary ECS transactions. The [queue](../queue.md) owns priority.
This page owns A1's execution and failure semantics; the
[frontier](actor-monolith-work-frontier.md) owns packet dependencies.

## Decision and the invariant being repaired

Session lifecycle owns a restore operation from request through terminal outcome.
A shrine owns interaction, healing and capture requests. Occurrence and item
owners own their snapshot data and reducers. Runtime composition selects those
domain contributions and orders their application; it does not implement them.

**One accepted restore must supply the destination, construction continuity,
subject restoration and item/occurrence restoration for the same operation.**
A refused request changes none of them. Preparation may inspect a candidate
checkpoint; it must not install that checkpoint into live resources in order to
read it. Completion means the room and every participating domain have passed
verification and the host has published the resulting simulation baseline.

This is stronger than checking admission in the room-routing function alone.
The current raw reset message has independent mutation consumers. Giving only
the routing function a better home would leave those consumers free to restore
state for a request that did not get the lifecycle slot.

## Current source facts and concrete edit sites

| Current source | Actual responsibility and required direction |
| --- | --- |
| `crates/ambition_platformer2d_actor_monolith/src/shrine.rs` | Healing/capture and unrelated startup/reset restoration share a file; move the latter, not the former |
| `crates/ambition_platformer2d_actor_monolith/src/session/lifecycle_commit.rs` | Owns the first-admitted-wins slot, originating frame and subject-preserving room intent; keep that authority here |
| `crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs` | Installs startup restoration; also restores custody directly from the raw reset message |
| `crates/ambition_platformer2d_actor_monolith/src/items/pickup/minted_horizon.rs` | Owns minted descriptions and owned-item baselines; owned-item restoration reads the raw reset message |
| `crates/ambition_platformer2d_shared_tangle/src/lifecycle/continuity.rs` | Restores occurrence state directly from the raw reset message |
| `crates/ambition_platformer2d_shared_tangle/src/lifecycle/horizon.rs` | Domain horizon channels/sets and registrations; currently describes raw requests as sufficient restoration authority |
| `crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs` | Adopts save data and emits a reset request; loading a save and restoring a live room must remain distinguishable |
| `crates/ambition_platformer2d_runtime/src/checkpoint_horizon.rs` | Installs domain offers and shared phase edges; currently puts both request routing and domain restoration into the restore set |
| `crates/ambition_platformer2d_runtime/src/room_transition/loading.rs` | Prepares construction against the live occurrence ledger; checkpoint preparation needs an explicitly selected candidate view |
| `crates/ambition_platformer2d_runtime/src/room_transition/commit.rs` | Common room application and eager commit gate |
| `crates/ambition_platformer2d_rollback_ggrs/src/lifecycle_commit.rs` | Confirmed-host gate, common application, baseline rebase; not a second checkpoint interpreter |

The startup function writes its routed latch before inspecting admission (F1).
More importantly, occurrence, owned-item and custody consumers read
`ResetToCheckpoint` independently of that admission result (F9). A system-set
ordering edge cannot repair this: the consumers need accepted operation data,
not a differently ordered read of the same unaccepted request.

Current room preparation computes an occurrence outlook from the live ledger and
uses it for cached-plan promotion and fresh construction. Redirect **both** to the
selected checkpoint view. Changing only fresh preparation would leave a prefetched
plan capable of reconstructing the wrong occurrence population.

## Three values, three lifetimes

The following names describe proposed types, not existing APIs.
<!-- cite-ok: proposed API vocabulary throughout the following schema -->

```text
CheckpointRequest
    why: StartupPlacement | Reset | ImportedSaveRestore
    session owner
    optional resolved primary subject

AcceptedCheckpointRestore
    operation key
    originating simulation frame
    existing structural room intent, when reconstruction is required
    primary subject identity, when this profile requires a subject
    pinned checkpoint revision and typed domain snapshots
    expected source room and active content binding

CheckpointRestoreOutcome
    operation key
    Committed | Cancelled(reason) | Failed(reason)
```

A request is not a second generic lifecycle queue. There is at most one
outstanding checkpoint request per session; repeated requests coalesce while it
is pending. The existing pending lifecycle slot remains the only admission
arbiter across checkpoint, door and replay requests. It is not moved to
`shared_tangle`, and no global lifecycle service is introduced.

The accepted value belongs in the session's pending-operation state. Its domain
snapshots are immutable values from their owners, not references to mutable
baseline resources and not a clone of `World`. Keep the common room intent's
subject-bearing versus bodyless distinction. Checkpoint continuity is an
additional **selected reconstruction input**, not an optional subject that makes
bodyless door crossings valid.

A concrete session-owned aggregate of supported checkpoint domains is acceptable:
this coordinator actually owns their consistency boundary. Use typed optional
members for optional capabilities, with presence established by composition. Do
not create a type-erased map of arbitrary snapshots, a save-every-component
protocol, or a restore-callback registry. The generic host can carry the aggregate
opaquely and invoke the session/domain operations through their existing typed
offers; it need not import every item component.

### Identity and idempotence

Use the existing active-session ownership stamp plus a session-owned admitted
operation sequence. If the current slot cannot distinguish repeated identical
intents, add that sequence to the slot, incrementing it **only on admission**.
Register the counter and accepted value for rollback. Do not reset the counter
at every room rebase; reject overflow rather than recycling a live identifier.

The operation key is not a replacement for payload validation. A host-side load
must match the key **and** the accepted intent, checkpoint revision, content
binding and source scope. Resimulation can reuse a sequence after rewinding its
allocation; a stale load for a different payload must not become authorized merely
because its integer matches. The host cache is derived, never the authority.

A frame number alone is insufficient: a room rebase restarts a rollback timeline.
Do not confuse rollback timeline generation, active session ownership, checkpoint
revision and content revision. Reuse the corresponding existing values rather
than spreading a new universal identifier through every crate.

An explicit standalone test/profile without `ActiveSessionScope` remains possible.
It has one declared lifetime and cannot retain loads across destruction/recreation.
Production session code must not treat an absent required scope as a wildcard
matching any scope, or use a missing subject as a request for bodyless replay.

## Request resolution and policy table

Resolve source data under one lifecycle owner before admission. A waiting request
may lack a subject while construction completes. Once the subject is resolved,
retain its SimId; do not re-resolve it from current control at commit time.

Pin the checkpoint and source-room preconditions on **successful admission**.
A busy slot can outlive a checkpoint capture; the request still waiting for
admission has not acquired the earlier checkpoint. An admitted operation must
not adopt a later checkpoint or newly controlled body while it waits for loading.

| Request | Target policy | Completion policy |
| --- | --- | --- |
| Startup, no checkpoint | Explicit no-op for this session | Mark startup satisfied; no room rebuild |
| Startup, checkpoint in current room, no durable reconstruction needed | Place the primary body once using existing transit semantics and zero velocity | Complete after the required body/motion inputs exist and placement is applied |
| Startup, valid checkpoint in another room | Existing subject-bearing transition to saved room/arrival | Complete only on matching committed outcome |
| Startup, saved destination missing | Preserve current diagnostic-and-skip policy in A1 | Record a terminal skipped reason, not an admitted route |
| Reset, valid checkpoint | Reconstitute from that checkpoint even when destination is the current room | Restore all installed domains in the same commit |
| Reset, absent/invalid checkpoint destination | Existing current-room authored-start fallback | An explicit start-state snapshot, not a fabricated saved checkpoint |
| Imported save requiring occurrence/item reconstruction | Adopt as candidate checkpoint data, then use the reset reconstruction road | Parsing/adopting a save is not successful live restoration |
| Bodyless world profile | Use the existing bodyless reconstitution intent only when selected explicitly by that profile | No attempt to manufacture or pick a primary body |

This packet preserves the primary-avatar restoration policy and all interacting-
body healing behavior. It does not resolve the pending product choice about
possession/save ownership. A checkpoint-only profile still has its declared
primary body; it simply omits held-item behavior and shrine entities.

Process imported-save readiness before startup placement resolution. A full
restore request subsumes a placement-only startup request for the same session;
there must not be both a local placement and a second room route for that save.
Repeated full reset requests coalesce. A new request after a terminal failure is
explicit; do not infer retry merely from an empty lifecycle slot.

## State machine and failure rules

| State / event | Required action | Forbidden action |
| --- | --- | --- |
| Waiting / subject or save adoption incomplete | Retain request and retry on subsequent simulation progress | Mark routed/completed, consume it forever, or default to another body |
| Waiting / another lifecycle intent owns slot | Preserve incumbent and checkpoint request; no restore writes | Restore ledgers/items before checking admission |
| Waiting / admitted | Store immutable selected snapshot and intent with operation identity | Retarget an existing load by editing its payload |
| Admitted / speculative rollback | Restore request/slot state from rollback; host cache remains derived | Treat host readiness as proof of confirmed admission |
| Admitted / preparation pending | Wait; keep selected inputs fixed | Swap live ledgers temporarily to prepare the room |
| Admitted / invalid preparation | Terminal failed outcome; discard candidate; retain current live state | Publish a partially prepared room or retry a permanent invalid definition every frame |
| Admitted / source scope, content or subject invalidated | Cancel the matching operation with a reason | Clear an unrelated newer slot or substitute a different subject |
| Commit authorized / transient precondition not ready | Retry without entering destructive application | Partially restore an item domain and return Retry |
| Apply started / trusted reducer or recipe violates contract | Contain failure; block gameplay/publication; report operation and failing domain | Claim the old world is intact, rerun arbitrary writes, or clear the failure and continue |
| Verified / published | Emit one terminal committed outcome and retire matching pending state | Publish before domain restoration or rebase before final verification |

Before destructive application, cancellation has no restoration side effects.
After destructive application begins, this contract promises fail-closed
publication, **not rollback of arbitrary Commands**. The stronger last-good-world
guarantee remains A10 and requires constrained inactive construction. Do not
implement that larger project as an undocumented prerequisite to A1.

The session coordinator is the sole writer of request progress/terminal outcome.
The lifecycle slot is the sole admission writer. Domains remain sole writers of
their own live data. A restored item is not simultaneously rebuilt by a live-ledger
path and a checkpoint candidate path.

## Capturing and preparing a coherent checkpoint

Checkpoint capture is an end-of-settlement operation. Keep healing and checkpoint
selection where they are, but stamp the committed capture with one revision.
Every installed domain contributes its snapshot under that revision. Capture
finishes only after all required contributions have been produced; a single
sequential schedule/barrier suffices. No general distributed consensus protocol
is needed inside one App.

Pin occurrence whereabouts, custody, owned-item counts and the minted definitions
needed to reconstruct those custody rows together. The candidate does not invent
persistence for dynamic objects that current policy does not preserve. Optional
item capability absence means "not participating," not "erase the item domain"
and not "install a dummy item provider."

A1's preparation API supplies a read-only continuity selection:

```text
Live continuity     -> current ledgers, for an ordinary door transition
Checkpoint continuity -> pinned restore snapshots, for a checkpoint reset
```

The same selection feeds occurrence outlook, cached-plan key/validation,
construction lowering, minted lookup and custody preflight. Adapt the existing
`OccurrenceContinuity` inputs; do not copy the room builder into a checkpoint
builder. A cache hit is valid only for the selected continuity fingerprint and
active content/scope constraints. A checkpoint change must invalidate a plan that
would produce a different population even when target room ID is unchanged.

Importing a save may populate candidate/baseline resources before any room is
active. Once a live world exists, imported values must not overwrite live
occurrence/custody state until its admitted restore commits. Distinguish
save-data-ready from world-restored in the existing durable-horizon completion
road; preserve its public load diagnostics rather than treating both as one flag.

## Commit ordering and visibility

Both hosts execute one shared restore application, with different authorization:

```text
collect/resume request in the simulation
    -> lifecycle admission + immutable selected checkpoint
    -> derived loading/preparation using selected continuity
    -> eager authorization OR matching confirmed rollback authorization
    -> revalidate source scope, content, subject and prepared plan
    -> apply checkpoint domain state and canonical room reconstruction
    -> flush declared construction work and reconcile restored relations
    -> verify room + domain postconditions
    -> publish readiness / install final frame-zero baseline
    -> terminal outcome and presentation notification
```

For a reconstruction restore, only the prefix through admission belongs in
ordinary speculative simulation. A same-room **startup placement only** remains
a small rollback-registered simulation operation with no room rebuild or host
rebase: the session owner runs it only when no conflicting lifecycle intent is
pending, at the existing nongameplay-gated placement phase, and records completion
after applying it. Do not manufacture a room-reconstruction intent just to move
an already constructed body. This exception does not authorize checkpoint ledger
or custody reconstruction from raw reset messages.
Do not restore live ledgers, spawn missing held objects, clear portals or reset
clocks merely because an unconfirmed reset was requested. The confirmed host
must select the accepted restore from the same confirmed authority as the room
intent, not combine a confirmed intent with a later speculative baseline resource.

At commit, make the selected occurrence/accounting values visible before the
canonical construction consumers that need them. Restore/materialize custody
against the resulting stable subject and item identities after queued structural
work is applied, then verify it before publication. Preserve the current body
carryover/One Body One Path route. Do not spawn a second primary body to avoid
restoring an existing one's state.

Use a typed, temporary restore context around a dedicated domain restore schedule
or direct typed domain operations in the existing common commit executor. Its
inputs are prepared; it is not a persistent event queue. The executor installs
and removes that context on every success, retry and error path. Domain systems
must be impossible to invoke as an effective restore with no authorized context.
There is no arbitrary callback registration by strings or TypeIds.

Express the required deferred-command flushes in this common path. Tests must
observe post-flush state, not just queued commands. Neither a Bevy set name nor a
message emission establishes that newly constructed entities already exist.
The rollback host rebases only after restoration and verification; otherwise its
first restore would undo the just-restored checkpoint. Frame-zero restoration
must reproduce occurrence, custody and body state, not only room geometry.

### Existing replay consumers

Audit the current `RoomReplayAdmitted` readers when moving the checkpoint branch:

- `crates/ambition_platformer2d_runtime/src/sandbox_reset.rs`, especially
  `return_the_replay_subject_to_spawn`;
- `game/ambition_content/src/portal/reset_adapter.rs`;
- `game/ambition_content/src/bosses/cut_rope/mod.rs` and
  `game/ambition_content/src/bosses/cut_rope/arena.rs`.

Admission notifications may describe admission. They cannot double as the
checkpoint commit token. Move checkpoint-caused restore/clear/reset mutations
into the committed domain application, or publish the corresponding committed
fact for non-authoritative presentation afterward. Do not emit both old and new
mutation triggers for one reset. The ordinary non-checkpoint replay road keeps
its existing policy until separately migrated; reuse its reducers where correct,
not its premature trigger. Do not locate content-specific algorithms in runtime.

## Implementation sequence

### A1a: repair the startup latch and establish the refusal witness — LANDED

**Was wrong:** `restore_checkpoint_on_session_start` wrote `routed_for` before
asking the slot and discarded the returned `Admission`, so a refused crossing
still spent the session's one resume and the player stayed in whichever room the
session opened in. **Fixed by** latching the generation only when
`Admission::admitted()`. **Guards:**
`a_refused_slot_leaves_the_checkpoint_resume_retryable` (poisoned
2026-09-08: forcing the latch unconditionally reddens it) and
`a_resume_with_no_constructed_subject_stays_pending_until_the_body_exists`;
once-only routing stays pinned by the existing
`a_checkpoint_in_another_room_of_this_world_routes_the_session_there`.

**Standing prohibition:** nothing may write checkpoint-resume progress, or any
other consequence of a lifecycle request, before the slot has said yes. The
`#[must_use]` on `record` is the reminder, not the enforcement.

F9 is NOT closed by this and cannot be — see the measured witness below.

#### F9, measured 2026-09-08 (executed, full checkpoint-horizon composition)

`game/ambition_app/tests/death_restores_the_checkpoint.rs::`
`f9_a_refused_reset_still_restores_the_domains_that_read_the_raw_request`.
One session banks a checkpoint, then acquires a stackable entitlement and picks
up an authored ground item; an unrelated intent holds the lifecycle slot; a raw
`ResetToCheckpoint` is written. On the single tick that follows, with the
incumbent intent still in the slot and the active room unchanged:

| Value | On a refused reset | What A1c owes |
| --- | --- | --- |
| `OwnedItems` count of the stackable entitlement | rolled back to the banked count | unchanged |
| The authored object acquired after the checkpoint | **zero live occurrences — destroyed, not returned** | still in the hand |
| `PendingLifecycleCommit` | incumbent retained (correct today) | unchanged |
| Active room | unchanged (correct today) | unchanged |

⛔ **The entity loss is the sharpest finding and it is worse than a rolled-back
ledger.** `restore_custody_to_checkpoint` takes the object out of the hand
because the banked custody relation did not have it there; the road that would
put it back on its pedestal is the room reconstruction the reset asked for — and
that is exactly what the slot refused. The two halves of one restore ran on
opposite sides of an admission neither consulted, and the object survives in
neither. A control arm in the same fixture runs the identical request with the
slot free and gets the object back, so the difference is the admission alone.

⇒ This is why an ordering edge cannot repair F9: the consumers need accepted
operation data, not a differently ordered read of the same unaccepted request.

### A1b: perform the ownership move without semantic changes — LANDED

`CheckpointResumeProgress`, `restore_checkpoint_on_session_start`,
`resume_at_checkpoint_on_reset` and their tests now live in
`crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs`, with the
rollback registration following the type and its wire key
(`resource.checkpoint_resume_progress`) unchanged.
`ActorCheckpointHorizonPlugin` composes two named offers —
`SessionCheckpointHorizonPlugin` and the existing item one — instead of
installing the session's reset resume inline.

**What was wrong:** `ItemPickupSimulationPlugin` initialized the resume state and
installed the startup resume, so a composition's ability to return to its
checkpoint depended on it having held items.
`a_checkpoint_only_composition_resumes_without_the_item_domain` is the guard: it
runs the real offer through the real sim schedule with no held-item plugin and no
shrine entity.

**Scheduling preserved verbatim, and the guard is explicit about it.** The resume
kept `ItemPickupSet::CoreHeldItems`, `.after(heal_save_shrine_system)` and
`.before(HeldItemStep::Release)`.
`the_session_checkpoint_resume_keeps_the_edges_it_was_carved_from` composes both
plugins and counts the attachments, because the item-only fixture beside it now
correctly sees one. Poison-verified 2026-09-08: dropping the set membership
reddens it.

⚠ **THOSE EDGES ARE INHERITED, NOT DERIVED.** Nothing establishes that a body
placement belongs among held-item steps; the resume's own doc says what it
actually needs (the first tick a constructed session has a body, ungated by
`gameplay_allowed`). `CoreHeldItems` is nested in `PlayerSimulation` by its
owner, so dropping the membership would also move the phase. A1c re-derives the
ordering as part of the accepted-restore road; until then the contract is "the
move changed no order".

**Channel ownership is unchanged and deliberate:** `ResetToCheckpoint` and
`RoomReplayAdmitted` are registered by the host's `CheckpointHorizonPlugin`, not
by a domain offer. A domain plugin that registered them would be a second owner
of the channel.

### A1c: make the accepted restore the common commit input

This is a coherent behavior change, implemented in buildable subcommits:

1. **LANDED 2026-09-08 — the admitted operation exists and is the only trigger.**
   `AdmittedCheckpointRestore` (shared_tangle lifecycle vocabulary, written only
   by the session coordinator) carries the accepted operation's frame and its
   resolved subject. `CheckpointRestore` gained an ordered
   `CheckpointRestoreStep::{Admit, Apply, Retire}` chain, owned by the session
   offer because the session is the admission authority. `ResetToCheckpoint` is
   now remembered in a session-owned `OutstandingCheckpointRequest` instead of
   being drained and lost: a reset asked for while another intent owns the slot
   is re-asked until it is admitted. Both are rollback state
   (`GGRS_ROLLBACK_SCHEMA_VERSION` 167 -> 168, baseline + absence contract
   updated in the same commit).

   ⚠ **The pinned snapshot aggregate is deliberately NOT here yet.** Admit and
   Apply run in one frame, so the live baselines *are* the pinned ones and an
   aggregate would be a value with no consumer — the shape that produced the
   four dead `LifecycleIntent` variants. It arrives in subcommit 3/4 with the
   deferred application that makes "capture changes while load waits" a
   reachable case, and that is the subcommit that owes the pinning test.

   ⛔⛔ **AND IT DOES NOT GO IN THE SHARED TOKEN.**
   `shared_tangle::AdmittedCheckpointRestore` is an authorization view: which
   operation, and whose. The pinned snapshots belong to the session's own
   accepted-operation state, because the session coordinator owns their
   consistency boundary and may name item and occurrence types `shared_tangle`
   must never depend on. Putting them in the shared token would make it the
   checkpoint coordinator under a vocabulary type's name, and every domain would
   then read the coordinator instead of its own owner's value. Guarded by
   `the_admitted_restore_carries_only_which_operation_and_whose`, whose
   exhaustive destructure stops compiling the moment a field is added.

   The token IS registered rollback state (`resource.admitted_checkpoint_restore`,
   schema v168) even though it is same-frame today: its lifetime changes in 3-5,
   and the registration must not be what someone has to remember.
   `the_admitted_restore_does_not_yet_outlive_its_own_frame` records the current
   shape so that change is visible rather than assumed.

   Still open in this subcommit: the coarse startup routed/completed flags
   (`CheckpointResumeProgress`) are untouched, and the same-room startup
   placement is still its own road.
2. **LANDED 2026-09-08 — no domain reads the raw request.**
   `restore_occurrence_baseline`, `restore_custody_to_checkpoint` and
   `restore_owned_items_to_checkpoint` read the admitted operation. The
   occurrence and entitlement reducers are separated from their triggers
   (`reduce_occurrences_to_baseline`, `reduce_owned_items_to_baseline`) so a
   domain test can hand them a snapshot; the custody reducer stays a system
   because it spawns and queries, and its domain test drives it through an
   admitted operation rather than a fabricated message. There is no public
   bypass around admission: the token has one writer.

   The registration is guarded, not just the source:
   `every_checkpoint_restore_system_is_inside_one_ordered_step` asks the SCHEDULE
   which set each reducer joined and fails if anything sits in
   `CheckpointRestore` outside the three steps — a fourth domain installing the
   old way is caught there rather than by a player losing an item.
3. **LANDED 2026-09-08 (preparation half) — preparation reads the operation's
   own population.** The session owns `AcceptedCheckpointRestore`: the accepted
   operation, the room intent it was admitted for, and the occurrence/minted
   inputs pinned when the slot said yes. It outlives its frame and is retired
   when the slot gives up the intent. Room loading derives BOTH the prefetch
   cache key and the fresh plan's `OccurrenceContinuity` from it, matched by
   intent so a door recorded while a restore is outstanding is still prepared
   from live state. Rollback-registered; wire format 168 -> 169.

   ⛔⛔ **WHAT THIS REPLACED WAS A PHASE-ORDERING ACCIDENT, measured 2026-09-08.**
   Preparation derived the destination outlook from the LIVE ledger. On a
   checkpoint reset that was the right answer only because
   `restore_occurrence_baseline` had overwritten the live ledger with the
   checkpoint's earlier in the same frame: `CheckpointRestore` sits in
   `PlayerInput`, room-transition readiness runs after `RoomTransitionSet::Detect`
   in `RoomTransition`, and the phase order puts one before the other. The room
   was prepared from a live resource swapped to the checkpoint value **in order
   to be read** — the preparation shape this document forbids — and nothing
   stated that ordering as a checkpoint requirement.

   ⚠ **AND THE SWAP HAD BECOME THE ONLY WITNESS OF ITS OWN REDUCER.** With
   `restore_occurrence_baseline`'s write disabled, **no test in the 595-test
   `app_it` suite reddens.** Every observable consequence of that write ran
   through room preparation, which no longer reads it. An attempt to add a
   direct witness failed its own poison: `AuthoredOccurrences` is largely
   RE-DERIVED from live state (`project_custody_onto_authored_occurrences`,
   `record_placed_ground_items`), so after the rebuild it converges to the
   banked value whether or not the reducer wrote it. ⇒ **The rows that need the
   reducer are the ones the rebuilt world cannot republish** — `Consumed`, and
   whereabouts in a room that is not the one being rebuilt. Subcommit 4 moves
   this reducer to the commit boundary and owes a witness built on one of those
   rows; a fixture whose poison passes is a finding about the fixture.

   Still open in this subcommit: the same selected input is not yet integrated
   into eager/confirmed commit execution, and custody/entitlement snapshots are
   still applied from live baselines in the restore set.
4. Move checkpoint replay consequences to that commit boundary, perform explicit
   flush/reconciliation/verification and publish the final baseline. Add failure
   cleanup for the temporary context and match outcomes to the original request.
5. Remove obsolete raw restore readers, redundant checkpoint mirrors, old item
   installer aliases and unused progress paths. Refresh the graph as a diagnostic.

Subcommits 2-4 may need one integration commit to remain buildable. Do not ship a
mixed mode where some installed domains use the candidate and others still act
on raw requests. This packet does not create a new crate, a universal checkpoint
registry or a second room-construction algorithm.

## Executable acceptance matrix

Each row is a test obligation, not a claim that this review ran it. Prefer the
existing production harnesses and domain test modules to a new test framework.

| Fixture | Required observable assertions |
| --- | --- |
| Busy slot plus different live and saved ledgers | Incumbent slot unchanged; no occurrence, custody, owned count, entity, subject-position, clock or portal mutation from the refused reset. ⛔ The measured failure is entity ANNIHILATION, not merely a rolled-back ledger: assert the acquired object is still live and still held, not only that a count matches. Invert the A1a-era witness in `death_restores_the_checkpoint.rs` rather than writing a second fixture |
| Busy slot released in same session | Waiting startup/reset can be admitted once; no false routed/completed latch |
| Two raw reset requests in one tick | One accepted operation, one reconstruction, one set of consequences |
| Missing primary during construction | Request remains pending; no bodyless fallback; later materialization satisfies it |
| Control changes after admission | The original primary SimId remains the restore subject |
| Capture changes while load waits | Admitted snapshot stays fixed; a later operation can use the newer capture |
| Same room, different checkpoint occurrence outlook | Prefetched live plan cannot suppress/recreate the wrong item; candidate plan is used |
| No checkpoint / invalid saved destination | Explicit current-start fallback for reset; documented startup skip behavior preserved |
| Save adoption plus startup in one lifetime | No duplicate placement/route; data-ready and world-restored diagnostics are distinct |
| Prepare failure / cancellation | Current live data unchanged; matching candidate retired, unrelated pending intent retained |
| Trusted failure after destructive apply | No readiness publication or continued normal simulation; no claim of old-world retention |
| Held item, thrown persistent item, minted carried item | One identity per restored occurrence, correct custody/counts and no double materialization |
| Restore followed by frame-zero rollback | Same authoritative room, subject, ledgers and custody after restore |
| Eager versus confirmed host | Equivalent final state for identical accepted snapshot, with no speculative destructive application |
| Checkpoint-only composition | Startup and reset work without held-item plugin or shrine entity |
| Domain-only reducer tests | Snapshot reduction works without session routing; production admission cannot be bypassed through that test entry |
| Teardown and re-entry | Old request/load/context cannot act in the new session |

Existing integration witnesses include
`game/ambition_app/tests/canonical_reconstitution.rs`,
`game/ambition_app/tests/death_restores_the_checkpoint.rs` and
`game/ambition_app/tests/carried_item_crosses_rooms.rs`. Use their actual registered
integration target from the current harness; a zero-test filter is not a pass.
Run the focused checkpoint/shrine tests, the corrected spawn-boundary check and
the established rollback lifecycle suite before reporting A1 complete.

A1 is complete when the code demonstrates **one selected restore, one commit
boundary and domain-owned reducers**, not when the shrine/session edge disappears.
