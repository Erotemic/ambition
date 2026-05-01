# Current state

This document is the current high-level source of truth for Ambition. Update it when the architecture or active design direction changes. Keep transient implementation details in focused patch docs or source comments.

## One-sentence summary

Ambition is a Rust/Bevy 2D metroidvania/platformer sandbox plus reusable mechanics engine, built code-first around excellent movement feel, data-driven rooms, generated assets, and eventually mathematical/story-driven progression.

## Active architecture

```text
ambition_engine
  Bevy-native reusable mechanics and data vocabulary.

ambition_sandbox
  Playable Bevy shell, LDtk-authored sandbox world-composition POC, RON tuning/audio data,
  debug tools, visual/audio adapter, and current experimental feature rooms.

future story/game crates
  Campaign content, progression, dialogue, world variants, and presentation choices.
```

The engine may depend on Bevy and Bevy-adjacent crates when useful. It should still avoid owning sandbox presentation details such as colors, HUD layout, inspector windows, and temporary visual experiments.

## Current stack

- Bevy 0.18
- Leafwing Input Manager for semantic controls
- serde / RON / `bevy_common_assets` for tuning/audio manifests
- LDtk JSON authoring via an Ambition adapter, with `bevy_ecs_ldtk` added as the Bevy LDtk bridge
- `bevy_asset_loader` foundation for future explicit loading states
- `petgraph` for room transition graphs
- `bevy-inspector-egui` and Bevy Gizmos for dev tooling
- `parry2d` for reusable geometry queries
- FunDSP for startup-rendered generated audio
- Bevy/glam math types such as `Vec2` and `Aabb2d`
- Bevy `States` for app-wide modes such as playing, paused, dialogue, transitions, and cutscenes
- `seldom_state` foundation for per-entity state machines
- `insta` and `proptest` as lightweight testing foundations

## Active gameplay state

The sandbox currently has:

- an all-abilities movement testbed,
- input presets through Leafwing,
- pause/game-mode gating,
- generated lo-fi audio and sound effects,
- LDtk-authored active-area composition for the central hub POC,
- a central hub with a literal drop-down basement stitched into one continuous active area,
- test rooms for hazards, enemies, boss patterns, breakables, pickups/chests, and NPC talk hooks,
- debug labels over loading zones,
- feature runtime behavior for current prototype entities,
- input-feel helpers such as jump/coyote/dash/interaction buffering,
- early state-machine/test/asset-loader scaffolding.

These prototype feature rooms are not the final game. They exist to validate reusable mechanics before content is curated.

## Current data location

The canonical sandbox tuning/audio manifest is:

```text
crates/ambition_sandbox/assets/ambition/sandbox.ron
```

The current sandbox level-authoring POC is:

```text
crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

Older root-level asset paths are obsolete unless a patch explicitly says otherwise.

## Stable decisions

- Movement feel matters more than final art or story polish right now.
- The game should remain fun with raw collision/debug geometry.
- Reusable mechanics should migrate into `ambition_engine` or reusable data specs.
- The sandbox should remain an adapter/lab, not a second engine.
- App-wide modes use Bevy `States`; per-entity behavior should move toward `seldom_state` gradually.
- Room/content authoring should be data-driven where practical. LDtk is now the first external level-editor adapter target; Ambition typed data remains canonical.
- Generated assets should remain inspectable, reproducible, and connected to gameplay semantics.
- Patches should include documentation notes, testing limitations, and a markdown paragraph commit message.

## Experimental / not final

The following areas are intentionally provisional:

- Feature basement layout and entity visuals.
- Enemy and boss behavior details.
- NPC dialogue presentation.
- Generated visual style.
- Asset-loading boot flow.
- Exact control presets and HUD layout.
- Roguelike / run-based modes.
- Non-Euclidean and non-metric space mechanics.

Do not document these as final game promises in the README. Use focused docs and ADRs.

## Known high-risk areas

Spatial reasoning and geometry code need extra review. In particular:

- LDtk chunk-to-active-area composition,
- room transition arrival repair,
- loading-zone placement and labels,
- camera/world coordinate conversion,
- collision edge-touch semantics,
- blink destination search,
- moving hazards/platforms,
- non-Euclidean seams or chart transforms.

When touching these systems, add an `AMBITION_REVIEW:` comment if the logic is easy to get subtly wrong, and add tests or debug visualization when practical. See `docs/AGENT_HANDOFF.md`.

## Current next good moves

1. Fix compile/runtime issues from user logs before adding new features.
2. Convert one enemy to the `seldom_state` path instead of migrating everything at once.
3. Build a small first-level vertical slice rather than only adding isolated labs.
4. Expand tests around room graphs, blink/collision, input buffering, and generated schedules.
5. Add a render/preview lab for procedural visuals before committing to a final style.
6. Keep updating ADRs when decisions supersede older notes.
