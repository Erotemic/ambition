# Ambition multiplayer

**2026-10-26 maintainer direction:** Ambition multiplayer should let players split up across
rooms, coordinate in chat, search/explore independently and receive benefits for regrouping; a
company-operated central server must never be required for play. A peer may temporarily host the
world, but the concrete authority, host migration, isolation and replication protocols remain
open. Per-room rollback is a candidate only where causal boundaries can be enforced. See [engine
reassessment](../engine/architecture-reassessment-and-composable-multiplayer.md) and [ADR
0035](../../adr/0035-distributed-multiplayer-without-required-services.md).
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

The default for ordinary local co-op is BG3-style adaptive split/merge:

1. when controlled subjects are close and one camera can frame them well, share
   one view;
2. as framing becomes poor, smoothly split into independent local views;
3. if participants cross into different rooms, split is mandatory unless one
   participant is intentionally following/spectating;
4. when compatible views regroup, merge with hysteresis so the screen does not
   chatter between states.

Exact thresholds, animation and layout are feel work. The engine needs only the
policy hooks and view-index model.

### Local co-op behavior decided in the 2026-10-24 playtest

These decisions apply to Ambition. Generic presentation and input contracts
remain in [`../engine/local-coop-integration.md`](../engine/local-coop-integration.md).

- **Layout choice:** `Adaptive` remains the default. Add `Always Split`: with
  two joined local participants, each always has an independent half-screen
  gameplay view, even while standing together in the same room. The views do
  not merge in this mode. With one joined participant, the viewport is full.
  Changing this setting must not rebuild gameplay or change input claims.
  Keep split-distance feel tuning for later.
- **Dialogue:** the participant who starts a conversation drives its choices.
  When two local views are split, display the dialogue only in that
  participant's viewport; on a shared view, show it once and identify the
  controller. The other participant can continue playing. A single live
  conversation per session is the current limit; multiple independent story
  dialogues remain a separate decision.
- **Friendly fire:** local co-op defaults to **ON**. Allow the host to set it
  **OFF**. Player-caused attacks can hurt the other controlled player when on,
  subject to ordinary self-hit, invulnerability, and attack rules. Do not
  change AI hostility or authored Versus rules as a side effect. All peers
  must agree on the active damage rule.
- **Pause menus:** each local participant can open, navigate, and close its
  own menu in its own view. The offline game stops simulating while one or
  both menus are open. If both are open, each can still operate its menu and
  the simulation resumes only when both are closed. In a shared view, menu
  layout must keep each owner's controls distinct. This policy is provisional;
  the online pause rule is **not** decided and cannot let one client stop all
  peers without a shared protocol.
- **Room crossing:** a participant's routine door or loading-zone transition
  must not black out, cover, reset, or interrupt an unrelated participant's
  viewport, controls, or gameplay, even when they are in different rooms.
  With shared views, the presentation must separate the affected view before
  showing a private loading cover, or use another presentation that does not
  cover the other player's image. Ordinary GGRS peer room travel now commits
  on the shared input timeline without a global hold/rebase (2026-11-04).
  The remaining acceptance covers broader peer/feature verification,
  distinct checkpoint/rebuild barriers, and real external transport; see
  [peer room commits](../engine/peer-room-commit-protocol.md).
- **Debugging:** F1 diagnostics must use the viewed room and participant, not
  the primary player's room. Developer-only graphics must obey the same
  room/view isolation rules as gameplay graphics.

These are product rules, not a request for `player2` variants of engine
systems. The executable work is the `MULTIPLAYER-SEAMS` row in
[`../queue.md`](../queue.md).

## Configurable party travel

**Implemented 2026-11-04 (PARTY-TRAVEL-POLICIES).** Independent exploration
with adaptive split/merge remains the default, and `Always Split` is a separate
view option. The optional leader-following view, out-of-view return prompt and
semantic Select return use shared bindings. A return preserves the controlled
body, inventory and capabilities. The rules below remain the authoritative
product contract for regression and future extension.

Keep view choice and travel rules separate. A game can configure these travel
rules for a group:

- **Independent:** each member crosses alone.
- **Leader:** the designated leader brings the group. Other members cannot
  activate a loading zone. A forbidden walk or edge exit blocks that member.
- **Any member:** a member who activates a zone brings the group.
- **All members:** each required member must agree to the same source zone and
  destination. Moving away cancels that member's agreement. Conflicting zones
  do not form an agreement. No member is held in an entry animation while the
  group is still deciding.

The favored-primary preset selects the leader view and leader travel. Other
games can select another combination through their own data and rules. A party
role is explicit. Do not infer it from camera presence or device order.

A group crossing is one admitted operation. Prepare the destination once and
transfer all selected bodies through the shared room application. Validate the
complete transfer before applying it. Keep body and item ownership, room
residency, safe arrival placement, and confirmed effects on the common path.
Use rollback state for group agreement and cancellation. Peers must agree on
the mechanical travel policy; local camera geometry must not decide it.

