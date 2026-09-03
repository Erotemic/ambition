# World facts, observations and memory — Engine 1.0 program

**State:** OPEN — authoritative-world/AI-belief separation is settled; fact and memory representation is not.

## Goal

Give systemic characters and agent tooling structured access to **what is true,
what happened, and what a particular actor could know**, without making an LLM
or dialogue generator authoritative over the simulation.

The governing rule is:

> The simulation determines what is true. AI decides what characters think,
> want, say and try to do about it.

## Three layers

### Authoritative world facts

Examples: door open, machine powered, item custody, actor alive/location,
encounter outcome, persistent world mutation.

### Observations/events

Structured facts that a character or system could have perceived: saw body X,
heard event Y, received item Z, witnessed gate opening.

### Memory/belief

Actor-specific retained interpretation of observations. This may be incomplete,
stale or wrong without changing world truth.

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

⭐ that makes it a demanding early customer of whatever fact/observation
representation this program picks: a fact that cannot be named in an authored
condition, or whose change cannot be observed, is not usable by a rule.

⛔ the governing rule above is unchanged by this. Authored rules alter
deterministic world state through semantic operations; **LLM character
intelligence never becomes the authoritative rule engine.** Simulation determines
reality; AI determines what characters think, infer, want, say, remember and
attempt.

## Candidate crate / Bevy shape

Do not begin with a universal key-value fact database. Prefer typed domain facts
and a narrow observation/projection seam. A common journal/memory crate should
emerge only if several domains need the same retention/query semantics.

An LLM adapter must sit above deterministic world state, not below it.

## Open design questions — deliberately unresolved

⭐ **THREE OF THESE HAVE SHIPPED ANSWERS FOR THE TACTICAL-BELIEF SLICE, and this
page did not know (checked 2026-09-03).** They are NOT answers for the general
fact/memory program — that is still open, and the layer below is one customer,
not the design. But a reader re-deriving them from scratch would be redoing work
that is already in the tree and already rollback-registered. See
[`bounded-perception-and-attention.md`](bounded-perception-and-attention.md).

- *"How is observation permission determined: proximity, line-of-sight, room,
  explicit communication, something else?"* — for a body perceiving other
  BODIES it is **viewport containment**, and deliberately not line-of-sight:
  `peer_is_visible_to_body` is
  `perception.knows_bodies_anywhere() || viewport.contains(peer.pos)`. No
  raycast, no room test. The omniscience escape is a policy, not a fallback.
- *"What parts, if any, participate in deterministic rollback?"* — the actor's
  remembered-actor set does. `WorldMemory` is rollback state with a
  `from_snapshot` road in `snapshot_impls.rs`, which is why the attention
  budget's ordering carries an id tiebreak: two peers at equal distance must be
  kept in the same order on every host or the snapshot diverges.
- *"How long should memories persist, and what is saved?"* — partially, and only
  the second half: what is CARRIED each tick is bounded at
  `TACTICAL_ATTENTION` (16), hostiles first and nearer first, with the remainder
  kept as counts and one distance rather than dropped silently. ⚠ That bounds
  the per-tick kept set, NOT retention over time, which is still open.

⚠ The remaining questions below are untouched by that work.


- Typed facts/components versus an extensible fact registry?
- Which events deserve durable history and which are ephemeral messages?
- How is observation permission determined: proximity, line-of-sight, room,
  explicit communication, something else?
- How long should memories persist, and what is saved?
- Should beliefs support contradiction/uncertainty explicitly?
- What facts are private to a participant in multiplayer?
- How are summaries generated for LLM context without losing critical detail?
- What parts, if any, participate in deterministic rollback?
