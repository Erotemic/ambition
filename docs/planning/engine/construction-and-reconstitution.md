# Construction and reconstitution

**State:** typed construction and lifecycle convergence is in place, and A10
candidate publication is closed. C1-C4 are convergence receipts. The checkpoint
contract is [A1](checkpoint-restoration-protocol.md). C5 remains
transport-specific.

## Goal

Construct the same authoritative population through one domain-owned path:

```text
prepared generation + pinned durable facts + accepted lifecycle policy
    -> typed domain plans -> verified candidate -> publication
    -> admitted live population + rollback baseline
```

New session, crossing, replay, checkpoint restore and development reconstruction
have different authorization and retention policies. They must not grow separate
semantic constructors. Domain state stays with its owner; the lifecycle coordinator
selects inputs and ordering rather than implementing body/item/world behavior.

## Current source and limits

Baseline inspected: `d81a7ae1d2db1fc5caa49efc807a39ea6b1ca266`.
No Rust or runtime acceptance was executed in this planning review.

| Source | Established interface or responsibility |
| --- | --- |
| Shared `construction/mod.rs` | Typed `ConstructionDomain`, plans, receipts, relationships and roster verification |
| `ConstructionRootCtx` / `RootScope` in that module | What a recipe receives; root-bound inserts only, no `Commands` and no `spawn` |
| Runtime `room_transition/` | Preparation, readiness, commitment and prefetch identity |
| Runtime `session_world.rs` | Immutable prepared source is distinct from live session selection |
| Actor monolith `session/checkpoint.rs` | `AcceptedCheckpointRestore` pins selected inputs and operation identity |
| Shared `lifecycle/horizon.rs` and `lifecycle/continuity.rs` | Occurrence restoration consumes explicit checkpoint inputs |
| Actor monolith `session/durable_horizon.rs` | Durable occurrence facts enter activation before population construction |

These interfaces are useful foundations. None is a generic transaction engine for
arbitrary Bevy plugins. [Immutable content](immutable-content-and-transactional-construction.md)
states the exact current and target guarantees.

## Authorization before consequences

A reset, replay, transition or reload request is an ask. No domain changes state
merely because the ask exists. Session admission selects the subject, destination,
content generation, pinned facts and operation key. Consumers use that accepted
value. Rejection terminalizes the operation once; it cannot leave an automatic
per-frame retry loop.

For checkpoint work, use the complete [checkpoint protocol](checkpoint-restoration-protocol.md).
Its accepted selected snapshot drives preparation and application. Do not read a
live mutable ledger in prefetch and a different ledger in commit. Rest-point
interaction/healing/capture remain content responsibilities; restoration progress
belongs to session lifecycle, including startup before normal gameplay.

Keep same-session replay distinct from a new gameplay session. A timeline rebase
preserves that session's rollback-health diagnosis. A new generation cannot be
used to hide an unhealthy timeline.

## Retention is explicit policy

| Operation | Population policy | State policy |
| --- | --- | --- |
| New session | Construct the selected starting population | Adopt admitted save facts or explicit new-game values |
| Room crossing | Retire departed room residents; retain eligible controlled/carried objects | Preserve domain occurrence/custody and progress under their contracts |
| Same-room replay | Reconstruct that room through the same constructors | Retract attempt state; retain session progress where its owner requires it |
| Checkpoint restore | Reconstruct from the accepted checkpoint's pinned inputs | Restore the selected domain baselines, not arbitrary current globals |
| Development generation change | Reconstruct the selected pinned scenario against the admitted new generation | Validate state mapping; reject unsupported migration without altering the durable save |

Do not classify an object from the constructor that created it. Use provenance,
attachment, custody and declared lifetime. A possession target is not necessarily
the home avatar. Human/AI/controller identity cannot select a second body path.
A relationship survives only when both endpoint authorities can be restored.

Room residents, carried objects, authored occurrences and dynamic effects do not
all share one deletion rule. The existing custody/occurrence authorities decide
what persists. A no-record state can mean 'as authored' under the owning domain;
it is not a general instruction to remint every dynamic spawn on re-entry.

## Construction phases and visibility

Domain plans resolve typed parameters and references. Authoring adapters such as
LDtk lower into those values; the generic geometry owner does not import character
catalogs to answer placements. No universal recipe enum or string/TypeId executable
registry is introduced.

| Phase | Required behavior |
| --- | --- |
| Prepare | Resolve immutable content and pinned continuity; determine exact intended population |
| Admit | Check installed domain support, source identity, scope and lifecycle authorization |
| Draft | Produce typed candidate state and relationship/resource changes without live mutation |
| Verify | Check the supported candidate's invariants and full relationship closure |
| Publish | Materialize/select at the declared visibility barrier; establish the timeline baseline |
| Retire | Release only obsolete owners, instances and attempt resources |

Room and session construction run these stages through the A10 candidate
bracket below. Plugins outside that bracket keep explicit fail-stop behavior on
post-commit defects.

