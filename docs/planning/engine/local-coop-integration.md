# Local co-op integration: input, bodies and adaptive multiview

**2026-10-26 multiplayer scope:** Correct participant/seat/body/view identity is necessary but
not sufficient for remote peers. Local co-op can still share one simulation; remote Ambition
players in independent rooms need region interest, an explicit authority model and room
transfers that do not hold unrelated peers. Loading covers and dialogue input are scoped to the
initiating participant/view where possible. See [architecture
reassessment](architecture-reassessment-and-composable-multiplayer.md).
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

**Join placement built 2026-10-28 (`1f0956abb`):** a join no longer puts
its body on the primary body. The common body recipe supplies its standing
footprint, including the motion model's terrain clearance. Placement checks
16 fixed nearby positions against the live room's composed terrain and its
occupied body boxes. It does not copy the primary body's passage permissions.
If all positions are blocked, no body is built; another Start press can retry.
Guards: the six nearby-placement tests in `rooms/spawn.rs` and
`a_seat_with_no_body_joins_beside_the_primary_on_a_start_press` (including
rollback). The app compile, 168 session tests and 35 workspace policy tests pass.

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
- `crates/ambition_platformer2d_session/src/join.rs` — sim join.
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
| Player 2 cannot drive dialogue that Player 2 opened | `InputOwner` projects from the conversation to `DialogState` and `DialogView`; input reads the accepted owner frame from `SeatMenuFrames`. | **Fixed in `94d8822f9`**; real controller tests cover same-room and separate-room dialogue, including navigation and close ownership. |
| Dialogue covers the entire display | Both presenters use `ParticipantSurface` to resolve the owner view and clip the root. `DialogPointerRegion` uses the same rectangle. | **Fixed in `94d8822f9`**; split/merge tests and a real GPU capture check bounds. This does not close other UI or transition surfaces. |
| Always-split view mode | `VideoSettings::local_view_layout` selects `Adaptive` or `AlwaysSplit` through the shared settings menu. The existing view and camera owners apply it. | **Built 2026-11-01** (`751ad0b34`). Controller and GPU tests cover live switching, retained session/body identity, separate rectangles and drawn panes. |
| Player 2 stays in blink state | Every `PlayerBlinkCameraState` advances on wall time. Every preview reads committed seat controls through the driven body. | **Fixed in `0754ed273`**; the two-seat simulation test covers press, hold, release and completed cooldown. A separate paused-clock test covers presentation decay. |
| Player strikes do not hurt the other player | `FriendlyFire::default().enabled` is false. Both driven bodies resolve to `ActorFaction::Player`; `damage_lands_between` allows same-faction damage only when friendly fire is enabled (unless another relation overrides). | **Confirmed default rule explains allied damage rejection**. Prove Ambition's actual resolved room rule and all attack families. The explicit `friendly_fire: false` in `game/ambition_app/assets/data/room_rules.ron` is a **Versus** stage declaration, not the general Ambition room default. |
| Only one player can use the pause/menu | Inventory opening and navigation read each menu owner's `SeatMenuFrames`. Open, focus, page, scroll and cache state belong to menu entities, but the frontend still creates one surface. `ShellPauseMenu` supports an owning seat but Ambition suppresses it and supplies its own menu. | **Confirmed singleton Ambition menu**. The shell seat rule is not two independent menus. |
| Either player's door flashes black over both panes | Transition covers and loading foregrounds now use `ParticipantSurface`. Loading input claims name the affected participant. | **Fixed in `e36b067fd` and `c89f0b828`**. Both crossing orders and separate-room GPU witnesses pass. Peer commits still hold the whole timeline. |
| F1 gizmos show the wrong room or offset | Both presenters use `RetainedDrawings` with native room/view ownership. Body queries are read-only. Strikes use their emission room; the primary inspector is labelled and clipped to P1. | **Fixed in `0e0099f0e`**. Shared-room and separate-room GPU guards cover grids, labels, portal diagnostics and private camera frames. Unit guards cover handle reuse, credit retirement and room retirement. |
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
`ConversationInstanceId`, and `InputOwner`. Its projection into
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

`VideoSettings::local_view_layout` owns one `LocalViewLayout` value:
`Adaptive` or `AlwaysSplit`. The default is `Adaptive`.
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
and grace timers per body. The presentation path arms each body's
`PlayerBlinkCameraState`. Its cleanup now decays every body's presentation
timers from wall time (`0754ed273`). The simulation still owns release,
cooldown and grace.

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

Implemented core: `0a4bdab1b` (schema 395). Remaining work is listed below.

