# Controlled-character actor kernel

**State:** NARROW / OPEN. This page defines what is allowed to remain in the
residual actor kernel; it is not a historical campaign log.

Executable SCC work:
[`actor-monolith-work-frontier.md`](actor-monolith-work-frontier.md).

## Kernel contract

A human-driven body, AI-driven body, possessed body and future remote-driven body
must all run through the same body simulation path.

The kernel may own:

```text
body state and actor-local lifecycle
accepted control/intent projection
movement/contact integration
core body reaction/action application
narrow observation/decision interfaces
```

The kernel must not own merely for convenience:

```text
room/session lifecycle
save/persistence policy
independent item/projectile domains
boss/encounter/dialogue orchestration
presentation/UI/audio
host/dev facilities
named game content
```

## Identity rules

Keep distinct concepts distinct:

```text
participant/network peer
input seat/device
currently driven body
home/avatar body
camera/view subject
presentation focus
```

A bug is one concept being used as another concept's authority. Reducing the
number of types is not a goal.

## Control/custody rule

The control/custody prerequisite is settled around explicit claims and effective
projection. Optional capabilities may contribute control claims; they do not get
parallel movement/combat loops.

Do not reintroduce player-only or AI-only simulation branches while carving
modules.

## Package rule for the current SCC

P1-P4 deliberately remove satellite responsibilities before deciding the hard
core. After those packets, the expected SCC is:

```text
abilities, control, features, items, session, world
```

At that point:

- `abilities + control` are allowed to remain together **only** if the edge ledger
  shows they are one actor-local control/action package;
- `world + session` are allowed to remain together **only** if remaining edges
  are lifecycle/world state that truly share one owner;
- `features` is not accepted as a permanent catch-all. Every retained feature
  responsibility must be named in the hard-core ledger;
- residual `items` code must justify why it belongs in the kernel rather than the
  already-carved item/persistence capability packages.

The decision artifact is
[`actor-monolith-hard-core-edge-ledger.md`](actor-monolith-hard-core-edge-ledger.md).

## Invariants that every carve must preserve

1. **One Body, One Path:** player/AI/possession differ in intent source, not body
   physics/combat implementation.
2. **One control answer:** transient control is derived from the canonical claim
   authority rather than independently written by capabilities.
3. **One geometry answer:** combat/projectile/contact systems consume published
   body/target geometry instead of family-specific duplicate envelopes when a
   canonical volume exists.
4. **One lifetime owner:** session/match/attempt/stock/process retraction is part
   of the moved authority's contract.
5. **Rollback follows canonical state:** moving a type cannot drop or duplicate
   its snapshot/registration semantics.
6. **Presentation stays downstream:** camera, sprite, portal, HUD and shell facts
   may observe the kernel; they do not become simulation authority.
7. **Optional capabilities remain optional:** the kernel does not depend upward
   on a gameplay capability solely because one game installs it.

## Required acceptance pressure

The residual package must continue to support:

- zero-human headless simulation;
- two or more independently driven bodies;
- possession/body switching without another body simulator;
- persistent NPCs through the same movement/combat seams;
- item/projectile attribution to the actual driven/firing body;
- home-avatar and camera presentation without making the home body global
  gameplay authority;
- rollback across control/lifetime transitions.

## Open questions that belong to P5, not P1-P4

Do not answer these during the four peel packets:

- Is possession/control vocabulary one package with abilities, or should
  abilities contribute through a lower claim/intent API?
- Which `features <-> world` edges are world contribution data versus residual
  orchestration?
- Should item persistence consume a generic durable-horizon milestone, or live
  in a session-owned persistence adapter?
- Which active-content/session/world binding owns room setup after placement
  specialization moves to construction?

Those questions are resolved only by the post-P4 edge ledger and package map.
