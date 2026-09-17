# Architecture review findings requiring implementation evidence

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
This is a bounded source-review finding set, not a replacement issue tracker.
The [queue](../queue.md) chooses work and the
[frontier](actor-monolith-work-frontier.md) supplies packets. Remove a resolved
finding from the live work surface after recording its test and resulting owner.
Git preserves the previous diagnosis, including
[archived epochs](../repository-history.md). Locally absent history is not proof
of a fabricated source reference.

✔ **ALL NINE RE-CHECKED AGAINST HEAD, 2026-09-17, by opening the code each
finding names.** Seven are closed (`F1`, `F2`, `F4`, `F5`, `F7`, `F8`, `F9`), one
is half closed (`F3` — the obstruction bypass is fixed, swept target selection is
`A2c`), and one is an open contract limitation (`F6`). ⚠ **A CLOSED FINDING IS
NOT A CLOSED PACKET:** `F8` closed while its packet `A12b` is two items of four,
and `F9` closed while `A1c/3-5` remains. Each row says which.

**Verification limit:** Rust/Cargo were unavailable in this review environment.
No Rust reproduction, gameplay session, rollback run, GPU run or release build
was executed. That limit describes the REVIEW; the re-checks above were run
against a working toolchain. The distinctions below are intentional:

- **Source-established:** the stated control/data flow is directly visible.
- **Conditional defect:** a supplied state can exercise a bad path; production
  reachability and the intended behavior still need the specified test.
- **Contract limitation:** an existing guarantee is narrower than a plan suggests;
  this is not automatically a new runtime regression.

## F1. Startup checkpoint routing spends its latch before admission — FIXED 2026-09-08

**Owner:** session lifecycle, now implemented and installed by
`crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs` (A1a
repaired the latch, A1b moved the owner). The latch is written only when
`Admission::admitted()`; guarded by
`a_refused_slot_leaves_the_checkpoint_resume_retryable`, poison-verified.
Retained here as the review's evidence at the baseline, not as open work.

### Evidence, at baseline `300004d601af1e633cfaee969f079cf9bb368ca8`

In `crates/ambition_platformer2d_actor_monolith/src/shrine.rs`, line 317 assigned
`progress.routed_for = Some(generation)` before the call at line 324 whose
admission result was discarded. (Both line numbers describe the reviewed
baseline; the code has since moved to `session::checkpoint`.) The preceding branch returns whenever that latch
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

## F2. ✔ CLOSED — every boss damage path reads ONE published geometry

**Owner:** simulation target geometry / hit reaction integration; closed by
`A2a`. **Verified against HEAD 2026-09-17** by opening the three files this
finding names, not by reading a status elsewhere.

`apply_boss_hit` (`features/ecs/damage/boss_hit.rs`) took `boss_catalog`,
`attack_state` and `animation_frame` for one purpose — building a
`BossVolumeContext` and deriving the hurt parts, twice in that function — while
the projectile preflight derived the same fact a third time. It now takes
`damageable: &DamageableVolumes`, and its own comment states what the three
inputs cost: *"a boss's authored hurtboxes governed nothing on the damage road
and an authored EMPTY override still offered a target."* `damage_predicates.rs`
says the same from the other side: *"Actors and BOSSES both answer from published
`DamageableVolumes`."*

**The characterization this finding asked for exists as two arms**, and both pin
the RULE rather than the patch:

* `absent_unpublished_published_and_intangible_are_four_different_answers` —
  absent and unpublished fall back to the coarse box, published non-empty answers
  from the silhouette, published-empty is an authored invulnerable window
  offering no target.
* `a_boss_is_reached_only_through_its_published_volumes` — a boss has NO coarse
  fallback, because *"answering `hit` invents a hull nobody authored — on the
  first eligible tick, which is exactly the frame the contact protocol forbids
  it on."*

⚠ Retained here as the review's evidence at the baseline, the way `F1` and `F9`
are. The paragraphs below describe the tree at
`300004d601af1e633cfaee969f079cf9bb368ca8` and are NOT a description of HEAD.

### Evidence, at baseline `300004d601af1e633cfaee969f079cf9bb368ca8`

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

### The characterization and repair that were required — both done

⇒ Kept as written, because it is the specification the two arms above satisfy and
a reader checking the close should be able to read the requirement beside it.

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

## F3. ◐ HALF CLOSED — the obstruction bypass is fixed; swept target selection is not

**Owner:** projectile travel/contact selection. **Live status is owned by
[`projectile-contact-protocol.md`](projectile-contact-protocol.md), not by this
page** — checked 2026-09-17, and this heading routes there rather than carrying a
second copy that can drift out of step with it.

