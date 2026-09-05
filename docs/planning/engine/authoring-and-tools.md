# Agent-native authoring and tools — Engine 1.0 program

**State:** OPEN. Ambition is the primary customer.

> **Guard pointer, added 0ac499bb1 (2026-09-02).**
> `scripts/check_zone_name_ratchet.py` ratchets player-visible loading-zone names
> that still look like authoring ids — a zone `name` is presentation text, so an
> underscore-shaped identifier wants authored prose rather than mechanical
> prettifying or hiding. Counts are tracked per world file, and symlinked worlds
> are deduplicated by real path so one world's improvement cannot mask another's
> regression. Green at `0ac499bb1`: **151 zones carry a name, 0% still look like
> authoring ids.**

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

## Competitive criterion

Godot/Unity demonstrate the value of discoverability and short feedback loops.
Ambition should achieve those outcomes through a different primary interface. The
comparison is not whether an operation has a visual editor panel; it is whether a
capable agent can perform the same meaningful engine task reliably and faster
through supported semantic tools.

For Engine 1.0, an authoring capability is competitive when an agent can:

- discover what can be expressed without implementation grep;
- inspect current semantic state before changing it;
- make the change through stable authored data or an intent-level operation;
- receive source-qualified validation/provenance failures;
- run the smallest representative simulation/render/test needed to check it;
- produce a concise human-review artifact when subjective visual/audio judgment
  remains;
- continue through build/package without manual editor operation.

A GUI is useful when the task is intrinsically visual or manual. It is not the
definition of engine capability.

See the cross-program bar:
[`godot-class-2d-capability.md`](godot-class-2d-capability.md).

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

## ⛔⛔ An unknown technique key PASSES validation, because absence means two things

**Measured 2026-09-05, and it is the same defect shape as `boss.cleared` one
surface over.** `ParamSchemaRegistry::validate`
(`crates/ambition_entity_catalog/src/lib.rs:151`) says so in its own doc:

> The engine matches no key, so an unregistered key always passes (a paramless
> content-const technique needs no schema).

⇒ **The reason is legitimate and the consequence is not.** A technique with no
params genuinely needs no check, so the registry cannot distinguish *"this key
has nothing to validate"* from *"this key does not exist"*. Both are an absence,
and absence passes. An authored `smash.teleprot` — a typo for a real technique —
validates clean at startup and does nothing in play.

⭐ **That is exactly the `boss.cleared` failure, and it cost weeks there:** a
missing save key read `Untouched`, so a wrong id was a silently shut door rather
than an error. Here a missing schema reads "fine", so a wrong key is a silently
inert effect. **An absence that reads as a pass is the shape to hunt.**

⇒ ⭐ **THE FIX IS TO SEPARATE THE TWO FACTS, not to make unknown keys fail.**
A technique should register its EXISTENCE always, and a param check optionally.
Then all three answers become distinct and correct:

| authored key | today | with the split |
|---|---|---|
| unknown (`smash.teleprot`) | ✔ passes | ⛔ *"no technique is installed under this key"* |
| known, paramless | ✔ passes | ✔ passes |
| known, with a schema | validated | validated |

⚠ **The cost is one registration per technique, and the design question that
comes with it is who owns the installed set** — the same crate that owns the
checks, or the content install that already calls `register`. That is the
question to answer before writing any of it, because an installed-set registry
maintained separately from the code that installs techniques is a second
authority over "does this exist", which is the thing this repo keeps removing.

ⓘ This is the substrate under the discovery surface Jon asked for —
`smash_tool techniques` / `technique <key>` / `mechanics <domain>` — and it is
the half worth building first: a catalog cannot list what the engine cannot be
asked about. `ConditionCatalog::describe` is the working precedent for the shape.

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

### A9 — project-wide semantic dependency and reference graph

⚠ **required, despite following the optional item** — the letters are labels, not
a priority order.

An agent should be able to ask, without source archaeology:

```text
what references character:fia?
what uses world_fact:bridge_powered?
which rooms instantiate composition X?
which rules invoke command Y?
which authored objects reference this portal?
what will break if I rename this mechanism?
what will break if I delete this definition?
```

⭐ **this builds on what already exists** — prepared references, the LDtk
relationship tooling, content schemas, provenance, and eventually authored rule
references. ⛔ do not create a separate disconnected graph beside them; a second
index that can disagree with the first is worse than no index.

Measurable milestones:

1. **cross-domain reference enumeration** — every authored reference a domain can
   express is enumerable through one query surface;
2. **reverse-reference queries** — "what points at this?" answered for each
   supported reference kind;
3. **structured unresolved-reference diagnostics** — an unresolved reference
   reports what was named, where, and why resolution failed;
4. **semantic rename planning / dry-run** — a rename reports every affected site
   before mutating anything, and applies transactionally across relevant
   authoring backends;
5. **rule dependency inspection**, once authored rules exist — see M6 of
   [`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md).

⛔ this is an extension of existing authoring/inspection work, **not** another
independent major campaign.

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
9. explicitly publish/install generated products where required;
10. run the relevant public build/test/package path without switching to a
    separate human-editor workflow.

Manual editor operation is not a prerequisite for this acceptance test. The
authoring loop should be benchmarked by successful intent-to-validated-change
work, not by editor-feature parity.
