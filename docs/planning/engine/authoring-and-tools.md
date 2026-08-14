# Agent-native authoring and tools — Engine 1.0 program

**State:** OPEN. Ambition is the primary customer.

## Goal

Make Ambition unusually easy for LLM agents to author correctly.

The near-term competitive target is **not** to reproduce the Godot/Unity visual
editor before the flagship game can be built. The engine should expose enough
semantic structure that an agent can discover available content vocabulary,
inspect what already exists, make intent-level changes, validate them before
runtime, explain what preparation produced, and generate concise artifacts for a
human to review.

Human visual editors remain useful optional frontends, especially when manual
editing becomes important. They should consume the same authored semantics rather
than becoming a second authority.

Durable doctrine:
[`../../concepts/agent-native-authoring.md`](../../concepts/agent-native-authoring.md).

## Existing advantage

This is an extension program, not a greenfield tooling project. The repository
already has substantial agent-operable authoring surfaces:

- `ambition_ldtk_tools` provides semantic world inspection, validation,
  transactional edits, spatial queries, semantic diffs, room renders and debug
  bundles;
- the sprite-renderer submodule discovers procedural Python/YAML targets and
  publishes deterministic sheets, metadata, portraits and review products;
- the music-renderer submodule treats MusicIR YAML as source of truth and emits
  reproducible audio plus structured diagnostics;
- the SFX-renderer submodule treats SFXIR YAML as source of truth and emits
  deterministic render manifests and audio;
- provider/content preparation already gives the runtime a validated semantic
  boundary instead of making authoring formats authoritative at runtime.

The root README and `docs/tools/index.md` contain canonical GitHub links for
submodules so an agent working from a source export does not mistake an absent
checkout for an absent capability.

## One preparation model, many authoring frontends

```text
Python / YAML / RON / Rust values / LDtk / Yarn / SVG / generators
                              |
                        inspect + discover
                              |
                     plan / semantic mutation
                              |
                    validate + resolve + prepare
                              |
                  immutable prepared game content
                              |
                  +-----------+-----------+
                  |                       |
             headless sim             visible host
```

The frontend may vary by content family. Runtime authority should not.

"Declarative" means authoring produces inert/composable values that are
validated before installation. Pure Rust `CharacterDefinition` construction can
be declarative; a plugin that mutates runtime state while pretending to be a
content document is not.

## Program requirements

### A1 — capability discovery

An agent should be able to discover supported vocabulary without grepping the
implementation. Existing registries/schemas should increasingly expose
machine-readable `list`, `describe`, schema, or equivalent surfaces for:

- characters, actions and authored capabilities;
- LDtk/world entity vocabulary and fields;
- sprite targets/families and published products;
- MusicIR constructs/cues;
- SFXIR layer/recipe capabilities;
- provider-owned authored extensions.

Do not require every tool to use one executable or one serialization format. The
contract is semantic predictability, not CLI uniformity for its own sake.

### A2 — semantic inspection before mutation

Reading the game is part of authoring. Prefer structured commands that answer
questions such as:

- what is in this room and how is it connected;
- what body/capabilities does this character prepare;
- what source owns this sprite/audio/content binding;
- what references will this authored object resolve;
- what review products can this target produce.

LDtk's `room describe`, spatial queries and semantic render/bundle commands are
the model to generalize where useful.

### A3 — intent-level mutation for fragile formats

Simple LLM-friendly sources may be edited directly. Complex external formats
should receive semantic operations instead of raw-format surgery.

For LDtk, commands should express intent such as "link this platform to this
path", "add reciprocal loading zones", or "place this encounter" and preserve
editor-compatible formatting/identity.

### A4 — unified preparation diagnostics and provenance

World, character, action, asset and narrative preparation should converge on
structured diagnostics that tools/external games can consume. A rejected object
should explain authored location, provider/schema, field/reference, expected
semantic target, and failure reason.

Prepared/runtime facts should increasingly be traceable back to authored source.

### A5 — cross-domain content preflight

Meaningful content units span tools. Add preflights that can report the missing
pieces of a whole authored unit rather than forcing an agent to discover them one
runtime failure at a time.

Examples:

- character body/kit + sprite/portrait + writing + referenced SFX/actions;
- room geometry + referenced characters/encounters/portals/paths/audio/dialogue;
- encounter composition + spawnable prepared characters + world gates/rewards.

### A6 — concise review artifacts

Each authoring surface should produce the smallest artifact that lets the
maintainer make the next subjective decision:

- canonical character sheet/portrait/hitbox strip;
- mastered soundtrack preview plus useful reports;
- SFX render + manifest/audit;
- semantic LDtk room summary/render/debug bundle;
- compact preparation/diff/provenance report.

Do not default to maximal diagnostic or preview bundles.

### A7 — agent-authored Ambition acceptance slice

Use a real Ambition task as the integration test: an agent should be able to
inspect an existing region, add or revise a room feature (moving platform is the
first world vertical slice), connect relevant content, validate it, generate a
review artifact, and explain the change without asking the maintainer for raw
coordinates, JSON structure, registry trivia, or migration history.

Secondary games then test that the same authoring seams are provider-extensible.

### A8 — optional human visual frontends

Invest in graphical/manual editing when it solves a real maintainer/content-team
need. LDtk, sprite rig editors, or future visual tools should sit over the same
semantic sources, preparation rules and provenance used by agents.

Do not create a GUI merely to imitate another engine's product surface.

## Immediate world-authoring slice

When this program is selected by the live queue, moving-platform/LDtk authoring
is the first spatial vertical slice because it exercises typed references,
paths, preparation diagnostics, dynamic world semantics, rollback and review
visualization together.

Focused plans:

- [`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md)
- [`kinematic-world-objects.md`](kinematic-world-objects.md)

## Acceptance

A strong Engine 1.0 authoring surface lets an agent take a natural-language
Ambition content request and, without repository archaeology:

1. discover the relevant authored vocabulary;
2. inspect existing semantic context;
3. choose a supported source/operation;
4. plan or dry-run the change when mutation is fragile;
5. apply it transactionally;
6. validate/prepare all affected references;
7. explain provenance and semantic diff;
8. produce a concise human-review artifact;
9. explicitly publish/install generated products where required.

Manual editor operation is not a prerequisite for this acceptance test.
