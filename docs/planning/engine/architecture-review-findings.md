# Architecture review findings requiring implementation evidence

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
This is a bounded source-review finding set, not a replacement issue tracker.
The [queue](../queue.md) chooses work and the
[frontier](actor-monolith-work-frontier.md) supplies packets. Remove a resolved
finding from the live work surface after recording its test and resulting owner.
Git preserves the previous diagnosis, including
[archived epochs](../repository-history.md). Locally absent history is not proof
of a fabricated source reference.

**Verification limit:** Rust/Cargo were unavailable in this review environment.
No Rust reproduction, gameplay session, rollback run, GPU run or release build
was executed. The distinctions below are intentional:

- **Source-established:** the stated control/data flow is directly visible.
- **Conditional defect:** a supplied state can exercise a bad path; production
  reachability and the intended behavior still need the specified test.
- **Contract limitation:** an existing guarantee is narrower than a plan suggests;
  this is not automatically a new runtime regression.

## F1. Startup checkpoint routing spends its latch before admission

**Owner:** session lifecycle, currently implemented in shrine and installed by
item pickup. **Priority:** first focused correctness characterization in A1.
**Confidence:** source-established latch/admission order; conditional lost-startup-
resume behavior. No production reproduction has been run.

### Evidence

`crates/ambition_platformer2d_actor_monolith/src/shrine.rs:317` assigns
`progress.routed_for = Some(generation)` before the call at line 324 whose
admission result is discarded. The preceding branch returns whenever that latch
already matches the session. In
`crates/ambition_platformer2d_actor_monolith/src/session/lifecycle_commit.rs`,
`PendingLifecycleCommit::record` returns `AlreadyPending` without accepting the
new request if the slot is occupied.

The source comment explicitly permits refusal and relies on a later
`ResetToCheckpoint` as another route. That can be an intentional reset policy;
it does not establish completion of the startup-resume operation. Do not report
this as an unconditional failure on every session start.

### Counterexample to characterize

Start in room A with a valid saved checkpoint in B, a primary body with SimId,
and an already occupied lifecycle slot. Run startup restoration. Retract/cancel
the incumbent without changing the session generation or arriving in B; run the
startup system again. The current latch can prevent it from requesting B.
Confirm that this cancellation/occupancy state is reachable in the production
composition. If the product deliberately abandons startup restoration on any
competing intent, encode an explicit cancelled/completed outcome instead of
naming the state as a successful route.

**Recommended behavior:** an admission refusal leaves startup restoration
retryable; an admitted request is not emitted again while pending. On admission,
completion and later cancellation are distinct states. The first repair need not
introduce a transaction-token framework: only advance the routed latch on an
accepted request and preserve existing once-admitted behavior. A future retry on
post-admission cancellation needs explicit lifecycle outcome evidence, not a
polling guess based solely on an empty slot.

### Tests and invariants

Add proposed tests `checkpoint_startup_retries_after_denied_admission` and
`checkpoint_startup_does_not_duplicate_an_admitted_route`. <!-- cite-ok: proposed test names -->
Keep the existing tests in
`crates/ambition_platformer2d_actor_monolith/src/shrine/tests.rs`, including
`a_checkpoint_in_another_room_of_this_world_routes_the_session_there` and
`two_driven_bodies_resting_at_a_shrine_both_heal_and_write_one_checkpoint`.
Test missing body, missing room, same-room placement and unchanged generation.
The admission policy remains first-admitted-wins, not an implicit queue or
chronological minimum of request timestamps.

## F2. Boss contact paths disagree about authored hurt geometry

**Owner:** simulation target geometry / hit reaction integration.
**Priority:** A2a, before any generic projectile target carve.
**Confidence:** source-established divergent geometry derivation; gameplay
consequences depend on authored geometry and the selected hit path.

### Evidence

`crates/ambition_platformer2d_actor_monolith/src/features/ecs/target_volumes.rs`
publishes boss damageable geometry from authored `ResolvedHurtboxes` and body
kinematics when available, preserving an authored empty set; fallback geometry
uses the boss volume context.

`crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage_predicates.rs`
uses boss-volume-context geometry for boss preflight. The ordinary and
world-kill paths in
`crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/boss_hit.rs`
also derive boss-context geometry rather than consume the same published authored
shape. Their query inputs in `features/ecs/damage/mod.rs` do not establish a shared
published-geometry read.

Old P2 proposed selecting targets through published generic damageable geometry.
Changing only selection would leave the application path with another geometry
interpretation. The result can be a selected/consumed projectile whose victim
application disagrees, or geometry-dependent differences between melee and
projectiles. Existing authored-volume policy requires consistency.

### Required characterization and repair

