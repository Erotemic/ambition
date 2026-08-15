# Authored gameplay logic and orchestration — Engine 1.0 program

**State:** OPEN / NAMED CAPABILITY — direction is settled, representation is not, and no implementation campaign is authorized yet. M0 is the gate.

## Goal

Let an agent author not only **what exists** in the world, but increasingly
**what the world does** — without writing a new Rust system for every ordinary
relationship between concepts the engine already models semantically.

```text
Rust extends the engine's vocabulary.
Authored gameplay content composes vocabulary that already exists.
```

And, unchanged:

```text
The deterministic simulation determines what is true.
Authored rules may invoke explicit semantic domain operations;
they do not directly mutate arbitrary ECS implementation state.
```

## Why this is now a capability gap and not an abstraction hobby

Ambition's LLM-native authoring story is strong for **nouns** — characters,
items, rooms, encounters, sprites, music, SFX, moving platforms, portals,
capabilities, prepared world content. An agent can discover, author, validate
and place all of those.

It is weak for **verbs and relationships over time**:

- when two switches are active, power a lift;
- when an item is placed somewhere, open a gate;
- after an actor observes an event, start another action;
- sequence several semantic world operations;
- latch a condition once it becomes true;
- react to a persistent world fact;
- wait for a semantic event;
- trigger an encounter consequence;
- coordinate mechanisms without a bespoke Rust system.

Today most of these fall through into hand-authored Rust.

⭐ **the evidence that a shared doctrine may be warranted is that the repository
already contains several independent partial implementations of the same idea** —
encounter scripts/triggers/effects, cutscene beats, boss phase triggers, boss
attack timelines, move/action event timelines, Yarn semantic commands, world/intro
flag chains, flag-gated mechanisms, and other hand-wired condition → action sites.

⛔ **that is evidence, not a mandate.** Their existence does not mean they are
wrong, does not mean they should be unified, and emphatically does not license a
`UniversalRuleVM`. Several of them predate the common doctrine and are perfectly
good code. They are **customers and evidence**, and M0 exists to find out which.

## The ladder this amends

[`extension-model.md`](extension-model.md) owns the supported extension ladder.
This program inserts one rung:

```text
authored data/content
    ↓
semantic authoring operations
    ↓
prepared immutable content
    ↓
prepared deterministic rules / orchestration programs   ← this program
    ↓
semantic domain commands and observations
    ↓
Rust plugin/domain implementations
```

⛔ **this rung is not Lua, not Rhai, not arbitrary runtime scripting, not
arbitrary ECS reflection, and not a Godot-style "mutate any property on any
object" surface.** See the non-goals below; they are load-bearing.

## Settled architectural direction

These are decided. Argue with the representation, not with these.

### 1. Conditions query semantic facts

A rule depends on stable semantic concepts — mechanism state, item custody,
actor/world occurrence state, capabilities, world facts, encounter state,
observations/events, reachability results where appropriate, timer/sequence
state, and provider-defined domain predicates.

⛔ a condition must not be phrased against an arbitrary ECS component layout. A
rule that reads a component is coupled to an implementation detail that the
owning domain is entitled to change.

### 2. Effects are semantic domain commands

Conceptually: open/close/set mechanism state, transfer/drop/consume an item,
command an actor, start/advance an encounter, spawn prepared content, set or
clear a world fact, publish an observation, activate a prepared behavior, request
a presentation consequence where appropriate.

⛔⛔ **there must not be one giant closed `EngineEffect` enum owned by a central
god crate.** The concrete domains own the actual operations. A new domain
participating must not require editing a central enum — that requirement is the
falsifier for whatever contract M1 proposes.

### 3. Domain ownership stays distributed; discovery may be aggregated

Each participating domain owns its condition vocabulary, its commands, their
schemas, validation, preparation, and execution adapters.

⭐ **but discovery is a different axis from authority, and this project has
previously confused them.** A composed, **read-only, derived** catalog surface is
not merely acceptable — it is the point. The installed engine should eventually
be able to tell an agent what conditions exist, what commands exist, their
parameter schemas, their documentation, their reference requirements, and which
domain owns each one.

See the doctrine correction in
[`architecture.md`](architecture.md) / [`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md):
**an authoritative central census is bad; a derived read-only index is good.**

### 4. Prepared programs are immutable

Authored source prepares into typed, validated runtime programs. Validation
happens before runtime, not on the tick. ⛔ nothing parses an expression string
during simulation. The prepared program belongs to the prepared-content epoch and
is immutable while the simulation runs.

### 5. Runtime rule state is explicit

Whatever state orchestration needs — sequence cursor, timers, once/repeat/latch,
branch state, cooldown, waiting-for-event — is visible and has stated semantics.

It must eventually participate coherently and **separately** in:

- GGRS rollback;
- room residency;
- occurrence lifetime;
- durable save/load where appropriate.

⛔ these are three or four different concerns, not one. ⛔⛔ durable gameplay
state must not hide inside opaque interpreter state — an interpreter's program
counter that decides whether a gate is open is world state wearing a disguise.

### 6. References are prepared semantically

⛔ do not build the final architecture around raw strings like `"gate_west"`, and
do not build it around runtime Bevy `Entity` values.

Use the project's existing prepared-reference and identity doctrine. This program
**integrates with** rather than bypasses the work on authored references, instance
identity, provenance, lifetime and residency — see
[`instance-lifetime-provenance-and-persistence.md`](instance-lifetime-provenance-and-persistence.md)
and the native LDtk `EntityRef` migration in
[`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md).

