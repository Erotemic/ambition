# 2026-05-20: Bevy `ui_api` transitively re-enables `bevy_winit`, so moving render features off the base dep isn't enough for a true headless build

OVERNIGHT-TODO #1 ("Make `headless` / minimal feature builds real") starts
from the observation that `crates/ambition_sandbox/Cargo.toml` declares a
`headless = []` feature but `cargo check --no-default-features --features
headless` fails to compile. This pass tried to lift the bevy rendering
features (`2d_bevy_render`, `ui_bevy_render`, `scene`, `png`) off the
base `bevy = { features = […] }` declaration onto the `visible` cargo
composite so that only `--features visible` (or its containing
composites) drags the renderer into the dep graph.

That move landed cleanly (commit `8b4cab1`) — desktop tests still pass,
and a headless build no longer compiles the 2D render / UI render /
scene / PNG loader. But it does **not** make headless actually build:

```text
$ cargo check -p ambition_sandbox --no-default-features --features headless
error: The platform you're compiling for is not supported by winit
```

`cargo tree -i winit` traces the dep to `bevy_winit`:

```text
winit v0.30.13
├── accesskit_winit v0.29.2
│   └── bevy_winit v0.18.1
│       └── bevy_internal v0.18.1
│           └── bevy v0.18.1
```

The natural next move is to drop `default_app` from the base bevy
features and inline the narrower no-window pieces (`async_executor`,
`bevy_asset`, `bevy_log`, `bevy_state`, `bevy_color`,
`reflect_auto_register`, `std`). That part also lands cleanly.

But the sandbox additionally needs `ui_api` for `Node` / `Style` /
`BackgroundColor` / `TextColor` — the non-render UI primitive types are
referenced by *non-presentation* modules: cutscene scaffolding, the fx
HUD primitives, the runtime setup text widgets, the schedule helpers'
query parameters. Pulling `ui_api` out of the base set surfaces 848
compile errors across `presentation/cutscene.rs`, `presentation/fx.rs`,
`presentation/rendering/health.rs`, `presentation/rendering/primitives.rs`,
`runtime/setup.rs`, and more.

So we have to keep `ui_api`. And `ui_api`, per Bevy 0.18's
`Cargo.toml`:

```toml
ui_api = [
    "default_app",
    "common_api",
    "bevy_ui",
]
default_app = [
    "async_executor",
    "bevy_asset",
    "bevy_input_focus",
    "bevy_log",
    "bevy_state",
    "bevy_window",
    "custom_cursor",
    "reflect_auto_register",
]
```

`ui_api` re-enables `default_app`, which re-enables `bevy_window`. And
`bevy_window`'s default features eventually pull in `bevy_winit`
through the accesskit / window-event chain. Removing `default_app`
from the base set is a no-op as long as `ui_api` stays — the feature
graph is unified across the build.

**Takeaway**: a true headless build for the sandbox needs more than
the Cargo.toml `[features]` split that #1 describes. It needs the
non-presentation modules to stop depending on `BackgroundColor` /
`TextColor` / `Node`. That likely means:

1. Define sandbox-local shim types for the few HUD / fx / cutscene
   primitives that the simulation half touches today.
2. cfg-gate the actual `bevy::ui` consumers behind `cfg(feature =
   "visible")` so a headless build resolves to the shim types.
3. Then drop `ui_api` from the base bevy dep and headless drops
   `default_app` cleanly, and winit is finally out of the graph.

Pre-stage that work behind a single PR; otherwise the contributor
ends up doing 848 cfg-gate edits in one go.

**Intermediate value of the partial slice**: moving render features
off the base dep is still useful because it makes "is this build
headless-eligible?" testable per cargo feature. The headless build
fails earlier (on winit, in the dep graph) without dragging the full
renderer through compilation. The size of the compile delta is small
but the architectural intent is clearer: presentation features live
on `visible`, base bevy is the minimum surface every build needs.