Ambition's saved co-op choice defaults to ON. The settings menu can turn it
OFF for the next session. Preparation copies the choice into the immutable
`ParticipantFriendlyFire` on the session root and includes it in prepared
content identity. A preference edit does not change a live session or a
content reload. A new session reads the current preference.

The rule governs allied driven bodies in Ambition's untagged rooms. NPC ally
damage retains the base policy. Tagged rooms and match teams retain authored
rules. `ResolvedCombatTuning::damage_policy` projects these facts through the
common damage relation. Damage permission does not change AI target selection.
Contact owners keep explicit self-hit exclusions; an environmental hazard has
its own self-damage contract.

Combat owns `AttackAllegiance`, shared by projectiles and detached strike
volumes. It captures faction, participant and team before contacts or owner
retirement. Reflection replaces the capture with the interceptor's side.
Do not infer a launched attack's side from its owner's current control.
Detached player areas use the common body resolver. Their feature pulse has
separate rollback state; a body's hit list is not a pulse sentinel. Schema 395
records these changes.

Guards: `coop_friendly_fire` in `app_it` covers both melee directions, ON/OFF,
projectiles and detached areas after control or owner loss, frozen session
identity, and peer refusal for different effective choices. The combat policy
test covers NPC allies, possessed factions, grudges and match-team precedence.
The runtime projection test covers two live rooms with different rule scopes.

Damage-box emission now captures room, session, round and allegiance before
source retirement (`753adeda8`, schema 396). Queued and immediate construction use one
factory. Unknown blast credit requires no temporary body. The two-room guard
`a_damage_box_keeps_its_emission_room_and_session_after_its_source_leaves`
covers owner loss, credit in another room, damage and room retirement.

Scripted strikes and joined-player damage landed in `69f705d8c` (schema 397).
`StrikePort` uses the scoped hitbox factory and the common body resolver.
The shared player damage consumer serves both joined bodies and reads each body's
seat for transition holds. Guard: `scripted_strikes_use_body_contacts_for_either_participant`.
It covers both seats, ON/OFF and separate live rooms.

Projectile splashes landed in `4f47d98a5` (schema 399). The common strike
factory keeps the shot's launch allegiance, hit cause and firing move after
control loss or firer retirement. Guard:
`projectile_splashes_keep_launch_allegiance_and_reach_either_participant`.

Inventory bombs use scoped environmental strikes (`3d6651f5a`, schema 400).
Resolved self-hits now reach both body consumers. Guards:
`inventory_bombs_hurt_either_participant_only_in_the_blast_room` and
`an_environmental_strike_can_hurt_its_credit_body_in_either_seat`.
Shark charge crashes name only the crashing body.

Sliding shells use scoped environmental strikes (`c7a8a04c4`, schema 406).
Kick grace excludes only the kicker; credit does not change the emission room.
Body targets name one recipient, and body consumers do not scan unresolved areas.
Guards: `a_snake_is_each_players` and
`an_excluded_sweetspot_cannot_suppress_an_allowed_sibling`.

Rendered acceptance passed in `6d9e70de3`. Both pads drive melee and
projectiles through raw input events in the main GPU host. ON damages the
other body; OFF permits attack emission but prevents body contacts and damage.
Neither attack hurts its source. Guards:
`two_pads_damage_each_other_in_the_rendered_game` and
`two_pads_spare_each_other_in_the_rendered_game`. Both captures show split
views and the expected health values. The dialogue and fixed-split GPU cases
also pass.

Broad verification remains tracked by TEST-LANES; focused passes do not close it.

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

Grid and Cube use one menu lifecycle (`47da81062`). The shared route owns
entry, close and platform pause. Guard:
`a_platform_pause_opens_the_grid_and_keeps_it_open`, with the existing menu
and dialogue cases. Do not restore backend-specific game-mode writes.

Item use and ability equipment requests name the acting participant
(`5549b8402`, schema 407). Simulation resolves its current controlled body.
An absent body cannot spend inventory. `BodyHealRequested` requires a target;
pickups and parries cannot fall back to the primary body. Guard:
`item_requests_restore_and_equip_the_joined_participant`, with consumable
rewind and projectile parry cases. These host intents remain local ingress;
peer agreement still needs the session input protocol.

Held-item menu actions select the acting participant's body (`1cf0a124a`).
Missing or ambiguous bindings change no hand. Portal-gun pairs stay with that
body. Guard: `held_item_actions_use_the_named_participants_body`, with the
missing-driver and portal-pair cases. These mutations still run in host Update;
stamped held-item intents and independent menu presentation remain open.


