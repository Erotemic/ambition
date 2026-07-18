---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/boss-encounter-architecture.md
  - docs/systems/actors-brains-and-character-content.md
---

# Boss behavior profiles

A boss is an ordinary actor body plus provider-authored identity, capabilities,
brain/action policy, phase data, and presentation. “Boss” must not create a
second combat or movement engine.

## Ownership

- Provider content owns named boss profiles, phase/seed data, encounter bindings,
  sprites, dialogue, audio, and rewards.
- Character/brain domains own reusable decision/state-machine vocabulary.
- Action schemes and move playback translate decisions into shared actor actions.
- Movement/combat/projectile/world domains execute authoritative effects.
- Presentation maps semantic state/effects to animation, VFX, SFX, and camera.

Current Ambition profile data lives under
`game/ambition_content/assets/data/`. Use `agent_query.py` to locate the current
schema and registration path rather than copying a field list from prose.

## Profile quality

A profile should describe reusable dimensions such as:

- phase conditions and deterministic transitions;
- move/sequence selection and cooldown/resource policy;
- perception/memory requirements;
- capability/action-set changes;
- body/armor/hurtbox policy;
- provider IDs for presentation and rewards.

It should not name Rust functions, mutate ECS directly, or duplicate a move's
hitbox/projectile implementation.

## Invariants

- The same boss profile can run headlessly.
- Move execution uses the same body/combat/action seams as non-boss actors.
- Phase transitions are deterministic, observable, and snapshot-safe.
- Encounter reset reconstructs the initial profile/brain/body state.
- Provider validation rejects unknown IDs and inconsistent phase references
  before activation.

## Validation

```bash
python scripts/agent_query.py "boss profile phase MovePlayback"
python scripts/agent_query.py tests "boss profile validator"
./run_tests.sh -p ambition_content -k boss
./run_tests.sh -p ambition_characters -k boss
./run_tests.sh -k boss
```
