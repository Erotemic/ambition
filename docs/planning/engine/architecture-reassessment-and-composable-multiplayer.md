# Engine architecture reassessment: optional simulation backends, peer multiplayer and physical objects

**State:** ACTIVE DESIGN REVIEW (2026-10-26); not an implementation-complete claim or a new execution queue.

**Baseline:** `54b1d45c96fa` source snapshot from 2026-10-26; recheck HEAD before writing code.

**Execution:** [`docs/planning/queue.md`](../queue.md) remains the execution selector; this document proposes experiments, not a blanket rewrite.

**Proposed ADRs:** [0035 — no mandatory service](../../adr/0035-distributed-multiplayer-without-required-services.md), [0036 — optional synchronization](../../adr/0036-synchronization-is-an-optional-game-capability.md), [0037 — simulation islands](../../adr/0037-causally-isolated-simulation-islands.md), [0038 — physical objects](../../adr/0038-persistent-objects-and-optional-physics.md), and [0039 — crate boundaries](../../adr/0039-evidence-based-crate-boundaries.md).

## Why the plans are being reopened

The source has strong deterministic and rollback foundations, but the earlier plan incorrectly
let whole-App GGRS history, room rebases, session lifecycle and app composition stand in for a
general multiplayer architecture. The practical result is that independent players in different
rooms still share a global synchronization/lifecycle barrier. This does not mean the rollback
engineering was wasted: it remains the correct starting point for bounded competitive games.

Plans and accepted ADRs are **reviewable engineering decisions**, not source facts or
irrevocable maintainer constraints. The binding product constraints here are: games can exclude
rollback and full physics; preserve determinism; Smash needs rollback; Mary-O/Sanic multiplayer,
if selected, is level-scoped and completion waits for all required players; Ambition wants
independent-room cooperative exploration; and multiplayer cannot require any company-operated
server. The room/island protocol, authority migration, regional rollback, networking libraries
and exact crate layout remain open.

## Product constraints versus architectural hypotheses

| Category | Rule | State |
| --- | --- | --- |
| Explicit constraint | No mandatory company-operated central server or online service; offline play and possible peer-to-peer play must survive a provider shutdown | Required product property |
| Explicit constraint | Keep deterministic mechanics; support no-rollback games, optional richer physics, Smash rollback | Required engine properties |
| Product direction | Ambition friends separate, cooperate and communicate across several live rooms without unrelated players stalling at a door | Core Ambition design target |
| Product direction | Mary-O/Sanic multiplayer stays in one level; the level completes only when all participating players finish | Game policy, not a network backend |
| Candidate | A participant may host an authoritative world while also playing | Recommended first comparison, not committed implementation |
| Candidate | Causally isolated simulation islands optionally have independent rollback histories | Feasibility research only |
| Candidate | Region-scoped authoritative replication and prediction | Feasibility research only |
| Candidate | Optional Avian2D mechanics, optional cosmetic ragdolls, persistent movable objects | Proposed capability boundaries |
| Candidate | Consolidate/rename crates where the resolved consumer matrix proves value | No merges authorized |

## Separate the axes that the current host conflates

1. **Authoritative simulation:** same gameplay systems, deterministic ordering, stable identities, explicit inputs and fixed simulation time; the base does not assume save/load history.
2. **Tick policy:** independent of render cadence; 60 Hz remains the current implementation, with configurable rates requiring a separate compatibility audit.
3. **Synchronization backend:** local fixed step, bounded input lockstep, GGRS rollback, or authoritative replication; selecting one does not imply the others.
4. **Synchronization extent:** match, level, or a causally closed region set; not necessarily a physical room or the full Bevy App.
5. **Persistence and committed events:** world save ownership, quest and object state, body and custody handoff, confirmed effects; persistence is not GGRS history.
6. **Presentation:** local seat, view, loading cover, ragdoll rendering and quality policy; client rendering readiness must not control unrelated authoritative timelines.
7. **Physical simulation:** static placement, simple deterministic motion, optional rigid bodies, optional articulated ragdolls; a physics solver is not an object identity service.

