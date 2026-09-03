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

> **Re-measured 2026-09-03 — the mechanism is covered; the audit surface is
> `PrimaryPlayerOnly`.** Two independent participants with independent control
> are not speculative: `multiplayer_smoke_tests.rs` holds **8** tests asserting
> two player entities keep separate attacks, safety anchors, slot-owned input,
> singleton queries and heal routing, joined by
> `two_participants_of_one_character_do_not_share_a_stream`
> (`crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_builders.rs:670`)
> and `a_second_participant_does_not_silence_the_global_menu_frame`
> (`crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs:1873`).
>
> ⚠ **What A1 still has to audit has a name and a count.**
> `PrimaryPlayerOnly` — `(With<PlayerEntity>, With<PrimaryPlayer>)`,
> `crates/ambition_platformer2d_shared_tangle/src/markers.rs:35` — appears in
> **35 production files** (and 28 test files). Its largest production sites are
> the sim harness, the headless and capture-scene tools and the single-player
> demos, where scoping to one player is correct BY CONSTRUCTION and no audit is
> owed. The sites that matter for A1 are in shipped simulation: `shrine.rs` (4),
> `morph_ball.rs` (3), `unified_melee.rs`, `unified_body_movement.rs`,
> `boss_contact_iframes.rs`.
> ⛔ **RE-COUNTED 2026-09-03 AND THAT LIST OF FIVE IS REALLY ONE.** The totals
> above hold exactly — **35 production / 28 test files** (`git grep -l` over
> `*.rs`, test = under `/tests/` or ending `tests.rs`/`_tests.rs`/`_test.rs`),
> and `shrine.rs` (4) and `morph_ball.rs` (3) are exact. But of the five named
> "shipped simulation" sites:
> - `shrine.rs` — `crates/ambition_platformer2d_actor_monolith/src/shrine.rs`,
>   genuinely simulation, 4 refs. **The one real A1 question.**
> - `morph_ball.rs` — `crates/ambition_render/src/rendering/morph_ball.rs`, which
>   is PRESENTATION, not simulation. A per-view question, not a per-participant
>   simulation-fact one.
> - `unified_melee.rs`, `unified_body_movement.rs`, `boss_contact_iframes.rs` —
>   all three are `game/ambition_app/tests/*`, i.e. acceptance TESTS. They fall
>   under the exemption this row already grants ("scoping to one player is
>   correct by construction and no audit is owed"), so they were never A1 work.
> ⇒ **A1's audit surface is smaller than the row promised, not larger** — which
> is the good direction, and worth having right because this row's stated value
> is that the remaining work "is enumerable today rather than discovered during
> it". ⚠ The heaviest production users are `sim_harness/runtime.rs` (9),
> `app_tools/bin/headless.rs` (5) and the demos, all already exempt by the same
> rule.
> ⇒ **This is not a defect list.** Each site is a question — *should this fact be
> per-participant when two people share a camera?* — and for a checkpoint shrine
> the answer may well stay "no". The value of the number is that A1's remaining
> work is enumerable today rather than discovered during it.

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