The return action must not also fire the existing reset action on Select.
Participant-scoped prompts must identify the correct binding. Cover joins,
leaves, possession, a member in another room, and a blocked arrival. A policy
change must not recreate bodies or lose held items.

Acceptance: default independent travel still works; the leader view stays
shared; return moves only its requester; leader and any-member travel move the
selected group once; a follower cannot cross a forbidden edge; all-member
agreement cancels on departure; conflicting exits do not commit; replay and
peer confirmation preserve the same group, room state, and item ownership.

The completed [PARTY-TRAVEL-POLICIES receipt](../queue.md#party-travel-policies--configurable-group-crossings-and-a-leader-view)
records the implementation. Do not requeue its acceptance as if it were unbuilt.

### Gate connection lifetime

Maintainer request, 2026-11-03. A dialed gate connection must work in both
directions. A traveller must not close it behind the next participant. Close
both ends only through a dial-device hangup or after the shared idle timeout.
Active passages and pending crossings keep the connection open. This rule also
applies in single-player. The default idle timeout is 30 simulation seconds;
the gate catalog can set another positive duration.

## Room and world consequences

Different-room play cannot assume one globally active room. Ambition already
supports multiple live room roots and separately controlled bodies. Persistent
world facts, item custody and ordinary room crossing use shared session
semantics; **remote replication, independently rewound regions and late joining
remain separate unimplemented network capabilities**. Extend from concrete
customers rather than building a general open-world streamer by default.

## Game-state questions to answer during implementation

These are product questions, not reasons to block the engine architecture:

- which story interactions pause only one participant versus the whole party;
- how dialogue choices work when participants are in different rooms;
- whether critical quest transitions require party regrouping;
- rejoin behavior when another participant remains alive elsewhere. Death is
  decided (Q151, 2026-10-03): an ordinary death is local to the dying
  participant and the affected room, and another participant's live room and
  accomplishments stay; only an explicit whole-session reload rewinds
  everything. Amended 2026-10-19: only a full-party wipe (every active
  participant dead) rolls the session back to the checkpoint; a death beside
  a living participant resets nothing. Later on 2026-10-19 (Q153): the
  default is independent players in one persistent world, and each
  participant respawns by the single-player respawn-location policy, for
  itself; respawning beside a living partner is an optional, configurable
  policy (queue rows COOP-RECOVERY, PARTNER-RECOVERY). Joining is ruled (Q153, 2026-10-19):
  solo by default, with every device on Player 1; an opt-in setting accepts
  a second player through an explicit join gesture, and a menu action
  removes them (queue row LOCAL-PLAY-MODES);
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

✅ Built. Adaptive multiview is the ordinary co-op default (Q162, 2026-10-19);
the thresholds below are engineering tuning. A
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
- **Through-portal windows:** each live room has its own portal eye
  (`PortalViewers`) and per-view camera sample (`PortalObserverViews`); portal
  meshes and capture rigs isolate windows to their room. Separate-room
  system witnesses cover eyes, camera samples, room window layers and captures;
  this does not substitute for a full graphical two-view acceptance test.
  **Still open:** windows do not recurse across two live rooms sharing a channel;
  channel-keyed sky captures can mix the rooms' skies. Two views of one room
  share the first view's portal eye (A2), portal continuity has one process
  screen anchor, and the portal dump describes only the first eye. The portal
  owner must address these without regressing room isolation.
- **Player trails are room-isolated** (2026-10-06): each trail follows
  its body's room band (`each_trail_is_drawn_in_the_gizmo_group_of_its_rooms_band`).
- **Developer view isolation fixed (`0e0099f0e`).** The debug presenters
  now use native room/view-owned retained drawings for grids, labels and
  portal diagnostics, with separate-room GPU witnesses. See
  [local co-op integration](../engine/local-coop-integration.md#source-findings-and-confidence).
  The rollback observatory's cross-room ghost presentation and any untested
  debug overlay paths need their own evidence; do not revive the old
  default-gizmo-group diagnosis as a current failure.
- **HUD/music:** the Q150/Q72 music priority rule is implemented, and vitals
  HUDs follow their owning views/participants, including stacked shared-view
  labels. **Still open:** the declared `HudReadouts` (Mary-O coins, Sanic rings)
  remain session-scoped, and remote local-seat filtering needs A4's
  client-local layout. Do not confuse those residuals with the finished
  built-in vitals HUD.
- A conversation holds only the talker, also with two players in one room
  (ruling 2026-08-06; Q163, its one-room duplicate, closed 2026-10-19):
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
Q155 (2026-10-19) treated the hold as a performance issue, not a fixed
policy. The 2026-10-24 requirement above now calls for uninterrupted gameplay
for an unaffected participant. That makes a global peer freeze on a routine
door crossing a **netplay readiness blocker**. Preserve the current barrier
until a peer-consistent, no-global-hold lifecycle passes the required tests. Named remainders: a link that loses
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