### 7. LLM inspection is an acceptance criterion, not polish

This is one of the main reasons to build the feature at all.

An agent should eventually be able to perform semantic operations of roughly this
shape (⚠ **names are illustrative and unsettled; the capability is not**):

```text
list-rule-conditions
describe-rule-condition item_held
list-rule-commands
describe-rule west_lift_power
validate-rule west_lift_power
explain-rule west_lift_power
show-rule-dependencies west_lift_power
dry-run-rule west_lift_power
```

⭐ **the single most important diagnostic is "why did this rule not fire?"** and
the answer must be structured, not a log line:

```text
west_lift_power
  switch_a active ........ yes
  switch_b active ........ no
      object: Hall/switch_b
      current state: released
  relay powered .......... yes

result: blocked by switch_b active
```

Treat explainability and structured diagnostics as product requirements.

## Explicit non-goals

Recorded so this plan cannot mutate into the wrong project:

- ⛔ no general-purpose programming language in the first campaign;
- ⛔ no embedded Lua/Rhai for Godot parity;
- ⛔ no arbitrary ECS reflection/mutation API;
- ⛔ no universal scene graph;
- ⛔ no universal `EngineEffect` / `EngineCondition` god enum;
- ⛔ no immediate replacement of every existing encounter/cutscene/boss/moveset
  representation;
- ⛔ no requirement that every game behavior be data-driven;
- ⛔ no requirement to eliminate Rust plugins;
- ⛔⛔ no reading of "LLM-native" as *"LLMs can write Rust, therefore semantic
  gameplay authoring is unnecessary."* The cost of a bespoke Rust system is not
  the typing; it is that it is undiscoverable, unvalidatable and unexplainable.

The boundary, restated: **use Rust to create new engine semantics; use authored
orchestration to compose existing semantics.**

## Falsifiers for the shared substrate

⭐ this program must be able to fail. It fails if any of these hold:

1. **M0 finds two existing customers whose condition/sequencing/state models
   cannot share one prepared representation without a branch that names the
   customer.** Then there is no common substrate — record the evidence and stop.
2. **A migrated customer needs more lines than it deletes.** Deletion is the
   proof; a rule layer that only adds is a wrapper.
3. **The second provider cannot be added without editing a central enum or
   registry** — that is the god-enum failure in disguise.
4. **The prepared program cannot answer "why did this not fire?"** without
   re-running the simulation. If explanation requires replay, the representation
   is wrong.
5. **Rule runtime state cannot be given rollback semantics without special-casing
   the interpreter** in rollback registration.

## Milestones

⚠ **none of these is authorized to start yet.** Priority rises when the selected
customers are ready, or when product work starts repeatedly requiring bespoke
Rust behavior wiring. Until then this is a named capability with an executable
first step.

### M0 — inventory and evidence  ⟵ the gate; everything else waits on it

Inventory the existing authored/semi-authored behavior systems and classify each
along: condition model · effect/command model · sequencing · runtime state ·
reference model · persistence/rollback semantics · preparation model ·
inspection/tooling support.

**Acceptance:**

- the major existing customers have actually been inspected at HEAD, not recalled;
- similarities **and genuine incompatibilities** are both recorded;
- no universal abstraction has been assumed;
- ⭐ two concrete proof customers are named with their expected deletions.

### M1 — vocabulary/provider contract

Prototype the smallest domain-owned contract by which **two** domains expose
semantic conditions/commands to preparation and discovery.

**Acceptance:** domain ownership stays local; a composed read-only catalog
enumerates both; ⭐ **no central engine enum is edited to add the second
provider** — this is the behavioral test, not a review opinion.

### M2 — prepared rule representation

Prepare one small deterministic condition/command program from authored source.

**Acceptance:** validation occurs before runtime; references are prepared, not
strings; the runtime parses no expression strings; program data is immutable
during simulation.

### M3 — two real customers

Migrate/prove two materially different Ambition customers.

**Acceptance:** one encounter/sequenced behavior; one ordinary world mechanism or
other independent domain; **neither requires an evaluator branch naming the
customer**; ⭐ if the abstraction becomes awkward for one customer, **stop and
revise rather than forcing convergence** — record the awkwardness as evidence.

### M4 — deterministic runtime state

