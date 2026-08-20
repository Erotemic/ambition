# Portable preparation, and a load barrier that can say what it is waiting for

**D124.** Opened 2026-08-15 from three maintainer observations that only a real
browser has ever made. The frame is **portability, not "optimize wasm"**: a
change earns its place here when it improves desktop, Android, Steam Deck and
the browser together. Brotli, `wasm-opt`, AudioWorklets and cache headers are
measurements; they are not this campaign.

## What Jon observed (2026-08-15, real served build)

- The transition into **Hall of Characters appeared to remain at 99%.**
- The **opening music crackled/distorted and then audibly "caught up"** while
  startup and loading were heavy.
- **Steady-state gameplay after startup settles is substantially better.**

Alongside, and already closed elsewhere: the browser boots and visibly runs
(D121, human-confirmed), and gameplay device input was dead because a developer
instrument owned the latch (D123, awaiting retest).

## ⛔ 99% does not mean "one asset left"

`LoadPresentationModel::from_snapshot` clamps an un-Ready barrier to 0.999 — the
number means only "not yet Ready," nothing about how much remains.

✔ **LANDED 2026-08-15.** A barrier that has not settled a single asset for five
seconds now files one `asset_stall_report` — the room, how long, how many of how
many are outstanding, and the first twelve of them by name — as state on the
transition, so a test can assert it and a dev overlay can show it. ⚠ not a
timeout: nothing is cancelled, because a slow connection legitimately spends
that long on a large room.

## ⛔ The phase timings were compiled out exactly where the stress is

`construction_preflight_duration` and `asset_manifest_duration` were both gated
`#[cfg(not(target_arch = "wasm32"))]` — recorded nowhere on the one platform
that needed them.

✔ **LANDED 2026-08-15**: both now use `bevy::platform::time::Instant` (sub-frame
on every platform), and the `#[cfg]` blocks are deleted. ⚠ `Time<Real>` is NOT a
substitute — it advances once per frame, so a within-frame span measures zero.

⚠ these are write-only diagnostics observed by no simulation decision — timing
data must never enter a deterministic decision.

## The synchronous seam this campaign is really about

```text
contribute_room_transition_assets_system
  → build_room_asset_manifest
      → demand_room_character_sheets
          → materialize_character_demand      (the whole staged cast, synchronously)
```

The manifest must enumerate concrete handles before the reveal barrier can be
built from them. Hall of Characters is the extreme case: its entire point is a
large cast standing together.

**MEASURED 2026-08-15:** `hall_of_characters.ldtk` authors 129 `NpcSpawn` rows
naming 129 distinct character ids; the staged cast reaches ~151; the reveal
barrier holds 164 activation-critical asset handles. Each is its own catalog
lookup, atlas layout, and image handle, materialized in the single frame that
builds the manifest — a stutter on desktop, and in a browser any single
straggler holds the barrier un-Ready with no way to ask which one, which is
what "stuck at 99%" looks like from outside.

⚠ so do not assume the Hall is stuck before the diagnostic exists — a barrier
correctly waiting on the slowest of 129 fetches and one deadlocked on an asset
that will never arrive are indistinguishable without it, and want opposite
fixes.

**MEASURED NATIVELY, 2026-08-15** (`hall_transition_cover`, prints only,
asserts nothing — a threshold would be a performance assertion inside a
correctness test):

```text
[hall-transition] preflight=1.64ms  manifest=18.23ms  barrier=(0, 164)  prefetch_hit=false
```

18ms is one dropped frame on warm desktop; the single-threaded browser decides
whether that's 18ms or 300ms there — the measurement to take on the next
browser run before any redesign.

## The prefetch skips the expensive rooms on purpose

`NEIGHBOR_PREFETCH_ROOM_BUDGET` is 4 rooms; counted at HEAD:

```text
central_hub_main       21 loading zones   <- holds the Hall door
central_hub_basement   18
hall_of_bosses         10
gradient_ascent         6
drain_alley             5
everything else       1-4
```

So the cache always covers cheap corridors, never the destinations behind a
hub.

⛔⛔ **Already tried, measured, and reversed.** Unbounded prefetch from the hub,
2026-07-30: staged cast on entering the Ambition route 162 characters, +157
images/+357.8MP decoded in the 10-15s window, 1803 MB resident image memory, 91
frames in that 5s window (p99 1372ms, max 1437ms).

The door transition's wait is covered (that's what the load foreground is for,
and `hall_transition_cover` pins that it holds); prefetching moves the same
work to a moment when nothing covers it, for rooms the player may never enter.
Trading a covered wait for an uncovered hitch is a straight loss.

