# Roadmap

Current source-backed state is in [`status.md`](status.md); execution order is in
[`tracks.md`](tracks.md); direct Jon decisions and confidence are in
[`maintainer-decisions.md`](maintainer-decisions.md).

**North-star composition principle:** providers add named game policy and content
through Bevy plugins and supported Ambition seams. Core changes are reserved for
reusable platformer capabilities rather than forbidden categorically.

## Phases

- **P1 — unified mechanical foundation:** substantially landed. One body path,
  explicit frames/time domains, world IR, movement models, moveset execution,
  and one-way observation are foundations rather than active decomposition work.
- **P2 — exclusive extension/lifecycle seams:** complete at the July 16 campaign
  bar. Placement lowering, provider lifecycle, session ownership, programmatic
  simulation, selected content ownership, touch separation, and repaired
  render/domain seams have one authoritative path.
- **P3 — acceptance-game pressure tests:** current product maturity phase. Super
  Mary-O has closed its level-1 gate; Sanic's complete-act proof and the remaining
  external provider-composition workflow evidence are still live. A demo may
  expose reusable engine work, but it may not create a named core branch or a
  private replacement for an ordinary engine responsibility.
- **P4 — richer mechanics and second consumers:** encounter lifecycle convergence
  is closed; later customers include Super Smash Siblings and Hollow Lite. Real
  second consumers decide optional domain and presentation seams.
- **P5 — broader maturity:** GGRS/bevy_ggrs drive rollback; later work includes
  stronger simulation-participation guarantees, local-N and optional online
  transport, additional acceptance games, measured budgets, and mature provider
  documentation.

The phase labels describe customer maturity, not a requirement to serialize all
work.

## Engine-competitiveness master plan

The complete capability destination, Bevy/Ambition ownership split, campaign
breakdown, and strategic dependency order live in
[`engine/competitive-2d-platformer-engine-roadmap.md`](engine/competitive-2d-platformer-engine-roadmap.md).
This file remains the durable phase map and decision register. Current facts live
in [`status.md`](status.md); executable ordering lives in [`tracks.md`](tracks.md).

The durable architecture sequence is:

1. finish the already-active construction/provider-composition closure;
2. make authoritative simulation participation and rewind proof difficult to omit;
3. finish participant action routing and add shared temporal action ownership on
   top of the landed slot-to-action and `MovePlayback` seams;
4. finish and enforce the existing swept collision/contact doctrine, then migrate
   remaining ad hoc consumers onto its cast/contact conventions;
5. compose shippable resource, input, presentation, persistence, and host
   capabilities with Bevy according to demonstrated game needs;
6. mature platformer-specific diagnosis, budgets, and supported provider seams.

Online transport may remain a live product track, but it is not a prerequisite
for declaring the single-player/local platformer engine core coherent.

## Acceptance-game matrix

| Game | Primary stress | Phase / state |
|---|---|---|
| **Sanic** | momentum movement, provider-owned playable identity, hosted rules | P3; mechanics/host path proven, complete act open |
| **Super Mary-O** | classic AABB, equipment/powerups, sequencing | P3; level-1 acceptance gate closed |
| **Super Smash Siblings** | N bodies/slots, full combat, local match state | P4 |
| **Hollow Lite** | exploration, encounter/boss quality, respawn/save policy | P4 |
| MoneySeize | precision feel and economy | P5 |
| Celeste slice | assist modes, wind, room gimmicks | P5 |
| Metroid slice | item-gated traversal, maps, saves | P5 |
| Braid slice | snapshot/rewind | P5 |
| Dead Cells slice | runtime room-graph assembly | P5 |
| Rain World slice | rig animation and ecosystem AI | far edge |

## Binding architecture decisions

M1 two-port body · M2 one control seam · M3 actors/props, no player/enemy type
axis · M4 relational state · M5 frame-agnostic mechanics · M6 install-time
content registries · M7 sprite metadata owns combat volumes · M8 LDtk owns space,
RON tuning, Yarn dialogue · M9 explicit time domains · M10 no generic pushout ·
M11 replace rather than bridge pre-release · M12 runtime owns global ordering.

| # | Decision | Owner |
|---|---|---|
| M13 | Path-dependent state uses swept evaluation. | [`engine/collision-and-ccd.md`](engine/collision-and-ccd.md) |
| M14 | Blocks are surfaces; AABB is a fast special case. | [`engine/spatial-model.md`](engine/spatial-model.md) |
| M15 | One damage meter, authored death policy. | [`engine/combat-model.md`](engine/combat-model.md) |
| M16 | Wearing a character means using that character's authored kit. | [`engine/unified-actors.md`](engine/unified-actors.md) |
| M17 | Shipped brains use the no-cheat observation contract. | [`engine/fighter-brain.md`](engine/fighter-brain.md) |
| M18 | Boss quality is measured by grammar, validation, and playtest data. | [`engine/boss-design.md`](engine/boss-design.md) |
| M19 | Demo rules are mode-scoped plugins. | [`demos/README.md`](demos/README.md) |
| M20 | Determinism is a managed same-build contract now; cross-platform bit exactness is not promised. | [`engine/netcode.md`](engine/netcode.md) |
| M21 | Encounter is orchestration, never an actor type. | [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md) |
| M22 | Cutscenes and encounters remain separate domain models; no universal sequence DSL. | [`maintainer-decisions.md`](maintainer-decisions.md) |
| M23 | Content eviction ends in an open provider-owned ownership shape. | [`maintainer-decisions.md`](maintainer-decisions.md) |
| M24 | Activation, reset, transition, and restore use one App-installed placement-lowering authority. | [`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md) |
| M25 | Session content is assembled deterministically, fingerprinted, and frozen; world construction is planned and validated before mutation, with explicit entity provenance. | [`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md) |
| M27 | GGRS and bevy_ggrs are the sole ephemeral rollback authority; Ambition owns only deterministic domain registration, exact content/schema binding, and session policy. | [`../adr/0027-ggrs-is-the-sole-rollback-authority.md`](../adr/0027-ggrs-is-the-sole-rollback-authority.md) |
| M26 | Room transitions are readiness-gated and progressively disclosed: the source room remains authoritative until one-shot target commit; fast loads avoid loading foregrounds, and slow or expensive commits occur behind a rendered cover without exposing partial rooms. | [`engine/room-transition-loading.md`](engine/room-transition-loading.md) |

## Durable uncertainties

- **LDtk at scale:** retain a backend-neutral world IR while actively developing one editor path.
- **Bevy churn:** narrow crate and plugin interfaces remain the shield.
- **Feel drift:** use per-body data and differential/property tests, never divergent kernels.
- **Deep host services:** audio/save/network services may need a small explicit contract when a real provider demands it.
- **Placement extension:** the common Tier-0 schema remains closed; whether providers ever receive a separate authored-placement channel is open.
- **Public naming:** the provider crate shipped as `ambition_platformer_provider`; engine/repository split timing and final `ambition_actors`/`features` names remain unsettled.
- **Boss carve:** convergence permits reassessment, but the current source review has not identified a concrete reuse, dependency, or build boundary; the maintainer ruling remains open.
- **Online transport:** GGRS integration is landed; Matchbox signaling/WebRTC and production connection policy remain after confirmed-frame effect quarantine.

## Standing practices

Trustworthy docs or no docs · data-driven ECS · evaluate ecosystem crates before
custom infrastructure · verify against the real headless simulation · visual
feel remains BLIND until judged · archive completed narratives · new scanners or
poison tests require evidence that types/APIs/behavioral tests cannot enforce the
invariant.
