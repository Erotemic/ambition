# Concept index

Concept pages are durable, agent-readable memory. They define vocabulary, aliases, invariants, edit protocols, implementation anchors, tests, and links to dev-memory evidence.

## Core concepts

| Concept | Read when |
|---|---|
| [`bevy-native-data-driven-ecs.md`](bevy-native-data-driven-ecs.md) | deciding whether a system should be code-driven, RON-driven, LDtk-driven, or ECS-driven |
| [`platform-targets.md`](platform-targets.md) | changing build features, input, assets, packaging, web, Android, mobile, controller, or Steam Deck paths |
| [`tools-and-generated-content.md`](tools-and-generated-content.md) | using or documenting author-time generators, validators, asset renderers, or generated outputs |
| [`movement-collision.md`](movement-collision.md) | touching movement, collision, body modes, slash/pogo, blink, ledges, wall cling, or OOB traces |
| [`ldtk-world-composition.md`](ldtk-world-composition.md) | touching LDtk, active areas, loading zones, editor roundtrip, or world/runtime projection |
| [`llm-spatial-authoring-discipline.md`](llm-spatial-authoring-discipline.md) | placing gates / walls / hitboxes / breakables / one-ways — read BEFORE asking "where exactly?" |
| [`input-and-game-modes.md`](input-and-game-modes.md) | changing controls, Leafwing actions, pause/dialogue/cutscene modes, touch, controller, or mobile input |
| [`asset-management.md`](asset-management.md) | changing asset IDs, platform profiles, web/static/served assets, Android bundles, or Steam Deck paths |
| [`sim-presentation-seam.md`](sim-presentation-seam.md) | changing events/messages, presentation adapters, visual/audio effects, or headless paths |
| [`testing-and-validation.md`](testing-and-validation.md) | choosing validation after a patch or adding regression coverage |
| [`rust-module-boundaries.md`](rust-module-boundaries.md) | splitting Rust modules, moving tests, changing facades, imports, derives, or helper visibility |
| [`generated-assets-audio.md`](generated-assets-audio.md) | changing generated music/SFX/sprite/background pipelines or reproducibility rules |
| [`patch-overlays-and-repo-state.md`](patch-overlays-and-repo-state.md) | preparing overlay packages or replacing broad files |
| [`brainstorms-design-incubation.md`](brainstorms-design-incubation.md) | using or editing `docs/brainstorms/` without demoting it to archive material |
| [`engineering-memory.md`](engineering-memory.md) | searching `dev/` or promoting hard-won lessons into durable docs |

## Maintenance

When a durable invariant changes, update the concept page in the same patch as the code. If the change is architectural, update or add an ADR.
