# ADR 0035: Multiplayer must not depend on a company-operated service

## Status

**Proposed architecture (2026-10-26); binding product constraint recorded.** The maintainer
explicitly requires multiplayer to survive the disappearance of any company-operated servers.
The specific distributed protocol remains undecided.

## Context

A persistent world with independently exploring players cannot be treated as a Smash-sized
rollback match. Equally, multiplayer must not cease to function if a publisher, platform,
discovery provider or relay operator shuts down. This does not mean every participant must
concurrently be the simulation authority: a participant's machine can temporarily host canonical
state without a dedicated or centrally operated service.

## Proposed decision

- Offline single-player, LAN play and direct peer addressing are first-class supported modes; no
  mandatory central account, matchmaking, authentication, save, relay or simulation service sits
  on the critical path.
- A player's machine may serve as the **temporary world host/authority** for an active session.
  A host is not a company-operated server, and the host may also control a local participant.
- Separate **save ownership**, **session admission**, **simulation authority**, **input
  authority**, **region authority**, and **presentation**. Do not infer one from another.
- Shared-world changes must have one unambiguous write authority per committed fact and session
  epoch. A participant may delegate scoped authority, but a network partition must not silently
  create two equally canonical futures.
- Save files and necessary authored content remain portable and usable offline; optional
  discovery and relay providers improve reachability but cannot become exclusive dependencies.
- Host migration, authority transfer between peers, NAT traversal, relay support, reconnect and
  divergent-save reconciliation are **open designs**, not features approved by this ADR.
- The same game mechanics must work offline without networking and in a peer-hosted session
  without a separate server-specific gameplay implementation.

## Alternatives and tradeoffs

A permanently operated authoritative service violates the maintainer's constraint. Pure all-peer
lockstep/rollback is a valid bounded-game option, but does not automatically support independent
persistent rooms or weak devices. A peer-hosted authority avoids a dedicated service but
requires state replication and may end the live session if its host disconnects. Fully
leaderless mutable state and automatic fork merging are not promised: they require explicit
consistency and conflict policies.

Direct connections can fail under some NAT/firewall conditions. LAN and direct addressing must
work where network connectivity allows; optional relays may be necessary to connect some peers
over the public internet.

## Evidence required before acceptance of a topology

- Two processes exchange gameplay inputs and state on a LAN with no external service or network
  account.
- An existing locally owned world save opens offline and remains usable if discovery and relay
  services are absent.
- Two participants explore different rooms without one participant's ordinary transition
  freezing the other.
- Explicitly document what happens when the authority disconnects, when peers partition, and
  when saved histories diverge; do not describe automatic recovery until proven.

See [ADR 0036](0036-synchronization-is-an-optional-game-capability.md), [ADR 0037](0037-causally-isolated-simulation-islands.md), and [network topology research](../planning/engine/network-topology-and-authority-research.md).