Prepared values may share immutable memory. Runtime callbacks, live resources and
registered state are not portable artifact payloads. Candidate building must not
call a publishing function and later attempt to undo it.

## A10 - candidate world publication

**State:** closed at room and session scope. A failed candidate world leaves the
playable world N intact and unchanged. The queue's A10 row lists the acceptance
arms. [Generation/reload](content-generation-and-reload.md) owns the candidate
seal and activation state machine; this section states the construction model that
all roads now use.

### Ownership model

```text
CANDIDATE SESSION                          CANDIDATE ROOM
SessionRoot + InactiveCandidate            every root minted InactiveCandidate
  + SessionScopedEntity(scope) on every      and stamped with the lane
    entity spawned through                   TransactionId
    SessionSpawnScope::candidate(scope)
  + ActiveContentBinding on the root       + PendingWorldReplacement,
  + the world bundle on the root             FrozenPublicationEffects and
  + process projections held as data in      CustodyHandoffs on the
    CandidateSessionPublication              RoomPublication entity
```

- **Visibility.** `InactiveCandidate` is a registered Bevy disabling component and
  is `pub(crate)` to `shared_tangle`. `SessionSpawnScope::apply_to` applies it to
  every entity a candidate session spawns. Hiding the root alone is not enough:
  disabling components do not inherit through ownership.
  `a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction`
  pins this. Two facts depend on it: the session root's `SimId` is a constant
  (`SimId::singleton("session", "root")`), which is sound only because one root is
  visible; and the session-activation reset's rollback waiver holds because the
  incoming root is hidden until adoption.
- **Staged state lives on the publication**, not in a process global. A domain
  that needs candidate-owned state holds it as a component on the
  `RoomPublication` entity. Publication adopts it; both refusal arms remove it
  explicitly (the publication entity has no `TransactionId`, so
  `retire_candidate` does not reach it).
- **Supersession** is a baseline declaration: `TransactionBaseline::superseding`
  states that the live body stands until publication. `PublicationEffects`
  carries `supersedes` (with a `DepartureAuthority` of `Publication` or
  `Custodian`), `retires` and `owners`. Custody handoffs record the holder and
  item spec when the supersession is declared, and publication drains them on
  every path.
- **What a candidate session owns:** room state, geometry, moving-platform state,
  content binding, prepared content, `SessionMechanics`, the player's mechanical
  transition state, construction roots, its minted durable horizon
  (`CandidateDurableHorizon::minted`) and its scope reservation. These are
  installed by adoption. `PlatformerSessionBuilder` holds no live durable
  resource, so candidate construction cannot read one.

### Verification

`project_post_publication_roster` builds live − declared retirements − superseded
live bodies + this publication's owned candidates. `verify_projected_roster`
checks: one authoritative occupant per identity (a declared deferred pair of two
is admitted, a third holder is not); no unowned candidate; every declared
supersession has both participants; and the roster invariants of
`verify_committed_roster`. `verify_staged_world` checks the non-entity world the
publication would install, and `apply_world_replacement` applies to the same
entity that was verified. The commit boundary compares the plan's generation with
the `ActiveContentBinding` on the target root.

There is no projected relation-endpoint check, because it would be vacuous:
`ConstructionPlan::commit` resolves both endpoints of every planned relation from
its own receipt, and any other endpoint is `unreachable!`. If relations ever gain a
live endpoint, that check becomes required.

`stage::construct_room_candidate` is one exclusive-world command that consults
`opening_refused` and builds nothing when the opening refused. A refusal that has
to be repaired afterwards is not a refusal: hooks and observers run during command
application.

### Publication boundary

One authority per level, in fixed order, inside one exclusive-world call:

```text
ROOM      publish_candidate -> apply_world_replacement -> retire_superseded
SESSION   install the aggregate's projections -> publish_candidate_session
CALLERS   every effect that means the operation happened is queued behind
          publication_succeeded(P)
```

- Each publication primitive has one production call site.
  `only-the-publication-authority-publishes-a-candidate` and
  `only-the-candidate-builder-hides-a-root` in `check_absence_contracts.py` keep it
  so. `apply_world_replacement` is private to its file.
- A room published inside a pending candidate session freezes the effects it owes
  the world outside its population (`FrozenPublicationEffects`), and
  `PreparedCandidateSession::adopt` finalizes them. `RoomLoaded` is such an effect:
  it voids checksummed pending hits.
- A road's effects include everything the system does, not only the writes inside
  its transaction. Developer status messages and rollback restarts are effects.
  Audit the whole system when you add a road.
- There is one activation road: an activation with no prepared candidate panics.

### Candidate session exits

A candidate session has exactly four exits:

```text
ADOPTED      publish_candidate_session   (the gate said Admit)
REFUSED      discard_candidate_session   (the gate said Refuse)
SUPERSEDED   discard_candidate_session   (a later pending route replaced it)
ABANDONED    discard_candidate_session   (ShellCommand::CancelPending ended it)
```

