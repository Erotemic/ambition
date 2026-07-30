# API reference

Everything a game calls, in one page.

⚠ **This page exists because the SDK was telling readers to do the thing its own
acceptance test measures.** `docs/sdk/README.md` recommended
`cargo doc -p ambition -p ambition_world --no-deps`, and both of blind run 4's
engine opens were exactly that — so the SDK's advice was generating the failures
the SDK is scored on. ADR 0031's gate is that an author never opens a file under
`crates/`; a document that sends them there cannot satisfy it, however useful
rustdoc is afterwards.

Kept honest by `scripts/tests/test_sdk_api_reference_is_current.py`: every
method named here must exist, and every public method must be named here. A
reference that drifts is worse than none, because a reader trusts it.

---

## `PlatformerApp` — the composition

| Method | What it does |
|---|---|
| `windowed(title)` | a game that opens a window |
| `headless()` | no display; one `update()` is one sim tick |
| `without_gpu()` | full render graph, no wgpu backend — for CI and display-less boxes |
| `with_game_assets()` | prepare art on a headless host (a window implies it) |
| `start_at_launcher()` | boot into a launcher over all mounted experiences, not into the first |
| `mount(module)` | fold in a `GameModule`; the FIRST mounted owns the host's home |
| `try_build()` | the `App`, or a `CompositionError` listing every problem at once |
| `build()` | same, panicking with those problems |
| `run()` | build and run |
| `install_into(&mut app)` | add to an `App` you already own — ⚠ cannot register asset sources if `AssetPlugin` already built, and says so |

## `ModuleDraft` — what a module declares

Call `experience(id)` first; everything after it attaches to that experience. A
composition may hold several, keyed by id.

| Method | Required? |
|---|---|
| `experience(id)` | yes — the first call for each experience |
| `gameplay_route(route)` | yes, per experience |
| `launcher_route(route)` | yes, on the FIRST experience (the host's home) |
| `characters(ron)` / `no_characters()` | yes if the composition prepares art |
| `no_audio()` | in practice yes, unless you register a real audio fragment |
| `playable(label, description, starting_character, starting_room, rooms)` | this is what registers the gameplay route |
| `room(metadata)` | optional — picks block/biome art at `Startup` |
| `capability(plugin)` | optional — a Bevy plugin the engine installs in its own order |

## `GameModule` — the trait

```rust
fn manifest(&self) -> ModuleManifest;   // needed BEFORE the Bevy foundation
fn define(&self, module: &mut ModuleDraft);  // never touches `App`
```

`ModuleManifest::new(id)` and `.asset_source(AssetSource::at(scheme, root))` —
the asset source is optional; you need one only if your game ships its own art.

## `HostStatus` — did it start?

`host_status(&app)` returns `NotComposed` / `Initializing` / `Activating` /
`Running { route, experience, prepared }` / `Refused { reasons }`.

| Method | Use |
|---|---|
| `is_running()` | live AND backed by a prepared session — both halves |
| `is_refused()` | it will never start; stop polling |
| `refusal()` | why |
| `route()` | the active route, if any |

## Constants

| Name | For |
|---|---|
| `MINIMAL_CHARACTER_ROSTER_RON` | a working one-character roster; the character it declares is **`my_hero`** |
| `EMPTY_CHARACTER_ROSTER_RON` | the empty case, for `no_characters()`'s shape |

## The modules a game names

| Module | Holds |
|---|---|
| `ambition::app` | everything above, plus `app::prelude` |
| `ambition::world` | rooms and geometry — `world::prelude` is the one to import |
| `ambition::actor` | `PrimaryPlayer`, `BodyKinematics`, spawn requests, ability sets |
| `ambition::sim` | `ControlFrame`, `drive_control_frame`, `WorldTime`, schedule sets |
| `ambition::character` | catalogs, action sets, sheets, brains |
| `ambition::view` | `GameAssets`, `SandboxAssetCatalog`, `RoomVisual` |
| `ambition::bevy` | Bevy itself, re-exported |

Anything else under `ambition::` is an implementation crate this facade mirrors,
carries no stability promise, and is measured as a leak by
`scripts/check_absence_contracts.py`.
