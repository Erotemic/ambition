# Controlled-character actor kernel

**State:** NARROW / OPEN — the first decision-authority convergence is complete.
This is no longer the Engine 1.0 P0 gate.

## Goal

A human-controlled body, AI-controlled body, possessed body and future remote
participant should use the same actor-local simulation path. Distinctions such
as home avatar, driving participant, input seat, camera/view subject and
presentation focus remain legitimate when they have different semantics; they
must not become alternate movement/combat authorities.

## Landed convergence

The major player-centric decision split that motivated this program has largely
landed:

- actor observation/decision phases are explicit enough to separate observation,
  targeting and pre-decision maintenance from later mutation;
- the old combat-slot arbitration anchored on `PrimaryPlayer` was deleted after
  proving no production reader consumed it;
- controlled and autonomous bodies consume increasingly shared actor/body
  decision contracts;
- several seat-zero/primary-player special roads have been removed or reduced to
  legitimate presentation/home-avatar fallback semantics;
- `CollisionWorld` is the shared collision observation authority instead of each
  large system independently recomposing room/platform/overlay geometry.

Do not reopen deleted player-vs-actor mirrors merely because old campaign history
mentions them.

## Residual kernel target

The residual actor kernel should own only behavior that genuinely needs tight
body-local integration, approximately:

```text
body state and actor-local lifecycle
    + accepted intent / control projection
    + movement/contact integration
    + core reaction/action application seams
    + narrow observation/decision interfaces
```

It should not be the composition root for persistence, rooms/session lifecycle,
boss/encounter orchestration, item/projectile domains, named content,
presentation/UI/audio, developer tools or host/platform policy.

## Current work

### K1 — finish the remaining body-integration fork only if it is still real

The previous campaign narrowed the structural fork to the remaining home-body /
generic-body integration difference. Re-measure HEAD before changing it. Merge
paths only when they express the same body semantic; do not erase legitimate
home-avatar presentation or shell policy for naming symmetry.

### K2 — per-driven-body item/projectile control

Review evidence still identifies two related control-authority leaks:

- held ranged shots attributed through slot-zero/primary-player state rather than
  the body/participant that actually fired;
- custom held-item abilities dispatched through one `ControlledSubject` while
  generic pickup/throw/fire already iterate multiple driven bodies.

The target is one per-driven-body ability/projectile request path. Prefer folding
held ranged fire into the shared projectile request/ownership seam over growing a
parallel held-shot simulation.

This is a real multi-seat correctness/customer issue and is stronger residual
pressure than generic cleanup of every `PrimaryPlayer` occurrence.

### K3 — preserve separate identities where they mean different things

Do not collapse these merely to reduce type count:

- gameplay participant / network peer;
- input seat/device assignment;
- currently driven body;
- home/avatar body;
- camera/view subject;
- presentation focus.

A bug is one concept accidentally used as another concept's authority, not the
existence of several concepts.

### K4 — let actor-monolith carves follow ownership

The residual-kernel definition should guide
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md). A carve is
valuable when it removes an unrelated authority/dependency from the actor kernel,
not when it merely makes this file's implementation smaller.

## Acceptance pressure

- zero-human-controlled-body headless simulation;
- two or more independently driven bodies with correct item/projectile ownership;
- possession/body switching without a second movement/combat path;
- local and future remote participants controlling ordinary bodies;
- persistent NPCs using the same body/control/navigation seams;
- home-avatar presentation remaining correct without becoming simulation
  authority for every body.

## Open design questions — deliberately unresolved

- What is the smallest stable observation/decision input without a giant context
  bag?
- Which remaining home-body integration differences are genuine semantics versus
  historical duplication?
- Where should provider-specific body abilities attach so multiple driven bodies
  can consume them without enlarging the generic actor action taxonomy?
