# Ambition reactive characters and dialogue

**State:** OPEN / LATER — desired behavior is clear; model/provider and memory design are unresolved.

## Goal

Let characters react to the world that actually exists instead of requiring
every interesting response to be hand-wired to a quest-stage integer.

Authoritative state remains in the simulation. Character intelligence/dialogue
consumes structured world facts, observations, relationships, goals, inventory
and recent events, then chooses what to say or try to do.

## Desired behavior

Characters should eventually be able to react to facts such as:

- which body a participant currently controls;
- what that body carries or has equipped;
- where important items/actors are;
- whether a mechanism/bridge/gate was physically changed;
- what the character personally observed or was told;
- relationship/disposition changes;
- nearby danger or ongoing encounters;
- persistent world changes since the last meeting.

Authored dialogue, Yarn content and hand-written scenes remain valuable. Agentic
or generated dialogue should extend them, not make continuity unknowable.

## Hard boundary

An AI may be wrong about the world. It may not **change** whether the key exists,
whether the bridge is repaired, or whether an item is in someone's inventory by
merely asserting it in dialogue.

### Measured 2026-09-03 — this is a RULE today, and the place it must become a mechanism has a name

✔ **Not currently violable: there is no AI dialogue road at HEAD.** A sweep for
`llm` / `LLM` / `openai` / `anthropic` across `crates/` and `game/` finds three
matches and all three are prose in comments (e.g. *"an ability a brain (or an
LLM) drives an actor's clusters into"*). Nothing generates dialogue, so nothing
can assert its way into world state yet.

⛔ **But the boundary has no mechanism behind it, and the road it would have to
police is public.** World mutation from gameplay goes through a message bus:
`SetFlagRequested` (`crates/ambition_combat/src/events.rs:80`),
`QuestAdvanceRequested` (`crates/ambition_persistence/src/quest/mod.rs:392`) and
their siblings, drained by `features::ecs::effect_bus`
(`apply_flag_effects`, `apply_quest_effects`, `apply_switch_effects`). Those
messages are ordinary public vocabulary written from at least four modules today
— chests, interactions, world facts, the encounter switch road. **The bus records
WHAT was asked and never WHO asked.**

⇒ So when a dialogue generator arrives, "it may not change the world by asserting
it" cannot be enforced by convention at the call site — every existing writer
looks the same to the bus. The boundary needs an authority distinction the bus
does not have: either a separate request type an AI may write and a translator
that refuses to promote it, or a provenance field the drain can reject on.
⚠ That is a small design decision NOW and an audit of four-plus call sites LATER.
It is recorded here because the page's own framing — *"authoritative state remains
in the simulation"* — reads as already-enforced, and it is a rule that currently
depends on nobody having written the offending code.

## Open design questions — deliberately unresolved

- How much dialogue is authored, templated, dynamically selected or model
  generated?
- What character memory is persisted and what can be summarized?
- What deterministic fallback exists when a model/service is unavailable?
- How are relationship/disposition facts represented?
- Which actions may an agentic character initiate directly?
- How are multiplayer/private observations handled?
- How much story structure is still needed to maintain strong authored arcs?
