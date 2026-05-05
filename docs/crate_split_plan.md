# Crate split plan: sandbox → game core + sandbox shell

## Why

`ambition_sandbox` started as a debug playground for the engine and has
quietly absorbed enough load-bearing gameplay code (encounter system,
boss phase machine, save schema, quest registry, cutscene player, NPC
AI conversion, F3 stats editor, ledge grab, swim, map menu) that the
real game can no longer treat it as "the test crate." The actual game
binary needs all of that logic but should not depend on the sandbox's
rooms, debug HUD wiring, devtools windows, or hot-reload glue.

This doc lays out the path from the current 2-crate layout
(`ambition_engine` + `ambition_sandbox`) to a 3-crate layout that
matches the real shape of the project today.

## Target layout

```
crates/
├── ambition_engine/      # unchanged role: pure mechanics
├── ambition_game/        # NEW: shippable gameplay crate
│   ├── encounter, boss_encounter, quest, cutscene
│   ├── save, mechanics, dialog routing
│   ├── ledge_grab, swim, map_menu, hostile NPC AI
│   ├── feature_runtime (FeatureEventBus, FeatureRuntime)
│   ├── audio (procedural music + SFX), boss_sprites, character_sprites
│   └── input, rendering primitives shared with sandbox
└── ambition_sandbox/     # SLIM: world/actor authoring + dev shell
    ├── thin lib re-exporting `ambition_game` for `cargo run`
    ├── dev_tools (F3 inspector resources)
    ├── debug_overlay, trace, ldtk hot reload, mechanics registry UI
    ├── pause_menu / inventory test UI, headless binary
    └── tests + content (sandbox.ron, sandbox.ldtk)
```

The split is driven by *consumer surface*, not by file name:
anything the real game binary will eventually link against moves to
`ambition_game`; anything that exists only to drive the sandbox's
testing and authoring loop stays in `ambition_sandbox`.

## What moves to `ambition_game`

These modules have no debug-only callers and are already invoked from
the production gameplay loop:

| Sandbox module | Game crate target | Notes |
| -------------- | ----------------- | ----- |
| `encounter.rs` | `game::encounter` | Encounter registry + Bevy systems are the real wave system. |
| `boss_encounter.rs` | `game::boss_encounter` | Phase state machine bridge. |
| `quest.rs` | `game::quest` | Registry + advance events. |
| `cutscene.rs` | `game::cutscene` | Library, playback runtime, room bindings. |
| `save.rs` | `game::save` | I/O + autosave. (Engine already owns the schema.) |
| `mechanics.rs` | `game::mechanics` | Maturity registry — game-facing. |
| `features.rs` | split — see below | |
| `dialog.rs` | `game::dialog` | Yarn integration is shipped logic. |
| `audio.rs` | `game::audio` | Procedural music is shipped logic. |
| `boss_sprites.rs`, `character_sprites.rs` | `game::sprites` | Shipped art runtime. |
| `ledge_grab.rs`, `swim.rs`, `map_menu.rs` | `game::*` (one module each) | New, already-shipped. |
| `physics.rs` (Avian glue) | `game::physics` | Avian secondary physics. |
| `platforms.rs` | `game::platforms` | Moving platform runtime. |
| `projectile.rs` | `game::projectile` | Player projectile state. |
| `inventory.rs` | `game::inventory` | Slot UI + state. |
| `pause_menu.rs` | `game::pause_menu` | Functional pause; sandbox extends if needed. |
| `settings/` (audio/controls/gameplay/video/persistence) | `game::settings` | Real gameplay settings. |
| `rendering.rs` | split — see below | Game keeps the shared visual primitives. |
| `feel.rs` | `game::feel` | Hitstop / time-scale tuning. |
| `input.rs` | `game::input` | `ControlFrame`, leafwing wiring. |
| `windowing.rs` | `game::windowing` | Window mode hotkeys are gameplay-facing. |
| `lib.rs` core types (`SandboxRuntime`, `GameWorld`, `LedgeGrabState`) | renamed to `GameRuntime` etc. in `ambition_game::lib` | The "Sandbox" prefix is misleading once the real game shares the runtime. |

