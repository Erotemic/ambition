# Fast iteration acceptance fixtures

**State:** executable specifications for future implementation, not executed tests.
[Packets](fast-iteration-implementation.md) own sequencing. This page owns the
cross-packet fixture definitions. Put implementation tests in existing shared
binaries/modules where possible. Do not create a separate executable per row.

FI1-FI10 name iteration fixtures only. They are distinct from F1-F9 findings in
older architecture/checkpoint reviews. None of these fixture IDs claims a test
with that name already exists in the source.

## What a completion claim contains

Record the source revision, selected profile, affected owners, exact command,
positive result, deliberate defect, observed failure and removed old path.
Record unavailable measurements as unavailable. A source scan can prove an import
or write-site restriction; it cannot prove runtime semantics. A green test that
stops before the relevant owner consumes its input is not acceptance.

For each deliberate defect, identify the exact production branch/registration/
field to alter. First prove the good fixture reaches it. Then alter only that
obligation and check that the named assertion fails for the intended reason.
Compilation failure alone is not a behavioral poison witness. Restore the code
and run the positive fixture again. Temporary mutations do not ship.

## FI1 - independent content production

**Packets:** I1/I2/I9. Use an external manifest and lockfile, outside the engine's
inherited workspace feature set. Build a move with the existing pure values and
builders. The fixture contains actual nondefault timing, nested references and a
technique; an empty move does not exercise hydration.

Check resolved normal/build dependencies and actual compilation units. Classify
dev dependencies separately. No host, rendering, game implementation or Bevy ECS
enters the portable author's normal/build closure. Compiler failure is not an
empty dependency set. Test the SDK's feature combinations that are supported,
not the accidental union of the whole workspace.

**Behavior:** two source frontends produce the same admitted move. A prebuilt host
uses it in a fixed exchange and reports its generation. A timing edit changes the
expected contact/action trace without a host build/link command.

**Defects:** restore a facade dependency; leave the old compiled move authoritative;
remove one nondefault field in lowering. Respectively fail closure, observed edit,
and semantic parity. Do not normalize away the field to make parity pass.

## FI2 - coherent candidate and dependency invalidation

**Packets:** I2/I3a. A pack contains two characters, shared moves and one unrelated
section. Edit a shared move, delete a referenced move, then change only diagnostic
provenance. Exercise the real selected profile's admission.

**Behavior:** the shared move invalidates each dependent and reuses the unrelated
section. A dangling reference refuses the whole candidate. Diagnostic-only edits
do not change mechanical identity. Missing/truncated section bytes never publish.
Record the prepared/reused set and why each decision occurred.

**Defects:** omit a reverse dependency; validate character A against candidate data
and B against active data; silently fall back to a built-in table. Each changes a
specific expected result. Warming or emptying the cache must not change admission
or the resulting canonical generation.

## FI3 - stale work and generation isolation

**Packets:** I3a/I3c. Start candidate A, request B, finish B first, then finish A.
Repeat while the installed profile changes. Repeat in two independent Apps.

**Behavior:** only the still-selected sealed candidate can publish. An invalid or
superseded attempt changes no active epoch, prepared registry or timeline. A
successful activation changes all selected owners at one visibility barrier. An
identical generation is a no-op. App-local choices do not contaminate each other.

**Defects:** accept a stale base epoch; publish one domain early; keep selection in
a process singleton; increment epoch on refusal. Observe the corresponding wrong
active identity, mixed read, cross-App change or unexpected history invalidation.

## FI4 - repeatable development reconstruction

**Packets:** I3b/I3c/A10. Use one supported scene with a body, active move or boss,
item custody and a relation. Pin the scenario seed/input/checkpoint and generation.
Prepare a changed generation while the old scene remains usable.

Inject malformed relationship, missing mechanical dependency, duplicate planned
identity and forbidden candidate resource write. The supported restricted path
must reject before publishing/retiring the old scene. Include a candidate whose
metadata is valid but whose materialization draft violates a domain invariant.

**Behavior:** refusal leaves the old scene unchanged; a supported recovery reports
recovered instead of unchanged. A valid edit re-enters the chosen scenario with
new behavior and a new timeline under the same session-health rule. A changed
schema without an accepted migration refuses state transfer. Local restart must
not overwrite the user's durable save. No step silently falls back to new-game.

**Defects:** retire active state before validation; use live mutable checkpoint
instead of the pinned input; let a candidate hook emit an effect; leave a stale
MovePlayback/cache reference outside the new generation; clear unhealthy status.
A fail-stop-only implementation reports partial completion, not a passing FI4.
Native panic/unsafe-plugin recovery is outside this bounded guarantee.

## FI5 - domain requests and acknowledgements

**Packets:** I4. Use one admitted technique with a real owner, one actor-spawn
request and one action with a resource cost. Native and module paths use the same
reducers; duplicate implementations are not comparison evidence.

**Behavior:** two competing valid requests reach domain arbitration. Submitted is
not applied. The refused request does not consume a success counter. Spawn state
moves from pending to attached only upon the construction result. Move contact
credit remains with the launching occurrence after reflection or interruption.
A request after its consume barrier gets the documented next-point/refusal result.

