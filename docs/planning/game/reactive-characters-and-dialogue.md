# Ambition reactive characters and dialogue

**State:** OPEN / LATER — desired behavior is clear; model/provider and
durable social-memory policy remain unresolved. The 2026-11-22 source
audit selects current tactical evidence/controller corrections
[OBS1–OBS3](../engine/world-facts-observations-and-memory.md), not an AI
dialogue implementation.

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

### Use the ordinary accepted-intent boundary

No AI dialogue road exists in `crates/` or `game/`, so nothing can assert its way
into world state yet.

The [agentic-runtime owner](../engine/agentic-character-runtime.md) defines
the arrival boundary: a delayed response proposes typed intent for its
subject and observation cut, and normal domain admission decides whether
that intent can execute. Accepted intent is replayed without asking the
model again. Generated dialogue must not directly emit arbitrary
`SetFlagRequested`, quest writes or world-value changes.

Do not solve this by adding an `is_ai` flag to every effect. A character
can legitimately open a gate or transfer an item through an authorized
operation; a generated assertion cannot create that operation's result.
The selected domain must validate subject, scope and the actual request.
Use existing controller, action and causal-world-write owners when the
first concrete generated-dialogue feature is selected.

## First knowledge customer: specify evidence and reset behavior

The [world-facts audit](../engine/world-facts-observations-and-memory.md#existing-social-state-and-its-horizons)
found tactical crew reports, live retaliation, intentionally narrow saved
Player hostility and per-seat dialogue visits. It did not find a general
theft-witness, recognized-person reputation or incident-memory system.
Saved hostile flags follow checkpoint fact restore; dialogue visits
deliberately survive it. Choosing one existing field as storage must not
silently choose the new feature's reset policy.

Before implementing a response such as “I saw you strike the guard,” write
one small behavior card, with ordinary authored dialogue as the first
reader if appropriate:

| Question | Required decision |
| --- | --- |
| What happened? | The native accepted outcome: a protected strike, block and injury need not support the same line. A swing or grudge is insufficient evidence of a landed blow. |
| Who learned it? | The observer admitted at the relevant room/read cut; hearing a report is distinct from seeing the act. |
| Who was recognized? | A live body occurrence, a character kind and a lasting social subject have different meanings. Do not infer recognition from a Yarn name. |
| What persists? | Room departure, death, checkpoint restore, room replay and save/load each have an explicit answer. |
| What can the line do? | State a belief and propose an ordinary action. It cannot change authoritative history by saying the event occurred. |

Use the existing native hit reporter and final reception owner for the
incident, then an admitted observer projection. Promote only the specific
recognized fact the chosen later conversation needs. Do not persist every
contact, tactical position or generated summary. The shared OBS2 changes
make this future feature easier without selecting its product policy now.

## Open design questions — deliberately unresolved

- How much dialogue is authored, templated, dynamically selected or model
  generated?
- What character memory is persisted and what can be summarized?
- What deterministic fallback exists when a model/service is unavailable?
- How are relationship/disposition facts represented?
- Which actions may an agentic character initiate directly?
- How are multiplayer/private observations handled?
- How much story structure is still needed to maintain strong authored arcs?
