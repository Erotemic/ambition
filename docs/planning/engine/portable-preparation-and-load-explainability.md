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

## ⛔⛔ 1803 MB of resident images, and nothing in the main world wants the pixels

The same 2026-07-30 capture recorded **1803 MB resident image memory**. On a
browser that is not a stutter, it is a tab that dies — and it is the largest
single number in this campaign.

**The cause is a default.** Bevy loads images with
`RenderAssetUsages::MAIN_WORLD | RENDER_WORLD` (`ImageLoaderSettings::default`
→ `RenderAssetUsages::default`, confirmed in bevy_asset 0.18.1), and Bevy's own
doc says an asset set to `RENDER_WORLD` without `MAIN_WORLD` has its pixel data
dropped from the main world after upload. So every decoded sheet keeps its full
RGBA in main-world RAM for the lifetime of the handle. `report_image_census`
sums exactly that — `image.data.as_ref().map(len)` — which is where 1803 MB was
measured.

⭐ **nothing reads those pixels — but the first audit was too narrow and said
"one reader" when there are seven.** Correcting it, because the difference
decides whether the flag change is one edit or a slice. Every main-world
`Assets<Image>` consumer either writes a runtime-generated image (touch
joysticks, portal view cones, capture readback, morph ball, bubble shield) or
touches a LOADED sheet for one of exactly two reasons:

```text
TEXTURE SIZE, to normalise an atlas frame rect
  clip_material::sprite_frame_basis                      ✔ converted
  hit_flash::current_sprite_uv_rect                      ✔ converted
  deep_dream::current_sprite_uv_rect                     ✔ converted
  demo_mary_o::quasar_shader                             ▢

READINESS — "has this decoded yet", using presence as the signal
  rendering::actors  (three sites)                       ▢
  rendering::actors::boss                                ▢

THE INSTRUMENT
  asset_census::report_image_census                      — measures `data`, must keep it
```

None of them wants a pixel. The size readers had `TextureAtlasLayout::size`
available all along; the readiness guards are asking a question
`AssetServer::is_loaded_with_dependencies` already answers without a main-world
copy — which is exactly how `inspect_room_asset_manifest` asks it.

⚠ **and the size readers are THREE COPIES of one computation**, in three crates,
one of whose docs says it "mirrors the hit-flash overlay's UV resolution" — a
citation, not a mechanism, and citations go stale. Converging them wants a home
both `ambition_render` and `ambition_portal2d_presentation` can reach; the
dependency runs render → portal, so it is neither of them. Named here as the
deletion gate rather than done as a crate move at the end of a long session.

✔ **all three size readers are converted (2026-08-15).** `TextureAtlasLayout`
already carries `size`, so an atlased sprite — every character — resolves its UV
rect without touching `Assets<Image>`. Four systems dropped the parameter
outright. Pinned by a test that resolves a frame whose image handle names nothing
in `Assets<Image>`, with the layout and image sizes deliberately DISAGREEING (the
pre-existing test used 64x32 for both and could not have told which was read).

⭐ the whole-image branch of `hit_flash` is the tell: it computed the texture size
and then returned the constant `(0, 0, 1, 1)` without using it. There the lookup
was pure readiness gating dressed as a size query.

## ⛔⛔ STOP HERE. The harvest is the CONTRACT, not the optimisation.

**Jon, 2026-08-15:** the browser is a powerful architecture test fixture while
the engine is being decomposed; it does not get to decide which subsystem gets
built next. D124 does not become the next campaign.

⛔ **DO NOT make character sheets `RENDER_WORLD`-only.** It is tempting, it is
large, and it is exactly the shape that turns a diagnosis into a month of
performance engineering. It is also unsafe at HEAD: four sites read presence in
`Assets<Image>` as "it decoded", so evicting after extraction would turn
*successfully uploaded* into *never loaded*, forever — characters that vanish the
moment their texture works.

⭐ **the engine-level lesson is a distinction the renderer currently conflates:**

```text
asset loaded / ready     !=     CPU representation resident     !=     GPU resident
```

That is a clean engine concept and it is worth having if the web target vanished
tomorrow. `AssetServer::is_loaded_with_dependencies` already answers the first
question — `inspect_room_asset_manifest` asks it that way — while
`Assets<Image>::get(..).is_some()` answers the second and is being used for the
first. Separating them is a small, bounded change that makes residency policy
something a consuming game can CHOOSE later, rather than something baked into
four render systems.

✔ **THE BOUNDED REMAINDER IS DONE (2026-08-15), and D124 rests here.** All four
readiness sites — `upgrade_actor_sprites`,
`refresh_player_sprites_for_resident_quality`,
`refresh_prop_sprites_on_game_assets_change`, `upgrade_boss_sprites` — ask
`AssetServer::is_loaded_with_dependencies` now, the same question the room
barrier already asks. Behaviour is identical today (an image enters
`Assets<Image>` exactly when it finishes loading, and a failed load is false
under both readings, so the placeholder is held either way); what changed is that
the answer no longer depends on a CPU copy existing.

