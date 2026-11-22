# Architectural audit frontier

**State:** audit map and research priorities, 2026-11-22. Latest source:
`00bf92c2214274fdf2ecaacdc0abfd19be569a50`. The latest pass audits shared
observers, source-input continuity, control handoffs and the provider-action
final hop. It follows the observation audit at documentation commit
`3ac54ecc1ada`; the runtime source is unchanged. This head includes the preceding
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
| Observer evidence, tactical knowledge and social retention | [OBS1](bounded-perception-and-attention.md#obs1--make-attention-the-native-evidence-admission-boundary), [OBS2 and horizons](world-facts-observations-and-memory.md), [module demand OBS3](agentic-character-runtime.md#obs3--resolve-observation-demand-across-controller-providers) | Native over-cap, dead/lost-sight, report and module-provider witnesses; durable incident/recognition work only with a concrete later dialogue customer |
| Source input, control handoffs and independent observers | [IN1–IN5](participant-action-system.md), [control](control-authority-and-ai-policy.md), [MV1/M3](multiplayer-and-multiview.md) | Native capture/rebind, charge interruption, prediction and independent-view witnesses; provider P2 and shared-view input policy only with their selected consumers |
| Content preparation, linking and authoring | [Linking](capability-linking-and-prepared-facets.md), [prepared media](asset-authoring-and-prepared-media.md), [asset delivery](asset-library-editor-and-delivery.md) | A demonstrated generation/publication gap or a repeated interpretation still outside its owner |
| Compiler, dependency and product boundaries | [Compiler/build products](compiler-boundaries-and-build-products.md), [runtime composition](capability-and-runtime-composition.md) | Fresh compile/link/size measurements after the planned cuts |
| Narrative roles, interaction and native completion | [Story](story-sequences.md), [interaction](interaction-opportunities-and-activation.md), [native actions](native-action-completion.md) | An actual lost invocation/result/role boundary, not a general task framework |
| Hosted observation, prediction and correction | [Hosted prediction](hosted-prediction-contract.md), [experiment](hosted-world-networking-experiment.md) | Native witness results and the next admitted capability; do not infer general simulation support from motion equality |

Input history and dormant plan completion had less explicit treatment than
these boundaries before the preceding pass. Hosted room transfer was already
selected as ISL-3; that pass sharpened its history cut. Later passes traced
hosted guest prediction and effective geometry, each then the first candidate.
Their unresolved implementation work stays with the domain owners. The
relationship pass below takes its candidate through native admission,
transfer, reset and reconstruction. The following observation pass traces
native sight, accepted hits, crew delivery and saved hostility. The latest
pass takes the separate shared-observer/control transition question through
native sampling, admission, pending action and view resolution. These plans
now need implementation evidence; source coverage is not runtime completion.

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

### An observation is admitted evidence, not a confidence score

**Owners:** [OBS1](bounded-perception-and-attention.md#obs1--make-attention-the-native-evidence-admission-boundary),
[OBS2](world-facts-observations-and-memory.md#obs2--preserve-evidence-meaning-through-memory-and-communication)
and [OBS3](agentic-character-runtime.md#obs3--resolve-observation-demand-across-controller-providers).

The tactical view keeps 16 actors, but the native decision path also selects
the cheap branch and folds every visible peer into memory. Those exact
memories can become crew reports. The measured view cap is real; it does
not certify bounded direct evidence or reporting. The first correction
routes tactical direct acquisition and memory refresh through the same
admitted set while preserving cheap target belief and intentional
omniscience. Old memory, a report or a hit can legitimately name an actor
outside current attention.

The memory row then loses evidence meaning. It drops observed liveness,
uses position confidence to decide whether an attributed hit changes
hostility, and treats confidence 1 as proof of a recent sight. A possessed
caller can retain old full-confidence memory while its autonomous fold is
skipped, then keep reporting it from its current location. A caller's
relative hostility can also make a report's subject believe it is its own
foe. These are source-derived sequences, not executed gameplay receipts.

Keep the sample source/cut distinct from a prediction and its confidence.
Carry observed liveness through the fold; retain a hit's independently
established contribution; give crew reports subject identity and their
original sight cut, then apply the receiver's existing assistance policy.
Do not turn these small rules into a generic evidence graph, permanent
incident journal or reputation system. Preserve Settle timing, room clearing,
direct-only reporting intent and the pirate crew customer.

Module controllers expose the same ownership gap from the other side.
Native demand comes from the native brain alone; `ActorThink.target`
separately rereads the target's live position. Resolve effective native/
module demand before one domain-owned projection is prepared. Offer a
selected sighted contract without narrowing today's authoritative world
rules or making a narrow module allocate a tactical view.

The social-state trace found deliberate saved Player hostility and seat
dialogue visits with different checkpoint horizons. It did not establish a
general theft/witness/recognition road. Preserve those policies and select
durable incident promotion only with its first actual dialogue customer.
A smaller independent [SS7 refinement](story-sequences.md#qualify-a-conversation-by-its-opening-scope)
captures opening room scope so equal authored pairs in two live instances
cannot share conversation identity.

The first useful performance changes are removing duplicate native work
and, if measured, selecting K borrowed candidates without sorting an N-row
temporary. Full sorting can remain the reference oracle. This pass does
not claim flat total room cost, a speedup, or a need for a data-structure
library. The plans specify native witnesses and controls, not an additional
test campaign.

### A view's selected policy owns its derived frame

**Owner:** [MV1](multiplayer-and-multiview.md#mv1--resolve-the-selected-view-before-its-framing-and-continuity),
with the existing M3 shared-orientation choice.

Per-view snapshot components coexist with a resolver that first requires
the session's default subject. A valid explicit no-driver spectator is
cleared when that default is absent. Conversely, a named body that has
disappeared can fall back to the still-valid default. When both resolve,
the explicit pane can inherit the other cast's eased size and the default
body's placement/blink reset. The common defect is resolving derived facts
before deciding which view policy requested them.

Resolve the declaration and selected population first. Omitted declaration
and failed explicit resolution are different. Scope framing extent and
temporal history to the view that consumes them, while sharing static
configuration and immutable observations where useful. Preserve the default
possession-follow policy's deliberate home/controlled-body dependencies,
native cast settling, portal chart continuity and current handoff easing.
No-driver views and separate 2D/2.5D adapters then use the ordinary seam.

The settings adapter also overwrites every view's reference-frame component
each Update. Make preference inheritance an explicit binding, so a declared
view policy survives the normal host path. A component-only test without
settings does not establish that composition.

For M3, distinguish body-relative control from observer-relative control.
One shared SubjectFrame cannot align simultaneously with upright and
inverted bodies. The current equivalence used by `under_camera` is valid
for the same controlled/observed basis; it is not a general shared-view
law. Preserve deliberate body-relative controls and held-direction
semantics. A selected observer-relative policy needs a logical admitted
input reference, never the eased render-camera angle.

### Source transitions, delivery and action continuation are separate

**Owner:** [IN4/IN5](participant-action-system.md#input-continuity-across-routing-and-control-changes),
with control authority and ACT2 retaining their existing responsibilities.

The local sampler resets analog hysteresis to Released while gameplay is
masked. Reopening can turn a continuous hold into a burst/C-stick/tap-jump
press. Rebind map replacement supplies another false neutral sample;
its button repair does not prime the runtime-owned analog carrier. Observe
valid physical samples before routing, discard masked edges, and treat
invalid map-replacement samples as a resynchronization boundary.

Native consumers expose a second meaning lost by filtering. Possession
rebuilds its combination edge from filtered held values, so focus reopening
can release possession under a continuous hold. A home-body fireball charge
can miss its release while captured or after possession moved its driver.
Current admitted input does not distinguish captured neutral from real
neutral, and the pending charge retains no initiating driver.

Carry bounded current supply eligibility and a retained interruption
through IN1's admitted input, preserving capture-and-resume between ticks.
Withdraw the old input-dependent continuation before admitting a new valid
edge in that same consume. Prediction must not repeat the interruption.
Observe prior driving ownership at its accepted transition or retain the
small continuation the native owner needs. The proposed pending fireball
policy cancels without firing; committed moves remain on their original
body, and an ACT2 body action hold does not itself end input ownership.
This needs neither a universal action cancellation bus nor a device epoch.

The first proofs must cross native capture/rebind, driver change and replay/
prediction boundaries. Source-derived cases are not executed witnesses.
There is no measured speedup: the planned state is bounded per participant
or view, and MV1 must not repeat all cast work for unrelated panes.

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

The observation pass also checked Radiant's room-rule observations and
fighter reaction timing. Radiant's current room-wide facts are intentional
executor inputs; LIFE1/LIFE2 already own dormant historical evaluation.
LIFE2 now records one concrete implementation trap: missing HTN operands
currently satisfy `Ne`, so omitting unavailable history alone is unsafe.
No second historical-facts program is selected.

Fighter engagement/pursuit deliberately reads a current admitted target
while tactical choice reads delayed perception. Existing guards cover that
policy. The mixed timing is not a newly found leakage defect. Production
world-value, body-query and whereabouts observation ports also remain
authoritative controls; the SDK label `Observation` does not mean NPC sight.

The provider-action audit found no production gameplay reader of
`SemanticActionPressed`. [P2](participant-action-system.md#p2--finish-provider-actions-at-composition-boundaries)
remains gated on a real final-hop consumer. A host event wired directly to
an effect would bypass recorded/peer input; registering result state for
rollback cannot recreate an omitted external cause. Refine that future
hop to use IN1 and existing native slot/technique/special admission. Keep
UI commands host-local, action occurrence distinct from content identity,
and wire vocabulary bounded. This is a guard against a proposed shortcut,
not a claim that the unbuilt bridge already fails in gameplay.

## Ranked candidates for the next audit

These are research questions, not established defects or selected work.
Each starts from an existing owner and a concrete consumer. An audit is
successful if it rules out a proposed abstraction as well as if it finds one.

### 1. Fresh implemented-head composition and native witnesses

The source-only candidates named by this review series have now received
owner guidance. This is not a claim that every engine area is complete.
On the unchanged source, another broad ownership/lifetime framework is less
useful than finding what survives an implemented cut.

Start from the next implementation head and its native results. Inspect one
boundary end to end: for example, source capture through IN1/IN5 into a
charge under possession and corrected input, or MV1's actual two-view host
through default loss and a separately selected target. Check that the
implementation removed the old authority, that a positive control still
works, and that its cost was not shifted into every unrelated consumer.
Then choose any new discrepancy as the next bounded audit question.

Do not reissue existing planned witnesses as newly discovered defects,
reopen completed receipts without contrary evidence, or expand the
backlog simply because no new runtime evidence is available.

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
