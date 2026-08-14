# Capability progression and world gating — Engine 1.0 program

**State:** OPEN — systemic gating is the preferred direction; capability ownership details are intentionally unresolved.

## Goal

Make exploration and progression primarily emerge from **what the controlled
body can do, what it carries/equips, and what the world has physically become**,
rather than from a long chain of story-stage flags.

Ambition should be able to gate routes through body size, movement abilities,
portal use, environmental resistance, tools, keys, powered machinery and other
mechanical facts. Explicit narrative gates remain available when sequencing is
actually the design.

## Gate families

- **body capability:** climb, fly, morph, blink, portal use, attack/tool ability;
- **body property:** size, mass class, locomotion type, damage/resistance facts;
- **item/equipment:** physical key, tool, wearable or held capability source;
- **world mechanism:** bridge repaired, machine powered, door physically opened;
- **soft systemic pressure:** danger, difficult traversal, hostile population;
- **social/knowledge:** character cooperation or discovered information;
- **story gate:** explicit authored sequencing, used deliberately rather than as
  the default progression representation.

## Engine/game boundary

The engine owns reusable requirement/capability facts and queries. Ambition owns
which theorems, characters, areas and progression meanings those facts represent.

Do not turn every gate into a generic quest condition. Conversely, if world
interaction, navigation, AI and authoring all independently need the same typed
requirement expression, promote that common vocabulary rather than duplicating
flag checks.

## Candidate crate / Bevy shape

A small capability/requirement vocabulary may eventually deserve its own crate,
but only if body construction, world interactions and reachability genuinely
share it. Prefer typed data and queries over a stringly universal expression
language.

## Open design questions — deliberately unresolved

- Which capabilities belong intrinsically to a body versus participant-level
  permanent progression?
- When possession changes bodies, which theorem abilities transfer and which do
  not?
- Can an item temporarily satisfy a capability requirement without becoming a
  body capability?
- How expressive should compound requirements be before they become an
  accidental scripting language?
- Should "knowledge" ever be an engine fact, or remain Ambition/social AI data?
- How should co-op gates behave when one participant can traverse and another
  cannot?
- What constitutes a soft gate that AI/navigation should still consider
  reachable?
