# Multiplayer, multiview presentation and world residency

**2026-10-26 clarification:** `participant`, controlled body, view, room and network peer are
distinct identities. Local split-screen is a presentation composition, not proof of remote
multiplayer. Ambition needs independent-room players and no required company server; Smash needs
an opt-in bounded rollback mode; Mary-O and Sanic may use one level-scoped timeline. Treat
the historic global room-travel rebase as a superseded October constraint:
ordinary peer crossings use the shared input timeline without a global
hold/rebase (2026-11-04). Checkpoint/rebuild barriers and the lack of
independent regional histories remain real constraints. See [peer room commits](peer-room-commit-protocol.md)
and [architecture reassessment](architecture-reassessment-and-composable-multiplayer.md).
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
- Camera reference-frame components, ease and resolved snapshots are per
  view. `PresentedViewState` exposes view-scoped presentation facts.
  The `00bf92c22142` source still resolves default target framing and
  placement history before selecting each view's target; MV1 below closed
  that ownership gap (2026-11-23). Per-view components alone do not establish independent
  resolution.
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
- selected installed renderer and its concrete camera/projection realization;
- presentation profile/safe-area where required;
- optional local participant ownership/association.

The mapping is not one-player-one-camera. A view may frame several participants;
a participant may have no local view on a remote client; inspection/spectator
views may have no controlling participant.

The selected [2D/2.5D work](presentation-order-and-25d-rendering.md) uses two
local views of one simulation as its first composition witness. One view can
use sprites while another uses 3D cards/meshes. Observer/layout policy stays
shared; only the selected adapter writes a view's concrete camera. Do not
let the current XY/Z-rotation follower overwrite a 3D camera. A single mixed
world view needs one depth/compositing owner; independently stacked 2D and
3D passes do not interleave object depths. Portal capture views retain the
existing observer/path scope and readiness contract.

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

### MV1 — resolve the selected view before its framing and continuity

**Done 2026-11-23.** `resolve_camera_observation` builds one
`SelectedFraming` per view before any framing is derived. That framing is
either the session default (home avatar, controlled subject or declared
cast) or the subject the view named. Focus, base size, blink, cast box and
the placement record all come from that selection:

- Losing the default clears only the views that frame it.
- A named subject that is gone withdraws its view.
- An explicit pane keeps the authored base size.
- `last_subject_placement` is kept per view, keyed by the body that view
  framed.

A temporary probe in `resolved_snapshot_lifetime_tests` ran each case on
HEAD and then on the fix. The probe was then removed.

| Case | HEAD | Fix |
| --- | --- | --- |
| No default; spectator of an undriven actor | blank | resolves |
| Spectator's body removed while a cast is valid | fell back to the cast at (800, 1988) | withdraws; the default view continues |
| Cast gap 200 → 2400 | pane view 427×240 → 2524×1420 | 569×320 both times (the default view still expands) |
| Default subject placed | pane read `Placed` | pane `Continuous` |
| Pane's own subject placed | pane read `Continuous` | pane `Placed` |

The section below is the plan as written.

**Selected shared correctness work, 2026-11-22.** Source baseline
`00bf92c2214274fdf2ecaacdc0abfd19be569a50`; planning follows the observation
overlay at `3ac54ecc1ada`. This packet needs neither a new renderer nor the
rest of adaptive M3. The following are source-derived counterexamples;
no new rendered, gameplay or timing witness was run.

#### One ownership gap, several native consequences

[`resolve_camera_observation`](../../../crates/ambition_sim_view/src/camera_snapshot.rs)
first resolves the session's home/controlled body or `FramedCast`, derives
framing and continuity from it, and only then considers per-view subjects.
The explicit override changes focus, orientation and some bounds, but does
not replace every dependency inherited from that first resolve.

| Native composition | Source consequence | Positive control |
| --- | --- | --- |
| `NoInitialBody`, no driven/default body or global cast; an ordinary undriven actor and a view created with `spawn_local_view`, `local_view_facts()` and `ViewSubject(actor)` | The global default resolve clears every view and returns before considering that valid explicit target. | Publishing an unrelated resolvable global cast lets the same spectator resolve. Removing only that cast must not disable it after the repair. |
| An explicit `ViewSubject` whose body disappears, while the default cast remains valid | `declared_subject` retains the named entity; when `view_focus` cannot read its body, the override falls through to the default. The pane can watch the wrong population instead of withdrawing. | Remove only the named body. Its view becomes unavailable; the valid default view continues. |
| A global two-body cast plus an explicit pane watching another body, in a large room without camera zones or overview scaling | Cast easing overwrites `base_view`. The explicit pane drops cast bounds but still receives the cast-derived base size, so moving only the unrelated cast changes its zoom. | Keep the explicit subject, body scale and view configuration fixed; change only the other cast's separation. A view intentionally framing that cast must still expand. |
| A continuous explicit pane while the default subject blinks or is placed elsewhere | `blink_snap` and `subject_placed` are computed once from the default and reused by the pane's snap, policy reset and continuity classification. | The explicit target's own placement must affect its view. The default's unrelated placement must not; ordinary movement and portal-presented translation keep their existing rules. |

