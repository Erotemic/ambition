# Item custody and accounting — Engine 1.0 program

**State:** OPEN — custody is settled and the seam is now MEASURED (the partition below): 5 of the catalog's 6 classes are counts forever, and the whole problem is the nine held weapons/abilities that are an instance and a count at once. ⭐ the blocking unknown is no longer *which items* — it is **who owns a body inventory**, because until that exists the count is doing durable-save duty for an instance and cannot simply be spent.

## Goal

Make it possible to answer, for every authoritative physical item instance:

> Where is it now, who or what has custody of it, and can the engine prove it is
> represented exactly once while it exists?

This must support world-unique objects, ordinary spawned loot, fungible
quantities, body-owned inventories, held/equipped items, containers, room
streaming, possession and multiplayer.

## Current pressure

The repository currently has several related but distinct representations:

- `ambition_items::OwnedItems` for catalog ownership/quantities;
- `ambition_combat::HeldItem` on a body;
- actor-side `GroundItem` and `WorldItem` ECS entities;
- save mirroring for Ambition inventory state.

These are useful mechanics but they do not yet form one conservation/accounting
model. In particular, catalog entitlement and physical custody must not be
assumed to mean the same thing.

### ⭐ Measured at HEAD, 2026-08-15 — the two halves exist and do not meet

- **Physical custody landed, for world item entities only.** `ItemCustody` is
  `InWorld | Held { holder }` on the item entity itself
  (`ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs:238`), is
  explicitly **rollback state rather than cache**, and is registered as
  `entity:item_custody` with a paired `rollback_map_entities` because the holder
  handle is remapped on load. ⭐ **and the deletion already happened**: pickup no
  longer despawns the ground entity, so an instance survives being picked up
  instead of being destroyed and re-created.
- **Accounting did not move.** `OwnedItems` — the 24-item catalog count table —
  still carries **~175 references across 7 crates** (`ambition_app` 80, actor
  monolith 38, `ambition_content` 27, `ambition_items` 22, plus runtime,
  inventory UI and menu), and it is what the save mirrors.
- ⇒ **the two halves have almost no overlap**: `ItemCustody` is referenced in
  4 crates and 6 files total. ⛔ **so custody is not yet the accounting model; it
  is a state on world objects.** The unanswered question is the seam between
  them — which items exist as instances, which exist only as catalog counts, and
  whether anything is currently allowed to be both.

⚠ that seam, not the count table's existence, is the next slice. `OwnedItems`
remains a **migration seam**: physical custody belongs to the body and the item
instance; participant entitlement is a separate fact with a different owner and a
different lifetime. ⛔ do not "fix" it by giving the count table a row per object
before measuring which of its ~175 consumers need an instance rather than a
count — most catalog consumers legitimately want a quantity.

### ⭐ THE PARTITION — measured slot by slot, 2026-08-15

The 24 catalog slots are **not** uniform. Instance-capability is decided by one
thing: whether `Item::held_item_id()` resolves, because that is the only key
`Item::from_held_item_id` matches a `GroundItem`'s `HeldItemSpec.id` against.

| class | slots | instance-capable? | who writes the count | readers want |
| --- | --- | --- | --- | --- |
| **Held weapons/abilities** | Axe, Javelin, GunSword, PuppySlugGun, Fireball, Blink, Grapple, MarkRecall, Bomb | ⭐ YES — `GroundItem` + `ItemCustody`, authored by LDtk, dropped by `drops_held_item` deaths and boss `signature_gauntlet` | world pickup · `shop::buy`/`sell` · `ItemGrantRequested` · boss `reward_ability` touch-collect · save restore | BOTH — the menu wants "owned", the throw wants the object |
| **Portal gun** | PortalGun | PARTIAL — a `PortalGunPickup` world body with **no** `ItemCustody`; still destroy-on-pickup / respawn-on-drop | `pickup_portal_gun_system` only | a flag, plus "which world body" |
| **Unwired abilities** | Fly, MorphBall, BubbleShield | NO — no spec, no world body | grant paths + the starter set | a flag |
| **Consumables / currency** | HealthCell, ManaCell, SpareBattery, DataChip, GoldPouch | NO — the `PickupFeature` / `PickupKind::Currency`\|`Health` path credits a NUMBER and marks its dispenser `Collected` | pickup dispensers · shop · grants · save | ⭐ a COUNT, forever |
| **Key/quest items** | MapFragment, SealedNote, FieldSurvey, GateKey, DebugLens | NO — narrative grants and story flags only | `ItemGrantRequested`, `grant_pirate_treasure_reward` | a flag |
| **Reserved** | ReservedSlot | NO | nothing | nothing |

⭐ **So 5 of 6 classes are genuinely fungible-or-flag and should stay counts
forever.** The seam is ONE class wide: the nine held weapons/abilities, which
exist as instances *and* are counted. `WorldItem` (touch-to-equip) is a third
thing that touches neither model — it carries an `EquipmentRow` into
`WornEquipment` and the object genuinely ends — so it is correctly outside both.

### ⛔ Yes, an item is allowed to be BOTH — and it duplicates

`crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs:616` —
`owned.grant(item, 1)` inside `pickup_held_item_system` — is the line where an
INSTANCE also becomes a COUNT. The throw in the same file deliberately does not
give the count back ("the thrower keeps catalog OWNERSHIP"). The consequence is a
loop with no bound:

1. pick up the authored axe → `counts[Axe] == 1`, instance `Held`;
2. throw → `counts[Axe] == 1`, instance `InWorld` on the floor;
3. menu → Equip Axe (the menu only checks `owned.has`) → the hand is filled from
   the COUNT, with no object behind it;
4. throw → `throw_held_item_system`'s mint arm finds no `Held { holder: me }` and
   **materializes a second axe**;
5. go to 3.

⛔ **the obvious fix is a trap.** Spending the count on throw (`owned.take`)
closes the loop and immediately opens a worse one: a dropped `GroundItem` is
room-scoped, so leaving the room destroys it, and the count is the ONLY thing
that survives. The count is not acting as entitlement here — it is acting as the
**durable-save mirror of an instance**, i.e. the third concern, and the mint arm
is its restore path. ⇒ closing this needs the body inventory that does not exist
yet; it is not a one-line correction and must not be attempted as one.

### ⛔⛔ The orphan — FIXED 2026-08-15

`unequip_held` empties the hand and touches nothing else, so the inventory menu's
**Stow** and its **equip-swap** (`game/ambition_app/src/menu/effects.rs` lines 154
and 192) left a picked-up object recording `ItemCustody::Held` by a body with an
empty hand — a third state the enum does not have: skipped by physics, by the
pickup, by the drawn view, and unfindable by the throw. An authored axe carrying
`SimId::placement(..)` silently ceased to exist **through the menu**, which is the
exact loss `ItemCustody` was introduced to prevent at the despawn. It is "retract
by REMOVING" wearing a different hat.

⭐ Fixed by `return_released_items` (first in the `CoreHeldItems` chain): custody
is **re-derived from the hand each tick** and reset to `InWorld` at the holder's
position. A derive rather than a call inside `unequip_held` because every
orphaning caller lives in `Update`, in another crate, holding no item query — what
a caller cannot be given it cannot be trusted to remember.

⚠ **still open:** a holder that DESPAWNS while carrying. That is the death
resolver's question (`caps.drops_held_item` already MINTS a replacement
`GroundItem` from the corpse's spec), and releasing here as well would put two
axes on the floor. The real answer is the death drop handing back the object it
has custody of instead of manufacturing a copy.

### ⭐ Deletion gates — what dies, and exactly when

- **`throw_held_item_system`'s mint arm** (the `SimId::spawned` fallback in
  `pickup/mod.rs`): dies the moment a body inventory can hand back a real object,
  because a hand will always have one behind it.
- **`OwnedItems::equipped`**: an `Option<Item>` on a process-global resource that
  the code itself calls "ONE FACT STORED TWICE" with the body's `HeldItem`. It is
  also structurally wrong for a couch — four seats, one equipped slot. It dies
  when equipped-ness is read off the body; that also deletes the
  `owned: Option<&mut OwnedItems>` parameter threaded through `equip_held_spec`,
  `unequip_held`, `equip_portal_gun` and `unequip_portal_gun`.
- **the portal gun's despawn-on-pickup / respawn-on-drop pair**: dies when
  `PortalGunPickup` becomes a `GroundItem`-shaped instance carrying
  `ItemCustody`, the same conversion the held weapons already had.

## Target distinctions

```text
item definition
    != item instance
    != participant entitlement/unlock
    != body inventory
    != physical custody
    != fungible quantity
```

Possible custody locations include world/room, body inventory, held slot,
equipment slot and container. Consumption/destruction are explicit terminal
transitions rather than unexplained disappearance.

## Core invariants

- one live physical item instance has one authoritative custody/disposition;
- transfers are explicit and atomic at the domain level;
- room unload does not silently delete persistent custody;
- body despawn follows an explicit inventory/drop/transfer policy;
- save/load and rollback reproduce the intended custody state;
- fungible quantities use accounting appropriate to quantities rather than
  manufacturing instance IDs for every coin;
- entitlement/progression can survive throwing a physical manifestation only
  when the item's design explicitly says so.

## Agent-native surface

The eventual inspection/preflight API should support questions such as:

```text
item where <instance>
item list --room <room>
item list --body <body>
item audit
item explain <instance>
```

## Candidate crate / Bevy shape

`ambition_items` is the natural place to evaluate ownership of definition,
quantity and custody vocabulary, but actor/world ECS adapters should not force it
to depend upward on the actor monolith. A reusable accounting core might become a
small Bevy plugin if its semantics prove useful outside Ambition.

## Open design questions — deliberately unresolved

- Is persistent inventory owned by a body, participant, party, or some mixture?
- Which items are entitlements that can be rematerialized versus physical
  instances that can be lost/stolen?
- How should possession transfer inventories, equipment and entitlements?
- How are stacks split/merged while preserving provenance that matters?
- What happens to persistent dropped items in unloaded rooms?
- How should unique-item loss, destruction and recovery be represented?
- What is authoritative in online multiplayer, and what can be predicted?
- Which item events belong in durable history versus transient diagnostics?
