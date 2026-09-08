# Engine architecture reassessment

**Decision baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
**Status:** proposed implementation direction, adopted by this planning revision;
not a maintainer ruling and not a claim that the target is implemented.
**Execution order:** [the queue](../queue.md). Detailed packets live in the
[work frontier](actor-monolith-work-frontier.md).

This assessment supersedes the mandatory P2 -> P3 -> P4 SCC sequence. It does
not supersede recorded maintainer decisions, One Body One Path, Bevy-native
implementation, or same-build rollback policy. Current durable architecture
outside `docs/planning` describes the implementation; promote a changed boundary
there when its implementation lands, not before.

## CURRENT MODEL

Ambition currently has a reusable Bevy-based platformer substrate, several
substantial capability implementations, and a distributed gameplay integration
kernel. The kernel is spread across the residual actor crate, combat, character
schemas, runtime lifecycle code and shared vocabulary. Cargo's acyclic package
graph conceals several of those authorities crossing package boundaries.

Three examples establish the distinction.

* Live actor query/mutation authority was initially extracted with construction.
  The correction restored it to the actor kernel while keeping spawn builders
  extracted. The module SCC improvement survived both the incorrect and corrected
  designs. Evidence: `crates/ambition_platformer2d_actor_spawn/src/lib.rs`,
  `crates/ambition_platformer2d_actor_monolith/src/actor_clusters.rs`, and
  `scripts/tests/test_actor_spawn_boundary.py`.
* Checkpoint startup restoration and reset routing are implemented in
  `crates/ambition_platformer2d_actor_monolith/src/shrine.rs`. Item pickup installs
  startup restoration; checkpoint-horizon composition installs reset restoration.
  The router transacts against session lifecycle state, while occurrence and
   item consumers can still restore directly from an unadmitted raw reset. The
   shrine/session cycle is misplaced lifecycle ownership, and fixing it requires
   a shared accepted operation, not a more generic shared slot.
* Projectile contact admission, boss damage application and published boss hurt
  geometry use different paths. A marker-only extraction can hide that mismatch
  without giving one authority responsibility for the contact decision. Evidence:
  `crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs`,
  `crates/ambition_platformer2d_actor_monolith/src/features/ecs/target_volumes.rs`,
  `crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/boss_hit.rs`.

The measured residual SCC has nine top-level modules: abilities, construction,
control, features, items, projectile, session, shrine and world. A separate
assets/character_sprites SCC has two. The measurement is a textual module-path
instrument, not an ownership proof or a complete graph of state, scheduling,
re-exports and cross-crate dependencies. Its output should change as a consequence
of a better design, not determine the design.

The workspace has 79 packages and 679,785 physical Rust lines under their `src`
directories, including tests and comments. The actor monolith accounts for 98,464
of those lines. Those numbers describe this archive only; they are not production
LOC, binary footprint or a measure of engine quality. See
[coverage and reproducibility](architecture-review-coverage.md).

### The main structural problem

An authority is more than the crate holding a component definition. For this
migration it includes the state, accepted writers, invariants, scheduling and
visibility rules, identity, lifetime, restoration and interpretation of outputs.
A package can own a definition while another package effectively owns all of its
semantics. Moving the definition lower can therefore make the ownership less
clear while improving the dependency graph.

`features` is a historical grouping, not a reusable domain. `combat` contains
both combat algorithms and unrelated gameplay integration. `characters` combines
runtime actor vocabulary, brain policy, prepared definitions and technique
schemas. `core` contains a substantial coherent platformer movement system plus
several smaller foundations. `runtime` is both assembly and a real session
lifecycle implementation. Treat each as a source container to be decomposed by
responsibility, not as a certified capability.

The [responsibility map](architecture-responsibility-map.md) identifies current
owners and target logical authorities. Logical authorities are not a request to
create one crate per row.

## WHAT THE EXISTING PLAN GETS RIGHT

Keep these decisions.

1. **Construction and live mutation are separate operations.** Keep the corrected
   spawn boundary, body-seed reuse and one structural body road. Spawn-time policy
   belongs with preparation/building; provoking, querying or rebuilding the live
   state of an existing actor does not become spawning because it uses similar
   data.
2. **Typed construction and prepared immutable content are useful boundaries.**
   Domain-owned parameters and relations are easier to inspect than a universal
   dynamically dispatched recipe registry. Preserve validation before mutation.
   Strengthen the description of commit failure: the current raw-Commands recipe
   interface is not an atomic rollback transaction.
3. **Published scheduling sets and explicit composition are appropriate.** A
   capability can own its installation and internal ordering. A host can order
   independently owned capabilities. Both are necessary; zero foreign installs
   is not an architecture requirement.
