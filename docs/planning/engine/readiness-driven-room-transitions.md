# Readiness-Driven Room Transitions and Loading Presentation

## Status

Selected architectural direction. **Phases 1-5 implemented 2026-10-05; phase 6
implemented with an unmeasured starting budget; phase 7's door ranking
implemented, its residency budget open.** See
"Discovery results and the implemented shape" below, which supersedes the
discovery gates where they conflict.

The player-facing property (Jon, 2026-10-04): *if the hardware is fast enough
a loading screen never appears; on a slower machine it lets the player know
the game has not frozen and is still working.*

Loading screens must represent **actual unmet readiness at the moment a transition needs to commit**. They must not be developer-authored waiting periods, guessed per-character rations, minimum display timers, or another source of truth for whether a room is ready.

The engine should hide load time by preparing likely destinations while the current room remains playable. If the player reaches a transition boundary and the required prepared generation is already ready, commit immediately. If it is not ready, the transition becomes blocked on that real readiness condition and only then should loading presentation appear.

The loading UI observes the same readiness authority that gates the transition. It does not own pacing and it does not estimate readiness independently.

This document is deliberately separate from the semantic-part rendering roadmap. The two campaigns meet at asset preparation cost, but room readiness is a session/lifecycle authority and should not be owned by the renderer.

## Discovery results and the implemented shape (2026-10-05)

### Measured problem (Jon's GPU host, hub to hall)

Bundle `dev/ambition_dev_measurements/profiles/desktop-timeline-run-20261005T025328Z`:
`asset_wait` 2,840.8 ms of a 2.9 s crossing; preflight 3.7 ms, manifest
3.1 ms, commit 26.3 ms, first frame 23.3 ms. The wait was the character start
ration (`MAX_CHARACTERS_MATERIALIZED_PER_FRAME = 1`, cost read off the quality
tier): 137 characters, 137 frames. The work behind it was about 68 MP of
decode, about 0.5 s of summed decode time (38.5 MP of it three baked boss
sheets). The hall was not prefetched (`NEIGHBOR_PREFETCH_ROOM_BUDGET = 4`, the
hub has more neighbours). The bar counted settled items, so it opened near 80%.

### The commit gate already existed

There is one readiness authority and nothing parallel was added:
`poll_room_transition_asset_readiness_system` folds
`inspect_room_asset_manifest` (every handle the destination's first frame
draws, through GPU preparation: `AppGpuPreparedImages`) and
`inspect_demanded_characters` (every demanded character realized) into
`readiness {settled, total, pending, failed}`. Commit also waits on the
construction plan and the cover acknowledgement; the cover lifts at commit
plus the target being presentable (`UnclaimedFeatureViews` empty).

### What changed

| plan item | implementation |
| --- | --- |
| no proxy start ration (phase 4) | `materialize_character_demand` drains its demand: every demanded character starts on the frame it is demanded. `take_bounded`, `take_within_budget` and the ration constants are deleted, and so is the room "remainder" forwarding they made necessary. <!-- cite-ok: deleted names --> |
| pacing at the expensive stage (phase 6) | Bevy's `RenderAssetBytesPerFrame` is the one pacing authority (`game/ambition_app/src/host/render_asset_budget.rs`): `VISIBLE_UPLOAD_BYTES_PER_FRAME` (16 MiB) while gameplay is visible, lifted while a cover or a load foreground hides the frame. `AMBITION_RENDER_ASSET_MB_PER_FRAME` overrides it. |
| prepare before the deadline (phase 7, first step) | The neighbour prefetch now loads a neighbour's whole cast in the open (it loaded the ration's one character before), protected by the upload budget. |
| prepare the door the player approaches (phase 7) | A live room's neighbours are ranked nearest door first from where its players stand (`RoomSet::neighbors_nearest_first`, fed by `prefetch_neighbor_room_preparation_system` from the driven bodies' positions); the budget of 4 takes the nearest. The central hub has 21 doors: by room index the hall of characters, behind the 15th, was never prepared, which is the loading bar Jon saw coming through its door. Evidence: `a_player_at_the_halls_door_enters_a_prepared_hall` (poisoned back to index order it misses the prefetch). |
| no minimum display time (phase 1) | `minimum_visible` (300 ms) is deleted. The cover lifts at commit + presentable. |
| honest progress (phase 5) | The room-transition load experience shows no percentage (`show_estimated_percentage = false`); the player sees the named work, and a spinner (`BasicLoadSpinner`) turns on real time as the sign of life. |
| readiness projection and diagnostics (phases 2, 5) | The gate's `pending` list is kept on `ContributedRoomAssets` (read-only). When a loading foreground appears it logs `loading_screen_reason`: the destination and the activation-critical work it waits on. The 5 s no-progress stall report is unchanged. |

