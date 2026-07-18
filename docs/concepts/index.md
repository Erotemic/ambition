# Concept index

Concept pages hold durable vocabulary, invariants, and edit protocols. They
should survive crate moves. Exact current symbols and files belong in source,
`MODULES.md`, and the generated `.agent/` indexes.

## Read first

| Concept | Read when |
|---|---|
| [`engine-mental-model.md`](engine-mental-model.md) | starting fresh, deciding which architectural layer owns a change, or reviewing a possible duplicate path |
| [`content-and-provider-boundaries.md`](content-and-provider-boundaries.md) | adding named content, catalogs, providers, session activation, or world-lowering seams |
| [`architecture-review-questions.md`](architecture-review-questions.md) | critically reviewing ownership, lifecycle, identity, transactionality, or public seams |
| [`autonomous-decision-making.md`](autonomous-decision-making.md) | making an architecture/design decision without blocking on a question |

## Engine contracts

| Concept | Read when |
|---|---|
| [`bevy-native-data-driven-ecs.md`](bevy-native-data-driven-ecs.md) | deciding whether behavior belongs in authored data, ECS state, systems, or a tool |
| [`input-and-game-modes.md`](input-and-game-modes.md) | changing devices, control authority, action slots, prompts, touch, menus, dialogue, or pause |
| [`sim-presentation-seam.md`](sim-presentation-seam.md) | changing messages, read models, rendering, audio, UI, or headless behavior |
| [`ldtk-world-composition.md`](ldtk-world-composition.md) | changing authored space, world records, lowering, loading zones, or room construction |
| [`movement-collision.md`](movement-collision.md) | changing movement, collision, body modes, blink, ledges, hitboxes, or projectiles |
| [`asset-management.md`](asset-management.md) | changing logical asset identity, provider catalogs, loading, or platform packaging |
| [`platform-targets.md`](platform-targets.md) | changing desktop, web, Android/touch, controller, or Steam Deck support |

## Engineering practice

| Concept | Read when |
|---|---|
| [`testing-and-validation.md`](testing-and-validation.md) | choosing validation and merge gates |
| [`test-placement.md`](test-placement.md) | deciding where a new test belongs |
| [`rust-module-boundaries.md`](rust-module-boundaries.md) | splitting modules, changing visibility, moving tests, or creating facades |
| [`tools-and-generated-content.md`](tools-and-generated-content.md) | adding or using generators, validators, and generated outputs |
| [`generated-assets-audio.md`](generated-assets-audio.md) | changing reproducible music, SFX, sprite, or background generation |
| [`engineering-memory.md`](engineering-memory.md) | searching or promoting hard-won lessons from `dev/` |
| [`patch-overlays-and-repo-state.md`](patch-overlays-and-repo-state.md) | preparing overlays or broad file replacements |
| [`llm-spatial-authoring-discipline.md`](llm-spatial-authoring-discipline.md) | placing gates, walls, hitboxes, one-ways, breakables, or encounter geometry |
| [`brainstorms-design-incubation.md`](brainstorms-design-incubation.md) | handling Jon's active brainstorm space correctly |

## Specialized references

- [`cryptography-crew.md`](cryptography-crew.md) — the crypto-themed NPC cast.

## Maintenance

When a durable invariant changes, update the concept in the same patch as the
code and add/update an ADR when the decision is architectural. Do not preserve
completed migration steps in a concept page.
