# Bevy 0.18 `ui_api` transitively re-enables `bevy_winit` through the feature graph

**Trap shape**: a Bevy 0.18 sandbox crate has `bevy/ui_api`,
`bevy/default_app`, `bevy/2d_bevy_render`, etc. in its base
`bevy = { features = [...] }` declaration. The crate wants a true
no-windowing headless build via
`--no-default-features --features headless`. The agent moves the
rendering features (`2d_bevy_render`, `ui_bevy_render`, `scene`, `png`)
off the base dep and onto the `visible` cargo composite. Default
desktop tests still pass, but the headless build STILL fails on
`The platform you're compiling for is not supported by winit`.

**Why**: Bevy 0.18's `ui_api` cargo feature transitively enables
`default_app`, which enables `bevy_window`, which (through accesskit
/ winit's window-event chain) enables `bevy_winit`. Even if the agent
also removes `default_app` from the base set, `ui_api` re-enables it.
The feature graph union is what cargo computes — `--no-default-features`
does not give you a "minimal bevy."

The trap is easy to fall into because:
1. Default desktop builds keep passing — `cargo check -p ambition_gameplay_core`
   succeeds and tests run.
2. The user-facing cargo feature `headless = []` looks self-contained.
3. The agent's natural model is "remove `default_app` from the base
   list, headless gets the narrower set" — which is wrong as long as
   any other base feature transitively enables `default_app`.

**Pre-flight check** before claiming "headless feature gate is done":
1. `cargo tree -p <crate> --no-default-features --features headless -i winit`
   must succeed AND show no winit dep, or
2. `cargo check -p <crate> --no-default-features --features headless`
   must succeed.

If winit is still in the tree, the headless build is still pulling
windowing. Find what's enabling it and either:
- Drop that feature from the base set and re-add it under `visible`
  (or a more specific feature like `ui_render`).
- Move the use sites that depend on it behind `cfg(feature = "visible")`
  so the base set can drop the feature entirely.

**Why the second route is large**: in the Ambition sandbox, dropping
`ui_api` from the base set surfaces ~848 compile errors across
non-presentation modules (cutscene scaffolding, fx HUD primitives,
runtime setup text widgets) that reference `BackgroundColor`,
`TextColor`, `Node`, `UiRect`, etc. directly. A true headless build
requires either staging cfg gates incrementally or replacing the bevy
UI primitive types with sandbox-local shim types under
`cfg(not(feature = "visible"))`.

**Bench question for a future agent**: "I want a headless build. I
removed `default_app` from the base bevy dep but winit is still in
`cargo tree`. What's enabling it?"

The expected answer: walk the base bevy features in Bevy 0.18's
`Cargo.toml`. Any of `ui_api`, `2d`, `3d`, `ui`, `common_api`,
`default_app` transitively enables `bevy_window` and from there
`bevy_winit`. The agent should `grep` Bevy's `Cargo.toml` for the
target feature in the LHS of each meta-feature definition rather
than guessing.

**Reference**: see [`bevy-headless-feature-graph-2026-05-20.md`](../journals/bevy-headless-feature-graph-2026-05-20.md)
for the live trace through the Ambition sandbox.