If the proof needs timers/sequences/latches, give that state explicit rollback
and lifetime semantics.

**Acceptance:** rollback behavior is tested (not asserted); residency and save
expectations are stated explicitly even where durable save support is deferred.

### M5 — agent diagnostics

**Acceptance:** list conditions/commands; describe schemas; validate an authored
program; inspect dependencies; and **explain why a condition/rule is or is not
satisfied in a test fixture** — structured output, checked by a test.

### M6 — semantic refactoring integration

Feed rule references into the project-wide semantic dependency graph owned by
[`authoring-and-tools.md`](authoring-and-tools.md).

**Acceptance:** reverse-reference queries include authored rules; rename/delete
planning reports affected rules before mutation.

## Proof customers

⚠ **selected by M0, not here.** The required shape is two materially different
real Ambition customers — ⛔ toy fixtures do not qualify:

- **Customer A — a currently Rust-constructed encounter sequence.** The cut-rope
  encounter is a likely candidate if HEAD still supports that characterization.
  The proof is *not* to redesign the encounter engine; it is to show that its
  authored sequence, conditions and references can be **prepared authored
  content** rather than manual Rust assembly.
- **Customer B — an ordinary world mechanism.** A flag chain, flag-gated lock,
  switch/mechanism, power-state transition or similar small world-state reaction.
  It must be different enough from an encounter to test whether the common
  substrate is real.

⛔ if those two reveal incompatible semantics, that is a **result** — record it
here and let the program fail honestly rather than forcing one abstraction.

## Relationships to other programs

- **[Instance lifetime, provenance and persistence](instance-lifetime-provenance-and-persistence.md)
  (D125):** ⛔ this program does not derail D125 — it **consumes** it. A condition
  like *"item occurrence X is held by body Y"* must have a well-defined semantic
  answer, and that answer is D125's. Rule runtime occurrences that carry state
  need the same lifetime/persistence doctrine. ⭐ D125 makes authored rules
  easier; it is not competing with them.
- **[Platformer navigation and reachability](platformer-navigation-and-reachability.md):**
  ⛔ does not derail the first capability-aware reachability customer. But
  `body X can_reach location Y` and `exit Z reachable_by body X` are plausible
  future authored conditions — ⭐ one more reason reachability should expose a
  clean query API with diagnostic explanation rather than stay buried in AI code.
- **[Reusable authored world composition](reusable-authored-world-composition.md):**
  a reusable composition may eventually reference prepared rule programs (a lift
  composition carrying its own power/control rules). ⛔ that direction is
  recorded, not adopted — composition stays INCUBATING until real content hurts.
- **[World facts, observations and memory](world-facts-observations-and-memory.md):**
  authored orchestration is both a **consumer and a producer** of structured world
  facts/events. ⛔ doctrine unchanged: simulation determines reality; AI determines
  what characters think, infer, want, say, remember and attempt. Authored rules
  change deterministic world state through semantic operations; ⛔⛔ **LLM
  character intelligence never becomes the authoritative rule engine.**
- **[Render, animation and VFX](render-animation-and-vfx.md):** presentation
  animation and authoritative gameplay timelines are different things. ⛔ arbitrary
  property animation must not become an escape hatch around simulation authority.
- **[Extension model](extension-model.md):** owns the ladder this amends.
- **[Inspection, diagnostics and workbench](inspection-diagnostics-and-workbench.md):**
  owns the discovery/explanation surface M5 lands in.

## Open design questions — deliberately unresolved

- Is the authored surface one rule form, or several domain-shaped forms sharing a
  preparation and discovery contract?
- Do rules live in room content (LDtk), in provider content, or in their own
  authoring backend — and who owns their identity namespace?
- Is sequencing expressed as an explicit state machine, an ordered program with a
  cursor, or a behavior-tree-shaped evaluation? (See the backend question below.)
- Should a rule's runtime occurrence be an ECS entity with the ordinary scope
  components, or state owned by the domain that installed it?
- What is the evaluation order contract between rules, and how is it made
  deterministic without a central scheduler owning every domain?
- Can a rule be authored against a *definition* and instantiated per occurrence,
  or is every rule placed?
- ⚠ **Could an existing deterministic control-flow backend (e.g. `bonsai-bt`)
  serve beneath Ambition-owned semantic conditions, commands, schemas and
  prepared programs?** The question is *not* "should Ambition become a Bonsai
  engine". The required shape would be
  `authored content → prepared Ambition representation → optional execution
  backend → semantic intent → authoritative simulation`.
  ⛔ do not expose any backend's AST as permanent Ambition content ABI without
  strong evidence, and ⛔⛔ do not let a behavior-tree blackboard become
  authoritative world state. A measurement/falsification spike answering
  determinism, rollback/serialization, per-instance memory, execution cost,
  inspectability, and whether one existing actor policy reproduces cleanly is
  sufficient — this is not a dependency-adoption campaign.
