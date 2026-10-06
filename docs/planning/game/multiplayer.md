# Ambition multiplayer

**State:** OPEN product plan. Ambition remains the main game.

This document states Ambition-specific multiplayer product intent. Reusable
participant, networking, world-residency and multi-view machinery belongs in
[`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md).

## Product direction

Ambition should be architecturally capable of multiplayer without turning the
single-player game into a separate implementation.

A participant may be local or remote and controls an ordinary body. The body
owns its capabilities/inventory; possession changes control authority rather
than changing body ontology.

Desired play configurations include:

- solo, unchanged;
- local couch co-op;
- online co-op;
- mixed local + online co-op;
- shared-screen play when the party can be framed coherently;
- fixed split-screen;
- **adaptive split-screen** that separates when participants diverge and merges
  when they regroup;
- participants occupying different rooms when the game mode permits independent
  exploration.

A mode may instead impose party cohesion and prevent independent room travel.
That is product/ruleset policy.

## Presentation behavior

The preferred default for ordinary co-op is potentially BG3-like:

1. when controlled subjects are close and one camera can frame them well, share
   one view;
2. as framing becomes poor, smoothly split into independent local views;
3. if participants cross into different rooms, split is mandatory unless one
   participant is intentionally following/spectating;
4. when compatible views regroup, merge with hysteresis so the screen does not
   chatter between states.

Exact thresholds, animation and layout are feel work. The engine needs only the
policy hooks and view-index model.

## Room and world consequences

Different-room play means Ambition cannot assume one globally active room.
Eventually the session must support at least the minimal case of two resident
room/active-area partitions with separate controlled bodies, while shared world
state, quests and persistent objects remain coherent.

The implementation should grow from real two-participant needs rather than
attempting a fully general open-world streamer in the first slice.

## Game-state questions to answer during implementation

These are product questions, not reasons to block the engine architecture:

- which story interactions pause only one participant versus the whole party;
- how dialogue choices work when participants are in different rooms;
- whether critical quest transitions require party regrouping;
- rejoin behavior when another participant remains alive elsewhere. Death is
  decided (Q151, 2026-10-03): an ordinary death is local to the dying
  participant and the affected room, and another participant's live room and
  accomplishments stay; only an explicit whole-session reload rewinds
  everything. Ambition seats a second player on a Jump press of a second
  pad that was connected when the session started (the Q153 default, in
  force until it is ruled);
- inventory transfer/trading rules between controlled bodies;
- save ownership and join/leave policy for remote participants;
- how far shared quest/world causality extends when players explore separately.

Use concrete Ambition content to answer them rather than inventing a universal
multiplayer narrative framework up front.

## Incremental acceptance path

The engine side is owned by
[`open-world-runtime-and-residency.md`](../engine/open-world-runtime-and-residency.md)
(OW1 and the view cuts V1–V5). Witnesses for A1–A3 are in
`game/ambition_app/tests/two_players_two_live_rooms.rs` unless stated.

### A1 — two local participants, same room, shared view

✅ Built. Two participants keep independent input, attacks, safety anchors and
heal routing (`multiplayer_smoke_tests.rs`). Shrines heal every driven body;
each body rests only at a shrine in its own live room; the checkpoint stays one
fact, written by the first resting body in seat order
(`two_driven_bodies_resting_at_a_shrine_both_heal_and_write_one_checkpoint` in
`shrine/tests.rs`).

`PrimaryPlayerOnly` (`(With<PlayerEntity>, With<PrimaryPlayer>)`) is still used
in production. Most uses are in single-player tools, the sim harness and the
demos, where one player is correct by construction. When you touch a shipped
simulation site that uses it, ask whether the fact should be per participant.

### A2 — adaptive split in one room

✅ Built (default feel, Q162 in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md)). A
game that declares `AdaptiveSplit` (`ambition_sim_view`; Ambition declares it)
opens a view for each seat when a seat's body in seat zero's room leaves 0.9 of
half the shared view's visible size. The views close when every body has been
within 0.5 of that size for 1 second. The merge measures by the size of the
shared view when the split opened, because the column views are narrower. A
match that frames a declared cast (`FramedCast`), or a composition that placed
its own views, does not split by distance. Witness:
`a_second_view_opens_when_the_players_drift_apart_in_one_room_and_closes_when_they_regroup`
in `the_screen_splits_when_two_players_drift_apart.rs`.

Not done in a one-room split: the two views share the through-portal windows
of the FIRST view. A live room has one eye (`PortalViewers`: the body of the
first view, by view id, that frames the room). The window of a portal is the
wedge that eye sees, and a body behind a pane is cut by where that eye is, so
the second view sees the first view's wedge and cut. A window for each view
needs a capture rig, a window mesh on a per-view layer and far-side pieces for
each view. Before 2026-10-06 the two views shared the windows of the primary
seat, so this is not a regression; it is now written down.

### A3 — two rooms resident

◐ Simulation built; view mostly built.

- Simulation: each seat crosses its own doors. A door one player crosses opens a
  second live room (`LiveRooms`) and leaves the other player's room live. A
  player who comes back joins the room the other holds. Encounters, bosses,
  switches, shrines, chests and items run in their own room, and a carried item
  crosses whole. One player's door load or conversation does not stop the other
  player's room. A GGRS sync test with two seats resimulates to the same world
  (`two_players_in_two_live_rooms_resimulate_to_the_same_world`).
- View: each view frames its own player in that player's room; each camera
  draws only its own room's visuals, sprites, items, projectiles, effects, LDtk
  levels, parallax, body-riding visuals and portals. A second view opens while
  the players are in two rooms and closes when they meet
  (`a_second_view_opens_while_the_players_are_in_two_rooms_and_closes_when_they_meet`).
- The through-portal window, built 2026-10-06: each live room that a view
  frames has the windows of its own portals, made for the body and the camera
  of that view, and a camera draws the windows of its own room only. Before,
  measured: the portal pair in the second player's room had no window ("4
  portals in 2 rooms, rigs=2"), and the window of the first player's room was
  on a layer that each camera draws (cone layers `[5, 520]`), so it was drawn
  in the second player's view at the same coordinates.
  - `PortalViewer` was one resource. It is now `PortalViewers`: one eye for
    each live room, published by the host from each local view's body
    (`sync_portal_viewer`; the first view by id that frames a room is its
    eye). The capture rigs are by (live room, channel).
  - A window mesh is stamped into its room. `isolate_live_rooms` moves the
    window layer of a stamped window to a window layer of its room, and a
    camera adds the window layer of the room its view frames. It is not the
    room band: a capture adds that band to see its room, and would capture
    its own window.
  - The host camera sample was one resource too ("the last writer wins").
    `camera_follow` now also records it for each view (`PortalObserverViews`),
    and the windows of a room are clipped to, and captured for, the camera of
    the view that is its eye.
  - A body behind a pane is cut by the eye of its own room.
  - Witnesses (system fixtures; the capture rigs do not run headless, so no
    shipped-game arm):
    `each_live_room_that_has_an_eye_has_the_windows_of_its_own_portals` and
    `the_windows_of_a_room_are_made_for_the_camera_of_its_own_observer`
    (`ambition_portal2d_presentation` `view_cones/rig_tests.rs`),
    `each_camera_draws_only_the_windows_of_the_live_room_its_view_frames`
    (`ambition_render` `view_isolation.rs`),
    `each_live_room_a_view_frames_has_the_eye_of_that_views_body`
    (`ambition_platformer2d_host` `portal.rs`),
    `the_camera_sample_of_each_view_is_recorded_for_that_view`
    (`ambition_render` `camera.rs`).
  - Not done: while two rooms are live a window shows no other window (no
    recursion): the layer of a portal's own window is by channel, and two
    rooms can hold one channel. The sky copies of a capture are also by
    channel, and are copied from the root view's sky: two rooms that hold one
    channel put two skies in each capture of that channel, and the window of
    the second room shows the first room's sky. Two views of ONE room share
    the windows of the first view (see A2). The portal camera continuity is
    still one screen anchor for the process. The developer dump and overlay
    of the windows describe the first eye only.
- Not done: debug overlays read the sole live room; the banner is
  session-wide and follows the primary seat. The Q150 ruling (2026-10-03, in
  [`../maintainer-decisions.md`](../maintainer-decisions.md)) makes the HUD
  per participant and the local music an authored-priority choice with the
  primary participant as the tie-break. The music is built. Of the HUD,
  the built-in vitals HUD is per view: each local view shows the meters of
  the body it follows (`ViewHudFacts`), in its own column
  (`each_view_of_the_split_shows_its_own_participants_purse`), and each other
  seat on a shared view has its own HUD on it
  (`bob_beside_alice_has_his_own_hud_on_the_shared_view`). While two or
  more HUDs are on the screen, each says whose it is, as "P1", "P2" (the
  seat that drives the body it shows, `ViewHudSeat`; one HUD says nothing,
  because it can only be the player's own; `stacked_huds_say_whose_each_is`).
  Open: the declared readouts (`HudReadouts`: Mary-O's coins, Sanic's rings)
  are one per session; and an online peer shows every
  seat on its shared view, since only A4's client-local layout knows which
  seats are its own.
- A conversation holds only the talker, also with two players in one room
  (ruling 2026-08-06; the one-room case is Q163 in
  [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md)):
  the session enters the dialogue mode only while one seat drives a body and
  one room is live. Before 2026-10-06 the mode was entered in one room, so a
  second player beside the talker moved but could not pick up or take a door
  (`a_conversation_holds_only_the_talker_with_two_players_in_one_room`).

### A4 — online participant

▢ Feed a remote participant through the same intent/control seam. Keep the local
view layout client-local.

✔ Measured 2026-10-04: what a room crossing costs a remote player. The rebase
does not use the rollback window: under a peer session the crossing freezes the
simulation, each peer commits alone on the frozen world, and each peer starts a
new session at frame zero (the "Remote peers" row of
[`open-world-runtime-and-residency.md`](../engine/open-world-runtime-and-residency.md)).
The cost is the freeze, for each peer and each live room:

    freeze = (confirmed frame reaches the freeze frame, and readiness)
           + (the handshake of the next session)
           = (2 to 7 updates) + (21 to 36 updates)
           = 23 to 43 updates = 0.38 s to 0.72 s at 60 Hz

at a link latency of 3 updates each way (`two_peers.rs`, four runs, two
crossings). The handshake is five GGRS round trips and is most of the cost.
Whether that hold is acceptable is Q155. Named remainders: a link that loses
parcels needs a linger before the old session ends, and the freeze is of the
whole world, also of the room that nobody leaves.

### A5 — mixed local + remote party

▢ Prove the architecture does not assume one input device per machine, one local
participant, or one view per participant.

## Relationship to other games

TwinTrack is the strongest acceptance customer for independent views/reference
frames. Smash is a strong N-participant combat customer and may become a
first-class game, but its arena rules usually keep fighters together and
therefore do not prove Ambition's multi-room requirement.
