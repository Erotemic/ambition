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
- respawn/rejoin behavior when another participant remains alive elsewhere;
- inventory transfer/trading rules between controlled bodies;
- save ownership and join/leave policy for remote participants;
- how far shared quest/world causality extends when players explore separately.

Use concrete Ambition content to answer them rather than inventing a universal
multiplayer narrative framework up front.

## Incremental acceptance path

### A1 — two local participants, same room, shared view

Prove two independent input participants/control assignments in ordinary
Ambition gameplay with one shared camera and body-owned HUD state.

### A2 — adaptive split in one room

Separate the views when the two controlled subjects exceed framing policy and
merge again when they regroup.

### A3 — two rooms resident

Let the participants cross different loading zones and continue in distinct
rooms without replacing one another's simulation state.

### A4 — online participant

Feed a remote participant through the same intent/control seam. Keep local view
layout client-local.

### A5 — mixed local + remote party

Prove the architecture does not assume one input device per machine, one local
participant, or one view per participant.

## Relationship to other games

TwinTrack is the strongest acceptance customer for independent views/reference
frames. Smash is a strong N-participant combat customer and may become a
first-class game, but its arena rules usually keep fighters together and
therefore do not prove Ambition's multi-room requirement.