`features.rs` splits roughly:

- `FeatureRuntime` + the per-feature runtime structs (`HazardRuntime`,
  `EnemyRuntime`, `BossRuntime`, `BreakableRuntime`, `PickupRuntime`,
  `ChestRuntime`, `NpcRuntime`, `SwitchRuntime`, `WaterVolumeRuntime`)
  → `ambition_game::features`.
- `FeatureEventBus` + `drain_feature_event_bus` → `ambition_game::features`.
- `apply_save_to_features` + `sync_features_with_save` → `ambition_game::features`.

## What stays in `ambition_sandbox`

Anything that exists for sandbox-shaped iteration, debugging, or
content authoring:

- `dev_tools.rs` — `EditableAbilitySet`, `EditableMovementTuning`,
  `EditablePlayerStats`, the F3 inspector. The game binary may
  ultimately ship a slimmed inspector for accessibility / cheats; for
  now keeping all of dev_tools sandbox-side avoids leaking egui
  schemas into the shipped game.
- `debug_overlay.rs` — gizmo overlays for collision / hitboxes.
- `trace.rs` — gameplay trace recorder + dump.
- `ldtk_world/` and `ldtk_world.rs` — LDtk hot reload + spine index +
  bevy_ecs_ldtk plumbing. The game ultimately wants pre-baked rooms
  rather than hot-reload off authored LDtk; `ambition_game::world`
  will eventually load a baked format. Until that lands, the LDtk
  loader stays in the sandbox.
- `dummies.rs`, `dev_tools.rs::sync_live_ability_edits` — sandbox-only.
- `headless.rs`, `bin/headless.rs` — the headless harness is a
  sandbox concept; the real game will have its own headless mode if
  ever needed.
- `bin/tune_preview.rs` — music tuning preview tool.
- `assets/ambition/sandbox.ron` and `assets/ambition/worlds/sandbox.ldtk`
  — sandbox content. Real game content lives in a separate assets
  pack the game crate loads.

## The migration in slices

Each slice ships separately and leaves the project compiling /
testing green between commits. Slices are ordered so the most
mechanical work lands first; the design-heavy renames come last.

### Slice 0 — set up the new crate
- Add `crates/ambition_game/` with a stub `lib.rs` that re-exports
  `ambition_engine`.
- Wire it into `Cargo.toml` workspace members.
- Add `ambition_sandbox = { path = "../ambition_sandbox" }` →
  `ambition_game = { path = "../ambition_game" }` for the new
  dependency.
- No code moves yet. Tests stay green.

### Slice 1 — move pure data + headless-friendly modules first
Order chosen so each move has zero or near-zero churn in the
consumer (sandbox) crate.

1. `save.rs` (sandbox-side I/O) → `ambition_game::save`.
2. `mechanics.rs` → `ambition_game::mechanics`.
3. `feel.rs` → `ambition_game::feel`.
4. `data.rs` → `ambition_game::data`.
5. `quest.rs` → `ambition_game::quest`.
6. `cutscene.rs` → `ambition_game::cutscene`.
7. `boss_encounter.rs` → `ambition_game::boss_encounter`.

Each module is a `mv` + a `pub use` shim in the sandbox so external
callers don't break. Once all callers migrate, drop the shim.

### Slice 2 — `features.rs`
The biggest single-file move. Split it before the move:
- Pull `WaterVolumeRuntime`, `FeatureEventBus`,
  `apply_save_to_features`, `sync_features_with_save` out into a
  separate `features/event_bus.rs` and `features/water.rs`.
- Pull `EnemyRuntime`, `BossRuntime`, etc. into per-type files under
  `features/`.
- Then move the whole directory into `ambition_game::features`.

### Slice 3 — `encounter.rs` + dialog + audio
- `encounter.rs` → `ambition_game::encounter`.
- `dialog.rs` → `ambition_game::dialog`.
- `audio.rs` → `ambition_game::audio` (under the `audio` feature).

