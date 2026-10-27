# Multiplayer network topology and authority — requirements fixed, topology in research

**Status: TOPOLOGY STILL UNDECIDED; requirements clarified 2026-10-26.**
**Recorded:** 2026-10-25; updated following the maintainer's 2026-10-26 architecture discussion.
**Scope:** Comparative research, not an implementation mandate or queue entry.

**Explicit constraints now recorded:** no mandatory company-operated service or central game server; independent-room Ambition multiplayer; deterministic simulation; rollback optional for games with unbounded world state and required as an option for Smash. A player temporarily hosting authoritative state is a candidate topology, not a company-run service. See [ADRs 0035-0037](../../adr/README.md) and [architecture reassessment](architecture-reassessment-and-composable-multiplayer.md).

> **Maintainer position updated 2026-10-26:** The absence of a required
> company-operated server is a firm constraint. Whether the world uses
> GGRS P2P, a temporary peer authority, or a hybrid remains open.
> **Do not treat a candidate protocol as selected, or replace working GGRS
> based only on this research document.**

This research complements the existing [GGRS implementation plan](netcode.md),
[multiplayer and multiview](multiplayer-and-multiview.md),
[local co-op](local-coop-integration.md), and [ADR 0027](../../adr/0027-ggrs-is-the-sole-rollback-authority.md).
The current GGRS contract remains authoritative **when that backend is installed**;
[ADR 0036](../../adr/0036-synchronization-is-an-optional-game-capability.md) explicitly reopens
whether it should be present for a particular game at all. The options below
are comparison candidates, not approved rollout work.

## Product shape being investigated

The most useful model for ordinary Ambition play may be **a single-player,
Skyrim-like world that friends can join**, rather than a fighting-game match or
an MMO:

- A player opens an existing save and plays normally, whether anyone joins or not.
- The save has a principal owner; that owner is the natural initial world host.
- Roughly **1–4 participants**, potentially including two controllers on one
  machine and remote friends, share one persistent world.
- Different players may explore, fight, and pass through doors in **different
  rooms**, without pausing unrelated players' simulation or rendering.
- The session exists when someone is actually hosting it; **no dedicated server
  or always-on hosting service is desired** for the initial product.
- Local presentation remains separate: shared screen, BG3-style adaptive split,
  or always-split views are all presentation policies, not networking modes.
- A different game made on the Ambition engine (notably its Smash-like mode)
  could prefer **input-synchronized P2P rollback** for a small bounded arena.

**Open ambiguity:** "the game runs while at least one player with the save is
playing" might mean (a) one named save owner hosts and everyone else visits,
or (b) transferable hosting among holders of an authorized copy of the save.
Those are different persistence and conflict-resolution requirements. The
first is a simpler initial prototype; neither is approved product policy yet.

## Separate the concepts before selecting a protocol

Do not conflate these responsibilities:

| Concept | Question it answers | Tentative examples |
| --- | --- | --- |
| Save owner | Who owns and can persist world progression? | Principal player, transferable authorized owner later |
| Live session host | Who admits peers and coordinates session creation? | A player's own game process |
| Simulation authority | Whose state is canonical when outcomes disagree? | All lockstep peers, or one player-hosted authority |
| Input authority | Which participant may issue commands for a body? | Participant/seat assignment, not a camera or connection |
| Synchronization method | How are remote inputs and state corrected? | GGRS P2P prediction; host snapshots + guest reconciliation |
| World residency | Which rooms exist, run, and replicate? | Host's live-room set; guest interest by participant/view |
| Presentation | What does a machine display? | LocalView(s), UI, split/merge, interpolation |
| Network topology | How are packets routed? | Direct peers, listen server, relay where needed |

A participant is not a transport connection. One client machine may control
multiple local participants; a spectator may control none. A host may also be
an ordinary player. A relay, matchmaking service, or "host" label does **not**
by itself imply that the host owns the simulation.

## What Ambition already has (source-review baseline)

The 2026-10-24 source archive at `057721102dc3` documents:

- Deterministic fixed-tick simulation and GGRS / `bevy_ggrs` save, restore,
  replay, and checksumming, owned by the active gameplay session. See
  [ADR 0027](../../adr/0027-ggrs-is-the-sole-rollback-authority.md) and
  [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md).
- A `SyncTestSession` and an in-memory GGRS P2P harness: `start_peer_session`,
  `PeerTransport`, `loopback_transports`, and
  `game/ambition_app/tests/two_peers.rs`. This is **not** yet a finished
  player-facing multi-process online game.
