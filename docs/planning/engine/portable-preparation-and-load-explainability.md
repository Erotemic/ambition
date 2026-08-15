# Portable preparation, and a load barrier that can say what it is waiting for

**D124.** Opened 2026-08-15 from three maintainer observations that only a real
browser has ever made. The frame is **portability, not "optimize wasm"**: the
browser is the harshest probe of foreground work that desktop hides, and a change
earns its place here when it improves desktop, Android, Steam Deck and the
browser together. Brotli, `wasm-opt`, AudioWorklets and cache headers are
measurements; they are not this campaign.

## What Jon observed (2026-08-15, real served build)

Do not reinterpret these. They are maintainer measurements.

- The transition into **Hall of Characters appeared to remain at 99%.**
- The **opening music crackled/distorted and then audibly "caught up"** while
  startup and loading were heavy.
- **Steady-state gameplay after startup settles is substantially better.**

Alongside, and already closed elsewhere: the browser boots and visibly runs
(D121, human-confirmed), and gameplay device input was dead because a developer
instrument owned the latch (D123, awaiting retest).

## ⛔ 99% does not mean "one asset left"

`LoadPresentationModel::from_snapshot` clamps an un-Ready barrier:

```rust
if !ready {
    estimate.fraction = estimate.fraction.min(0.999);
}
```

So the number means exactly *"the barrier has not reached Ready"* and nothing
about how much remains. **Reading it as 1% is the first wrong turn.**

The engine could not answer the question that matters — *what exact required fact
is keeping this barrier from Ready?* — and it was one field away.
`inspect_room_asset_manifest` already computed:

```rust
RoomAssetReadiness { settled, total, pending: Vec<String>, failed: Vec<String> }
```

and `ActiveRoomTransitionLoad` retained only `last_asset_progress: (settled,
total)`. The names were computed every poll and dropped every poll. **A load
coordinator that knows it is blocked must be able to say what on.**

✔ **LANDED 2026-08-15.** A barrier that has not settled a single asset for five
seconds now files one `asset_stall_report` — the room, how long, how many of how
many are outstanding, and the first twelve of them by name — as STATE on the
transition, so a test can assert it and a dev overlay can show it, and as one log
line rather than one per frame. ⚠ not a timeout: nothing is cancelled, because a
slow connection legitimately spends that long on a large room. The report is how
a maintainer tells that apart from a barrier that will never move.

⭐ **the pin found the trap on its first run.** `last_asset_progress` has TWO
writers — the contributor stores it once when it builds the manifest, the poll
updates it as it changes — and the stall clock was taught to only the poll. The
Hall then sat at `(0, 164)` with `asset_progress_since: None` forever: it could
never become old enough to explain itself, because the contributor had already
stored the key the poll would have called a change. Both now go through
`observe_asset_progress`, which is the only way to record progress and therefore
the only way to restart the clock.

## ⛔ The phase timings are compiled out exactly where the stress is

`construction_preflight_duration` and `asset_manifest_duration` were both written
under `#[cfg(not(target_arch = "wasm32"))] std::time::Instant::now()`. The
browser is the platform whose numbers would explain the observation, and it was
the one platform that recorded none.

✔ **LANDED 2026-08-15**: both use `bevy::platform::time::Instant`, already in the
dependency graph (`web-time` on wasm, `std` elsewhere) and sub-frame on both, and
the two `#[cfg]` blocks are deleted. ⚠ `Time<Real>` is NOT a substitute here: it
advances once per frame, so a within-frame span measures zero — which is exactly
what the manifest burst is.

⚠ these are write-only diagnostics observed by no simulation decision. Timing
data must never enter a deterministic decision — that is why they are recorded on
the feel clock and read only by instruments.

## The synchronous seam this campaign is really about

```text
contribute_room_transition_assets_system
  → build_room_asset_manifest
      → demand_room_character_sheets
          → materialize_character_demand      (the whole staged cast, synchronously)
```

The source says plainly why it is synchronous: the manifest must enumerate
concrete handles before the reveal barrier can be built from them. Hall of
Characters is the extreme customer of that decision — it is the room whose entire
point is a large cast standing together.

