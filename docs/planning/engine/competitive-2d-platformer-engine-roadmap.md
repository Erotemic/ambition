# Competitive 2D platformer engine — remaining capability gaps

> **Verified against `cecd01ca` (2026-08-13).** The original master campaign
> accumulated the implementation history for capabilities that are now largely
> present: shared movement, deterministic/headless execution, the action seam,
> swept contact doctrine, prepared/transactional construction, provider
> composition, input sources, animation/camera policy, the shell audit, and a
> versioned checkpoint/save flow. The complete campaign record is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/competitive-2d-platformer-engine-roadmap.md`](../../archive/planning-superseded/2026-08-13/engine/competitive-2d-platformer-engine-roadmap.md).
>
> This page is now a strategic **remaining-work map**, not an execution queue.
> [`../queue-72h-2026-08-08.md`](../queue-72h-2026-08-08.md) owns execution order;
> [`../tracks.md`](../tracks.md) is the standing reservoir.

## Product bar

Ambition should become unusually coherent for 2D platformers rather than
matching Unity/Godot/Unreal feature-for-feature. A competent external developer
should be able to build a materially different game through supported provider,
content, control and runtime seams without learning Ambition's migration history
or editing Ambition-specific engine internals.

The engine should preserve:

- one rich controlled-body/actor path regardless of controller kind;
- deterministic/headless simulation and rollback correctness;
- explicit state, policy and lifecycle ownership;
- authorable actions, contacts, construction and content diagnostics;
- Bevy-native composition rather than a second general-purpose engine layer;
- low change amplification and reasonable compile/iteration cost;
- shipping-quality desktop/mobile/web profiles where there is a real customer.

## Foundations already present — do not schedule as fresh architecture

Current source already provides substantial implementations of:

- specialized shared movement kernels, gravity/reference-frame support,
  one-ways/slopes/moving platforms and ledge/dodge/shield movement vocabulary;
- GGRS-backed rollback schedules, snapshot codecs, headless simulation and
  deterministic control frames;
- shared character/action/moveset/combat primitives and prepared characters;
- canonical `Contact`/`SweepSample`/cast vocabulary and swept movement;
- immutable prepared content, structured construction plans and room staging;
- provider/plugin composition, semantic facade surfaces and content packs;
- keyboard/gamepad/touch/replay/brain input sources plus participant/seat state;
- provider-driven animation/camera/presentation paths;
- shell, menus, settings, audio, VFX and renderer integration;
- versioned persistence plus checkpoint routing through ordinary room transition.

The remaining work below should **finish gaps in these systems**, not replace
them with parallel frameworks.

## A. Authority and deterministic simulation

### Finish current actor/body authority convergence

The immediate authority is
[`../authority-convergance-campaign-2026-08-13.md`](../authority-convergance-campaign-2026-08-13.md).
Close D73 before broadening the architecture campaign.

### Invert rollback participation after D73

The generic runtime still knows domain-specific rollback types. Replace the
central census with a declaration/registration seam that preserves deterministic
schema ownership without pushing a networking implementation dependency into
leaf gameplay crates. This is a successor campaign, not part of D73.

### Decompose oversized ECS authority

Use
[`../triage/bevy-system-parameter-architecture.md`](../triage/bevy-system-parameter-architecture.md)
and
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md).
Split systems/crates around coherent authority and phases; do not hide broad
access behind tuple/SystemParam packing or create a new umbrella context.

## B. Participants, actions and local-N composition

Continue
[`participant-action-system.md`](participant-action-system.md) and
[`character-actions.md`](character-actions.md):

- finish participant/seat/control-channel ownership where legacy identity still
  leaks into gameplay;
- finish context-aware inventory/specialized menu control;
- complete the physical-binding side of provider-defined semantic actions;
- prove local-N composition through the Smash acceptance customer rather than a
  second input stack.

## C. Collision, interaction and combat geometry

The shared sweep/contact foundation is present. Remaining work is at specific
consumers:

- finish shaped/multi-part hurt and attack volumes in
  [`../shaped-volumes-campaign-2026-08-01.md`](../shaped-volumes-campaign-2026-08-01.md);
- finish the real movement-kernel → loading-zone transaction case in
  [`room-transition-loading.md`](room-transition-loading.md);
- make projectile/victim collision consume the victim's authored published
  silhouette where the coarse box still loses gameplay information;
- finish only genuinely desired platform-fighter combat vocabulary listed in
  [`combat-model.md`](combat-model.md), after the current D72 feel/body-generic
  campaign establishes the need.

## D. Construction, providers and public engine surface

Continue the residual work in
[`immutable-content-and-transactional-construction.md`](immutable-content-and-transactional-construction.md),
[`api-1.0-campaign.md`](api-1.0-campaign.md), and
[`binding-resolution-boundary.md`](binding-resolution-boundary.md):

- snapshot/reconstruction and possession/transition/carry cases must use the
  same canonical prepared construction facts;
- close optional-capability leakage that forces unrelated domains into a
  consumer's dependency closure;
- establish a domain-owned rollback declaration seam;
- use at least one materially external consumer to prove the public facade and
  provider extension paths;
- make unresolved declared content fail authoring/preparation or produce
  actionable provenance instead of silently disappearing.

## E. Shippable runtime gaps

### Asset/package lifecycle

Use Bevy's asset/runtime substrate while preserving exact content identity and
transactional activation. Remaining concrete work lives in
[`../sprite-residency-and-live-quality.md`](../sprite-residency-and-live-quality.md),
[`../triage/declared-id-resolution-checks.md`](../triage/declared-id-resolution-checks.md),
and the shipping/profile items in [`../tracks.md`](../tracks.md).

Do not build another asset database merely to resemble an editor engine.

### Presentation completeness

Rendering/audio/UI/settings are existing systems, not blank-slate roadmap items.
The verified residual capability gaps are in
[`presentation-and-shell-audit.md`](presentation-and-shell-audit.md) and
[`../triage/gameplay-presentation-profiles.md`](../triage/gameplay-presentation-profiles.md):

- localization when a real localized customer exists;
- real accessibility gaps such as general text scaling/non-dialogue captions and
  presentation use of authored color/flash preferences;
- safe-area/overlap/layout behavior and live quality application;
- per-route music where an experience genuinely needs route-specific scoring.

### Host profiles and measured budgets

Keep explicit supported profiles for desktop/web/mobile/headless builds. Measure
representative workloads and produced artifacts before creating performance or
platform campaigns. Hardware-specific work needs a reproducible customer/device.

## F. Diagnostics and evaluation

The repository already has gameplay traces, headless harnesses and debug views.
The remaining maturity work is to make them answer real questions cheaply:

- finish the fighter-brain scenario evaluation/calibration rig in
  [`fighter-brain.md`](fighter-brain.md);
- improve cross-tick causal/frame explanation when an actual debugging case
  cannot be answered from existing traces;
- maintain measured compile/runtime/host budgets for representative workloads,
  not prose estimates or permanent source-scanner rituals.

## G. Persistence and long-lived content

The Ambition save has versioning, migration verdicts and checkpoint routing.
Do not reopen persistence as a generic architecture campaign. Add provider-owned
save fragments, item identity or richer migration policy when a second real
consumer needs them; the trigger-based work is recorded in
[`closeout-review-followups-2026-07-20.md`](closeout-review-followups-2026-07-20.md).

## Competitive acceptance levels

### Core engine competitive

- authoritative simulation participation is structurally hard to omit;
- one rich body path serves human, AI, possessed and match-controlled bodies;
- participant/context routing has one semantic path;
- representative mechanics use canonical contacts/casts and construction;
- headless and rollback runs prove the same authoritative outcomes;
- providers compose through supported plugins/data seams without editing engine
  internals.

### Shippable runtime competitive

- declared host profiles boot produced artifacts and enter representative rooms;
- asset/readiness failures are transactional and diagnosable;
- required device/context profiles work through participant-local semantics;
- animation/camera/render/audio/UI/settings/persistence gaps are driven by real
  game customers rather than duplicate frameworks;
- representative performance/iteration budgets are measured.

### Mature reusable engine competitive

- multiple materially different games use the same core semantics;
- provider documentation exposes stable-enough extension patterns;
- causal diagnostics explain important platformer decisions;
- optional local-N, online rollback transport, exploration persistence, unusual
  gravity and alternate authoring backends compose without becoming mandatory
  dependencies for every game.

## Deferred / acceptance-driven candidates

Keep these dormant until a focused customer supplies concrete pressure:

- production online transport beyond the existing rollback contracts;
- capability-conditioned traversal graphs;
- generalized animation blend trees;
- advanced 2D lighting/occlusion/postprocessing beyond game needs;
- a standalone editor/inspector application;
- broad authoring-backend importers beyond active LDtk needs;
- public long-term API compatibility guarantees before a real release;
- platform profiles without hardware, a shipping customer or reproducible CI.

A deferred candidate becomes active only when there is a concrete consumer,
missing capability, intended owner, smallest proof and deletion/migration plan.
