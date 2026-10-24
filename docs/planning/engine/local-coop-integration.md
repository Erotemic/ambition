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

## Follow-up audit: co-op works, but participant and view scope leaks (2026-10-24)

**Baseline:** Ambition local co-op was integrated before source snapshot
`e2fddc465d5a` (2026-10-24). The maintainer reports that both participants
can play and adaptive views split and merge. The earlier camera and sprite
issues are historical findings, not current open defects. The observations
below are from the maintainer's two-controller playtest and a source review.
The source review did not execute a GPU playtest. Do not mark a symptom as
fully fixed from a source change alone.

**Execution:** [`../queue.md`](../queue.md), row `MULTIPLAYER-SEAMS`, is the
only task list. This section gives causes, owners, contracts, and test design.
It extends this integration plan; it does not replace
[`multiplayer-and-multiview.md`](multiplayer-and-multiview.md),
[`netcode.md`](netcode.md), or
[`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md).
The current product rules are in
[`../game/multiplayer.md`](../game/multiplayer.md).

### Source findings and confidence

| Reported symptom | Source finding | Confidence and limit |
|---|---|---|
| Player 2 cannot drive dialogue that Player 2 opened | `features/ecs/interact.rs` records an initiating `ConversationInputOwner`; `declare_in_session_input_contexts` claims dialogue input for that participant. But `ambition_dialog::systems::dialog_input` reads the global `MenuControlFrame`, not the owner's frame from `SeatMenuFrames`. | **Confirmed routing mismatch**; test actual dialogue commands and input capture on both seats. |
| Dialogue covers the entire display | `ambition_sim_view::DialogView` has no viewer/owner field. `ambition_content::presentation::dialog::sync_ambition_dialog_ui` uses one `DialogView` and a window-wide absolute root (`left/right/top/bottom = 0`). | **Confirmed global presentation**; renderer must prove viewport clipping with the real UI backend. |
| Cannot select always-split view mode | Ambition installs `AdaptiveSplit` in `ambition_content::provider`. `split_views_by_live_room` splits by room count and optional distance; `GameplaySettings` has no layout choice. | **Confirmed missing policy**. Add an explicit view policy instead of altering thresholds to prevent merging. |
| Player 2 stays in blink state | `cleanup_timers_system` decays `PlayerBlinkCameraState::{blink_in_timer,camera_snap_timer}` only for `PrimaryPlayerOnly`. `sync_player_presentation` can arm those timers for every `PlayerEntity`. The body kernel separately advances `BodyBlinkState` and `AxisManeuverState` on sim ticks. | **Confirmed presentation timer leak**; **not yet proven** to explain a stuck simulation blink hold/aim. Check both states on press, hold, release, and cooldown. |
| Player strikes do not hurt the other player | `FriendlyFire::default().enabled` is false. Both driven bodies resolve to `ActorFaction::Player`; `damage_lands_between` allows same-faction damage only when friendly fire is enabled (unless another relation overrides). | **Confirmed default rule explains allied damage rejection**. Prove Ambition's actual resolved room rule and all attack families. The explicit `friendly_fire: false` in `game/ambition_app/assets/data/room_rules.ron` is a **Versus** stage declaration, not the general Ambition room default. |
| Only one player can use the pause/menu | Ambition's kaleidoscope and grid menu systems read one `MenuControlFrame`, one `InventoryUiState`, and singleton focus/page/cursor state. `ShellPauseMenu` supports an owning seat but Ambition suppresses it and supplies its own menu. | **Confirmed singleton Ambition menu**. The shell seat rule is not two independent menus. |
| Either player's door flashes black over both panes | `room_transition_presentation::drive_room_transition_presentation` spawns one 100%-width/height `RoomTransitionCoverRoot` with `GlobalZIndex(900)`. `RoomTransitionLoadState` has one active transaction. | **Confirmed full-window cover**. Test whether sim time stops separately. The eager host already avoids `GameMode::RoomTransition` when another participant has play in progress; the peer host can still hold all simulation for a synchronized commit. |
| F1 gizmos show the wrong room or offset | `ambition_render::rendering::debug_viz::draw_debug_viz` and `ambition_app::dev::debug_overlay::draw_debug_overlay` select `PrimaryLiveRoom`/`PrimaryLiveRoomSpec`. They use the default gizmo path, which all gameplay cameras can see. App labels also convert through the primary room. | **Confirmed primary-room leak**, consistent with the report. Player trails already use retained `GizmoAsset`s with room `RenderLayers`; reuse that boundary. |
| Adaptive split/merge feels inconsistent | `AdaptiveSplit` keeps `split_beyond`, `merge_within`, `merge_after`, and a local `DistanceSplit` latch. | **No source defect established**. The maintainer deferred threshold and animation polish. Preserve the current policy and test topology only. |

### Architecture contracts for local and remote players

1. **Authority:** `ParticipantId` owns gameplay decisions. It has one explicit
   control assignment to a body while it is active. A local input seat is a
   device-routing channel; it is not a network identity or a permanent body.
   Validate the mapping when a controller joins, leaves, or reconnects, or
   when a player changes form or possesses another body.
2. **Simulation:** the fixed-step world owns body position, blink lifecycle,
   combat damage, dialogue decisions that affect the world, and room crossing.
   A camera, UI root, menu state, or local setting must not modify those facts
   without a typed, authorized simulation intent. Peer-relevant rules are
   identical on every machine and covered by the session contract.
3. **Local input:** each local participant has its own `SeatMenuFrames` frame
   and gameplay frame. A conversation or menu consumes only the frame of its
   declared owner. `AllParticipants` requires one explicit arbitration rule;
   iteration order must not decide a winning choice.
4. **Observation:** each `LocalView` has a stable identity, a placement, a
   resolved camera, a room, and optional participant associations. A shared
   view can show many participants. A local client need not render a remote
   body's own camera; a spectator view need not have a driver. Do not put a
   `LocalViewId` in rollback game state.
5. **UI:** a UI surface has its own *interaction owner*, *presentation target*,
   and *blocking scope*. These are independent facts. `ParticipantId` chooses
   input authority; a local presenter maps that participant to the current
   view/viewport; an explicit rule states whether it blocks one actor, local
   input, one room, or the whole session. A fullscreen UI root is reserved for
   intentionally session-wide content.
6. **Lifecycle:** room loading is a host preparation task. Admission/commit of
   room state is a simulation-authorized action. A door has an affected body,
   destination, transaction identity, and local view set. The global session
   does not have to become a loading screen just because one view changes.
7. **Diagnostics:** a diagnostic sample names the observed body, live room,
   and/or view before drawing. It uses the same presented pose and coordinate
   transform as the art it diagnoses. A debug tool may default to the primary
   seat only if its output is shown only in that seat's view.

These are independent axes. Do not replace singleton state with a pair of
`player1_*` and `player2_*` resources. Prefer keyed instances, typed
relationships, and shared system logic. Only create session-global state
when the product rule is truly session-global.

### Dialogue: one owner for choices, one target for the panel

`ActiveConversation` already carries the initiating actor, a stable
`ConversationInstanceId`, and `ConversationInputOwner`. Its projection into
`DialogState` is not rollback authority; narrative effects re-enter the
simulation through `NarrativeInputLedger`. Preserve that contract.

- Resolve the dialogue input owner's `ParticipantId` to the local input seat
  through the existing participant-seat map. Read **that** frame from
  `SeatMenuFrames`. Do not route all dialogue to seat zero. Make the
  owner-specific context claim control reveal/hold. If a different seat opens
  its pause menu, it must not silently suppress the owner's dialogue unless
  that is an explicit session-level rule.
- Project dialogue ownership and conversation identity alongside content in
  a presentation-neutral read model. Do not let the Bevy UI infer the speaker's
  owner from the first active camera or the selected primary player.
- When the participants share a viewport, show the conversation once in that
  viewport and label who controls it. When they split, constrain the UI to the
  initiating player's viewport, including portrait, choices, pointer/touch
  hit testing, safe area, focus, scaling, and clipping. Retarget on split/merge
  without restarting Yarn or losing the selected choice.
- The current authority permits **one live conversation per session**. Do not
  claim it supports two concurrent conversations. Before generalizing that
  policy, establish a product rule for simultaneous story choices in different
  rooms and a deterministic narrative transaction identity. An initial
  acceptance test must prove that an unrelated player keeps moving while one
  player talks.
- For netplay, distinguish a **locally presented** dialogue from a **world
  decision**. Only an authorized participant's stamped action may resolve a
  world-affecting choice. A spectator or other peer can observe without
  becoming the input owner.

### Split policy: adaptive by default, fixed split as a local option

Create one explicit layout-policy value, for example `Adaptive` or
`AlwaysSplit`, in the local presentation settings. The default is `Adaptive`.
`AlwaysSplit` shows separate independent viewports whenever at least two
local participants are active, including while they are in the same room and
standing together. It never merges while two local participants remain.
One active local participant still uses the full viewport.

The policy changes only `LocalView` membership and placement. It does not
change seat claims, rollback participants, body motion, room residency,
network transport, or camera authority. Reuse `PresentsView`,
`ViewPlacement`, and the shared camera-lifecycle renderer. The local client's
choice must not change a remote peer's checksum. Keep `AdaptiveSplit`'s
thresholds and dwell times unchanged for now. Switching either way while two
players are alive must not reset the session or steal input.

### Blink: separate simulation completion and visual ease

The movement kernel's `handle_blink_clusters` starts the held aim state and
completes the blink on the release edge. The movement tick decays cooldown
and grace timers per body. The presentation path can arm each player's
`PlayerBlinkCameraState`, but its cleanup currently selects only the primary
body. This is a valid fix target without moving blink logic into a new host
special case.

- Tick presentation-only blink easing for **every** body with the component,
  once each applicable presentation frame. Retain the existing unscaled
  `WorldTime::wall_dt` policy for presentation clocks. Do not migrate a
  simulation-owned cooldown to wall time or mutate rollback state from UI.
- Test two bodies with independent timers and a control body with no blink.
  Test blink press, hold, release, post-blink grace, cooldown, room crossing,
  and view split/merge. Assert both kernel state (`BodyBlinkState`,
  `AxisManeuverState`) and presentation state reach their normal end.
- If only the visual timer remains armed, fix its decay at the existing
  presentation owner. If the simulation hold itself stays active, trace the
  per-seat press/hold/release latch through the same input bridge; do not
  diagnose a core-movement bug from the visual symptom alone.

### Friendly fire: peer-safe, participant-scoped damage policy

Set Ambition's co-op **default** to friendly fire enabled. Allow the session
host to turn it off. This option governs allied participant-controlled bodies
and appropriate owned projectiles, with self-hit exclusions kept explicit.
It must not silently change nonplayer ally damage or Versus room rules.

The engine already has `FriendlyFire` and `ResolvedRoomCombatRules`. Identify
where the current Ambition room obtains its resolved value before adding the
option. Specify precedence between the session co-op choice and authored
room/match rules, and test it. Keep damage eligibility distinct from AI target
selection: an allied player can be hit accidentally without becoming an NPC's
foe. Audit melee, ranged hits, area effects, body ownership after possession,
and any scripted hit path that bypasses the common combat relation.

Store the **effective gameplay rule** in the session's agreed/rollback-safe
configuration, not only in one client's `GameplaySettings` resource. Validate
rule identity across peers at session start, and define a deterministic
change boundary for changing it during play (or require a session restart).
Persist the local preference separately from the agreed session value. In
solo play the option has no second player target.

### Menus and pause: per-seat focus, explicit global pause aggregation

The immediate couch-co-op product rule is:

- Each player can open a menu in the viewport that currently presents that
  player. Its focus, cursor, page, scroll, pointer target, and close action
  belong to that player, not to one `InventoryUiState` singleton.
- While **at least one** local player's pause menu is open, the offline shared
  simulation pauses. Either player can open, navigate, and close its own menu
  during that pause. The simulation resumes only when the last pause request
  is removed. Closing Player 2's menu must not close Player 1's menu.
- The shared-camera case must assign clear, nonoverlapping UI presentation
  targets without creating an unwanted split of the gameplay simulation.
  The product may show two independent menu regions or temporarily allocate
  view regions; neither choice changes the participant-to-body relation.
- Each menu's gameplay actions must name its acting participant and flow
  through the typed action/intent authority. Participant-specific inventory,
  leave, and settings actions cannot use `PrimaryPlayer` as a fallback.

An explicit session pause aggregator should own the final local `GameMode`
request. Menu instances only assert or release their individual pause claim.
Also test a dialogue under one menu and a room transition under another, so
unrelated close events cannot unpause or suppress the other surface.

Ambition's kaleidoscope menu has a special 3D camera and full-window gate;
viewport-scoped menus need a per-view presentation design, render target, or
clipped alternative without one whole-window cube drawing over both panes.
The grid backend must obey the same menu-instance contract. Refactor the
shared menu *model* before duplicating render cameras.

**Netplay policy is a separate decision:** a client's local menu cannot freeze
remote peers on its own. Define whether that client continues to submit neutral
control frames, asks all peers for an agreed pause, or uses another explicit
session rule. Do not reuse the offline pause aggregator as the online
protocol without that decision.

### Room crossing: isolate cover and eliminate unrelated simulation stalls

The eager transition host currently avoids entering `GameMode::RoomTransition`
when another driven participant still plays, but the full-window black cover
remains. Therefore visual disturbance is source-confirmed even where the
simulation continues. Do not infer an actual sim stall only from a black pane.

- Give the transition presentation a subject/participant and target-view set.
  Scope its loading cover, loading text, fade, and readiness indication to
  the local viewport(s) following the crossing body. Preserve the other
  viewport's pixels and its camera. Rebind correctly if the view layout
  changes or the two players briefly share a view.
- Keep readiness room-specific (`PresentationSettling::pending_for` already
  accepts target rooms). Track which view waits for which room. Do not make
  one room wait for unrelated assets. The current `RoomTransitionLoadState`
  holds only one active transaction, so simultaneous independent crossings
  need an explicit queuing/parallelism policy rather than accidental override.
- Separate presentation cover lifetime, asynchronous host preparation, and
  committed simulation lifecycle. A player who waits for a door must not
  cause a scene-wide UI or `GameMode::Playing` request to override another
  player's pause/menu state.
- Test an actor crossing while another fights in a different live room:
  render two successive frames of the uninvolved viewport and assert it has
  no cover/fade, no camera reset, no black flash, and no control interruption.
  Test the same-room case and two near-simultaneous crossings in both orders.

**A distinct online blocker:** the existing peer lifecycle path uses a
synchronized freeze and `PeerLineage` session rebase for a room commit
(`ambition_platformer2d_rollback_ggrs::lifecycle_commit`). Its 2026-10-04
measurement reports **23–43 held simulation updates per door crossing** at
three-update link latency, including the new-session handshake. The freeze
holds **all rooms**, even if only one participant crosses. A viewport-only
cover cannot make that gameplay uninterrupted.

Do not remove the freeze as a quick fix: the existing two-peer tests prove that
committing at different unconfirmed states desynchronizes the peers. Design
and prove a **routine crossing without a global timeline rebase** (for
example, prepare immutable room plans off the sim clock and admit a
confirmed, deterministic lifecycle event at one agreed simulation boundary).
The design must state how peers agree on content hashes, readiness, event
ordering, rollback restoration, concurrent operations, and failures. Other
participants keep simulating until and through the event. Retain an explicit
global barrier for a true whole-session reset if required. A comparison
witness must run two real peer simulations through two-room crossings with
nonzero input and show identical world hashes, no freeze, and clean recovery
when a peer cannot prepare a room. Do not promise this behavior until those
tests pass.

### Debug view: facts are per room and presentation is per view

Both debug paths currently choose the primary room and issue unscoped gizmos.
A useful internal precedent exists: `avatar/trail.rs::present_player_trails`
uses retained `GizmoAsset`s with `TrailDrawing` and room-layer masks. The
same separation is suitable for view-visible collision shapes, player
hitboxes, camera frames, text labels, and portal diagnostics.

- Build diagnostic read models keyed by room and, where needed, body or view.
  A body-specific diagnostic must use that body's own `PresentedPose` and
  room geometry. A per-view camera box uses that view's camera snapshot.
- Present the facts through the selected local view's render layers. Support
  two cameras of **one** room (both may show the same room geometry with their
  own view-specific extras) and cameras of **different** rooms (no leakage).
- Preserve an explicit primary-seat developer-inspection mode if useful, but
  label it and draw it only in that seat's viewport. Never label global
  simulation geometry as P2's just because it appears in P2's camera.
- Add isolation tests for collision grids, hitboxes, HUD text gizmos, portal
  diagnostic wedges, and offscreen/retired room geometry, plus a rendered F1
  witness. Do not let a diagnostic pass modify gameplay or rollback state.

### Netplay gates and acceptance: catch ownership leaks before transport

Each new feature slice needs a deterministic headless test, a main-host input
or camera/UI test, and the relevant two-peer test. Keep recorded evidence
small and repeatable. Do not use lint as a substitute for the behavior gate.

| Gate | Required acceptance |
|---|---|
| Dialogue owner | Player 2 starts a conversation. Only Player 2's directions and Select change it. Player 1 still moves. Swap roles. Pause either player's own menu during dialogue. |
| Dialogue viewport | In split view, Player 2's panel and pointer hit targets are inside its own viewport. Player 1's pane has no dialogue overlay. A split/merge keeps the dialogue instance and selection. |
| View layout | Both `Adaptive` and `AlwaysSplit` work with the same session. Solo is fullscreen in both modes. AlwaysSplit never merges with two local drivers. View switching never adds a body or changes its authority. |
| Blink | Two bodies press/hold/release blink in overlapping time windows. Both sim maneuvers and visual timers terminate. Camera snap and pose are correct after room crossing and rollback. |
| Friendly fire | Both players damage each other by melee and projectile with policy ON; neither can with OFF. Self-hits remain excluded. AI target selection is unchanged. Room-rule overrides are deterministic on two peers. |
| Concurrent local menus | Player 2 opens its own menu first. Player 1 opens a distinct menu while paused. Both navigate and close independently; game resumes only after both close. Repeat with Grid and Kaleidoscope. |
| Room cover isolation | In two different rooms, Player 2 crosses a door. Player 1 keeps rendering and moving without a blackout, view reset, or pause. Reverse roles. Include one-room and two-crossings races. |
| Debug isolation | F1 grid/hitboxes and labels use each selected room and presented pose. Two cameras of the same room draw the right facts; different-room cameras do not leak. |
| Peer simulation liveness | A remote peer crosses a door while the other fights. Confirmed ticks and actions keep advancing on both peers; hashes agree, even after prediction, rollback, delayed or failed preparation. |
| No new singletons | A search of the changed consumers finds no new `PrimaryPlayerOnly` use for a per-participant timer and no new window-wide UI root for a participant-owned surface. |
| Performance | Compare solo, shared two-player, adaptive split, and fixed split on desktop and Android. Measure view render cost, menu cameras, transition preparation, and peer rollback cost without a forced 60 Hz sim tick. |

**Implementation order:** (1) owner-scoped dialogue input and view-scoped
presentation; (2) blink timer symmetry and combat-session policy; (3) explicit
layout mode; (4) per-seat menus and pause aggregator; (5) view-scoped door
presentation and per-view gizmos; (6) the peer-lifecycle no-global-hold
protocol, after the peer-consistency proofs. The queue records exact slices
and acceptance. These are dependencies and review gates, not separate queues.