⚠ so the budget is correct; the expense itself is the answer — the only move
that helps is making the room's own staging cost smaller or spreading it under
existing cover.

## ⛔⛔ 1803 MB of resident images, and nothing in the main world wants the pixels

Bevy loads images `MAIN_WORLD | RENDER_WORLD` by default, so every decoded
sheet keeps its full RGBA in main-world RAM for the handle's lifetime
(`report_image_census` measured the 1803 MB this way).

Every main-world `Assets<Image>` consumer either writes a runtime-generated
image, or touches a loaded sheet for one of two reasons:

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

None wants a pixel: size readers had `TextureAtlasLayout::size` available all
along; readiness guards are asking a question
`AssetServer::is_loaded_with_dependencies` already answers without a
main-world copy.

✔ **all three size readers converted (2026-08-15).** `TextureAtlasLayout`
carries `size`, so an atlased sprite resolves its UV rect without touching
`Assets<Image>`. Pinned by a test that resolves a frame whose image handle
names nothing in `Assets<Image>`, with layout and image sizes deliberately
disagreeing.

## ⛔⛔ STOP HERE. The harvest is the CONTRACT, not the optimisation.

**Jon, 2026-08-15:** the browser is a test fixture while the engine is being
decomposed; it does not decide which subsystem gets built next. D124 does not
become the next campaign.

⛔ **DO NOT make character sheets `RENDER_WORLD`-only.** Unsafe at HEAD: four
sites read presence in `Assets<Image>` as "it decoded," so evicting after
extraction would turn *successfully uploaded* into *never loaded*, forever.

The distinction the renderer currently conflates:

```text
asset loaded / ready     !=     CPU representation resident     !=     GPU resident
```

`AssetServer::is_loaded_with_dependencies` answers the first;
`Assets<Image>::get(..).is_some()` answers the second and was being used for
the first.

✔ **THE BOUNDED REMAINDER IS DONE (2026-08-15), and D124 rests here.** All four
readiness sites — `upgrade_actor_sprites`,
`refresh_player_sprites_for_resident_quality`,
`refresh_prop_sprites_on_game_assets_change`, `upgrade_boss_sprites` — ask
`AssetServer::is_loaded_with_dependencies` now. Behaviour is identical (an
image enters `Assets<Image>` exactly when it finishes loading, and a failed
load is false under both readings), so residency no longer depends on a CPU
copy existing.

⚠ **the first attempt broke the quality-convergence fixtures**, correctly: a
handle handed straight to the main world (`reserve_handle`, `add`, procedural
sprite) has no load to ask about, so the server reports "never loaded"
forever. **The discriminator is who OWNS the handle** —
`AssetServer::get_load_state` returns `Some(..)` for server-tracked handles,
`None` otherwise; `texture_is_ready` routes on that.

⛔ **and that is the stop.** Not the usages flag. Not a residency scheduler.
Not a Hall streaming system. Residency policy is now a choice a consuming
game can make later.

⚠ **do not quote 1803 MB as a Hall baseline** — it came from the 2026-07-30
unbounded hub-prefetch run and includes that speculative population, not the
current cost of entering the Hall.

### Checked, so nobody re-checks

- readiness polling (`inspect_room_asset_manifest`) asks the `AssetServer`, not
  `Assets<Image>` — unaffected by any usages change;
- `NoWindow` fixtures have no render app, so nothing extracts and nothing is
  evicted — `hall_transition_cover`'s never-settling barrier is unaffected;
- the per-load setting is `asset_server.load_with_settings::<Image,
  ImageLoaderSettings>`, and `load_sprite_pages` is the single site that loads a
  character page.

**Measure before redesigning.** The numbers that still decide the shape: how
many of the demanded characters were already materialized, how many needed
new work, atlas/layout creations, and how much repeats on re-entry.

If measurement confirms a large synchronous burst, the direction is

```text
authored definition → reusable prepared artifact → cache → room/view demand → budgeted realization
```

and NOT a Hall special case. ⛔ do not make `RoomConstructionPlan` partially
authoritative to smooth rendering: deterministic simulation construction and
host presentation realization stay separate.

✔ **ANSWERED 2026-08-15, negative result.** `materialize_declared_character_sprite`
opens with `CharacterSheetState::Ready(_) => return` before any sheet lookup —
a character another room already prepared costs nothing to stage again.
Pinned by `re_demanding_a_resident_character_repeats_no_preparation` (counts
atlas layouts, not wall time), falsified against a disabled short-circuit:
1 → 2 layouts.

So the remaining Hall cost is first-visit work, which wants the first visit
BUDGETED, not repeats memoized.

## What was already found on the way in

