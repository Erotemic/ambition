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

The engine cannot currently answer the question that matters —
*what exact required fact is keeping this barrier from Ready?* — and it is one
field away from being able to. `inspect_room_asset_manifest` already computes:

```rust
RoomAssetReadiness { settled, total, pending: Vec<String>, failed: Vec<String> }
```

and `ActiveRoomTransitionLoad` retains only `last_asset_progress: (settled,
total)`. The names are computed every poll and dropped every poll. **A load
coordinator that knows it is blocked must be able to say what on.**

## ⛔ The phase timings are compiled out exactly where the stress is

`construction_preflight_duration` and `asset_manifest_duration` are both written
under `#[cfg(not(target_arch = "wasm32"))] std::time::Instant::now()`. The
browser is the platform whose numbers would explain the observation, and it is
the one platform that records none.

`bevy::platform::time::Instant` is already in the dependency graph (`web-time` on
wasm, `std` elsewhere) and is a sub-frame clock on both. `Time<Real>` is NOT a
substitute: it advances once per frame, so a within-frame span measures zero.

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

⭐ **MEASURED, 2026-08-15, from `hall_of_characters.ldtk`: 129 `NpcSpawn` rows
naming 129 DISTINCT characters, with no repeats.** Not "a big room" — the worst
possible shape for this seam. Every one of the 129 is its own catalog lookup, its
own atlas layout, and its own image handle, materialized in the single frame that
builds the manifest; then the reveal barrier waits on all 129 before anything is
shown. On a desktop with a warm page cache that is a stutter. In a browser each
one is an HTTP request, and *any* single straggler holds the whole barrier
un-Ready — which is precisely what "stuck at 99%" would look like from outside,
with no way to ask which one.

⚠ **so do not assume the Hall is stuck before the diagnostic exists.** A barrier
correctly waiting on the slowest of 129 fetches and a barrier deadlocked on one
asset that will never arrive are indistinguishable today, and they want opposite
fixes. Naming the pending set is what separates them.

**Measure before redesigning.** The numbers that decide whether there is anything
here: demanded characters, how many were already materialized, how many needed
new materialization, elapsed CPU, atlas/layout creations, handles requested, and
how much of it repeats on re-entry.

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
