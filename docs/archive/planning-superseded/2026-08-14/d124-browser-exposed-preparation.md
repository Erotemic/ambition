# D124 — what the browser exposed (BOUNDED AND RESTING, 2026-08-14)

⚠ **EVIDENCE, NOT AUTHORITY.** Jon bounded this thread deliberately: *"the browser
is an architecture TEST FIXTURE while the engine is decomposed; it does NOT decide
which subsystem gets built next."* ⭐ his test for any tempting performance task:
**would we want this abstraction if the web target disappeared tomorrow?** ⛔ the
answers filed as NO — Brotli, wasm audio scheduling, Hall streaming, a generic
residency scheduler, byte shaving, production HTTP compression — are not to be
revived from this file.

- ▢ **D124 — What the browser exposed that desktop was hiding: bursty
  preparation, an unexplainable load barrier, and startup audio.**

See
[`engine/portable-preparation-and-load-explainability.md`](engine/portable-preparation-and-load-explainability.md).

⛔ **THREE MAINTAINER OBSERVATIONS THAT ONLY A BROWSER HAS MADE, and none of them
may be closed by a green native suite.** From Jon's 2026-08-15 session: (a) the
transition into Hall of Characters appeared to remain at 99%; (b) the opening
music crackled/distorted and then audibly "caught up" during heavy startup; (c)
steady-state gameplay after startup settles is substantially better. ⚠ **99% does
not mean "one asset left"** — `LoadPresentationModel::from_snapshot` clamps an
un-Ready barrier to `0.999`, so the number means only *"the barrier has not
reached Ready"*, and the engine currently cannot say what it is waiting for:
`RoomAssetReadiness` computed `pending: Vec<String>` and `failed: Vec<String>`
and `ActiveRoomTransitionLoad` kept only `(settled, total)`, so the names were
computed and dropped every frame. **A load coordinator that knows it is blocked
must be able to say what on.** ✔ **that half landed 2026-08-15**: a barrier that
settles nothing for five seconds files one `asset_stall_report` naming the room,
the duration, the outstanding count and the first twelve pending assets — state a
test asserts and an overlay can show, not just a log line, and not per-frame. ⚠
not a timeout; nothing is cancelled. ⭐ measured, and name the denominator because
there are three: the Hall authors **129 `NpcSpawn` rows naming 129 DISTINCT
character ids**, stages **~151**, and produces a barrier of **164** asset handles
— every character its own layout and image, materialized in the frame that builds
the manifest. ⭐ the pin caught the trap immediately: `last_asset_progress` has TWO
writers and the stall clock was taught one, so the Hall sat at `(0, 164)` unable
to ever become old enough to explain itself; both go through
`observe_asset_progress` now. Phase timings are portable too —
`bevy::platform::time::Instant` replaces the `#[cfg(not(wasm32))]`
`std::time::Instant`, so the browser records the numbers that would explain it.
⭐⭐ **measured natively: preflight 1.64ms, manifest 18.23ms, barrier (0, 164),
`prefetch_hit=FALSE`.** `NEIGHBOR_PREFETCH_ROOM_BUDGET` is 4 ROOMS;
`central_hub_main` has **21 loading zones** and holds the Hall door. That reads
like a cache anti-correlated with cost, and ⛔⛔ **the obvious fix was already
made, measured and REVERSED — the constant's own doc holds the numbers**:
unbounded hub prefetch cost **p99 1372ms / max 1437ms frames and 1803 MB
resident images** (2026-07-30, Jon's timeline capture). ⭐ **a hub is not idle
time**: the door's wait is COVERED by the load foreground, and prefetching moves
that work to a moment when nothing is covering anything, for up to 21 rooms the
player may never enter. Trading a covered wait for an uncovered hitch is a
straight loss. **So the budget is right and the expense itself is the target** —
18ms of CPU and hundreds of MB of images is expensive wherever it is paid, and on
a browser 1803 MB is not a stutter but a dead tab. ⭐⭐ **SIZE CENSUS DONE.** `--served` at HEAD: 178MB rust wasm → 164MB
wasm-bindgen → **26.4MB gzipped** (so "220MB" was never a NETWORK problem; the
164MB is what the browser must PARSE AND COMPILE). Sections: code 89.6MB,
`custom:name` **37.4MB of debug symbol names in a release build**, data 35.1MB.
⛔ there is no `[profile.release]` override, so `release` is Cargo's default with
`strip = "none"` — and `[profile.web-release]` (fat LTO, `opt-level="s"`,
`strip="symbols"`) has sat in the root `Cargo.toml` unused. Measured with
`--optimize`: **164MB → 84MB, 26.4 → 14.3MB gzipped, the name section GONE**, for
9 minutes of build. ⭐ so half of it was a profile choice, not architecture. What
SURVIVES full LTO — 50.8MB code, 31.6MB data (data barely moved, −10%) — is the
real number, and reducing it is a composition question. ✔ dead hypotheses: the
five demo crates are reachable product, not bloat; and `static_map`'s baked
worlds are 4.2MB of that data section, not 35. ⛔⛔ **D124 IS NOW BOUNDED AND RESTING (Jon, 2026-08-15): the browser is an
architecture TEST FIXTURE while the engine is decomposed, not the thing that
decides the next subsystem.** The harvest was a contract the renderer conflated —
`asset loaded/ready != CPU resident != GPU resident` — and SEVEN systems dropped
their `Assets<Image>` parameter: four readiness sites now ask
`AssetServer::is_loaded_with_dependencies` (the same question the room barrier
asks), three UV-size readers take `TextureAtlasLayout::size`. Residency policy is
a consuming game's choice now. ⛔ NOT taken and explicitly out of scope: the
`RENDER_WORLD`-only usages flag (unsafe until those four moved, and a performance
campaign after that), a per-frame preparation scheduler, Hall streaming, audio
architecture, byte shaving, HTTP compression. ⚠ 1803MB is NOT a Hall baseline —
it came from the unbounded hub-prefetch run. **What stays open is only the two
maintainer retests above.** Next work is D116 M2, not this.

⭐ **the frame is PORTABILITY, not "optimize wasm".** The browser is the harshest
probe of work desktop hides; a fix earns its place when it improves desktop,
Android, Steam Deck and web together. Brotli, `wasm-opt`, AudioWorklets and cache
headers are measurements, not the campaign. ⚠ the phase timings that would answer
(a) are `#[cfg(not(target_arch = "wasm32"))] Instant::now()` — compiled out
exactly where the stress signal is strongest.

⛔ **MAINTAINER RETEST REQUIRED, and these two lines are the completion
condition** — do NOT check them because a native benchmark got faster, because a
test reports every asset loaded, or because the load coordinator's unit tests
pass: (1) a real served browser transitions into Hall of Characters, the
foreground leaves 99%, the Hall becomes playable, and no readiness stall is
observed; (2) the opening music does not crackle, race or catch up under browser
startup load.