### Slice 4 — runtime + presentation primitives
- `lib.rs::SandboxRuntime` → `ambition_game::GameRuntime`.
- `lib.rs::GameWorld` → `ambition_game::GameWorld`.
- `lib.rs::LedgeGrabState` → `ambition_game::ledge_grab::LedgeGrabState`.
- `physics.rs`, `platforms.rs`, `projectile.rs`, `rendering.rs` (the
  half that's not debug-only), `boss_sprites.rs`,
  `character_sprites.rs`, `inventory.rs`, `pause_menu.rs`,
  `settings/` → `ambition_game::*`.
- `input.rs` → `ambition_game::input`.
- `windowing.rs` → `ambition_game::windowing`.

### Slice 5 — sandbox cleanup
What's left in `ambition_sandbox` is the dev shell:
- `app.rs` (or rename to `sandbox_app.rs`) calls
  `ambition_game::install_game_plugin` then layers on dev_tools,
  debug overlay, trace recorder, LDtk hot reload, mechanics registry
  UI.
- `dev_tools.rs`, `debug_overlay.rs`, `trace.rs`, `ldtk_world*`,
  `headless.rs`, `bin/headless.rs`, `bin/tune_preview.rs` stay.
- `assets/` continues to ship the test content.

### Slice 6 — name pass
Once the dust settles, rename:
- `SandboxRuntime` → `GameRuntime`.
- `SandboxSave` → `GameSave`. (Save schema stays
  `SandboxSaveData` until/unless we break v2 compatibility.)
- `sandbox_update` → `game_update`.

## Risks

- **Compile time blowup.** The sandbox is already ~10 min cold. A
  fresh `ambition_game` crate that depends on `bevy`, `bevy_kira_audio`,
  `avian2d`, `bevy_ecs_ldtk`, `seldom_state`, `petgraph`, etc. will
  rebuild a lot of those deps separately the first time. Mitigate by
  feature-gating the heavy deps in `ambition_game` the same way
  `ambition_sandbox` already does (the `audio` / `physics_debris` /
  `ldtk_runtime` features pattern transfers as-is).

- **Test surface.** Many sandbox tests use `SandboxRuntime` literals.
  After Slice 4 those need to flip to `ambition_game::GameRuntime`.
  Doable with a sed-friendly rename and a CI green check between
  each slice.

- **LDtk asset paths.** The sandbox's `bevy_ecs_ldtk` config points at
  `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk`. The
  game crate will eventually want its own asset roots; for now leave
  LDtk in the sandbox so we don't break asset loading mid-migration.

- **Per-player state leak.** `SandboxRuntime` is a single global
  resource holding what the architecture targets memory says belongs
  on a Player entity. The crate split is a good moment to take that
  on, but it doubles the diff and risks regressions. Recommendation:
  defer the per-player refactor until the crate split is stable.

## What to do *not* do

- Don't migrate the F3 inspector / egui to the game crate. The
  shipping build wants a different UX (accessibility menu, cheat
  console gated behind dev mode, etc.); reusing the sandbox's
  inspector windows would lock that in.
- Don't try to slim `ambition_sandbox` to a single `app.rs` shell on
  day one. The interim shape is "sandbox imports `ambition_game::*`
  but still owns the test content + dev tools." That intermediate
  shape works fine and unblocks the real game crate.
- Don't merge `ambition_engine` into `ambition_game`. The
  engine-vs-sandbox crate boundary memory keeps the engine free of
  Bevy, audio, and LDtk. That stays.

## Testing strategy

- Each slice keeps the existing 88 + 136 unit tests green.
- Add a smoke test in `ambition_game` that constructs a `GameRuntime`
  + drives a few ticks once Slice 4 lands.
- Re-run the headless binary after every slice.

## When to start

After:
- The remaining engine refactors documented in
  `docs/architecture_targets.md` (events refactor) are *either*
  finished or paused with a clean checkpoint. The crate split shouldn't
  ride the same change as a deep simulation refactor.
- A green build + green CI on `main`.

The work itself is mechanical once those preconditions hold;
reckon two focused agent passes for Slices 0–3 and one for Slices 4–6.
