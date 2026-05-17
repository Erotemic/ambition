# Ambition Sandbox — Web (wasm32) build

Browser build for `ambition_sandbox`. Targets `wasm32-unknown-unknown`
and boots the visible Bevy app inside a `<canvas>` with keyboard input.

There are **two browser personas**, selected by Cargo feature:

| Persona | Cargo feature | `AssetProfile` | Asset source | Use case |
| ------- | ------------- | -------------- | ------------ | -------- |
| **WebStatic** (embedded core) | `--features web` | `WebStatic` | LDtk + a bounded set of UI fonts + primary character sheets + core entity sprites embedded via `include_bytes!`. Out-of-set art falls back to colored rectangles. | Smoke build, single-file demo (~86 MB bg.wasm). |
| **WebServedAssets** (served full game) | `--features web_served_assets` | `WebServedAssets` | LDtk embedded; everything else fetched over HTTP from `/assets/...` served alongside `index.html` via the symlink `crates/ambition_sandbox/web/assets/`. | "Same game in the browser." Smaller wasm (~81 MB bg.wasm); art served separately. |

Common to both:
- `web_platform` — Bevy's `bevy/web` + `bevy/webgl2` + canvas-backed
  winit, plus `wasm-bindgen` / `console_error_panic_hook` for the JS
  bootstrap.
- `static_map` — LDtk JSON `include_bytes!`'d so the LDtk loader
  doesn't need an async fetch.
- `visible_web_base` — `ldtk_runtime`, `input`, `static_map`.

## Subsystem matrix

| Subsystem | WebStatic | WebServedAssets | Notes |
| --------- | --------- | --------------- | ----- |
| LDtk world rendering | ✅ | ✅ | `sandbox.ldtk` + `intro.ldtk` embedded via `AmbitionAssetSourcePlugin` under `static_map`. |
| Keyboard input | ✅ | ✅ | `leafwing-input-manager`. Click the canvas to capture focus. |
| Player + base enemy sprites | ✅ embedded | ✅ served | WebServedAssets fetches `/assets/sprites/player_robot_spritesheet.png` etc. |
| Core entity sprites | ✅ embedded | ✅ served | Same. |
| Out-of-set sprites (breakables, NPCs, parallax, blink walls, ...) | ❌ placeholder | ✅ served | WebServedAssets resolves every entity sprite to a synthesized `BevyPath`; Bevy's wasm HTTP reader fetches it from `/assets/...`. Missing files surface as Bevy load warnings + transparent quads. |
| Parallax layers | ❌ placeholder | ✅ served | WebServedAssets fetches `/assets/backgrounds/parallax_layers/...`. |
| UI fonts | ✅ embedded | ✅ served | InterDisplay + JetBrainsMono. |
| LDtk tiles | ✅ | ✅ | `bevy_ecs_ldtk` loads via the embedded source. |
| Music tracks (`music.track.*`) | ❌ procedural fallback | ⚠️ served path resolves; **playback requires `audio` feature, not yet in `visible_web`** | Catalog hands Bevy a path; the music director silently drops requests because `bevy_kira_audio` isn't compiled into the wasm. See "Audio status" below. |
| SFX bank (`audio.sfx_bank`) | ⚠️ `static_sfx_bank` only | ⚠️ served path resolves; same `audio`-feature caveat | Same. |
| Dev tools (egui inspector, file watcher) | ❌ | ❌ | Excluded from `visible_web_base`. |
| Mobile touch | ❌ | ❌ | `virtual_joystick` excluded. |
| Physics debris | ❌ | ❌ | `avian2d` excluded. |
| Save / settings persistence | ❌ | ❌ | No-op on wasm32; in-memory only. |
| FPS overlay | ✅ default-on | ✅ default-on | Bottom-right Text node, default-on across desktop/web/Android. Toggle from the **Video settings page → "FPS Overlay"** row (persisted in `settings.ron`), or press **F3** for an in-session keyboard toggle that writes the same setting. |

`docs/asset_manager.md` has the full per-asset / per-profile matrix.

## Audio status

Audio (`bevy_kira_audio` + `fundsp` procedural synthesis) is intentionally
excluded from `visible_web` and `visible_web_served` today because the
sandbox-side `audio` feature also pulls a procedural fundsp music
fallback path that costs binary size without playing audibly in the
browser. The catalog already produces correct `/assets/music/...` paths
under `WebServedAssets` — adding `audio` to `visible_web_served` and
verifying browser playback is a follow-up task. Known browser-side
hurdle: Chrome blocks autoplay until a user gesture; the bootstrap
must defer `AudioContext.resume()` until the canvas-click handler.

## One-time setup

```sh
./scripts/setup_web_prereq.sh
./scripts/setup_web_prereq.sh --doctor      # re-check
./scripts/setup_web_prereq.sh --with-server # also install basic-http-server
```

