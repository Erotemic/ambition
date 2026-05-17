# Ambition Sandbox — Web (wasm32) build

Browser build for `ambition_sandbox`. Targets `wasm32-unknown-unknown`
and boots the visible Bevy app inside a `<canvas>` with embedded
core visual + LDtk assets and keyboard input.

The `web` Cargo feature composes:
- `visible_web` — gameplay subsystems that work in a browser (LDtk
  runtime, leafwing input, the catalog-driven loaders, `static_map`,
  `static_core_assets`).
- `web_platform` — Bevy's `bevy/web` + `bevy/webgl2` + canvas-backed
  winit, plus `wasm-bindgen` / `console_error_panic_hook` for the JS
  bootstrap.

## Scope (current as of slice 15)

| Subsystem | Status | Notes |
| --------- | ------ | ----- |
| LDtk world rendering | ✅ | `sandbox.ldtk` + `intro.ldtk` embedded via `AmbitionAssetSourcePlugin` under `static_map`. |
| Keyboard input | ✅ | `leafwing-input-manager`. Click the canvas to capture focus. |
| Player + base enemy sprites | ✅ | `player_robot`, `robot`, `goblin`, `sandbag` spritesheets embedded under `static_core_assets`. |
| Core entity sprites | ✅ | Chests, pickups, doors, projectile, solid / one-way / hazard tiles, boss core embedded under `static_core_assets`. |
| UI fonts | ✅ | Dialog regular/semibold + debug mono embedded under `static_core_assets`. |
| LDtk tiles | ✅ | `bevy_ecs_ldtk` loads via the embedded source. |
| Out-of-set sprites (breakables, NPCs, parallax, blink walls, etc.) | ❌ | No `EmbeddedBinary` candidate yet → renderer paints colored rectangles. Slice 16/17 work. |
| Audio (music + SFX) | ❌ | `kira` / `fundsp` are not in the wasm dep graph; the music director falls back to procedural synth when the catalog returns `None`. |
| Dev tools (egui inspector, file watcher) | ❌ | Excluded from `visible_web`. |
| Mobile touch | ❌ | `virtual_joystick` excluded. |
| Physics debris | ❌ | `avian2d` excluded. |
| Save / settings persistence | ❌ | No-op on wasm32; in-memory only. |

`docs/asset_manager.md` has the full per-asset / per-profile matrix.

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

## Browser smoke (build + serve + open)

```sh
./build_for_web.sh --serve          # build + serve at http://localhost:8000/
./build_for_web.sh --serve 9000 --open
./build_for_web.sh --debug --serve  # dev profile (much larger, faster compile)
./build_for_web.sh --doctor         # verify tools + report what would run
```

`build_for_web.sh` runs:
1. `cargo build --release --target wasm32-unknown-unknown --no-default-features --features web`
2. `wasm-bindgen` → `crates/ambition_sandbox/web/pkg/{ambition_sandbox.js, ambition_sandbox_bg.wasm}`
3. `python3 -m http.server -d crates/ambition_sandbox/web 8000` (or
   `basic-http-server` if Python is missing).

Hand-running the equivalent:

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
   This is the boot banner from `run_web`. If `static_map = false` or
   `static_core_assets = false`, the build was compiled with the
   wrong feature set — the visible art will be missing.
2. **LDtk tiles render** — the active sandbox area paints its tile
   layers (not just background color).
3. **Player spawns** — the bipedal player sprite is visible at the
   authored spawn point (not a magenta rectangle).
4. **Goblin / sandbag spawns** — the goblin enemy + sandbag dummies
   render with their spritesheets.
5. **Core entities render** — chests / pickups / doors / projectiles /
   tiles use real PNGs, not colored rectangles.
6. **UI fonts are real** — the HUD text uses InterDisplay, not the
   Bevy default sans. (Compare against a stock Bevy `Text2d` look.)
7. **Keyboard moves the player** — arrow keys / WASD / jump bindings
   advance the player like the desktop build.
8. **Out-of-set art still falls back** — breakables, parallax skyboxes,
   NPC sprites paint as colored rectangles. This is by design until
   their `EmbeddedBinary` candidates land.

### Known browser-side gaps

- **No audio.** The wasm dep graph drops `bevy_kira_audio`, so the SFX
  director and music director run in silent fallback mode. The
  catalog still reports the music track and SFX bank entries; they
  just don't load through `AssetServer`.
- **No saves.** Settings / sandbox-save persistence is cfg-gated to
  no-op on `wasm32`. Pause-menu toggles work for the session.
- **No hot reload.** `LdtkHotReloadState::from_catalog` reports
  `watch_path = None` under `WebStatic`. Editing `sandbox.ldtk` on
  disk requires a fresh `./build_for_web.sh --serve`.

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
