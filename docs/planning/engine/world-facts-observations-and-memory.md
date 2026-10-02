# World facts, observations and memory — Engine 1.0 program

**State:** OPEN. The separation of authoritative world truth from AI belief is
settled. The durable fact layer exists as typed save families. The general
observation and memory representation is not decided.

## Goal

Give systemic characters and agent tooling structured access to **what is true,
what happened, and what a particular actor could know**, without making an LLM
or dialogue generator authoritative over the simulation.

> The simulation determines what is true. AI decides what characters think,
> want, say and try to do about it.

## Three layers

### Authoritative world facts

Examples: door open, machine powered, item custody, actor alive/location,
encounter outcome, persistent world mutation.

**Current shape.** The durable fact layer is `AmbitionGameSaveData`: typed fact
families (flags, switches, items, wallet, occurrences, custody, minted items,
encounters, bosses, quests, dialogue visits, checkpoint, inventory-saved), not a
key-value map. Its fields are `pub(crate)` behind readers and named setters, so
no other crate writes a durable fact by assignment. Paired setters keep one
fact whole: `set_inventory` also sets `inventory_saved`, and
`set_durable_horizon` writes occurrences with custody.
`scripts/durable_fact_writers.py` lists who writes each family.

Authored rules read facts through the condition catalog. Each domain publishes
its own conditions beside the systems that write the fact:

| Condition | Reads | Published by |
| --- | --- | --- |
| `world.flag_set`, `world.switch_on` | the save | `actor_monolith/world_facts.rs` |
| `boss.cleared` | the save (keyed by authored encounter id) | `ambition_boss_encounter` |
| `encounter.cleared` | the save | `ambition_encounter_features` |
| `quest.active` | the save | `game/ambition_content` (the game publishes it) |
| `inventory.holds` | live ECS | `actor_monolith/items` |
| `custody.is_held` | live ECS | `ambition_held_items` |
| `wallet.can_afford` | live `BodyWallet` | `actor_monolith/items/wallet_conditions.rs` |
| `body.can`, `body.fits` | live ECS | `actor_monolith/body_conditions.rs` |

The save is the durable mirror of the rule-readable surface, not the surface
itself. A rule asking "does the player hold X" wants the live hand. The facts
read from the save are the ones with no live form between sessions (a cleared
boss, a set flag). `scripts/authored_route_gates.py` counts which families
conditions read.

`wallet.can_afford` reads the price through `ambition_items::shop::authored_price`,
the same reading `<<buy_item>>` uses, so a price the guard refuses is a price the
transaction refuses. A no-wallet composition answers `Unanswerable`, not `false`.

`dialog_visits`, `checkpoint`, `minted_items` and `inventory_saved` publish no
condition. Do not publish one per field: `inventory_saved` and `minted_items`
are restore mechanics no rule should ask about.