Construct a boss whose authored hurt volume is disjoint from its fallback shape.
Drive the production publication, selection and application order. Probe a hit
at each shape and an explicitly empty authored set. Record contact, projectile
consumption, damage and emitted reaction separately: invulnerability may reject
damage while physical contact legitimately consumes a projectile.

Choose one simulation geometry snapshot and have selection and reaction consume
it, or use a single resolver with the same authored inputs when a snapshot is not
available at that phase. Do not make the renderer or texture quality authoritative
for hurt geometry. Preserve named special reactions only where their semantics
require them; do not replace all boss behavior with body damage as part of this
repair. Include authored override, fallback absence, empty override, phase
invulnerability, environmental kill and per-frame animation changes.

## F3. Feature contact can run before a nearer blocking world contact

**Owner:** projectile travel/contact selection.
**Priority:** A2b, before claiming a family-free contact seam is behaviorally sound.
**Confidence:** source-established ordering and bypass; concrete production-path
reproduction still required.

### Evidence

In `crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs`, the
`UnresolvedFeatures` branch checks endpoint boss/breakable contact, emits a hit,
despawns and continues. World collision and its finite-body sweep occur after
that branch. Therefore the later sweep does not constrain every feature contact.
The ordinary body-victim path also selects by endpoint overlap, not a sweep over
the full body path, and its center-ray obstruction check is not identical to the
finite-size, projectile-policy-aware world sweep.

### Counterexamples to test

Use a straight, finite-size expiring shot with a thin blocking wall between its
previous location and an endpoint-overlapping breakable or boss. Advance the
actual production projectile step. Assert that the wall wins and the farther
feature is not hit. Put a normal actor wholly between start and endpoint to test
fast-body traversal. Add a corner clip where a center line is clear but the
projectile body is blocked. Test one-way policy separately for bouncing and
expiring shots.

**Do not claim these fixtures already pass or fail in this review.** The source
permits the bypass; a focused executable fixture must establish the reachable
case and keep any deliberate game policy explicit.

### Repair direction

A2b first fixes the obstruction bypass using the current travel policy and shape.
A2c then makes contact selection coherent across bodies, destructibles and world
geometry. Evaluate candidate contacts over the traveled segment(s), select by
contact time with a deterministic tie policy and apply the chosen travel outcome.
Do not sort endpoint centers and call it continuous collision detection. Preserve
returning shots, reflection, absorption, splash, already-hit lifetime and the
existing one-way policy. Do not combine all path/portal/moving-target generality
into an unreviewable first patch.

## F4. Authored interaction values can be accepted without runtime semantics

**Owner:** domain preparation and object runtime; product choices remain Q63.
**Priority:** characterize/diagnose, not an authorization to redesign Interact.
**Confidence:** source-established for the three fields checked below; the broader
existing Q63 inventory is not recertified by this review.

`crates/ambition_interaction/src/lib.rs` declares `Interactable.requires_facing`,
`Pickup.collected` and `Chest.persistent`.
`crates/ambition_platformer2d_actor_monolith/src/features/ecs/spawn_static.rs`
copies those fields into runtime representations. Search across `crates` and
`game` finds no behavior consumer for the first and third, and collection uses
separate live `Collected` state instead of the copied `Pickup.collected` value.
Tests that inspect a copied value do not prove it changes gameplay.

Reproduce the authoring gap with two otherwise identical inputs differing only
in the relevant nondefault field; inspect the prepared diagnostic and resulting
behavior. For a known unsupported nondefault semantic value, preparation should
report an unsupported-field diagnostic rather than imply the behavior is active.
Do not delete authored values or start gating Interact without the relevant
maintainer decision. Supporting the field and rejecting unsupported use are
separate choices. Preserve existing default content behavior while resolving Q63.

## F5. Facade render opt-out does not exclude renderer dependencies

**Owner:** public SDK / Cargo feature composition.
**Priority:** A9 baseline and staged manifest work; independent of SCC count.
**Confidence:** source-established mandatory workspace dependency path and stale
fixture comment. Full Cargo feature closure was not resolved here.

`crates/ambition_platformer2d/Cargo.toml` marks its direct render dependency
optional but keeps `ambition_platformer2d_host` nonoptional.
`crates/ambition_platformer2d_host/Cargo.toml:15` unconditionally depends on
`ambition_render`. A traversal of normal, nonoptional internal path dependencies
finds 51 other workspace packages reachable from the facade. This is a lower
bound, excluding optional edges, feature activation, external packages, build/dev
dependencies and binary dead-code elimination.

The comment in `fixtures/minimal_game/Cargo.toml` implying that removing the
render feature drops the render chain is unsupported by this path. This overlay
does not change that non-planning file; A9 updates it with the implementation.
A headless installation can still work while compiling renderer packages.