⭐ **MEASURED 2026-08-15 — and name the denominator, because there are three.**
`hall_of_characters.ldtk` authors **129 `NpcSpawn` rows naming 129 DISTINCT
character ids, with no repeats**; the staged cast the transition demands reaches
**~151**; the reveal barrier that results holds **164 activation-critical asset
handles**. (An older comment in `hall_transition_cover.rs` says "144 NPCs"; the
129 above is a direct count of the authored file at HEAD.)

Not "a big room" — the worst possible shape for this seam. Each of the 129 is its
own catalog lookup, its own atlas layout, and its own image handle, materialized
in the single frame that builds the manifest; then the reveal barrier waits on
all 164 before anything is shown. On a desktop with a warm page cache that is a stutter. In a browser each
one is an HTTP request, and *any* single straggler holds the whole barrier
un-Ready — which is precisely what "stuck at 99%" would look like from outside,
with no way to ask which one.

⚠ **so do not assume the Hall is stuck before the diagnostic exists.** A barrier
correctly waiting on the slowest of 129 fetches and a barrier deadlocked on one
asset that will never arrive are indistinguishable today, and they want opposite
fixes. Naming the pending set is what separates them.

⭐ **MEASURED NATIVELY, 2026-08-15** (`hall_transition_cover`, which prints these
now and asserts nothing about them — a threshold would be a performance
assertion inside a correctness test):

```text
[hall-transition] preflight=1.64ms  manifest=18.23ms  barrier=(0, 164)  prefetch_hit=false
```

18ms of synchronous manifest work is one dropped frame on a warm desktop. It is
the single-threaded browser that decides whether that is 18ms or 300ms, and that
number now records there — which is the measurement to take on the next browser
run before any redesign.

## ⛔⛔ The prefetch is ANTI-CORRELATED with cost

`prefetch_hit=false` on the most expensive transition in the game, and it is not
a timing accident. `NEIGHBOR_PREFETCH_ROOM_BUDGET` is **4 rooms**, justified in
its own comment by *"every corridor and lab in `sandbox.ldtk` has at most four
exits, so ordinary traversal is unaffected and only the hubs are trimmed"*. Both
halves of that are true. Counted at HEAD:

```text
central_hub_main       21 loading zones   <- holds the Hall door
central_hub_basement   18
hall_of_bosses         10
gradient_ascent         6
drain_alley             5
everything else       1-4
```

So the rooms whose preparation is cheap are the ones that always get prefetched,
and the rooms behind a hub — the Hall, the boss hall, every destination a player
actually chooses from — are the ones structurally guaranteed to miss. The budget
was added to stop a hub's fan-out stuttering the launch (2026-07-30, from Jon's
own desktop timeline), and the shape it produced is a cache that covers exactly
what does not need covering.

⚠ **and the hub is the right place to pay.** A player standing in a hub choosing
a door is idle; the door itself is the one moment they are not. Moving the Hall's
18ms there is strictly better — *if* it is spread rather than spent at once,
which is the same budgeted-realization question as everything else here.

⛔ **recorded, not cut.** Raising or reordering the budget without a work
measure would re-create the launch stutter it exists to prevent. The redesign is
"bound the prefetch by WORK, and order it by expected cost", and it needs the
browser numbers first. ✔ note the prefetch does now prepare its four neighbours
at all — before the seven-roads fix below, every neighbour containing a
character-built body was refused and re-attempted every frame, so the cache was
empty AND expensive.

**Measure before redesigning.** The numbers that still decide the shape: how many
of the demanded characters were already materialized, how many needed new work,
atlas/layout creations, and how much repeats on re-entry.

If measurement confirms a large synchronous burst, the direction is

```text
authored definition → reusable prepared artifact → cache → room/view demand → budgeted realization
```

and NOT a Hall special case. ⛔ do not make `RoomConstructionPlan` partially
authoritative to smooth rendering: deterministic simulation construction and host
presentation realization stay separate. Presentation may realize N character
artifacts across several covered frames; the plan may not become partial.

⚠ **and establish the EXISTING cache semantics before adding another cache.**
`CharacterLoadStates` and `PreparedCharacterRegistry` already exist; the question
is what they already guarantee about re-entry, not what a new layer would.

## What was already found on the way in

