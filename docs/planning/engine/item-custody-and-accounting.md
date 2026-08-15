# Item custody and accounting — Engine 1.0 program

**State:** OPEN — the need for explicit custody is settled; inventory/entitlement semantics require further design.

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
