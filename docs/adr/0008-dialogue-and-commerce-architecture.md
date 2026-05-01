# ADR 0008: Dialogue and commerce architecture

## Status

Accepted as a sandbox foundation.

## Context

Ambition needs NPC dialogue that can grow into story, tutorials, merchants,
choice gates, and recurring common game interactions without becoming a pile of
one-off UI code. The sandbox already has room-authored `Interactable` objects and
NPC hooks. The next step is to make those hooks pause gameplay, show a dialogue
box, support choices, and preserve a path toward Yarn-authored content.

## Decision

Use `bevy_yarnspinner` as the durable authored-dialogue direction and register it
in the sandbox app now. Keep the first visible dialogue view custom and lightweight
so it can match Ambition's debug-first presentation, interact cleanly with
`GameMode::Dialogue`, and avoid binding core gameplay to an example UI crate.

Use `bevy_material_ui` as the likely Material Design UI foundation for richer
menus and merchants. The first dialogue box intentionally uses stable Bevy UI
primitives plus Material-inspired styling while the game-specific dialogue and
merchant contracts settle.

Room NPCs should continue to be authored as `Interactable(... kind:
Npc(dialogue_id: Some("...")))`. The sandbox runtime maps the `dialogue_id` to a
conversation node set. This keeps the path clear for later migration from the
small code-side registry to Yarn source files under `assets/dialogue/`.

## Merchant and common-dialog contract

Merchants should be implemented as dialogue-capable interactables, not as a
separate special-case UI path. A shop inventory row is conceptually a dialogue
choice with extra data:

- price and currency/provenance source
- requirements and preview text
- reward effect: heal, ability, route unlock, story flag, custom effect
- persistence and refund policy
- consequence text, including generated-system contamination when relevant

Common dialogue should use the same shell:

- tutorial NPCs: line + continue/choice
- lore NPCs: branching nodes and flags
- merchants: choice rows with transaction effects
- doors/locks: requirement explanation plus optional action
- save/checkpoint terminals: confirm/cancel choices

## Consequences

The sandbox now has a real `Dialogue` game mode and a visible dialogue overlay.
The current registry is intentionally small and code-owned; Yarn source files are
included as the migration target and authoring reference. Future patches should
move node content into Yarn once the dialogue view handles Yarn events directly.