Inventory open state now belongs to a menu entity with an explicit
participant (`a27074b3f`). Input claims affect only that owner. One pause arbiter
holds offline play until the last menu closes, restores the underlying mode,
and clears claims on session retirement. Guards:
`each_inventory_claims_only_its_owner_without_a_game_mode` and
`only_the_last_menu_close_restores_the_underlying_mode`, with transition-mode
and retirement cases. The host dialogue-under-menu test also passes.
Pending mode requests survive the pause step (`7711a82cc`); guard:
`a_mode_request_in_the_open_frame_is_not_replaced_by_menu_pause`.
Do not add a resource mirror.

Page models now live on their menu entity (`b972851eb`). The Cube ring uses
that entity and stores its own publication cache. It reconciles only its child
faces. Guards: `each_menu_reconciles_only_its_own_published_faces` and
`a_delayed_ring_renders_its_latest_publication_without_rebuilding_another_menu`.
The production open-menu capture also draws the faces, controls and highlight.


**Netplay menu policy:** a local menu does not pause a remote timeline
(`a27074b3f`). Its input context captures only its participant, so the existing
router submits neutral gameplay controls for that participant. Guard:
`a_local_menu_does_not_pause_a_remote_timeline`. This is local pause policy;
it does not prove peer agreement for inventory mutations or lifecycle changes.

### Room crossing: isolate cover and eliminate unrelated simulation stalls

Per-view covers, loading foregrounds, activities and input claims landed in
`e36b067fd`. Both participant orders pass the same-room input and GPU guards.
Opaque regions control the upload budget by their union. Stable body identity
orders simultaneous passage admission; a second accepted crossing waits until
the committed destination is presentable. Do not restore global room covers.

Separate-room GPU acceptance and both queued-crossing orders passed in
`c89f0b828`. The other participant retains its camera and gameplay input.
Layout changes and view retirement use the shared surface resolver. The peer
hold below is a separate simulation protocol.

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

Both debug paths now use native room/view ownership (`0e0099f0e`). Trails share
the same retained drawing path. It reuses entities, handles and vertex buffers.
Room geometry and labels use their own room; camera frames use their own view.
Body diagnostics read components without mutation and use each body's presented
pose. Strike scope follows the live strike, including after credit retirement.

The primary inspector is labelled and clipped to P1's viewport. Shared-room and
separate-room GPU guards cover grids, hitboxes, labels, portal diagnostics and
private camera frames. Unit guards cover room retirement, handle reuse and
turning the overlay off. Do not restore primary-room or unscoped F1 drawings.

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

Menu events now name their menu entity (`4d4ddf89c`). Press, confirmation,
preview and scrollbar state belong to that entity. Grid, Cube and shell
consumers reject events from other menus. Both renderers share one drag reader.
Guards: `equal_row_keys_in_two_menus_keep_independent_press_arms`,
`a_destructive_confirmation_in_one_menu_cannot_confirm_another_menu`,
`two_menu_drags_survive_widget_rebuilds_and_release_independently` and
`foreign_menu_messages_cannot_drill_turn_or_scroll_this_menu`.
Do not restore global gesture state or events with no owner.

Menu equipment, actions and maps use the owning participant's current body
(`69de9c2cd`). Cells, details and confirm hints share the body's inventory view.
The Grid map updates when that body changes live rooms. Guards:
`either_backend_uses_the_menu_owners_current_body_and_hand`,
`each_menu_map_follows_its_controlled_body_between_live_rooms` and
`an_open_grid_map_follows_the_owner_without_a_map_resource_change`.
Missing or ambiguous bindings have no body; do not select the primary body.
Held-item mutations still need stamped intents before peer agreement.

Navigation, quality confirmation, rebind arms, scroll state and caches now
belong to each menu entity (`d3ea64d9b`). Both backend dispatchers resolve the
menu named by each event. Cube focus and text use each node's menu owner.
Each menu has its own publication key. Guards:
`two_menus_dispatch_drill_quality_rebind_and_close_independently` and
`each_menu_caches_and_publishes_its_own_system_page_without_idle_churn`.
The production equipment capture also passes visual review. Do not restore
resource copies of menu state.

Inventory opening and directional navigation read the menu owner's
`SeatMenuFrames` (`fe46de32c`). Start latches belong to menu entities. Consuming
one seat's navigation leaves other seats and the shared shell frame intact.
Touch drags also reach the primary seat's menu frame. Guard:
`two_seats_open_navigate_and_close_their_own_menus_with_one_pause` covers both
backends and release of the last pause claim. The production capture still
draws the equipment menu. Do not restore a global input fallback.
Keep a joining seat's Start press distinct from opening its menu.

Cube folds, material fades, camera gates and scrims follow each menu owner
(`10541009d`). Animation changes only that ring's faces. Material fades resolve
ownership through the scene tree and retain the settled fast path. Picking
checks each active camera and its owner's controls inside its viewport.
Guards: `a_settled_ring_does_not_stop_another_menus_fold`,
`each_menu_fades_only_its_own_descendants` and
`each_cubes_camera_and_scrim_follow_its_own_fold`. The production equipment
capture also passes visual review. Do not restore a global fold state.