These are not the closed October black-pane/sprite-sync defects. They
arise after the ordinary view-spawn and per-view snapshot seams have been
used correctly. A spectator-only early-return exception leaves zoom and
continuity wrong.

#### Bounded correction

1. **Resolve the requested population first.** Use the existing
   `ViewSubject` and `ViewParticipant` declarations and their resolved
   subject. A body-follow stays with that body; a participant-follow tracks
   the accepted driving relation. A selected cast is a framing population,
   not an implicit fallback for every view. The session default remains a
   composition choice for views that request it. An unavailable explicit
   target must not silently become the primary body: retain the difference
   between an omitted declaration and a failed explicit resolution.
2. **Derive framing from that selection.** Build focus, bounds, baseline
   extent and reference anchor for the selected policy before resolving the
   snapshot. The ordinary authored base size may be shared; an eased cast
   extent is derived state of the view framing that cast. Preserve the
   existing default-follow policy that deliberately uses home-avatar zoom
   and blink state while following the possessed body's position. Such a
   dependency belongs to that selected policy, not to an unrelated pane.
   Framing samples the presented pose; placed/travelled detection samples
   the tick pose. A cast aggregate already contains its members' presented
   offsets and must not receive the anchor's offset a second time.
3. **Keep temporal state with its view.** Placement history, cast population
   and settling state must describe the population that view presented.
   Keep room changes, subject changes, blink and portal chart translation
   distinct. Reuse the existing travelled/placed predicate and portal
   presentation facts; do not add another motion-frame or teleport solver.
   Retire it with its view and reset the portions invalidated by the actual
   transition. A changed followed body is not automatically a teleport:
   preserve the current possession handoff's ground/policy reset without
   unconditionally snapping or re-adopting camera ease. Preserve native
   cast member-loss and respawn settling too.
4. **Preserve absence and readiness meanings.** An absent selected subject
   clears that view's snapshot/ease according to the existing lifetime
   contract. Losing the session default clears views that use it, not every
   view. Room preparation or portal readiness retains its stated hold policy;
   do not convert temporary unavailability into a different target.

This can be a local restructuring of the existing resolver and view facts.
Use an explicit small selection/result only where it removes ambiguous
fallbacks. A cast's aggregate focus is a presentation value, not another
simulated body. No global observer registry, reference-frame graph,
renderer-specific authoring type or second simulation is needed.

#### Bind preferences without overwriting view-authored policy

**Done 2026-11-23.** Each local view carries `ReferenceFrameSource`
(`Preference` by default, in `local_view_facts`); the settings writer
updates only views bound to the preference, and
`ReferenceFrameSource::authored(frame)` marks a composition's own choice.
A temporary probe through `CameraObservationPlugin` and an Update (a default
view and an authored world-fixed pane) read (SubjectFrame, WorldFixed) under
a SubjectFrame setting, (WorldFixed, WorldFixed) under WorldFixed, and the
pane re-authored SubjectFrame kept it under WorldFixed; it was removed.

The same source has a second ownership collision:
`apply_camera_reference_frame_setting` writes the global setting into
**every** differing `CameraReferenceFrame` each Update. A composition's
explicit WorldFixed pane therefore does not survive a host Update when
the preference is SubjectFrame. The direct-write test
`a_views_reference_frame_is_a_component_a_game_can_write` has no settings
or Update; it proves component availability, not the production composition.

Make inheritance from user preference an explicit binding/default policy.
Views bound to it keep receiving real preference changes; explicitly
authored views retain their own policy. Do not infer ownership from whether
the current values happen to compare equal. Resolve one effective policy
per view and let the existing snapshot/renderer road consume it.

#### Native proof, deletion and cost

Extend `resolved_snapshot_lifetime_tests` through the public view-spawn seam
with an explicit no-driver spectator and an unnamed default view together.
Remove and restore only the default cast: the default view withdraws and
adopts its next frame, while the spectator continues. Then remove only the
explicit subject: only that view withdraws, with no default fallback. Add
the independent cast-spread and placement cases above; test both directions
of the placement dependency, not only the primary teleport.

Run the preference-binding case through a normal Update with settings:
one view inherits, one explicitly selects the other policy. Change the
global preference and preserve both intended results. Keep the existing
single-view preference test, cast lifetime/adoption, human/CPU cast framing,
and possession-follow behavior as controls. Include an actual possession
handoff in the two-view fixture: driven position/velocity/down changes,
the declared home baseline/zoom/blink dependency remains, and the unrelated
pane stays continuous. Control/body-motion tests alone do not prove this
camera policy. Keep `cast_camera_jerk_on_a_fast_display` as the
presented/tick-sample control. Use the real two-view host for changed
rendered camera behavior; headless snapshot assertions alone do
not certify pixels or controller orientation.

