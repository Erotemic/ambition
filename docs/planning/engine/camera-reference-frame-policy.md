# Camera reference-frame policy

**State:** OPEN / DECIDED DIRECTION. The capability is wanted; exact presentation UX remains intentionally unresolved.

## Maintainer decision

Ambition should support an **optional per-view camera reference-frame policy**.
The existing world-fixed/external-observer behavior remains a first-class option.
A second mode follows the controlled/view subject's resolved body frame so a
gravity change can appear as the world rotating around an upright controlled
body.

This is a **presentation policy**, not a gravity rule. Gravity, collision and
body integration remain exactly the same simulation facts regardless of which
view observes them.

The policy may be selected by the game, mode, room/context, local view or user
setting as appropriate. Do not bake one choice into the engine globally.

## Why this belongs in the engine

The current engine already separates most of the required facts:

- every integrated body receives one `ResolvedMotionFrame` for the current sim
  tick;
- `ResolvedMotionFrame` exposes the same basis/down direction that movement,
  abilities and body-relative input consume;
- movement and aim already have `InputFrameMode` policies, including
  `BodyRelativeStrict`;
- `CameraSnapshot2d` already carries `rotation_radians`;
- the renderer already applies that roll to the Bevy camera transform;
- portal-camera continuity already proves a non-zero camera roll can be layered
  in presentation;
- HUD/UI presentation is already separable from the gameplay camera.

What is missing is an explicit observer-frame policy and correct camera geometry
when the viewport is rotated.

## Policy model

Do not replace the existing camera framing/zoom/scroll policies. Reference-frame
selection is orthogonal to them.

Conceptually:

```text
Local view
  subject/framing policy
  viewport/layout policy
  camera framing/zoom/scroll policy
  reference-frame policy
```

The first two reference-frame semantics we need are:

### External/world observer

The current ordinary behavior. Screen orientation remains tied to the world
frame even when the controlled body enters sideways or inverted gravity.

This is useful for ordinary platformer readability, distant-observer contexts,
shared cameras, some rooms, and games that do not want gravity to rotate the
screen.

The exact public enum name is deliberately not fixed here. `WorldFixed`,
`ExternalObserver` and similar names all describe aspects of the intent.

### Subject-relative observer

The view orientation follows a designated view subject's resolved reference
frame. The body-local `side` axis should read as screen-right and body-local
`down` as screen-down, so when gravity rotates the frame the presented world
rotates instead of forcing the participant to mentally rotate controls.

The subject is normally the body controlled by the local participant, but the
engine concept should be a **view subject**, not a privileged protagonist. A
spectator, replay or future inspection view may choose another subject.

A later explicit observer-frame mode may be useful for TwinTrack or other games
whose observer is not simply world-fixed or attached to one body. Do not invent
that API before a real second customer requires it.

## Input/control relationship

A subject-relative view should normally be paired with body-relative movement
and aim semantics so the participant's visual screen axes and controlled body's
local axes agree.

The existing `BodyRelativeStrict` behavior is the closest mechanical statement
of that rule. However, **camera code must not mutate gameplay settings or input
state**. Keep these as separate policies and let a game/context/user-facing
preset select a coherent combination.

Conceptually:

```text
simulation frame     = authoritative body/world fact
control-frame policy = interpretation of participant input
view-frame policy    = presentation of that fact
```

They can deliberately agree without becoming one subsystem.

Existing screen-relative and assisted control modes remain available when a
game or participant chooses an external/world observer.

## Multi-view relationship

This policy is per local view.

A future split-screen session may legitimately have:

- one subject-relative Ambition view and one world-fixed spectator view;
- two subject-relative views following bodies under different gravity frames;
- TwinTrack views using different observer/reference-frame presentation;
- a shared view using an external/world observer because its subjects disagree
  about orientation.

Do not store one process-global `camera_frame_mode` that becomes impossible to
index when D116 lands.