4. **Rollback registration should not drag a backend into every domain.** The
   backend-neutral registrar is justified by an actual backend boundary. Domain
   state remains registered with its semantics and lifetime, not by a central
   list whose author must understand every field.
5. **Optional capabilities need absence tests, and SDK ergonomics are separate
   from internal crate topology.** Preserve the facade and external-game route.
   Add real Cargo-closure evidence: installation opt-out alone does not make a
   dependency optional.
6. **World-to-domain lowering belongs at a composition/preparation boundary.**
   The P4 diagnosis is supported. `ActorPlacementContext` carries catalogs,
   prepared character definitions and sprite preparation inputs, rather than
   only spatial world facts. Move that bridge toward construction, with its
   callers, without making world geometry depend on character materialization.

## WHAT SHOULD CHANGE

### Decision A: replace graph-first peeling with authority-first packets

The prior sequence was too certain about three different kinds of change. It
combined a valid bridge relocation, a prematurely generic contact seam and an
incorrect lifecycle ownership move. Its hard-core ledger postponed the difficult
ownership decisions until after those changes.

| Previous packet | Ownership judgment | Revised disposition | Why |
| --- | --- | --- | --- |
| P1, settlement state / corrected actor spawn | Supported by the actual state and caller responsibilities | Preserve; do not reopen the spawn mistake | Construction-only code can be understood without live actor mutation authority |
| P2, replace projectile feature-family queries with a generic target capability | Direction supported, proposed seam insufficient | Replace with A2 contact/geometry work; do not start with a marker-only carve | Selection, geometry, consumption and victim reaction must agree; a capability tag alone establishes none of them |
| P3, move lifecycle commit vocabulary from session into shared_tangle | Not supported as a generic-foundation move | Replace with A1 checkpoint restoration ownership | The slot contains room/session intent and admission policy; shrine restoration is the misplaced consumer |
| P4, move actor placement lowering from world into construction | Supported, subject to preserving provider and preparation boundaries | Retain as A3, independently schedulable after its preflight | This moves a concrete integration adapter rather than adding indirection |
| P5, wait for an expected six-module SCC before doing semantic analysis | Too late and too tied to a predicted graph | Use the current semantic edge ledger now; refresh per packet | Some cycles are coherent control/custody relationships and may remain inside a package |

Do not turn A1-A12 into another mandatory fixed sequence. The frontier
records dependencies and explicit holds. The queue selects the current work.

### Decision B: give checkpoint restoration to lifecycle, leave rest points as content

A rest point can detect interaction, heal and request a checkpoint capture. A
session resume must choose the saved destination, wait for a subject, request
admission, survive loading and confirmation, and avoid repeating placement after
completion. Those are distinct responsibilities.

Move startup/reset restoration, its progress state and its installation together
to the existing session region first. Keep the lifecycle slot there. Preserve the
current primary-avatar restore policy and all interacting-body healing behavior;
this architecture packet does not decide multiplayer save policy. A generic
checkpoint subsystem is justified only when its non-shrine customer and reduced
prerequisites are demonstrated. There is no reason to rename `shrine` before
removing the lifecycle responsibilities from it.

The source also records a route attempt before checking slot admission. Under a
denied-admission path the startup system can latch itself off for the session.
This is a source-supported conditional defect, not an executed Rust reproduction.
A1a fixes that latch; A1b performs the ownership move. The deeper trace also
finds raw reset readers that restore occurrence/custody/accounting regardless of
room admission (F9). A1c pins the accepted checkpoint, prepares against its
read-only continuity view and applies all participating reducers through the
common authorized commit. It does not promise arbitrary ECS undo. The
[checkpoint protocol](checkpoint-restoration-protocol.md) now owns the complete
state machine, phase/identity rules and migration.

### Decision C: separate contact selection, accepted damage and presentation

A projectile's travel policy decides which contacts are physically relevant. The
victim's published simulation geometry decides whether a contact exists. Victim
reaction decides whether damage, invulnerability response, deflection, knockback
or destruction follows. Game rules decide score, stock loss and rewards. None
of those stages should reclassify an already selected victim by historical
feature-family strings.

Do not introduce a universal combat service. First make the existing boss
publisher, preflight and application agree on geometry. Then resolve direct
contacts against stable victim identity, with an explicit result for contact
without damage. Splash/area damage may intentionally select several victims;
that is a separate operation from one projectile touching one body. A published
empty authored hurt-volume set must retain its existing meaning, rather than
fall back to a convenient AABB.

