---
id: content-and-provider-boundaries
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
related_docs:
  - docs/planning/engine/architecture.md
  - docs/planning/engine/immutable-content-and-transactional-construction.md
  - docs/concepts/engine-mental-model.md
  - docs/concepts/ldtk-world-composition.md
---

# Content and provider boundaries

A provider is a complete playable experience assembled from reusable engine
capabilities and provider-owned content. Ambition and each demo are providers;
none is allowed to become an implicit engine default.

## Providers own names

Providers own named:

- worlds and entry rooms;
- characters, enemy rosters, items, bosses, and encounters;
- dialogue, barks, quests, cutscenes, and progression rewards;
- art, sprite registrations, music, SFX cues, and product presentation;
- game rules and host-facing experience identity.

Reusable crates own schemas, registries, state machines, validation, lowering,
and execution. An empty engine composition must be valid even when no provider
contributes a particular content family.

## Registration is App-local and explicit

Content enters through typed App-local fragments, catalogs, plugins, or
provider preparation values. Explicit provider registration is intentional.
Opaque global discovery and process-global install order are not goals.

A good registration seam has these properties:

- deterministic assembly order;
- provider-qualified diagnostics and IDs;
- duplicate/conflict validation before activation;
- no mutation of another App in the process;
- no game crate dependency from a reusable engine crate.

## Prepared content is not live authority

Preparation may parse, validate, resolve, and freeze immutable content. It must
not partially mutate the live session. Activation consumes prepared evidence and
commits through canonical lifecycle/construction seams.

The important distinction is:

```text
provider fragments -> prepared immutable value -> exact activation transaction
                                      !=
                              live gameplay state
```

Live mutable authority belongs to session-scoped entities/resources. Derived
indexes and caches must be rebuildable and mechanically invalidated.

## Session identity is exact

A session identifies an activated provider/content epoch, not merely “the game
is running.” Provider switch, relaunch, reset, room replacement, and restore
must not inherit stale entities, handles, registries, or caches from a prior
session.

Acceptance requires both:

1. leak-free sequential activation of different sessions/providers; and
2. exact reconstruction through reset/restore using the canonical paths.

Bevy `Entity` values are never durable content identity. Persist stable,
provider-qualified IDs and resolve live entities at the boundary that needs them.

## World authoring and lowering

Authoring backends write typed records. `ambition_ldtk_map` is today's LDtk
adapter; `ambition_world` owns reusable world vocabulary and lowering seams.
Simulation/content interpreters register how authored records become canonical
session-scoped ECS state.

Do not make the world vocabulary depend on named character, item, or encounter
types. Use closed common placement records plus narrow registered lowering or
content-staging seams.

## The extension test

Before placing a feature in an engine crate, imagine a second game that:

- uses different worlds and rosters;
- has no Ambition-specific IDs;
- supplies a different presentation;
- runs headlessly;
- activates after another provider in the same process.

If the feature still makes sense and can be configured through public seams, it
is probably reusable. If it names Ambition or assumes its catalogs, it belongs
in the provider.

## Current localization

The repository is still decomposing. Use generated navigation rather than this
page for exact paths:

```bash
python scripts/agent_query.py "provider preparation activation content catalog"
python scripts/agent_query.py crate ambition_platformer_provider
python scripts/agent_query.py crate ambition_content
python scripts/agent_query.py crate ambition_world
python scripts/agent_query.py tests "provider switch session teardown restore"
```