Prove the intended contract with an independent consumer manifest and
`cargo metadata`/`cargo tree`, not by inspecting only the facade's feature list.
Split host window/presentation dependencies from simulation-host dependencies,
then check every remaining alternate path. An empty minimal host that never
constructs or advances a body is not the acceptance witness.

## F6. Construction verification detects invalid mutation; it cannot undo it

**Owner:** typed construction and lifecycle publication.
**Priority:** clarify guarantees now; harden only against an explicit failure/
publication requirement in A10.
**Confidence:** source-established contract limitation, explicitly acknowledged
by source comments; not a claim of a newly observed production corruption.

`crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs` gives
`ConstructionExecCtx` raw Commands. `ConstructionPlan` validates parameters,
stable IDs and relations before commit, then runs trusted recipe functions.
`verify_committed_roster` runs after mutation/flush and can reject a recipe that
removed a root, changed its SimId or created an undeclared occurrence. The source
explicitly notes that Bevy Commands do not roll back their mutations.

Consequently, failed preparation preserves the unmodified world, and failed
verification can prevent publication; failed verification does **not** by itself
restore the old world. A unit-returning recipe is not a proof of infallibility.
Trusted Rust can also mutate resources beyond an entity roster.

The immediate contract is fail-closed publication with a stopped/recovered host
on invalid trusted code, not continued simulation of a half-accepted revision.
If last-good-world retention is required, build inactive candidate state,
constrain writes to it and prove cleanup before atomic activation. Arbitrary
Commands cannot provide that guarantee by documentation alone. Test an invalid
recipe that changes a root and one that writes an external resource; do not hide
the second case by checking only entity counts.

## F7. Technique validation is disconnected from installed executable support

**Owner:** domain preparation and technique installation; existing O4 authoring work.
**Priority:** A11, an independent authoring-admission packet.
**Confidence:** source-established absence of production callers of the parameter
registry; a live flow customer/interpreter now exists. No malformed shipped
technique was demonstrated by running the game.

`crates/ambition_entity_catalog/src/lib.rs` defines `EffectRef` with a String key
and opaque parameters. Its `ParamSchemaRegistry` has no production register or
validation callers in the reviewed `crates`/`game` sources. Unknown keys pass its
standalone validation method. The runtime capture consumer in
`game/ambition_demo_smash/src/capture.rs` explicitly acknowledges that hydration
failure is handled during play rather than rejected at startup.

Meanwhile `game/ambition_demo_smash/src/moveset.rs` authors the `read_and_seize`
flow, and `crates/ambition_combat/src/moveset/mod.rs` interprets flows. The old
planning assertions that there is no interpreter and every flow is None are
stale. Graph-shape validation of a flow does not validate the keys/parameters
emitted by its nodes.

Use one installed-technique declaration per owner/profile to couple handler
installation with existence and parameter validation metadata. Keep paramless
known, parameterized known, known-but-not-installed and unknown distinct. Traverse
all semantic effect-reference locations through one domain enumeration, including
flow emits, timeline events, sustained windows and any nested technique payload
that the domain can reference. Validate before publishing prepared content.
Runtime handlers remain ordinary typed domain systems; this catalog is not a
new executable service locator.

### The duplicate-policy rationale is incorrect

The registry's source comment argues that function pointers cannot be compared
as stable content identity, therefore an honest Conflict result is impossible
and last-write-wins is necessary. The first concern is valid; the conclusion does
not follow. Duplicate-key rejection needs only a key-presence check. It need not
prove two functions behaviorally equivalent. Reject every duplicate by default,
or allow an explicit same-owner/revision registration policy with a defined
contract. Never infer executable equivalence from metadata equality.

Use owner/schema/revision identity for diagnostics and fingerprints, not function
addresses. If replacement is genuinely needed, make it an explicit pre-freeze
operation that invalidates preparation, not an accidental insertion overwrite.
Update the source rationale when implementing A11; this overlay only corrects
the planning advice. Tests: unknown key, known paramless, invalid parameters,
missing installed handler, conflicting owners, repeated installation policy and
invalid flow-emitted technique detected before the first tick.

## F8. Flow validation does not establish runtime index/timeout bounds

**Owner:** move-scoped authored flow preparation/execution.
**Priority:** A12, small validation fixes before broader interpreter work.
**Confidence:** source-established validation and representation mismatch;
no such invalid flow was observed in shipped content and Rust tests were not run.

`TechniqueFlow::problems` in
`crates/ambition_entity_catalog/src/lib.rs` checks transition targets against
`nodes.len()` and tests wait timeout positivity. It does not bound node indices
to the runtime cursor representation or require a finite timeout.
`MovePlayback.flow_node` is u16, while flow transitions are usize and are cast
with `as u16` in `crates/ambition_combat/src/moveset/mod.rs`.

