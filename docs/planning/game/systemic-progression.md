# Ambition systemic progression

**State:** OPEN — capability-first direction is settled; exact progression economy is not.

## Goal

Progress through **embodiment, tools, theorem abilities and physical world
change** more than through invisible story-stage locks.

The game may still use quests and authored sequencing, but the preferred answer
to "why can I go there now?" is something the simulation can explain.

## Ambition progression sources

- theorem abilities intrinsic to or usable by a body;
- possessed-body capabilities and physical properties;
- held/equipped tools and world-unique items;
- persistent participant-level unlocks where the design truly calls for them;
- repaired/powered/opened world mechanisms;
- knowledge/social cooperation as softer gates;
- explicit story gates only when sequence itself matters.

### Measured 2026-09-03 — the seven sources and the engine's seven gate families do not line up

[`../engine/capability-progression-and-world-gating.md`](../engine/capability-progression-and-world-gating.md)
measured seven GATE FAMILIES against HEAD. This page lists seven progression
SOURCES. Six correspond; the mismatch is not a naming difference and it lands
exactly on the question the next section calls the hard one.

| this page's source | engine gate family at HEAD |
|---|---|
| theorem abilities usable by a body | `AbilitySet` — body-owned |
| possessed-body capabilities AND physical properties | TWO families: body capability, body property |
| held/equipped tools, world-unique items | item/equipment |
| **persistent participant-level unlocks** | ⛔ **no family, and no vocabulary** |
| repaired/powered/opened world mechanisms | world mechanism (partial) |
| knowledge/social cooperation | social/knowledge (nothing route-facing) |
| explicit story gates | story gate |

⛔ **THE ENGINE HAS NO PARTICIPANT-OWNED CAPABILITY VOCABULARY AT ALL.** Every
capability family the engine measured is BODY-owned — `AbilitySet`, `mass`,
`standing_height`, `Locomotion`. A search for participant-owned unlocks
(`ParticipantUnlock`, `ParticipantCapabilit*`, `UnlockedAbilit*`) finds nothing,
while `ParticipantId` itself has ~150 references — so participants are a settled
concept and *what a participant owns* is simply not one of the things they can
have.

⇒ **That makes the next section's ambiguity currently unrepresentable rather than
merely unresolved.** It says some theorem capabilities *"may transfer across
possession; others may be body-owned"*, and that the ambiguity *"should be
represented explicitly"*. Today only one side of it can be said. A body that
flies is expressible; a participant who has learned to fly is not.
⚠ This is not a defect — nothing has asked for the other side yet, and the
engine page is right that the first slice should decide capability ownership
against real types. It is recorded because a reader of EITHER page alone sees
seven items and no gap; the gap only exists between them.

## Design pressure from possession

Do not flatten all progression into "the participant permanently owns ability
X". A different controlled body may fly, fit through a gap, resist a hazard or
lack a tool. Some theorem capabilities may transfer across possession; others
may be body-owned.

The ambiguity is part of the game design and should be represented explicitly,
not hidden behind `PrimaryPlayer` flags.

## Open design questions — deliberately unresolved

- Which flagship theorem abilities are participant knowledge versus body
  capability?
- What survives leaving/dying/abandoning a possessed body?
- Can a physical item be required for traversal even after its underlying
  entitlement was discovered?
- How should co-op handle asymmetric capabilities and temporary separation?
- How much sequence breaking is desirable?
- Which social/knowledge gates should AI/navigation treat as potentially
  resolvable versus hard blockers?
- How are progression requirements exposed to LLM world-authoring tools?