**The neighbour prefetch had never prepared a room containing a character-built
body.** It hand-assembled its `ActorConstructionContext` and passed neither the
prepared cast nor the published brain profiles, and
`preflight_planned_bodies` treats an absent registry as an EMPTY cast ("not an
exemption", in its own words). So every such neighbour failed preflight, was
forgotten, and was re-prepared from scratch on the next frame — a whole
`RoomConstructionPlan` per neighbour per frame, discarded, for as long as you
stood there. That is Jon's *"`goblin` … which this composition has not
registered"* line repeating at frame rate for a character that is registered, and
it also meant the prefetch never covered the rooms that cost the most to prepare.

⭐ **it was the THIRD road of seven, and four were incomplete.** Startup, reset,
transition, hot reload, provider activation, the exclusive-world rebuild and the
prefetch each built that context by hand; AC6 fixed the exclusive-world one on
2026-08-13 and nobody counted the rest. The authorities are constructor
PARAMETERS now (`ActorConstructionContext::for_room_construction`), so the next
one a room may consult is a signature change that breaks every road at once.

## ⛔ MAINTAINER RETEST REQUIRED — the only thing that closes this

Neither line may be checked because a native benchmark got faster, because a test
reports every asset loaded, because the load coordinator's unit tests pass, or
because the browser links.

- ▢ a real served browser transitions into Hall of Characters, the foreground
  leaves 99%, the Hall becomes playable, and no indefinite readiness stall is
  observed.
- ▢ the opening music does not crackle, race or catch up under browser startup
  load.

## Audio: trace before guessing

⛔ no speculative WebAudio workaround. The trace to establish first: context
unlock → music handle requested → root asset loaded → decoder/source ready → play
command issued → channel begins → the workload during those first seconds → any
playback-rate/seek/resynchronisation command.

The specific thing to search for is **any music advancement derived from game or
render time**. Musical time must not be advanced to compensate for time spent
loading. The portable contract:

> When music begins, it begins from a stable audio-clock position after its
> required source is ready, and heavy game preparation never causes musical time
> to race forward to catch up.

⭐ **searched at HEAD, 2026-08-15: there is no music time manipulation to find.**
`playback_rate`, `seek`, `start_from` and every sibling return nothing across the
tree; `play_music_track` calls `music_channel.play(handle)` with a 220ms fade and
nothing else. So *"musical time raced forward to compensate"* has no code that
could do it, and the leading hypothesis is the opposite direction: **crackle and
"catch up" are what STARVATION sounds like.** A stream whose position follows the
audio clock, starved while the main thread does something long, drops samples
(the crackle) and then resumes at the position the clock has already reached
(the "catch up"). Nothing in the game decided that; the game simply stopped
feeding it.

⚠ **and that makes it the SAME defect as the Hall, not a second one.** A wasm
build without threads runs audio callbacks on the main thread, so an unbounded
synchronous burst — 129 character materializations in one frame — starves the
mixer directly. If that is what is happening, budgeted realization fixes both
symptoms and no audio-specific change is needed.

⛔ **do not act on that until it is measured.** The instrument now exists and is
the same one: `asset_manifest_duration` is recorded on wasm as of this slice, so
a browser run will say how long that one frame actually took. A manifest frame in
the hundreds of milliseconds is the confirmation; a manifest frame of 20ms
falsifies it and sends the search to the audio path proper.

If no portable defect can be proven natively, leave the retest row open with the
logging ready. Do not invent a fix for a symptom that cannot be reproduced.

## Executable size: measure before comparing

Two numbers exist and they are not comparable yet — ~222 MB in Jon's build and
~163 MB in a later served-persona measurement. **Explain the discrepancy before
drawing any conclusion**: record the Rust wasm before `wasm-bindgen`, the
`wasm-bindgen` output, `web` vs `web_served_assets`, the profile, the feature set,
and compressed transfer size where that is trivial.

Then classify what is found by PORTABILITY: capability the persona cannot
exercise, platform backends pulled into unrelated executables, dev capability
surviving a production persona, broad Bevy feature bundles, duplicated runtime
implementations, and large monomorphised surfaces (which also cost native
compile, link and cold code pages). `twiggy`/`wasm-tools` are fine as
instruments; that does not make the fix wasm-specific. ⛔ do not start a Cargo
feature redesign to chase a byte target.
