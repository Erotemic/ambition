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
encounter signaling, dialogue-authored commands, and cutscene `Command` and
`WaitFor` beats with the `actor.*` vocabulary
([story sequences](story-sequences.md)).

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

### O5 - castle completion is a concrete authored domain sequence

**Real customer:** Mary-O's castle in commit `16186fc` (2026-10-11).
`game/ambition_demo_mary_o/src/castle_end.rs` reacts to `FlagSequence` and
`ControlledSubject`, searches for Void Lizards by their brain key, and writes
`BodyHealth.health.current = 0`. It then registers a Rust-built
`CutsceneScript` during gameplay and keeps a global tally counter below
`TALLY_HOLD` while the banner plays. The shipped test proves the castle
advances, but not two simultaneous castle occurrences or encounter death
phase/reward behavior.

**Authority gap:** `ambition_boss_encounter::encounter_script::tick_encounter_scripts`
already handles `EncounterEffect::ForceKill` by reducing member health **and**
advancing the encounter's phase. The castle's direct health write does not.
A killed ordinary enemy is not necessarily an encounter participant; first
check whether the Void Lizard is intentionally a normal actor or an encounter
member. Do not silently convert it into a `BossEncounter` just to reuse an
effect. Make its kill go through its actual owning damage/death protocol.

**Target:** the castle level authors a switch, a bridge state, a defeat
condition, a named rescue cutscene and a completion gate. The room/sequence
domain owns the order and the one-shot latch for that occurrence. Domain
commands own bridge state, actor defeat, cutscene request and level transfer.
Use the existing prepared condition/command and encounter-gate roads where
their semantics match. When they do not, add the smallest typed request to
the *owning* domain. Resolve authored IDs once at preparation; reject a
missing goal, boss reference, bridge or cutscene with room/source diagnostics.
The cutscene belongs to prepared content, not a system that repairs a
missing session library inside the simulation tick.

**Phase and scope:** no `single()` over every castle's `FlagSequence`, and
no implicit global `ControlledSubject` for independent encounters. Bind the
completion to a live-room and participant/goal occurrence; the authored next
room is still the authority for travel. Two parallel endings cannot share a
single mutable cutscene request or dwell counter without an explicit policy.
Rewind must restore the switch, bridge, living/dead members, cutscene latch
and score consistently. Preserve the current visible Mary-O sequence.

