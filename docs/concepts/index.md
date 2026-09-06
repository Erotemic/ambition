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
| [`one-body-one-path.md`](one-body-one-path.md) | writing anything keyed to "player" or "actor/enemy/boss" — the bifurcation smell test, what is already unified, and what stays separate on purpose |
| [`bevy-native-data-driven-ecs.md`](bevy-native-data-driven-ecs.md) | deciding whether behavior belongs in authored data, ECS state, systems, or a tool |
| [`input-and-game-modes.md`](input-and-game-modes.md) | changing devices, control authority, action slots, prompts, touch, menus, dialogue, or pause |
| [`sim-presentation-seam.md`](sim-presentation-seam.md) | changing messages, read models, rendering, audio, UI, or headless behavior |
| [`ldtk-world-composition.md`](ldtk-world-composition.md) | changing authored space, world records, lowering, loading zones, or room construction |
| [`movement-collision.md`](movement-collision.md) | changing movement, collision, body modes, blink, ledges, hitboxes, or projectiles |
| [`asset-management.md`](asset-management.md) | changing logical asset identity, provider catalogs, loading, or platform packaging |
| [`platform-targets.md`](platform-targets.md) | changing desktop, web, Android/touch, controller, or Steam Deck support |
| [`invariants.md`](invariants.md) | you need the standing list of engine invariants rather than the page that argues one |
| [`api-growth.md`](api-growth.md) | adding a public export, or asking why a clean facade can still force a game to link what it never asked for |

## Engineering practice

| Concept | Read when |
|---|---|
| [`testing-and-validation.md`](testing-and-validation.md) | choosing validation and merge gates |
| [`test-placement.md`](test-placement.md) | deciding where a new test belongs |
| [`rust-module-boundaries.md`](rust-module-boundaries.md) | splitting modules, changing visibility, moving tests, or creating facades |
| [`tools-and-generated-content.md`](tools-and-generated-content.md) | adding or using generators, validators, and generated outputs |
| [`agent-native-authoring.md`](agent-native-authoring.md) | designing or using LLM-facing content discovery, inspection, mutation, validation, provenance, or review workflows |
| [`generated-assets-audio.md`](generated-assets-audio.md) | changing reproducible music, SFX, sprite, or background generation |
| [`engineering-memory.md`](engineering-memory.md) | searching or promoting hard-won lessons from `dev/` |
| [`patch-overlays-and-repo-state.md`](patch-overlays-and-repo-state.md) | preparing overlays or broad file replacements |
| [`llm-spatial-authoring-discipline.md`](llm-spatial-authoring-discipline.md) | placing gates, walls, hitboxes, one-ways, breakables, or encounter geometry |
| [`brainstorms-design-incubation.md`](brainstorms-design-incubation.md) | handling Jon's active brainstorm space correctly |
| [`anti-llmism-style-guide.md`](anti-llmism-style-guide.md) | writing or auditing player-facing text — Yarn dialogue, barks, banter, cutscenes, fallback lines. Its general rhetorical rules MAY be applied to documentation; its hard bans are scoped to dialogue |

## Specialized references

- [`cryptography-crew.md`](cryptography-crew.md) — the crypto-themed NPC cast.
- [`hall-of-characters-is-not-special.md`](hall-of-characters-is-not-special.md) — why the hall is ordinary content, not an engine case.

> ⚠ **The four pages above were all missing from this index until 2026-09-03**,
> and one of them — the style guide — had **zero inbound links anywhere in the
> repository**. A concept page nothing points at is doctrine that cannot be
> followed. ⇒ When adding a page under `docs/concepts/`, adding the row here is
> the second half of the same change; nothing checks it, so nothing will remind
> you.

## Maintenance

When a durable invariant changes, update the concept in the same patch as the
code and add/update an ADR when the decision is architectural. Do not preserve
completed migration steps in a concept page.
