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

## ⭐⭐ THE LIVE CUSTOMER — persistent occurrence continuity (2026-08-15)

⚠ **two of the "deliberately unresolved" questions below stopped being
hypothetical today**, and they were forced by real product pressure rather than
by design appetite. The cross-room custody slice made a carried item survive a
room transition — and the moment an occurrence outlives the room that authored
it, reconstruction has a question it cannot currently answer.

**The question, stated once:**

> When authored placement **P** has produced a persistent runtime occurrence that
> subsequently **moved**, was **consumed**, was **destroyed**, or entered
> **custody elsewhere**, how does world reconstruction know what should happen
> to P?

The concrete defect: an authored floor item has a placement-derived `SimId`.
Carry it out of room A, reload room A, and construction instantiates the authored
placement again — so `placement(room_a, axe_17)` names **two live occurrences**,
one carried and one freshly built.

⭐ **this is the systemic-world problem the project has been trying to reach**,
and it is no longer *"how does an inventory work?"* It sits underneath persistent
items, moved NPCs, opened/removed mechanisms, destroyed objects, relocated quest
objects, persistent populations, room streaming and save/load.

**The falsifier, brutally concrete:**

```text
enter room A → authored axe P exists → pick up P → carry P to room B
→ return to A → P must NOT respawn → the original occurrence still exists elsewhere
```

and eventually the complementary terminal cases:

```text
P destroyed permanently      → reconstruction knows not to recreate it
P intentionally resettable   → reconstruction MAY recreate it
```

⛔⛔ **do not solve this by teaching the room loader to inspect inventories** —
that merely creates another composition census, and the custody slice's whole
achievement was that room transition never learned items exist.

⭐ **the abstraction belongs around the STATE/DISPOSITION of the authored
occurrence**, with the storage representation discovered from this one customer.
⛔ we still do not need a universal `EverythingInstanceRegistry`.

### ⭐⭐ THREE HORIZONS — the maintainer's decision of 2026-08-15

**The checkpoint is the reset baseline.** Death/retry restores the latest
*committed* checkpoint; ordinary traversal and unload preserve current state.
⇒ a key item survives a death because acquiring it **committed a checkpoint**,
⛔⛔ **not because it is a key item** — an item-kind rule would be a second
authority that disagrees with the checkpoint the moment content changes.

So the model needs **three** horizons, not two:

```text
1  current occurrence state          what is true right now
2  state at the reset/checkpoint     what a death restores to
3  durable save state                separate, and still later
```

⚠ **that is the sharp version of the "do not mix derived and authoritative"
rule**: `AuthoredOccurrences` today mixes a derived live projection with what
would become authoritative state, and the middle horizon is exactly the piece
that has nowhere to live.

