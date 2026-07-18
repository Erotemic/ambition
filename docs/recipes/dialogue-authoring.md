---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/content-and-provider-boundaries.md
  - docs/systems/actors-brains-and-character-content.md
---

# Dialogue authoring

Dialogue text, node names, speaker identities, voice mappings, and game-specific
commands are provider content. The reusable dialogue domain owns runtime state,
command interfaces, input/navigation, and presentation-neutral events.

## Find the current provider surface

```bash
python scripts/agent_query.py "dialogue Yarn provider command"
python scripts/agent_query.py tests "dialogue lint yarn compile"
```

Ambition's Yarn sources currently live under:

```text
game/ambition_content/assets/dialogue/
```

Use stable node/dialogue IDs that match the provider's character/interactable
references. Do not add a second RON dialogue graph or put named dialogue in a
reusable crate.

## Edit loop

1. Edit the smallest `.yarn` file in the owning provider.
2. Keep command names/arity aligned with the registered bridge.
3. Keep actor-targeting explicit; a command acting on “the current NPC” must use
   the runtime's established dialogue subject/context.
4. Register/update provider voiceprint/SFX mappings when needed.
5. Run static lint and Yarn compile tests before a visible smoke test.

```bash
./run_tests.sh -p ambition_content -k dialogue
./run_tests.sh -p ambition_content -k yarn
./run_tests.sh -p ambition_dialog
```

## Durable authoring rules

- Dialogue progression authority lives in the dialogue runner, not the UI text
  widget.
- Commands emit typed domain requests/facts; they do not reach into renderer
  internals.
- Character IDs, quest flags, item IDs, and room IDs are stable provider IDs.
- UI navigation consumes the shared menu-control frame.
- Closing/interruption/reset leaves no stale actor focus, menu mode, audio cue,
  or pending command.
- A headless test can start, advance, choose, and complete dialogue without a
  window or audio device.
