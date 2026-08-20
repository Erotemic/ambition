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

✔ **LANDED 2026-08-15.** A barrier that has not settled a single asset for five
seconds now files one `asset_stall_report` — the room, how long, how many of how
many are outstanding, and the first twelve of them by name — as STATE on the
transition, so a test can assert it and a dev overlay can show it. ⚠ not a
timeout: nothing is cancelled, because a slow connection legitimately spends that
long on a large room; the report is how a maintainer tells that apart from a
barrier that will never move. Both writers of `last_asset_progress` (the
manifest contributor and the poll) now go through `observe_asset_progress`, the
only way to record progress and therefore the only way to restart the stall
clock — a second writer bypassing it is how this bug happened the first time.

## ⛔ The phase timings are compiled out exactly where the stress is

`construction_preflight_duration` and `asset_manifest_duration` were both
`#[cfg(not(target_arch = "wasm32"))]`-gated, so the browser — the platform whose
numbers would explain the observation — recorded none.

✔ **LANDED 2026-08-15**: both use `bevy::platform::time::Instant` (already in
the dependency graph; sub-frame on every platform), and the `#[cfg]` gates are
deleted. ⚠ `Time<Real>` is NOT a substitute: it advances once per frame, so a
within-frame span measures zero — exactly what the manifest burst is. These are
write-only diagnostics observed by no simulation decision; timing data must never
enter a deterministic decision, which is why they live on the feel clock and are
read only by instruments.

## The synchronous seam this campaign is really about

```text
contribute_room_transition_assets_system
  → build_room_asset_manifest
      → demand_room_character_sheets
          → materialize_character_demand      (the whole staged cast, synchronously)
```

The manifest must enumerate concrete handles before the reveal barrier can be
built from them. Hall of Characters is the extreme customer of that decision —
the room whose entire point is a large cast standing together.

⭐ **MEASURED 2026-08-15.** `hall_of_characters.ldtk` authors **129 `NpcSpawn`**
rows naming 129 distinct character ids; the staged cast the transition demands
reaches **~151**; the reveal barrier holds **164** activation-critical asset
handles. Each of the 129 is its own catalog lookup, atlas layout, and image
handle, materialized in the single frame that builds the manifest; the reveal
barrier waits on all 164 before anything is shown. On desktop with a warm page
cache that is a stutter; in a browser each is an HTTP request, and any single
straggler holds the whole barrier un-Ready — indistinguishable from outside from
"stuck at 99%", with no way to ask which one until the stall report above.

Native measurement (`hall_transition_cover`, which prints these and asserts
nothing about them — a threshold would be a performance assertion inside a
correctness test):

```text
[hall-transition] preflight=1.64ms  manifest=18.23ms  barrier=(0, 164)  prefetch_hit=false
```

18ms of synchronous manifest work is one dropped frame on a warm desktop. The
single-threaded browser decides whether that is 18ms or 300ms — the measurement
to take on the next browser run before any redesign.

## The prefetch skips the expensive rooms ON PURPOSE

`prefetch_hit=false` on the most expensive transition in the game looks like a
cache anti-correlated with cost (`NEIGHBOR_PREFETCH_ROOM_BUDGET` is 4 rooms, and
hub rooms hold 18-21 loading zones each vs. 1-4 for corridors), so the obvious
move is to raise the budget so the Hall gets prefetched from the hub.

⛔⛔ **THAT MOVE WAS ALREADY MADE, MEASURED, AND REVERSED** — unbounded prefetch
from the hub, 2026-07-30, staged 162 characters and drove resident image memory
to 1803 MB with a 1372ms max frame. **A hub is not idle time**: the door
transition's wait is covered (which `hall_transition_cover` pins), but
prefetching moves that same work to a moment when the player is playing and
nothing is covering anything, for up to 21 rooms they may never enter. The
budget is correct; the expense itself is the answer — a room whose staging costs
18ms of CPU and hundreds of MB of resident images is expensive wherever it is
paid, and the only move that helps is making that cost smaller or spreading it
under cover that already exists.

## ⛔⛔ 1803 MB of resident images, and nothing in the main world wants the pixels

The same 2026-07-30 capture recorded **1803 MB resident image memory** — on a
browser that is not a stutter, it is a tab that dies.

