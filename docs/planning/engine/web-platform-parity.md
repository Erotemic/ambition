# Web platform parity — the browser ran a different application

> **Opened 2026-08-14** from Jon's report: the wasm builds and links, the page and
> canvas load, the overlay says *"click the canvas to capture keyboard"* and never
> clears, and **no shell or game presentation ever appears**. Assets return `200`;
> only `.meta` sidecar probes 404.
>
> **Closed 2026-08-14.** Four separate defects, not one. Each is measured below.

## ⭐ THE CAUSE WAS MEASURED, AND THE SOURCE HAD ALREADY NAMED IT

`run_web()` composed the plugin trio and ran. `build_visible_app_with` composed
all of that **and three more things**:

```text
app.insert_resource(AmbitionShellHosted)              ⟵ absent on web
compose_ambition_shell_host / ..._booting_to(app)     ⟵ absent on web
install_ambition_shell_visuals(app)                   ⟵ absent on web
```

No shell host ⇒ no route. No route ⇒ nothing to boot into. No
`install_ambition_shell_visuals` ⇒ no room. That is the blank canvas.

⛔⛔ **and `build_visible_app_with`'s own doc comment described this exact
failure, from the last time somebody hand-spelled the composition:**

> *`capture_scene` therefore spelled the whole composition out by hand — and the
> hand-spelled copy silently lost the `--route` positional, the headless display
> surface, `--dev-overlays`, `--combat-overlay`, and (2026-08-06 → 08-08, for two
> days) **the entire room**, because `install_ambition_shell_visuals` was never
> added to it.*

Two hand-spelled copies, two silent blanks, one lesson written down between them
and not acted on. **A build gate proves *links*. Nothing proved *composes*.**

## What landed

### 1. One visible composition — `game/ambition_app/src/app/visible_composition.rs`

```text
platform host foundation        ← DIFFERS: a desktop window, a no-window render
                                  graph, an offscreen GPU surface, a browser
                                  <canvas>; arg parsing; asset roots; the
                                  persistence/clock/audio policy of a
                                  non-session process
            ↓
compose_ambition_visible_game   ← IDENTICAL: engine states, the simulation host,
                                  the game plugins, the shell host, the initial
                                  route, the shell visuals, the asset source
            ↓
platform run                    ← DIFFERS: App::run in a binary, app.update in a
                                  test; the native builder RETURNS the app
```

`VisibleGameSpec` carries the four things hosts genuinely differ on
(`shell_hosted`, `tile_spine`, `startup_loading_curtain`, `asset_config`), and
`VisibleGameSpec::browser()` names the browser persona as a **value** rather than
a passage inside a `cfg(wasm32)` function no native test can reach. `run_web` is
now a foundation plus one call.

Deletion payoff: the third hand-spelled composition stopped existing, and
`set_simulation_host` / `insert_starting_character_override` /
`InitialGameplayReadiness::closed()` each went from two copies to one.

### 2. The browser registered no `game://` asset source

Found while measuring #1. Every world is addressed `game://worlds/<file>` and the
vanity card its own art the same way; on wasm that source was never registered,
so those loads resolved through a source that did not exist. `static_map` hid it
for the worlds — the embedded fallback answered — and nothing hid it for anything
else.