- Participant/controlled-body separation, local seats, `LocalView` and
  multiview, and **several live rooms**. See
  [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md) and
  [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md).
- Exact prepared-content identity, rollback schema and input-identity work;
  see [`netcode.md`](netcode.md) and
  [ADR 0026](../../adr/0026-immutable-prepared-content-and-exact-session-identity.md).
- A **safe but expensive** current peer room-commit mechanism: the replicated
  simulation freezes through a coordinated commit / new GGRS session
  generation. `netcode.md` reports **23–43 held updates** at simulated
  link latency of three updates. This measurement is not a generalized network
  benchmark, but the unrelated-room stall is a known design problem.

**Important distinction:** All-peer deterministic simulation and GGRS P2P
networking are valuable existing investments. Neither guarantees efficient
four-room play on weaker clients, late joining to a running saved world, or
host-authoritative state replication. Conversely, the existence of a save owner
does **not** prevent GGRS P2P: persistence and simulation authority can differ.

## Candidate synchronization models

### A. Host-coordinated GGRS P2P — retain the current road

The save owner coordinates admission and persistence, but each participating
machine simulates the same authoritative gameplay state by exchanging inputs.
GGRS handles prediction, rollback scheduling, save/load requests, and desync
checks. "Host" in the menu is **not** an authoritative simulation server.

**Reasons to investigate:** strongest reuse of existing rollback investment;
good candidate for small, bounded combat, the Smash customer, and perhaps
ordinary co-op if measured whole-world costs are acceptable.

**Costs/questions:** every peer may simulate every active room, even when
viewing only one; rollback repeats that work. Whole-session lifecycle barriers,
late join, disconnection, save transfer, and peer agreement on prepared content
need explicit protocols. Low-powered clients can become the bottleneck.

**Do not assume** that GGRS `SpectatorSession` is a drop-in state-replicated
playable client: it receives confirmed inputs, not a general authoritative
world-delta feed.

### B. Player-hosted authoritative world — listen server, no dedicated host

The save owner's normal game contains the canonical world simulation. Remote
guests send participant-scoped inputs/actions and receive authoritative world
observations. The guests render what they need, interpolate remote bodies,
and optionally predict nearby locally controlled gameplay. Disagreements are
corrected by authoritative snapshot + restore/replay (client-side prediction
and reconciliation). The host can play simultaneously; this need not require
launching a second process or paying for a dedicated server.

**Reasons to investigate:** the host alone needs to simulate every active
room. Guest work can be bounded to their visible/predicted area, which may
be important for multi-room exploration and Android performance. Save ownership
and late join have a natural central coordinator.

**Costs/questions:** replicated state schema, entity identity, spawn/despawn,
room interest, baseline/delta transfer, acknowledgment, prediction scope,
reconciliation, correction smoothing, host-latency advantage, and persistence
all require real engineering. **Stock GGRS P2P does not implement this
server-authoritative replication protocol.** Its deterministic simulation and
rollback lessons can be reused, but the GGRS session itself cannot simply be
relabeled a server.

Potential design inspiration (not a prescription): Unity Netcode for Entities
separates predicted, interpolated, and irrelevant replicated entities; Godot
provides authority, spawning, synchronization, and visibility primitives.

### C. One gameplay engine, two session backends — possible long-term hybrid

Possible session configurations:

| Session profile | Canonical gameplay result | Network synchronization | Candidate use |
| --- | --- | --- | --- |
| `Local` | One local world | None needed for remote peers | Solo and couch co-op |
| `PeerRollback` | Peers agree by deterministic input simulation | GGRS | Smash-like matches, perhaps compact co-op |
| `HostedWorld` | Player-hosted world | Input upstream, snapshots downstream, selective client prediction | Persistent, independent-room adventure |

Names above are **conceptual only**. No enum, interface, or public API is
approved. A backend is chosen for a session, not based on whether two players
happen to walk into the same room. Gameplay, movement, physics, abilities,
combat, quest rules, and content definitions must remain shared.

**Promising, but not automatically cheaper:** two backends mean two protocol
implementations, integration test matrices, failure-handling models, and
long-term maintenance costs. Only establish a shared abstraction when two
working vertical slices reveal the actual common contract. Do not fabricate
an overly broad universal rollback interface in advance.

### D. Mixed synchronization within one running world — speculative only

A hosted adventure might someday hand a bounded arena encounter to a
peer-rollback subsystem and commit its confirmed result back to the canonical
host world. This is **not** the recommended first hybrid implementation and
has no agreed design. Simultaneous encounter authority, entity membership,
world-event ordering, people entering/leaving mid-fight, rollback horizons,
and atomic result commits make this a separate research problem. Avoid
implicitly switching authority at room boundaries.

