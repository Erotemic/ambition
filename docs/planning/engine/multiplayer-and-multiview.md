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

**Re-measured 2026-08-14 against HEAD, and the shape of the slice is decided by
one fact: the resolve is ALREADY a single function over an input bundle.**
`resolve_follow_camera_snapshot(CameraSnapshotResolveInput, &mut CameraEaseState)`
is pure apart from the ease state it threads. So M1 is not "rewrite the camera" —
it is **give the input bundle an owner**, and the owner is a view entity.

| what moves onto the view | who writes it | who reads it |
| --- | --- | --- |
| `CameraViewport` | host `publish_camera_viewport` | the resolve |
| `CameraScreenFraming` | host `publish_camera_screen_framing` | the resolve |
| `CameraPresentationInputs` | render `publish_portal_camera_clamp` | the resolve |
| `CameraEaseState` | the resolve (sole writer) | the resolve; host portal reset |
| `ResolvedCameraSnapshot` | the resolve | render `camera_follow` |
| `CameraReferenceFrame` | nobody yet — the SELECTION D118 C2 left open | the resolve |

⭐ **that last row is why this slice is worth taking now.** D118 C1–C4 landed the
whole mechanism; the only thing keeping subject-relative view unselectable is
that a policy belonging to a view has nowhere to live. A component on the view
entity is that place, and the one-view case is the one-entry case.

⛔ **stays global, with the reason stated rather than forgotten:**
`ResolvedGameplayPresentation` (a DISPLAY resolve — one physical screen, which
becomes per-view only when layout splits), `CameraViewState` (a render
diagnostic read by the debug overlay and nameplates), `CameraShakeState` (whose
per-view semantics are C4's open shake question, not this slice's).

⚠ **spawn the view at plugin BUILD time, not from a startup system.** Every
reader would otherwise need a `single()` + `else { return }`, which is the exact
shape that has produced four production defects in this repo — a system that
silently does nothing is indistinguishable from one that ran.

⇒ deletion payoff: five process-global resources stop existing. Nothing new
"indexes" anything — the index IS the entity, and M2 adds a second one.

**DONE 2026-08-14.** `local_view.rs` holds `LocalView` + `LocalViewId`; the six
facts above are components on it; `CameraObservationPlugin` spawns it at plugin
BUILD time and the resolve iterates views. Every one of the six resources is
gone — including `CameraEaseState`, which lost its `Resource` derive entirely, so
"the camera's ease state" is no longer a question that can be asked without
naming a view.

⭐ **the reference-frame policy has a home, which is what D118 C2 was waiting
for.** Selecting subject-relative presentation is now writing a component on a
view. It still defaults to `WorldFixed` everywhere, so nothing moved — but the
selection is a product decision now rather than a plumbing gap.

⚠ **`the_only_view` exists and is named to be uncomfortable.** Fixtures and
diagnostics that genuinely assume one view call it and PANIC if that stops being
true; no production system does. The alternative — `single()` inside each reader
— is the shape that has silently produced four defects here.

⛔ **and the view carries NO `Name`, which cost a red suite to learn.** `Name` is
rollback-registered (`entity.name`), and the coverage contract derives its swept
population from *"an entity carrying even one type the rollback knows about is an
entity the rollback participates in"* — so a debug label enlisted the whole view
in the sim sweep. `CameraEaseState` was immediately reported as an unrewound
desync risk, and `rollback_exit_oracle` went red because the entity was being
saved and restored across a forced rollback. `LocalViewId` is the identity; the
label is not worth the enlistment.

⇒ **what M2 needs next**, in the order the code will demand it: a link from a
CAMERA entity to the view it presents (`camera_follow` uses `Single` today and
says so), per-view gameplay rectangles from a split layout, and then
`ControlledSubject`.

### M2a — one body-generic presented pose (LANDED 2026-08-14)

Taken first because Jon's F1 overlay handed it a live symptom, and because every
later per-view slice draws bodies that are not the player's.

**The measurement.** `PresentedPose` followed `BodyPoseView`, and
`rebuild_body_pose_views` is filtered `With<PlayerVisual>`. So no boss and no
actor had a presented pose at all — and three separate consumers read that
absence as *"this body has no interpolation"* rather than *"this body is not in
the population"*:

| consumer | what a non-player body got |
|---|---|
| combat overlay (F1) | strike drawn on the TICK clock beside a player's on the frame clock |
| unauthored attack stand-in | skipped entirely, with a warn naming a population that could never be there |
| slash visual | miss arm — the drawn blade never followed a boss |

Two of the three carried a comment asserting the population was identical
because `rebuild_body_pose_views` "requires only `BodyKinematics`" — true of its
OPTIONAL facts, false of its filter. ⛔ one wrong claim, copied twice, hid three
defects.

**The change.** `advance_presented_body_poses` reads `BodyKinematics` — which is
where `BodyPoseView` copied `pos`/`vel` from verbatim, so the player population's
numbers are unchanged to the bit. Every body now publishes a presented pose, and
the seam every body-attached consumer reads is one method:

```text
PresentedPose::delta()  =  presented − authoritative
```

Applied to the whole rigid group in the same frame — collision envelope,
hurtboxes, body-anchored strikes, and the tuning readout that hangs off the box.
⛔ translating a subset does not fix a shudder, it relocates it, which is what
the strikes-only version did.

**Deletion payoff.** `CombatStrikeGeometryView::owner_anchor` is gone. It existed
so an observer could re-place a strike by `presented − owner_anchor`: one lookup
per STRIKE, with no equivalent for a body's own boxes — the shape that made the
half-fix possible. It was also `ZERO` for a world-anchored strike, which is not
an anchor. ⭐ and the delta is the more honest translation: `presented −
owner_anchor` silently re-anchors a strike whose volume was resolved against a
position the body has since left, absorbing a real disagreement a diagnostic
exists to show.

**⭐ and widening the population immediately exposed a fourth defect the narrow
one had been hiding.** `resolve_camera_observation` ended with

```rust
if let Ok(presented) = presented.get(followed) { player_body.pos = presented.presented(); }
```

which is correct only when the pose being framed IS the followed body's own. For
a FRAMED CAST it is not: `pos` is the pair's CENTRE and the presented sample
comes from one anchor seat, so assigning threw the centre away and pointed the
camera at seat 0. Unreachable while fighters published no presented pose;
instantly live once they did — `two_cpus_can_fight_each_other` went red on the
same commit that widened the population, which is the test earning its keep.

The repair is the same rule again: `pos += presented.delta()`. Byte-identical
for the home-avatar and controlled-subject paths (`pos` IS that body's
authoritative pose there), and correct for the cast. A framing centre is rigidly
attached to its cast exactly as a hitbox is to its owner.

**Still open here.** `PresentedFeaturePoses` remains a second, id-keyed history
for the same actor bodies, because feature VISUALS join to the sim by string id.
That is a fork worth closing once a feature visual can name its sim entity — not
before, and not by giving the read model a second identity type.

## `ControlledSubject`: measured 2026-08-14, and it is NOT a view question

50 non-test reader sites, 32 of them in the actor monolith. The plan expected to
classify all 50 by *which view's subject?* — **the measurement says that is the
wrong question twice over.**

⭐⭐ **it is already a DERIVATION, not an authority.** `resolve_controlled_subject`
scans for the entity carrying `Brain::Player(PlayerSlot::PRIMARY)` and writes it,
with a `debug_assert!(count <= 1)` that states the single-participant assumption
out loud. So the assumption is not spread across 50 sites — it is ONE projection
with a hardcoded slot, and the 50 readers consume its result.

⭐ **and the readers mean PARTICIPANT, not VIEW.** Sampled in their own words:

| site | what its comment says it means |
| --- | --- |
| `abilities/traversal/blink` | *"the vacated home avatar is not the subject, so it never blinks"* |
| `features/ecs/interact` | *"the body you are standing next to, not whatever the vacated home avatar is next to"* |
| `affordances/intent` | *"the controlled body's slot input and its facing"* |
| `session/teardown` | *"the driven-body handle… self-heals from the `Brain::Player` query"* |

A second local VIEW does not multiply blinks, interacts or intents. A second
PARTICIPANT does — and that axis already exists as `PlayerSlot` /
`SlotControls` / `Brain::Player(slot)`, which is what the derivation reads.

⇒ **the slice is: make the projection per-SLOT.** Same scan, keyed by
`PlayerSlot` instead of hardcoding `PRIMARY`; the `count <= 1` invariant becomes
per slot, which is exactly what it always meant. Readers then say WHICH
participant they mean, and the ones that cannot are the interesting ones.

⚠ **a view's subject is a genuinely different, much smaller concept** — which
body a view FRAMES, possibly a spectated body no participant drives. The camera
resolve already threads it as the followed entity plus `subject_down`.

⛔⛔ **and the first version of this census claimed three of the four `sim_view`
readers were that concept wearing this name. READING THEM REFUTED IT.** In their
own words: the HUD meters follow *"THAT body's meters, never the vacated home
avatar's"*; the held-item view, the blink reticle (*"the body you are driving"*)
and the control prompt (*"the body you are DRIVING — the same relativity rule the
camera and input already obey"*) all mean the participant's driven body. Every
one of the 50 means PARTICIPANT. `camera_snapshot` is the sole site that reads it
as a view question, and even there it is the default POLICY — *this view follows
the local participant's body* — not a confusion to unpick.

⇒ that makes the concept UNIFORM and the slice cleaner than the mixed picture
suggested: one derivation, one meaning, one axis (`PlayerSlot`), and a view's
subject is a policy layered on top rather than a tangle inside.

⛔ still do not do both at once. Per-slot participants is a control-authority
change; per-view subjects is a presentation change. They meet only at that
default, and conflating them is how a process-global gets replaced by a
process-global with a longer name.

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
