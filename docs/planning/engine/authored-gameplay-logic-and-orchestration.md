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

⚠ **THIS ROW'S PREMISE WAS STALE AND ITS PRESCRIPTION IMPOSSIBLE; both corrected
2026-09-02 against the code.** It said `gated_lock_walls` "still assembles
condition arguments at runtime rather than holding a prepared condition produced
at content-preparation time". Two things wrong with that:

- the per-tick re-mint was already fixed by `68d80d653` (2026-08-26, *"The lock
  wall holds its question instead of re-minting it every frame"*); the row was
  last edited 2026-08-30 by a bulk docs pass that did not re-verify it;
- ⛔ **"produced at content-preparation time" CANNOT HAPPEN HERE.** Preparation
  needs the `ConditionCatalog`, and a condition's evaluator is a Rust FUNCTION
  POINTER published through `App::publish_condition`. Content preparation
  (LDtk → `RoomSpec`) is a data transformation with no `App`, so the catalog
  cannot exist there. The earliest point where the authored text and the catalog
  coexist is room load — which is where preparation already happens.

✔ **WHAT WAS ACTUALLY LEFT, AND IS NOW DONE** (acceptance clauses 6 and 7):

- the last per-tick preparation road is deleted. A per-frame `.or_else(prepare)`
  retry existed to cover a provider publishing AFTER the first room was cached;
  the cache is now keyed on `ConditionCatalog`'s change tick, so that case is
  handled once on the edge instead of by re-preparing every wall every tick;
- ⛔⛔ the silent soft-lock is gone. `prepare_question` ended in `.ok()`, throwing
  away a `PreparationError` that carries the authored source AND the reason — its
  own doc says it keeps the source because *"a diagnostic an author cannot act
  on"* is useless. An unpublished condition therefore produced a wall standing
  FOREVER, in a room the player cannot finish, with nothing written anywhere. It
  now reports room, wall id, authored text and reason. Behaviour is unchanged —
  the wall still stands — what changed is whether anyone can find out.

▢ **WHAT REMAINS, and it is a design question rather than a slice.** Clause 2
("preparation catches invalid ids/arity/types before runtime") is met for ids and
arity but NOT for the authored flag name, and cannot be as things stand:
`world.flag_set`'s only param is `ParamKind::Name`, whose `prepare_one` arm is
`Ok(AuthoredArg::Name(text.to_string()))` — every string is valid. A MISSPELT
flag prepares perfectly and then answers `false` forever, which is
indistinguishable from a flag legitimately unset, and the wall stands with no
diagnostic. `ParamKind::Reference` does validate (namespace, non-empty body,
known namespace), so the shape of an answer exists — it needs a registry of
authorable flag ids to validate against. ⛔ Do not build that registry
speculatively; it wants a second consumer or a real authored mistake first.
✔ **Premise re-verified against the code 2026-09-02 and it still holds exactly**:
`prepared.rs`'s arm is `ParamKind::Name => Ok(AuthoredArg::Name(text.to_string()))`
— unconditional — so every authored string still prepares. Nothing has quietly
fixed this and the deferral is still the right call; recorded so the next reader
does not have to re-derive it.

### O2 — collapse duplicated authored-argument preparation

Dialogue-authored commands still have their own text-to-`AuthoredArg` conversion
path. Reuse the shared preparation semantics if doing so removes a real duplicate
without forcing dialogue control flow into the shared substrate.

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
