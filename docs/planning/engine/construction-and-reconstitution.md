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

⛔ **AND A SECOND ONE: the local rollback restart.** MEASURED — across a refused
reload `session_is_active` went `true -> false`. Both `stop_session_deferred` and
the restart marker were issued at the TOP of `handle_ldtk_hot_reload`, before the
function that owns the bracket was called at all. The marker is verdict-gated now
(`true -> true` across a refusal) and the deferred stop is deleted rather than
moved — the `PostUpdate` owner already stops a live session before releasing
ownership, and nothing simulates between them.

⇒ **A ROAD'S EFFECTS ARE NOT ONLY THE WRITES INSIDE ITS TRANSACTION.** Two
unconditional effects survived a change whose entire subject was unconditional
effects, because both sat outside the closure that change moved: one after the
bracket (the status) and one before it (the restart). When auditing a road for
A10, read the whole SYSTEM, not the transaction function.

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

⛔⛤ **AND A CANDIDATE SESSION HAS EXACTLY FOUR EXITS — closed 2026-09-15, when
TWO of them turned out not to exist.**

```text
ADOPTED      publish_candidate_session   (the gate said Admit)
REFUSED      discard_candidate_session   (the gate said Refuse)
SUPERSEDED   discard_candidate_session   (a later pending route replaced it)
ABANDONED    discard_candidate_session   (ShellCommand::CancelPending ended it)
```

⭐ **THE LAST TWO ARE ONE SITE, because they are one question**: *is the router
still pursuing this candidate's activation?* `discard_abandoned_candidate` runs at
the HEAD of the preparer and answers it, so a future fifth way for a pending route
to end needs no fifth copy of the cleanup.

The third was `slot.0 = Some(candidate)` — a whole prepared session going out of
scope in silence, its hidden root, hidden first room, publication receipt and
reserved scope all alive and unreachable. ⇒ **A candidate that is neither
published nor discarded is the state this lifecycle exists to make impossible,
and it was one `=` away.** MEASURED: it fires 0 times in `app_it`, so it was
unwitnessed as well as broken; the arm that reaches it discards 20 entities. The
fourth did not exist at all, and leaked the same four things.

Both are witnessed on the shipped composition, and each is the other's control:
`a_candidate_session_replaced_while_pending_is_discarded` and
`a_candidate_session_whose_route_is_cancelled_is_discarded`. POISON-VERIFIED
together — disabling the one cleanup site leaves
`session_root_for_scope(SessionScopeId(0))` returning `Some(1489v0)` for the
cancel and trips the preparer's own `debug_assert` for the supersession.

⛔⛤ **THE FOURTH EXIT WAS FOUND BY ENUMERATING THE WRITES TO
`CandidateSessionSlot` (exactly three) AND ASKING WHAT ELSE CAN END A PENDING
ROUTE.** `ShellCommand::CancelPending { request }` can — `Q118`'s "breaking the
authorization early cancels both halves". It clears the router's pending
transaction and told this provider nothing.

⇒ The hard part was the DISCRIMINATOR, not the cleanup. "The router is no longer
pending on this candidate's activation" is ALSO true for the window between
activation and adoption, and discarding there would destroy the candidate adoption
is about to demand — which PANICS by design, the fallback having been deleted.
MEASURED across the whole `app_it` suite before the condition was written: asking
the RESERVATION as well (the ledger adoption consumes with `take`) fires **exactly
once in 663 arms**, and that once is the supersession case —

```text
[probe] stale-slot activation=ShellActivationId(2) pending=Some(ShellActivationId(3))
```

— so there are **zero false positives on the healthy activation path**, and no
`Activated`-read-a-frame-late window exists to race.

⇒ **AND THE ROOM LEVEL HAS NO SUCH PROBLEM, FOR A STRUCTURAL REASON WORTH
NAMING.** `transaction::open` and `transaction::close` sit in ONE function with
NOTHING between them but the closure that builds the room — MEASURED by reading
`spawn_contents_for`: no `return`, no `?`, no `continue`, no `if`, no `match`
between the two calls. A room candidate cannot escape its bracket because the
bracket is straight-line code.