**Acceptance:** one real castle switch triggers one death (with the right
phase/reward semantics for the actor's kind), one cutscene and one transfer;
a second castle in another live room is untouched; a second participant does
not take over the first one's sequence; a rewind before the switch replays the
same result once. Poison: update `BodyHealth` from the demo's castle-system
callback, or register the cutscene inside a simulation tick; the boundary
test must reject the duplicate authority. Do not add a universal sequencer.

### O6 — bind invocation roles through the existing call boundary

**Planned from the 2026-11-21 source review:**
[story SS7](story-sequences.md) supplies two customers: Creator Yarn dialogue
and intro cutscene direction. Their commands and conditions need the same
occurrence-bound participants and room context. Preparation validates a role
parameter; the scene/conversation binds it through the existing occurrence
resolver. A raw string still cannot forge a reference, so O2's refusal remains
correct for unbound text.

Extend the current prepared-call boundary and its command/condition context.
Keep required/optional roles, fixed cohorts and live population queries
distinct. Bind later-spawned actors at their actual creation milestone; do not
require every role at scene start. Playback and conversation owners retain
their separate cursors and rollback state. This is the invocation counterpart
to [construction's local references](reusable-authored-world-composition.md),
not another global name registry or a new target-query language.

SS8 also makes application and completion distinct for scene skipping. Reuse
the existing native action outcomes. The shared command catalog does not
decide how a throw, departure or scene completes.

#### O6a — bind the merchant's question and action to the same subject

**Done 2026-11-23.** The shop's payer rule is now one function,
`ambition_platformer2d_items::narrative::payer_among`: the body driving
the conversation's seat, or with no seat the one primary. Two candidates
name no payer.

- **Shop:** `apply_shop_transactions` charges by it.
- **Questions:** `shop_payer` reads the same rule off a world for a
  question asked before the purchase.
- **Subject form:** `wallet.can_afford` publishes `can_afford_for` (that
  body's own purse).
- **Catalog:** `ConditionCatalog::evaluate_of` asks a subject through the
  same checks and the same recorded verdict as `evaluate`. A condition with
  no subject form is unanswerable there, not answered for the population.
- **Yarn:** `can_afford(n)` and `wallet_balance()` select the payer from the
  speaking conversation's input owner at each read. Nothing is stored
  between the offer and the purchase. With no single payer, `can_afford`
  answers false (logged) and the balance line shows 0.

A temporary probe was then removed. With 100 coins on the primary and 0 on
seat 1, the old global answer for an 8-coin offer was satisfied and the
payer answer was not. Reversed, the old answer refused and the payer answer
allowed. Both times the payer was seat 1's body, the purse `buy_item`
charges. The generic `condition("wallet.can_afford", n)` form keeps the
primary-wallet population answer; no shipped Yarn uses it. The two-seat
merchant was not driven through a live session here.

The audit as written:

**Open, source review at `f41218979625`:** the current merchant supplies a
smaller customer than a new role-authoring surface. In
`game/ambition_content/src/yarn_vocabulary.rs`, `ask_can_afford` asks the
global condition form and `ask_wallet_balance` reads `PrimaryPlayer`.
`wallet_conditions.rs` also selects the primary body. However, `buy_item`
and `sell_item` use the executing `RunnerConversation`'s input-owner seat,
and `platformer2d_items::narrative` charges that seat's driven body.

With 100 coins on the primary body and 0 on seat 1, seat 1's 8-coin merchant
option can open and then refuse. Reverse the balances and an affordable
option can close. This follows from source; it was not run in this review.
The amount parser is already shared. The selected subject is not.

Use the existing `SubjectConditionEvaluator`, `publish_subject_condition`
and prepared condition boundary. Resolve the conversation's payer policy
once per read/consume cut and pass the selected body to body-relative
questions. Keep these meanings separate:

| Meaning | Required behavior |
| --- | --- |
| Source attribution | `AuthoredAsk` identifies the node or source that asked; its string is not a body reference |
| Invocation context | The executing conversation supplies its occurrence, input owner and bound roles |
| Question subject | The domain's declared receiver selects the body or other occurrence whose state answers the question |
| Action authority | The domain rechecks the request against current state at acceptance |

Global world questions stay global. A required subject that cannot be
resolved answers `Unanswerable`; it does not silently become a primary-body
query. Keep the current explicit scripted/no-participant payer policy, or
bind an explicit payer role when that customer needs one. An initiator,
speaker and input owner are not interchangeable.

Extend one existing query entry to retain both provenance and subject.
`ConditionCatalog::ask_for` currently bypasses the global path's verdict
diagnostics; using it directly would lose source attribution. Select the
receiver form at preparation where possible, then dispatch through the same
diagnostic boundary. Numeric `wallet_balance` uses the same subject resolver;
it does not require turning the condition catalog into a numeric registry.
If that numeric read has no subject, apply its explicit adapter refusal or
presentation policy. A display fallback is not a proved gameplay balance.

The preview and action share a payer-selection policy, not a stored Entity
from an earlier option display. A possession change or a spend between those
cuts requires a fresh lookup and a fresh decision. Keep the path bounded to
the selected subject; no per-actor offer table or per-frame query cache is
needed. The merchant correction can land before all of SS7's role binding.

#### O6b — preserve an unanswered question through the dialogue decision

**Done 2026-11-23.** `verdict(id, arg)` (in
`ambition_conversation/src/dialog/authored_conditions.rs`) answers with the
verdict word: `"satisfied"`, `"not_satisfied"` or `"unanswerable"`. Each
Boolean alias has a verdict form: `boss_cleared_verdict`,
`quest_active_verdict` and `can_afford_verdict` (`yarn_vocabulary.rs`).
`decided` binds each answer once for the runner's current continuation. A
runner continues at most once a frame, so the binding is keyed by speaking
runner and frame and is discarded with the decision. It is not a Yarn
variable.

The 44 shipped `<<if>>` statements with another arm moved to verdicts, and
so did the three negated asks. Each question's `"unanswerable"` takes a
refusal arm before the next question is asked. The last question's negative
is an explicit `"not_satisfied"` arm, and the final `<<else>>` refuses. A
gameplay gate refuses with `<<stop>>`. The Kernel Guide's diagnostic pages
are presentation only: there the refusal arm says that nobody could answer.

The intro's compound `found and not checked` refuses when an operand it
needs is unanswered. Positive-only gates and option gates keep the Boolean
forms, because an unanswered question there hides the branch.

The real-Yarn harness proves the three arms, one ask per decision, the
compound, and a downstream `<<mark>>` command that an unanswered question
never reaches. `dialogue_lint::no_shipped_gate_reads_an_unanswered_question_as_no`
refuses two things in shipped content: a negated Boolean ask, and a Boolean
ask in an `<<if>>` that has another arm.

The text below is the plan as written.

**Open, same source review:** the native `ConditionOutcome` distinguishes
`Satisfied`, `NotSatisfied(WhyNot)` and `Unanswerable`. The Yarn adapter in
`ambition_conversation/src/dialog/authored_conditions.rs` returns only
`outcome.is_satisfied()`. Thus an unanswered question becomes false, and
`not condition(...)` turns it into true. An ordinary `else` also merges an
answered negative with a missing answer.

The shipped `sandbox/intro.yarn` uses negated encounter/flag questions and a
compound flag question. This proves a real authoring customer for the lost
verdict; it does not prove that those installed intro domains currently fail
to answer. The existing missing/malformed-question test covers a positive
`if` and an informational `else`, so it does not prove the negated case.

The supported gameplay gate must keep the result until it selects an arm:

| Native result | Supported gameplay decision |
| --- | --- |
| `Satisfied` | The positive arm is eligible |
| `NotSatisfied` | The explicit answered-negative arm is eligible |
| `Unanswerable` | The selected refusal/abort path; no arm may treat it as an answered negative |

Use an outcome-valued binding over Yarn's existing scalar/string values and
migrate the actual gameplay gates to explicit positive and answered-negative
comparisons, with an explicit refusal path. Bind a result once for the
decision; do not re-query the domain separately for each arm. Consume that
result in the current runner's immediate, non-yielding decision. Ordinary
Yarn variables are shared between runners. If a result must survive a line,
option or wait, use explicitly runner-local transient storage and discard it
when the decision ends. It must not become a shared variable or a cross-beat
query cache. Compound gates
must preserve an unanswered operand when that operand is needed to decide
the gate. Include provider aliases such as `can_afford`, not only the generic
`condition` function. Keep presentation-only fallback policy explicit.

This is a bounded adapter/content migration. It does not make arbitrary Yarn
Boolean expressions preserve three verdicts. In the supported authoring
surface, negative gameplay branches must test the answered-negative result;
`!= satisfied`, Boolean negation and a catch-all `else` are not substitutes.
Use the current authoring/compiler boundary to identify unsupported uses as
the relevant files migrate. Do not add a new expression language.

An alternative can preserve the Boolean syntax only after proving a
synchronous host refusal that stops the current continuation before a branch
or command executes. The local `RunnerFunction::call_with_world` returns a
`YarnValue`, not `Result`; the current pending-close path is not that proof.
Do not plan an imaginary `return Err` or assume a next-update close prevents
commands in the current continuation.

#### O6 delivery and removal

Land O6a first with the real two-seat merchant. The
[item exchange packet](item-custody-and-accounting.md#a7-exchange--one-rule-for-an-offer-and-its-accepted-exchange)
then makes the offer ask about the same concrete exchange that acceptance
will evaluate. Preview remains advisory; it reserves no coins or quantities.
These corrections do not wait for the general verdict migration.

For O6b, extend the real-Yarn harness with all three outcomes, the current
positive/negative/compound forms, and a downstream gameplay command marker.
An unanswered required question must not reach that command. Static ID and
arity checks cannot prove a live subject or domain state exists.

Remove the merchant's private primary-wallet selection and the migrated
gates' Boolean verdict collapse. Retain one source-attributed call boundary,
the existing condition catalog, and domain-owned action outcomes. The result
is a reusable invocation contract, not a new dialogue sequencer.

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