Do **not** assume `ambition_sim_view` is a ready-made wire protocol. A remote client needs
stable replication identities, interest filtering, baseline/delta or equivalent state transfer,
confirmation, loss handling and version negotiation. View read models are presentation-oriented
and may use local entity identities.

## Target game compositions to prove, not merely name

| Consumer | Desired model | Essential negative dependency test |
| --- | --- | --- |
| Independent minimum/headless game | Deterministic configurable fixed-step movement and collisions | No mandatory GGRS, named bosses, dialogue, UI, renderer, or Avian |
| Smash | One bounded arena, GGRS P2P rollback and confirmed effects | No Ambition persistent-world/residency requirement |
| Mary-O | Level-scoped synchronization, all-player finish gate | No general multi-room lifecycle or persistence replication |
| Sanic | Level-scoped synchronization, all-player finish gate | No general multi-room lifecycle or persistence replication |
| Ambition offline | One persistent world with multiple active room instances | No GGRS or external service required |
| Ambition co-op | Independent participants/regions, temporary peer world authority, optional region-specific prediction/rollback | No company server; no unrelated-room global rollback barrier |
| Cosmetic ragdoll client | A death event and local articulated rendering | No gameplay collision change or authoritative solver requirement |

These are compatibility targets, not claims of current support. Game modes may choose to share
an implementation crate while using different runtime installation and network policies.

## The boundary problem: room, region, island and session

A spatial room is an authored/residency unit; it is not automatically a rollback island. An
**island** is a causally closed set of mutable facts, while a **world session** coordinates
admitted regions and committed facts. Examples:

- Two players fighting in one room may share a bounded rollback population; a third player in
  another room should not rewind with them.
- A portal that only displays the other room does not connect simulation authority. A
  projectile, limb or carried object capable of immediate cross-room effect **does** connect
  them; merge the affected island histories or cross a confirmed delayed-event boundary.
- A body stepping through an ordinary door carries stable identity, mechanical state and custody
  through one identified transfer. The source relinquishes authority and the destination accepts
  it exactly once; unrelated regions keep advancing.
- Quest facts, durable pickups and placed props are committed across regions. A rollback of
  speculative combat cannot remove a reward already made canonical elsewhere without an explicit
  compensation protocol.

The current GGRS session-wide `lifecycle_commit` protocol remains a valid *implemented safety
mechanism* for that backend. It must not become a universal rule for every game host. An island
proof requires real state isolation, not just a room filter on a global solver or the presence
of `InRoomInstance`.

## Physics, props and corpse semantics

A persistent physical object owns its stable occurrence identity, placement, current custody,
room membership and disposition horizon independently of movement policy. Its motion may be
static, simple deterministic/kinematic, Avian2D, or an authored animation. A visual ragdoll may
render a death that already happened without changing collision or persistent world state. An
interactable corpse with mechanical collision is a different, authoritative object and must
follow the chosen region's synchronization and persistence rules.

Clients may independently reduce **cosmetic** physics quality. They may not disable
collision-active, switch-triggering, carryable or puzzle-critical solver rules while remaining
authoritative peers of the same game. A host may choose a ruleset/physics policy for the shared
simulation, but its results must be consistently observed. Avoid making global solver state part
of an unrelated region's rollback history by default.

## Crate and SDK policy while investigating

- Preserve the semantic facade as the current primary game API and the independent extension
  SDK; neither a facade-only rule nor the existing crate topology is beyond review.
- Classify crates as domain, pure contract/spec, integration adapter, backend, presentation,
  tooling, fixture or composition root, then inspect actual resolved dependency closures.
- Do not merge by size, direct consumer count, or the fact that a game installs two crates
  together. A tiny spec may be what permits a headless or portable consumer.
- Do not extract by filename size, old D33 carve count, module-cycle count, or a blanket rule
  that every capability gets a crate.