**The cause is a default.** Bevy loads images with
`RenderAssetUsages::MAIN_WORLD | RENDER_WORLD` (confirmed in bevy_asset 0.18.1),
so every decoded sheet keeps its full RGBA in main-world RAM for the life of the
handle. `report_image_census` measures exactly that (`image.data.as_ref().map(len)`).

Every main-world `Assets<Image>` consumer either writes a runtime-generated
image (unaffected) or touches a loaded sheet for one of two reasons:

```text
TEXTURE SIZE, to normalise an atlas frame rect
  clip_material::sprite_frame_basis, hit_flash::current_sprite_uv_rect,
  deep_dream::current_sprite_uv_rect                      ✔ converted
  demo_mary_o::quasar_shader                               ▢

READINESS — "has this decoded yet", using presence as the signal
  rendering::actors (3 sites), rendering::actors::boss     ✔ converted

THE INSTRUMENT
  asset_census::report_image_census                        — measures `data`, must keep it
```

None of them wants a pixel. ✔ **All three size readers converted (2026-08-15)**:
`TextureAtlasLayout` already carries `size`, so an atlased sprite resolves its UV
rect without touching `Assets<Image>`; pinned by a test that resolves a frame
whose image handle names nothing in `Assets<Image>`, with layout and image sizes
deliberately disagreeing (falsified: the pre-existing test used 64x32 for both
and could not have told which was read).

## ⛔⛔ STOP HERE. The harvest is the CONTRACT, not the optimisation.

**Jon, 2026-08-15:** the browser is a powerful architecture test fixture while
the engine is being decomposed; it does not get to decide which subsystem gets
built next. D124 does not become the next campaign.

⛔ **DO NOT make character sheets `RENDER_WORLD`-only.** It is unsafe at HEAD:
four sites read presence in `Assets<Image>` as "it decoded", so evicting after
extraction would turn *successfully uploaded* into *never loaded*, forever.

**The engine-level lesson** is a distinction the renderer currently conflates:

```text
asset loaded / ready     !=     CPU representation resident     !=     GPU resident
```

`AssetServer::is_loaded_with_dependencies` answers the first question;
`Assets<Image>::get(..).is_some()` answers the second and was being used for the
first. Separating them makes residency policy something a consuming game can
choose later, rather than something baked into four render systems.

✔ **THE BOUNDED REMAINDER IS DONE (2026-08-15), and D124 rests here.** All four
readiness sites — `upgrade_actor_sprites`,
`refresh_player_sprites_for_resident_quality`,
`refresh_prop_sprites_on_game_assets_change`, `upgrade_boss_sprites` — ask
`AssetServer::is_loaded_with_dependencies` now, the same question the room
barrier already asks. The discriminator for which question to ask is **who OWNS
the handle**: `AssetServer::get_load_state` returns `Some(..)` for anything the
server tracks and `None` otherwise. `texture_is_ready` routes on that —
server-owned sheets get the semantic question and survive main-world eviction;
main-world-owned images (`reserve_handle`, `add`, procedurally generated
sprites) keep presence as their readiness, the honest answer since they have no
load to ask about.

⭐ Three systems dropped their `Assets<Image>` parameter outright (the UV-size
readers). The remaining main-world readers of a loaded sheet are the image
census (instrumentation — reading `data` IS its job) and the whole-image branch
of two UV helpers, which genuinely need a texture's own dimensions.

⛔ **and that is the stop.** Not the usages flag. Not a residency scheduler. Not
a Hall streaming system.

⚠ **do not quote 1803 MB as a Hall baseline.** It came from the 2026-07-30
UNBOUNDED hub-prefetch run and includes that speculative population; it is
evidence CPU image residency can get enormous, not the current cost of entering
the Hall. A before/after needs a current measurement, which is a later
performance campaign's job.

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

✔ **ANSWERED 2026-08-15 — the existing cache semantics, a negative result.**
`materialize_declared_character_sprite` opens with `CharacterSheetState::Ready(_)
=> return`, before any sheet lookup, atlas build or handle request: a character
another room already prepared costs nothing to stage again. Pinned by
`re_demanding_a_resident_character_repeats_no_preparation`, which counts atlas
layouts rather than wall time, falsified against a disabled short-circuit: **1 →
2 layouts.**

⭐ **so the remaining Hall cost is genuinely FIRST-VISIT work** — it wants the
first visit budgeted, not repeats memoized (opposite fixes; building the cache
that already exists would have changed nothing).