Evidence: `game/ambition_app/tests/a_ready_room_shows_no_loading_screen.rs`.
A prefetched neighbour is entered with the foreground never visible. On a
simulated slow machine (250 ms frames) the unprepared hall shows it and drops
it within a frame of ready. On a 1/60 s clock the unprepared hall now crosses
in 11 frames, inside the 250 ms reveal grace, with no loading screen at all.

### Kept, and why

- **The opaque cover at the door** stays as transition presentation: it hides
  the frame where the old room's world is replaced and the new one's views are
  still spawning. For a ready destination it lasts the frames to commit and
  draw (a cut, not a loading screen). The loading FOREGROUND (text, spinner) is
  what "loading screen" means here, and it shows only after the reveal grace.
- **`loading_reveal_after = 250 ms`** is perceptual hysteresis on the
  foreground, not a wait: it never delays commit, and it is what makes fast
  hardware show nothing.

### Still open

1. **The visible upload budget is a starting value.** Measure the frame-time
   per uploaded byte on the GPU host (an uncovered prefetch of a large
   neighbour is the case) and set it from that.
2. **Prefetch selection is a room count.** `NEIGHBOR_PREFETCH_ROOM_BUDGET = 4`
   stands in for a memory/residency budget. The order is now nearest door
   first; the budget in resident bytes, and how long a room that left the
   nearest four keeps its preparation (today: dropped, re-prepared on return),
   are the rest of phase 7.
3. **Images keep a CPU copy after upload** where nothing reads it (the render
   target census reported 153 MB of `cpu_bytes`). `RenderAssetUsages` per
   image kind is a memory item for `asset-preparation-and-residency.md`.
4. **The minimum-display-time removal has no failing test.** A floor shorter
   than the time the foreground was already up is invisible to the slow-machine
   arm (poisoned in, it stayed green); the code and this plan hold it.

## Problem statement

The recent Hall load exposed two forms of policy leakage.

First, character loading was rationed by a guessed cost attached to quality tier. One Full-tier character was effectively admitted per frame even when a part-based character only needed a small amount of image data. The Hall therefore spent roughly one frame per character draining an artificial queue rather than waiting on the real work needed to make the room usable.

A pixel-based ration is a better estimate, but it is still an estimate applied at the wrong stage if the actual hitch comes later when decoded images become GPU/render-world resources.

Second, room-transition presentation currently has policy such as creating a cover up front and enforcing a minimum visible interval. That conflates:

- deliberate transition presentation;
- actual loading/readiness;
- pacing intended to protect uncovered gameplay frames.

Those concepts need separate authorities.

## Core rules

### 1. Readiness gates commit

A room/session transition may commit when the existing prepared generation/candidate has all simulation-critical and required presentation assets/resources needed to begin the destination correctly.

The precise readiness structure should extend the existing prepared-generation/candidate lifecycle rather than creating an independent loader truth.

```text
prepare destination
      |
      v
existing candidate/prepared-generation readiness
      |
      +------ ready ------> commit immediately
      |
      +---- not ready ----> transition waits
                              |
                              v
                       loading UI observes
```

