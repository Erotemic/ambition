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
  everything. Ambition has no production road for a second player to join
  (Q153); only Smash seats slot 1;
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

▢ Separate the views when the two subjects exceed framing policy, and merge them
with hysteresis when they regroup.

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
- Not done: debug overlays read the sole live room; the through-portal window is
  drawn for the primary seat's view only; banner, music and HUD are
  session-wide and follow the primary seat. The Q150 ruling (2026-10-03, in
  [`../maintainer-decisions.md`](../maintainer-decisions.md)) makes the HUD
  per participant and the local music an authored-priority choice with the
  primary participant as the tie-break; not built yet.
- Two players in one room still share a conversation's pause (the first
  game-state question above).

### A4 — online participant

▢ Feed a remote participant through the same intent/control seam. Keep the local
view layout client-local. Measure what a room crossing's rebase costs a remote
player's rollback window.

### A5 — mixed local + remote party

▢ Prove the architecture does not assume one input device per machine, one local
participant, or one view per participant.

## Relationship to other games

TwinTrack is the strongest acceptance customer for independent views/reference
frames. Smash is a strong N-participant combat customer and may become a
first-class game, but its arena rules usually keep fighters together and
therefore do not prove Ambition's multi-room requirement.
