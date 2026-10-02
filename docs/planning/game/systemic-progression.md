# Ambition systemic progression

**State:** OPEN — capability-first direction is settled; exact progression economy is not.

## Goal

Progress through **embodiment, tools, theorem abilities and physical world
change** more than through invisible story-stage locks.

The game may still use quests and authored sequencing, but the preferred answer
to "why can I go there now?" is something the simulation can explain.

## Ambition progression sources

Engine gate families are owned by
[`../engine/capability-progression-and-world-gating.md`](../engine/capability-progression-and-world-gating.md).

| progression source | engine gate family |
|---|---|
| theorem abilities usable by a body | body capability (`AbilitySet`) |
| possessed-body capabilities and physical properties | body capability; body property |
| held/equipped tools and world-unique items | item/equipment |
| persistent participant-level unlocks | **none** — no vocabulary exists |
| repaired/powered/opened world mechanisms | world mechanism (partial) |
| knowledge/social cooperation (softer gates) | social/knowledge (nothing route-facing) |
| explicit story gates, only when sequence matters | story gate |

Every engine capability family is body-owned (`AbilitySet`, mass, standing
height, locomotion). The engine has no participant-owned capability type. So "a
body that flies" is expressible and "a participant who has learned to fly" is
not. Nothing has asked for the participant side yet. The first slice that needs
it should decide capability ownership against real types.

## Design pressure from possession

Do not flatten all progression into "the participant permanently owns ability
X". A different controlled body may fly, fit through a gap, resist a hazard or
lack a tool. Some theorem capabilities may transfer across possession; others
may be body-owned. Represent that ambiguity explicitly, not behind
`PrimaryPlayer` flags.

Current rulings for routes:

- **A route asks the body a participant is driving.** `body.can(verb)` and
  `body.fits(height)` read the driven body, not the home avatar the player left
  behind. Nothing transfers across possession for routes.
- **A body gate is evaluated per actor** (Q54 in
  [`../maintainer-decisions.md`](../maintainer-decisions.md)). A wall is open for
  the body that satisfies it and solid for the body beside it that does not
  (`a_body_gate_is_open_only_for_the_bodies_that_satisfy_it`). Do not open a gate
  for the party because one member qualifies.

## Open design questions — deliberately unresolved

- Which flagship theorem abilities are participant knowledge versus body
  capability?
- What survives leaving, dying in or abandoning a possessed body? (Answered for
  routes: nothing. Open for progression.)
- How should co-op handle asymmetric capabilities and temporary separation?
  (Answered for gates: per actor. Separation is A3 in
  [`multiplayer.md`](multiplayer.md). Open for progression.)
- Can a physical item be required for traversal even after its underlying
  entitlement was discovered?
- How much sequence breaking is desirable?
- Which social/knowledge gates should AI/navigation treat as potentially
  resolvable versus hard blockers?
- How are progression requirements exposed to LLM world-authoring tools?
