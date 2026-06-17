# Crate foundation: state machines, asset collections, snapshots, and property tests

This patch brings in four foundational crates without trying to redesign the live sandbox in one step.

## `seldom_state`

`ambition_engine::state_machines` defines shared entity-local state components for enemies, bosses, chests, and breakables, and exposes `AmbitionStateMachinePlugin`, which registers `seldom_state::StateMachinePlugin` in Bevy-facing crates.

The division of responsibility is intentional:

```text
Bevy States:
  app-wide modes such as loading, playing, pause, dialogue, room transition, cutscene

seldom_state:
  per-entity behavior such as enemy idle/patrol/telegraph/attack/recover,
  boss dormant/intro/phase/defeated, chest closed/opening/opened,
  breakable intact/cracking/broken/respawning
```

The existing sandbox `FeatureRuntime` remains in place. Future patches should migrate one entity family at a time, starting with a small enemy such as a `GradientSeeker`.

## Boss pattern schedules

`ambition_engine::boss_patterns` adds small deterministic schedule values for the first-pass `Gradient Sentinel` boss concept. These are not rendering systems; they are reviewable design artifacts that future Bevy systems can interpret into telegraphs, active hitboxes, recovery windows, and effects.

## `insta`

`insta` is now an engine dev-dependency. The first snapshot tests use inline snapshots for:

- the public state-machine vocabulary,
- `Gradient Sentinel` phase 1 schedule,
- `Gradient Sentinel` phase 2 schedule.

Inline snapshots avoid generating separate `.snap` files in the first pass and make review simple in normal code review.

## `proptest`

`proptest` is now an engine dev-dependency. The first property tests check lightweight invariants:

- AABBs generated from min/size contain their expected corners and remain finite,
- line kinematic paths require two points and positive speed,
- boss schedules have finite, positive timings.

Future property tests should cover blink destination validity, room graph links, spawn repair, damage state transitions, and procedural visual geometry.

## `bevy_asset_loader`

`ambition_gameplay_core::loading` adds the first `SandboxAssetCollection` using `bevy_asset_loader`. It currently initializes the sandbox manifest handle with `init_collection`, while the embedded RON fallback remains the authoritative startup path.

This is deliberately conservative. A future patch can promote this into a real `BootState::Loading -> Ready` flow once the sandbox has enough assets to justify moving setup out of `Startup`.