**A2b landed the half this finding leads with.** Both branches now sweep the
SHOT'S BOX under the shot's own `WorldHitPolicy`: the victim branch used to cast
`raycast_solids` at the victim's CENTRE with `include_one_way = false` hard-coded,
so a wall covering a body but not its centre did not block, a corner clip did not
block, and an `ExpireOnContact` shot damaged through a one-way its own contract
says ends it. The travel leg is captured once before integration, so the segment
cannot describe space the shot never crossed. Witness:
`a_one_way_blocks_the_shot_whose_policy_says_it_should_and_no_other`.

**What this finding asked for that is still open:** contact selection over the
traveled segment with a contact-time ordering — the protocol page's own words are
*"the swept-target half has not"* landed, and `A2c` is open there. ⇒ The
counterexamples below are still the specification for that half.

⚠ Below is the review's evidence at the baseline and is NOT a description of HEAD.

### Evidence, at baseline `300004d601af1e633cfaee969f079cf9bb368ca8`

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

✔ **RESOLVED BY DELETION 2026-09-12 (all three), and a fourth sibling
`BreakableSpec.debris_cue` on 2026-09-17.** None of the three is declared any
more, so preparation cannot accept a value it will ignore. The measurement of all
four, and why the "wiring-or-deleting is a design call" framing was the thing
keeping them alive, is on
[item custody](item-custody-and-accounting.md). The product choices — whether
facing-gating, per-chest persistence and per-pickup collected state are intended
capabilities — stay open on Q63, which is now phrased as those choices rather
than as the fields.

⚠ The original remedy recorded here was *"do not delete authored values ... report
an unsupported-field diagnostic."* A diagnostic is the right answer for a field
the engine INTENDS to support and cannot yet honour; it is the wrong answer for a
field the engine never had a plan to read, because it preserves a public
serializable surface whose only effect is to mislead a third-party provider.

## F5. ✔ CLOSED — the facade's render opt-out DOES exclude the renderer

**Owner:** public SDK / Cargo feature composition.
**Priority:** was A9 baseline and staged manifest work; independent of SCC count.

⛔⛤ **THIS SECTION STATED ITS DEFECT IN THE PRESENT TENSE FOR EIGHT DAYS AFTER
THE DEFECT CLOSED, WITH THE CLOSURE WRITTEN THREE LINES BELOW IT.** The body read
*"`crates/ambition_platformer2d_host/Cargo.toml:15` unconditionally depends on
`ambition_render`"*, while the re-measurement note under it already said *"Three
edges closed … the render path through host"*. A reader stopping at the heading
and the first paragraph — which is most readers — got the opposite of the truth.

**Measured at HEAD, 2026-09-17:**
`cargo tree -e normal --no-default-features -p ambition_platformer2d` reaches
`ambition_render` **zero** times; `-i ambition_render` reports the package is not
in that graph at all. The closure is **48** other workspace packages (49 unique
`ambition_*` names less the facade itself), reproducing the 2026-09-10 reading at
`939d6aaa5` exactly. The host's manifest now says
`ambition_render = { path = "../ambition_render", optional = true }`, and its
`[features]` block explains the split in its own words: the facade takes the host
with `default-features = false`, and that edge was *"the ONE path … that put the
renderer in a no-default-features consumer's compile closure, and it was there
because this edge was not optional."*

⚠ **WHAT THE ORIGINAL FINDING GOT RIGHT AND IS WORTH KEEPING:** 48 is still a
LOWER bound on the facade's weight — it excludes optional edges, feature
activation, external packages, build/dev dependencies and binary dead-code
elimination. The opt-out working for the renderer says nothing about the other
48. ⇒ Reproduce with the command above: count the unique `ambition_*` names and
subtract the facade.

⛔ The 51 in the original reading was the
`300004d601af1e633cfaee969f079cf9bb368ca8` baseline; three edges closed on
2026-09-09 — the render path through host, five dead dependency declarations, and
the map capability.
⛔ **THE UNIT IS THE TRAP.** 49 counts the facade, 48 does not, and this page's
51 is an *other-packages* count. A number that cannot say which it is cannot be
quoted. See `scripts/measure_minimum_profile_parentage.py`.

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
**Priority:** source limitation remains open; A10/I3b now has the explicit
repeated-development-reconstruction customer. I1/I2 do not depend on it.
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
For the supported reload path, build typed inactive candidate state, constrain
writes and prove its visibility/cleanup under A10 before claiming retained-scene
activation. Arbitrary native-plugin undo remains outside that contract. Arbitrary
Commands cannot provide that guarantee by documentation alone. Test an invalid
recipe that changes a root and one that writes an external resource; do not hide
the second case by checking only entity counts.

