# Instance lifetime, provenance and persistence — Engine 1.0 program

**State:** OPEN — the definition/instance/lifetime separation is settled; exact identity types are not.

## Goal

Give the engine a coherent answer to three different questions:

1. **What authored thing is this?**
2. **Which runtime occurrence is this?**
3. **Why does this occurrence exist, and how long should it continue to exist?**

The same model must accommodate a normally unique named character, twenty mobs
from one `CharacterDefinition`, a persistent quest object, an encounter-spawned
pickup, a summoned actor and an ephemeral projectile without pretending they
all need the same persistence semantics.

## Model

```text
authored definition
        +
spawn provenance
        ↓
runtime instance
        +
lifetime policy
        +
persistence policy
        ↓
explicit terminal transition
```

Authored identity does **not** imply world uniqueness. The engine may instantiate
the same `CharacterDefinition` or item definition multiple times. "Normally one
Fia" or "the West Tower key is unique" is content/game policy, not the meaning
of the definition ID.

## Provenance examples

- authored placement;
- encounter/wave spawn;
- quest or world-mechanism grant;
- loot/drop result;
- summon/ability result;
- runtime/system spawn;
- debug/test spawn.

## Lifetime examples

- persistent world instance;
- session-persistent instance;
- room-resident instance;
- encounter-bound instance;
- condition-bound instance;
- timed instance;
- intentionally ephemeral simulation object.

Rollback participation and save persistence are separate axes. An encounter mob
may need rollback but not durable save identity.

## Candidate crate / Bevy shape

Do not immediately invent one `UniversalInstanceId`. Domain-specific instance IDs
may be clearer. Extract a common instance/provenance crate only after actors,
items and world objects demonstrate genuinely shared operations rather than
similar vocabulary.

A reusable Bevy plugin could own generic provenance/lifetime components and
messages if the semantics remain independent of Ambition's save and story
policy. See [`bevy-plugin-and-crate-strategy.md`](bevy-plugin-and-crate-strategy.md).

## Open design questions — deliberately unresolved

- One common instance ID versus typed actor/item/world-object IDs?
- Which IDs survive save/load, and which may be regenerated deterministically?
- Does an authored placement retain one persistent instance identity after it is
  picked up/moved, or is placement only provenance?
- Do terminal instances require tombstones to prevent respawn, and for how long?
- How should world-unique/per-owner-unique validation be expressed without making
  uniqueness fundamental to definition identity?
- How do instance records relate to ECS `Entity` generation/reuse?
- How much provenance must be serialized versus retained only for diagnostics?
