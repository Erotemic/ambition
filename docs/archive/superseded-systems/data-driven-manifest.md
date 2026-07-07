# Archived: data-driven-manifest.md

Superseded by current LDtk/data-driven ECS docs and concept pages.

Original path: `docs/systems/data-driven-manifest.md`

---

# Data-driven sandbox manifest

The sandbox now has one top-level RON manifest at `crates/ambition_actors/assets/ambition/sandbox.ron`. It is loaded in two ways:

1. synchronously through `SandboxDataSpec::load_embedded()` so the current one-state prototype can boot immediately; and
2. as a Bevy asset through `bevy_common_assets::ron::RonAssetPlugin`, which gives us the asset-loading path we will use once the sandbox grows a loading screen or hot-reload workflow.

The manifest currently owns four categories of data:

- `abilities`: the active `AbilitySet` used when creating the sandbox player;
- `tuning`: `MovementTuning`, including gravity, dash, blink, flight, coyote time, pogo speed, and related values;
- `audio`: generated SFX envelopes and the lo-fi music pattern/chords/lead notes;
- `rooms`: room specs, loading zones, and graph links.

## Room graph

Room transitions are no longer authored as target spawn coordinates. Each room defines named loading zones, and the manifest's `links` list connects `(room, zone)` to `(room, zone)`. `RoomSet` converts this into a directed `petgraph::Graph` at startup. When the player overlaps a source zone, the graph resolves the destination room and target zone. The actual arrival point is derived from the destination zone geometry:

- edge exits place the player just inside the corresponding side of the destination room;
- doors place the player near the bottom-center of the destination door volume;
- `validated_spawn` still repairs the computed arrival if the authored zone geometry would embed the player in a wall or floor.

This makes room links much less brittle. Moving a door or edge exit updates both visuals and spawn placement without editing a second coordinate by hand.

## Why one file for now?

`bevy_common_assets` registers loaders by extension, so loading many independent `.ron` files into different asset types would need distinct extensions or a small custom loader. A single `SandboxDataSpec` avoids extension conflicts and keeps the first data migration simple. Later, we can split it into `*.room.ron`, `*.audio.ron`, and `*.tuning.ron` if we want independent reloads.