Installs the `wasm32-unknown-unknown` rustup target and a
`wasm-bindgen-cli` pinned to the exact `wasm-bindgen` crate version in
`Cargo.lock`. Mismatched CLI / crate versions are the most common
cause of a "version mismatch" runtime error in the browser.

## Compile-check only (fast feedback)

```sh
cargo check -p ambition_sandbox \
    --target wasm32-unknown-unknown \
    --no-default-features --features web
```

Run after any change that touches the sandbox crate. Pairs with the
desktop `cargo check -p ambition_sandbox` smoke.

## Browser smoke — WebStatic (embedded core)

```sh
./build_for_web.sh --serve          # build + serve at http://localhost:8000/
./build_for_web.sh --serve 9000 --open
./build_for_web.sh --debug --serve  # dev profile (much larger, faster compile)
./build_for_web.sh --doctor         # verify tools + report what would run
```

`build_for_web.sh` (default) runs:
1. `cargo build --release --target wasm32-unknown-unknown --no-default-features --features web`
2. `wasm-bindgen` → `crates/ambition_sandbox/web/pkg/{ambition_sandbox.js, ambition_sandbox_bg.wasm}`
3. `python3 -m http.server -d crates/ambition_sandbox/web 8000` (or
   `basic-http-server` if Python is missing).

## Browser smoke — WebServedAssets (full game, served `/assets/`)

```sh
./build_for_web.sh --served --serve         # build, symlink assets/, serve
./build_for_web.sh --served --debug --serve
```

The `--served` flag flips three things:
- Cargo features: `--features web_served_assets` (drops `static_core_assets`
  to keep the wasm small; selects `AssetProfile::WebServedAssets` at runtime
  via the `web_served` marker).
- Symlinks `crates/ambition_sandbox/assets` into
  `crates/ambition_sandbox/web/assets` so `/assets/...` URLs the page
  fetches actually resolve (falls back to `rsync -a` if symlinks aren't
  available on the filesystem).
- Boot banner reads `AssetProfile = web_served_assets` instead of `web_static`.

Hand-running the equivalent:

```sh
# 1. Build wasm with the served-assets persona.
cargo build -p ambition_sandbox --lib \
    --target wasm32-unknown-unknown \
    --no-default-features --features web_served_assets \
    --release

# 2. Wrap it for the browser.
wasm-bindgen \
    target/wasm32-unknown-unknown/release/ambition_sandbox.wasm \
    --out-dir crates/ambition_sandbox/web/pkg \
    --target web --no-typescript

# 3. Make the page-served `/assets/` URL reachable.
ln -sfn $PWD/crates/ambition_sandbox/assets \
        $PWD/crates/ambition_sandbox/web/assets

# 4. Serve.
python3 -m http.server -d crates/ambition_sandbox/web 8000
```

The same `python3 -m http.server` serves `/`, `/pkg/...`, and
`/assets/...` from one directory.

## WebStatic hand-build (without the helper script)

```sh
cargo build -p ambition_sandbox --lib \
    --target wasm32-unknown-unknown \
    --no-default-features --features web \
    --release
wasm-bindgen \
    target/wasm32-unknown-unknown/release/ambition_sandbox.wasm \
    --out-dir crates/ambition_sandbox/web/pkg \
    --target web --no-typescript
python3 -m http.server -d crates/ambition_sandbox/web 8000
```

### What you should see

Open `http://localhost:<port>/` in any modern browser, click the
canvas to capture keyboard focus, and look for:

1. **Browser console** logs one line like
   ```
   web start: AssetProfile = web_static | static_map = true | static_core_assets = true | static_sfx_bank = false
   ```
   …on a `--features web` build, or:
   ```
   web start: AssetProfile = web_served_assets | static_map = true | static_core_assets = false | static_sfx_bank = false
   ```
   …on a `--features web_served_assets` build. This is the boot
   banner from `run_web`. If `AssetProfile = web_static` and you see
   colored rectangles for chests/players, the build was compiled
   with the wrong feature set.
2. **FPS overlay**, bottom-right corner: `FPS 60  |  frame 16.6ms`
   (rolling average). Default-on across every platform. Hide via
   the Video settings page → "FPS Overlay" row (persisted), or
   press **F3** for an in-session keyboard toggle that writes the
   same setting.
3. **LDtk tiles render** — the active sandbox area paints its tile
   layers (not just background color).
4. **Player spawns** — the bipedal player sprite is visible at the
   authored spawn point (not a magenta rectangle).
5. **Goblin / sandbag spawns** — the goblin enemy + sandbag dummies
   render with their spritesheets.
6. **Core entities render** — chests / pickups / doors / projectiles /
   tiles use real PNGs, not colored rectangles.