**The neighbour prefetch had never prepared a room containing a
character-built body.** It hand-assembled its `ActorConstructionContext`
without the prepared cast or published brain profiles, and
`preflight_planned_bodies` treats an absent registry as an EMPTY cast. So
every such neighbour failed preflight and was re-prepared from scratch every
frame — a whole `RoomConstructionPlan` per neighbour per frame, discarded, for
as long as you stood there — and the prefetch never covered the rooms that
cost the most.

It was the third of seven roads that build this context by hand (startup,
reset, transition, hot reload, provider activation, exclusive-world rebuild,
prefetch); AC6 fixed the exclusive-world one on 2026-08-13, the rest are
uncounted. The authorities are constructor parameters now
(`ActorConstructionContext::for_room_construction`).

## ⛔ MAINTAINER RETEST REQUIRED — the only thing that closes this

Neither line closes because a native benchmark got faster, a test reports
every asset loaded, load coordinator unit tests pass, or the browser links.

- ▢ a real served browser transitions into Hall of Characters, the foreground
  leaves 99%, the Hall becomes playable, and no indefinite readiness stall is
  observed.
- ▢ the opening music does not crackle, race or catch up under browser startup
  load.

## Audio: trace before guessing

⛔ no speculative WebAudio workaround. The trace to establish first: context
unlock → music handle requested → root asset loaded → decoder/source ready →
play command issued → channel begins → the workload during those first
seconds → any playback-rate/seek/resynchronisation command.

The portable contract:

> When music begins, it begins from a stable audio-clock position after its
> required source is ready, and heavy game preparation never causes musical
> time to race forward to catch up.

**Searched at HEAD, 2026-08-15: no music time manipulation exists.**
`playback_rate`, `seek`, `start_from` and siblings return nothing across the
tree; `play_music_track` calls `music_channel.play(handle)` with a 220ms fade
and nothing else. So the leading hypothesis is the opposite: crackle and
"catch up" are what STARVATION sounds like — a stream whose position follows
the audio clock, starved while the main thread does something long, drops
samples then resumes at the position the clock has reached.

⚠ that would make it the SAME defect as the Hall: wasm without threads runs
audio on the main thread, so an unbounded synchronous burst (129
materializations in one frame) starves the mixer directly. Budgeted
realization would fix both.

⛔ do not act on that until measured. `asset_manifest_duration` is now
recorded on wasm; a manifest frame in the hundreds of ms confirms it, one of
20ms sends the search to the audio path proper.

If no portable defect can be proven natively, leave the retest row open with
the logging ready. Do not invent a fix for a symptom that cannot be
reproduced.

## Executable size

⚠ provenance: figures below are each labelled with the build profile that
produced them, after two builds got compared across different profiles by
mistake.

```text
                          rust wasm     wasm-bindgen out
--served (release)          178 MB           164 MB
--served --optimize          89 MB            84 MB      [profile.web-release]
```

The repository's `[profile.web-release]` (`strip = "symbols"`, `lto = "fat"`,
`opt-level = "s"`, `codegen-units = 1`) roughly halves the module a browser
must parse and compile, and it was sitting unused. Plain `release` has
`strip = "none"`.

```text
                     release(mine)   release(Jon's rebuild)   web-release
total                    163.3 MB           241.7 MB             83.1 MB
  code                    89.6 MB            88.8 MB             50.8 MB
  custom:name             37.4 MB           116.6 MB               GONE
  data                    35.1 MB            35.1 MB             31.6 MB
gzip                      26.4 MB                  -             14.3 MB
```

`code` and `data` agree across the two `release` builds; only the
debug-symbol (NAME section) volume differs by 79 MB. `strip = "symbols"`
removes it.

⚠ gzip is compression potential for a correctly configured production server,
not the current transfer: `build_for_web.sh --serve` serves the file raw
(`Content-Encoding: none`, `Content-Length: 253401282` for the 241.7 MB
build). Chasing that is deployment work, out of scope here.

✔ the five demo crates are not bloat: `ambition_demo_{sanic,pocket,mary_o,smash,twintrack}`
are non-optional dependencies of `ambition_app`, and `shell_host` registers
all five unconditionally so the browser's launcher lists them — they are
product.

Classify what portability finds by: capability the persona cannot exercise,
platform backends pulled into unrelated executables, dev capability
surviving a production persona, broad Bevy feature bundles, duplicated
runtime implementations, and large monomorphised surfaces (which also cost
native compile, link and cold code pages). `twiggy`/`wasm-tools` are fine as
instruments; that does not make the fix wasm-specific. ⛔ do not start a
Cargo feature redesign to chase a byte target.
