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

### What the ROOM scope delivered, and what it measured — 2026-09-14

Steps 1-5 are implemented for the room; step 6's assertions are in place. The
SESSION scope has none of them.

```text
1 inventory           ConstructionPlan rows + PublicationEffects: additions,
                      supersessions, retirements, retained-by-omission
2 typed drafts        RootScope / RelationScope; no raw Commands or World
3 validate            verify_committed_roster + verify_projected_roster +
                      verify_staged_world, before any live mutation
4 constrained publish apply_world_replacement, one authority, one order
5 publish then retire publish_candidate -> apply -> retire_superseded
6 malformed candidate real production refusals, not parse errors
```

**Candidate-owned state is held UNDER the candidate, not in a process global.**
`construction::spawn_candidate_state` puts the room's staged world — outgoing
roster, room set, target index, geometry, platform state, arriving body — on a
hidden entity stamped with the transaction and marked `CandidateState`. Three
consequences are structural rather than remembered: a refusal retires it with
everything else the transaction made; a leak carries a dead stamp no later room
can find; publication ADOPTS it (takes the component, applies it, despawns the
carrier). `CandidateState` is excluded from `candidate_roots`, so "N roots
admitted" still counts authoritative bodies only.

**Supersession is a third baseline declaration, not a relaxed reconstruction.**
`TransactionBaseline::reconstructing` keeps its meaning ("the old body should
already be gone"); `superseding` states that the live body stands until
publication. `transaction::open` splits the plan against the baseline it captures,
per identity, and only under the candidate bracket — without it the roots spawn
visible and there is no *beside*.

**Measurements that should outlive the packet:**

| question | measured |
| --- | --- |
| is the declaration covered by the shipped suite? | poisoning `transaction::open` to declare nothing superseded reddens **23 `app_it` tests** |
| does the projected verifier do anything? | skipping it: **655 passed, 0 failed**. Breaking its arithmetic: **632 passed, 23 failed**. It runs and decides, and has never REFUSED on shipped traffic |
| how often does a room refuse in production? | **zero refusals across 686 publications** — the whole refusal apparatus is proven by its arms |
| may publication despawn a superseded body in custody? | no. Doing so failed `death_restores_the_checkpoint` 1/11 and `two_persistence_authorities_for_one_item` with `still_owned=1`: `restore_custody_to_checkpoint` unequips AND despawns as one operation keyed on that entity. With the skip: 11/11 and 0 |
| was the construction suite testing the shipped road? | no. With publication broken: **92 passed, 0 failed** on the live road the harness defaulted to, **78 passed, 14 failed** once moved to the bracketed one |
| what does a refused transition cost the player? | nothing measurable: displacement across the verdict frame is `Vec2(0.0, 0.0)` guaranteed against `Vec2(-1782.7, -188.0)` with the arrival applied anyway |
| is the session handoff multi-frame? | no — retire A, activate B and publish B's room are all frame 243 on the shipped app. One frame, but not one command flush |

**The dev LDtk hot reload IS covered now, in both directions — A10.3 is MEASURED
as of 2026-09-15.** Until then it was the one room road with no app-level arm
(transition, death reconstruction, reset, first-room publication and shell handoff
all had one), which is why A10.3 was labelled REASONED. Two arms in
`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs` close it on the
shipped `build_visible_app`: press `ApplyLdtkReload` with the world intact and the
effects run; press it with two process-resident holders of one
`SimId::placement` standing — a world the baseline cannot describe — and the
candidate room is refused, the preset flash stays `0.0`, `applied_count` does not
move and the player is in the same room.

⭐ **NO FILE IS WRITTEN.** The harness this document previously asked for wrote a
broken LDtk project over the watch path; that is a shared tree and an unnecessary
one. Re-reading the SAME project is an equivalent reload — it prepares a candidate,
builds the room as hidden candidates and publishes it, the whole bracket — and the
refusal is induced in the WORLD instead of on disk.

⛔ **AND WITNESSING IT FOUND A REAL DEFECT.** The developer-facing status was the
last effect on this road still running ahead of the verdict: `mark_applied` was
called on the `Ok` of `reload_ldtk_world_from_disk`, and that `Ok` means STAGED.
MEASURED: a refused reload reported `applied_count 0 -> 1` and `"world reload
applied to 'X' (#1)"`. It is verdict-gated now, and a refusal records the
verification's violations. ⇒ **The effect a human READS is an effect.** A road can
have every mechanical write correctly bracketed and still tell the developer the
world in front of them is the world on disk when it is not.

**What a refusal costs.** No `RoomLoaded`, so no fresh attempt anywhere: staged
victim hits are not voided and per-attempt state is not re-armed, both correct
because no attempt began. `RoomLoaded` has three production readers through
`ambition_combat::events::FreshAttempt` — an earlier claim that it had none was
wrong.

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

### A10 checkpoint report — 2026-09-14

**1. The candidate-world ownership model as implemented.** Two levels, the same
shape at both.

```text
CANDIDATE SESSION                          CANDIDATE ROOM
SessionRoot + InactiveCandidate            every root minted InactiveCandidate
  + SessionScopedEntity(scope) on every      under ROOM_CANDIDATE_BRACKET,
    entity spawned through                   stamped with the lane TransactionId
    SessionSpawnScope::candidate(scope)
  + ActiveContentBinding on the root       + PendingWorldReplacement on the
  + the world bundle on the root             publication entity (CandidateState)
  + process projections held as DATA in
    CandidateSessionPublication
```

Visibility is `InactiveCandidate`, a registered disabling component that stays
`pub(crate)` to `shared_tangle`; it is applied to session-owned entities by
`SessionSpawnScope::apply_to`, which is the single point all six
`spawn_*`/`insert_*` helpers pass through. Hiding the ROOT alone is not enough
and was a real defect: Bevy's disabling components do not inherit through
ownership, and the gameplay queries that find a body find it by its own markers.

**2. How staged supersession is represented.** `TransactionBaseline::superseding`
is a third declaration beside `capture`/`retiring`/`reconstructing` and keeps
their meanings intact: the live body stands until publication.
`transaction::open` splits the plan against the baseline it captured, per
identity, and only under the candidate bracket. `PublicationEffects` carries
`supersedes` (live → candidate, with a `DepartureAuthority` of `Publication` or
`Custodian`), `retires` and `owners`. The departure authority is DECLARED from
the baseline, not discovered at retirement.

**3. What the projected verifier validates.** `project_post_publication_roster`
builds `live − declared retirements − superseded live bodies + THIS publication's
owned candidates`, and `verify_projected_roster` asks of that projection: exactly
one authoritative occupant per identity (a declared deferred pair of exactly two
is admitted, a third holder is not); no candidate stamped by nobody
(`CandidateUnowned`); every declared supersession has both its participants; and
the roster invariants `verify_committed_roster` already enforces. Beside it,
`verify_staged_world` asks whether the non-entity world the publication would
install is coherent, and the commit boundary compares the plan's generation
against the `ActiveContentBinding` on the root it is publishing into.

**4. What remains OUTSIDE candidate ownership.** Nothing named in the A10 brief.
A candidate session owns its room state, geometry, moving-platform state, content
binding, prepared content, session mechanics, the player's mechanical transition
state and its construction roots; and as of A10.5 it owns its SCOPE IDENTITY
before anything decides it may be played (`ActiveSessionScope::reserve` /
`publish`, with the shell's `ReservedGameplayScopes` telling the bridge which
reservation to adopt). `ActiveGameplaySession` is still created by the bridge at
`RouteActivated` — but by then the candidate has already been verified, so the
pointer is created for a session that is known to exist.

**5. The publication boundary.** One bounded authority per level, in a fixed
order, inside one exclusive-world call so nothing scheduled observes a partial
publication (atomic to SYSTEMS, not to hooks or observers — `InactiveCandidate`
being `pub(crate)` is what closes that by making the marker unnameable outside
the crate).

```text
ROOM      publish_candidate -> apply_world_replacement -> retire_superseded
SESSION   install the aggregate's projections -> publish_candidate_session
CALLERS   every effect that MEANS the operation happened is queued behind
          publication_succeeded(P): the transition's finalize, the dev reload's
          body transit and presentation, the reset's whole sandbox wipe
```

**6. The production tests proving failure leaves N untouched.** At SESSION scope,
in the shipped app:
`a_candidate_session_the_transaction_refuses_leaves_the_live_session_playable`
drives a real handoff with two process-resident holders of one identity standing,
so the candidate's first room cannot be verified at all; it asserts the PREMISE
(a transaction ran and was refused) and then that the live session keeps its
activation id, its scope, its population and its room.
`a_candidate_session_does_not_retire_the_playing_sessions_world` samples every
frame of a successful handoff and asserts N's population is untouched while N is
still the live scope — poison-verified at 163 against 181, the 18 bodies a
process-wide baseline despawned before the verifiers were taught whose world they
were looking at. The admission control is
`a_shell_handoff_publishes_the_incoming_sessions_room`.

At ROOM scope, in the shipped app: `a_room_the_transaction_refuses_leaves_the_room_the_player_is_in_intact`
drives a real stale-generation refusal and asserts the active room, the geometry
and the roster are unchanged, the body did not move across the verdict frame, the
developer flash / arrival flash / room-visual request did not fire, no checkpoint
restore is owed and the transaction is not reported committed —
with `a_crossing_that_publishes_does_every_transition_effect` as the control that
makes those assertions falsifiable. `the_shipped_apps_own_first_room_publishes`
and `a_shell_handoff_publishes_the_incoming_sessions_room` are the admission
controls.

**7. Remaining work.** The acceptance criterion is MET at both scopes; these are
the gaps that remain beside it, none of which falsifies it.

- **The transition state machine advances to `playing` on a refusal** as well as
  on success, so a persistently refused door is a livelock rather than a
  corrupted world.
- **Custody supersession is Model B**: publication may leave the predecessor and
  the candidate both standing for the window in which custody removes the
  predecessor. Declared rather than hidden, and not to be widened; Model A
  remains the better end state, particularly for replication.
- **A10.5's fallback is still live**: an activation nobody prepared a candidate
  for builds its own world inside the activation, with the pre-A10.5 guarantee.
  The world log names which road each session took (`road=…`), so the fallback is
  removed on evidence rather than on hope.

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