### 2. Loading presentation never makes something ready

Showing a loading screen must not start a timer whose completion permits transition.

Removing a loading screen must not depend on an authored minimum duration.

The only exception is an independently authored cinematic/fade/door animation. If such presentation intentionally delays control, model it as transition presentation, not as loading readiness and not as fake progress.

### 3. Prepare before the deadline whenever possible

The best loading screen is one that never appears.

Use world topology, known portals/doors, residency policy, and existing candidate preparation to start likely destination work while the player is still active in the current room when that is safe and economical.

Preparation should not transfer authority to the destination early. A prepared candidate remains speculative until the lifecycle commits it.

### 4. The cover is a consequence of a missed readiness deadline

When the player actually requests/forces a transition:

- if ready, commit without a loading screen;
- if not ready, enter the existing blocked/preparing transition state and show a loading cover because there is now real work preventing immediate commit;
- remove the cover as soon as the destination can correctly commit, subject only to any independent transition animation that is intentionally part of the game's presentation.

There is no universal 300 ms (or other) loading-screen floor.

### 5. Covered and uncovered loading have different scheduling needs

A covered transition is already hiding a hitch. Do not artificially serialize cheap demand merely to keep frames smooth behind an opaque cover unless doing so improves actual wall-clock completion or avoids a real resource hazard.

Uncovered streaming during gameplay is different. It must protect interactive frame time, but pacing belongs at the expensive completion/admission stage that actually causes hitches, not at an earlier proxy stage merely because it is easy to count.

### 6. Progress must be honest

A progress bar may only display quantified progress derived from the same real work/readiness graph that gates commit.

If the engine cannot honestly establish a denominator, use an indeterminate loading indicator rather than invented percentages or weighted developer guesses.

Do not show "80%" because eight arbitrary stages are done while one unbounded stage remains.

## Target authority model

There should be one lifecycle authority for a prepared destination and its readiness.

Conceptually:

```text
Prepared destination / generation
    |
    +-- simulation construction readiness
    +-- required authored-content readiness
    +-- required image/resource decode readiness
    +-- required render/GPU readiness
    +-- other proven commit blockers
          |
          v
      ReadyToCommit
```

The exact fields/stages must be discovered from the production pipeline; do not add a parallel generic `LoadingProgress` authority that independently decides the same facts.

The loading UI receives a **read-only projection** of that readiness for presentation/debugging.

A separate scheduler may decide how aggressively background work is admitted, but it cannot declare readiness and it cannot invent progress.

## Simulation-critical versus presentation-optional readiness

The implementation agents must explicitly classify what must be complete before simulation can safely begin in the destination.

Do not automatically make every visual asset a hard blocker if the engine has a safe placeholder/late-realization road. Conversely, do not begin simulation if missing data would change collision, authored geometry, actor construction, deterministic state, or any other simulation fact.

This classification is important for both responsiveness and determinism.

Potential categories include:

- **hard simulation blocker** - destination cannot begin correctly without it;
- **hard presentation blocker** - destination would be visibly broken and no valid fallback exists;
- **streamable presentation** - can arrive after commit using a valid fallback;
- **speculative/prefetch-only** - useful but not required for this transition.

The classification must be owned by the resource/content type or preparation contract, not by an ad hoc loading-screen table.

## Covered transition behavior

Once an immediate transition has missed its readiness deadline:

1. Preserve the correct lifecycle/freeze semantics already required by the session/rollback model.
2. Present an opaque or otherwise safe loading cover.
3. Request/admit all required work as aggressively as the backend safely allows.
4. Do not deliberately drip one character/load request per frame simply to protect a frame the player cannot see.
5. Commit as soon as the actual prepared destination satisfies the commit gate.
6. Remove the loading presentation as part of/after that commit according to the normal presentation lifecycle.

