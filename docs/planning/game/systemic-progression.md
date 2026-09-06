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

✔ **RE-DERIVED 2026-09-05 and holding, every part of it.** All three searches
still return ZERO files (`ParticipantUnlock`, `ParticipantCapabilit`,
`UnlockedAbilit`), and `ParticipantId` is at **157** references. ⭐ It also agrees
with an independent measurement taken the same day from the other end — the
capability page's layer table (authored defaults → `AbilityBase` →
`BodyAbilities`, plus a session mask) finds **no participant layer at all**, and
its one non-body layer is an INTERSECTION that can only take verbs away. ⚠ That
layer is the F3 inspector (`EditableAbilitySet` lives in `ambition_dev_tools`),
which strengthens this row rather than weakening it: the sole non-body layer is
not even a game mechanism.

⇒ **That makes the next section's ambiguity currently unrepresentable rather than
merely unresolved.** It says some theorem capabilities *"may transfer across
possession; others may be body-owned"*, and that the ambiguity *"should be
represented explicitly"*. Today only one side of it can be said. A body that
flies is expressible; a participant who has learned to fly is not.
⚠ This is not a defect — nothing has asked for the other side yet, and the
engine page is right that the first slice should decide capability ownership
against real types. It is recorded because a reader of EITHER page alone sees
seven items and no gap; the gap only exists between them.

✔ **RE-DERIVED 2026-09-04 evening and UNCHANGED**, recorded so the next reader
does not spend the search again: `ParticipantUnlock` / `ParticipantCapabilit*` /
`UnlockedAbilit*` still return **0** across `crates/` and `game/`, while
`ParticipantId` returns **154** (the "~150" above). ⇒ The asymmetry is intact —
participants are a settled concept and *what a participant owns* is still not
one of the things they can have.
⚠ **And it did not move on a day when the gate vocabulary moved a lot.** Ten
conditions are published now, up from six, and `body.can`/`body.fits` made body
capability route-readable — none of which touches the participant side. That is
the useful shape: **the engine grew the BODY half of this table and left the
participant half at zero**, which is what the next slice would have to change.

## Design pressure from possession

Do not flatten all progression into "the participant permanently owns ability
X". A different controlled body may fly, fit through a gap, resist a hazard or
lack a tool. Some theorem capabilities may transfer across possession; others
may be body-owned.

The ambiguity is part of the game design and should be represented explicitly,
not hidden behind `PrimaryPlayer` flags.

### ⭐ AND THE ENGINE RULED ON BODY-OWNERSHIP IN CODE, 2026-09-04

The gap above is no longer only "unrepresented" — the route road now actively
answers it in the body's favour, and a reader of this page should know that a
decision has been taken on one axis while the vocabulary is still missing on the
other.

`body.can(verb)` and `body.fits(height)` asked *"any body holding `PlayerEntity`
OR `DrivingParticipant`"* until 2026-09-04. Possession moves the seat OFF the
home avatar and onto the target (`control/authority.rs:39-45`) while the home
KEEPS `PlayerEntity` — so a wall gated on climbing opened on the strength of the
body the player had left behind, and refused a possessed vessel that could climb.
Both directions are fixed and pinned, and the production-path test drives the
disagreement through an authored `GatedLockWall`.

⇒ **The ruling, in this page's own vocabulary: NOTHING TRANSFERS. A route asks
the body a participant is DRIVING.** That answers *"what survives leaving a
possessed body"* **for routes** — nothing, because the route never asked about
the participant. ⚠ It does NOT answer it for progression: whether a participant
carries anything across bodies is the other side of the gap above, and is still
unrepresentable.

⛔⛔ **AND THE CO-OP QUESTION BELOW IS NOW LIVE RATHER THAN HYPOTHETICAL —
filed as `awaiting-maintainer-decision.md` #54.** The predicate falls back to an
existential over every `DrivingParticipant` holder, so with two seats **a wall
gated on `body.can wall_climb` opens when EITHER driver can climb**, and the seat
that cannot walks through a wall its own body never satisfied. The code answers
"the party", by default, today.
⚠ The per-BODY answer is not a predicate change: `gate_solids` is one
`Vec<Block>` on one overlay read by body collision, projectiles and rendering
alike, so a wall that stands for one player and not another is a mechanism
change. ⇒ Which makes this page's *"do not flatten all progression into the
participant permanently owns ability X"* pressure concrete: the flattening
already happened for co-op, in the direction of the PARTY, and it happened
because nobody chose.

## Open design questions — deliberately unresolved

- Which flagship theorem abilities are participant knowledge versus body
  capability?
- ◐ What survives leaving/dying/abandoning a possessed body? **Answered for
  ROUTES 2026-09-04 — nothing does; a route asks the driven body.** Open for
  progression.
- ⛔ How should co-op handle asymmetric capabilities and temporary separation?
  **LIVE, not hypothetical — filed as awaiting-maintainer-decision #54.** The
  code answers "the party" today, by an existential nobody chose.
- Can a physical item be required for traversal even after its underlying
  entitlement was discovered?
- How much sequence breaking is desirable?
- Which social/knowledge gates should AI/navigation treat as potentially
  resolvable versus hard blockers?
- How are progression requirements exposed to LLM world-authoring tools?
