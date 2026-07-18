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

## Speaker portraits and expressions

The interaction context supplies a stable listener character id automatically.
Ordinary one-character conversations therefore need no portrait command: the
catalog's `default` clip is selected.

Use the reusable presentation commands only when the authored scene needs an
explicit speaker or expression:

```yarn
<<present_speaker "npc_alice">>
<<portrait_clip "speaking">>
Alice: This line uses Alice's animated speaking clip.
<<portrait_clip "focused">>
Alice: This line holds the focused still.
<<portrait_clip "default">>
Alice: This returns to the catalog default.
```

- `present_speaker` takes a stable character-catalog id, never a display name or
  texture path. It is primarily for scripted dialogue and multi-speaker scenes.
  Switching speaker resets the portrait clip so expressions cannot leak between
  characters. Passing an empty id returns to the conversation endpoint.
- `portrait_clip` takes an opaque clip name authored by that character's
  portrait generator. `default` clears the override. Missing clip names fall
  back to the catalog/manifest default instead of failing dialogue.
- Clip timing and image rectangles belong to the generated portrait manifest;
  Yarn content does not know about frames or asset paths.

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
