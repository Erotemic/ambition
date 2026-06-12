# Build Personas and Feature Gates

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


The feature system should support known build personas rather than every arbitrary combination.

## Build personas

### `headless_runtime`

Purpose: fast compile/test of runtime logic without presentation/authoring/audio/devtools.

Should include:

```text
platformer core/runtime
movement/collision
rooms/lifecycle
world query
generic projectiles if needed
selected content only if tests require it
```

Should exclude:

```text
rendering
sprites/UI
audio
Yarn/dialogue runtime
LDtk runtime rendering
devtools/inspectors
portal render
portal LDtk
```

### `gameplay_dev`

Purpose: gameplay systems without full visual stack.

May include:

```text
runtime
movement
rooms
combat
held items
portal core
Ambition content
```

May exclude:

```text
heavy presentation
audio
devtools
```

### `desktop_dev`

Purpose: normal local development.

Includes most useful systems:

```text
visible
audio
ldtk
portal
portal_render
portal_ldtk
ambition_content
devtools
```

### `web`

Purpose: web target with selected presentation/audio constraints.

### `android`

Purpose: mobile input/platform integrations.

### `content_validation`

Purpose: load/validate content without running full game presentation.

## Feature sketch

```toml
[features]
default = ["desktop_dev"]

headless = [
    "dep:ambition_platformer_core",
    "dep:ambition_platformer_ecs",
    "dep:ambition_platformer_physics",
    "dep:ambition_platformer_rooms",
]

desktop_dev = [
    "visible",
    "audio",
    "ldtk",
    "devtools",
    "portal",
    "portal_render",
    "portal_ldtk",
    "ambition_content",
]

visible = ["dep:ambition_platformer_render"]
ldtk = ["dep:ambition_platformer_ldtk"]
portal = ["dep:ambition_mechanics_portal"]
portal_render = ["portal", "visible", "dep:ambition_portal_render"]
portal_ldtk = ["portal", "ldtk", "dep:ambition_portal_ldtk"]
audio = ["dep:ambition_audio"]
dialogue = ["dep:ambition_dialogue"]
devtools = ["dep:ambition_devtools"]
ambition_content = ["dep:ambition_content"]
```

## CI/check commands

Start with a small supported matrix:

```bash
cargo check -p ambition_sandbox --no-default-features --features headless
cargo check -p ambition_sandbox --features desktop_dev
cargo check -p ambition_sandbox --no-default-features --features "headless portal"
cargo check -p ambition_sandbox --no-default-features --features "ldtk portal portal_ldtk"
```

As plugin boundaries stabilize, add:

```bash
cargo test -p ambition_mechanics_portal
cargo test -p ambition_platformer_ecs
cargo test -p ambition_content --no-default-features
```

## Important rule

Feature gates should follow ownership boundaries:

```text
portal       gates portal gameplay
portal_render gates portal visuals
portal_ldtk   gates portal authoring conversion
visible       gates generic presentation
ldtk          gates generic LDtk adapter
devtools      gates debug/profiling/inspector tools
```

Do not put generic raycast, body transit, room lifecycle, or gravity primitives behind `portal`.
