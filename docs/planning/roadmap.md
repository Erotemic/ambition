# Roadmap

The current repository state is summarized in [`status.md`](status.md); the
current queue is [`tracks.md`](tracks.md). This document holds only phases,
binding decisions, durable uncertainties, and questions that still require Jon.

**North-star oracle:** could another platformer be built by adding a content
crate without editing core?

## Phases

- **P1 — unification:** complete. Historical detail is archived.
- **P2 — decomposition and trust:** active tails. The crate/host/runtime split,
  authored placement channel, mode scope, module-size policy, and demo assembly
  exist. Remaining work is exact restore, enforceable collision/boss gates, and
  real encounter lifecycle convergence.
- **P3 — demo wave 1:** Sanic and Super Mary-O. Sanic needs reusable playable
  presentation/input proof. Mary-O needs its remaining equipment customer and
  full game acceptance.
- **P4 — demo wave 2:** Super Smash Siblings, then Hollow Lite. This phase pulls
  local multiplayer, the remaining combat stack, fighter-brain evaluation, and
  boss-quality installation policy.
- **P5 — long game:** relativity mechanics, moving/angled portals, online
  lockstep/rollback, additional acceptance games, public engine naming/versioning,
  and broader documentation refresh.
- **Ambition-the-game:** remains the first customer throughout; it consumes each
  capability rather than owning engine exceptions.

## Acceptance-game matrix

| Game | Primary stress | Phase |
|---|---|---|
| **Sanic** | momentum movement, hosted rules, playable presentation | P3 |
| **Super Mary-O** | classic AABB, equipment powerups, sequencing | P3 |
| **Super Smash Siblings** | N bodies/slots, full combat, local match state | P4 |
| **Hollow Lite** | exploration, boss-quality pipeline, respawn policy | P4 |
| MoneySeize | precision feel and economy | P5 |
| Celeste slice | assist modes, wind, room gimmicks | P5 |
| Metroid slice | item-gated traversal, maps, saves | P5 |
| Braid slice | snapshot/rewind | P5 |
| Dead Cells slice | runtime room-graph assembly | P5 |
| Rain World slice | rig animation and ecosystem AI | far edge |

## Binding decisions

M1 two-port body · M2 one control seam · M3 actors/props, no player/enemy type
axis · M4 relational state · M5 frame-agnostic mechanics · M6 install-time
content registries · M7 sprite metadata owns combat volumes · M8 LDtk owns space,
RON tuning, Yarn dialogue · M9 explicit time domains · M10 no generic pushout ·
M11 replace rather than bridge pre-release · M12 runtime plugin group owns
ordering.

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
| M22 | Cutscenes are scripted with limited interaction; encounters are interactive with limited scripting. Separate domain models; no universal sequence DSL. | [`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md) |
| M23 | A content eviction terminates in a structurally open ownership shape (provider-owned catalog/registration/presentation plugin), never a relocated closed table. | [`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md) |
| M24 | One placement-lowering authority: activation, reset, transition, and restore all lower through the App-installed registry. | [`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md) |

## Durable uncertainties

- **LDtk at scale:** keep the world IR backend-swappable.
- **Bevy churn:** the runtime group and narrow crate interfaces are the shield.
- **Feel drift:** repair with per-body data and differential tests, not divergent
  kernels.
- **Fighter-brain rollout cost:** L3 is budgeted and degradable; L2 remains a
  valid shipped tier if rollout proves too expensive.
- **Deep host services:** audio buses and save files may need a small explicit
  host-services contract when a hosted demo first requires them.
- **`features/` naming:** this is a naming decision only. No further
  `ambition_actors` crate split is owed by the decomposition plan.

## Questions for Jon

- ~~2026-07-15 recon decisions D1–D8~~ **RESOLVED 2026-07-16** — the
  fable+GPT-5.6 consensus, ratified by Jon, is
  [`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md)
  (accepted/rejected/deferred/repairs/campaigns/non-goals). Still open from it:
  the placement extension model, the provider crate name, and R6e.
- **Q1/Q3:** intended 1.0 audience; engine name and repository-split timing.
- **Q2-name:** retain `ambition_actors`, or choose a different public crate name.
- **Q5:** confirm online netcode remains post-1.0 while local-N ships with Super
  Smash Siblings.
- **R6e:** choose a coherent `features/` module/type-family rename or explicitly
  accept the documented current name. See
  [`engine/refactor-chain.md`](engine/refactor-chain.md).

## Standing practices

Trustworthy docs or no docs · data-driven ECS · evaluate ecosystem crates before
custom infrastructure · verify against the real headless simulation · visual and
feel commits remain BLIND until Jon judges them · commit is a checkpoint · record
units for measurements · archive completed narratives instead of carrying them
in the live queue · a NEW policy test must justify why the compiler, API design,
or a behavioral test cannot enforce the invariant (Jon, 2026-07-16 — existing
regression guards stay; no new scanner/poison ceremony without that justification).
