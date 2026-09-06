---
status: current
last_verified: 2026-08-13
---

# Actors, brains, and character content

The runtime model is **one body, one construction path**. Controlled bodies,
NPCs, enemies, bosses, summons and match fighters are ordinary actor bodies
whose intrinsic kit comes from prepared character content. Controller,
participant, disposition, placement, encounter role and ruleset are contextual
facts rather than alternate body species.

## Current authorities

### Authored character composition

`CharacterDefinition` is the reusable authored composition seam for a character.
It can carry or reference intrinsic body/kit facts such as locomotion, vitals,
abilities/action sets, moves, hurt geometry, contact behavior, death traits and
presentation identity.

Ambition still has provider-owned catalog/source data for broad cast metadata,
presentation/writing/defaults and tooling. That source may participate in
preparation. **The catalog is not a second live body-construction authority.**
Preparation resolves/folds authored inputs into the runtime value.

### Prepared runtime character

`PreparedCharacterDefinition` is the complete immutable character value that
runtime construction consumes. By the time a body is constructed, authoring
fallback and cross-document resolution have already happened; spawn roads do not
choose between an enemy archetype, player archetype and character path.

D73 deleted the separate `ArchetypeSpec` / `CharacterRoster` body ontology and <!-- cite-ok: names the deleted archetype system to say it is not the workflow -->
the build-legacy-body-then-patch seam. Historical migration details are archived
under `docs/archive/planning-superseded/2026-08-13/`.

### Body and control

The body owns intrinsic state/capabilities. A controller/brain supplies intent.
Human input, AI, possession, replay and future remote participants should
converge through the same control/action seams before body execution.

`BrainBinding` preserves reconstructible autonomous control where appropriate.
Changing who controls a body does not change the body's authored identity or
silently grant a different kit.

### Combat and actions

`ambition_combat` owns shared combat/moveset vocabulary and move playback.
Character/action preparation derives the body-valid action repertoire consumed
by human and AI controllers. Do not add player-only attack state or boss-only
combat execution for a rule that belongs to an ordinary body.

## Authoring ownership

Provider/game content owns named characters, writing, art references, tuning and
composition choices. Reusable engine crates own schemas, preparation,
validation, simulation and generic capability behavior.

Authoring may come from RON, Rust values, generated metadata or another
validated provider source. "Declarative" means the authored value is inert and
composable before installation; it does not mean every character must live in
one giant data file.

## Construction shape

```text
provider-authored character inputs
  CharacterDefinition
  catalog/source fragments where used
  referenced moves/art/writing/etc.
             |
             v
        preparation
  resolve + validate + flatten
             |
             v
PreparedCharacterDefinition
             |
             +--> ordinary body construction
             +--> headless simulation
             +--> hosted/standalone games
```

Placement/session facts enter beside the prepared character, not inside its
identity:

```text
prepared character + placement + disposition + controller/session context
                               |
                               v
                         ordinary actor body
```

## Multiplayer consequence

A participant is not a body species. Local and future remote participants can
control ordinary bodies through the same assignment seam. Camera/view focus is
also independent of body identity; multi-view work is specified in
[`../planning/engine/multiplayer-and-multiview.md`](../planning/engine/multiplayer-and-multiview.md).

## Adding a character

Use [`../recipes/adding-a-character.md`](../recipes/adding-a-character.md).
Extending reusable brain/action vocabulary is separate and described in
[`../recipes/extending-brains-and-action-sets.md`](../recipes/extending-brains-and-action-sets.md).

## Validation principle

Validate authored references and completeness during preparation. Runtime
systems should receive resolved identities/values rather than silently looking
up strings and choosing a fallback body.

Use the focused character/content test suites plus the real provider/headless
construction path for the character being changed; exact commands evolve with
the repository and should be localized with `scripts/agent_query.py`.
