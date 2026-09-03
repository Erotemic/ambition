# Engine extension model — Engine 1.0 horizon decision

**State:** OPEN / STRATEGIC — do not add runtime scripting merely to complete a feature matrix.

## Goal

Define the supported ladder for extending the engine so SDK 1.0 does not freeze
an accidental boundary.

Current preferred ladder:

```text
data/content source
 -> semantic authoring operations
 -> provider-owned prepared content
 -> prepared deterministic rules / orchestration programs
 -> semantic domain commands and observations
 -> Rust Bevy plugin/provider crate
 -> game crate / host composition
 -> engine modification
```

This is already powerful and particularly well suited to LLM agents.

For the Godot-class capability target, this ladder is judged by **what behavior a
game can express and ship**, not by whether Ambition has a GDScript-shaped entry
point. Godot's scripting/extension stack is useful evidence that games need both
low-friction authored behavior and deep native extension. Ambition may satisfy
those needs through prepared authored rules plus Rust/Bevy plugins instead.

Runtime scripting becomes necessary only if a real requirement remains unsolved:
for example user modding without recompilation, downloadable behavior, or a
deployment boundary where Rust provider crates are impractical.

See [`godot-class-2d-capability.md`](godot-class-2d-capability.md).

> **⭐ THE LADDER IS NOT ASPIRATIONAL — every rung but one has a named
> implementation, measured `544d716fe` (2026-09-02):**
>
> | rung | what implements it |
> |---|---|
> | provider-owned prepared content | `PlatformerAuthoredCatalogRegistry`, `SchemaRegistry`, `ContentPackDraft` |
> | prepared deterministic rules / orchestration | ◐ **partial** — `PreparedCondition` / `PreparedCommand` are validated immutable values (`crates/ambition_platformer2d_shared_tangle/src/authored_logic/prepared.rs:63`, `:80`; consumed by `world/authored_switch_commands` and rollback-registered as `derived.authored_switch_commands`); no general rule/sequencing representation |
> | semantic domain commands and observations | `SemanticActionId`, `ActionRegistry`, `InstalledActions` (`ambition_input/src/semantic.rs`) |
> | Rust Bevy plugin/provider crate | `game/ambition_demo_pocket` — a FOURTH-provider acceptance fixture whose manifest says it exists to prove the provider surface admits another author |
> | game crate / host composition | `ShellComposition` (`platformer2d_provider/src/composition.rs`) |
>
> ⇒ **So the page's own self-assessment checks out**, which is worth recording
> rather than assuming: the gap it names is the gap the code has. The
> orchestration rung is the only one without a general representation, and it is
> partial rather than empty — the prepared-call substrate is real and
> rollback-registered.
>
> ⛔ A rung having an implementation is not the same as that rung being GOOD. This
> table says the ladder exists, not that its steps are the right height.

⭐ **the orchestration rung is new and is the one identified gap** — authoring is
strong for nouns and weak for verbs and relationships over time. It is owned by
[`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md).
⛔ it is **not** runtime scripting: the doctrine is *Rust extends the engine's
vocabulary; authored content composes vocabulary that already exists.* That is a
strictly narrower thing than the Lua/Wasm question below, and adding the rung does
not answer it.

## Direction

- Rust/Bevy plugins are the primary behavior-extension mechanism today.
- Declarative/provider content should cover large amounts of game authoring
  without recompiling engine internals.
- Runtime Lua/Wasm/dynamic scripting is **not** a requirement until a concrete
  product/modding/deployment need demonstrates value.
- Public extension points should be semantic and narrow rather than exposing the
  whole internal crate graph.

## Relationship to Bevy ecosystem

A reusable Ambition capability should look like an ordinary Bevy plugin whenever
that model fits. See
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).

## Open design questions — deliberately unresolved

- Is user modding a flagship Ambition requirement or an ecosystem-only future?
- Would runtime scripting solve a real deployment problem that Rust providers
  cannot?
- Do we ever need dynamically loaded plugins/ABI stability?
- How do external plugins declare capability dependencies and prepared content?
- What compatibility commitment does SDK 1.0 make across engine releases?
- How does agent-native authoring expose plugin-defined vocabulary without
  repository-specific instructions?
