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

> **⚠ THE CANDIDATE LIST, measured `255e1ec0a` (2026-09-03) — a list, NOT a verdict.**
> O3's gate turns on whether a real feature's current format handles triggers
> POORLY, which is a judgement about authoring experience and not something a
> grep can settle. What a grep can supply is who the candidates are, so whoever
> makes that call starts from the actual consumers:
>
> - `authored_switch_commands` (`ambition_platformer2d_actor_monolith::world`) —
>   4 references, and rollback-registered as `derived.authored_switch_commands`;
> - `gated_lock_walls` — 4 references;
> - the substrate itself,
>   `crates/ambition_platformer2d_shared_tangle/src/authored_logic/prepared.rs:63`
>   and `:80`.
>
> **Two consumers, both in the same crate, neither yet reported as struggling.**
> ⛔ On this page's own rule that is NOT a promotion trigger — it is the evidence
> that the trigger has not fired, recorded so the next reader does not have to
> re-derive it before deciding to keep waiting.
>
> ⭐ Worth knowing from the other side: `extension-model.md`'s ladder measures
> this rung as the ONLY one without a general representation, and calls it
> partial rather than empty for exactly the substrate above. The two pages agree,
> which is itself worth recording — most of the cross-plan joins checked this
> week did not.

✔✔ **THE GATE FIRED 2026-09-05 — Jon, after reading the tree and the moves
already built.** The row above recorded two consumers "neither yet reported as
struggling" and correctly concluded the trigger had not fired. A third customer
has now been named, and it satisfies this row's own three clauses rather than
two of them:

- **a trigger representation its current format handles poorly** —
  PK-Thunder-style behaviour: spawn a projectile, lease steering input to it,
  restrict fighter control, wait on *self-hit / other-hit / expiry*, and branch
  to a directed launch. A `MoveSpec` timeline can state WHEN something happens;
  it cannot state *what happens next, based on what happened before*, which is
  the whole of this move.
- **semantic effects that already have owning domains** — every operation in it
  is owned elsewhere already: projectile spawning and trajectory by projectile
  authority, input routing by control authority, fighter motion by movement
  authority, contact by the relevant contact authority. ⭐ Not one of them
  belongs to the sequencer, which is why this customer tests the rung rather
  than smuggling in a general one.
- **who owns per-occurrence runtime memory, and its rollback semantics** —
  `MovePlayback`, extended with current flow node, trigger/input latches,
  issued-event bookkeeping, timeouts and local symbolic slots. ✔ Verified
  2026-09-05 against `crates/ambition_combat/src/moveset/mod.rs`: it already
  carries `t`, `landed_hit` / `connected_hit` / `blocked_hit` / `hit_targets`,
  `aim` / `aimed_stick`, `charge`, `looped_s` and `instance` — per-use
  deterministic state that is rollback-carried today. The flow adds a cursor to
  an occurrence record that already exists rather than a second lifetime.

⛔⛔ **AND IT IS SCOPED SO IT CANNOT BECOME THE UNIVERSAL SEQUENCER THIS PAGE
REFUSES.** Named `TechniqueFlow` / `MoveFlow`, explicitly NOT `ActionGraph` —
Jon's reason is that the latter "sounds like a universal gameplay execution
model, which is precisely what the planning docs are trying not to introduce".
The node vocabulary is `emit` / `wait` / `branch` / `finish` plus symbolic slots,
with **no** variables, arithmetic, arbitrary expressions, arbitrary ECS queries,
general scripting, behaviour trees or global blackboard. ⇒ It is MOVE-SCOPED:
"a domain decides *when* to ask/effect" still holds, and the domain here is the
move. A slot exists so a move references a semantic occurrence (`"thunder"`)
instead of holding an `Entity`, which is what keeps rewind, inspection and
remote detonation clean.

⇒ Design and capability map:
[`expressive-move-capabilities.md`](expressive-move-capabilities.md).
Execution order:
[`../demos/campaigns/expressive-moves-2026-09-05.md`](../demos/campaigns/expressive-moves-2026-09-05.md).

### O4 — keep discovery/inspection first-class

Authoring and agent tooling should be able to enumerate:

- available conditions and commands;
- parameter kinds and documentation;
- prepared source location/provenance;
- preparation failures;
- the owning domain/capability.

Do not require a developer to search Rust registration topology to discover the
authored vocabulary.

⭐ **A CONCRETE ASK LANDED HERE 2026-09-05, from the same direction that fired
O3, and it names the exact hole.** `ParamSchemaRegistry` intentionally accepts an
unknown effect key, so it can report *the parameters for `smash.teleport` are
malformed* but not *`smash.teleprot` does not exist*. ⇒ Add an **installed
technique descriptor catalog** — preparation, inspection and tooling only, ⛔
never the runtime reducer — carrying per technique: key, owning
capability/domain, documentation, parameter schema, where it may be used
(event / sustained window / on-hit), the signals it may produce, and examples.
Then make the Smash provider's content-finalization pass strict (**every
authored `EffectRef` must resolve to an installed technique**) and expose
`smash_tool techniques`, `smash_tool technique <key>` and
`smash_tool mechanics <domain>`.

ⓘ **The motivation is agent behaviour, not typo-catching.** Jon: authoring
agents do not reach for capabilities unless told they exist, and a catalog makes
it materially harder to decide *"I guess I need to write a new system"* before
discovering that capture, teleport, stored charge and mount already do it.

✔ M5, the why-not half, landed 2026-09-02: `ConditionOutcome::NotSatisfied`
carries `WhyNot { term, subject, observed }` and every production evaluator
states one; `GatedLockWallVerdicts` is the first read model built on it. See the
inspection plan's why-not item for what is still open.

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
