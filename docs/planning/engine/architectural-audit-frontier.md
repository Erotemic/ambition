# Architectural audit frontier

**State:** audit map and research priorities, 2026-11-22. Latest source:
`00bf92c2214274fdf2ecaacdc0abfd19be569a50`. The latest pass audits relation
admission, endpoint transitions and retained exit context. It follows the
effective-geometry audit at documentation commit `f533f6c03070`; the runtime
source is unchanged. This head includes the preceding
planning overlays and the agents' IR1 item-recipe and AC1 construction-receipt
work. Earlier input/life/history and hosted-prediction findings below came
from `34b4ab0b7f4f`; they remain separate owner work.

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
| Portal geometry, connection identity and material authority | [Portal connection](portal-pair-identity-and-aperture.md), [world geometry](world-geometry-and-spatial-semantics.md), [navigation coverage](platformer-navigation-and-reachability.md#static-geometry-coverage--native-support-before-finite-preparation) | Native witnesses for material overlap, static geometry coverage and partial publication; do not repeat the general portal redesign |
| Body scale, native motion and presentation order | [Scale behavior](scale-relative-gameplay.md), [presentation](presentation-order-and-25d-rendering.md) | Implementation evidence or a consumer whose frame/appearance contract is still missing |
| Actions, combat reception, hit time, pose and recovery | [Body actions](body-action-execution.md), [combat reception](combat-reception-and-injury.md), [Smash quality](../demos/super-smash-siblings.md) | A new native completion or prediction customer; preserve the completed first slices |
| Resource/value ownership, effects and shared world writes | [Causal world commit](causal-world-commit.md), [effects](actor-effects-and-modifier-composition.md), [custody](item-custody-and-accounting.md) | Historical membership, new world-owner operations and endpoint retirement |
| Construction, residency and reconstitution | [Construction](construction-and-reconstitution.md), [residency](open-world-runtime-and-residency.md) | Transfers across independent histories and supported dormant execution |
| Relationship admission and endpoint lifetime | [REL1–REL4](relationship-endpoints-and-transitions.md), with capture, mount, control and item owners | Native witnesses for actual transfer cohorts/configurations, late refusal, reset state and orphan continuation; do not repeat the general anchor or ID audit |
| Content preparation, linking and authoring | [Linking](capability-linking-and-prepared-facets.md), [prepared media](asset-authoring-and-prepared-media.md), [asset delivery](asset-library-editor-and-delivery.md) | A demonstrated generation/publication gap or a repeated interpretation still outside its owner |
| Compiler, dependency and product boundaries | [Compiler/build products](compiler-boundaries-and-build-products.md), [runtime composition](capability-and-runtime-composition.md) | Fresh compile/link/size measurements after the planned cuts |
| Narrative roles, interaction and native completion | [Story](story-sequences.md), [interaction](interaction-opportunities-and-activation.md), [native actions](native-action-completion.md) | An actual lost invocation/result/role boundary, not a general task framework |
| Hosted observation, prediction and correction | [Hosted prediction](hosted-prediction-contract.md), [experiment](hosted-world-networking-experiment.md) | Native witness results and the next admitted capability; do not infer general simulation support from motion equality |

Input history and dormant plan completion had less explicit treatment than
these boundaries before the preceding pass. Hosted room transfer was already
selected as ISL-3; that pass sharpened its history cut. Later passes traced
hosted guest prediction and effective geometry, each then the first candidate.
Their unresolved implementation work stays with the domain owners. The
relationship pass below takes the next candidate through native admission,
transfer, reset and reconstruction. It leaves the separate social-knowledge
and observer/control questions open.

## Findings retained from recent passes

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

### A guest needs a native continuation, not just an observation

**Owner:** [HP1–HP5](hosted-prediction-contract.md), routed through the existing
hosted experiment. The dated run/jump/strike measurements remain valid.

Five motion floats cannot distinguish a stationary no-launch hit that
changes hitstop and the next legal dodge. Restoring the guest's post-tick
save and replaying later local input cannot recreate the missing accepted
hit. Received correction values also need a lifetime outside the native
entities that restore may replace.

The same boundary exposes distinct meanings for executed simulation time,
publication revision, finality, body assignment and construction provenance.
The current guest can skip native steps by changing a tick label, discard a
revised host publication at the same tick, or keep a history for a seat/room
after possession changes its body. Forward prediction also wakes a world
write path whose absent-boundary default permits local guest settlement.

Use native owner-defined continuation at one admitted cut. Declare which
external trajectories or feedback state the supported unit consumes; keep
autonomous observation out of unrelated AI execution. Admit a platform
driver's revision even if the body at its anchor compares equal. A small
status outside the body's lifetime preserves prediction invalidation until
a new baseline, including unsupported changes that reverse between sends.

Begin with explicit world-commit permission and one complete body profile.
Then add an autonomous platform, a feedback lift, native assignment and
population support. This does not require a universal dependency graph,
second physics implementation, new transport, or generalized event journal.
Source counterexamples are established; the new native witnesses and cost
measurements are planned, not executed in this audit.

### A spatial result must retain the meaning of its evidence

**Owner:** [world geometry and query meaning](world-geometry-and-spatial-semantics.md).
Three bounded corrections came from the `00bf92c22142` source:

1. **Current occupied material.** Water/climb reads and physical interaction
   still use a complete root AABB. A stationary portal straddler can miss a
   real far-side contact or accept one in a clipped-away root portion. Share
   the native material-overlap query and retain the admitted map. Water
   metrics, native climb-region passability, facing and target preference
   remain domain rules. Keep discrete timing and direct exact-box paths.
2. **Native geometry before finite navigation.** Visual fill admits support,
   polygon bounds erase clear standing places, and flight occupancy omits
   stationary Solid chains. Native candidates, selected preparation and
   answer coverage are distinct. An unprepared support is not an exhausted
   route; an unchecked blocker cannot become free space. Keep the finite
   cost policy without encoding it through painted earth.
3. **Publication coverage during partial replay.** A correct rebuild over
   visible room A clears global geometry from hidden B. Unhiding B can
   expose the missing partition before a full tick. T3 owns scoped
   replacement or a correct pure full republish; ISL supplies the witness.
   Derived classification alone does not prove covered population.

These are source-supported counterexamples with planned native witnesses.
No new Rust build, gameplay reproduction or cost measurement was performed.
They refine existing owners and execution lanes. They do not select a
universal collider, zone hierarchy, global revision bus or always-on exact
reference solver.

The older Smash state's thin `line_of_fire` probe also lacks selected-shot
geometry and world-contact policy. This is a bounded
[REC3 ranged-action customer](fighter-brain.md#ranged-action-query-follow-up),
using permitted perception, not another architecture packet or a higher
priority for Smash polish.

### Relations need accepted transitions and sufficient continuation

**Latest owner:** [relationship endpoints and transitions](relationship-endpoints-and-transitions.md).
The hard boundary is between current relation state and a proposed change
to its endpoints. The native owners need a shared operation cut, not shared
ownership policy.

Current custody grows beneath session/possession travelers. Native NPC
departure can start elsewhere, so a room-scoped rider can move without its
mount. A possessed target can also move while its inactive home stays in a
source retained by another participant. Releasing then copies destination
coordinates into a home still assigned to the source. Prepare the actual
cohort from native policy, and distinguish membership-only companions from
active pairs that need a consistent destination pose. The existing mounted
crossing fixture manually pins the mount; its survival receipt does not
prove native arrival configuration.

A capture admitted in one room can split at transfer while both bodies
survive. Ordinary maintenance retains it; replay can release it when the
foreign captor is hidden. Resolve a split hold at final transfer admission,
after the last application preflight, before source context is destroyed.
Do not add captive custody or foreign-room replay reads as a repair.

The same law sharpens entry and exit. Shared `begin_capture` mutation does
not stop the native hold adapter from bypassing fighter endpoint matching.
Mount admission omits common residence, `RidingOn` can mean reset intent,
and two end requests can publish two ends for one ride. Make those decisions
native; keep different targeting, reset and control policies explicit.

Held reconstruction exposes a continuation loss: the current hand view
works while stored fallback position is zero. A later orphan release reads
that value. Retain exit placement and its chart at the item owner; prepare
an explicit destination fallback on accepted transfer and admit actual
materialization against the then-current context. Also scope custodian
handoffs to the old occurrence and an authorized successor, instead of
resolving bare authored IDs across every live room.

REL1–REL3 refine ARCHITECTURE-COMPOSITION; REL4 refines A7/IR2/IR3 and A10's
existing receipt boundary. All new witnesses are planned. No runtime build,
gameplay reproduction or speedup is claimed. Preserve IR1, AC1's first slice,
AC2, A1/A2 and I1/I2. There is no new universal relation manager, durable
mount graph or per-frame all-body scan.

## Areas audited without another architecture packet

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

That pass found no additional high-value boundary to justify another reload
plan. A small process-lifetime string pool is not evidence that full content
generations leak; no cost was measured. Revisit this area with a specific
failed generation/selection case, rather than adding another registry.

The geometry pass inspected composed collision-cache keys, room replacement,
overlay retraction and moving-platform publication. It found no new stale-cache
correctness case: root identity, source change ticks and relevant overlay
meaning are already represented. Empty platform sets already avoid a needless
change tick. Keep the K1 spatial-index and E4 cache receipts. Foreign cache
eviction during replay may add reconstruction work; measure it with the T3
customer before changing retention.

The source reviews also checked the existing perception/knowledge ownership
map, but did not trace enough social or observer behavior to promote a new
finding there. Those questions remain candidates below.

## Ranked candidates for the next audit

These are research questions, not established defects or selected work.
Each starts from an existing owner and a concrete consumer. An audit is
successful if it rules out a proposed abstraction as well as if it finds one.

### 1. Observation, attention and durable social knowledge

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

### 2. Shared observers, provider actions and control ownership changes

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