## What was already found on the way in

**The neighbour prefetch had never prepared a room containing a
character-built body.** It hand-assembled its `ActorConstructionContext` and
passed neither the prepared cast nor the published brain profiles, and
`preflight_planned_bodies` treats an absent registry as an EMPTY cast; every
such neighbour failed preflight, was forgotten, and was re-prepared from scratch
on the next frame — a whole `RoomConstructionPlan` per neighbour per frame,
discarded, for as long as you stood there, and the prefetch never covered the
rooms that cost the most to prepare.

Of the seven roads that build this context by hand (startup, reset, transition,
hot reload, provider activation, the exclusive-world rebuild, the prefetch), AC6
fixed the exclusive-world one on 2026-08-13; the rest are unaudited. The
authorities are constructor parameters now
(`ActorConstructionContext::for_room_construction`), so the next one a room may
consult is a signature change that breaks every road at once.

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

The portable contract:

> When music begins, it begins from a stable audio-clock position after its
> required source is ready, and heavy game preparation never causes musical time
> to race forward to catch up.

⭐ **Searched at HEAD, 2026-08-15: there is no music time manipulation to find.**
`playback_rate`, `seek`, `start_from` and every sibling return nothing across the
tree; `play_music_track` calls `music_channel.play(handle)` with a 220ms fade and
nothing else. So the leading hypothesis is the opposite direction: **crackle and
"catch up" are what STARVATION sounds like.** A stream whose position follows the
audio clock, starved while the main thread does something long, drops samples
(the crackle) and then resumes at the position the clock has already reached
(the "catch up") — nothing in the game decided that; the game simply stopped
feeding it.

⚠ **and that makes it the SAME defect as the Hall, not a second one.** A wasm
build without threads runs audio callbacks on the main thread, so an unbounded
synchronous burst — 129 character materializations in one frame — starves the
mixer directly. If that is what is happening, budgeted realization fixes both
symptoms and no audio-specific change is needed.

⛔ **do not act on that until it is measured.** `asset_manifest_duration` is now
recorded on wasm, so a browser run will say how long that frame actually took. A
manifest frame in the hundreds of milliseconds confirms it; a manifest frame of
20ms falsifies it and sends the search to the audio path proper. If no portable
defect can be proven natively, leave the retest row open with the logging ready.

## Executable size

⚠ **a size census names the build that produced it, or it is not a
measurement.** Every figure here is labelled with its profile.

```text
                          rust wasm     wasm-bindgen out
--served (release)          178 MB           164 MB
--served --optimize          89 MB            84 MB      [profile.web-release]
```

Section census, each labelled with the build it came from:

```text
                     release           web-release
total                    163.3 MB           83.1 MB
  code                    89.6 MB           50.8 MB
  custom:name             37.4 MB            GONE
  data                    35.1 MB           31.6 MB
gzip                      26.4 MB           14.3 MB
```

⭐ **the repository's own `web-release` profile roughly halves the module a
browser must parse and compile**, and it was sitting unused. There is no
`[profile.release]` override, so `release` is Cargo's default with `strip =
"none"`; `[profile.web-release]` sets `strip = "symbols"`, `lto = "fat"`,
`opt-level = "s"`, `codegen-units = 1` — `strip = "symbols"` alone removes the
whole 37 MB custom:name section.

⚠ gzip size is compression potential for a correctly configured production
server, not the current transfer — `build_for_web.sh --serve` serves the file
raw via `python3 -m http.server` with no `Content-Encoding`. ⛔ chasing transfer
config is deployment work, not architecture — out of scope here.

✔ **the five demo crates are not bloat.**
`ambition_demo_{sanic,pocket,mary_o,smash,twintrack}` are non-optional
dependencies of `ambition_app`; `shell_host` registers all five
unconditionally, so the browser's launcher lists them and they are reachable —
product, not overhead.

Classify what is found by PORTABILITY: capability the persona cannot exercise,
platform backends pulled into unrelated executables, dev capability surviving a
production persona, broad Bevy feature bundles, duplicated runtime
implementations, and large monomorphised surfaces (which also cost native
compile, link and cold code pages). `twiggy`/`wasm-tools` are fine as
instruments; that does not make the fix wasm-specific. ⛔ do not start a Cargo
feature redesign to chase a byte target.