- A current misleading name such as `actor_monolith` can be corrected when its responsibility is
  verified. Treat owner identifiers, asset paths and rollback/schema names as *different
  compatibility surfaces* from Rust package names.
- For `shared_tangle`, identify the foundational contracts versus active construction/lifecycle
  authority versus host/presentation concerns **by real consumers and dependency direction**
  before deciding whether to rename, divide, or retain it.
- Prototype irreversible merges on branches with measured before/after compile closure and
  source behavior, preserving a restoration route; separate mechanical renames from algorithmic
  work.

## Evidence sequence before committing to a network model

The following is an experiment backlog, **not a replacement for `docs/planning/queue.md`**.
Select one minimal experiment at a time and report actual results before planning larger
migrations.

1. **Offline minimum without rollback.** Use a real external/headless game with deterministic fixed-step movement; demonstrate absence of `bevy_ggrs` from its resolved Cargo closure and compare CPU/memory/scheduling with the rollback host.
2. **Smash rollback as independent consumer.** Exercise two real processes with a bounded match, exact input/content identity and the same movement/combat implementations; measure confirmed effects and latency.
3. **Independently reversible regions.** Drive two rooms concurrently, rewind one and prove the other and committed global facts remain bit-stable; compare isolated Bevy worlds to bounded snapshots and measure overhead.
4. **Body/prop custody handoff.** Transfer exactly once at an agreed tick with failure injection and competing transfers. Test cross-room portals before declaring room == island.
5. **Peer-hosted network vertical slice.** Two real client processes, direct/LAN networking and one participant-authoritative world; a participant changes rooms while the other keeps playing; no external service.
6. **Gameplay physics choice.** Carry, place, leave, reload and retrieve one generic prop under a simple solver and optional Avian; measure solver cost and state horizon.
7. **Visual ragdoll choice.** Two clients render different death presentations from the same confirmed event, but observe identical canonical mechanical state.
8. **Dependency-boundary audit.** For any rename or merger, produce a closure diff across minimum, Smash, Mary-O, Sanic, Ambition, extension and external consumers; authorize changes only after independent review.

## Risks and explicit nonclaims

- Current code does **not** implement island-scoped GGRS, remote host-authoritative replication,
  generic object placement, host migration or a multiplatform fully deterministic solver solely
  because a document proposes them.
- Same-build determinism does not imply cross-platform bit-exact P2P rollback. Android/desktop
  interoperability needs a measured protocol choice.
- No persistent central service does not mean arbitrary NAT traversal works without optional
  relay infrastructure.
- Without a network partition/conflict policy, a temporary peer authority may lose session
  availability if its machine leaves. An ordinary portable save is a valid first product
  guarantee; automatic host migration remains undecided.
- A physically connected set of rooms may need one rollback history; high-fidelity ragdolls or
  contraptions should not automatically make every room in the world one island.
- Current status pages, historical issue ordering and automated guard baselines are evidence for
  their own dates; do not silently rewrite them to claim planned capabilities shipped.

## Interacting owner documents

Read [architecture](architecture.md), [capability composition](capability-and-runtime-composition.md), [netcode](netcode.md), [network topology research](network-topology-and-authority-research.md), [multiplayer and multiview](multiplayer-and-multiview.md), [open-world residency](open-world-runtime-and-residency.md), [simulation determinism](simulation-authority-and-determinism.md), [public SDK](public-sdk-1.0.md), [crate consolidation](../consolidation/consolidation-plan.md), [room transitions](room-transition-loading.md), [local co-op](local-coop-integration.md), [tick-rate compatibility](simulation-tick-rate-compatibility.md), [item custody](item-custody-and-accounting.md), [kinematic world objects](kinematic-world-objects.md),
and [semantic part ragdolls](semantic-part-rendering-and-ragdolls.md).

When a new experiment changes an old assumption, update the affected ADR and owner plan, not
just this page. Avoid new standing checkers if a small behavioral consumer fixture can enforce
the boundary.
