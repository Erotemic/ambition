# Instance lifetime, provenance and persistence — Engine 1.0 program

**State:** ⭐ **MOSTLY BUILT AT HEAD (measured 2026-08-14) — read the mapping
below before designing anything.** The definition/instance/lifetime separation is
settled AND implemented, under names this document does not use. What is still
genuinely open is **persistence policy** and **the explicit terminal transition**.

## ⛔ What already exists, and what to call it

| This document's question | HEAD's answer |
|---|---|
| What authored thing is this? | `WornCharacter(CharacterId)` + `PreparedCharacterRegistry` — and it already splits NAMING a template from APPLYING it (`RecharacterizeBody`) |
| Which runtime occurrence? | **`SimId`** — deterministic, namespaced (`placement:`/`slot:`/`encounter:`/`spawned`/`strike`), dynamic spawns minted as `(spawner SimId, per-spawner counter)`; every snapshot row and checksum projection keys on it |
| Why does it exist? | **`SpawnOrigin`** — `Authored` / `ProviderStaged` / `Dynamic{parent: SimId, sequence}`, `parent` non-optional, verified against the construction roster, encoded into rollback blobs |
| How long should it last? | four ENFORCED scopes, each owning a sweep: `RoomScopedEntity`, `ModeScopedEntity`, `RoundScopedEntity`, `SessionScopedEntity`, plus per-domain TTLs and `EncounterCleanupPolicy` |

⭐ **`SpawnOrigin`'s module already states this plan's own rule** — *provenance is
data, never recovered by parsing an id string* — and `round.rs` states the hardest
one unprompted: *"round scope is a LIFETIME, not a provenance; where an entity
CAME FROM does not say how long it should live."*

⛔⛔ **the only gap found was a FALSE DECLARATION.** `RunScopedEntity` and
`PersistentEntity`, with `spawn_run_scoped` / `spawn_persistent`, had zero
producers and zero consumers and no sweep read them — so two of `SpawnScopedExt`'s
four verbs silently did nothing, and a call site declaring "dies with the run"
got an entity outliving every boundary the engine has. Both are deleted.
`RunScopedEntity` duplicated `SessionScopedEntity`; `PersistentEntity` was a
second spelling of absence, since every sweep culls on marker *presence*. ⇒ **a
scope is spelled here only if a sweep enforces it.**

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