⛔⛤ **A BRACKET HELD IN A RESOURCE ACROSS FRAMES HAS AS MANY EXITS AS THE WORLD
HAS WAYS OF MOVING ON.** The session candidate's bracket spans frames and lives in
`CandidateSessionSlot`, so "open" and "close" are not two statements a reader can
see together, and two of its four exits were simply missing. The room's
`PendingRoomTransitionFinalize` is the safe middle case: also a resource, but
filled and drained by two `.chain()`ed systems in one schedule pass, so it has
exactly one exit and no frame boundary to be abandoned across. ⇒ When a lifecycle
moves from a call bracket into a resource, ENUMERATE THE EXITS — the compiler
stops helping at exactly that moment.

⭐ **AND THE RECEIPT HALF OF THAT RULE IS MEASURED NOW TOO.** A
`PublicationRetention::UntilOwnerRetires` receipt is an ENTITY that stands until
its owner calls `retire_publication`, and "every owner retires it" was a property
held by reading five call sites — the same shape as the candidate-slot exits, and
nothing could count them. `rooms::outstanding_publications` can: it is asserted to
be **0** after a committed dev reload and after a cancelled candidate, and
POISON-VERIFIED (dropping the reload's own retire leaves 1). ⚠ A COUNT, not a
list — the receipts stay `pub(crate)`, because a reader outside that crate has no
business holding one.

⛔⛤ **AND A10'S INVARIANT IS ASSERTED AS A NUMBER NOW.**
`construction::outstanding_candidates` counts the hidden candidate entities
standing at a settled moment, and it is **0** after a cancelled candidate and
after a committed dev reload. ⇒ *No candidate outlived its transaction.* That is
the statement the whole campaign is about, and until now nothing could ask it: a
candidate that outlives its transaction is hidden by a DISABLING marker from every
ordinary query in the game, so it is invisible to exactly the systems that would
otherwise trip over it.

⭐ **IT CARRIES ITS OWN POSITIVE CONTROL, and it needs one.** A census that
reports 0 because its query is wrong reads exactly like a world with no orphans.
The cancel arm asserts the count is **> 0** three frames in, while the candidate
is supposed to be standing — so the zero at the end is a claim about the WORLD
rather than about the query.

⭐ **EACH EXIT OWES THE SAME FOUR RELEASES, and that is the thing to check when a
fourth exit is ever added:** the candidate's entities, its publication receipt,
its scope RESERVATION (`ReservedGameplayScopes::release`, the refusal half of
`take`), and its gate registration — the ROUTE HOLD first and the EVALUATOR
second. ⚠ That order is load-bearing: forgetting the evaluator while the hold
stands leaves the router a hold it cannot evaluate, and measured, the route is
then wedged forever and the SUPERSEDING session never starts either. The hold is
released with a route id recorded ON the candidate, because a candidate
superseded by a route of a different name cannot be cleaned up from the
superseding route's id.

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

⚠ **ONE ITEM ON THE BRIEF'S LIST IS ABSENT, AND BY CONSTRUCTION RATHER THAN BY
OMISSION: *valid relation endpoints in the projected world*.** The hazard it names
is a RETAINED live entity holding a relation whose endpoint publication would
retire. MEASURED by source 2026-09-15: that state is not expressible here.
`ConstructionPlan::commit` resolves both endpoints of every planned relation out
of the RECEIPT — the identities this commit itself built — and a relation naming
anything else is an `unreachable!`, not a skipped row:

```text
planned relation {from} -> {to} names an identity this commit did not build
```

⇒ **Every relation this system can express has both endpoints inside one
transaction**, so publication cannot orphan one: either both ends go or both ends
stay. A projected endpoint check would be a guard on a state no plan can reach,
and it would need a live-relation enumerator the domain does not expose
(`ConstructionRegistry` holds relation IDENTITY only; there is no inverse of
`dispatch_relation`). ⇒ **If relations ever gain a live endpoint, this check stops
being vacuous and becomes required** — that is the trigger to write it, and the
`unreachable!` above is what will announce it.

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

⭐ **AND THE BRACKET FLAG ITSELF NEEDS NO ASSERTION — MEASURED 2026-09-15, AND
THE MEASUREMENT IS WHY THE ASSERTION WAS NOT WRITTEN.** `ROOM_CANDIDATE_BRACKET`
is a `const bool` that turns the whole candidate road on, and the construction
tests deliberately READ it rather than hardcoding `true`, so flipping it would
take the tests with it — the shape of a control that switches itself off. It is
not: poisoning it to `false` fails **26 `app_it` arms** (save/load custody,
canonical reconstitution, death and checkpoint restore, the duel restage, the dev
reload's committed arm). ⇒ **The flag is guarded by CONSEQUENCE, which is stronger
than a guard on its value**, and a `assert!(ROOM_CANDIDATE_BRACKET)` would have
added a line that fails second and explains less.

⛔⛤ **AND A SECOND CALLER WOULD BE A SECOND AUTHORITY, WHICH IS NOW A CONTRACT
RATHER THAN A HABIT (2026-09-15).** MEASURED: each publication primitive has
EXACTLY ONE production call site — `publish_candidate`, `retire_superseded` and
`retire_candidate` in `world/rooms/transaction.rs`, `publish_candidate_session`
and `discard_candidate_session` in the provider's adoption and its gate. They are
`pub` because they cross a crate boundary, not because anyone else may call them,
and until now nothing said so. `check_absence_contracts.py` carries
`only-the-publication-authority-publishes-a-candidate`, POISON-VERIFIED (a call
added to `room_transition/commit.rs` reds it) and with its own fire test, so it is
not one of the contracts that sit skipped for want of a fixture.

⚠ The patterns ask for the MODULE PATH, not the bare name: `game/ambition_content`
defines an unrelated `publish_candidate` of its own, and a name-only contract
would red on a function with nothing to do with A10. A second pattern catches the
IMPORT, because an imported name is then called bare and a qualified-call pattern
cannot see it.

⭐ `apply_world_replacement` is absent from the contract because it is PRIVATE to
its own file — unexpressible rather than forbidden, which is the stronger form and
the one to prefer whenever a primitive does not have to cross a crate boundary.

⛔⛤ **AND THE SAME RULE FOR HIDING, which is the other half of the bracket.**
`InactiveCandidate` is `pub(crate)`, so no outside crate can name the marker or
hook it; `hide_candidate_session_root` and `hide_candidate_session_entity` are the
only doors through that wall, and each has exactly ONE production caller (MEASURED
2026-09-15) — the room builder hiding the candidate session root it just minted,
and `SessionSpawnScope::apply_to` applying the hiding policy to what a candidate
spawns. `only-the-candidate-builder-hides-a-root` keeps it that way,
poison-verified the same way. The failure it prevents is the least debuggable this
machinery can produce: a DISABLING marker applied to something that is not a
candidate removes that entity from every ordinary query in the game while leaving
it alive.

⛔⛤ **AND "EVERY EFFECT" MEANS EVERY EFFECT — TWO ESCAPED THE FIRST PASS, BOTH ON
THE DEV RELOAD, AND BOTH OUTSIDE THE CLOSURE THAT PASS MOVED (closed 2026-09-15).**
One was AFTER the bracket: `mark_applied` on the `Ok` of
`reload_ldtk_world_from_disk`, where `Ok` means STAGED — a refused reload told the
developer *"world reload applied to 'X' (#1)"*. One was BEFORE it:
`stop_session_deferred` plus the local-GGRS restart marker, issued at the top of
`handle_ldtk_hot_reload` before the function that owns the publication was called
at all — MEASURED, a refused reload took `session_is_active` from `true` to
`false`, tearing down the rollback timeline of the world it left standing.

⇒ **A ROAD'S EFFECTS ARE NOT ONLY THE WRITES INSIDE ITS TRANSACTION, and the
effect a human READS is an effect.** When auditing a road against this boundary,
read the whole SYSTEM — everything from its first statement to its last queued
closure — not the transaction function.

⛔⛤ **AND THE SUCCESS ARM'S HALF IS ASKED OF THE SAME AUTHORITY (2026-09-15).**
"No duplicate authoritative identity in the published world" is now asserted after
a committed dev reload AND after a shell handoff, and it is asked with
`TransactionBaseline::capture` rather than with a census written for the test:
that call is precisely the operation which cannot describe a world holding one
identity twice, and `BaselineCaptureError::DuplicateIdentity` is the refusal these
same arms induce ON PURPOSE elsewhere. ⇒ **The success arm and the refusal arm now
turn on one mechanism**, so neither can drift into certifying something the other
does not mean. For the handoff it says something a roster COUNT cannot: the
retired session's world is genuinely gone rather than standing beside the one that
replaced it.

⚠ **AND THE CANCEL ARM DELIBERATELY STARTS FROM A COLD APP, NOT A LIVE SESSION —
MEASURED 2026-09-15, AND THE MEASUREMENT IS THE REASON.** The stronger arm to want
is *cancel while a session is PLAYING, and watch the playing world survive*. It
cannot be written this way: from a live session, with the experience's content
already prepared, the second route is **fully activated within three frames** —

```text
[probe] candidates=0 scope0=None scope1=Some(1490v34) activation=None
```

— the incoming session is already live at scope 1, the outgoing one at scope 0 is
already retired, and NOTHING IS PENDING to cancel. ⇒ **The pending window a
correlated requester could cancel in is a cold-start phenomenon**, because what
makes the window is content preparation, and a warm app has already done it. An
arm that cancels a route which has already activated measures nothing, and would
have read as a passing cancel witness. The cold-start shape is the honest one.

⛔⛤ **AND THE SUPERSESSION CASE HAS A PRODUCTION WITNESS (2026-09-15) — "old A
remains live while hidden B is verified".** The brief asks for one explicitly, and
the property is provable from two facts rather than by observing the world
mid-verification: `ProjectionViolation::SupersededNotLive` exists precisely to
REFUSE a declared supersession whose live half is already gone, so **a declared
supersession that PUBLISHED is one whose predecessor was still standing when the
projected post-publication world was verified**.
`LastConstructionVerification::supersessions` records the declaration count so a
test can say a supersession happened at all, and
`a_custody_deferred_supersession_is_never_visible_as_two_holders` asserts it is
`> 0` on a reset that re-authors an identity a hand is holding — the *old A live,
hidden B verified, B declares it supersedes A* shape, on the shipped road.

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

At the DEV RELOAD road, in the shipped app (2026-09-15, the last road to get one):
`a_refused_world_reload_leaves_the_running_game_untouched` presses Apply Reload
with two process-resident holders of one `SimId::placement` standing and asserts
the preset flash, the applied count, the active room and the live rollback
timeline are all unmoved; `a_committed_world_reload_applies_its_effects` is its
control and asserts each of those DOES move on a reload that commits. No file is
written — re-reading the same project runs the whole candidate bracket, so the
refusal is induced in the world rather than on a shared tree's disk.

**A10.5's fallback is DELETED, 2026-09-15 — there is one activation road.** An
activation nobody prepared a candidate for used to build its own world INSIDE the
activation, with the pre-A10.5 guarantee: the outgoing session was already retired
by the time anyone knew whether the incoming one could be built. It was kept
deliberately (the adopt-only first version failed 17 `app_it` arms with *"reached
no session world"*) and was to go *"only when measurement says nothing uses it"*.

MEASURED: **655 activations, 0 fallback** — 634 in `-p ambition_app --test app_it`
and 21 in `--workspace --lib`. Reaching it is a `panic!` now that names the two
schedule edges (`.after(PlatformerPreparationSet)`,
`.before(AmbitionGameShellSet::Pending)`) which make it unreachable, and 661
`app_it` arms pass without firing it. Its shell-side twin went with it:
`candidate_session_gate` used to `Admit` when nothing was prepared, on the
reasoning that holding would wedge a route it does not own — but the evaluator is
registered ONLY against holds this provider created, so an empty slot means the
candidate for ITS OWN route is gone. It refuses now, and the playing session
survives.

⛔ **AND THE EVIDENCE THIS DOCUMENT ASKED FOR COULD NOT HAVE ANSWERED IT.** The
bullet removed here said to read the `road=` label out of the world log. That log
is capped at `WORLD_LOG_CAP` = 4000 lines PER PROCESS, and one `app_it` run hits
the cap: it reported **458** of the 634 activations it saw. A census read off it is
a FLOOR, and a floor of zero in the bucket you are draining is not a zero — the
count above comes from an uncapped probe whose positive control is the same
strong-road bucket. ⇒ **A diagnostic log with a line cap is not a census
instrument**, however precisely it labels what it does print.

**7. Remaining work.** The acceptance criterion is MET at both scopes; these are
the gaps that remain beside it, none of which falsifies it.

- **A persistently refused door is un-passable, and silently so.** Re-read
  2026-09-15: the earlier wording here (*"the state machine advances to `playing`
  on a refusal"*) made this sound like an unhandled case. It is not — both hosts
  CANCEL. The eager host calls `cancel_eager_room_transition_transaction` (load
  cancelled, the active transition cleared, the rollback intent spent) and the
  confirmed host returns `CommitOutcome::Cancelled`; returning to `Playing` is the
  correct terminal policy, because the player is standing in a room that was never
  touched. What actually remains is that the refusal reaches only a `warn!` and a
  `room_commit_refused` world-log line: **the player is given no signal, and the
  door simply never opens.** That is a presentation gap, not a world-integrity
  one.
- **Custody supersession is Model B, and the window is now MEASURED rather than
  declared.** Publication leaves the predecessor standing when it declared
  `DepartureAuthority::Custodian`, because `restore_custody_to_checkpoint`
  unequips AND despawns as one operation keyed on that entity. MEASURED
  2026-09-15: **5 of 838 publications in `app_it` open such a window**
  (`left_to_custodian=1`, `retired=0`; room `central_hub_complex`, identities
  `placement:ground_gun_sword` and `placement:ground_grapple`) — so it is real in
  production, not theoretical. And
  `a_custody_deferred_supersession_is_never_visible_as_two_holders` samples EVERY
  frame of the reset and finds **peak 1**: the duplicate the projected verifier
  ADMITS is never observable to anything that steps. POISON-VERIFIED — disabling
  `apply_committed_checkpoint_restore` takes the peak to **2**, so the arm is
  about a state that genuinely occurs and the custodian is genuinely what closes
  it.
  ⇒ Model A remains the better end state for replication, but the gap it closes
  is narrower than this document used to imply: it is not an observable window,
  it is a window whose closing depends on a second authority running.

  ⭐ **AND THE MECHANISM IS WRITTEN DOWN NOW, which is what the next attempt was
  missing.** MEASURED 2026-09-15 by probing the custodian's own decisions at the
  moment the window is open:

  ```text
  [probe] custodian reinstate=0 retract=1 inventory=[
      "placement:ground_gun_sword@521v0=Held { holder: 532v0 }",   <- the predecessor
      "placement:ground_gun_sword@548v0=InWorld",                  <- the candidate
  ]
  ```

  Both hold the identity at once; the custodian RETRACTS the predecessor — strips
  the hand and despawns it as one operation — and the candidate the room authored
  is what remains. The two facts it needs from the predecessor to do that are the
  HOLDER entity and the item's SPEC ID, and it needs them from the LIVE entity,
  because it compares the spec id before stripping a hand that an equip-swap may
  have refilled with something else.

  ⇒ **Model A is therefore: capture `(holder, spec_id)` when the supersession is
  DECLARED — the declaration already reads `InCustodyOf` off the baseline entry to
  choose `DepartureAuthority::Custodian`, so the entity is in hand at exactly the
  right moment — let publication despawn the predecessor, and have the custodian
  consume the recorded pair instead of the live entity.**

  ⛔⛤ **AND THE LEDGER'S DRAIN IS THE HAZARD THAT DECIDES THE DESIGN — derived
  2026-09-15 while scoping the change, and it is why this is not a two-file
  patch.** The moment publication despawns the predecessor, the hand is holding a
  `HeldItem` that names a dead entity, and it stays that way until something
  strips it. Under Model B that never happens because the despawn and the strip
  are one operation. So Model A's ledger must be drained on EVERY path that can
  declare a custody handoff, not just the one that motivated it: the obvious home
  is inside `restore_custody_to_checkpoint`, but that runs only when the commit
  carries a `checkpoint_operation`, and a plain room TRANSITION can declare a
  custody supersession too. An undrained ledger is strictly worse than the window
  it replaces — a stale hand rather than a duplicate that closes itself.

  ⇒ The drain therefore wants to be an UNCONDITIONAL step of the publication
  tail, ordered with the same guarantee the despawn has, rather than a passenger
  on the checkpoint restore. That is the part to design first, and the part the
  straight inversion never reached.

  ⚠ **The record must NOT live on `Supersession`.** That type is generic
  construction vocabulary in `shared_tangle`; a held-item spec id is items-domain
  content, and putting one in the other is the layering mistake that would make
  this change permanent. The ledger belongs to the items domain, keyed by `SimId`,
  written when it sees the custody-deferred declaration and consumed by
  `restore_custody_to_checkpoint`. Blast radius is death and checkpoint restore —
  the same arms the straight inversion failed (1/11) — so it wants a full
  `app_it` run, not a targeted one.
  `LastConstructionVerification::left_to_custodian` exists so the PREMISE is
  assertable — a custody-window test that never opens one passes while measuring
  nothing, which is exactly what the first version of that arm did.

## How the session scope got closed — the investigation, kept out of the queue row

⚠ **HISTORY, NOT CURRENT STATE.** Every blocker below is resolved; A10.5 landed
on 2026-09-15 and its fallback was deleted the same day. This is here because the
measurements are worth keeping and the queue row is not a diary.

⚠ **THE REFUSAL HALF HAS NO PRODUCTION WITNESS.** The shipped app cannot be made
to refuse its first room the way the transition arm is: the first room's
`ActiveContentBinding` is written by setup from that room's own plan, so it
always matches. The PUBLISH half is exercised by every `app_it` test that boots.
The vocabulary the design rests on is witnessed at unit level by
`a_hidden_candidate_session_root_is_invisible_to_the_live_lookup_and_visible_to_its_transaction`,
which asserts the premise (an ordinary root IS visible) first so an unregistered
filter cannot fake it. ⇒ **A10 IS NOT CLOSED**: the acceptance criterion is a
PRODUCTION composition demonstrating that a failed candidate leaves the last-good
world playable, and at session scope that demonstration does not exist yet.

**ACTUAL BLOCKER — WORLD N IS DESTROYED BEFORE CANDIDATE N+1 IS BEGUN, AND IT IS
AN ORDERING FACT, NOT A MISSING TEST (MEASURED 2026-09-14 from source).**
`translate_shell_session_lifecycle` emits `RouteDeactivated` and `RouteActivated`
from ONE run, so a handoff retires and activates in the same frame; and
`SessionScopePlugin` chains `RetireAuthority -> Cleanup -> Activate ->
Presentation`, with `despawn_retired_session_entities` in `Cleanup`. ⇒ By the
time the provider builds the incoming session's candidate, the outgoing session's
entities are ALREADY DESPAWNED, unconditionally.

⚠ **THAT ORDER IS NOT AN ACCIDENT AND MUST NOT SIMPLY BE REVERSED.** It was
changed to retire-first on 2026-09-13 for a measured reason recorded in the
plugin: with `Cleanup` last, the incoming session's provider built its room while
the outgoing scope's placements were still live and the whole room was refused —
`room-refused central_hub_complex :: 18x Duplicated` — and
`reset_session_scoped_resources_on_retire` removed `SessionMechanics` after the
activation that installed it.

⇒ **THE SESSION-SCOPE ACCEPTANCE CRITERION IS THEREFORE UNREACHABLE TODAY**: a
refused candidate session leaves NO session at all, because N was already gone.
This is why A10 is not closed, and it is a larger statement than "the refusal
half has no witness".

⚠ **AND THE FIX IS NOT TO REORDER THE RETIREMENT — CORRECTED 2026-09-14, THE
SAME DAY THE ROW ABOVE WAS WRITTEN.** My first reading said the outgoing
session's retirement had to become a declared effect of the incoming
publication, the room packet's shape lifted one level. It does not: the
retirement is fine where it is, because **a candidate verified BEFORE the route
activates is already known-good by the time `RouteDeactivated(A)` is written**.
A refused candidate never activates, so A is never retired. That also leaves the
measured 2026-09-13 reason for the current order untouched.

⛔⛤ **A10.5 WAS ATTEMPTED AND REVERTED — 2026-09-14, AND THE MEASUREMENT IS THE
ROW.** The full change compiled (shell reservation ledger + `adopt_world`,
`build_candidate` spawning its own hidden root, a pending-phase preparer holding
the route, a registered `ShellActivationGates` evaluator, adoption on
`Activated`) and then failed 17+ `app_it` arms with *"reached no session world"*.
The working tree was reverted to the green HEAD; the patch is kept out of tree.

⇒ **THE CAUSE IS SHAPE, NOT DETAIL: I REPLACED AN UNCONDITIONAL CONSTRUCTOR WITH
A CONDITIONAL ONE.** The activation system built a world for
EVERY activation of an authored-catalog experience. The candidate road only fires
when a pending route has already published its prepared session, so every
activation that does not pass through that exact state — and there are several
roads that do not, plus an ordering question about whether preparation has
published by the time the pending-phase system looks within the same frame — got
no world at all.

⇒ **THE NEXT ATTEMPT MUST BE ADDITIVE.** Activation keeps a constructor for the
case where no candidate was prepared for it; the candidate road is an
OPTIMISATION of the ordinary road, not a replacement for it, and only the routes
that actually prepared a candidate get the strong guarantee. Ship it behind that
fallback, measure which activations take which road, and only then consider
removing the fallback. (⚠ REASONED, not measured: the two causes above were not
separated before the revert — the next attempt should instrument which one fires.)

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
