# Architectural audit frontier

**State:** audit map and research priorities, 2026-11-22. Runtime source:
`34b4ab0b7f4f5a950a52e6031423e76b5e75f6ac`. Planning baseline:
`07c27e7732a8d009fcc8831a39af1f059efe076c`, including the preceding shared
action/Smash reviews. No newer runtime source was supplied for this pass.

This page answers where another deep audit is likely to add useful guidance.
It is not an execution queue, a claim that a subsystem has no defects, or a
new architecture authority. [Queue](../queue.md) retains selected work and
product order; [tracks](../tracks.md) retains unselected implementation
work. Findings belong in the existing domain owner documents linked below.

## What has already received substantial attention

Coverage means a boundary has source-backed reasoning and an owner plan.
It does not mean that every consumer is migrated or every acceptance case
has run. “Next audit” below means an unresolved question in this review
series and the inspected plans, not proof that no other contributor has
ever read that code.

| Boundary | Existing owner and coverage | Remaining reason to return |
| --- | --- | --- |
| Portal geometry, connection identity and material authority | [Portal connection](portal-pair-identity-and-aperture.md), [world geometry](world-geometry-and-spatial-semantics.md) | Cross-consumer query meaning and invalidation; do not repeat the general portal redesign |
| Body scale, native motion and presentation order | [Scale behavior](scale-relative-gameplay.md), [presentation](presentation-order-and-25d-rendering.md) | Implementation evidence or a consumer whose frame/appearance contract is still missing |
| Actions, combat reception, hit time, pose and recovery | [Body actions](body-action-execution.md), [combat reception](combat-reception-and-injury.md), [Smash quality](../demos/super-smash-siblings.md) | A new native completion or prediction customer; preserve the completed first slices |
| Resource/value ownership, effects and shared world writes | [Causal world commit](causal-world-commit.md), [effects](actor-effects-and-modifier-composition.md), [custody](item-custody-and-accounting.md) | Historical membership, new world-owner operations and endpoint retirement |
| Construction, residency and reconstitution | [Construction](construction-and-reconstitution.md), [residency](open-world-runtime-and-residency.md) | Transfers across independent histories and supported dormant execution |
| Content preparation, linking and authoring | [Linking](capability-linking-and-prepared-facets.md), [prepared media](asset-authoring-and-prepared-media.md), [asset delivery](asset-library-editor-and-delivery.md) | A demonstrated generation/publication gap or a repeated interpretation still outside its owner |
| Compiler, dependency and product boundaries | [Compiler/build products](compiler-boundaries-and-build-products.md), [runtime composition](capability-and-runtime-composition.md) | Fresh compile/link/size measurements after the planned cuts |
| Narrative roles, interaction and native completion | [Story](story-sequences.md), [interaction](interaction-opportunities-and-activation.md), [native actions](native-action-completion.md) | An actual lost invocation/result/role boundary, not a general task framework |

Input history and dormant plan completion had less explicit treatment than
these boundaries. Hosted room transfer was already selected as ISL-3;
this pass sharpens its history cut rather than declaring it unexplored.

## Findings added by this pass

### Input is a request with a stable execution cut