Menu scenes now have private render layers and participant regions
(`c6c696a66`). Dialogue and menus share the `ParticipantSurface` resolver.
Late cube scenes inherit their own layers and release their cameras with the
menu root. Narrow regions change field of view, not camera position. Guards:
`late_menu_scenes_inherit_private_layers_and_release_their_cameras` and
`adjacent_menu_viewports_share_one_rounded_edge_at_fractional_scale`.
The GPU guard `two_menu_cameras_draw_only_their_own_scene_into_their_regions`
(`--ignored`) checks pixels from two four-face scenes. The production equipment
capture also passes visual review. Do not restore world-layer cube scenes or
camera-distance fitting.

Grid publication now rebuilds and closes only the owning menu's tree
(`71ac19417`). Late menus receive scrims. Grid roots, scrims and Cube cameras
share a linked lifetime with their menu. Guards:
`grid_trees_rebuild_close_and_retire_with_only_their_own_menu` and
`late_menus_get_scrims_and_retire_without_removing_the_display_camera`.
The production equipment capture passes visual review. Do not restore global
Grid teardown or startup-only scrims.

Wheel input now targets the visible menu under the mouse in its display window
(`e9600e15f`). Both backend readers use each menu's state. The guard
`wheel_scrolls_only_the_visible_menu_under_the_mouse` covers both backends,
adjacent region edges and hidden menus. The Cube guard also covers a second
participant's drag. Do not restore a global wheel target or a primary-menu
fallback.

Rebind rows now show each owner's live bindings (`4692366bb`). Capture reads
only that participant's assigned devices and edits that profile. The primary
profile persists in settings; other profiles stay on their participants. Both
frontends use one cached System model and one action consumer. Guards:
`rebind_capture_follows_the_owners_devices_and_changes_only_that_profile`,
`each_menu_publishes_and_refreshes_its_owners_rebind_rows` (both backends) and
`a_backend_change_applies_once_and_the_next_action_uses_the_new_frontend`.
Do not restore global capture or per-backend action consumers. The current
two-pad admission and capture guards are below.

Grid text fits its menu region and restores its authored maximum (`c10ff5c21`).
Guard: `text_fits_its_menu_region_and_recovers_without_changing_another_menu`.
Grid uses the host font; both renderers receive owner detail text (`72af9be68`).
Production Rebind and equipment captures pass visual review. Do not restore
fixed text sizes or a Cube-only detail provider.

Grid gestures now use picking and stable menu owners (`e3c8824da`).
Guards: `motion_and_release_follow_the_finger_that_armed_each_menu`,
`tab_gestures_follow_their_own_pointer_across_menus` and `a_raw_release_uses_the_pressed_bounds_after_a_widget_rebuild`.
Rendered one-frame touches open Rebind and select Map. Do not restore global
pointer selection or window-only activation.


Session admission now uses `SessionParticipants` (`0d184ce24`). Membership
survives loss of a body assignment and returns with rollback. Guards:
`start_does_not_readmit_a_participant_without_a_current_body_assignment` and
`rewinds_across_a_two_human_activation_and_reconstructs_both_seats`.
Do not derive admission from offered devices or current bodies.

The shared touch confirm prompt follows the primary menu (`e5c60f897`).
Guards: `the_shared_prompt_follows_the_primary_menu_and_clears_when_it_retires`
and `an_inventory_context_gets_its_verb_without_a_simulation_tick`.
Cue publication uses the frame clock and the inventory input context.
Do not let another participant's menu change this prompt.

Local menus now follow session admission and local handles (`a3d838d1c`).
The join press stays separate from opening a menu. Rebind capture consumes only
its owner's menu frame, including Inventory and Map shortcuts. Guards:
`two_pads_join_open_and_close_their_own_menus_on_both_backends` and
`participant_and_session_retirement_remove_linked_presentations`.
Do not derive menu lifetime from device or body presence.

Cube hover now uses pointer identity (`499fa1a51`). Guard:
`another_participants_pad_does_not_block_mouse_focus_in_a_cube_menu`.
Another participant's pad must not block mouse focus in a menu.

Slice 4 is complete (`57462d561`, `1861a88b3`): narrow menus use their own
canvas, shared inventory layout, and page-relative text. Item details fit the
panel. Guards: `each_layout_publishes_its_item_details_inside_the_panel` and
`two_pad_menus_draw_in_their_own_regions`, with both captures reviewed.
Do not restore window-relative page text or fixed navigation dimensions.
