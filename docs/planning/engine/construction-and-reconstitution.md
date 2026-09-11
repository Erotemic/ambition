# Construction and reconstitution

**State:** existing typed construction and lifecycle convergence, with bounded
candidate isolation still planned. C1-C4 are historical population-convergence
receipts. The current checkpoint contract is [A1](checkpoint-restoration-protocol.md),
not the earlier raw-reset diagnosis. C5 remains transport-specific. A10 now has a
concrete customer in I3's repeatable development reload.

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
| `ConstructionExecCtx` in that module | Currently exposes raw Bevy Commands, so verification cannot undo arbitrary mutations |
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

The existing source combines some of these stages and performs roster verification
after commit. Do not rename that path and claim the stronger ordering already
exists. A10 splits the selected migrated construction path. Unmigrated raw-Commands
plugins retain explicit fail-stop behavior on post-commit defects.

Prepared values may share immutable memory. Runtime callbacks, live resources and
registered state are not portable artifact payloads. Candidate building must not
call a publishing function and later attempt to undo it.

## A10 - bounded safe candidate materialization

The required customer is the repeated content-edit loop. The relevant target is
one supported scenario/room construction, not undo of any possible native action.
Implement with [generation/reload](content-generation-and-reload.md), which owns
the candidate seal and activation state machine.

1. Inventory the selected constructor's parameters, services, entity creation,
   resource writes, emitted messages, hooks/observers and referenced live entities.
   Classify each as candidate data, retained-live input, or publication effect.
2. Factor preparation into typed domain-owned drafts. Give it immutable services
   and explicit pinned inputs. No raw Commands or mutable World access to active
   state during this stage. Move an existing policy; do not write a second one.
3. Validate identity uniqueness, exact population, relationships, component/value
   requirements, retained references and resource deltas. Candidate failure ends
   the attempt without retiring the old scene or selecting new definitions.
4. Constrain the materializer to the verified draft. Discover no new assets,
   capabilities or fallible external requirements after retirement begins. Audit
   hooks and observers as part of the materializer, not as invisible implementation.
5. Publish through the existing lifecycle coordinator, with normal consumers
   excluded from intermediate visibility. Install the selected generation and
   new baseline coherently. Then release old references and attempt resources.
6. Test malformed candidates after metadata admission, not only parse errors.
   [FI4](fast-iteration-acceptance.md) specifies the live-scene and failure assertions.

Default to inactive typed data, not arbitrary World cloning. An alternative
same-World staging population must prove isolation from every relevant query,
observer, hook and resource write. A marker or a paused fixed schedule alone is
not proof. A separate World needs explicit resource/entity transfer and does not
magically preserve arbitrary plugin state either.

Unexpected native panic, allocator exhaustion or unsafe plugin mutation remains
an engine fault. This boundary does not sandbox Rust. Do not convert such faults
into a successful refusal by printing a warning. If a fallback reconstructs the
pinned old scenario, label it recovered rather than unchanged. If recovery is
unavailable, stop normal simulation and report failure.

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
| C3 | `758e9df37` adopted the saved occurrence ledger at activation before initial construction; keep the every-frame interim-population witness |
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