If decode/upload concurrency itself can exhaust memory or destabilize the driver, retain bounded resource concurrency for that concrete hazard. The bound should express the hazard (memory, outstanding upload bytes, worker count), not simulate a loading duration.

## Uncovered streaming behavior

Background streaming while gameplay remains visible should protect frame latency.

The recent Hall diagnosis suggests that image decoding already occurs off the main thread and that the large hitch historically happened when many completed images reached render-world/GPU preparation together. If that remains true, throttle or budget the **completion/admission stage** rather than rationing decode starts by guessed character cost.

A good scheduler shape is:

```text
background decode / preparation
          |
          v
completed resources waiting for expensive admission
          |
  frame-budgeted admission
          |
          v
render/GPU-ready resources
```

The exact budget mechanism is an implementation discovery item. Possibilities include:

- actual decoded byte size admitted per frame;
- actual GPU-upload byte size where available;
- bounded number of expensive prepare operations;
- feedback from measured recent frame time;
- a combination.

Do not encode character quality tier as a proxy when the real resource size is known by this stage.

## Progress presentation

### Preferred source

Expose a diagnostic/read-only readiness report from the prepared destination. It should explain both:

- **what remains blocking commit**;
- **what measurable work is complete**.

In debug/development builds it should be possible to answer questions such as:

```text
Hall preparation
  room construction       ready
  actor definitions       138 / 138
  required image bytes    41.2 MiB / 45.7 MiB ready
  render preparation      131 / 138 ready
  blocking items          7 character realizations
```

The names above are examples, not a prescribed stage list.

### Determinate versus indeterminate UI

Use a determinate bar only when the denominator is real and monotonic enough to be meaningful.

For mixed work where no honest scalar denominator exists, prefer:

- a spinner/indeterminate bar for the player;
- detailed stage/blocker information in diagnostics.

Never create arbitrary stage weights merely to keep a percentage moving smoothly.

### No minimum display time

Do not keep the loading UI visible so the player has time to perceive it. If preparation finishes almost immediately after the deadline, the cover may flash briefly. Prefer preventing that flash through earlier preparation or a normal transition fade, not by adding fake wait time.

If UX later demonstrates that a sub-frame cover causes unacceptable flicker, solve that at the presentation-transition level without changing readiness or claiming work remains when it does not.

## Relationship to room/candidate preparation

This work should reinforce the existing prepared-generation/candidate architecture.

Preparation can happen while the old room/session remains authoritative. The destination becomes authoritative only at lifecycle commit.

Do not introduce a loader that mutates the live session early to make progress.

For multiplayer/rollback transitions, the peer commit barrier remains authoritative. Local asset readiness may be one input to peer preparation/eligibility, but a loading UI must not bypass or replace the network/session commit protocol.

## Relationship to semantic part rendering

Part-based characters should naturally become cheaper to prepare because they reuse a small set of semantic/vector-derived images instead of large baked sheets.

The loader must charge/observe their actual preparation work rather than assigning the historical cost of a Full-tier baked character.

The rendering roadmap may also change what constitutes "ready" for a part character (for example, direct part textures versus a precomposed body atlas). This loading plan must consume whatever readiness contract the renderer/content pipeline exposes; it must not know implementation details such as impostor pages or camera counts.

## Migration phases

### Phase 0 - discovery and instrumentation

Before replacing pacing logic, identify the full production readiness chain for a representative room and the Hall.

Instrument timestamps/blockers sufficiently to answer:

- when demand is issued;
- decode start/end;
- asset availability in the main world;
- extraction/render-world preparation;
- GPU upload/preparation if observable;
- character/room realization completion;
- prepared-generation eligibility;
- transition commit.

The goal is not permanent tracing everywhere. It is to locate the actual wall-clock and hitch-producing stages before choosing a scheduler.

### Phase 1 - remove fake loading-duration policy

Separate transition presentation from loading readiness.

Remove/de-authorize:

- minimum loading-screen duration;
- progress derived from elapsed time;
- loading cover as a prerequisite for readiness;
- any room-authored/developer-authored "wait N frames/ms" loading policy.

Keep deliberate door/fade/cinematic timing as its own presentation concept where wanted.

### Phase 2 - make existing readiness observable

Expose a read-only readiness/blocker projection from the same prepared-generation/candidate authority that gates commit.

Use it for:

- debug logs;
- tests;
- loading UI state;
- profiling Hall preparation.

Do not let the projection become writable authority.

### Phase 3 - transition-deadline-driven loading cover

Change the visible transition road so that merely approaching/requesting a destination does not automatically imply a loading screen.

At commit time:

```text
if destination is ready:
    commit immediately
else:
    show loading cover
    remain blocked on the real readiness authority
    commit immediately when ready
```

Add poison tests for both paths.

### Phase 4 - remove proxy start ration behind covered loads

For a covered transition, remove the one-character/quality-tier/pixel-estimate demand ration unless measurement proves a real resource hazard that requires bounded concurrency.

Request the required room resources promptly and let parallel decode/preparation proceed.

Measure Hall wall-clock preparation time and peak memory after the change.

### Phase 5 - honest progress

Connect player-facing progress only to measurable readiness/work facts.

Use an indeterminate indicator where a real scalar denominator is unavailable.

Add diagnostics that list the actual blockers when a transition takes longer than a threshold. This should make future "stuck at 80%" investigations trivial.

### Phase 6 - move uncovered-streaming pacing to the expensive stage

Only after the covered road is correct, address background streaming hitches.

Budget the render/GPU/expensive completion stage based on actual work and measured frame behavior. Delete the earlier proxy ration once the real stage is controlled so two pacing authorities do not coexist.

### Phase 7 - prefetch policy refinement

Use measured transition behavior to improve which adjacent/likely rooms are prepared early and how long speculative preparation is retained.

This is an optimization layer over the readiness architecture, not part of correctness. Do not delay the earlier phases while trying to predict player movement perfectly.

## Discovery gates for the implementation agents

The following items still need explicit investigation. They are part of the task, not unanswered design questions that should create parallel authorities.

### Exact commit blockers

1. Enumerate every predicate currently preventing a prepared room/generation from committing.
2. Identify which are simulation correctness requirements versus presentation-quality requirements.
3. Identify blockers that are historical artifacts of the old baked-character pipeline and can be removed.
4. Determine whether any readiness test currently waits on work that can safely stream after simulation starts.

### Character load pipeline

1. Confirm where image decoding occurs and on which threads.
2. Confirm the stage that caused the historical 516 ms hitch and reproduce it if practical on current code.
3. Determine what resource size is known at each stage: compressed bytes, decoded pixels/bytes, GPU texture bytes, page count, etc.
4. Determine whether Bevy/WGPU exposes enough information/control to budget GPU upload directly or whether the closest controllable completion stage should be used.
5. Determine whether current part-based publication leaves unnecessary CPU copies or duplicate images resident during preparation.

### Hall readiness

For the Hall specifically, capture a preparation waterfall before and after removing proxy rationing:

- number of required character realizations;
- unique images/pages actually required;
- decode wall-clock time;
- render/GPU preparation time;
- candidate/room construction time;
- time from transition intent to commit eligibility.

This is the acceptance evidence for eliminating the artificial 80%-to-100% crawl.

### Existing loading-cover semantics

1. Locate every code path that creates/removes the transition cover.
2. Identify all minimum-duration/fade rules and whether they are intended as aesthetics or loading policy.
3. Determine how local pause/freeze interacts with rollback/multiplayer while readiness is pending.
4. Determine whether a cover can currently appear for a transition that is already fully prepared.
5. Identify any tests that accidentally encode minimum loading duration as expected behavior and replace them with readiness semantics.

### Progress denominator

