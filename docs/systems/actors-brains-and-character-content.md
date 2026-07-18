---
status: current
last_verified: 2026-07-18
---

# Actors, brains, and character content

Characters are assembled from provider-owned identity/content plus reusable
body, action, brain, perception, combat, and presentation machinery. “Player,”
“enemy,” “boss,” and “NPC” are controller/capability/content distinctions, not
separate body implementations.

## Authorities

- `ambition_characters` owns reusable actor identity/control vocabulary,
  `Brain`, perception/memory, action schemes, equipment-to-parameters, and the
  character-catalog schema/assembly registry.
- `ambition_actors` currently owns the unified live body/simulation integration
  and some remaining adapters. It is not a license for named content to move
  back into machinery.
- `ambition_combat` owns movesets, `MovePlayback`, attack timelines, and shared
  action execution.
- provider crates own catalog fragments, roster membership, default presets,
  named techniques, sprite/audio/dialogue IDs, and boss/encounter content.
- sprite/render/read-model crates own reusable visual registration and
  presentation.

## Character catalog

Ambition's authored rows live in:

```text
game/ambition_content/assets/data/character_catalog.ron
```

The schema/parser/App-local fragment registry live under
`ambition_characters::actor::character_catalog`. Runtime consumers read the
assembled `CharacterCatalog`; they do not parse a second copy or keep a
hard-coded roster in engine core.

A row may reference reusable brain/action presets and provider-owned sprite,
dialogue, bark, or presentation IDs. Those references must be validated as a
cross-content graph before the character is considered complete.

## Brain and control contract

A brain observes through `WorldView`/perception vocabulary and emits actor-local
intent/actions. Human input, AI brains, temporary possession, mounts, and RL
control converge before body/action execution.

`BrainBinding` preserves the reconstructible autonomous source. Temporary
control changes who drives the body without deleting the brain configuration
that resumes later. Commands issued while controlled must update the correct
underlying authority rather than a transient mirror.

Brains may use deterministic simulation budgets. Wall-clock cutoffs cannot be
authoritative decisions when replay/resimulation is expected.

## Action contract

- abilities and movesets are live authorities;
- `ActorActionScheme` is derived from those authorities;
- the shared resolver turns slots into body-valid actions;
- `MovePlayback` is the attack/action timeline authority;
- prompts and touch labels consume the same resolved scheme.

Do not add a player-only attack state, boss-only hitbox emitter, or content
system that steals raw input before the shared resolver.

## Adding a character

Use [`../recipes/adding-a-character.md`](../recipes/adding-a-character.md).
Extending the reusable brain/action vocabulary is a separate operation described
in [`../recipes/extending-brains-and-action-sets.md`](../recipes/extending-brains-and-action-sets.md).

## Validation

```bash
./run_tests.sh -p ambition_characters
./run_tests.sh -p ambition_content
./run_tests.sh -k character_catalog
./run_tests.sh -k brain_binding
./run_tests.sh -k action_scheme
```
