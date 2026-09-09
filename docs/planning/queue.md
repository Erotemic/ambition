# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript, campaign archive or place to preserve completed investigations.
Git history owns those records, including intentionally retired epochs in
[the cold history store](repository-history.md).

A row stays here only when an engineer can act on it without first reconstructing
weeks of context. Durable design belongs in the linked owner document. Product
questions belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

**Architecture review source:** `300004d601af1e633cfaee969f079cf9bb368ca8`
(2026-09-08 committed archive). Revalidate before changing a newer head. The
review made no Rust execution claim; see the coverage receipt.

## P0 - characterize and repair current correctness gaps

### A1a - DONE 2026-09-08

`restore_checkpoint_on_session_start` latched `routed_for` before asking the
lifecycle slot and threw the `Admission` away, so a refused crossing spent the
session's one resume and stranded the player in the room the session opened in.
Fixed by latching only on `Admission::admitted()`; guarded by
`a_refused_slot_leaves_the_checkpoint_resume_retryable` (poison verified) and the
missing-subject arm beside it. A1b then moved the state, systems, installation
and tests out of `shrine`/item-pickup into `session::checkpoint`. F9's executed witness landed
with it: on a refused reset the entitlement ledger rolls back **and the object
acquired after the checkpoint is destroyed outright**, because custody
restoration and the room reconstruction that would re-author it fall on opposite
sides of an admission neither consults. Measurements and the A1c obligation are
in the [protocol](engine/checkpoint-restoration-protocol.md#f9-measured-2026-09-08-executed-full-checkpoint-horizon-composition).

**Standing prohibition:** no consequence of a lifecycle request may be written
before the slot has said yes.

### A2 - unify projectile contact geometry and obstruction semantics

**Owner:** [projectile contact protocol](engine/projectile-contact-protocol.md),
packet A2 and findings F2/F3.

Establish shared geometry, actual travel legs and finite-shape obstruction order
before replacing family dispatch. Preserve synchronous interception and later
hit reception. Carry collider contributor identity: a destructible's own wall
and its hurt shape can be one compound contact, not competing unrelated targets.

**A2a landed 2026-09-09** (one boss hurt geometry, published once at the
damage-facing sample; the catalog and the attack/animation inputs left the
consumers with the derivation). **A2b's obstruction half landed**: the travel leg
is captured rather than reconstructed, and both branches ask one swept question
with the shot's own box and world-hit policy. Receipts in the owner document.

**Swept target contact landed 2026-09-09**, with candidate ordering by time of
impact and the targeted hit event carrying the box AT CONTACT — the delayed
applier re-tests that volume, so a shot that genuinely crossed its target used to
fail its own re-test and land nothing.

**The boss/breakable branch is swept too, 2026-09-09**, and the receiver-neutral
returning-shot lifetime landed with it as its own semantic change: that branch
despawned every shot that reached a boss or a breakable, so the same boomerang
came back from a body and vanished into a crate.

**Direct-before-splash and targeted delivery both landed 2026-09-09.** A
projectile's direct contact now names its boss or breakable recipient
(`HitTarget::Feature`, schema 177) instead of broadcasting a volume the applier
re-scans — which damaged every breakable that volume overlapped and let query
order pick which part of a multi-part boss was credited.

**The family-predicate deletion gate closed 2026-09-09**: all three discrete
`ecs_hit_event_hits_*` predicates had no production caller once contact became
swept, and are gone. One of them had none beforehand either — reachable, tested,
unreached — so its rule moved onto the function every consumer actually calls.

⚠ That paragraph and the ones above it once read as though every A2 correction
had landed. They had not: world-versus-target ordering was still split when they
were written, which is the defect the review found. The claim is scoped to the
deletions and the swept-contact corrections it actually names.

**A2b was REOPENED and re-closed 2026-09-09 (GPT review #7).** The ordering the
protocol asks for was still split three ways, and the review was right:

- the BOSS/BREAKABLE branch compared nothing. It computed a swept contact and, if
  it found one, emitted the targeted hit and the splash and `continue`d — so the
  world sweep ran only when NO feature was reached. The ordering was *"feature
  contact, else world"*, and a crate or a boss standing behind an unrelated solid
  was struck through it;
- the BODY branch asked the right question of the wrong geometry: it swept from
  the muzzle to the victim's CENTRE, which answers *"is a wall before the
  victim's middle?"*. On a wide body the shot reaches the near face first, and a
  wall between that face and the centre refused a hit that physically happened;
- and the world branch swept a third time for its own pull-back.

⇒ ONE sweep over the shot's actual travel leg, its finite `time_of_impact` kept
in the leg's own [0, 1] parameter and compared directly against every body's and
every feature's contact time. A blocker strictly earlier wins; a tie goes to the
target, which restates the strict comparison the body branch already used rather
than inventing a policy (nothing can yet tell a destructible's own surface from
an independent blocker — that is A5's contributor identity). The pull-back reuses
the same result instead of re-sweeping. Three witnesses, each poison-verified
against the exact code it replaced: a crate behind a wall is not broken, the same
crate with the wall moved PAST it still is (the anti-vacuity floor moves the wall,
not the crate, so an arm that broke nothing would fail it), and a wall standing
inside a wide body past its near face no longer saves it.

**A2's remaining item is deliberately not taken.** A cohesive
`projectile/contacts.rs` <!-- cite-ok: proposed module path --> inside the same
crate removes no authority and no dependency edge — the code would import what it
imports now and be reachable by the same callers — and a same-crate move that
names neither is churn by this repository's own test. It becomes worth doing when
it enables a deletion: a `pub(crate)` boundary, or a split that lets flight state
leave for `ambition_projectiles` without the victim queries following. The semantic corrections and
deletion gates the owner document requires have landed, INCLUDING the finite-time
ordering rule reopened by review #7 above.

⚠ WHAT IS DELIBERATELY NOT CLOSED, stated rather than implied: the acceptance
matrix's COMPOUND SOLID OBJECT row. The tie rule this road runs is declared and
uniform — a strictly earlier contact wins, and an equal time goes to the target
(against a wall) and to the body (against a feature) — but a genuine compound
contact, where a destructible's own collision surface and its damageable volume
are ONE contact rather than two competitors, needs stable collider-contributor
identity. That is A5 infrastructure.

⭐ **AND THE CASE IS NOT REACHABLE ON THIS ROAD AT ALL, measured 2026-09-09.** A
breakable authored `BreakableCollision::Solid` DOES publish a `BlinkWall` block —
`world/overlay.rs` writes it into `FeatureEcsWorldOverlay::blocks` — but
`ambition_projectiles::collision_world::ProjectileCollisionWorld::solids()`
composites only `gate_solids`, `portal_carves` and `removed_block_names`.
`overlay.blocks` (every ECS breakable surface and every pogo orb) is not in the
world a projectile sweeps. So a solid crate's own surface is not a wall this road
can hit, and the tie rule has no compound case to get wrong yet.
⛔ A first attempt to witness the compound case here produced a test that passed
under a deliberately broken comparison TWICE — once because the fixture never
published the surface, and once because the shot's landing splash broke the crate
whether or not the direct hit landed. It was deleted rather than kept as a green
row that measures nothing. Whether a projectile SHOULD collide with an ECS
breakable's published surface is a separate open question, not this packet's.

**Acceptance:** authored-empty geometry, thin wall/target, equal-time ties,
compound solid object, reflection/absorption, returning shots and rollback have
explicit production-road outcomes. The initial sampled-target sweep is not a
claim of full moving-target CCD. No second family query chooses the victim.

### A11/A12 - make authored technique admission truthful and bounded

**Owner:** [authored technique admission](engine/authored-technique-admission.md).

**A12a landed 2026-09-09.** The flow validator now rejects an infinite timeout
(`f32::INFINITY > 0.0` is true, so the mandatory-timeout check admitted the exact
value it forbids), a graph past 256 nodes (a cursor bound: `flow_node` is a `u16`
written with a narrowing cast, so node 65,536 silently becomes node 0), any cycle
reachable from node 0 (`reaches_finish` is existential, so a branch that
terminates on one road and loops on the other passed), and a node nothing arrives
at. The flow-owned-lifetime prose is corrected against the runtime: the timeline
ends the move, so none of these traps a fighter — they lose authored steps, and a
cycle fires a technique N times where the author wrote one.

**A11a's support authority landed 2026-09-09.** `TechniqueSupport` refuses a
second claim on a technique key and refuses an authored key nothing installed
declares; `install_technique` adds the handler system and declares its key in one
statement, so a capability cannot install one without the other. Four handlers
converted. It replaces `ParamSchemaRegistry`, which admitted unknown keys by
design, overwrote duplicates silently, and had zero production callers — so a
misspelled effect key reached the runtime and became a warning mid-fight.

**A11b's exhaustive visitor landed 2026-09-09.** `MoveSpec::effect_refs` returns
every authored effect with the path it sits on, destructuring without `..` at
every level so a fifth reference site is a compile error rather than a silent gap.
It replaced two hand-kept lists that each read two of the four sites — including
the bare-specials census whose own doc records it having already missed a site
once, and the held-item guard, where the miss means shipping a placeholder quad.

⚠ **The remaining seventeen technique installers are all in
`game/ambition_demo_smash/**`** (measured 2026-09-09 by locating every handler),
which is the Smash lane. The four the engine composition installs are converted.
Until those declare their keys the strict unknown-key pass cannot turn on, because
it would reject every authored use of them.

**A12b's structural half landed 2026-09-09.** `FlowNode`'s edges are the runtime
cursor's own `u16`, so the interpreter's narrowing `as u16` — which turned an
authored edge of 65,536 into node 0, a terminating flow into a per-tick loop, with
nothing in the pipeline able to report it — is gone with the type that allowed it;
the 256-node bound is a budget again rather than a stand-in for cursor safety. The
witness sits at the deserialization boundary because that is the only road the
defect was reachable on: every in-repo flow is Rust literals. `MovePlayback::spec`
is an `Arc<MoveSpec>`, so a move can no longer edit the definition it is executing
(the contract's rule, previously unenforced) and the per-tick deep clone of the
authored graph is deleted. `TechniqueFlow::successors` is now the one edge
enumeration its own doc claimed it was — `reaches_finish` and the dangling report
each had a second and third copy.

Next in this lane: A11b's rejection half — poison a reference site and verify the
active registry and generation are unchanged — plus domain nested-reference
policies and the public production insertion paths. Then A12b's remainder: the
prepared constructors are still public and infallible, and no prepared REVISION is
pinned on the playback.

**Acceptance:** invalid/uninstalled calls cannot publish definitions; rejection
leaves active generation unchanged; existing 3-/4-node flows retain their traces.
Finish does not remove recovery, Wait does not extend the move, and late contact
feedback cannot mutate another move occurrence. No generic execution registry.

## P1 - ownership and independently testable composition

### A1c - DONE 2026-09-08; A1 CLOSED by review #7, 2026-09-09

**Owner:** [checkpoint restoration protocol](engine/checkpoint-restoration-protocol.md).

**Signed off 2026-09-09.** Review #7 accepts A1 as closed: the rollback host
checks `LocalSyncTest` ownership before terminalizing a host-local preparation
failure, respects the confirmed-frame boundary, and the abandonment note carries
enough to reject a stale note from a rewound branch rather than matching a reused
operation key. The unconfirmed-abandonment and key-reuse tests are named as the
right witnesses. Delete this row when the next queue pass compresses it.
A1b (ownership move) and A1c subcommits 1-2 landed 2026-09-08: no domain reads
the raw `ResetToCheckpoint` any more, a refused request changes no domain state,
a refused request is remembered rather than lost, and a no-item checkpoint
composition works.

Subcommit 3's preparation half landed 2026-09-08: the accepted operation now
outlives its frame, carries the occurrence/minted inputs pinned at admission, and
room loading derives both the prefetch cache key and the fresh plan from it
instead of from a live ledger the restore had swapped in order to be read.

Subcommit 3b landed 2026-09-08: destructive domain application left ordinary
speculative simulation for `CheckpointDomainApply`, a schedule only a commit
executor runs, with its inputs installed for that schedule's duration and removed
on every path. Both executors are witnessed and poison-verified. The transitional
`AdmittedCheckpointRestore` token was deleted rather than given a longer
lifetime: authorization is structural now.

The accepted operation now has an identity — `CheckpointOperationKey`, the
session's ownership stamp plus an admission-only sequence — and every stage after
the transaction opens matches on it rather than on intent equality.

Verification, terminal outcomes and startup unification landed 2026-09-08: the
commit checks the applied world against the snapshots the operation was accepted
with, publishes exactly one outcome per key, blocks gameplay on failure, and
startup routing is now the same operation mechanism —
`CheckpointResumeProgress` is deleted rather than renamed.

Verification now covers the ledger, the bag, the room the operation claimed to
reconstruct, the subject it restores around, and — per banked custody row — that
the occurrence is carried, carried ONCE, and carried by the custodian the
checkpoint names. Each has a case that fails for it alone. The operation
counter's overflow path refuses the lifecycle slot rather than the identity
(poison-verified), the key has one canonical projection, and the terminal outcome
is a closed `RestoreFailure` rather than a free-form string.

The acceptance matrix was audited row by row on 2026-09-08 and every one of its
**seventeen rows now has a witness that fails for that row's property**; the
per-row receipts are in the protocol.

⛔ **TWO ROWS WERE CLOSED ON A STRUCTURAL ARGUMENT AND BOTH ARGUMENTS COVERED
HALF A ROW.** "Nothing destructive runs before the commit" proves *retain live
state* and says nothing about terminalization or permanent retry — both were
broken. "Every cached plan carries the default outlook and `promote` refuses a
different one" was sound about the comparison and silent about whether the
comparison was ever reached: it was not, because the prefetch cache's identity
had two keepers and only the host's copy was ever set, so the first transition of
every session cleared every warm plan before looking one up. **A structural
argument is evidence; a row closes on a test that fails for it.**

⭐ Repaired with the witness: `PrefetchIdentity` is one value that `publish`
requires, so a plan cannot enter the cache without the cache knowing the world it
was prepared for, and there is no public reset.

⛔ **THE TERMINAL ROAD'S ROLLBACK BOUNDARY WAS THE THIRD HALF-REPAIR IN THIS
PACKET.** Ending a failed preparation moved off `Update` onto a commit boundary,
which fixed the schedule but not the authorization: the host-side abandonment
note could still spend rollback state before the session-ownership gate and
before its operation's admitted frame was confirmed, and a note held only a KEY
while the sequence counter that mints keys rewinds. Repaired and witnessed on
both properties. The P2P road is deliberately inert — the terminalization sits
below `commit_confirmed_lifecycle`'s `LocalSyncTest` gate, because ending an
operation on a local asset failure is a lifecycle decision only an owning host
may make. A coordinated peer-level rule is a new packet.

**Recorded as deliberate non-goals, not open work** — each needs a NEW DECISION
to become a task:
- ⛔ verification's remaining omissions are DELIBERATE, not pending: body
  placement has no comparand but the arrival, which transit legitimately
  reconciles off, and clocks/portals have no accepted snapshot at all. Checking
  either needs a new decision (a transit postcondition; a snapshot of what a
  checkpoint means for a clock), not another assertion. Population completeness
  landed with its own failing case;
- the terminal outcome has no presentation consumer. When one is wanted, publish
  a message at completion rather than polling the session's single-latest-outcome
  resource.

**Acceptance:** preparation/prefetch read the pinned snapshot rather than a
modified live ledger; a checkpoint change invalidates a prefetched plan that
would produce a different population even at the same target room; final
verification and rollback rebase include restored custody/occurrences. A failed
destructive native apply remains fail-closed, not an invented undo guarantee.
Retain the corrected actor-spawn boundary.

### A3 - relocate actor-specific world placement lowering

**Owner:** prepared construction integration; packet A3.

**Re-measured 2026-09-09, and the row's edge was already gone; a different one
was not.** `ambition_platformer2d_world`'s manifest names no actor crate at all
and its `LoweringCtx<C>` is generic, so the queue's stated acceptance — *"world
no longer imports actor preparation solely to lower a placement"* — held at HEAD
before this row was touched. The frontier's "world region" means the MONOLITH's
`src/world/`, where the coupling measured as two files: `placements.rs` (119
lines: the context struct and three type aliases) and `rooms/stage.rs` (two
production signatures). The actor-specific lowering implementations the frontier
says to move alongside are already under `features/ecs/spawn/**`.

**The real defect the preflight found, and it landed 2026-09-09:** the five
fields of one authored snapshot reached their single assembly point by TWO
carriers. `prepared`, `brain_profiles` and `forced_brains` arrived on
`ActorConstructionContext` — whose own doc says the authorities are parameters of
one value precisely so a road cannot forget one — while the CHARACTER CATALOG and
the AUTHORED SHEETS were bare positional arguments threaded through
`RoomConstructionPlan::prepare_from_parts`, `prepare_spec` and
`RoomFeatureConstructionPlan::prepare` for the sole purpose of being assembled
beside them. They ride on the context now; three signatures each lost two
parameters (9→7, 9→7, 7→5) and all three shed
`#[allow(clippy::too_many_arguments)]`. `SimulationSetup` lost both fields too —
setup reads `construction.characters`, so one value carries the catalog where
three spellings did. Poison-verified: emptying the catalog at the assembly point
reddens 17 `app_it` tests (0 in the monolith's own 1,108, which is a fact about
where the coverage is).

**What remains is a file move that removes no edge:** `ActorPlacementContext`
sits in `src/world/placements.rs` rather than under `construction/`. Do it when
something else opens that file; do not spend a commit on it.

**Acceptance:** met — provider validation, body construction and failure behavior
remain covered, and no executable type-erased recipe registry replaced the direct
adapter.

### A9 - establish truthful minimal engine profiles

**Owner:** public SDK and composition; packet A9.

Record the Cargo feature closure of real external fixtures. Separate compiler
reachability, runtime installation and public-import ergonomics; repair one
dependency path at a time.

**The render path closed 2026-09-09, and it was one edge.** Re-measured at HEAD
with `cargo tree -e normal --no-default-features -p ambition_platformer2d`: 51
other workspace packages, matching the number this row already carried, and
`-i ambition_render` named exactly ONE path — `ambition_platformer2d ->
ambition_platformer2d_host -> ambition_render`, non-optional in the host's
manifest. The host's camera, projectile-visual and fx-pipeline plugins are behind
its own `render` feature now (which carries `ambition_menu` and
`ambition_sprite_sheet` with it, since nothing outside that feature's code named
them); the crate's own default stays `render` so building it alone still builds
the windowed face its description promises, and the facade takes it
`default-features = false` and forwards it from its `ambition_render` feature.
Closure 52 → 50: `ambition_render` and `ambition_sprite_fx` left.

⚠ **The existing capability-footprint sentinel cannot see this**, and that is why
a second contract exists rather than a wider baseline: `fixtures/minimal_game`
ASKS for the renderer (its exit criterion draws a windowed face), so the renderer
is legitimately in its closure. `the-featureless-facade-links-none-of-these` in
`scripts/check_absence_contracts.py` walks the feature-resolved tree instead —
the manifest walk the other dependency contracts use counts optional edges and
would report a renderer no feature enables. Poison-verified twice: restoring the
facade's `default-features` on host reddens it naming both crates, and pointing
its `cargo tree` at a package that does not exist trips the anti-vacuity floor
rather than printing `ok` on an empty measurement.

**Five dead dependency declarations removed the same day.**
`scripts/measure_unreferenced_workspace_dependencies.py` (committed with the
change, poison-verified by putting one back) found four crates declaring an
`ambition_*` dependency their whole source tree never names. All five lines were
genuinely removable — the compiler is the judge and it agreed:
`ambition_abilities -> ambition_boss_encounter` (which said "an abilities crate
needs a boss system" to every SCC measurement and dragged encounter, persistence
and cutscene behind it in a manifest walk) and `-> ambition_gameplay_trace`;
`ambition_touch_input`'s `mobile_touch -> ambition_cutscene`; the facade's
`all_capabilities -> ambition_sfx_bank`, a capability name activating a crate the
facade never re-exports.

⛔ **And one of them was a FEATURE THAT PUBLISHED NOTHING.**
`ambition_characters`' `causal` said *"publish this capability's causal facts
(brain decisions, for now)"* and the crate contained no `cfg(feature = "causal")`
and no reference to `ambition_causal` at all — a composition could turn it on,
pay the compile, and receive no brain decisions, with the manifest comment
asserting otherwise. Deleted along with the monolith's forwarding of it; the
other three `causal` forwards (`combat`, `damage`, the monolith's own) are real.
⚠ The feature-resolved closure did not move: every one of those crates is also
reached through `ambition_platformer2d_actor_monolith`, which is the packet's own
thesis rather than a reason to leave a false edge standing.

**And the map became a capability, 2026-09-09.** Tracing every crate in the
featureless closure to its activating parents (the frontier's "trace every
alternate path") found exactly ONE single-parent capability edge:
`ambition_menu <= ambition_platformer2d_runtime`, which installed
`MapStatePlugin` and `install_map_simulation_systems` unconditionally. The facade
already OFFERED `ambition_menu` as a named capability — an optional capability
with one unconditional installer is not optional, and that is what kept the menu
crate in a movement-only game's closure. It is behind the runtime's `map` feature
now, forwarded from the facade's `ambition_menu`, so naming the capability
installs it rather than linking a crate nobody steps.
`ProgressionSet::Map` is `shared_tangle`'s and stays configured either way.
Closure 50 → 49, and RATCHETED: `ambition_menu` joined
`the-featureless-facade-links-none-of-these`'s forbidden set, so the promised
absence is checked and not merely claimed — poison-verified by giving the runtime
a `default = ["map"]`, which reddens it naming the crate. ⚠ Scoped to THIS
PROFILE: `ambition_platformer2d_host` legitimately links the menu crate under its
`render` feature for the `MenuFont` handoff, so "the menu is never linked without
the map" would be false. The positive wire has its own witness — dropping the
facade's forwarding reddens
`every_room_the_map_calls_visited_has_its_visit_on_the_save`.

⚠ The trace's real finding is the shape, not the win:
`ambition_platformer2d_runtime` and `ambition_platformer2d_actor_monolith` are
parents of nearly everything, and `ambition_platformer2d_core` has 33 parents.
Every remaining crate has two or more parents, so no further SINGLE-EDGE closure
decrement exists.

⛔ That is a statement about the graph and nothing else, and it must not be read
as "the monolith carve is next". A closure measurement cannot say which ownership
change is semantically correct — the `actor_spawn` carve is this repository's own
receipt for that, where a green SCC number sat beside a live view the extraction
had taken with it. Any further reduction here has to establish STATE, BEHAVIOUR
and INVARIANT ownership first and let the closure follow, not the other way
round.

**Next in this row:** the frontier asks for a SEPARATE headless consumer
workspace — "a constructed body advancing against world geometry, without
renderer, audio, inventory, encounters or game content". That fixture does not
exist; `minimal_game` is the windowed sentinel and cannot stand in for it. The 49
crates that remain in the featureless closure have been traced to their
activating parents (see above); what has NOT been established is which of them a
minimum profile has a right to expect, which is that fixture's job.

**Acceptance:** a supported profile constructs and steps a real subject, its
promised absent capability is absent from both installation and resolved closure,
and the full Ambition composition continues to work.

### C2 - capability-owned installation, only where ownership is established

**Owner:** [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

Fresh source instruments report 0 capability/ruleset private orderings,
73 composition private orderings, 174 foreign installations, and 3 mechanically
reducible versus 38 irreducible installation blocks. These are locator metrics.

Move a reducible block only after establishing one implementation owner. Keep
cross-capability policy in explicit composition. Resolve Q73 before adopting a
plugin form that conflicts with the recorded combat convention; an owner helper
can be sufficient. Zero foreign installations is not the target.

**Re-measured and one block moved, 2026-09-09.** The instrument reports 2
reducible / 39 irreducible at HEAD (was 3/38 when this row was written; the
counts drift with the composition, not with progress). Both reducible blocks were
in `ambition_platformer2d_host`, both installing `actor_monolith` input systems.

The frame-to-tick latch drain moved:
`install_latched_slot_publication` sits beside `install_roster_seating`, whose own
doc already made the argument — *"the order is this crate's fact, not the
composition's"*. The host had spelled the system, its sim phase, the
`InputSet::Route` edge it must precede, and the fixed-tick condition, and reached
for `app.sim_schedule()` to do it; it names one function now. Reducible 2 → 1.

⛔ **The last one is DECLINED, and the reason is the row's own rule.** It is
`.add_systems(Startup, spawn_primary_input_participant)` — one system, no
ordering anchors, which is exactly the shape the instrument cannot classify: it
sees "one capability could install this" and cannot see that what the composition
is deciding is *that this app has a person in front of a controller*. A headless
or RL composition installs the monolith and wants no leafwing participant. No
implementation owner is established, so it stays.

⛔⛔ **AND A POISON THAT PASSED FOUND SOMETHING BIGGER.** Making
`install_latched_slot_publication` a no-op leaves all 602 `app_it` tests green.
`ambition_platformer2d_runtime::input_drive::drive_slot_frame` — the helper every
scripted test drives input through — writes the latch *if the latch table
exists*, and otherwise falls through to writing `SeatRawFrames` and `SlotControls`
DIRECTLY. So a scripted press is delivered whether or not the drain ever runs:
the fallback makes the drain unfalsifiable from the road every test takes. The
gap predates this move (the move is registration-identical) and is recorded here
rather than papered over with a test that constructs its own subject.

**Acceptance:** public set ancestry and deferred visibility are covered, optional
capabilities remain optional, and no broad runtime policy object replaces imports.

The remaining A4-A8/A10 packets have evidence-based holds in the frontier. They
are not executable queue commitments. SCC counts do not release those holds.

## P2 — current engine/game work

### D-TETHER-LINE — give the ledge tether a readable generic reach line

**Owner:** [`engine/expressive-move-capabilities.md`](engine/expressive-move-capabilities.md).

The reel is mechanically visible through movement but has no attachment line.
Do not teach `sim_view` about the Smash-specific `TetherReel` component. Publish a
generic body-to-world reach/attachment fact from gameplay and let the existing
presentation line road consume it.

**Acceptance:** diagonal ledge tether draws from body to actual anchor, Performer
flyline/grab reach remain correct, and no engine/view crate imports Smash ruleset
state.

### D-POTATO-ASPECT — resolve tier-dependent character trim/aspect drift

**Owner:** [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).
**Maintainer choice:** Q69 in the decision ledger covers the interim `0_25x`
fallback policy.

**Do:** keep generated-tier measurement explicit; do not turn missing ignored
manifests into a pass. Fix generation/trim semantics so a selected tier preserves
the promised frame geometry.

**Acceptance:** same authored frame at full/half/quarter/potato has bounded
anchor/aspect drift; potato does not receive fewer drawable pixels than its
selected quality contract promises.

### D129 — finish authored-geometry sprite clipping repair

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md)
and current renderer geometry contracts.

Work from the current measured clipped-sheet population, not historical counts.
Fix per character/sheet where authoring is genuinely wrong; do not introduce a
global scale heuristic to erase asset mistakes.

**Acceptance:** render-time clipping warning population decreases for intentional
repairs and unchanged composited/tiling cases stay classified rather than hidden.

### D-BRAIN-MENU — make the fighter brain able to order from its authored move menu

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md).

The generic fighter brain can have legal authored attacks that its scoring shape
never selects. Implement the owner doc's current scoring/menu packet; do not add
per-character special-case button scripts.

**Acceptance:** representative CPU can select movement-compatible attacks,
smashes/charged options become live customers where the authored menu permits
them, and easiest difficulty remains intentionally poor rather than suicidal.

### D72 — continue Smash parity from the inventory, not a campaign diary

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

Choose the highest-priority remaining parity row whose primitive is not blocked by
a maintainer decision. Implement through reusable engine capability when the move
class is reusable; demo-only policy stays in Smash.

**Acceptance:** update the inventory row and add production-path acceptance. Do
not append another chronology to a retired expressive-moves campaign.

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

Continue only from the owner doc's measured census. Migrate a field when there is
a real duplicate/competing authoring authority, not because a struct looks large.

**Acceptance:** one authoritative authored value, all runtime projections derive
from it, and the old duplicate path disappears.

### D-SCENARIO-IDENTITY — finish scenario identity transport/cache ownership

**Owner:** performance/scenario tooling.

The report-side identity distinction is complete; remaining work is transport and
cache identity. Unsupported geometry must continue to refuse rather than staging
Flat data under a different scenario name.

**Acceptance:** two scenario geometries with the same benchmark knobs do not
share a cache/result identity; unsupported geometry exits as unsupported.

### Closed by re-derivation, 2026-09-09

⛔ **D-PORTAL-INTERACT-SEAT.** Re-derived against HEAD rather than inherited, and
all three of its acceptance clauses already hold:

- *two driven bodies can independently interact in the same tick* —
  `two_driven_bodies_each_flip_their_own_switch`, beside
  `a_second_seat_spends_its_own_buffered_interact` and
  `a_seat_that_pressed_nothing_does_not_interact_on_another_seats_press`;
- *portal presence does not change which semantic interaction intent exists* —
  vacuously, and measured: `grep -c 'portal\|Portal'` over the whole 275-line
  interaction system is ZERO. There is no portal dependency to remove;
- *no query-order `.next()` arbitration returns* — the one `.next()` left is the
  documented startup fallback for the frame before any seat is attached, not an
  arbitration among candidates.

⚠ RECORDED RATHER THAN SILENTLY DELETED, because a row that describes a defect
nobody can reproduce costs the next reader the same re-derivation. The rule this
follows is the file's own: a row stays only when an engineer can act on it.

### Closed 2026-09-09 — D-RESET-ROAD-RESIDUE

`ActorMutIntegrationExt::reset_to_spawn` restored an enemy's spatial baseline,
health and respawn policy in place on a surviving entity, and had no production
caller: `git grep` found its definition and its own four `#[cfg(test)]` tests.
It is superseded rather than merely unwired — a room transition retires every
`RoomResident` and rebuilds the destination from its construction plan, and a
same-room replay reconstructs the population at the confirmed lifecycle boundary,
so no entity survives for a reset-in-place to restore.

⭐ **THE REPLACEMENT WITNESS ALREADY EXISTED AND IS BETTER THAN THE FOUR.** The
row asked for the respawn-policy claims to be asserted against the shipped
reconstruction before the false witness was deleted.
`a_body_that_never_persists_its_death_ignores_a_flag_bearing_its_name` does
exactly that, through `sync_ecs_actors_with_save` — the reconstruction's own
liveness reader — with both halves: an `OnRoomReenter` body ignores a flag its
kind never writes, and a `DeadStaysDead` body under the SAME flag stays dead, so
a green cannot be bought by the flag being absent. Deleted with its tests.

⭐ **AND THE `OnRest` HALF WAS A LIVE DEFECT, not residue.**
`clear_dead_until_rest_flags` had zero callers in the workspace, so an `OnRest`
death wrote `enemy_<id>_dead_until_rest`, nothing cleared it, and `OnRest`
behaved exactly like `DeadStaysDead` across the 14 shipped placements that author
it (`scripts/measure_persisting_enemy_placements.py`: `pirate_sky_arena` and
`pirate_sky_lookout`). A rest clears them now, at the shrine seam rather than on
`CheckpointCommitted` — that message is "whatever a game decides a checkpoint
is", including an autosave, and reviving a corpse because the game autosaved is
not the mechanic. The suffix has one owner instead of two spellings kept in sync
by a comment.

⇒ `check_no_warnings` is GREEN, which it had not been all session: the dead
method was its only finding until two more of my own appeared beside it.

### D-ID-CONVENTION-DRIFT — keep shared semantic key builders single-owned

**Owner:** registry/identity owners.

Continue only when a producer and consumer still construct the same semantic ID
with separate format strings. Move spelling into the semantic owner and update all
customers in one change.

**Acceptance:** grep finds one constructor for the migrated key family and both
producer/consumer tests use it.

**Re-measured 2026-09-09** with `scripts/measure_id_prefixes_spelled_twice.py`:
38 prefixes appear as a literal, TWO in both a producer and a consumer.

- `_dead_until_rest` — **migrated**, and it was a live defect rather than drift.
  See the D-RESET-ROAD-RESIDUE receipt above.
- `respawn_platform_` — `game/ambition_demo_smash/src/lib.rs`, the Smash lane.
- ⚠ `npc_` — **A FALSE POSITIVE, and it must not be "fixed".** The sweep matches
  a PREFIX, so three unrelated conventions that share four characters read as one
  drifting id: `npc_{id}_hostile` and `npc_{dialogue_id}_talked` are save FLAGS
  built in `features/npcs.rs`; `character_id.strip_prefix("npc_")` in
  `ambition_sprite_sheet` is a PLACEMENT-ID convention for finding a sheet
  record; and `npc_talked:{id}` in `ambition_persistence::quest` is a quest
  objective key. Verified by grep: each flag has exactly one production
  constructor, and the remaining literals are test spellings, which are
  deliberate — an independent spelling in a test is what catches a rename.

⇒ Nothing is left in this row for the engine lane. It stays open for the Smash
one, and for the next producer/consumer pair a re-measurement finds.

### D-BUILD-GRAPH-BLINDNESS — keep optional/dependency measurements non-vacuous

**Owner:** build/architecture tooling.

Do not interpret declaration count as capability reachability. Measurements must
resolve definitions, feature conditions and actual closure.

**Acceptance:** fixtures distinguish declared-but-unused, feature-gated and
actually linked dependencies.

### D-LANE-UNRUNNABLE / D-APPIT-FLAKE — preserve executable test lanes

**Owner:** test runner / app integration lane.

When a lane cannot run because the environment lacks a precondition, report
**incomplete**, not pass. For flakes, isolate the production ordering/state source
instead of increasing retries.

**Acceptance:** missing Cargo/target/GPU prerequisites are explicit receipt states;
known deterministic fixtures do not depend on wall-clock or entity order.

**Sighting 2026-09-09, measured rather than guessed.**
`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`
failed in 2 of ~8 full `app_it` runs and **0 of 25 consecutive runs of its own
module**, plus 3 consecutive clean full runs. ⇒ it fails only under full-suite
load, which points at parallel execution or process-global state rather than at
the test's own logic. The panic text was never captured, because a run that fails
prints it only for the failing test and every attempt to capture reproduced a
green run — that is the row's own lesson restated: **a flake with no message is a
sighting, not a diagnosis.** Whoever sees it next should run the full suite with
output redirected to a file so the message survives the run that produced it.

⚠ The obvious suspect was ruled out: the test's own doc says "the only thing that
would make it fail is a boss system that some other capability turns out to
require", and A2a had just made the damage-facing publication require
`Res<BossCatalog>`. If that were it the failure would be deterministic in the
disabled arm; 25 clean module runs say it is not.

### POST-CARVE-DOC-SWEEP — update moved-source references in the same carve

**Owner:** the carve author.

Run citation/link/source-reference guards on the **diff** after a move. Re-tense
historical prose where useful; delete live directions to old paths. Do not retain a
huge global post-carve diary.

## P3 — human-gated measurements and local-machine work

These rows cannot be completed from an ordinary headless source review. Keep the
measurement here; keep analysis/results in the owning tool or owner document.

- **D-RASTER-3:** run the remaining weak-GPU framebuffer-scale versus source-tier
  experiment on the intended GPU. Owner: `engine/performance-and-iteration.md`.
- **Switch Pro outer range:** run the controller diagnostic on both machines and
  record the measured radial maxima/dead-zone behavior before changing stick
  thresholds.
- **Web reveal branch:** validate the existing reveal-barrier branch on the real
  browser/GPU target before merge; do not infer from native first-draw behavior.
- **Kaleidoscope Bevy-0.19 flash:** reproduce interactively before filing a fix;
  stale no-repro descriptions are not a queue substitute.
- **LDtk preview tilesets:** measure whether editor-preview assets still decode the
  full player sheet on current boot before changing residency policy.
- **Capture stays alive after window close:** reproduce with current capture
  tooling and identify the live owner keeping the process alive before patching.
- **External consumer/platform checks:** use the SDK/external-consumer owner docs;
  do not claim portability from in-workspace fixtures alone.

Product/content choices such as dense-room composition, evergreen settings,
camera legibility limits and asset policy live in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), not here.

## Replenishment rule

Add a queue row only when all of these are known:

1. the current production failure or missing capability;
2. the semantic owner;
3. the next concrete edit or measurement;
4. an acceptance test/receipt that can falsify the work.

If one is unknown, put the question in the relevant owner document or maintainer
decision ledger instead. When a row is complete, delete it from this file. Git
history is the completion log.
