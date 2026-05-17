# Engine architecture

`ambition_engine` is the reusable Bevy-native mechanics crate. It owns gameplay semantics that should be testable without the full visible sandbox.

## Owns

- movement and collision semantics;
- body modes, body shapes, abilities, resource meters;
- combat intents, hitboxes, damage/faction vocabulary;
- projectiles and motion-input recognition;
- actors/interactions/breakables/encounter vocabulary;
- geometry queries and reusable test fixtures;
- state-machine vocabulary where it helps multiple runtime users.

## Does not own

- sprite/HUD/presentation choices;
- app/window/audio plugin setup;
- platform packaging scripts;
- LDtk editor metadata;
- temporary sandbox room visuals;
- story/campaign policy.

## Boundary rule

Use Bevy-native types when helpful. The engine is not backend-neutral, but it should still avoid sandbox-only presentation policy.

See ADR 0002 and `docs/concepts/bevy-native-data-driven-ecs.md`.