## Shared architecture worth preserving whatever wins

1. **One gameplay semantics.** Engine systems process the same typed
   participant commands and produce the same game rules, independent of
   transport. No "network player" physics or dialogue branch.
2. **Explicit scopes.** Session identity, rollback timeline, save ownership,
   entity identity, room residency, and presentation views must not be
   interchangeable. Never let a retired session's authority govern another.
3. **Simulation/presentation separation.** Local camera, dialogue panel,
   pause UI, audio, and loading cover are not canonical world state. The
   recent local-co-op defects are regression cases, not excuses for duplicated
   per-network-mode logic.
4. **An authority at a time.** GGRS currently owns rollback in a GGRS
   session under ADR 0027. Do **not** install competing rollback history or
   reconciliation drivers in that session. If hosted prediction is developed,
   its driver belongs to a separately selected session model; an ADR change
   would be needed before it is made production policy.
5. **World history is not frame history.** The host's durable checkpoint/save
   must not be a `bevy_ggrs` snapshot. The host's persistent changes and
   confirmed combat/quest consequences have explicit persistence semantics.
6. **Room activity is independent of visibility.** One player's door, blink,
   camera or renderer must not stall someone in an unrelated live room. Keep
   the existing safe P2P room barrier until a deterministic replacement is
   proven; do not remove it just to improve responsiveness.
7. **Mixed locality from the start.** A connection may own multiple seats;
   local and remote participants must use the same body-assignment protocol.
8. **Security and trust remain explicit.** A small friendly session may
   tolerate host advantage and weaker anti-cheat protections. It still needs
   input validation and save/world-state integrity; a guest must not be able
   to write arbitrary host state.

## Questions not answered yet

| Decision to investigate | Why it matters |
| --- | --- |
| Who may host an existing save? | Named owner vs authorized rehosting; divergent save copies |
| What does progress belong to? | Shared quests, personal inventory/XP, visitor persistence, save conflicts |
| Is late join required during an ongoing fight? | Baseline transfer, current-room state, stable identities, rollback frontier |
| What happens if the host leaves? | End session, migrate authority, or offer optional save handoff |
| How often are players in different heavy rooms? | Determines value of interest-based replication vs full rollback |
| What is the acceptable latency and correction feel? | Compare GGRS prediction to hosted prediction/reconciliation |
| What does friendly fire mean across modes? | Deterministic game rules must be identical; no local-only damage decision |
| Who can pause and who can interact in dialogue? | Menu/view ownership and simulation time policy remain distinct |
| How are prepared assets and procedural rooms delivered? | Deterministic generation vs snapshot/content transfer, compatibility checks |
| How much networking infrastructure is acceptable? | Direct connections, NAT traversal, optional relay/signaling, no dedicated game server |
| Does Smash need a distinct session profile? | Product demand and maintenance burden of two supported backends |
| Which consoles/devices are minimum participants? | Worst-client CPU and rollback memory may dominate architecture |

## Research and experiment plan (not yet authorized execution work)

**R0 — Establish a falsifiable baseline.** Inventory the exact current GGRS
integration, rollback registrations, room-commit protocol, and available
metrics. Define a representative multiplayer play trace: two-player same-room
combat; split rooms; simultaneous transitions; death/respawn; dialogue; save,
reconnect, and late join. Establish latency/jitter/loss profiles before
comparing modes.

**R1 — Measure replicated P2P cost, not only packet bandwidth.** Exercise
1, 2, and 4 participants with 1–4 busy live rooms using the real deterministic
simulation and loopback delay/fault injection. Measure per-tick simulation
CPU, worst-frame rollback/resimulation, saved-frame bytes, memory, bandwidth,
waiting/stall time, and state divergence. Repeat on weaker hardware/Android
where practical. Separate the known *room-commit barrier* cost from the
ordinary *multi-room simulation* cost. Do not infer production performance
from an in-memory test alone.

**R2 — Playable multi-process GGRS vertical slice.** Assess the minimum
transport/handshake to let two actual graphical processes play one match/world
through existing `PeerTransport` and session identities. A save owner can
coordinate without changing simulation authority. Preserve the current safe
room-commit protocol while probing its limitations. This slice is valuable
even if hosted adventure later wins, especially for Smash.

