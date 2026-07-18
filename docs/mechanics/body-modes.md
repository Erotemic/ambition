---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/movement-collision.md
  - docs/mechanics/abilities.md
---

# Body modes

A body mode is an authoritative change to the actor's physical/capability
configuration: standing, crouched, crawling, sliding, compact, swimming,
climbing, mounted, armored, transformed, or another provider-defined mode.

It is not merely an animation state.

## Mode contract

A mode may change:

- collision shape and feet/anchor relationship;
- locomotion capabilities and movement tuning;
- action availability or slot meaning;
- hurtbox/armor/invulnerability policy;
- interaction reach and traversal permissions;
- derived presentation state.

Transitions are requests that must validate the destination shape and required
capabilities before committing. Expanding from a compact mode under a low
ceiling, for example, must remain compact rather than overlap geometry.

## Stable rules

- Modes belong to the shared actor/body model.
- Shape transition and collision validation are atomic.
- Gravity/orientation is explicit; “up” is not assumed to be world +Y.
- Authored surfaces and volumes expose semantic permissions rather than naming
  one character.
- Presentation derives an animation/look from the committed mode.
- Provider data chooses which bodies possess or unlock each mode.
- Reset/restore reconstructs the mode through canonical actor construction and
  snapshot seams.

## Composition over enumeration

Avoid a central enum that grows one variant for every game-specific costume or
move. Prefer reusable dimensions—shape profile, locomotion profile, capability
mask, armor/state tags, and active move—when they can vary independently.

A named mode is useful when it represents a coherent atomic transition or
provider-facing authoring concept. It should still lower to reusable body data.

## Tests

Test transition properties:

- enter/exit succeeds in free space;
- expansion is rejected under obstruction;
- contact/ground state remains coherent;
- action prompts and execution agree after the mode changes;
- equivalent non-player bodies use the same transition path;
- save/reset/replay does not leave presentation or collision in the old mode.

```bash
python scripts/agent_query.py "body mode compact crouch collision shape"
python scripts/agent_query.py tests "body mode obstruction"
```
