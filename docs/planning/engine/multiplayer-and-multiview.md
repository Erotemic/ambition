# Multiplayer, multiview presentation and world residency

**State:** OPEN successor program. Ambition is the primary product customer;
TwinTrack remains a focused acceptance customer.

## Goal

Keep four independent questions separate:

1. **transport/locality** — local, remote, replayed or AI participant;
2. **control assignment** — which body a participant currently drives;
3. **world residency** — which room/partition contains that body and must be
   resident/simulated;
4. **presentation layout** — which local views render which subjects and whether
   those views are shared, split or regrouped.

A participant is not a body, room, or camera. A local view is not a participant.

## Current architecture

The earlier one-view migration has substantially landed:

- `ParticipantId` and participant/seat input state are explicit;
- `DrivingParticipant` represents body control rather than encoding humans as a
  special brain variant;
- `LocalView` / `LocalViewId` identify local presentation views;
- camera reference-frame, ease and resolved snapshot state are carried per view;
- `PresentedViewState` exposes view-scoped presentation facts;
- host tests compose two local views in one room through the same view-spawn
  seam;
- `CameraReferenceFrame` is a view component, and the user-facing world-fixed /
  subject-relative option is shipped.

The one-view path should continue to be represented as `LocalViewId::FIRST`, not
as a separate singleton architecture.

## Target model

### Participants and bodies

Control assignment is an explicit relationship:

```text
ParticipantId -> control assignment -> body entity
```

AI uses the same downstream actor intent/body seams without pretending to be a
local input device. Possession/body swaps alter assignment, not participant
identity.

### Local views

A local view owns presentation facts such as:

- subjects/framing policy;
- viewport rectangle;
- camera/reference-frame policy;
- camera easing/resolved snapshot;
- presentation profile/safe-area where required;
- optional local participant ownership/association.

The mapping is not one-player-one-camera. A view may frame several participants;
a participant may have no local view on a remote client; inspection/spectator
views may have no controlling participant.

### View grouping

Presentation policy may choose:

- **Shared** — one view frames several subjects;
- **Split** — several independently framed views;
- **Adaptive** — shared while framing/topology permits, split when separation or
  rules require it, merge again with hysteresis.

Adaptive policy is presentation. It must not change simulation authority or
participant assignment merely to make the camera convenient.

### World residency

Different-room multiplayer is a world-residency problem, not merely a camera
problem. The session may need several partitions resident or simulated even if
one client can see only a subset.

Use the distinctions in
[`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md):
existence, residency, simulation and visibility are independent.

### Networking

Transport and rollback input exchange are downstream customers of participant
identity and deterministic simulation. Do not encode local/remote state into
actor identity or view layout.

Real network transport remains customer-gated under
[`netcode.md`](netcode.md).

## Current work

### M3 — adaptive shared/split presentation

Promote when Ambition needs dynamic shared/split behavior rather than merely a
fixed second viewport.

Acceptance should cover:

- stable split/merge thresholds with hysteresis;
- explicit shared-view reference-frame policy when subjects disagree on
  orientation;
- correct per-view camera continuity and layout during regrouping;
- no participant/control reassignment caused by layout changes.

### M4 — several resident rooms

Promote with the persistent-world architecture when participants must occupy
separate rooms concurrently.

Required foundations:

- explicit occurrence/lifetime ownership;
- canonical construction/reconstitution;
- room/residency identity independent of camera visibility;
- deterministic simulation policy for resident/background partitions.

Do not create a universal dormant-world scheduler before a product customer
requires one.

### M5 — view-scoped HUD/prompt/presentation ownership

The current `ControlPrompt` remains one global read model for the primary/shared
screen. That may be correct for one display/touch overlay, but independent split
views may require participant- or view-scoped prompts/HUD facts.

Resolve this with
[`participant-action-system.md`](participant-action-system.md) from product/UI
requirements rather than mechanically pluralizing every resource.

### M6 — network and mixed locality

After local multiview/residency and deterministic state are sound, add a real
transport customer that can combine local and remote participants without
changing the body/view model.

## Camera reference-frame policy

Durable behavior lives in
[`../../systems/camera-reference-frames.md`](../../systems/camera-reference-frames.md).

Each independent view may select its own reference frame. Shared views need an
explicit policy when presented subjects disagree about orientation; participant
zero is not an implicit authority.

## TwinTrack

TwinTrack remains a useful acceptance customer for:

- composing two local views through the ordinary host seams;
- proving that presentation facts are genuinely view-indexed;
- exercising different observer/reference-frame policies without creating a
  second simulation.

Do not let demo-specific camera code become the multiview architecture.

## Acceptance matrix

The architecture should eventually demonstrate:

| Scenario | Control | Residency | Presentation |
|---|---|---|---|
| one local participant | one assignment | one or more rooms | one view |
| two local, same room | two assignments | shared room | shared or split |
| two local, different rooms | two assignments | several resident rooms | split |
| remote participant | remote input authority | required remote room state | optional local view |
| mixed local + remote | independent assignments | required partitions | client-specific views |
| spectator/inspection | no body ownership required | selected state | view without participant |

## Do not pre-generalize

Do not introduce:

- player-number-specific camera types;
- transport state on actor identity;
- one camera per participant as an invariant;
- a global "main camera" fallback for state that has become genuinely per-view;
- several-room residency machinery before occurrence/lifetime/reconstitution
  foundations and a concrete customer require it.

## Exit

The program can leave active architecture planning when local and remote
participants, body assignment, room residency, and view layout can vary
independently through supported engine seams, with Ambition's required shared /
split / different-room cases covered by representative hosts.
