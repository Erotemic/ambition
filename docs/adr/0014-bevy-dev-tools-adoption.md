# ADR 0014: Adopt bevy_dev_tools for dev-time overlays and CI utilities

## Status

Accepted.

## Context

Ambition's debug tooling is currently a hand-rolled HUD (`update_hud`
in `main.rs`) plus `bevy-inspector-egui` panels gated by F3/F4. It's
serviceable but ad-hoc. As the project moves toward a vertical slice
and the events refactor (ADR 0012), several developer-experience
capabilities become valuable:

- An always-correct FPS / frame-time display that doesn't depend on
  custom HUD plumbing.
- A frame-time graph for catching frame spikes (moving platforms,
  particle bursts, LDtk hot reload).
- States debugging (the `GameMode` state machine is currently
  diagnosed only through the inspector).
- CI testing utilities — taking screenshots, running for N frames
  then exiting, asserting on output. This is exactly the shape Phase
  2 of the headless work and Phase 3 RL adapter need.

`bevy_dev_tools` is a first-party Bevy ecosystem crate (lives in the
Bevy repo as `bevy::dev_tools`, accessible via the `bevy_dev_tools`
feature). It provides FPS overlay, frame-time graph, picking debug,
states debug, CI testing utilities, an `EasyCameraMovementPlugin`,
and `EasyScreenshotPlugin`.

The "use existing packages" feedback memory and ADR 0013's
compile-time discipline both apply. `bevy_dev_tools` is a first-party
Bevy crate, so its compile-time cost is bounded and aligned with the
rest of the Bevy graph; adopting it is consistent with both
principles.

## Decision

Adopt `bevy_dev_tools` for dev-time overlays and CI utilities. Gate
behind a feature flag so distribution builds don't pay the cost.

### Feature integration

Add to the sandbox crate's `Cargo.toml`:

```toml
[features]
dev = [
    "bevy/dynamic_linking",
    "bevy/file_watcher",
    "bevy/bevy_dev_tools",
]
ci = [
    "bevy/bevy_dev_tools",
]
```

`dev` (per ADR 0013) bundles dynamic linking + file watcher + dev
tools. `ci` enables only `bevy_dev_tools` so CI smoke runs can use
its testing utilities without the dev-only dynamic-linking baggage.

### Adopted plugins

Initial integration adopts these dev_tools features:

- **FPS overlay** — replaces the FPS portion of the custom HUD.
  Visible when `dev` feature is enabled, toggled via existing F1
  debug-HUD key or a new dedicated key.
- **Frame-time graph** — visible in the inspector layer (F3/F4) when
  `dev` is enabled. Helps diagnose frame spikes during hot reload,
  large room transitions, particle storms.
- **States debug** — expose `GameMode` transitions in a debug overlay
  so transitions like `Playing → Dialogue → Playing` are visually
  obvious without inspector drilling.
- **CI testing utilities** — used by the headless binary
  (`bin/headless.rs`) and forthcoming smoke-test suite. Specifically
  the screenshot-after-N-frames-then-exit pattern is useful for
  visual regression testing once a renderer is available in CI.

`EasyCameraMovementPlugin` and `EasyScreenshotPlugin` are deferred —
the existing camera follow + manual screenshot story is fine for now.

### Replacement vs augmentation

`bevy_dev_tools` augments the existing custom HUD; it does not replace
it wholesale. The custom HUD's gameplay-state lines (combo, zone
hints, LDtk runtime-spine stats) remain because they're sandbox-
specific and not in dev_tools. Migrate items to dev_tools where
dev_tools has a first-class equivalent (FPS, frame-time graph);
keep custom code where it carries gameplay-specific information.

## Consequences

- Sandbox crate gains a `dev` feature flag. Run with
  `cargo run --features dev` for the full developer experience.
- A small amount of HUD logic (FPS line) can be removed from
  `update_hud` once dev_tools is wired in.
- Compile-time impact is bounded: dev_tools is part of the Bevy
  workspace and compiles alongside Bevy itself. With the `dev`
  feature off (default), dev_tools is not in the dependency graph
  — distribution builds pay nothing.
- CI smoke tests gain a structured "run-N-frames-and-screenshot"
  capability for future visual regression work.
- States debug overlay surfaces `GameMode` transitions, which
  clarifies dialogue/pause/transition-mode bugs that currently
  require inspector drilling.
- Moves the project further along the "use existing packages" axis
  rather than continuing to grow custom dev tooling.

## Initial implementation target

Conservative, sequenced:

1. Add the `dev` feature to sandbox `Cargo.toml` per ADR 0013.
   Include `bevy/bevy_dev_tools` in it.
2. Conditionally register `FpsOverlayPlugin` and `FrameTimeGraphPlugin`
   when the `dev` feature is enabled. Wrap with `#[cfg(feature = "dev")]`.
3. Conditionally register the states-debug plugin for `GameMode`.
4. Add a sentence to the contributor README directing dev runs to
   `--features dev`.
5. Defer CI utility integration until Phase 2 events refactor lands;
   the `bin/headless` binary will use it then for smoke-test screenshots.

## Non-goals for the first implementation

- Removing the existing custom HUD wholesale. It carries gameplay-
  specific information dev_tools doesn't replace.
- Adopting `EasyCameraMovementPlugin` — the existing `camera_follow`
  works for the gameplay camera, and free-fly debug movement isn't
  a current pain point.
- Adopting `EasyScreenshotPlugin` for player-facing screenshots.
  In-game screenshots are not a current product feature; this is
  about CI/dev-time testing.
- Picking debug. The sandbox doesn't currently use Bevy picking;
  revisit if/when picking becomes part of the gameplay or editor
  experience.
- Integrating `bevy_dev_tools` into release/distribution builds.
  Feature-gated; never in ship builds.

## Review notes

- After adoption, run `cargo build --timings` before and after
  enabling `dev` to confirm the compile-time impact is bounded.
  Record in `docs/compile_time_audits/`.
- Cross-references: ADR 0013 (the `dev` feature is defined there;
  this ADR adds dev_tools to it). The "use-existing-packages"
  feedback memory documents the principle this ADR applies.
- Future-coupled with ADR 0012 Phase 2: the events refactor + CI
  testing utilities together produce a real automated-test surface
  ("tick app, send scripted events, screenshot, assert") that today
  doesn't exist.

## Sources

- [`bevy::dev_tools` — docs.rs](https://docs.rs/bevy/latest/bevy/dev_tools/index.html)
- [`bevy_dev_tools` crate](https://crates.io/crates/bevy_dev_tools)
- [Bevy 0.18 release notes](https://bevy.org/news/bevy-0-18/) — current ecosystem state
- [Bevy `cargo_features.md`](https://github.com/bevyengine/bevy/blob/main/docs/cargo_features.md) — feature flag definitions for `bevy_dev_tools` and friends