A 65,537-node flow whose first Emit targets Finish at index 65,536 is in range
for the authored vector but wraps the runtime cursor to zero. Reachability of
Finish in the authored graph does not prevent that. Positive infinity also
passes the positive-timeout check, but cannot expire under finite elapsed time.
Do not describe this as proof that a fighter stays stuck forever: the enclosing
move duration/teardown also constrains playback. The narrower defect is that
accepted authored control flow can mean something different at runtime.

The interpreter has a per-tick step limit equal to authored node count. That
prevents an unbounded loop in one tick for a fixed graph, but is not an engine-
controlled work budget when graph size is unbounded. It also clones the flow
before each interpretation pass; that is a visible allocation/copy opportunity,
not a measured frame-time bottleneck.

The [authored-technique protocol](authored-technique-admission.md) now fixes
A12's choice: 1-256 reachable nodes, acyclic control flow, checked indices and
finite positive waits, with the existing move-clock lifetime. The game Rust
constructors contain three concrete flows of 3, 3 and 4 nodes, all acyclic.
This is an engineering admission policy, not a measured performance bound or
proof that every external serialized input already fits it. Reject invalid data before publication rather than widening rollback
state as an incidental fix. Inspect direct MovePlayback construction paths as
well as provider preparation. Tests include the boundary indices, infinity/NaN,
empty graph, dangling edge, cycles, and existing normal flows. Keep temporal
signals scoped to their actual per-move occurrence semantics; a second Wait does
not currently mean a second independent hit.

## F9. Raw checkpoint reset readers can mutate domains without room admission

**Owner:** session checkpoint coordination and domain restore reducers.
**Priority:** A1c, after A1a's witness and A1b's move.
**Confidence:** source-established independent raw-message readers; the complete
busy-slot behavior and reachable production cases require the specified Rust
fixture. No executed reset reproduction is claimed by this review.

`restore_occurrence_baseline` in
`crates/ambition_platformer2d_shared_tangle/src/lifecycle/continuity.rs`,
`restore_owned_items_to_checkpoint` in
`crates/ambition_platformer2d_actor_monolith/src/items/pickup/minted_horizon.rs`,
and `restore_custody_to_checkpoint` in
`crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs` read
ResetToCheckpoint and mutate their domain state without receiving the admission
result from session. The custody reader can also materialize restored objects.

`resume_at_checkpoint_on_reset` in shrine separately records the room intent and
only emits RoomReplayAdmitted if accepted. The runtime checkpoint installer puts
routing and domain restoration in the restore phase, but that phase membership
does not communicate whether the slot accepted the request. Moving or ordering
the router alone therefore cannot enforce the slot's documented refusal contract.

Construct a full checkpoint-horizon fixture with differing live/baseline
occurrence, custody and owned-item state, plus an incumbent lifecycle intent.
Emit a reset, advance the actual schedule, flush commands, and assert the desired
contract: incumbent unchanged and zero restore-caused domain/entity/subject
mutation. Check each domain independently so one absent capability cannot make
the test vacuous. Follow with release-of-incumbent, missing-primary and confirmed-
rollback variants.

The target [checkpoint protocol](checkpoint-restoration-protocol.md) pins a typed
snapshot on admission, feeds it to both cached and fresh room preparation, and
applies domain reducers at the common authorized commit. Pre-admission and
preparation failure leave live data intact; post-destructive failure blocks
publication without claiming arbitrary rollback. A single accepted-marker event
is insufficient to prove that preparation and commit used the same snapshot.

## Investigation boundaries, not established bugs

1. Projectile leg reconstruction currently uses current position/velocity and dt.
   Acceleration, returning motion and portals may require actual path segments;
   inspect those implementations and construct a witness before asserting a bug.
2. Global room lookup and stable placement strings need a live-instance namespace
   for repeated simultaneous room instances. That is a multi-instance design gap,
   not proof that the current single-active-room profile is incorrect.
3. Prepared content metadata/fingerprint does not fingerprint arbitrary Rust
   function behavior. Same-build networking must identify compatible executables;
   do not infer a cross-build bug or demand a compatibility layer without checking
   the actual host handshake and the existing same-build policy.
4. Player-only stand breaking may be intentional. Generalizing all body filters
   while moving destructibles changes gameplay and requires a separately approved
   eligibility rule.

## Closure receipt

For each implemented finding, record the exact base, changed owner, reachable
fixture, test command/result, preserved behavior and remaining limitation. A
source inspection is not a passing runtime test; a unit fixture is not a GPU or
P2P receipt. Do not reopen the already repaired mark-clock, hit-flash, spawn or
portal-order defects merely because this review references their invariants.