Delete the pre-view default gate and singleton target-derived placement/
cast-easing ownership. Keep shared static configuration and native framing
helpers. Do not move the whole old prelude inside the loop and repeat work
for every unrelated cast. Resolve only requested populations; shared
immutable observations may be reused, while temporal easing remains scoped
to the view and its policy.

The current companion loop scans all `BodyKinematics` for each view to
find `FrameAlongside`. If that loop is touched, prefer querying the actual
companion relation before considering a new index. This is a source-visible
cost opportunity, not a measured speedup. Retain bounded per-view state and
the direct single-subject path; no new all-body pass or cache-invalidation
program is selected.

### M3 — adaptive shared/split presentation

Promote when Ambition needs dynamic shared/split behavior rather than merely a
fixed second viewport.

Acceptance should cover:

- stable split/merge thresholds with hysteresis;
- explicit shared-view reference-frame policy when subjects disagree on
  orientation;
- correct per-view camera continuity and layout during regrouping;
- no participant/control reassignment caused by layout changes.

**Shared orientation also constrains input meaning.** MV1 makes the view's
selected anchor explicit; it does not decide which participant-relative
control style a product wants. The current cast helper chooses the first
resolvable body, and match publication sorts by seat. A stable first seat
is not an authored shared-orientation policy.

[`InputFrameMode::under_camera`](../../../crates/ambition_geometry/src/reference_frame.rs)
collapses every preference to BodyRelativeStrict under SubjectFrame. Its
equivalence assumes the observing view follows the same basis as that
controlled body. For a shared observer upright with A (`down = (0, 1)`),
while B's down is inverted (`down = (0, -1)`), a fresh right input
interpreted body-relative goes world-right for A and world-left for B.
Body-relative control remains a legitimate
choice; it is not equivalent to the shared screen direction for both.

At the selected M3 customer, state the shared orientation rule and each
participant's logical input-reference policy. Preserve explicitly
body-relative behavior and held-world-direction continuity through gravity
changes. If screen/observer-relative controls are selected, identify the
logical observer/reference at [IN1's admitted input cut](participant-action-system.md#in1--record-and-replay-at-the-same-admitted-input-cut),
with sufficient replayed meaning for deterministic body interpretation.
Do not read eased render-camera rotation in simulation or resolve input
against a body at host capture before its authoritative cut. Extend the
bounded admitted representation only if the chosen policy cannot be
expressed by the existing fields. Layout changes must not silently change
input ownership or the intended reference.

Prove fresh direction and held direction separately with two differently
oriented bodies in one shared view, then split and regroup. Keep the
same-body SubjectFrame equivalence and current per-seat possession/frame
tests as controls. This is a refinement of M3's existing product choice,
not an unconditional replacement of the shipped control modes.

Ambition's integration of it — the input → body → picture and view → camera
chains, the defects found on 2026-10-24 and the acceptance matrix — is
[`local-coop-integration.md`](local-coop-integration.md).

### M4 — several resident rooms

Built for the simulation and the view (OW1, V1-V5 in
[`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)).
A second seat joins with a Jump press at the primary's room
(`session/join.rs::seat_a_joining_participant`), and a fallen second seat comes
back beside the primary; both were built before the Q153 ruling of 2026-10-19
(an explicit join in an opt-in mode, queue row LOCAL-PLAY-MODES). A
participant's death is local to it and its room, and only a full-party wipe
rolls the session back (Q151, amended 2026-10-19).

### M5 — view-scoped HUD/prompt/presentation ownership

`ControlPrompt` and the banner are one per session and follow the primary seat.
The Q150 ruling (2026-10-03) makes HUD state per participant, also when views
merge onto one screen, so no "primary-player HUD" arbitration exists. Local
music uses authored priority across local participants, with the primary
participant breaking a tie. The simple heard-room tier selection is built.
[MUS2](music-programs-and-playback.md#mus2--select-one-occurrence-owned-goal-for-all-music)
closes the remaining adaptive-only priority and within-room occurrence
gaps; it does not replace the completed room filter or tie rule.
The built-in vitals HUD is per view: each
`LocalView` carries `ViewHudFacts`, the meters of the body it follows, and has
its own HUD in its own column. On a merged screen, the first view that names
nothing carries `SharedViewHudFacts`: each other seat whose body is in the
controlled body's live room and that no view follows, with its own HUD
stacked in that view. Open: the declared readouts (`HudReadouts`) are one per
session; stacked HUDs carry no participant name; and every seat counts as
local, so an online peer would show a remote seat's HUD (A4).

The same future locality authority must supply music's local listening
participants. Remote seats must not arbitrate another client's output.
Music decode/device readiness remains presentation state and must not
become a peer-room commit condition.

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