Projectile world sweep currently occurs after a feature-hit branch that may
consume the projectile. Moving that branch behind a generic query would preserve
the ordering defect. Characterize obstruction and fast-body traversal before
claiming the new seam is complete. Do not combine the entire CCD redesign with a
mechanical type move. The [contact protocol](projectile-contact-protocol.md)
now fixes actual-segment ordering, compound destructible surface/target contact,
immediate interception versus later damage, sampled-target sweep scope and
identity-preserving delivery. See F2/F3 and A2's staged subpackets.

### Decision D: retain coherent control cycles; split provenance from behavior

Possession eligibility is an ability policy. The accepted participant-to-body
relation and the projection of input onto that body are control authority. Those
parts should be understandable together even if that means an internal cycle
remains. Human input, a deterministic brain and a remote participant are intent
producers, not separate kinds of physical body.

Likewise, action/technique definitions and their deterministic executor may share
one package when the protocol is small and tightly coupled. Brain selection,
content loading, sprite residency and match roster activation need not share
that package. Do not place a runtime actor query into an authored-character
schema crate merely to make imports convenient.

### Decision E: split by semantics before splitting Cargo packages

Use modules and visibility to establish ownership first. Extract a crate when
there is a real consumer, an independent installation/lifetime boundary, a
feature-closure benefit, or a measurable compile/dependency benefit. A conceptual
owner does not require a plugin, registry, trait and crate of its own.

Avoid turning `core`, `combat`, `characters` or `runtime` into freshly renamed
containers for the same mixed responsibilities. Keep `actor_monolith` and
`shared_tangle` as honest migration labels until their documented exit conditions
are met. Retire them by emptying the accidental responsibilities, not by calling
them `actors` and `foundation`.

### Decision F: distinguish four forms of modularity

A boundary can be successful in one dimension and incomplete in another:

| Dimension | Falsifiable question |
| --- | --- |
| Semantic authority | Can the owner be understood without the sibling's internal policy and state writers? |
| Runtime installation and lifetime | Does the capability run with only declared prerequisites, including teardown/re-entry? |
| Compile-time closure | Is an absent capability absent from the selected normal dependency/feature closure? |
| Public API | Can an external game use the supported path without internal re-exports and source knowledge? |

The facade has at least 51 other workspace packages in its transitive,
nonoptional normal dependency graph at this baseline. This is a conservative
manifest traversal, not a resolved Cargo feature graph or binary measurement.
One concrete path is facade -> platformer2d_host -> ambition_render, despite the
facade's own render dependency being optional. Fix the documentation claim and
introduce a real minimal-profile witness before using a passing optional-plugin
fixture as evidence of footprint isolation. See F5 and A9.

### Decision G: retain explicit, bounded extension points

App-local provider registration is useful when independent game providers really
supply different implementations. A closed set of construction recipes can use
typed dispatch and a metadata registry. These are different jobs. The small
`ambition_registry_core` identity/conflict utility does not justify a universal
registry through which gameplay operations discover one another.

For each proposed seam, ask: after the change, can a reader understand the caller
while knowing materially less about the callee? If the caller still constructs
all the callee's private state, knows its scheduling internals and interprets its
special cases, a trait or message has mostly moved the coupling. Keep a direct
dependency when the semantics already have the correct direction.

### Decision H: define transaction guarantees at the mutation boundary

Prepared content validation is genuinely separate from world mutation. However,
construction recipes receive raw Bevy Commands and the post-commit roster
verifier detects violations after commands have mutated the world. It can prevent
publication; it cannot undo arbitrary commands. An invalid trusted recipe can
therefore leave changed world state even when verification fails.

Document this as a trusted-code, fail-closed publication protocol. If last-good
world retention across commit failure becomes a requirement, implement an
inactive candidate population and constrain all writes before activation; prove
that external resources are not mutated before promising atomic replacement.
Do not infer isolation from the name `ConstructionPlan` or solve the problem by
cloning an arbitrary Bevy World. See F6 and the construction owner document.

### Decision I: make agent authoring a control-plane contract

LLM authors should work through inspectable source, typed authoring schemas,
preparation, deterministic validation, a headless behavioral fixture and an
explicit publication operation. The model's reasoning process is not part of
the simulation tick. A runtime model-backed character remains a remote intent
participant with deadlines/fallback policy, as already planned.

Support two trust levels explicitly: validated authored data/programs with a
bounded vocabulary, and trusted Rust providers with full application privileges.
A Rust plugin is not sandboxed because its registration API is typed. Do not add
a scripting VM merely for competitive positioning. The missing work is reliable
end-to-end authoring, provenance, diagnostics, replayable tests and publication,
not another language surface.

