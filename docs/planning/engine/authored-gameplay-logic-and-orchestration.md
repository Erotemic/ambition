# Authored gameplay logic and orchestration

**State:** OPEN capability. The semantic condition/command and preparation
substrate exists. A general rule/sequencing representation is
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

### O1 — validate authored flag names

Lock walls hold a prepared condition. The cache is keyed on
`ConditionCatalog`'s change tick, so no road prepares a condition per tick. A
condition that fails to prepare reports room, wall id, authored text and reason;
the wall still stands. Preparation happens at room load, because a condition's
evaluator is a function pointer published through `App::publish_condition` and
content preparation (LDtk -> `RoomSpec`) has no `App`.

**Open:** a misspelled flag name prepares and then answers `false` forever.
`world.flag_set` takes `ParamKind::Name`, and every string prepares as
`AuthoredArg::Name`. `ParamKind::Reference` shows the shape of a fix: validate
against a registry of authorable flag ids. Do not build that registry until a
second consumer or a real authored mistake needs it.

### O2 — one authored-argument preparation road (closed)

Dialogue commands delegate to
`ambition_platformer2d_shared_tangle::authored_logic::prepare_authored_arg`.
The refusal sentence for a reference argument has one copy,
`dialog::reference_is_not_authorable_by_dialogue`. `ConditionCatalog::evaluate`
compares the argument kind with the descriptor before dispatch, so the catalog
owns the safety and the dialogue surface owns the wording. Guard:
`a_reference_argument_never_reaches_the_evaluator_whatever_the_surface_does`.

### O3 - keep sequencing with the domain that owns its occurrence

The concrete move-local sequencer already exists: TechniqueFlow in entity-catalog
and its interpreter in combat's MovePlayback. Three game-source flows use 3, 3
and 4 nodes. Their limited use does not imply the interpreter is absent, nor does
it justify extending it into a universal world/dialogue state machine.

The [authored technique protocol](authored-technique-admission.md) now chooses
version-1 admission: acyclic, reachable, 1-256-node programs with checked indices,
finite waits and existing per-occurrence contact latches. Finish completes the
flow but leaves recovery intact. The normal move clock/teardown ends playback;
unresolved Wait is not authority to extend it. Current charge/repeat mechanics
retain their own timing rules.

A controllable projectile with steering leases, expiry/self-hit outcomes and
release policy remains a distinct future customer. It needs typed domain lifetime
and occurrence signals before it can be represented honestly. Do not represent
those outcomes by clearing global contact latches or adding arbitrary string
signals to this validation packet. Domain-specific sequencing remains the default
where a move-local occurrence is not the owner.

### O4 - bind discovery and validation to actual installed support

Follow A11/A12 in [the frontier](actor-monolith-work-frontier.md), with their
normative [protocol](authored-technique-admission.md). Each typed capability offer
installs its handler and declares support in one place; profile finalization
freezes a read-only validation/discovery view. Duplicate keys are errors without
function-pointer comparison. Paramless, unknown, disabled and unsupported-site
cases are distinct.

One exhaustive typed visitor over expanded moves covers timeline, sustain,
on-hit, flow and schema-declared nested references. The same traversal supplies
forward/reverse inspection. Do not maintain a separate catalog that can claim a
handler is installed while preparation/runtime use another authority.

Invalid authored input must not enter the active prepared registry. Existing
moves retain their selected definition, and mechanical revisions activate at an
explicit session/reconstruction boundary first. A metadata hash is not proof of
native-code compatibility, and a typed Rust provider is not sandboxed.

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

The extension model now supports arbitrary game-specific algorithms in a separate
procedural tier. That does not require widening move-local flows, conditions or
encounter programs into one language. A module can provide an existing typed
technique or consume a different domain's published entry points. Runtime state
and domain ownership follow the same rollback contract regardless of binding.

## Non-goals

- this domain orchestration layer does not implement a general-purpose language;
  procedural modules and their shared state/port contract belong to the
  [extension model](extension-model.md);
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