A shared view containing several subjects with incompatible body frames is a
real design case. The default should not silently pick "player one". The
presentation policy may retain an external/world frame, choose an explicit
orientation subject, or cause an adaptive view policy to split. The exact
selection rule remains open.

## Camera geometry that must become rotation-aware

Setting `rotation_radians` is not sufficient.

The current camera resolver reasons about an axis-aligned `visible_view` while
room/camera-zone clamps assume the viewport footprint is axis-aligned in world
space. A rolled view has a rotated footprint.

For cardinal 90-degree orientations this visibly swaps the effective world
width/height. For arbitrary angles the world-space footprint is the AABB/convex
footprint of a rotated rectangle.

The implementation must ensure that:

- room clamps contain the actual rotated view according to the selected clamp
  policy;
- camera-zone offsets and soft framing have defined world-vs-view semantics;
- safe-area framing remains correct;
- capture/headless resolution produces the same camera snapshot semantics as the
  visible renderer.

Do not special-case only four gravity directions if the underlying frame model
already permits arbitrary orientation.

## Presentation composition

The base observer roll must compose with other presentation effects rather than
compete with them.

In particular audit:

- portal continuity: today it can overwrite `CameraSnapshot2d.rotation_radians`;
  a body-relative base roll and portal transition roll need one deliberate
  composition rule;
- camera shake: decide whether shake offsets are screen-local or world-local
  under a rolled view;
- camera easing: decide whether orientation snaps, eases, or uses a policy based
  on why the body frame changed;
- possession/body switching: a subject-relative view changes orientation source
  when control moves to another body;
- UI/HUD: keep UI upright by default; rotating UI should require an explicit
  separate presentation policy.

## Implementation phases

### C1 — pure observer-frame policy — DONE (2026-08-14)

`CameraReferenceFrame` (`WorldFixed` default, `SubjectFrame`) and
`observer_roll_radians` live in `ambition_sim_view::camera_snapshot`, next to the
snapshot they orient. The helper takes a DIRECTION, never an entity, so nothing
on this path can assume the subject is a protagonist, and it queries no gravity —
the caller passes the already-resolved down axis.

⭐ **the sign convention is not a choice, it is inherited.** Render space is world
space with y flipped, which is what `portal_transit_roll` already established for
the only other producer of `rotation_radians`; screen-down is render `(0,-1)`, so
the roll is `atan2(down.x, down.y)`. Ordinary gravity is the identity, which is
what keeps every existing room from tilting the moment the mode is selectable.
Pinned by tests over the four cardinals, a diagonal (the model permits any
orientation, so quantising to four would be a different feature), and the
degenerate cases.

`CameraSnapshotResolveInput` carries `reference_frame` + `subject_down`, and the
resolver sets `rotation_radians` from the helper — so the policy has a real
consumer at the authoritative seam rather than sitting unused until C2. All five
construction sites state the world-fixed default, and because the struct is built
exhaustively, a new view cannot forget to say which frame it presents in.

### C2 — one-view Ambition proof — DATA WIRED, SELECTION OPEN

The live camera system reads the followed body's `ResolvedMotionFrame` and passes
its down axis as `subject_down`. ⭐ **read off the SAME entity the framing
follows**, so orientation and framing cannot disagree about whose view this is,
and read as an already-resolved fact rather than by asking gravity anything.

⛔ **`GravityField` was the tempting shortcut and is the wrong source.** It is a
per-tick mirror of the PRIMARY BODY's resolved frame, so a view following any
other subject would orient on the protagonist — the exact assumption acceptance
forbids — and it is a world-gravity resource, which is what "do not put gravity
queries in the camera" rules out.

**What remains is the selection, and it is deliberately unbuilt.** The policy
still resolves to `WorldFixed` everywhere, so selecting `SubjectFrame` is now a
policy change rather than a plumbing change. ⛔ whatever selects it must not be a
process-global mode: a VIEW owns this, and the one-view case should be the
one-entry case rather than an architecture to remove at D116. A component on the
camera/view entity is the shape that survives that migration; a resource is not.
Pairing the product preset with body-relative movement/aim stays a separate
policy — camera code must not mutate input state.