**Defects:** acknowledge on enqueue; collapse identical payloads with different
occurrence IDs; sort by module key to override owner priority; substitute current
MovePlayback for launch provenance; spend cost outside the action transaction.
Each must fail an outcome assertion after the domain has run.

## FI6 - populated state, ownership and rewind

**Packets:** I4/I5. A module owns a bounded graph, per-actor records, a session record,
a spawn counter and a pending-result cursor. Use nondefault values that control
future behavior. At least one actor is created after the snapshot and one is
removed, then restored/remapped. Two records share the same shape but not identity.

Drive the existing real GGRS SyncTestSession. Rewind between request submission and
result, then between graph change and future spawn. Compare canonical logical
state, accepted request trace, population and semantic references. An encode/decode
roundtrip alone is insufficient.

**Defects:** omit store registration; omit only the changing hash field; keep a
cursor in guest globals; shallow-share mutable snapshot data; swap records between
actors; skip removal/restoration of a population member. Each needs a distinct
observation. Duplicate schema keys, foreign writes, stale handles and aggregate
budget overflow also refuse through the checked binding.

## FI7 - owner retirement and external effects

**Packets:** I4/I5/I7. Produce a pending request and a presentation claim, then
retire the actor/module/room before its next invocation. Rewind and resimulate a
frame that now emits no effects. Exercise an irreversible-effect sink twice with
the same confirmed occurrence.

**Behavior:** the relevant owner cancels or completes the pending action under its
policy; cleanup runs without calling dead code. An empty resimulation replaces
prior speculative effects. Owner-scoped claims retract without clearing another
owner's claim. Confirmed external work uses existing session authorization and a
sink-specific idempotence key where retry is supported.

**Defects:** cleanup only inside the producer's callback; skip empty-frame journal
replacement; release on speculative submission; use a process-global effect key.
Do not claim durable exactly-once delivery merely because the in-process journal
released once. Record retry/loss semantics at the real external sink.

## FI8 - arbitrary algorithm and actor symmetry

**Packets:** I7/I9. Implement a graph/topology or strategy algorithm using existing
query/request ports plus new module-owned schema. Maintain a graph, compute a
nontrivial property across ticks, and use its result to request an existing domain
action. Its state affects a future step, not only a diagnostic counter.

Run it for human-driven, AI-driven and otherwise controlled bodies, plus more than
two actors. The body/action owner remains the same. A boss is an authored role,
not a second actor pipeline. A brain receives its selected knowledge view; a
world-rule module may use its explicit broader grant.

**Behavior:** changing only module code/schema produces the new algorithm without
host relinking or a named core IR opcode. Headless and presented profiles agree
on simulation. Physical Bevy entity order and view allocation cannot select the
winning target or the identity of the actor.

**Defects:** branch on player zero; run both old and new mechanic writers; add a
closed engine dispatch case for this algorithm; leak omniscient facts into a
limited-knowledge controller. Measure code/compile independence and outcomes,
not a textual ban on any particular algorithm name.

## FI9 - lifecycle scale and repeated definitions

**Packets:** A8/I5/I9. These are separate stages; do not require full streaming for
I1-I3. First test an active scene plus many dormant persistent records. Later run
two live instances of the same authored room, each with the same local placements.

**Behavior:** one active actor edit does not walk/snapshot all dormant records.
Promote records to live ownership and demote them at the supported confirmed
boundary; save/reload preserves occurrence and custody. In the two-instance case,
queries, contacts, construction, observers, teardown and save keys stay distinct.
Two views of one instance do not create two simulations. The one-instance profile
uses the same path with one instance, not the old singleton fallback.

**Defects:** choose current camera as world authority; key live data by room
definition alone; copy the durable ledger into every rollback snapshot; leave two
live writers during promotion; retire all instances when one unloads. Counter-based
work assertions verify the intended work set; wall-clock M2 samples quantify cost.

## FI10 - backend and cost conformance

**Packets:** I6/I8/I9. Feed the same admitted inputs to the static reference and
candidate executable binding. Compare state and requests under the supported
execution profile. Do not advertise cross-platform native equivalence without
that separate proof.

**Behavior:** guest mutable memory/globals/tables/imported state cannot retain
unregistered authority between independent calls. A work-limit fault is consistent.
A removed code generation remains pinned while callbacks/destructors can reach it.
Module editing compiles only the selected independent module; host build/link
activity invalidates the no-relink result even if total time is small.

Run M0-M3 with recorded populations, payloads, write density and rollback depth.
Report boundary crossings, bytes copied/hashed, reset and snapshot costs, queue
latency and first observed behavior. Compare cold/no-op/warm cases separately.

**Defects:** retained guest global, stale callback, hidden nondeterministic import,
incorrect write barrier, missing delta base, or helper accepting an old generation.
Unsupported targets are limitations, not simulated passes. Keep an optimization
only when its measured benefit and correctness justify its added machinery.

## Fixtures are not a second test framework

Reuse current harness, app_it grouping, dependency guards and domain tests.
Fast pure tests run without rendering. One selected assembled/GGRS fixture proves
each changed host boundary. An accumulation checkpoint can run the broader suite;
an ordinary content edit does not need that suite repeatedly. Add a narrow missing
assertion or observation where the current harness cannot reach the required state.
Do not substitute a new source-string ratchet for that runtime evidence.