## F7. ✔ CLOSED — installation declares support, and the barrier refuses on it

**Owner:** domain preparation and technique installation (`A11`).
**Verified against HEAD 2026-09-17** by opening the registry, the join and the
barrier, not by reading a status elsewhere.

Every part of the repair this finding specifies is in the tree:

* **One declaration per installing owner.** `ParamSchemaRegistry` is replaced by
  `TechniqueSupport`, whose doc states the property the old one could not have:
  *"A KEY PRESENT HERE MEANS A CAPABILITY SAID 'I install the thing that answers
  this'; a key absent means nothing does."* The composition's table is
  `InstalledTechniques`, and it stays in `ambition_combat` rather than moving
  down the graph for a consumer's convenience — recorded there as *"DEPENDENCY
  CONVENIENCE WEARING OWNERSHIP'S CLOTHES"*.
* **Duplicates rejected, not silently overwritten** — the exact repair the
  section below demands. `TechniqueSupport::declare` *"refuses a second claim on
  one key instead of replacing it."*
* **One exhaustive enumeration of effect sites.** `MoveSpec::effect_refs`
  destructures without `..` at every level, so a fifth effect site is a compile
  error rather than a silent gap.
* **Validated before publishing.** `unsupported_authored_effects` joins the
  visitor to the table, and `activate_staged_revision` returns
  `RevisionAdmission::Refused` on any refusal — *"admitted against the WHOLE
  candidate, not against the edit alone"*.

⭐ **AND THE ORDERING IS WITNESSED, WHICH IS THE PART A REVIEW WOULD NOT HAVE
THOUGHT TO ASK FOR.** The checked and unchecked roads were two `PreStartup`
systems racing a flag, and a hand-driven `App::update()` does not run plugin
`finish()` — so the fixture won a race the shipped game lost.
`the_barrier_closes_through_the_checked_road_under_the_real_lifecycle` drives
`finish()`/`cleanup()`/`update()` in Bevy's own order, and
`the_shipped_composition_withheld_nothing_at_its_barrier` reads
`AuthoredEffectRefusals` instead of re-deriving the answer.

⚠ Below is the review's evidence at the baseline and is NOT a description of
HEAD; the duplicate-policy section is kept because it is the argument the repair
implements.

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

## F8. ✔ CLOSED — the cursor's width is the authored width, and the timeout must be finite

**Owner:** move-scoped authored flow preparation/execution (`A12`).
**Verified against HEAD 2026-09-17** by opening `TechniqueFlow::problems` and the
`FlowNode` definition.

Both halves of the representation mismatch are gone, and each was closed at the
place that makes the defect unspellable rather than by adding a check:

* **The index.** `FlowNode`'s edges are `u16` — the cursor's own width — so the
  interpreter's `as u16` narrowing is gone with them. Its doc states the defect
  it removes: *"node 65,536 becoming node 0 SILENTLY, a terminating flow turned
  into a loop by a cast, with every edge still in range and nothing for the
  dangling-edge check to see."* Conversion now happens once, at the authoring or
  deserialization boundary, where an out-of-range index is a hard error naming
  the field. The 1–256 node bound still exists, as a READABILITY contract rather
  than as a stand-in for cursor safety.
* **The timeout.** *"⛔ FINITE, not merely positive. `f32::INFINITY > 0.0` is
  TRUE"* — the exact value this finding named as slipping through the positive
  test. `NaN` already failed it and is named in the diagnostic so the message
  says which value was wrong.

⭐ **AND THE FINDING'S OWN CLAIM ABOUT WHAT AN UNBOUNDED WAIT COSTS WAS
CORRECTED, IN THE DIRECTION OF BEING SMALLER.** It is not a fighter frozen for
the match: `MovePlayback::finished()` is `t >= spec.duration_s`, so the TIMELINE
ends the move whatever the flow is doing. What it costs is every `Emit` after the
wait never firing — the authored sequence silently doing half its job. This
finding's own text already declined the larger claim (*"do not describe this as
proof that a fighter stays stuck forever"*), and the runtime confirmed it.

⚠ Two other observations here were NOT defects and are not claimed closed: the
per-tick step limit equal to node count, and the clone before each interpretation
pass. The node bound makes the first finite; the PER-TICK clone is gone
(`Arc::clone(&pb.spec)`), and a per-move-START clone remains — `StartingMove.spec`
is a `MoveSpec` by value out of `MovesetContract::moves`.