### C2 — one-view Ambition proof

Feed the current controlled/view subject's `ResolvedMotionFrame` into the camera
observation seam and expose the subject-relative mode in an Ambition-accessible
configuration path.

Pair the product preset with body-relative control semantics without coupling the
camera implementation to input mutation.

### C3 — rotation-aware framing and clamp

Make room clamp, camera zones, soft framing, aspect handling and scene capture
correct for a rotated viewport.

⚠ **C3 ALREADY HAS A LIVE CUSTOMER, AND C4 BLOCKS IT — measured 2026-08-14.**
This reads as future work for a mode nobody can select yet. It is not: portal
continuity rolls the view TODAY, `somersault_roll` returns ±π/2 for a
floor↔wall pair, and at a quarter turn the world-space footprint swaps width for
height. The clamp does not know. `CameraExtraClamp` looks like the compensation
and is not — it widens the clamp by an extra CENTER so the portal's target
position stays reachable, and says nothing about extents. So a floor↔wall
transit can already show outside the room.

⛔ **and the obvious fix cannot be applied where the bug is.** The resolver
computes the clamp from axis-aligned `half_view_w`/`half_view_h`; the portal roll
is written onto the snapshot afterwards, in the renderer. A resolver cannot
account for a rotation it is never told about. Making the clamp rotation-aware
therefore requires the roll to be known at resolve time — which is exactly C4's
composition question, because it means the portal roll and the base observer roll
must arrive at one place under one rule.

⇒ **do C4 before C3.** The base roll already reaches the resolver (C1/C2); the
portal roll does not. Until both do, a rotation-aware clamp would be correct for
the mode nobody selects and still wrong for the one that ships.

✔ **DONE 2026-08-14, immediately after C4 unblocked it.** The clamp now measures
the view's world FOOTPRINT rather than its size: `rolled_view_half_extents`
returns the axis-aligned bound of the rotated rectangle, which is the question a
clamp that must CONTAIN the view is actually asking. At zero roll it is the
identity, so every upright view — every view outside a transit — clamps exactly
as it did; at a quarter turn it swaps width and height, which is the defect.

⭐ **the sign convention turned out not to reach it**, which was worth deriving
rather than assuming: both terms take absolute values and `cos(-t) == cos(t)`,
`|sin(-t)| == |sin(t)|`, so a render-space angle and a world-space clamp agree
about the footprint. Rolling either way occupies the same rectangle. Pinned by a
test, because the next reader will wonder the same thing.

Still open for later: `visible_view` keeps the UNROTATED extents (it describes
the view's own size, which is what its consumers mean), camera-ZONE bounds are
clamped by the same footprint, and safe-area framing has not been re-derived for
a rolled view. Capture parity holds trivially today — captures pass no transit
and select no subject frame — and needs a real test the moment either changes.

### C4 — presentation composition — PORTAL HALF DONE (2026-08-14)

Specify and test composition with portal continuity, shake, easing, possession
and abrupt gravity changes. **The portal composition is derived, implemented and
tested; shake, easing and possession remain.**

⭐⭐ **THE TWO ROLLS ARE NOT INDEPENDENT ANGLES TO ADD.** That was the tempting
rule and it is wrong. The portal roll is the render-space rotation of the portal
MAP (`portal_transit_roll`, a property of the pair — not of gravity, not of the
observer, not of the body), and its own docs say the final camera orientation is
always identity: it is a temporary alignment so the visible chart lines up
during the seam.

Take a floor↔wall pair, map rotation `M = ±π/2`:

- **destination gravity matches the source's.** The body somersaults, its down
  axis is unchanged, the base roll is unchanged. The view needs `M` on top of
  its base for continuity and gives it back at the end ⇒ `base + M`.
- **the portal also changes the body's frame** (it lands on what is now its
  floor). The subject's down rotates by `M` too, so a `SubjectFrame` view's base
  roll has ALREADY moved by `M` the instant the body crossed. Adding `M` again
  spins the world a full extra half turn through the seam and then spins it back.

