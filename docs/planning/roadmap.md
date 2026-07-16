# Roadmap

Current source-backed state is in [`status.md`](status.md); execution order is in
[`tracks.md`](tracks.md); direct Jon decisions and confidence are in
[`maintainer-decisions.md`](maintainer-decisions.md).

**North-star oracle:** could another platformer be built by adding a provider and
content crate without editing core?

## Phases

- **P1 — unified mechanical foundation:** substantially landed. One body path,
  explicit frames/time domains, world IR, movement models, moveset execution,
  and one-way observation are foundations, not active decomposition campaigns.
- **P2 — make the extension/lifecycle seams exclusive:** current priority.
  Unify placement lowering, extract the provider protocol, finish session-root
  authority/exact reconstruction, evict named content structurally, and extract
  the simulation harness.
- **P3 — first complete external-game proofs:** Sanic and Super Mary-O close
  their full playable/headless acceptance loops without engine exceptions.
- **P4 — richer mechanics and second consumers:** boss/moveset convergence,
  encounter lifecycle convergence, Super Smash Siblings, and Hollow Lite. These
  customers decide whether menu or boss crate extraction is warranted.
- **P5 — mathematical and distributed generalization:** stronger frame/clock
  mechanics, slower-light observation, moving/angled portals, rollback/netcode,
  additional acceptance games, and public engine naming/versioning.

Ambition-the-game remains the first customer throughout; it consumes capabilities
rather than defining exceptions inside reusable crates.

## Acceptance-game matrix

| Game | Primary stress | Phase |
|---|---|---|
| **Sanic** | momentum movement, provider-owned playable identity, hosted rules | P3 |
| **Super Mary-O** | classic AABB, equipment/powerups, sequencing | P3 |
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

## Durable uncertainties

- **LDtk at scale:** retain a backend-neutral world IR while actively developing one editor path.
- **Bevy churn:** narrow crate and plugin interfaces remain the shield.
- **Feel drift:** use per-body data and differential/property tests, never divergent kernels.
- **Deep host services:** audio/save/network services may need a small explicit contract when a real provider demands it.
- **Placement extension:** the common Tier-0 schema remains closed; whether providers ever receive a separate authored-placement channel is open.
- **Public naming:** provider crate name, engine/repository split timing, and final `ambition_actors`/`features` names remain unsettled.
- **Online netcode:** remains later than local multiplayer unless Jon changes scope.

## Standing practices

Trustworthy docs or no docs · data-driven ECS · evaluate ecosystem crates before
custom infrastructure · verify against the real headless simulation · visual
feel remains BLIND until judged · archive completed narratives · new scanners or
poison tests require evidence that types/APIs/behavioral tests cannot enforce the
invariant.
