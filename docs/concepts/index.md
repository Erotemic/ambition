# Concept index

Concept pages are durable, agent-readable memory. They define vocabulary, aliases, invariants, edit protocols, implementation anchors, tests, and links to dev-memory evidence.

They are not tutorials and they are not a replacement for code search. Use them to localize the right code and validation path quickly.

## Seed concepts

| Concept | Read when |
|---|---|
| [`movement-collision.md`](movement-collision.md) | touching player/enemy collision, kinematic sweeps, wall cling, ledge snap, body modes, or trace-driven OOB debugging |
| [`ldtk-world-composition.md`](ldtk-world-composition.md) | touching LDtk, active areas, loading zones, editor roundtrip, or world/runtime projection |
| [`rust-module-boundaries.md`](rust-module-boundaries.md) | splitting Rust modules, moving tests, changing facades, imports, derives, or helper visibility |
| [`sim-presentation-seam.md`](sim-presentation-seam.md) | changing events/messages, presentation adapters, visual/audio effects, or sim-to-render boundaries |
| [`generated-assets-audio.md`](generated-assets-audio.md) | changing generated music, SFX, sprite/background generators, asset manifests, or reproducibility rules |
| [`input-and-game-modes.md`](input-and-game-modes.md) | changing Leafwing controls, menu navigation, pause/dialogue/cutscene modes, or touch/controller behavior |
| [`testing-and-validation.md`](testing-and-validation.md) | deciding what to run after a patch or adding regression coverage |
| [`patch-overlays-and-repo-state.md`](patch-overlays-and-repo-state.md) | preparing overlay packages, replacing broad files, or preserving platform entrypoints |
| [`brainstorms-design-incubation.md`](brainstorms-design-incubation.md) | using or editing `docs/brainstorms/` without demoting it to archive material |
| [`engineering-memory.md`](engineering-memory.md) | searching `dev/` or promoting hard-won lessons into durable docs |

## Concept page maintenance

When a durable invariant changes, update the concept page in the same patch as the code. If the change is a durable architectural decision, add or update an ADR too.