**Owner:** [IN1–IN3](participant-action-system.md#input-history-and-body-interpretation).

The recorder captures input after deterministic interpretation and edge
consumption, while replay injects it before those steps. Reset exposes a
lost cause; portal shaping exposes coordinate-dependent interpretation.
These are consequences of using one mutable representation for admitted
input and its interpreted result.

The bounded correction establishes an immutable admitted value inside
each tick execution, publishes it for all current backend/driver paths,
and records and replays at that same cut. Body interpretation remains
downstream. A physical movement guard resolves the body's frame and
expressible commands before applying a world-space constraint. It must not
change an unrelated attack's aim by rewriting a shared raw axis.

This improves a shared contract for local play, GGRS, hosted input, replay
and harness drivers. It does not require another input manager, general
event log, or wholesale action migration. The smaller qualified-press
coalescing defect is recorded with IN3; it is not the architectural thesis.

### A plan predicts completion; an executor earns it

**Owner:** [LIFE1/LIFE2](radiant-npc-planning.md#dormant-completion--execute-a-supported-prefix-of-the-plan).

Dormant NPC catch-up grants every step's effects after planning. It budgets
each claimed object's hours but not the actor's total sequential work.
It can also grant arrival effects before resolving a destination, and use
current presence to answer a past-hour condition.

The first correction executes only the supported prefix under the existing
schedule clock, books its actual consumed object time, and credits effects
only at completion. Destination and observation assumptions are admitted
explicitly. Hourly cancellation/restart remains a valid policy. A persistent
activity cursor, generic operation catalog and exact hidden physics are not
prerequisites.

This is useful before expanding authored lives or allowing real object use.
It provides a small common execution law for different approximations:
declared preconditions and consumed time constrain which outcomes may be
settled. It does not claim identical live and dormant trajectories.

### A room transfer changes history membership as well as location

**Owner:** [ISL-3 refinement](island-rollback-and-compact-peer-input.md#isl-3-refinement--history-membership-and-scoped-transfer-cuts),
with the existing [CW4](causal-world-commit.md#cw4--replace-the-global-confirmation-floor-only-when-useful).

The hosted implementation has per-room saved histories but a global
crossing barrier and confirmation floor. The barrier avoids a real duplicate
destination failure when replay hides roots. Removing its scope without
replacing that contract would be incorrect.

Derive the affected source and destination histories from the existing
transfer plan. Preserve unrelated history and replay, while keeping foreign
lifecycle work out of hidden-world replay. Correct the affected pre-cut
prefixes before publishing body/custody and input membership together.
Waiting for a watermark while refusing the needed correction is not enough.

This first slice can keep conservative global finality. A later scoped query
must use the membership of the history that produced a request; current
occupants cannot establish the finality of old input. Whole-world checkpoints
retain aggregate finality. A transfer that never holds its affected histories
requires a further retained destination import, not merely retained snapshots.

## Area audited without another architecture packet

Reload and asset realization were inspected against the existing plans.

In the inspected path, [`SpriteContent`](../../../crates/ambition_sprite_sheet/src/baked_sheet_rons.rs)
retains prepared metadata and image mappings;
[image realization](../../../crates/ambition_sprite_sheet/src/game_assets/mod.rs)
uses the supplied generation's immutable bundle path.
[Module replacement](../../../crates/ambition_extension_host/src/reload.rs)
stages its composition and checks live/saved schema compatibility before
publication. The existing [reload](content-generation-and-reload.md) and
[residency](asset-preparation-and-residency.md) plans already address dependency
invalidation, private preparation, coherent publication, App-owned selection,
mechanical revision pinning, and retain/rebind/retire policy.

This pass found no additional high-value boundary to justify another reload
plan. A small process-lifetime string pool is not evidence that full content
generations leak; no cost was measured. Revisit this area with a specific
failed generation/selection case, rather than adding another registry.

The source review also checked the existing perception/knowledge ownership
map, but did not trace enough social or observer behavior to promote a new
finding there. Those questions remain candidates below.

## Ranked candidates for the next audit

These are research questions, not established defects or selected work.
Each starts from an existing owner and a concrete consumer. An audit is
successful if it rules out a proposed abstraction as well as if it finds one.

### 1. Hosted replication and prediction as an explicit partial simulation

**Why next:** the engine now has both a sleeping guest and a guest that
predicts its own body. Their earlier witnesses establish useful cases, but
do not define the full causal state that the predictor may rely on. This is
a boundary where a convenient replicated view can be mistaken for a
sufficient simulation input.

Start with [hosted networking](hosted-world-networking-experiment.md),
[`prediction.rs`](../../../crates/ambition_hosted_world/src/prediction.rs)
and [room replay](island-rollback-and-compact-peer-input.md). Trace one
predicted body touching an authoritative moving surface or an admitted
mount, with a remote strike as a control.

Determine:

- Which rows are visual observations, authoritative correction state, or
  inputs to local prediction?
- What causal state must accompany an authoritative body row at its tick
  for the supported interaction to replay?
- Which foreign capabilities are held, extrapolated or deliberately excluded
  from prediction, and who states that choice?
- Does correcting the local body reconstruct its dependencies, or only its
  visible result? Does ownership change invalidate the old prediction scope?

**Useful artifact:** a capability/state matrix and one bounded correction
contract with an existing host/guest witness. Keep transport choice and
general rollback replacement out of scope. Do not promise that the entire
world must be simulated on the guest.

### 2. Effective geometry as a query contract across consumers

**Why high value:** movement, attacks, interaction, navigation and diagnostics
need related geometry with different acceptance rules. Earlier portal and
recovery work covers parts of this, but does not by itself prove that all
consumers ask the same spatial question at the same admitted revision.

Start with [world geometry](world-geometry-and-spatial-semantics.md),
[collision/CCD](collision-and-ccd.md), [geometry diagnostics](geometry-observability-and-f1.md)
and [navigation](platformer-navigation-and-reachability.md). Follow one
moving portal-bearing or subject-passable surface through a query, replay
restore and derived-index rebuild.

Determine which meaning is canonical shape, subject-specific passability,
effective carved material, attack acceptance, or a conservative broad phase.
Check whether cache keys and invalidation include the actual room/content/
geometry state they depend on. Establish whether replay-hidden roots and
moving geometry can enter a derived query from the wrong cut.

**Useful artifact:** the smallest shared query contract and dependency table
that deletes repeated interpretation, plus a counterexample/control pair.
Do not build a universal collider, require full reference geometry per
gameplay tick, or create a data-structure package before finding a repeated
algorithm with a measurable cost.

### 3. Relationship endpoints through transfer, replacement and retirement

**Why high value:** mount, capture, held item, tether and spawned-action
relations can share coordinate bindings while retaining different lifetime
laws. The existing anchor work solves two-ended geometry; it does not imply
one common answer when an endpoint disappears or changes authority.

Start with [body anchors](body-anchor-bindings.md), [custody](item-custody-and-accounting.md),
[native actions](native-action-completion.md), [effects](actor-effects-and-modifier-composition.md)
and [construction](construction-and-reconstitution.md). Trace one relation
through death, form replacement, possession and room transfer; compare a
second relation only where it exposes a different disposition.

Separate issuer, current controller, physical attachment, custody and
lifetime owner. Ask which endpoint identity survives each transition and
which native owner must detach, cancel, transfer or preserve the relation.

**Useful artifact:** a disposition matrix and the narrowest reusable endpoint
binding/admission rule, with one migrated relation. Do not infer a generic
entity graph from a shared pair of IDs, or make attachment imply ownership.

### 4. Observation, attention and durable social knowledge

**Why high value:** tactical belief, social knowledge and planner conditions
answer different questions about truth, access and time. The dormant
`Present` finding is a concrete small example; it does not yet establish a
design for durable knowledge.

Start with [bounded perception](bounded-perception-and-attention.md),
[world facts and memory](world-facts-observations-and-memory.md),
[orchestration](authored-gameplay-logic-and-orchestration.md) and
[character perception](../../../crates/ambition_characters/src/perception.rs).

Follow one witnessed theft or hostile act through perception, attention,
dialogue and later return. Determine what was observed, what was attended
to, which identity was recognized, what can be inferred or told to another
actor, and what remains after room retirement or checkpoint restoration.
Keep unknown, forgotten and known-false separate when the consumer needs
that distinction.

**Useful artifact:** one native observation-to-knowledge path with explicit
evidence/time/identity and an authoring example. Avoid a universal fact
database, an all-event journal, or extending decaying tactical memory into
permanent social truth.

### 5. Shared observers, provider actions and control ownership changes

**Why high value:** a body frame, a seat's observer frame and a render camera
are distinct authorities. The present input correction leaves shared-camera
policy and open provider actions intentionally bounded to their existing
owners.

Start with [participant actions](participant-action-system.md),
[control authority](control-authority-and-ai-policy.md) and
[multiview](multiplayer-and-multiview.md). Trace two bodies with different gravity under one view,
then a held control through focus capture, device reassignment or possession.

Check the planned P2 provider-action final hop: a gameplay action emitted
from host `Update` must reach the admitted simulation input/operation cut.
UI-only actions remain host-owned. Determine which held levels resume,
which require release, and which edges belong to the former controller.

**Useful artifact:** observer/control policy plus one transition witness.
Do not read an eased camera in deterministic gameplay or create a second
execution path for semantic actions.

### Conditional: compiler and binary costs after the planned cuts

The [compiler/build owner](compiler-boundaries-and-build-products.md)
already provides a source-based program. The next useful audit needs
measurements: clean and incremental compile, codegen/link time, representative
binary size, feature sets, and the actual repeated instantiations that
survive optimization.

Choose one expensive generic or dependency edge and compare its concrete,
erased or inverted forms under the same product. A crate split or trait
object can move cost instead of removing it. Do not create more packages,
type erasure or interchangeable collection APIs from file size alone.

## How to promote a candidate

Record the actual source baseline and distinguish a supported counterexample
from a shipped gameplay reproduction. State the existing owner, the first
native customer, the invariant it cannot currently express, the code that
would be removed, and the smallest proof. Measure performance only where
the proposed benefit depends on it.

Prefer refinements that make different consumers share one owned fact while
retaining their native policy. Reject a unification that adds a second source
of truth, forces unrelated consumers through an interpreter, or makes the
fast gameplay path pay for a reference model it does not need.

The order above recommends research attention. It does not move networking,
NPC work or Smash polish ahead of Ambition's selected product priorities.