7. **UI fonts are real** — the HUD text uses InterDisplay, not the
   Bevy default sans. (Compare against a stock Bevy `Text2d` look.)
8. **Keyboard moves the player** — arrow keys / WASD / jump bindings
   advance the player like the desktop build.
9. **On WebServedAssets**: parallax skyboxes, breakables, NPC sprites,
   etc., should also render — the wasm HTTP reader fetches them from
   `/assets/...`. Watch the Network panel: you'll see GETs like
   `GET /assets/sprites/entities/breakable_intact.png` and the
   request should return 200 (the symlinked `assets/` tree). Missing
   files surface as 404 + a Bevy load warning in the console.
10. **On WebStatic**: out-of-set art (breakables, parallax, NPC
    sheets) still paints as colored rectangles. This is by design;
    only the bounded `static_core_assets` set has authored embedded
    candidates.

### Known browser-side gaps

- **No audio.** Both web personas drop `bevy_kira_audio` from the
  wasm dep graph. Under `WebServedAssets` the catalog still resolves
  `music.track.<id>` and `audio.sfx_bank` to real `/assets/...`
  paths, but the music director skips playback because the audio
  subsystem isn't installed. Enabling `audio` for the web personas
  is a follow-up.
- **No saves.** Settings / sandbox-save persistence is cfg-gated to
  no-op on `wasm32`. Pause-menu toggles work for the session.
- **No hot reload.** `LdtkHotReloadState::from_catalog` reports
  `watch_path = None` under both web personas. Editing
  `sandbox.ldtk` on disk requires a fresh `./build_for_web.sh`.

## How the embedded source plugin works

Bevy's `AssetServer` on wasm doesn't have a host filesystem. Two
sources are wired:

1. **Embedded source** — `AmbitionAssetSourcePlugin::for_profile(...)`
   inserts every embedded core asset's bytes (via `include_bytes!`)
   into Bevy's `EmbeddedAssetRegistry`. The catalog's authored
   `EmbeddedBinary` candidates point at the same `embedded://...`
   URLs, so `try_path_for_load` returns paths that actually load.
2. **HTTP source** — not wired yet (slice 18). The catalog would emit
   `https://...` URLs if any entry authored an `HttpRemote` candidate;
   today none do, so `try_path_for_load` returns `None` for those
   classes on `WebHttp`.

The macro-emitted `register_embedded_core_assets` in
`crates/ambition_sandbox/src/sandbox_assets.rs` is the canonical list
of registered URLs. Adding a new embedded asset is **one row in the
`embed_core_assets!` table** plus one `with_embedded_core_candidate(...)`
call on the corresponding catalog entry. The
`embedded_core_urls_have_authored_catalog_candidates` test catches
mismatches.

## Web entry point (wasm-bindgen)

`run_visible` (the desktop entry point) does CLI parsing, the
no-display-server probe, and a headless fallback. None of that makes
sense in a browser, so it is `#[cfg(not(target_arch = "wasm32"))]`.

The web build enters through `ambition_sandbox::web_start` in
`src/lib.rs`, a `#[wasm_bindgen(start)]` function the browser fires on
its own once the wasm module finishes instantiating. `web_start`:

1. Installs `console_error_panic_hook` so Rust panics surface in
   browser devtools.
2. Calls `app::run_web()`, which builds a Bevy `App` with a
   canvas-pinned `WindowPlugin`, the `SandboxSimulationPlugin` +
   `SandboxLdtkPlugin` + `SandboxPresentationPlugin` trio, and the
   `AmbitionAssetSourcePlugin` (registered AFTER `DefaultPlugins` so
   `AssetPlugin`'s `EmbeddedAssetRegistry` is already present).

## Where things live

- `crates/ambition_sandbox/web/index.html` — page + JS bootstrap.
- `crates/ambition_sandbox/web/pkg/` — generated by `wasm-bindgen` (git-ignored).
- `crates/ambition_sandbox/src/lib.rs` — `web_start` `#[wasm_bindgen(start)]` entry.
- `crates/ambition_sandbox/src/app/cli.rs::run_web` — Bevy `App` builder for the browser.
- `crates/ambition_sandbox/src/sandbox_assets.rs::AmbitionAssetSourcePlugin` — embedded asset registration.
- `crates/ambition_sandbox/src/sandbox_assets.rs::embed_core_assets!` — declarative table of embedded core assets.
- `crates/ambition_sandbox/Cargo.toml` — `web`, `visible_web`, `web_platform`, `static_core_assets` features.
- `scripts/setup_web_prereq.sh` — installs the wasm rustup target + version-matched `wasm-bindgen-cli`.
- `build_for_web.sh` — runs `cargo build` + `wasm-bindgen` + optional `--serve`.
