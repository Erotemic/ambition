# Local co-op integration: input, bodies and adaptive multiview

**Scope:** The Ambition game's BG3-style adaptive shared/split-screen couch co-op,
using the engine's ordinary participant, live-room and view systems. This is a
focused integration/acceptance design, not an independent work queue or a new
multiplayer model. The executable priorities remain in
[`../queue.md`](../queue.md) (`LOCAL-PLAY-MODES`, `COOP-RECOVERY`, and subsequent
multiview work).

**Architecture authorities:**
[`multiplayer-and-multiview.md`](multiplayer-and-multiview.md) owns the generic
participant/view model;
[`../game/multiplayer.md`](../game/multiplayer.md) owns Ambition's product
rules; [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)
owns room residency; [`control-authority-and-ai-policy.md`](control-authority-and-ai-policy.md)
owns body control; and
[`participant-action-system.md`](participant-action-system.md) owns the
participant-scoped UI/action direction. Reconcile any disagreement with those
authorities rather than silently superseding them here.

## Product behavior to preserve

Solo remains the default. A gameplay setting may *permit* a second local
participant, but enabling permission is not itself a join. With permission off,
the keyboard and unclaimed gamepads drive the one local player. With permission
on, a distinct controller pressing Start can claim a second seat, creating a
second ordinary controlled body in the same persistent world. The setting
should expose the supported number of local players without confusing the
**maximum admitted** with the **number currently joined**. The controller/seat
association must survive ordinary render changes; leaving must be explicit.

The default local-co-op presentation is adaptive:

1. **Share** a view while one camera can frame both participants coherently.
2. **Split** into separately tracked views when their separation exceeds the
   framing threshold, or immediately when they are in distinct live rooms.
3. **Merge** after both return to compatible framing, with hysteresis and a
   dwell time; do not toggle repeatedly near a threshold.
4. All view transitions are *presentation only*. They do not teleport actors,
   alter input assignments, reset simulation, or change world residency.

This is the intended behavior, not an optional temporary implementation.
Nothing here mandates two permanent half-screen cameras. Shared, fixed and
adaptive layouts remain distinct policies over the same views.

## Reported failure and source findings (2026-10-24)

The player reports that controller 2 can pose or trigger some actions but does
not appear to move its character, and that the pane created during a split is
black. Do not mark those behaviors fixed solely because a narrow unit test
passes. Prove them in the shipped host with a real gamepad and visible render.

Two concrete, separable defects in source snapshot `cc2f82eaf891`:

- `ambition_render::rendering::actors::sync_visuals` places the sprite of the
  one `PrimaryPlayer`. A second body's simulation/animation can advance while
  its picture remains at the old transform. It must project **every**
  `PlayerVisual` from **that entity's** `BodyPoseView`, `PresentedPose`, and
  `LiveRoomOf<RoomGeometry>`, using the existing authored-offset, feet-anchor,
  crouch, gravity and roll rules. Keep primary-only sprite-size logging
  primary-only; do not make rendering position primary-only.
- The generic presentation plugin registers `present_split_view_rigs`, but the
  main Ambition host installs a different render composition and omits that
  lifecycle. `LocalView` facts can therefore exist without a Bevy camera
  bound to the added view. Register one reusable split-view camera capability
  in shared host composition. Every extra rig must have `PresentsView`, the
  gameplay render-layer base, and the expected screen-effect eligibility;
  the host's primary camera remains host-owned. Extra rigs retire when their
  views retire.

The packaged code diff addresses these two source defects and adds tests.
**The remaining physical analog-input defect is not established as fixed.**
Existing app witnesses primarily exercise D-pad button input. A new left-stick
regression is included, but the agent must confirm how Bevy populates sampled
axes under the actual input schedule, including newly connected devices.

The joined body is also currently created at the primary body's location;
collision-safe deterministic join placement is still needed. That is not
proven to be the cause of the controller problem and must not be used as a
substitute for tracing the input/body pipeline.

## Model and ownership boundaries

