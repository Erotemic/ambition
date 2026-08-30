# Authored gameplay logic and orchestration

**State:** OPEN capability, narrowed 2026-08-30. The semantic condition/command
and preparation substrate exists. A general rule/sequencing representation is
still deliberately unchosen.

## Goal

Let authored content ask semantic questions and request semantic domain actions
without moving domain authority into a universal scripting engine.

The stable split is:

```text
authored source
    -> preparation / validation
    -> semantic condition or command
    -> owning domain evaluates/reduces it
    -> authoritative simulation
```

The substrate owns description, preparation and discovery. Domains own mutable
state and sequencing policy.

## Landed architecture

### Semantic conditions

Conditions query domain facts through registered semantic condition vocabulary.
The authored surface does not reach arbitrary ECS components or execute Rust
callbacks from strings.

### Semantic commands

Commands publish typed semantic requests to the domain that owns the mutation.
The authored layer does not become a second reducer for encounters, dialogue,
world state or another domain.

The existing command road already has real customers, including authored
encounter signaling and dialogue-authored commands.

### Prepared calls

`PreparedCondition` and `PreparedCommand` are validated immutable values.
Preparation resolves:

- registered semantic id;
- arity;
- parameter kinds;
- typed references such as `SimId` namespaces.

Runtime consumers do not reparse authored text. Invalid calls are refused before
they can become runtime work.

### No universal sequencer

The earlier census found several legitimate control-flow shapes: monotonic
cursors, interruptible boss-pattern state, dialogue/Yarn control flow, encounter
state machines, and event-triggered one-shot commands.

That evidence argues **against** forcing every customer into one universal
program counter, behavior tree, or engine-owned sequencer.

A domain decides *when* to ask/effect. Shared authored-logic infrastructure
answers *what semantic question/request is being made*.

## Current work

### O1 — adopt prepared conditions where a real per-tick parser still exists

`gated_lock_walls` remains a concrete candidate: it still assembles condition
arguments at runtime rather than holding a prepared condition produced at
content-preparation time.

A promoted slice should delete the runtime validation/parsing road rather than
introduce another wrapper around it.

### O2 — collapse duplicated authored-argument preparation

Dialogue-authored commands still have their own text-to-`AuthoredArg` conversion
path. Reuse the shared preparation semantics if doing so removes a real duplicate
without forcing dialogue control flow into the shared substrate.

### O3 — wait for a real rule-form customer

Do **not** build `when ... then ...`, an ordered generic rule list, a behavior
-tree AST, or a universal state-machine format merely because prepared conditions
and commands now exist.

Promote a rule representation only when a real authored feature needs both:

- a condition/trigger representation that its current domain-specific format
  handles poorly; and
- a semantic command/effect that already has an owning domain.

The first implementation must state who owns per-occurrence runtime memory and
how that memory participates in rollback/lifetime semantics.

### O4 — keep discovery/inspection first-class

Authoring and agent tooling should be able to enumerate:

- available conditions and commands;
- parameter kinds and documentation;
- prepared source location/provenance;
- preparation failures;
- the owning domain/capability.

Do not require a developer to search Rust registration topology to discover the
authored vocabulary.

## Determinism and lifetime

Prepared program data is immutable. If a future authored rule has mutable
runtime occurrence state—cursor, cooldown, branch memory, trigger history—that
state belongs to an explicit domain/session occurrence and must follow the same
rollback, stable-identity, deterministic-order and lifetime rules as other
authoritative simulation state.

A blackboard owned by an execution backend is not automatically valid gameplay
authority.

## Optional execution backends

A deterministic behavior-tree/state-machine library may eventually implement a
specific domain's control-flow backend. That is an implementation choice behind
Ambition-owned semantic content contracts.

Do not expose a third-party AST as permanent Ambition content ABI without a real
customer and evidence for deterministic execution, rollback/state ownership,
inspection and maintenance value.

## Non-goals

- no general-purpose scripting language;
- no arbitrary ECS reflection/mutation from authored content;
- no universal sequencer owned by the shared substrate;
- no central god registry that absorbs domain mutation authority;
- no runtime string parsing when content could have been prepared;
- no speculative behavior-tree adoption.

## Acceptance for the next promoted slice

A slice should demonstrate all of:

1. a real authored customer;
2. preparation catches invalid ids/arity/types before runtime;
3. the runtime holds prepared semantic values rather than authored text;
4. the owning domain remains the sole mutation authority;
5. any runtime occurrence memory has explicit rollback/lifetime ownership;
6. a duplicated hard-coded or per-tick interpretation path is deleted;
7. diagnostics can explain the authored source and semantic owner.

## Exit

This plan remains open because the representation for reusable authored rules is
intentionally unresolved. It can close when real customers have either proven a
small shared rule form or demonstrated that prepared semantic calls plus
independent domain control-flow backends are sufficient.
