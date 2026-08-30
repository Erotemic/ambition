---
id: agent-native-authoring
status: current
aliases:
  - LLM authoring
  - agent authoring
  - content authoring
related_docs:
  - docs/concepts/llm-spatial-authoring-discipline.md
  - docs/planning/engine/authoring-and-tools.md
  - docs/tools/index.md
last_verified: 2026-08-30
---

# Agent-native authoring

Ambition treats an LLM agent as a first-class content author.

The long-term competitive advantage is not reproducing the Godot or Unity editor
before the game can be built. It is making the engine unusually easy for an agent
to inspect, author, validate, explain, and iterate on through supported semantic
surfaces.

Human-facing editors remain valuable, especially for manual visual refinement.
They should be optional frontends over the same authored semantics rather than a
second authority.

## Competitive implication

When Ambition is compared with Godot/Unity-class engines, authoring should be
judged by **intent-to-validated-change efficiency and expressiveness**, not by
visual-editor parity. If another engine exposes an operation through a scene tree,
inspector or asset browser, Ambition needs a supported way to accomplish the
meaningful engine task; that way may be a schema, query API, semantic command,
provider, Rust plugin, generated review artifact or CLI.

A visual tool should be added when direct manipulation itself is valuable. It
should not be added merely so the feature can be checked off against another
engine's editor.

## The authoring loop

A supported content family should converge on this loop:

```text
 discover vocabulary/schema
        ↓
 inspect existing content semantically
        ↓
 plan or dry-run an intent-level change
        ↓
 mutate the authored source
        ↓
 validate + resolve + prepare
        ↓
 inspect semantic diff / provenance
        ↓
 generate the smallest useful review artifact
        ↓
 publish/install explicitly
```

An agent should not need repository archaeology to discover what can be authored
or raw-format knowledge to make a common change.

## Durable rules

1. **Reading is a supported operation.** Tools should expose enough structured
   inspection that an agent can understand existing content before modifying it.
2. **Discoverable vocabulary.** Schemas, registries, `list`/`describe` commands,
   and machine-readable help should answer what nouns, fields, actions, targets,
   and relationships exist.
3. **Simple source stays simple.** LLM-friendly declarative YAML/RON/Rust/Python
   authoring may be edited directly when the format is stable and reviewable.
4. **Complex external formats get semantic operations.** LDtk JSON is the model:
   agents use intent-level query/mutation tools rather than reconstructing editor
   internals by hand.
5. **Preparation is the error boundary.** Resolve references and aggregate useful
   diagnostics before mutating authoritative runtime/session state.
6. **One authored truth.** Generated images/audio/manifests are disposable or
   explicitly published products, not parallel editable authorities.
7. **Provenance is queryable.** A prepared object should be able to explain which
   authored source/provider/field supplied a fact and why a candidate failed.
8. **Cross-domain preflight matters.** A character, room, encounter, or other
   meaningful content unit may span several authoring systems; tooling should
   increasingly report missing/inconsistent dependencies together.
9. **Review artifacts are intentionally small.** Generate the minimum image,
   mastered audio preview, report, room render, or semantic summary that lets the
   maintainer make the next subjective decision.
10. **Human editors are optional frontends.** Native LDtk/editor affordances are
    useful because they produce clearer data and support manual editing later;
    engine architecture must not require a human to operate the editor for normal
    agent-authored changes.

## Submodule visibility

Several major authoring/content systems are separate repositories mounted as git
submodules. They are part of the project architecture even when their checkout is
missing from a source bundle.

| Local path | Canonical repository |
|---|---|
| `tools/ambition_sprite2d_renderer/` | <https://github.com/Erotemic/ambition_sprite2d_renderer> |
| `tools/ambition_music_renderer/` | <https://github.com/Erotemic/ambition_music_renderer> |
| `tools/ambition_sfx_renderer/` | <https://github.com/Erotemic/ambition_sfx_renderer> |
| `dev/ambition_dev_measurements/` | <https://github.com/Erotemic/ambition_dev_measurements> |
| `game/ambition_map_assets/` | <https://github.com/Erotemic/ambition_map_assets> |

An audit performed without those checkouts must say its authoring coverage is
partial. It must not infer that sprite, music, SFX, measurement, or map-authoring
capability is absent.

## What to improve next

The existing tools are already individually capable. Engine 1.0 work should make
them feel like parts of one agent-operable engine by improving:

- common machine-readable discovery and structured diagnostics;
- semantic `describe`/query surfaces before mutation;
- transactional plan/dry-run/apply/diff workflows for fragile formats;
- cross-domain content preflight;
- provenance from authored source through prepared/runtime projection;
- concise, consistent review artifacts;
- provider-owned extension of authoring vocabulary without closed engine switches.

Do not build a GUI merely to make the authoring story look more like another
engine. Add a visual frontend when manual editing itself becomes the product need.
