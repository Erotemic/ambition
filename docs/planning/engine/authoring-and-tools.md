# Authoring and tools — engine product program

**State:** OPEN. Ambition is the primary customer.

## Goal

Treat authoring as a first-class engine product surface rather than a collection
of repository-specific scripts surrounding a good runtime.

A Godot/Unity-class competitor needs a developer to be able to create, inspect,
validate, preview and revise content without learning internal crate topology or
reverse-engineering converter assumptions.

Ambition's real content loop is the primary forcing function. Secondary games
prove that the tools and preparation seams are reusable.

## One preparation model, many authoring frontends

The engine does not need one universal file format. It needs a common semantic
boundary:

```text
LDtk / RON / Rust authored values / sprite metadata / SVG / Yarn / generators
                              |
                              v
                    validate + resolve + prepare
                              |
                              v
                  immutable prepared game content
                              |
                  +-----------+-----------+
                  |                       |
             headless sim             visible host
```

The frontend may vary by content family. Runtime authority should not.

"Declarative" means authoring produces inert/composable values that are
validated before installation. A pure Rust `CharacterDefinition` can be
declarative; an imperative plugin that mutates runtime state while pretending to
be a content document is not.

## Product surfaces

### Spatial world authoring

LDtk is Ambition's preferred spatial editor. Invest in native field types,
`EntityRef` relationships, semantic validation, previews, room diagnostics,
intent-level mutation tools and transactional hot reload.

Focused plan:
[`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md).

### Kinematic/dynamic world authoring

Moving platforms are the first vertical slice where editor intent, typed
references, preparation, dynamic collision, rollback and presentation must all
agree.

Focused plan:
[`kinematic-world-objects.md`](kinematic-world-objects.md).

### Character authoring

The post-D73 path should be easy to understand from provider-facing docs and
tools: authored character inputs resolve into one complete
`PreparedCharacterDefinition`; placement/controller/session facts remain
contextual. Character art/writing/gameplay metadata should be inspectable without
creating parallel identity authorities.

Current docs:
[`../../systems/actors-brains-and-character-content.md`](../../systems/actors-brains-and-character-content.md)
and [`../../recipes/adding-a-character.md`](../../recipes/adding-a-character.md).

### Sprite / SVG / visual asset authoring

Keep the existing code-authored sprite workflow while improving editable
component/SVG options, semantic target metadata, editor icons, quality/residency
variants and concise diagnostic renders. Generated presentation data must remain
derived from the authoring source rather than becoming a second body authority.

### Dialogue and narrative authoring

Yarn and character writing metadata should receive the same preparation-quality
principles: stable speaker/listener identity, useful diagnostics, provider-owned
content, and runtime behavior that consumes resolved facts rather than strings.

### Inspection and provenance

Every prepared object should be explainable:

- where was it authored;
- which provider/schema owns the field;
- what references were resolved;
- what capability consumes it;
- why was a candidate rejected;
- what runtime projection will exist.

This should grow through structured preparation diagnostics, not a permanent
source scanner over authoring files.

## Tooling principles

1. **Intent-level operations beat raw-format surgery.** A tool command should say
   "link this platform to this path" or "add this reciprocal door" rather than
   asking an agent to construct JSON internals.
2. **Native editor affordances where useful.** Prefer LDtk point arrays,
   `EntityRef`, layers/tags and editor icons over opaque strings when the editor
   already models the concept.
3. **Preparation is the error boundary.** Aggregate useful failures before
   mutating the live session.
4. **Semantic previews.** Room renderers, character sheets, collision/hurtbox
   overlays, path previews and content summaries should expose game meaning, not
   merely serialized bytes.
5. **Hot reload uses the same compiler.** A hot-reload candidate is prepared and
   validated like cold load; it does not create a permissive second path.
6. **Provider extensibility.** A game/provider should be able to contribute
   authored vocabulary and diagnostics without editing a closed engine switch in
   several crates.
7. **Automation remains reviewable.** Generated edits remain understandable in
   the native tool and semantic diff/inspection output.

## Phases

### T1 — make moving-platform LDtk authoring excellent

Execute the LDtk/kinematic vertical slice end to end. This is the immediate
Ambition need and a good test of schema, references, validation, preview and
runtime projection.

### T2 — unify preparation diagnostics

Give world, character, action and asset preparation a consistent structured
error/provenance vocabulary that tools and external games can consume.

### T3 — improve character authoring ergonomics

Make a new character's body/kit, art, writing and placement workflow obvious
through current docs/tools without resurrecting archetypes or requiring authors
to understand D73 history.

### T4 — transactional hot reload

Use the same validation/preparation boundary for LDtk and other content revisions
where hot reload is valuable.

### T5 — provider tool extension

Prove another game can add an authored spatial/content vocabulary plus tooling
and diagnostics without modifying Ambition-specific tooling internals.

### T6 — authoring product audit

For a representative Ambition task (new room + moving platform + character +
dialogue hook), measure the number of internal concepts/commands an author must
know and remove avoidable repository archaeology.

## Acceptance

A competent developer should be able to author a meaningful Ambition room and a
small external game through supported editor/tool/provider surfaces, receive
useful errors before runtime, inspect what preparation produced, and iterate
without understanding the engine's migration history.