A normal local-control path should be understandable as:

```text
physical device / keyboard
  -> stable local device identity and seat claim
  -> participant / player slot
  -> per-slot raw input sample and control intent
  -> control assignment / DrivingParticipant
  -> authoritative actor movement + actions (fixed-step simulation)
  -> body kinematics, posed state, and live-room identity
  -> frame-timed presented pose
  -> entity's own sprite / articulated rig transform

local participant(s) / framed subjects
  -> LocalView(s) + view policy + view placement
  -> resolved per-view camera snapshot
  -> one bound Bevy gameplay camera per active LocalView
  -> viewport, room/view render layers, parallax and portal captures
  -> player-scoped HUD and UI on the correct display region
```

Do not collapse any adjacent rows merely because today there are two players.
In particular:

- **Device is not participant.** An input device may disconnect, reconnect or
  be reassigned; the participant's sim identity need not change. Never assume
  the most recently active controller drives the primary actor.
- **Participant is not body.** Bodies can die, respawn, transform, be
  possessed, or change rooms without minting a new participant identity.
  Control is an explicit assignment and simulation uses a common movement
  path for both participants. Do not add `player_2_speed`, cloned fixed-step
  systems, or seat-index-dependent actor physics.
- **Body is not visual.** The authoritative body location/room/roll belongs
  to simulation and the published pose. The sprite and articulated components
  are projections, not alternative spatial authorities. A body must not be
  moved to compensate for a picture that was never synchronized.
- **View is not participant.** A shared view frames several bodies; one local
  participant can have no view on a remote client; a spectator view can have
  no controlling participant. Use explicit `PresentsView` links instead of
  selecting an arbitrary `MainCamera` or `LocalView` from query order.
- **View is not room.** Two views can frame the same room; two participants
  can keep different rooms resident. Visibility follows the **view's** room
  and render-layer contract, not whichever room the primary body occupies.
- **HUD is not simulation.** Prompt, health, inventories and input glyphs
  follow explicit participant/view ownership. Shared camera does not mean
  shared body state.

Primary-player shortcuts are appropriate only for deliberately single-player
features or the expressly designated fallback. For each shipped gameplay and
presentation use of `PrimaryPlayer`, classify it as genuinely primary-only,
per-participant, per-view or per-session. Do not bulk-replace it blindly.

## Rendering architecture (one owner per responsibility)

**View policy** (`ambition_sim_view`) decides which `LocalView`s exist, what
they frame, and their `ViewPlacement`/camera snapshots. It owns split/merge
hysteresis and room-dependent splits, not physical cameras.

**Camera lifecycle** (`ambition_render::platformer_presentation`) creates and
retires the extra Bevy rigs for opened views. The hosting app owns the primary
camera and display/full-screen UI rig. Multiple game hosts should reuse the
same capability, without duplicate plugin registration or camera creation.
The invariant is one bound gameplay camera for every *active rendered* view,
no unbound/stale extra camera, and no accidental second camera for the host's
primary view. Registration must be idempotent at the composition boundary.

**Camera application** (`rendering::camera_follow` and the host viewport
applier) resolves a camera exclusively through its `PresentsView` link, using
that view's camera snapshot, room geometry, physical viewport and scale.
A camera should not inherit one global player-centered sample when two
observers are active.

**Scene isolation** (`rendering::view_isolation`, parallax/room presenters,
portal presentation) attaches visuals to the correct live-room and view
render-layer bands. Both same-room and different-room splits need correct
backgrounds, actors, projectiles, effects, HUD, screen effects and portals.
The gameplay layer base is shared between primary and split cameras; room
bands and view bands are added by the isolation passes. Do not fix a black
pane by making every camera draw every room.

**Sprite/rig placement** uses each drawn entity's own published pose and room,
including animated characters, fallback sprites, transformed characters,
physics orientation, and portal chart continuity. Test both the actor root and
articulated children; a working pose animation does not imply correct root
placement. Missing room association is a diagnosable defect, not permission
to project the entity in the primary room.