Both roots are ONE root in a packaged build, which is the case
`layered_asset_source` already documents ("the packager has already merged the
trees… with one root there is nothing to fall back to, so the platform default IS
the correct reader"). The browser now says that rule as the platform default it
reduces to — spelled directly because `consumer_source` is
`not(target_arch = "wasm32")`, being built on `FileAssetReader`.

### 3. Web packaging published one implementation crate's `assets/`

`build_for_web.sh --served` symlinked `web/assets` →
`crates/ambition_platformer2d_actor_monolith/assets`. Wrong twice: it named a
crate the decomposition is dismantling, and it published only ONE of the two
roots. **Measured: the served tree had no `worlds/` directory at all** — every
`.ldtk` fetched over HTTP would have 404'd.

`scripts/package_asset_guard.py` is already the single seam that collapses the
roots, forbids implicit overrides, and emits a byte contract; Android verifies it
against the APK and the Steam Deck deploy after rsync. Web is now its fourth
consumer and names no crate. Jon's acceptance condition — *the served-web path
keeps working if the actor-monolith crate is renamed or deleted* — holds.

`--materialize link` is new and dev-loop only: the composed tree is 1.1 GB across
4485 files. The contract and its full hash audit are unchanged; only the bytes
are shared. A linked **directory** stays forbidden in both modes, because that is
what takes a subtree out of the contract's control — precisely the shape the old
web symlink had.

Measured: 4485 files, both roots, contract verified, 7.9 s.

### 4. The page's status line was not truthful

It announced "loaded · click the canvas to capture keyboard" the instant
`init()` resolved and never changed again — surviving the click, reading as a
live instruction, and promising a Pointer Lock capture the app never requests.
It now reports only what the page can observe (module instantiated; canvas
focused or not), fades out once keyboard focus is on the canvas, and never
speaks about game readiness, which the DOM has no view of. A startup failure now
interrupts instead of whispering at 12px in a corner.

## What pins it

- `tests/visible_composition_contract.rs` — the composition contract measured on
  a built App: `AmbitionShellHosted`, a non-empty initial route, and
  `SessionRoomVisualsPlugin`. Each names something whose absence produces a
  *silently blank* app. Its `shell_hosted` is read off
  `VisibleGameSpec::browser`, so the browser and the test cannot drift apart
  without the shared spec changing under both. ⛔ carries its own poison: a bare
  `App` must report every probe as missing, or the assertions pin nothing.
- `tests/asset_id_platform_parity.rs` — hold the PRODUCTION manifest fixed, vary
  only the profile, ask whether the platform changes which file an `AssetId`
  names. **All 967 entries agree**: 963 resolve to a byte-identical relative
  path, and the four `game://worlds/*.ldtk` differ only in delivery (an absolute
  dev path the file watcher can see, versus the copy embedded in the wasm). ⛔ the
  suffix comparison carries its own poison: a match without a path boundary would
  call `miniboss.png` and `boss.png` the same file.

## What was verified over HTTP

`./build_for_web.sh --served --skip-build` ran the whole packaging path
(wasm-bindgen 0.2.120 → 163 MB module + 112 KB JS; publication → 4485 files,
contract verified), and the tree was served with `python3 -m http.server` and
probed:

| URL | before | after |
|---|---|---|
| `assets/worlds/sandbox.ldtk` | **404** — not in the published tree at all | **200**, 2.7 MB |
| `assets/worlds/hall_of_characters.ldtk` | **404** | **200**, 481 KB |
| `assets/ambition/platformer_defaults.ron` | 200 | 200, 2.9 KB |
| `assets/audio/sfx.bank` | 200 | 200, 31 MB |
| `assets/fonts/bundled/InterDisplay-SemiBold.otf` | 200 | 200, 625 KB |
| `assets/sprites/judy_spritesheet.png` | 200 | 200, 468 KB |
| `assets/backgrounds/parallax_layers/forest_near_background.png` | 200 | 200, 197 KB |
| `assets/audio/music/generated/burn_rate_bossa/full.ogg` | 200 | 200, 1.97 MB |
| `assets/sprites/judy_spritesheet.png.meta` | 404 | **404 — unchanged, deliberately** |

⚠ **what this does NOT show.** No browser is installed on this machine, so
nothing here observed `App::run()` being entered, a state after startup, or a
frame drawn. The composition is measured behaviourally on a native App
(`visible_composition_contract`), the artifact is measured by a release link,
and the transport is measured by the table above — but the last step, a human
seeing the launcher, is Jon's.

## ⚠ NOT the bug

`.meta` 404s beside `200`s for the real asset are Bevy probing optional metadata
sidecars: `AssetMetaCheck` is never configured anywhere in the tree, so the
default `Path` behaviour applies, and **zero `.meta` files exist under either
asset root** — none is expected, none is generated, and a 404 means "no meta,
use defaults" rather than a failed load. The favicon 404 is not game-blocking.
Neither explains a blank canvas. Left alone deliberately: `AssetMetaCheck::Never`
would silently disable any future asset processing, and a clean HTTP log is not
a goal.

## §10 — why this survived

The dates answer it exactly, and the answer is not carelessness.

| | |
|---|---|
| `run_web` born | **2026-05-16** (`first-pass wasm32 / browser build`) |
| `web/assets` symlink born | **2026-05-17** (`WebServedAssets profile + --served`) |
| `package_asset_guard.py` born | **2026-07-21** — two months later |
| shell host + `install_ambition_shell_visuals` in the native builder | later still (K2b) |

**Both web paths were faithful copies on the day they were written.** In May
there was no shell host to compose and no second asset root to publish. Every
subsequent architectural addition went to the native path, and nothing existed
that would notice the browser had been left behind — because the only web
coverage was a compile, and later a link, both of which stayed green throughout.

That is the argument for the shape of the repair rather than for a patch: a
second builder does not drift because someone is careless, it drifts because
being a second builder is what drift *is*.

## §9 — declarative platform policy, without pre-generalizing

`VisibleGameSpec` is the seam a persona is expressed through, and
`VisibleGameSpec::browser()` is the first one to name itself. An Android or
Steam Deck persona can be a second and third constructor beside it — host
integration, asset profile, quality profile, and input capabilities selected
independently, over one unchanged game composition.

⛔ **the other constructors were deliberately NOT written.** The native builder
has one call site and derives its spec from `(render, shell_hosted)`; adding
four unused constructors to look declarative is exactly the pre-generalizing the
engine direction warns against. The shape is reachable when a real second
platform asks for it.

## Still open

- ▢ **Jon has not yet confirmed the browser** shows the launcher. Everything
  above is measured on the composition, the publication, and the link; the last
  step is a human opening the page. `./build_for_web.sh --served --serve`.
- ▢ **The `web` (non-served, embedded-assets) persona is unaudited for #3.** It
  embeds rather than serves, so the publication repair does not touch it, and no
  measurement here says whether its embedded set is complete.