The source already has a TechniqueFlow interpreter and a live authored customer,
contrary to stale authoring prose. Its parameter-validation registry has no
production caller, and flow validation does not bound usize transitions to the
u16 runtime cursor. A11/A12 close those concrete admission gaps. Duplicate keys
can be rejected without comparing function pointers; the current rationale for
mandatory last-write-wins is incorrect. The
[authored-technique protocol](authored-technique-admission.md) selects acyclic
1-256-node flows, exhaustive installed-profile validation, private checked runtime
values and session-boundary activation. Finish stops the flow, not move recovery;
the normal move clock/teardown remains authoritative. See F7/F8.

### Decision J: defer multi-instance generalization until its acceptance case

The live world and several identity/query roads assume one active room. That is
valid for the current host but is not multi-resident-world support. A room
**definition**, an occurrence of it in a live session, a participant, a body and
a rendered view have different identities and lifetimes. Existing stable IDs do
not automatically establish the namespace needed for two live copies of a room.

Before streaming/multiview across rooms, prove two simultaneous room instances,
repeated definition use, scoped lookup/collision, portal handoff, save ownership
and teardown. Do not spread a new universal instance identifier through every
crate ahead of that customer. Preserve the one-room profile as a supported,
simpler specialization.

## What not to do

* Do not move the lifecycle slot to shared_tangle merely to erase the
  shrine/session edge. Move the consumer that owns the operation.
* Do not split accepted control, possession projection and live custody into
  independent services that need a round-trip registry to maintain one invariant.
* Do not create a `features` plugin or crate covering unrelated room objects,
  actor decisions, item custody and session restoration.
* Do not make the host a new gameplay kernel by moving every difficult system
  into runtime. Host integration may order independent owners; it must not hide
  their algorithms behind an unbounded context object.
* Do not use a dependency-neutral bag of shared enums for all domain commands.
  A vocabulary should have a semantic owner and a bounded interpretation.
* Do not replace valid direct calls with messages solely to remove an import.
  Deferred delivery changes visibility, ordering and sometimes rollback meaning.
* Do not declare success from fewer SCCs, fewer re-exports or zero private-system
  names while the same state remains multiply interpreted.
* Do not preserve compatibility aliases in internal migration paths just to avoid
  changing callers. Public compatibility is a separate, explicit product choice;
  this repository does not currently promise stable cross-build rollback wire.
* Do not copy Godot's scene tree or Unity's authoring UI. The competitive test is
  whether a game can be expressed, inspected, tested, iterated and shipped.
* Do not add a repository-wide scanner or mutation-test campaign for every new
  rule. Prefer the smallest behavioral witness and existing architecture checks
  that can falsify the actual defect.

## RECOMMENDED NEXT MOVE

**A1: make checkpoint restoration a session-owned operation.** A1a reproduces
and repairs the startup admission latch and characterizes denied-reset mutation.
A1b moves restoration state, systems, rollback ownership and installers together
without changing behavior. A1c then changes reset admission/commit sequencing so
room and domain restoration use one selected checkpoint and authorized commit.
It preserves the product's checkpoint, healing and primary-avatar policies, not
the defective raw-message mutation timing. Do not create a new crate, generic
lifecycle bus or shared slot.

This has a narrow responsibility boundary, a concrete correctness witness and a
clear absence test: checkpoint resume must not require installing held-item
behavior or a shrine entity. It also makes the later custody, session and world
cuts easier to judge. A3 can proceed independently once it has a fresh caller
inventory. A2 needs geometry/contact characterization before extraction. The
remaining kernel splits are conditional packets, not authorized directory moves.
A1c is now required before declaring checkpoint restoration coherent: raw reset
consumers must use the same selected checkpoint and commit boundary as the room.

## External design references

These references inform the comparison; the repository evidence above determines
its boundaries.

* Godot, **Optimization using Servers**, stable documentation: low-level render,
  physics and audio APIs can be used without making scene-tree authoring the only
  route. Borrow the layered-access principle, not its exact object model.
  <https://docs.godotengine.org/en/stable/tutorials/performance/using_servers.html>
* Cargo Book, **Features**: feature activation is additive across dependency paths;
  an opt-out on one edge does not establish a minimal transitive configuration.
  <https://doc.rust-lang.org/cargo/reference/features.html>
* Bevy 0.19.1 API, **Deferred**: queued system buffers are applied at deferred
  synchronization, so schedule changes can change observation of mutations.
  <https://docs.rs/bevy/0.19.1/bevy/ecs/system/struct.Deferred.html>
* Unity Entities, **Entity command buffer playback**: parallel recording needs
  an explicit deterministic playback order. For Ambition, derive tie-breakers
  from simulation identity/sequence, not incidental entity allocation.
  <https://docs.unity3d.com/Packages/com.unity.entities%406.6/manual/systems-entity-command-buffer-playback.html>