**Portal windows** require another later cut: two views of the *same* room
currently share the first view's observer for through-portal wedges and
body cuts. Different-room isolation has made progress, but per-view portal
captures, clipping and sky copies still need explicit observer/view keys.
Keep this separate from fixing the black camera, so the next patch can be
reviewed independently. Do not duplicate authoritative room simulations to
produce different windows.

**Split transitions** must preserve stable seat ownership, body pose and
camera continuity. Target viewport rectangles and HUD regions must change
together. Verify the boundary frame at split, at merge, and when a participant
crosses a doorway as the other remains behind. Preserve access to menu UI
while gameplay cameras are in half-screen viewports.

## Input and body correctness

For every joined participant, test the complete mapping from a particular
physical controller to the intended body, not just per-slot input resources.
Input tests should distinguish directional D-pad, analog stick (including
nonzero diagonal axes and deadzones), jump, attack, pose, pause, and release.
A controller that animates the body without changing `BodyKinematics` demands
inspection of raw axes, latched `SlotControls`, controlling assignment, movement
intent, and actor motion for **that seat**. A body that moves in
`BodyKinematics` while its `Transform` remains fixed demands presentation
investigation instead. These are different failures.

The join operation belongs in deterministic simulation: claim the input seat;
join a participant once; bind exactly one controlled ordinary actor; place
that actor through a deterministic, collision-safe rule in the intended live
room; and publish its appearance. Avoid spawning bodies in overlapping solid
geometry, unbounded local scans, frame-order-derived randomness, and ad hoc
teleports. A later join after leave must not reuse stale view/HUD/camera
entities. Retaining a seat claim while local joins are toggled off follows the
existing Q153 policy until the participant explicitly leaves.

Disconnects should stop or reassign the **input** through explicit policy,
without accidental body death or duplicate ownership. Reconnect should
restore the claimed device's channel, with negative tests proving another
controller cannot steal it through an unrelated action.

## Game-state rules (already decided; do not reopen silently)

- A participant death/respawn is local to that participant and the affected
  room. Only a full-party wipe rolls back the session to a checkpoint. Each
  participant uses the ordinary single-player respawn-location rule for
  itself; respawning beside a living partner is an opt-in policy, not default
  (Q151/Q153; see `COOP-RECOVERY`).
- The paused menu is owned by the seat that opened it; that seat drives it.
  Player 1 can remove Player 2 and Player 2 can leave from its menu. Leaving
  retires the body via a stamped sim intent and accounts for carried items
  without loss or duplication (`LOCAL-PLAY-MODES`).
- Interactions, item ownership/transfer, prompts and readouts must identify
  the acting participant/body; merged view does not grant the primary player
  sole authority. Shared quest/story decisions are product policy where not
  yet ruled, not a reason to make cameras own simulation state.

## Engineering sequence and evidence

These are implementation dependencies, **not** an alternative priority list to
`../queue.md`. A working end-to-end co-op slice requires all of the following.

**1. Restore invariants at the existing seams.** Integrate and, if necessary,
rebase the packaged changes: shared split camera lifecycle, symmetric
body-to-visual projection, layer parity, and main-game regression witnesses.
Check that the new camera plugin composes only once with both Ambition and the
generic platformer presentation. Avoid host-specific second-camera logic.

**2. Prove actual movement and actual pixels.** Test controller A and B with
D-pad and left stick. Capture per-seat evidence at device raw frame, slot
control, assignment, `BodyKinematics`, presented pose and sprite transform.
For the black pane, check `LocalViewId`, `PresentsView`, active camera,
viewport physical rectangle, projection, visibility mask, rendered room
layers and camera clear/order. One camera being counted is necessary, not
sufficient: its viewport must contain the correct pixels.

**3. Close join/leave and respawn contracts.** Add deterministic safe spawn,
independent pause/leave, item accounting, checkpoint and local respawn
behavior. Keep participant lifecycle separate from camera lifecycle. Drive
join/leave through the normal simulation/rollback intents.

