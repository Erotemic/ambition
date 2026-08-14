# Roadmap — Ambition and Engine 1.0

Current facts are in [`status.md`](status.md). Current execution order is in the
self-replenishing [`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md).
[`tracks.md`](tracks.md) is the standing reservoir, not a second queue.

## North star

Ambition is the flagship game. The engine should become a credible
Godot/Unity-class 2D engine because that makes Ambition better to build and makes
its successful capabilities reusable by other games.

The external-composition oracle remains useful:

> Can Ambition use the capability deeply while another game can opt into the
> same capability through supported engine/provider seams without editing
> Ambition-specific engine code?

That is stricter than "can the Ambition app do it" and more useful than abstract
internal purity.

## Post-D73 phase map

D73 closed the duplicate character/body authority. The next phases are capability
programs rather than another migration chronology.

### E1 — world authoring and kinematic world objects

Ambition's level-building loop is a product surface. Mature LDtk integration,
typed references, validation and semantic tooling, with moving platforms as the
first dynamic-world vertical slice.

Owners:
[`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md),
[`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).

### E2 — simulation authority and deterministic composition

Make mutation phases, actor/control authority, rollback participation and
cross-domain ownership explicit enough that scheduler topology and global type
censuses cannot silently change semantics.

Owner:
[`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

### E3 — multiplayer, multi-view and world residency

One participant model should support solo, couch co-op, online co-op and mixed
local/remote play. Presentation may be shared, fixed-split or adaptively split;
participants may occupy different rooms when rules permit it.

Ambition is the primary customer. TwinTrack is a strong independent-observer
acceptance customer.

Owners:
[`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md),
[`game/multiplayer.md`](game/multiplayer.md).

### E4 — capability/runtime composition

Make optional capabilities honest in dependencies and plugin/runtime assembly.
A minimal game should not inherit unrelated bosses, portals, persistence,
rollback adapters, presentation or Ambition-specific policy by accident.

Owner:
[`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

### E5 — public SDK and authoring ergonomics

Expose semantic game concepts rather than implementation topology. Improve the
workflow for worlds, characters, actions, encounters, assets, services,
diagnostics and provider extensions by exercising real consumers rather than
institutionalizing source-text/API migration rituals.

Owner: [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md).

### E6 — performance and iteration

Compile topology, runtime budgets, multi-view rendering cost, asset residency,
mobile quality profiles, headless throughput and content iteration are engine
product quality.

Owner:
[`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

The phase labels describe strategic capability fronts, not a requirement to
serialize all work.

## Game/customer matrix

| Game | Project role | Primary pressure |
|---|---|---|
| **Ambition** | **flagship / primary product** | deep content, LDtk authoring, possession, portals, persistence, multiplayer, multi-room presentation, long-term engine ergonomics |
| **Super Smash Siblings** | serious acceptance customer; possible future first-class game | N participants, body-generic combat, fighter AI, match state, stage authoring |
| **TwinTrack** | acceptance/research game | independent observer/reference-frame views, split-screen, unusual spatial presentation |
| **Sanic** | acceptance game | high-speed movement/collision and momentum feel |
| **Super Mary-O** | acceptance game | classic platforming, authored levels, equipment/powerups |
| **Hollow Lite** | acceptance game | encounters, bosses, respawn/save and combat authoring quality |

Acceptance games are not disposable tests: they are persistent engine customers.
A customer may graduate into a first-class game if product investment warrants
it. Ambition remains the main game.

## Binding architectural decisions

M1 two-port body · M2 one control seam · M3 actors/props, no player/enemy type
axis · M4 relational state · M5 frame-agnostic mechanics · M6 install-time
content registries · M7 sprite metadata owns combat volumes · M8 LDtk owns space,
RON/Rust/provider data own non-spatial authored composition, Yarn owns dialogue ·
M9 explicit time domains · M10 no generic pushout · M11 replace rather than
bridge pre-release · M12 runtime owns global ordering.

| # | Decision | Owner |
|---|---|---|
| M13 | Path-dependent state uses swept evaluation. | [`../concepts/movement-collision.md`](../concepts/movement-collision.md) |
| M14 | Blocks are surfaces; AABB is a fast special case. | [`engine/spatial-model.md`](engine/spatial-model.md) |
| M15 | One damage meter, authored death policy. | [`engine/combat-model.md`](engine/combat-model.md) |
| M16 | Wearing a character means using that character's authored kit. | [`../concepts/one-body-one-path.md`](../concepts/one-body-one-path.md) |
| M17 | Shipped brains use the no-cheat observation contract. | [`engine/fighter-brain.md`](engine/fighter-brain.md) |
| M18 | Boss quality is measured by grammar, validation and playtest evidence. | [`engine/boss-design.md`](engine/boss-design.md) |
| M19 | Game/mode rules are scoped composition, not core actor taxonomy. | [`demos/README.md`](demos/README.md) |
| M20 | Determinism is a managed same-build contract now; cross-platform bit exactness is not promised. | [`engine/netcode.md`](engine/netcode.md) |
| M21 | Encounter is orchestration, never an actor type. | [`../systems/boss-encounter-architecture.md`](../systems/boss-encounter-architecture.md) |
| M22 | Cutscenes and encounters remain separate domain models; no universal sequence DSL. | [`maintainer-decisions.md`](maintainer-decisions.md) |
| M23 | Content eviction ends in an open provider-owned ownership shape. | [`maintainer-decisions.md`](maintainer-decisions.md) |
| M24 | Activation, reset, transition and restore use one App-installed placement-lowering authority. | [archived decision record](../archive/planning-superseded/2026-08-13/engine/decisions-2026-07-16.md) |
| M25 | Session content is assembled deterministically, validated before mutation and committed transactionally. | [`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md) |
| M26 | Room transitions are readiness-gated; source authority remains until one-shot target commit. | [`engine/room-transition-loading.md`](engine/room-transition-loading.md) |
| M27 | GGRS/bevy_ggrs own ephemeral rollback; domains own deterministic state declarations and session policy. | [`../adr/0027-ggrs-is-the-sole-rollback-authority.md`](../adr/0027-ggrs-is-the-sole-rollback-authority.md) |
| M28 | The actor monolith is decomposed incrementally by semantic ownership and measurable dependency/change amplification. | [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md) |
| M29 | Transport, control assignment, world residency and local view layout are orthogonal multiplayer axes. | [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md) |
| M30 | Views observe one simulation; split-screen is indexed local presentation, not duplicated game state. | [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md) |

## Durable uncertainties

- **LDtk at scale:** invest deeply in the current preferred editor while keeping
  the world IR backend-neutral.
- **Multi-room residency:** grow from a concrete two-participant/two-room
  Ambition slice before designing a general open-world streamer.
- **Network product policy:** transport is an engine capability; matchmaking,
  join/save ownership and narrative party rules are game policy.
- **Capability granularity:** split real independently useful domains; do not
  create package noise for theoretical optionality.
- **Bevy churn:** narrow semantic crate/plugin interfaces remain the shield.
- **Feel drift:** authored body policy and behavioral/property tests beat
  divergent kernels.

## Standing practices

Trustworthy docs or no docs · Ambition-first product pressure · data-driven ECS ·
evaluate ecosystem crates before custom infrastructure · verify against the real
headless simulation · visual feel remains BLIND until judged · archive completed
migration narratives · source scans/poison tests are exceptional rather than a
planning default.