1. Determine which required work items have known total byte/item counts before they start.
2. Determine which stages can discover additional work dynamically.
3. Decide where a determinate fraction is honest and where the UI must remain indeterminate.
4. Ensure progress cannot move backward because two independently weighted estimators disagree. Prefer stage-specific facts over a fabricated global scalar.

### Background streaming

1. Reproduce an uncovered streaming hitch with current assets before adding a new scheduler.
2. Identify the actual expensive completion stage.
3. Measure a frame-time/byte relationship on representative hardware if using a byte budget.
4. Decide whether adaptive feedback is necessary or a simple conservative completion budget is sufficient.
5. Ensure streaming budget state is presentation/resource scheduling state, not rollback simulation authority.

### Prefetch and residency

1. Determine which room transitions already prepare candidates before the player reaches the door.
2. Determine how portal/adjacency topology can trigger preparation without excessive memory residency.
3. Identify when speculative candidates/resources are evicted.
4. Ensure prefetch does not change durable/session ownership before commit.

## Required tests

### Immediate-ready transition

Prepare a destination fully before the player crosses the transition boundary.

Assert:

- transition commits without entering loading state;
- no minimum loading-cover timer delays the commit;
- simulation starts from the correct prepared generation.

### Late-ready transition

Hold one real required resource/preparation predicate incomplete, request the transition, then complete it.

Assert:

- loading presentation appears only after the transition is actually blocked;
- commit remains blocked while the real predicate is false;
- commit occurs promptly when it becomes true;
- no additional authored delay remains.

### Fake-progress poison

Construct a preparation stage with an unknown/variable duration.

Assert the player UI does not invent a determinate percentage from arbitrary weights. The debug view may report the blocking stage/item.

### Part-versus-baked load cost

Create two character sets with materially different actual image work but the same legacy quality tier.

Assert that covered preparation wall-clock behavior is driven by actual work/resources rather than one-character-per-frame admission.

### Uncovered streaming budget

Once implemented, stream a burst while gameplay remains visible and assert the expensive completion stage respects its frame budget without delaying already-ready room transitions through a second independent ration.

### Multiplayer/session correctness

A loading cover must never make a peer transition eligible by itself. Peer/session commit still waits on the existing lifecycle protocol and required local readiness.

## Diagnostics

Retain lightweight diagnostics that make future loading investigations cheap.

When a blocked transition exceeds a development threshold, log something like:

```text
transition still waiting after 250 ms
  destination: hall_of_characters
  generation: ...
  blockers:
    - character realization: ...
    - render preparation: ...
  measurable work:
    images ready: ... / ...
    bytes ready: ... / ...
```

Exact formatting is unimportant. The important property is that the engine can explain **why commit is not yet legal** using the same facts that actually gate it.

## Non-goals

- Do not guarantee that every room transition is instant.
- Do not hide a real missing simulation dependency behind a placeholder solely to avoid showing a loading screen.
- Do not make the loading UI an authority over lifecycle state.
- Do not create authored room-specific wait durations.
- Do not solve every speculative streaming/prefetch heuristic before fixing covered transitions.
- Do not couple room readiness to the current character compositor implementation.
- Do not weaken multiplayer/rollback commit barriers to make the loading screen disappear sooner.

## Completion criteria

This roadmap is complete when:

1. a fully prepared destination commits without showing a loading screen or waiting on an arbitrary loading timer;
2. a not-yet-prepared destination shows loading presentation only because real readiness blocks immediate commit;
3. the loading UI observes the same readiness facts that gate lifecycle commit;
4. covered loads are not artificially serialized by guessed character/tier costs;
5. uncovered streaming, if pacing is needed, budgets the actual expensive completion stage rather than decode-start proxies;
6. progress is based on real measurable work or is explicitly indeterminate;
7. the Hall no longer spends dozens/hundreds of frames draining an artificial per-character ration;
8. diagnostics can identify the concrete blocker for any slow transition;
9. there is no duplicate loading/readiness authority competing with the prepared-generation/session lifecycle.