⇒ **one rule covers both: the view presents the roll it had ADOPTED at entry,
turned by the chart rotation.** In the first case the adopted roll is still the
live base (`base + M`); in the second it is the pre-crossing base, so the result
is exactly the post-crossing base — the view arrives where it was always going,
with no overshoot. `WorldFixed` is the degenerate case, not a separate path: its
base is identically 0, so the rule reduces to `M` decaying to 0, which is
byte-identical to the overwrite that shipped. The old code was not wrong; it was
one instance of this rule, written where the general case could not be expressed.

**Where it now lives.** `presented_roll_radians` in `ambition_sim_view`, fed by
`CameraPresentationInputs` — which REPLACED the single-field `CameraExtraClamp`
rather than adding a resource beside it, because both fields are the same act:
*what the presentation layer needs the resolve to know before it resolves.*
`publish_portal_camera_clamp` writes both pre-resolve and latches
`observer_roll_at_entry` on the transit's rising edge from the previous frame's
resolved roll (that frame had no transit, so its roll is the pure base).
`camera_follow` no longer writes `rotation_radians` at all.

⇒ **C3 is unblocked**: the resolved snapshot now states the view's actual final
orientation, so a rotation-aware clamp is expressible where the clamp is
computed.

### C5 — view-index migration

When D116 indexes observer facts by local view, move this policy with the view.
The current one-view representation should become the one-entry case rather than
an architecture that must be removed.

### C6 — acceptance customers

Use at least:

- an Ambition gravity-change room with world-fixed mode;
- the same room with subject-relative mode;
- body-relative movement/aim in the rotating view;
- possession or another view-subject change;
- eventually two simultaneous views with different policies;
- TwinTrack as the stronger independent-observer/reference-frame customer.

## Acceptance

- current world-fixed/external-observer camera behavior remains selectable;
- a subject-relative view keeps the designated body's resolved frame visually
  upright while gravity changes;
- camera choice changes no simulation/gravity/collision state;
- movement and aim can be configured coherently with the subject-relative view;
- rotated viewport clamping is correct rather than merely visually rolled;
- portal roll, shake and easing have explicit composition semantics;
- the policy belongs to a view and is compatible with future split-screen;
- no code path assumes the designated subject is `PrimaryPlayer`.

## Open design questions — deliberately unresolved

The direction above is decided. These choices still need evidence or feel work:

- What are the public names: `WorldFixed`, `ExternalObserver`, `SubjectFrame`, or
  something clearer?
- Is subject-relative mode exposed primarily as a user setting, a game/mode
  policy, authored room policy, or a layered precedence among them?
- Should the user-facing subject-relative preset force strict body-relative
  movement **and** aim, or should games be allowed to pair it with assisted
  controls? The low-level policies must remain independent either way.
- When gravity changes discontinuously, should orientation snap, ease by the
  shortest arc, or use context-specific transition policy?
- Should camera zones/clamps be authored entirely in world space, view space, or
  support explicit semantics for each?
- How should screen-local shake compose with a rotating observer?
- How should portal continuity roll compose with the base observer frame during
  a portal crossing?
- When a shared view frames subjects with incompatible body frames, should it
  remain external/world-fixed, follow one explicit orientation subject, or ask
  adaptive layout to split?
- Does a possession transition inherit/ease into the new body's orientation
  immediately or according to a separate feel policy?
- Which parts of this policy belong in a future independently consumable Bevy
  camera/observer plugin versus Ambition-specific presentation presets?

Do not answer these by silently encoding the easiest current single-view case.
