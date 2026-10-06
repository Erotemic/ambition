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

- `ParticipantId` and participant/seat input state are explicit.
- `DrivingParticipant` represents body control. Humans are not a special brain
  variant.
- `LocalView` / `LocalViewId` identify local presentation views. The one-view
  path is `LocalViewId::FIRST`, not a separate singleton architecture.
- Camera reference frame, ease and resolved snapshot are per view.
  `PresentedViewState` exposes view-scoped presentation facts.
- Host tests compose two views through the one view-spawn seam
  (`spawn_local_view`).
- `CameraReferenceFrame` is a view component. The world-fixed /
  subject-relative option is shipped.
- **Several live rooms.** Two seats' driven bodies can be in two live rooms at
  once. Each seat's driven body takes the doors of its own room. While the
  seats are in two or more live rooms, each seat gets a view
  (`split_views_by_live_room`), and the views close when the seats meet. Each
  camera draws only its view's live room. See "The view half" in
  [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md).
  The split changes only what it owns: the views it opened and the placements
  it wrote (`PlacedByLiveRoomSplit` holds the value it wrote). A placement a
  composition wrote is not written over, not marked and not removed at the
  merge. A view shows a seat when the body it frames drives that seat, by
  `ViewSubject` or by `ViewParticipant`, so a view that names Bob's body is
  Bob's view and the split opens no second one.
- A camera count is not a view count. `[census] views` and `[census] camera`
  (`crates/ambition_render/src/runtime_census.rs`) print both; the census
  writes to stderr. Exactly one camera names each view.

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

Built for the simulation and the view (OW1, V1-V5 in
[`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)).
Open: Ambition has no production join road for a second seat, and a seat-driven
body's death takes the enemy road (Q153). A participant's death is local to it
and its room (Q151 ruling).

### M5 — view-scoped HUD/prompt/presentation ownership

`ControlPrompt` and the banner are one per session and follow the primary seat.
The Q150 ruling (2026-10-03) makes HUD state per participant, also when views
merge onto one screen, so no "primary-player HUD" arbitration exists. Local
music is chosen by authored priority across the participants, the primary
participant breaking a tie (built). The built-in vitals HUD is per view: each
`LocalView` carries `ViewHudFacts`, the meters of the body it follows, and has
its own HUD in its own column. On a merged screen, the first view that names
nothing carries `SharedViewHudFacts`: each other seat whose body is in the
controlled body's live room and that no view follows, with its own HUD
stacked in that view. Open: the declared readouts (`HudReadouts`) are one per
session; stacked HUDs carry no participant name; and every seat counts as
local, so an online peer would show a remote seat's HUD (A4).

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
- a global "main camera" fallback for state that has become genuinely per-view.

## Exit

The program can leave active architecture planning when local and remote
participants, body assignment, room residency, and view layout can vary
independently through supported engine seams, with Ambition's required shared /
split / different-room cases covered by representative hosts.

## Separate participant multiplicity from world multiplicity

Seat/control arbitration, multiple views, multiple live world instances and
network transport are independent axes. Two players in one room do not prove two rooms can coexist;
two cameras do not require two authoritative simulations.

A4 preserves one accepted driving relation with all current mount/possession
constraints. Two-instance tests use repeated authored IDs across instances, not
only different room IDs, and tear one instance down while the other stays live. Checkpoint restore currently follows its established primary-avatar policy;
A1's ownership move does not decide co-op save ownership. Gate policy is
ruled (Q54, 2026-10-01): a body/capability gate is evaluated per actor.