**4. Finish per-participant presentation.** Individual vitals already exist;
finish named HUD, prompts, interactions, inventory menus, screen overlays,
and UI routing while merged and split. Viewport-aware HUDs must not duplicate
world actions.

**5. Complete multi-observer portal and polish.** Move one-eye-per-room portal
presentation toward one-eye-per-view where the same room is watched twice;
then tune split/merge thresholds, transition animation and camera easing.
Avoid weakening isolation just to make the pane nonblack.

**6. Profile and guard.** Measure one camera vs two in the shipped Ambition
hub and Mary-O-sized scenes on desktop/Android. Capture CPU fixed/update,
visibility/extraction, render passes, portal capture count, parallax and
memory. Keep single-player overhead bounded: absent a second participant,
there should be no second gameplay rig or duplicated per-frame simulation.
Do not force the simulation tick to 60 Hz to make split-screen behave.

## Acceptance matrix and non-regression witnesses

| Scenario | Required evidence |
|---|---|
| Solo, joins disabled | One body/view/camera; every unclaimed controller drives the existing solo seat; no new actor from Start/Jump. |
| Join on pad B | Exactly one new claimed seat/body after Start; pad A/keyboard still drive Player 1; pad B drives only Player 2. |
| Analog movement | Player 2's raw left-stick axis, latched intent and body displacement agree; D-pad and actions also work. |
| Body vs picture | Each participant's `BodyKinematics`, presented pose and sprite root move consistently in the correct live-room geometry; animations and roll remain correct. |
| Same-room shared | Both bodies visible and independently moving; both HUDs readable; one gameplay camera. |
| Same-room far apart | Adaptive policy opens exactly one extra view/camera; each pane shows world, backgrounds and its own framed body. |
| Regroup | Hysteresis merges to one camera; extra camera/HUD projections retire; neither body teleports or loses input. |
| Different rooms | Both rooms stay simulated/resident; each camera renders only the correct room and associated effects. |
| Portal seen from two eyes | Each view's window/pieces reflect its own observer; no shared primary-view cut or leaked sky. |
| Controller disconnect/reconnect | Claimed seat retains ownership or follows explicit reconnect policy; no duplicate body or reassignment. |
| One death | Independent local recovery, surviving player and world persist; full-party wipe restores the checkpoint exactly once. |
| Leave/rejoin | Body/items, claim, HUD, room residency and cameras clean up once; subsequent join is fresh and deterministic. |
| Menus during split | Seat that paused controls menu; full-screen menu and UI camera stay visible and clickable. |
| Performance | Single-player baseline unchanged in structure; two-view costs profiled on weak target hardware. |

Use deterministic headless and app-level tests for identity, movement, camera
lifecycle and isolation, plus at least one **rendered** main-app acceptance
scenario. A headless camera-count test cannot prove a nonblack viewport or
visible pose. Prefer focused tests on each authority boundary instead of one
large brittle test that mocks half the engine.

## Concrete code anchors at the October 24 baseline

- `crates/ambition_input/src/local_seats.rs` — device claims/seats.
- `crates/ambition_platformer2d_actor_monolith/src/session/join.rs` — sim join.
- `crates/ambition_platformer2d_host/src/lib.rs` — shared host composition.
- `crates/ambition_sim_view/src/local_view.rs` — adaptive view opening/merging.
- `crates/ambition_render/src/platformer_presentation.rs` — extra camera rigs.
- `crates/ambition_render/src/rendering/actors/mod.rs` — body/visual projection.
- `crates/ambition_render/src/rendering/view_isolation.rs` — camera room/view bands.
- `crates/ambition_platformer2d_host/src/gameplay_presentation.rs` — viewports.
- `game/ambition_app/src/app/scene_setup.rs` — primary host camera/HUD.
- `game/ambition_app/tests/local_play_modes.rs` — shipped-host control tests.

Do not assume these filenames remain fixed as the repository moves. Trace the
current owners before moving responsibilities; preserve generic ownership,
not this baseline's incidental module layout.
