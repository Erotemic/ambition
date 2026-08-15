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

## The prefetch skips the expensive rooms ON PURPOSE

`prefetch_hit=false` on the most expensive transition in the game, and the first
reading of that was wrong. `NEIGHBOR_PREFETCH_ROOM_BUDGET` is **4 rooms**, and
counted at HEAD:

```text
central_hub_main       21 loading zones   <- holds the Hall door
central_hub_basement   18
hall_of_bosses         10
gradient_ascent         6
drain_alley             5
everything else       1-4
```

So the cache always covers the cheap corridors and never covers the destinations
behind a hub. That looks like a cache that is anti-correlated with cost, and the
obvious move is to raise or reorder the budget so the Hall gets prefetched while
the player stands in the hub choosing a door.

⛔⛔ **THAT MOVE WAS ALREADY MADE, MEASURED, AND REVERSED — the constant's own doc
carries the numbers.** Unbounded prefetch from the hub, 2026-07-30, from Jon's
desktop timeline capture:

```text
staged cast on entering the Ambition route:  162 characters
decoded in the 10-15s window:                +157 images, +357.8 MP
resident image memory:                       1803 MB
frames in that 5s window:                    91   (p99 1372ms, max 1437ms)
```

⭐ **and the reason it hurts is the half that inverts the argument: a hub is not
idle time.** The door transition's wait is COVERED — that is what the load
foreground is for, and `hall_transition_cover` pins that it holds. Prefetching
moves that same work to a moment when the player is playing and nothing is
covering anything, for up to 21 rooms they may never enter. Trading a covered
wait for an uncovered hitch is a straight loss, and multiplying it by 21 is the
1372ms frame.

⚠ **so the budget is correct and this section is a correction to an earlier
reading of it.** The prefetch cannot be the answer for expensive rooms. The
expense itself is the answer: a room whose staging costs 18ms of CPU and hundreds
of megabytes of resident images is expensive wherever it is paid, and the only
move that helps is making that cost smaller or spreading it under the cover that
already exists.

⚠ **1803 MB resident is its own finding** and belongs to the same campaign. On a
browser that number is not a stutter, it is a tab that dies.

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

✔ **ANSWERED 2026-08-15, and the answer is a NEGATIVE result — the useful kind.**
`materialize_declared_character_sprite` opens with `CharacterSheetState::Ready(_)
=> return`, before any sheet lookup, atlas build or handle request. A character
another room already prepared costs nothing to stage again. Pinned by
`re_demanding_a_resident_character_repeats_no_preparation`, which counts atlas
layouts rather than wall time (a timing assertion there would be a flaky
performance test; a layout built twice is the actual work and is countable), and
falsified against a disabled short-circuit: **1 → 2 layouts.**

⭐ **so the remaining Hall cost is genuinely FIRST-VISIT work, and that changes
the design.** It wants the first visit BUDGETED, not repeats memoized — opposite
fixes, and building the cache that already exists would have cost the campaign
while changing nothing.

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

## Executable size: MEASURED 2026-08-15

`./build_for_web.sh --served` at HEAD — features `web_served_assets`, profile
`release`:

```text
Rust wasm before wasm-bindgen   178 MB
wasm-bindgen output             164 MB wasm + 112 KB js
gzip of the wasm                26.4 MB      <- what a browser actually DOWNLOADS
```

⭐ **so "220 MB" was never a network problem.** The transfer is 26 MB. The 164 MB
is what the browser must PARSE AND COMPILE, and that is the cost that matters —
compile time and peak memory, on a device that may have neither.

Section census of the 163.3 MB module:

```text
code                  89.6 MB   54.8%
custom:name           37.4 MB   22.9%   <- debug symbol names, in a release build
data                  35.1 MB   21.5%   <- baked into the SERVED-ASSETS persona
elem/function/rest     1.1 MB    0.7%
```

⛔⛔ **and the workspace already has the profile that addresses two of those, and
nobody ran it.** There is no `[profile.release]` override, so `release` is
Cargo's default — `strip = "none"`, which is why 37.4 MB of name section ships.
`[profile.web-release]` right there in the root `Cargo.toml` sets `strip =
"symbols"`, `lto = "fat"`, `opt-level = "s"`, `codegen-units = 1`, and
`build_for_web.sh --optimize` selects it.

✔ **MEASURED, and it halves the artifact** (`--served --optimize`, 9m02s build):

```text
                     release      web-release
wasm (pre-bindgen)    178 MB          89 MB     -50%
wasm-bindgen out      164 MB          84 MB     -49%
gzip transfer        26.4 MB        14.3 MB     -46%
  code               89.6 MB        50.8 MB     -43%
  custom:name        37.4 MB           GONE    -100%
  data               35.1 MB        31.6 MB     -10%
```

⭐ **so half of "the wasm is enormous" was a build-profile choice, not
architecture** — and the lever was already in the repository, unused. The
browser's parse-and-compile load halves with it.

⛔ **and the other half is now a real number rather than an excuse.** 50.8 MB of
code and 31.6 MB of read-only data SURVIVE fat LTO, `opt-level = "s"` and symbol
stripping. Data barely moved (−10%), which says it is genuine static content —
type names, format and panic strings, tables — not slack the optimiser can take.
Any further reduction is a composition question: what this persona links that it
cannot exercise. That is where a `twiggy`-class breakdown of the surviving code
would earn its keep.

⚠ `--optimize` costs ~9 minutes against a fast dev cycle, so it should not become
the default for iteration. `build_for_web.sh` now names the measured trade in its
own size warning, and stops claiming "no LTO" when the build did use it.

⚠ **35.1 MB of `data` in the SERVED persona looked like embedded assets and is
mostly not.** `visible_web_base` does pull `static_map`, so all four LDtk worlds
bake in even though this persona fetches everything else over HTTP — but measured,
they are **4.2 MB** of it (`sandbox` 2.62, `intro` 0.98, `hall_of_characters`
0.46, `you_have_to_cut_the_rope` 0.18). The other ~31 MB is ordinary Rust
read-only data: panic and format strings, type names, static tables. That scales
with the same monomorphisation bulk as the 89.6 MB code section, which means it
is the same problem and not a separate one — and `opt-level = "s"` + fat LTO is
aimed squarely at it.

✔ **and one hypothesis is already dead: the five demo crates are not bloat.**
`ambition_demo_{sanic,pocket,mary_o,smash,twintrack}` are non-optional
dependencies of `ambition_app`, which looks like exactly the "capability the
persona cannot exercise" pattern — but `shell_host` registers all five
unconditionally, so the browser's launcher lists them and they are reachable.
They are product, not overhead.

Then classify what is found by PORTABILITY: capability the persona cannot
exercise, platform backends pulled into unrelated executables, dev capability
surviving a production persona, broad Bevy feature bundles, duplicated runtime
implementations, and large monomorphised surfaces (which also cost native
compile, link and cold code pages). `twiggy`/`wasm-tools` are fine as
instruments; that does not make the fix wasm-specific. ⛔ do not start a Cargo
feature redesign to chase a byte target.