⚠ **and the first attempt was wrong in a way worth recording.** Asking the asset
server unconditionally broke the quality-convergence fixtures, and the fixture was
right: a handle handed straight to the main world (`reserve_handle`, `add`, a
procedurally generated sprite) has no load to ask about, so the server reports
"never loaded" forever and such a sprite would never bind. **The discriminator is
who OWNS the handle**, and `AssetServer::get_load_state` answers it — `Some(..)`
for anything the server tracks, `None` otherwise. `texture_is_ready` routes on
that: server-owned sheets get the semantic question and survive main-world
eviction; main-world-owned images keep presence as their readiness, which is the
honest answer for a game that builds its own sprite.

⭐ **DELETION PAYOFF: three systems dropped their `Assets<Image>` parameter
outright** (the UV-size readers — hit-flash, deep-dream, portal clip), and the
four binders stopped using presence as a load signal even though they still hold
the resource for the main-world-owned case. The remaining main-world readers of a
LOADED sheet are the image census (instrumentation — reading `data` IS its job)
and the whole-image branch of two UV helpers, which genuinely need a texture's own
dimensions.

⛔ **and that is the stop.** Not the usages flag. Not a residency scheduler. Not
a Hall streaming system. Residency policy is now a choice a consuming game can
make later, which was the whole point of harvesting this as a contract instead of
an optimisation.

⚠ **and do not quote 1803 MB as a Hall baseline.** It came from the 2026-07-30
UNBOUNDED hub-prefetch run and includes that speculative population. It is
evidence that CPU image residency can get enormous; it is not the current cost
of entering the Hall. A before/after would need a current measurement, and taking
one is a later performance campaign's job, not this one's.

### Checked, so nobody re-checks

- readiness polling (`inspect_room_asset_manifest`) asks the `AssetServer`, not
  `Assets<Image>` — unaffected by any usages change;
- `NoWindow` fixtures have no render app, so nothing extracts and nothing is
  evicted — `hall_transition_cover`'s never-settling barrier is unaffected;
- the per-load setting is `asset_server.load_with_settings::<Image,
  ImageLoaderSettings>`, and `load_sprite_pages` is the single site that loads a
  character page.

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

## Executable size: what is solid, and what I got wrong

⚠ **provenance note, because this looked like a measurement error and was not.**
The census below was taken from artifacts this session built; Jon rebuilt the web
bundle concurrently, so `web/pkg/` later held a different (plain-`release`) file —
241.7 MB with a 116.6 MB name section — and the two sets of numbers appeared to
contradict each other. They describe different builds. ⭐ **the lesson is
procedural: a size census names the build that produced it, or it is not a
measurement.** Every figure here is labelled with its profile.

The sizes `build_for_web.sh` printed as it produced each file:

```text
                          rust wasm     wasm-bindgen out
--served (release)          178 MB           164 MB
--served --optimize          89 MB            84 MB      [profile.web-release]
```

⭐ **the conclusion survives: the profile the repository already carries roughly
halves the module a browser must parse and compile**, and it was sitting unused.
There is no `[profile.release]` override, so `release` is Cargo's default with
`strip = "none"`; `[profile.web-release]` sets `strip = "symbols"`, `lto = "fat"`,
`opt-level = "s"`, `codegen-units = 1`.

Section census, each labelled with the build it came from:

```text
                     release(mine)   release(Jon's rebuild)   web-release
total                    163.3 MB           241.7 MB             83.1 MB
  code                    89.6 MB            88.8 MB             50.8 MB
  custom:name             37.4 MB           116.6 MB               GONE
  data                    35.1 MB            35.1 MB             31.6 MB
gzip                      26.4 MB                  -             14.3 MB
```

⭐ `code` and `data` agree across the two `release` builds; only the NAME section
differs, and by 79 MB. That is a debug-symbol volume difference between two
otherwise-identical profiles — worth knowing, and not something to chase here.
`strip = "symbols"` removes the whole question.

⚠ **the transfer figure was stated wrongly and is corrected here.** An earlier
line called a gzip measurement "what a browser actually DOWNLOADS". It is not.
`build_for_web.sh --serve` runs `python3 -m http.server`, which serves the file
raw. Measured against it:

```text
Content-Length: 253401282   (241.7 MB)
Content-Encoding: (none)
Content-type: application/wasm
```

So gzip size is **compression potential for a correctly configured production
server**, not the current transfer. ⛔ and chasing that is deployment work, not
architecture — out of scope here.

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
