# Multiplayer, multi-view presentation and world residency

**State:** OPEN successor program. Ambition is the primary customer.

## Goal

Support multiplayer without conflating four independent questions:

1. **transport/locality** — is a participant local, remote, replayed or AI?
2. **control assignment** — which body does that participant currently drive?
3. **world residency** — which room/partition contains that body, and which
   partitions must be simulated/resident?
4. **presentation layout** — which local views render which subjects, and are
   those views shared, split or dynamically regrouped?

Keeping these axes separate allows one architecture to support local couch
co-op, online co-op, mixed local+remote parties, possession/body swapping,
spectating, replay, shared-screen arenas and participants exploring different
rooms.

## Ambition product requirement

Ambition should eventually support multiplayer in the main game, not merely in
an acceptance demo.

Valid modes include:

- two or more local participants sharing one screen while close enough;
- fixed split-screen;
- **adaptive shared/split presentation**: share a view while subjects can be
  framed coherently, split when separation or room residency demands it, and
  merge again with hysteresis when appropriate;
- local participants in different rooms, with independent views;
- online participants in the same or different rooms;
- mixed local + remote participants in one session.

A game mode may intentionally require participants to stay together. That is a
**ruleset constraint**, not an engine limitation.

See [`../game/multiplayer.md`](../game/multiplayer.md) for Ambition-specific
product sequencing.

## Current state

Input is already moving toward participant-aware composition:

- `ParticipantId` is explicit;
- local seats/input participants exist;
- multiple human seats can be created for a roster;
- action contexts are participant-owned.

Presentation remains predominantly singular:

- `ControlledSubject` is one optional entity;
- `FramedCast` is one vector;
- `CameraViewport`, `CameraScreenFraming` and `ResolvedCameraSnapshot` are
  singleton observer facts;
- the platformer renderer spawns one `MainCamera` and several systems explicitly
  assume that one main gameplay camera exists.

TwinTrack and other presentation features already prove Bevy can host additional
viewport cameras. What is missing is an engine-level view model rather than
special-purpose extra cameras.

## Target model

### Participants and bodies

A participant is an input/control authority, not a body and not a camera.
Control assignment is an explicit relationship:

```text
ParticipantId -> ControlAssignment -> Body Entity
```

AI can occupy the same downstream intent seam without pretending to be a local
input device.

### Local views

Introduce a view-indexed presentation vocabulary conceptually like:

```text
LocalViewId
LocalView {
    subjects / framing policy
    viewport rect
    presentation profile
    optional owning local participant(s)
}
```

Names and exact storage are implementation questions. The invariant is that one
simulation may publish **N observer views**. The current singleton camera path
becomes the one-view case rather than a separate architecture.

HUD/prompt/dialogue presentation should be able to target a local view or local
participant while truly global UI remains global. A view context should also be
the natural owner/index for camera shake, safe-area/layout facts, view-local
post-processing and—where the game needs it—an audio-listener/mix policy. Do not
let those services silently fall back to a process-global "main camera" once
more than one local view exists.

The mapping is deliberately many-to-many: one view may frame several local
participants, one participant normally has at most one primary local view, and a
remote participant may have no local view on a given client at all. Spectator or
inspection views may exist without owning a participant.

### View reference-frame policy

A local view also owns an observer/reference-frame policy. The existing
world-fixed/external-observer camera remains a valid mode; Ambition additionally
wants a subject-relative mode where the view follows the designated body's
resolved reference frame and gravity changes appear to rotate the world.

This policy is independent of transport, control assignment, residency and view
layout. Two split views may therefore follow two bodies under different gravity
frames. A shared view whose subjects disagree about orientation must use an
explicit shared-view policy rather than silently adopting participant one as the
frame authority.

Use [`camera-reference-frame-policy.md`](camera-reference-frame-policy.md) for the
focused design. Camera/reference-frame selection is presentation; coherent
body-relative movement/aim is a control-policy pairing, not something camera code
mutates.

### View grouping policy

Presentation policy chooses grouping independently of simulation:

- `Shared` — one view frames a group;
- `Split` — one or more independently framed views;
- `Adaptive` — shared while framing remains acceptable, split when a threshold,
  topology/room separation or explicit game rule requires it, merge again using
  hysteresis to prevent oscillation.

Do not encode the policy as "player 1 camera / player 2 camera". Views may frame
multiple subjects and participants may temporarily share a view.

### World/room residency

Different-room multiplayer requires more than multiple cameras. The session must
be capable of keeping/simulating the partitions needed by current controlled
bodies rather than treating one active room as universal authority.