⚠ **THIS FINDING IS CLOSED; ITS PACKET IS NOT.** `A12b` still owes private
fallible prepared constructors and a prepared REVISION pinned on the playback —
see the frontier's sub-packet table. F8's two specific defects are what closed.

⚠ Below is the review's evidence at the baseline and is NOT a description of HEAD.

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

## F9. Raw checkpoint reset readers can mutate domains without room admission — FIXED 2026-09-08

**Owner:** session checkpoint coordination and domain restore reducers.
**Confidence at review time:** source-established independent raw-message
readers; no executed reset reproduction was claimed. **Since executed:** the
fixture the review asked for was built (A1a) and it found the defect to be
sharper than described — a refused reset did not merely mutate domain state, it
DESTROYED an object acquired after the checkpoint, because custody restoration
removed it from the hand while the room reconstruction that re-authors it was
exactly what the refusal cancelled.

**Fixed by** A1c/1-2: one `AdmittedCheckpointRestore`, written only by the
session coordinator and only with an `Admission` in hand, is what every domain
reducer reads; an ordered `CheckpointRestoreStep::{Admit, Apply, Retire}` chain
makes the answer available before any domain acts; and a refused request is
remembered in `OutstandingCheckpointRequest` rather than dropped. **Guards:**
`a_refused_reset_changes_no_domain_state_and_is_not_lost` (both halves
poison-verified) and `every_checkpoint_restore_system_is_inside_one_ordered_step`
(asks the schedule, and fails on anything installed into `CheckpointRestore`
outside the three steps). Retained below as the review's evidence at the
baseline; the pinned-snapshot half remains A1c/3-5.

`restore_occurrence_baseline` in
`crates/ambition_platformer2d_shared_tangle/src/lifecycle/continuity.rs`,
`restore_owned_items_to_checkpoint` in
`crates/ambition_platformer2d_actor_monolith/src/items/pickup/minted_horizon.rs`,
and `restore_custody_to_checkpoint` in
`crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs` read
ResetToCheckpoint and mutate their domain state without receiving the admission
result from session. The custody reader can also materialize restored objects.

`resume_at_checkpoint_on_reset` (in shrine at the reviewed baseline; now
`session::checkpoint`) separately recorded the room intent and
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

## A SECOND REVIEW, 2026-09-17 — six findings, and this page is not their home

⛔ **THIS PAGE'S NINE ARE A BOUNDED SET AT `300004d6`; these six are a different
review of a different window** (`90b7135..027915a`, 89 commits) and are recorded
here only so a reader asking *"what have reviews found"* gets one answer. Each
disposition was re-derived by opening the code before acting, and each landed
change carries its reasoning in its own commit.

| # | finding | disposition |
|---|---|---|
| 1 | Q142 framed three rollback defects as a maintainer decision | ✔ FIXED — `EncounterScript`, `ReleaseOnDeath` and `RecharacterizeBody` registered, schema v197 → v198; `PostBossNpc` stays open because its question is what an admitted REPLAY sweeps |
| 2 | the frame-zero carrier rebase cannot see a hidden construction candidate | ✔ FIXED — it now counts carriers through `count_matching_including_hidden_candidates` and REFUSES rather than rebasing a partial population; ⛔ including candidates would be worse, because an order INDEX is positional and a candidate on one peer only shifts every index after it |
| 3 | the rebase fails open on a missing or duplicate `SimId` | ◐ FILED AT THE CODE — the alternative today is not a refusal, it is keeping the App-lifetime history, which is wrong by more. The condition that flips it is a real remote peer, and the comment says so |
| 4 | `clean_workspace_crates.sh` released the build lock before deleting | ✔ FIXED — an open descriptor is held across `du`/`find`/`mv`/`rm`. Measured both ways on a 9,000-file tree: 0 lock steals against 12 |
| 5 | the Fade audit counted one authored fade where three ship | ✔ FIXED — `test_intro`, `intro_wake`, `drain_market_arrival`; 2.2 s of invisible wait across three rooms. The planning page that quoted the number was the second copy and is corrected too |
| 6 | the audit campaign is drifting into machinery `AGENTS.md` forbids | ◐ PART — the source-comment PATH gate is demoted to reporting; the writer-set ratchet keeps its ratchet and now states, at the top of the file, that moving a writer between schedules leaves it green and what behavioural arm should replace it |

⚠ **WHAT IS NOT DONE:** the rewind arm for `ReleaseOnDeath` and
`RecharacterizeBody` (`EncounterScript` has one — 945 frames of clock drift
against 1 tick when the registration is removed), and the `CutsceneTriggerQueue`
behavioural arm that would let its ratchet be deleted. Both are named at the code
that owes them.

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