`discard_abandoned_candidate` handles the last two at the head of the preparer.
The slot is emptied by a discard, never by an assignment. Each exit owes four
releases: the candidate's entities, its publication receipt, its scope reservation
(`ReservedGameplayScopes::release`), and its gate registration, route hold first
and evaluator second. If you add an exit, check all four.

When a lifecycle moves from a call bracket into a resource that spans frames,
enumerate its exits: the compiler no longer shows them. `construction::outstanding_candidates`
and `rooms::outstanding_publications` count leftovers; arms assert both are 0
after a cancelled candidate and after a committed reload, with a positive control
while the candidate stands.

For each `debug_assert`, read the line after it:

```text
unreachable, or a contract the types enforce   -> debug_assert
control continues and picks an answer          -> log it as well (keep the assert)
control continues into data loss               -> handle it: discard, refuse, error
```

### Open items beside A10

- **Refused door feedback (design question).** A refused room transition cancels
  cleanly on both hosts, but the player gets no signal; only a `warn!` and a
  `room_commit_refused` world-log line. Ask the maintainer what a player should
  see (nothing, a diegetic refusal, or a developer-only overlay) before building
  anything. Refusals do not occur on shipped roads today.
- **Save-described mint fixture.** The positive half of "a candidate's first room
  rebuilds a save-described mint" has no witness. It needs a save whose ledger,
  custody and minted rows describe a mint the start room reinstates.
- `DepartureAuthority::Custodian` remains a defensive fallback for a hand the world
  cannot resolve; the production witness asserts `left_to_custodian == 0`.
- The first room of the shipped app cannot be made to refuse (its binding is
  written from its own plan), so session-scope refusal is witnessed through a
  handoff instead.

Out of scope: peer-stable identity (ID-PEER), general native-plugin undo and
arbitrary schema migration. Unexpected native panic, allocator exhaustion or unsafe
plugin mutation remains an engine fault; do not convert it into a successful
refusal. If a fallback reconstructs the pinned old scenario, label it recovered,
not unchanged.

## Rollback, persistence and multiple rooms

The existing confirmed room transition starts a new baseline; snapshots do not
silently cross that boundary. Eager and rollback hosts use the same construction
semantics but different authorization. A local synchronous test is not proof of
an external peer barrier.

A8 extends scoped population semantics to two simultaneous instances. Membership
changes must retain unaffected instances while establishing a coherent session
baseline. Do not implement this by clearing every room or creating a separate
simulation for each local camera. The one-instance profile uses the same path.

Durable save data is product state, not opaque ECS snapshots. Item/wallet restore
may need an existing body; that does not justify creating an incorrect room first
and repairing its authoritative occurrence population later. Adopt the pinned
facts before the constructor whose population they control.

Do not assert whole-save equality across replay: session progression may correctly
survive while attempt-local facts retract. Tests compare each owner's declared
retention semantics and the later door/dialogue/combat observations that consume
those facts, not only final entity counts.

## Existing convergence receipts

These describe earlier work recorded in the repository, not tests rerun here.

| Row | Established direction and standing constraint |
| --- | --- |
| C1 | Retention classes are explicit at lifecycle boundaries, including room residence and custody |
| C2 | Replay consumes canonical construction; acceptance follows the actually controlled body and excludes duplicate carried objects |
| C3 | The saved occurrence ledger is adopted at activation before initial construction; keep the every-frame interim-population witness |
| C4 | Eager/headless and confirmed rollback hosts consume the same prepared construction semantics |
| C5 | External/P2P lifecycle coordination remains a real-transport task; do not substitute local sync testing for it |

The earlier claim that raw `ResetToCheckpoint` remained the current restoration
input is superseded by A1's accepted/pinned inputs in the inspected source. The
[A1 matrix](checkpoint-restoration-protocol.md) owns its remaining witness limits.
Do not infer that all construction failures became reversible when A1 landed.

## Existing test entry points and new evidence

Start in `game/ambition_app/tests/canonical_reconstitution.rs` and
`game/ambition_app/tests/rollback_lifecycle_reset.rs`. Reuse the existing app_it
module registration rather than create another integration executable.

Useful controls include the nonempty population premise, room leave/return, same
room replay, actually controlled body, carried object retention, relocated authored
occurrence, replayed attempt residue, and session progress. Preserve
`a_load_never_authors_the_occurrence_it_is_about_to_suppress` as the startup control.
These names/locales must be checked on the implementation head before invocation.

New FI4 adds supported candidate isolation and repeated reload. New FI9 adds duplicate
room definitions, independent teardown and active/dormant work accounting. Poison
one obligation at a time: early retirement, stale pin, missing occurrence input,
foreign resource mutation, wrong scope or premature effect. An empty census or an
endpoint-only check cannot prove the intermediate population was safe.

## Exit

Every supported lifecycle path uses one set of domain constructors, pinned accepted
inputs, explicit retention, coherent publication and correct replay ownership.
Reload safety is demonstrated for the migrated path. General native-plugin undo,
arbitrary schema migration and peer coordination are not implied by that result.