**R3 — Small host-authoritative feasibility spike, isolated from GGRS.** One
host game and one guest on localhost, initially in one bounded room: canonical
server tick, guest input sequence, host snapshot, guest interpolation;
then local movement prediction, authoritative correction, and replay of
unacknowledged inputs. Measure correction smoothness, CPU, and bandwidth.
Explicitly list required serialized gameplay fields and stable entity IDs.
Do **not** rewrite the entire game or add parallel authority to a GGRS world.

**R4 — Multi-room and lifecycle comparisons.** For both candidates, test two
participants in different rooms, independent ordinary door transitions, one
player loading while the other fights, disconnect/reconnect, join into a
nonempty world, incompatible prepared content, and a failed room preparation.
Verify that no remote peer can block the world indefinitely. Include save
recovery and persistence validation.

**R5 — Compare measured results and choose intentionally.** Present a
side-by-side matrix with CPU (host/guest), peak memory, bandwidth,
input-to-visible latency, correction artifacts, room-crossing stalls,
late-join complexity, implementation effort, and maintenance burden.
Recommend `PeerRollback`, `HostedWorld`, or both only after this evidence,
with a scoped ADR and an updated execution plan. Numeric acceptance
thresholds are still **TBD**, rather than invented here.

### Minimum comparison scenarios

| Scenario | P2P question | Hosted-world question |
| --- | --- | --- |
| Two players together fighting | Is rollback smooth and bounded? | Does local prediction survive shared collisions? |
| Four players in four rooms | Can all peers afford all room ticks? | Can the host afford them and guests avoid them? |
| One player crosses a door | Can the others continue through an agreed timeline? | Can room loading/replication stay participant-local? |
| New participant joins an active save | Can peers establish an exact synchronized baseline? | Can host stream a consistent relevant baseline? |
| Host quits/crashes | Who writes the last confirmed save? | Can clients recover without a split-brain world? |
| Weak client / network jitter | How much prediction and catch-up accumulates? | How much correction/interpolation becomes visible? |
| Two players on one device + one remote | Do local seats map to one peer? | Can one connection submit multiple participant inputs? |

## Provisional order of investigation, not a roadmap decision

The cheapest evidence may come from **keeping the existing GGRS harness and
measuring it first**, then building one small, deliberately isolated
host-authoritative spike to compare. That is an **experiment order**, not a
promise to ship either backend or a decision to support both. We can abandon
an expensive abstraction before it becomes permanent architecture.

Do not implement automatic per-room P2P/host switching, global room freezes
as the intended product behavior, permanent dedicated-server requirements,
or a new rollback driver inside the existing GGRS session in response to this
note. Do not create queue tickets for all R0–R5 until the maintainer selects
which investigation to fund.

## Sources and follow-up reading

**Ambition (current authority for what exists):**

- [`netcode.md`](netcode.md) — current peer protocol, lifecycle, measured freezes.
- [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md) — participants,
  bodies, rooms, views, and locality.
- [`local-coop-integration.md`](local-coop-integration.md) — real two-player
  UI/room/rendering failures to use as acceptance cases.
- [`../game/multiplayer.md`](../game/multiplayer.md) — product policy and
  undecided questions.
- [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md),
  [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md).
- [ADR 0027](../../adr/0027-ggrs-is-the-sole-rollback-authority.md) —
  accepted *current* GGRS rollback ownership, not approval of a future
  authoritative-replication backend.
- `crates/ambition_platformer2d_rollback_ggrs/src/peer.rs` and
  `game/ambition_app/tests/two_peers.rs` — existing harness and protocol tests.

**External architecture references (examples, not proposed dependencies):**

- [GGRS crate documentation](https://docs.rs/ggrs/latest/ggrs/) — P2P,
  spectator and sync-test sessions, rollback requests and prediction.
- [Unity Netcode for Entities prediction](https://docs.unity.cn/Packages/com.unity.netcode%401.0/manual/prediction.html)
  — authoritative snapshots with client rollback/replay; compare scope and
  costs, not API compatibility.
- [Unity netcode package overview](https://docs.unity.com/en-us/multiplayer/netcode/netcode)
  — GameObjects vs Entities, different guarantees.
- [Godot MultiplayerSynchronizer](https://docs.godotengine.org/en/stable/classes/class_multiplayersynchronizer.html)
  and [MultiplayerSpawner](https://docs.godotengine.org/en/stable/classes/class_multiplayerspawner.html)
  — examples of authority/replication/visibility seams.

**Review rule:** Until new measurements and a maintainer ruling exist, state
this direction as *in research*. Future agents should bring evidence and
tradeoffs, **not** convert this note into an implementation order or declare
that GGRS must be abandoned.
