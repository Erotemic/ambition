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

## Open design questions — deliberately unresolved

- How much dialogue is authored, templated, dynamically selected or model
  generated?
- What character memory is persisted and what can be summarized?
- What deterministic fallback exists when a model/service is unavailable?
- How are relationship/disposition facts represented?
- Which actions may an agentic character initiate directly?
- How are multiplayer/private observations handled?
- How much story structure is still needed to maintain strong authored arcs?