**The Yarn mirror.** `ambition_dialog::YarnStateMirrorData` holds only
`visit_counts` (dialogue's own bookkeeping) and content `extras`. Every other
Yarn function asks the catalog live or reads the live component
(`wallet_balance()` reads `BodyWallet`). A new mirror field is a claim that the
catalog cannot answer the question, and the burden is on the field. Enumerate
the authored verbs bound over a field, not only the field.

**Switch writers.** The save's switch family has two writing roads, disjoint
by action kind: `drain_switch_activations` (`ambition_encounter/src/switches.rs`)
and `apply_wave_encounter_effects` (`ambition_encounter_features/src/systems.rs`,
which greens every switch of a completed encounter). The falling-sand spouts
only read (`FallingSandSpoutState::from_save`). The activation queue is a
cross-tick channel: the drain is ordered before `EncounterSimulation` by its
consumers, and `apply_switch_effects` pushes in `GameplayEffects`, so an
activation resolves on the next tick
(`a_switch_activation_is_drained_on_the_tick_after_it_was_pushed`). Ordering the
drain after the push would be a schedule cycle.

**Open: which source a condition reads.** When a fact has both a live form and a
durable row, the choice is implicit in each evaluator (`boss.cleared` argues
its choice in its docstring). A `reads: FactSource` field on
`ConditionDescriptor` would force the choice, but nothing would read it yet. Add
it with its first consumer: the condition inspector showing the source, a guard
that durable gates do not use live-reading conditions, or agent tooling that
filters by source.

### Observations/events

Structured facts that a character or system could have perceived: saw body X,
heard event Y, received item Z, witnessed gate opening. No durable
representation exists. The save records what is true, not what happened or who
could have seen it.

### Memory/belief

Actor-specific retained interpretation of observations. It may be incomplete,
stale or wrong without changing world truth.

**Tactical belief is built.** `WorldMemory` (`ambition_characters::perception`)
is the per-controller belief that outlives the viewport: keyed by actor id,
refreshed for what is seen, decayed for what left view, forgotten below a
confidence floor. Its `update` is pure. It is rollback state, and it records the
live room it was formed in, so a body that changes live room forgets it. It is
in no durable save family.

Durable social knowledge ("this NPC knows you stole the thing") is a different
mechanism. `WorldMemory` decays by construction, which is right for sight and
wrong for a grudge. Do not extend `WorldMemory` to carry durable knowledge.

## Why this matters

- reactive dialogue without giant quest-stage switches;
- agentic character planning constrained by reality;
- explainable LLM context instead of dumping raw ECS state;
- social/knowledge gating that remains separate from physical capability gates;
- debugging of "why does this character believe that?".

## Deterministic authored orchestration is both a consumer and a producer

[`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)
will read world facts and observations as rule **conditions**, and will set or
clear facts and publish observations as rule **effects** — through explicit
semantic domain operations.

That makes it a demanding early customer of whatever fact/observation
representation this program picks: a fact that cannot be named in an authored
condition, or whose change cannot be observed, is not usable by a rule.

The governing rule above is unchanged by this. Authored rules alter
deterministic world state through semantic operations; **LLM character
intelligence never becomes the authoritative rule engine.** Simulation determines
reality; AI determines what characters think, infer, want, say, remember and
attempt.

## Candidate crate / Bevy shape

Do not begin with a universal key-value fact database. Prefer typed domain facts
and a narrow observation/projection seam. A common journal/memory crate should
emerge only if several domains need the same retention/query semantics.

An LLM adapter must sit above deterministic world state, not below it.

## Open design questions

Answered for the tactical-belief slice (see
[`bounded-perception-and-attention.md`](bounded-perception-and-attention.md)),
open for the general program:

- Observation permission for a body perceiving other bodies is viewport
  containment within its own live room, not line-of-sight
  (`peer_is_visible_to_body`). The omniscience escape is a policy.
- The remembered-actor set is rollback state (`WorldMemory` has a
  `from_snapshot` road), so attention ordering carries an id tiebreak.
- The per-tick kept set is bounded at `TACTICAL_ATTENTION` (16), hostiles first
  and nearer first, with the rest kept as counts. Retention over time is open.

Open:

- Typed facts/components versus an extensible fact registry?
- Which events deserve durable history and which are ephemeral messages?
- How is observation permission determined for the general program?
- How long should memories persist, and what is saved?
- Should beliefs support contradiction/uncertainty explicitly?
- What facts are private to a participant in multiplayer?
- How are summaries generated for LLM context without losing critical detail?

## Knowledge reduction at the observation boundary

A consumer of a published fact needs less knowledge of its producer. An observation view may expose a
stable, scoped consequence without handing its reader the producer's mutable
resources, broad context or callback. Keep fact identity, observer memory and
live simulation authority distinct.

An agent response based on an observation needs subject/session and revision or
expiry checks before normal action admission. Replaying accepted intentions must
not rerun external observation/model queries. Multi-instance scope comes from the live room
(`InRoomInstance`), not from a universal world-facts bus.

## Procedural callers receive the appropriate knowledge contract

A portable module is not automatically omniscient. An actor controller consumes
its granted observation/knowledge projection; an explicitly authorized world-rule
module may consume wider authoritative facts. Diagnostic inspection is a separate
read-only role. [Domain contracts](extension-domain-contracts.md) records that
scope with the port, including instance, coordinate frame, phase and missing data.

World truth, an observation and a belief remain different values. Do not create
one generic WorldSnapshot that silently gives limited-knowledge brains every
fact or makes gameplay rules read stale beliefs. Caches pin their source revision
and read cut; restoring or clearing a derived cache cannot change the outcome.
