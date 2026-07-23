# Roadmap

Current source-backed state is in [`status.md`](status.md); execution order is in
[`tracks.md`](tracks.md); direct Jon decisions and confidence are in
[`maintainer-decisions.md`](maintainer-decisions.md).

**North-star oracle:** could another platformer be built by adding a provider and
content crate without editing core?

## Phases

- **P1 — unified mechanical foundation:** substantially landed. One body path,
  explicit frames/time domains, world IR, movement models, moveset execution,
  and one-way observation are foundations rather than active decomposition work.
- **P2 — exclusive extension/lifecycle seams:** complete at the July 16 campaign
  bar. Placement lowering, provider lifecycle, session ownership, programmatic
  simulation, selected content ownership, touch separation, and repaired
  render/domain seams have one authoritative path.
- **P3 — complete external-game proofs:** current product priority. Finish one
  complete headless/visible Super Mary-O level and one complete Sanic act without
  engine exceptions.
- **P4 — richer mechanics and second consumers:** encounter lifecycle
  convergence closed 2026-07-16; later customers include Super Smash Siblings
  and Hollow Lite. Real second consumers decide menu or other crate boundaries.
- **P5 — rollback and broader generalization:** GGRS/bevy_ggrs now drive the
  real simulation harness and the custom rollback engine is gone. Production
  online work starts with confirmed-frame side-effect quarantine and a
  Matchbox-backed two-peer handshake; broader generalization continues through
  ConstructionPlan, stronger frame mechanics, and additional acceptance games.

The phase labels describe customer maturity, not a requirement to serialize all
work. Encounter convergence and atomic room restore may proceed while P3 demos
close.

## Next major engine-architecture push

The immutable prepared-content and exact session-identity milestone of
[`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md)
is complete, and so is its explicit-provenance plus three-origin
`ConstructionPlan` vertical slice (Milestone B, 2026-07-22): one authored
placement, one provider-staged actor, and one runtime-dynamic family now share a
pure, preflightable planner and a recipe-backed reconstruction path, and
`SpawnOrigin` replaced the id-string parsing that stood in for provenance.

The current broad architecture push is that doc's Phase 4 — making room
activation, reset, transition, hot reload, and snapshot reconstruction
variations of ONE construction transaction, which turns the remaining
family-specific spawn loops into plan rows. **Its substrate closed 2026-07-23**
(Checkpoints A through "C step 2"): the room transaction owns publication and
verification at the outer boundary, the authoritative roster and plan identity
derive from the completed plan, rig composition is verified exactly, and the
first two families migrated by the pattern Phase 4 repeats — giant hosts+hands
and authored mount links are plan rows with wired-and-verified relations
(`PendingMountLinks` deleted). Remaining: the nine-family migration, lifecycle
unification, and the staging/commit boundary that turns verification from a
detector into a preventer (tracks.md #5 has the decomposition). That is the
foundation for prefab-like authoring, broader transactional room replacement,
persistence/reconstruction, and a credible external SDK. Rollback itself is
owned by GGRS (ADR 0027).

Ambition-the-game remains the first customer throughout; it consumes capabilities
rather than defining exceptions inside reusable crates.

### Immediate room-loading integration — LANDED 2026-07-17

[`engine/room-transition-loading.md`](engine/room-transition-loading.md)'s
Phases 1–4 executed: ordinary transitions mint a real readiness barrier through
the load coordinator, the source room stays authoritative until one-shot
commit, presentation is contributor-neutral (`LoadPresentationCommand` + shell
adapter), and neighbor prefetch with promotion is concrete. Remaining: Phase 6
performance closure; full canonical-plan convergence rides the provenance
track.

## Current critical path

```text
immutable PreparedContent + exact GGRS session identity (done)
        ↓
confirmed-frame external-effect quarantine (done 2026-07-21)
        ↓
Matchbox two-peer handshake + predicted-A/corrected-B oracle   ← next online step

in parallel (CURRENT):
  explicit provenance + three-origin ConstructionPlan vertical slice (done)
        ↓
  transaction substrate: boundary/roster/plan-id/relations (done 2026-07-23)
        ↓
  Phase 4 — nine-family migration + lifecycle unification + commit boundary

  Super Mary-O level-1 acceptance (done 2026-07-21)
  Sanic complete-act acceptance (open — demos/sanic.md)
```

## Acceptance-game matrix

| Game | Primary stress | Phase / state |
|---|---|---|
| **Sanic** | momentum movement, provider-owned playable identity, hosted rules | P3; mechanics/host path proven, complete act open |
| **Super Mary-O** | classic AABB, equipment/powerups, sequencing | P3; core level mechanics landed, full level proof open |
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