This should evolve toward a residency set:

```text
Session world
  -> resident partitions / active areas
  -> participants/bodies located in partitions
  -> per-partition presentation and streaming needs
```

Room transition remains transactional for each body/party transition. A local
view moving to another room must not globally replace the room underneath a
participant who stayed behind.

Do not jump immediately to an MMO-style world streamer. Start with the minimal
2-room simultaneous-residency case Ambition split-screen needs, but make the
residency representation a set/policy rather than baking "exactly two rooms"
into engine types. Ambition may later need three-or-more local/remote party
members spread across several rooms even if the first acceptance test uses two.

### Networking

Network transport feeds the same participant intent/action contract local input
uses. Presentation layout is local client policy and generally does not belong in
rollback state. Authoritative control assignments, body state, ruleset state and
world residency do.

## TwinTrack acceptance

TwinTrack is a particularly strong multiview customer because its views may
legitimately use different observer/reference-frame presentation.

A TwinTrack split-screen proof should allow two controlled bodies to have:

- independent viewport rectangles;
- independent view subjects/framing;
- independent observer/reference-frame presentation, including different view-frame policies;
- one shared authoritative simulation.

The proof must not clone the world per viewport.

## Smash relationship

Super Smash Siblings may ultimately become a first-class game rather than remain
only a demo. Its normal arena rule can still choose one shared view because all
fighters are intentionally constrained to one arena. It validates N participant
input and body-generic combat, while Ambition/TwinTrack validate the harder
multi-view and multi-room cases.

## Census: what actually assumes one view (2026-08-14)

⛔ **`FeatureViewIndex` is not a view index.** It is a per-FEATURE render
read-model that happens to share the word. Local-view identity does not exist at
HEAD; nothing needs deleting first.

Counting reader/writer parameters, excluding tests:

| One-view state | Sites |
| --- | --- |
| `ResolvedCameraSnapshot`, `CameraViewport`, `CameraExtraClamp`, `CameraEaseState`, `CameraScreenFraming`, `FramedCast`, `PortalCameraContinuityState` | **15 across all seven** |
| `ControlledSubject` | **49** |

⭐ **the presentation state is cheap and `ControlledSubject` is the campaign.**
Seven camera/view resources total fifteen sites — indexing them is a contained
slice. `ControlledSubject` alone is three times that surface, 26 of them inside
the actor monolith, and it is defined as *"which body this local
presentation/control context follows"*. With two local views there are two such
contexts, so every one of those 49 has to answer a question it cannot currently
be asked: **which view's subject?**

⇒ split the first proof accordingly. Index the view-owned presentation state
first — small, self-contained, and it is what the camera reference-frame policy
(D118) is waiting on, since that policy must belong to a view rather than become
a global mode. Then take `ControlledSubject` as its own slice, classifying its
consumers by whether they mean *this view's* subject, *a* controlled body, or the
home avatar; the six-name map in `docs/concepts/one-body-one-path.md` is the
vocabulary for that classification.

## Phases

### M1 — view-index observer facts

Remove singleton assumptions from the camera/read-model seam while preserving
one-view behavior byte-for-byte where practical.

### M2 — two local views, one room

Render two independently framed gameplay views over one simulation. Prove local
HUD ownership and input routing.

### M3 — adaptive share/split

Add a policy that groups nearby compatible subjects into one view and splits on
separation. Specify thresholds/hysteresis as presentation data, not simulation
magic.

### M4 — two resident rooms

Allow two controlled bodies to occupy different rooms simultaneously. Make room
construction/residency and lifecycle semantics explicit enough that one body's
transition does not replace the other's world.

### M5 — network and mixed locality

Feed remote participants through the same control contract. Prove local+remote
composition and client-local view policy.

### M6 — TwinTrack reference-frame proof

Use split views with independent observer frames as an acceptance test for the
view-index architecture.

## Acceptance matrix

| Case | Transport | Room relation | Presentation |
|---|---|---|---|
| Ambition solo | local | one room | one view |
| Ambition couch co-op close | local + local | same room | shared |
| Ambition couch co-op separated | local + local | same room | adaptive split |
| Ambition independent exploration | local + local | different rooms | split |
| Ambition online co-op | local + remote | same or different | client policy |
| Ambition mixed party | 2+ local + remote | same or different | shared/split mix |
| TwinTrack | local/local initially | same or different | independent reference-frame views |
| Smash | local/AI/remote eventually | arena together | usually shared arena view |

The architecture is successful when these are policy/configuration differences,
not separate body/session implementations.
