---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/boss-behavior-profiles.md
  - docs/systems/persistence-settings-and-progression.md
---

# Boss encounter architecture

A boss encounter composes generic encounter lifecycle with one or more actor
profiles and provider-owned spatial/reward/presentation content.

## Flow

```text
provider encounter spec + LDtk placement
    -> provider preparation/validation
    -> room/session construction transaction
    -> encounter state machine owns activation/waves/completion
    -> actor profiles/brains request shared actions
    -> combat/world domains emit authoritative facts
    -> encounter consumes facts and commits completion/reward
    -> read models drive UI/audio/VFX/camera
```

## Authority boundaries

- Encounter state owns activation, participants, gates, phase orchestration, and
  one-shot completion.
- Actor health/body/brain owns each participant's live state.
- Combat owns accepted hit/damage facts.
- Progression/persistence records durable completion/reward facts.
- Provider content owns names, room placement, roster, rewards, music, dialogue,
  and visual treatment.
- Presentation never decides whether the encounter is complete.

## Lifecycle

All encounter-spawned actors, walls, hazards, prompts, and temporary state must
belong to an explicit session/room/encounter scope. Room replacement, reset, and
snapshot restore use canonical construction and cleanup rather than bespoke
lists.

Completion and rewards are idempotent. Re-entering a completed encounter should
follow provider policy without duplicating rewards or resurrecting stale gates.

## Validation

Test the full state machine headlessly:

- inactive -> armed -> active -> phase/wave transitions -> completed;
- defeat/reset/re-entry;
- participant despawn and room replacement;
- one-shot reward and persisted completion;
- provider validation for unknown characters, rooms, moves, rewards, or audio IDs.

```bash
python scripts/agent_query.py "boss encounter lifecycle reward"
python scripts/agent_query.py tests "encounter completion reset"
./run_tests.sh -p ambition_encounter
./run_tests.sh -p ambition_content -k encounter
./run_tests.sh -k boss
```