**Acceptance fixture (the maintainer's own):**

```text
C0 key on pedestal → pick up → die         → key RETURNS to the pedestal
pick up again → commit C1 → die            → key stays acquired, pedestal EMPTY
temporary item picked up after C1 → die    → key stays acquired, temporary RESETS
```

⭐ **the third line is load-bearing.** A `KeyItem => survives reset` special case
satisfies the first two and fails it — which is precisely why the rule is
checkpoint-shaped rather than item-shaped.

⚠ **a sandbox reset is a DIFFERENT road from death/retry** — it rebuilds the start
room from authored records alone and restores nothing outside room scope.

### ✔ How the three horizons actually compose (answered 2026-08-15)

✔✔ **HORIZON 2 IS BUILT (2026-08-15). All three of the maintainer's lines hold
through production roads** — `game/ambition_app/tests/death_restores_the_checkpoint.rs`,
seven beats in `central_hub_basement`, including the one that kills the
item-kind reading: one death, two objects of the same kind, opposite answers,
separated only by which side of the checkpoint each acquisition fell on.

⛔⛔ **BUT THE PREDICTION THIS SECTION MADE WAS WRONG, AND THE CORRECTION IS THE
PART WORTH KEEPING.** It said the baseline *is* a copy of the whereabouts
ledger. It is not — it is a **projection of DOMAINS**, each capturing from its
own live authority:

```text
lifecycle::horizon    two messages, two sets, and NOTHING else
OccurrenceBaseline    what became of each authored occurrence
CustodyBaseline       which BODY was carrying which occurrence, both by SimId
```

A single ledger copy cannot express line 2, and this section said why without
drawing the conclusion: an `InCustody` row says *somebody* has it, which is
enough to stop a room minting a second one and **not enough to put it back**.
The custodian's identity is a fact the BODY's domain owns, and stuffing it into
the occurrence ledger would put two questions with different owners and
lifetimes on one value.

⭐ **and a reset really is the DEGENERATE CASE**: a host that never emits
`CheckpointCommitted` restores the empty baseline, which is what a sandbox reset
means. That prediction held.

⭐⭐ **three things the fixture found that this design did not**, all by being
run:

1. **restoring the ledger and emptying the hand DELETES the object.** The room
   replay resets features in place and never re-runs authored construction, so
   nothing authored it back. ⇒ a death is a checkpoint **resume**: it records the
   same `LifecycleIntent::Transition` a session-start resume records.
2. **custody is a FORKED relation** — `ItemCustody` on the object, `HeldItem` on
   the body. Retracting one half left the body holding a ghost and refusing every
   later pickup. ⛔ the tempting generic repair would disarm every authored
   fighter, whose `held_item` needs no world object at all.
3. **a hand must be EMPTIED before it can be FILLED.**

⛔ **this section used to end "what horizon 2 still cannot do: put back a
baseline row whose occurrence has no live entity". CLOSED — twice, and the
second time is the interesting one.** See the third and fourth legs below. The
baseline listing above is also one short: there are THREE domain projections
now, and the third is not an identity.

✔ **the body-inventory leg this section said was blocking is answered for this
customer and no further.** `CustodyBaseline` records physical custody by
identity at a horizon boundary; `OwnedItems` was not migrated, and ⛔ should not
be on this customer's account.

⚠ **and treat the current residency mechanism as a bridge, not the answer.**
`InCustodyOf` / `RoomResident` is right for today's **single-active-room** host,
but `RoomScopedEntity` does not encode *which* room owns an occurrence, so
"released" resolves to *"whatever room is active"*. ⭐ that is exactly why the
slice needed no memory of the destination — and exactly what will not survive
participants occupying different rooms simultaneously, which needs keyed,
explicit ownership. ⚠ **the second leg (below) did NOT close this**: the ledger's
`Placed { room, .. }` now names the room an occurrence is lying in, which is the
value a keyed `RoomScopedEntity` would need, but the scope marker itself is
unchanged.

### ✔ First leg landed 2026-08-15 — and the shape it chose is the part to keep

A rebuilt room now asks **what became of the occurrence it minted last time**.

⭐⭐ **the question is a DISPOSITION, not a liveness probe.**
`OccurrenceDisposition::{Authored (default), Persisting, Consumed}` — and
construction retains only requests whose disposition `authors_a_fresh_occurrence()`.
⛔ *"is something with this id alive?"* would have been the tempting phrasing and
the wrong one: it answers today's custody case and has nowhere to put permanent
destruction, deliberate respawn, or a persistent actor that simply moved.

⭐ **`Consumed` is spelled and read but has NO PRODUCER yet, deliberately.** That
reserves the honest slot for permanent destruction and makes **ephemeral /
resettable the DEFAULT** rather than a special case — so the terminal cases named
above have somewhere to land instead of being retrofitted.

⭐ **the authority is STATED, including stating `None`.** Construction gained
`occurrences: Option<&AuthoredOccurrences>` under the same contract the cast and
brain policies already use, so a seventh construction road cannot silently forget
it. ⛔ **that is the pattern; an implicit default is how the eighth road gets
missed.**

✔ **deleted with it:** an 84-line `RoomConstructionPlan::prepare(&World, ..)` with
**zero callers** — itself a road an added authority would have been forgotten on —
and `RoomConstructionError::MissingService`, its only raiser.

⚠ **two risks carried forward, both recorded rather than fixed:**

- **`SimId::placement(id)` is a GLOBAL namespace whose uniqueness is only checked
  PER ROOM.** Pre-existing, and now load-bearing: two rooms authoring the same
  placement id would suppress both.
- **the ledger is not experience-scoped**, so a suppressed row can survive into a
  new session. Every consumer treats absence as *"author it"*, so the failure mode
  is a stale suppression, not a stale spawn.

⚠ **and a defect its own test found:** a **carried object survives a reset**. The
first test to actually *execute* a reset contradicted a sibling that asserted the
scope component was present and **inferred** the sweep consequence in prose.
⛔ an inferred consequence is not a checked one.

### ✔ Second leg landed 2026-08-15 — construction RECONSTRUCTS residency

⭐⭐ **room construction stopped being a pure function of one `RoomSpec`.** What a
room owes the world is its current RESIDENCY — derived from the world's
DEFINITIONS plus the authoritative disposition of every occurrence — and an
occurrence carried out of the room that minted it and put down elsewhere belongs
to the room it is lying in, under a record that room does not own.

⭐ **the two halves are one row and land together, enforced by a TYPE.**
`outlook_for` now answers `Reinstated` in the room an occurrence lies in and
`Suppressed` in every other room including its home — and construction takes
`OccurrenceContinuity { remembered, world }`, one value carrying the ledger and
the world's room definitions. ⛔ **a road holding only the ledger could perform
the suppression and was structurally unable to perform the reinstatement**, which
is a permanent DELETION traded for a duplication; that road is no longer
expressible. Exactly one caller states it (the room-transition load), and every
other construction road still states `None` and means it.

⭐ **the foreign lookup is bounded by the ledger, not by a scan.** A room derives
records from another room only for identities the ledger says are lying in it and
its own records do not produce, and stops the moment the debt is settled —
`reinstatable_authored_requests`, paired with `relocate_request`: an occurrence
gets a `Placed` row only from a producer that read a POSITION off it, so a family
joins both functions or neither.

⚠ **two consequences recorded rather than fixed:**

- a foreign room that cannot yield its records **refuses** this build (a
  preflight failure, raised while the outgoing room is still whole) rather than
  silently dropping the occurrence. The same defect already makes that room
  unbuildable.
- a debt no record in the world can settle — the record was edited away — is a
  **warning**, not a refusal: refusing would make the room permanently
  unenterable for a content change.

⛔ `Consumed` still has **no producer**, deliberately, and horizon 2 is still
blocked on the three things that do not exist.

### ✔ Third leg landed 2026-08-16 — a death can MATERIALIZE what it puts back

A baseline row whose occurrence has no live entity is rebuilt **by identity**
from the authored record that minted it, wherever in the world that record lives,
directly into the custodian's hand — `authored_occurrence_request`, driven from
`items::pickup::restore_custody_to_checkpoint`. It comes back with the record's
own `SimId` and `SpawnOrigin`, which is what makes it the same occurrence rather
than a look-alike.

⛔ **and it named its own boundary**: materialization is bounded by *"some room
authors a record with this id"*.

### ✔ Fourth leg landed 2026-08-16 — and it is the first step of DURABLE SAVE

The inhabitant of that boundary is a **runtime-minted instance**
(`SpawnOrigin::Dynamic`). It is room-scoped and carryable, so it can enter the
custody baseline, and **no record anywhere can rebuild it.** The production road
that mints one is the unclosed inventory leg: `OwnedItems` is a count table, the
menu equips out of it, and throwing what it equipped turns a QUANTITY into an
INSTANCE.

⭐⭐ **THE MINIMAL DURABLE DESCRIPTION, MEASURED RATHER THAN ASSUMED:**

```text
identity     the occurrence's own SimId          the map key
provenance   SpawnOrigin::Dynamic{parent, seq}   what makes it re-mintable AGAIN
definition   the item spec's authored id         what it IS — a REFERENCE, not a copy
```

**and nothing else.** No position, no velocity, no component snapshot.

⭐ **"a hand needs strictly less than a world" HELD.** A held object has no place
in the world — the hand supplies one, and `ground_item_physics` refuses to step
anything whose custody is not `InWorld`, so `GroundItem::pos` is not read while
it is carried. ⚠ that is a claim about restoring into a HAND. The day a
checkpoint owes a minted instance back to the FLOOR, the description grows a
position, and nothing today remembers one.

⛔ **the third field is where the prediction was one short.** `(identity, spec)`
looked sufficient and is not: a rebuilt instance with no `SpawnOrigin::Dynamic`
cannot say which spawner it descends from — the state that component's own doc
refuses to let anyone spell — and would therefore be invisible to the NEXT
capture. It would survive exactly one death and then become unrecoverable.

⚠ **and the mint site was not stating it.** `throw_held_item_system` minted a
`SimId::spawned(...)` and no `SpawnOrigin` at all, so the only legitimate way to
tell a mint from an authored placement did not exist on the object. Identity and
provenance are now minted as ONE `Option`, so "dynamic, parent unknown" stays
unspellable.

⭐ **the shape follows the domain rule this page already learned**: the baseline
is a projection of DOMAINS, each capturing from its own live authority, and
`MintedItemBaseline` lives with the item domain because only it knows what an
item is. The lifecycle crate's two projections stay identity-only.

⛔⛔ **it is a SNAPSHOT AT COMMIT TIME, never a registry of every mint** — and
that is measured, not asserted. Turned into a growing registry written at each
tick, with the restore rebuilding every row into its spawner's hand, the
"banked mint comes back" fixture stayed green and
`a_runtime_mint_the_checkpoint_never_saw_is_not_resurrected_by_a_death` went red.
The restore is still driven by the CUSTODY baseline: this map answers *how*, the
custody baseline decides *whether* and *into whose hand*.

⚠ `GGRS_ROLLBACK_SCHEMA_VERSION` 32 → 33: `resource.minted_item_baseline` is the
third leg of the same checkpoint and the first that is not an identity.

⚠ **what durable SAVE still needs that this did not provide**: a minted instance
that is NOT in a hand at the commit (lying in a room, in flight) is still
undescribed and still lost; `OccurrenceWhereabouts::Placed` rows for unloaded
rooms have no on-disk representation; nothing here is serialized to a file, only
to a rollback blob; and `OwnedItems` — the QUANTITY half of the same inventory —
persists through a different mechanism that the horizon does not coordinate with.

## D132 — the two authorities, measured (2026-08-16)

`game/ambition_app/tests/two_persistence_authorities_for_one_item.rs` is the
first fixture that puts the durable save and the checkpoint in one sentence. It
saves a count of 1, loads, equips out of the count table, throws (which MINTS an
instance), picks it up, banks it at a shrine and dies. What it measured:

```text
holding it AND owning it?   BOTH — an instance in the hand and a stored row
decremented once/twice/never?   NEVER, at any beat
second save round-trip agrees?  yes, and by coincidence rather than by rule
```

⭐ **so the predicted double-restore is not what is wrong.** The real defect is
next door and the fixture that found it is
`a_death_that_returns_the_object_leaves_nothing_in_the_catalog_claiming_it`:
`OwnedItems` **is not checkpoint state at all**, so an acquisition made after the
checkpoint sticks permanently while the object it referred to is taken back. The
pressed pickup used to `grant` a catalog row beside taking custody — one
acquisition, two records, and only one of them rewound. The player kept owning a
gun-sword lying back on its shelf, the durable save wrote the phantom to disk,
and the menu would equip it and mint a second real one on the next throw.

⛔ **the fix is a DELETION, and both halves are falsified by probe**: the pickup's
`grant` is gone, and `OwnedItems::count` PROJECTS the equipped slot instead —
restoring either one turns the fixture red. `to_persisted` reads the stored
quantity, never the projection, so a hand never reaches disk as a row. The two
populations are disjoint now: a row is a quantity with no object, an object is an
occurrence the checkpoint owns.

⚠ **THE GATE for the other half, named rather than taken.** A quantity conferred
by `<<give_item>>`/shop/drop still keeps its row through the mint, so it can
still manifest a second object. Spending the row at the mint is the obvious fix
and is wrong while the catalog sits outside the checkpoint horizon: a death that
retracts an instance minted after the checkpoint would find the quantity already
spent and annihilate it — the mirror image of the phantom. `OwnedItems`
participating in the checkpoint baseline is the prerequisite, and the mint can
spend the row in that same change and not before.
`a_granted_quantity_survives_the_death_that_retracts_the_instance_minted_from_it`
is the poison that keeps the wrong shortcut (retracting the row at the reset)
out.

⚠ **and a composition finding**: the durable-save leg (`InventoryRestored` plus
`restore_inventory_from_save`/`persist_inventory_to_save`) is installed by
`install_menu_setup_and_hotkeys`, inside `add_presentation_plugins` — "visible
binary only". No headless composition schedules it, which is a large part of why
the two authorities had never met in a test.

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
- ⛔ ~~Does an authored placement retain one persistent instance identity after it
  is picked up/moved, or is placement only provenance?~~ **NO LONGER OPTIONAL —
  see the live customer above.** A carried item now outlives its authoring room,
  so this must be answered, not deferred.
- ⛔ ~~Do terminal instances require tombstones to prevent respawn, and for how
  long?~~ **the same customer forces this**: "destroyed permanently" and
  "intentionally resettable" are two of its three terminal cases.
- How should world-unique/per-owner-unique validation be expressed without making
  uniqueness fundamental to definition identity?
- How do instance records relate to ECS `Entity` generation/reuse?
- How much provenance must be serialized versus retained only for diagnostics?
