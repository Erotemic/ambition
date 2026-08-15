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
[`bevy-plugin-and-crate-strategy.md`](bevy-plugin-and-crate-strategy.md).

## Open design questions — deliberately unresolved

- Is user modding a flagship Ambition requirement or an ecosystem-only future?
- Would runtime scripting solve a real deployment problem that Rust providers
  cannot?
- Do we ever need dynamically loaded plugins/ABI stability?
- How do external plugins declare capability dependencies and prepared content?
- What compatibility commitment does SDK 1.0 make across engine releases?
- How does agent-native authoring expose plugin-defined vocabulary without
  repository-specific instructions?
