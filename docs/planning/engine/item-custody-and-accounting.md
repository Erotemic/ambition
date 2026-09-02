# Item custody and accounting

**State:** OPEN / NARROW — physical custody is established; remaining work is the
instance/count boundary for held weapons/abilities and persistent occurrence
behavior across residency/restore.

## Goal

Keep these concepts separate:

```text
item definition
    != physical item occurrence
    != fungible quantity
    != body inventory
    != participant entitlement/unlock
    != current custody/equipment
    != durable occurrence disposition
```

A body may physically hold or inventory an item without that fact becoming a
participant-wide entitlement. A participant may know/unlock a capability without
a particular physical manifestation being indestructible.

## Settled ownership

The body owns physical inventory/equipment/capabilities that describe that
body. Participant-owned knowledge/keys/theorems and possession-transfer policy
are separate authorities with different lifetimes.

Do not reopen `OwnedItems` as an undecided global authority. It is migration
pressure where one process-global representation duplicates body-local held or
equipped state and cannot represent several independently driven bodies.

## Current item partition

Most catalog item classes are naturally fungible counts. The difficult residual
is the held weapon/ability population that currently participates in both worlds:

- there is a physical instance with provenance/custody that can be dropped,
  carried or moved between rooms;
- there is also durable accounting/availability represented as a count or
  entitlement.

That dual role is legal. The architecture should make the transition explicit
rather than pretending every item is purely a stack or purely a unique ECS
entity.

## Current invariants

- one live physical occurrence has one authoritative custody/disposition;
- custody transfers are explicit and atomic at the item/body domain boundary;
- room unload does not silently delete a persistent occurrence;
- body despawn follows an explicit drop/transfer/retention policy;
- pickup may merge into fungible accounting only when item identity/provenance no
  longer matters;
- drop/rematerialization mints or restores an occurrence according to item
  policy, not by fabricating an unrelated replacement;
- save/load persists only relationships the durable road can reconstruct;
- rollback reproduces live custody state without becoming the durable save
  format;
- participant entitlement and physical custody are not inferred from each other.

## Remaining migration pressure

### I1 — body inventory replaces process-global equipped mirrors — ✔ CLOSED 2026-09-02

`OwnedItems::equipped` is gone. It was a process-global mirror of "some body
holds X", written by every equip road (`equip_held_spec`, the portal-gun twins,
the checkpoint restore) and read by the menu — and four seats could not share
it: seat two picking up a gun-sword marked it equipped in seat one's menu. Now
the hand IS the record: `items::pickup::item_in_hand(held, portal_gun)`
projects a body's `HeldItem` / active `PortalGun` to the catalog `Item`, the
menu reads the PRIMARY player's through `menu::effects::PrimaryHand`, and
`ambition_items::Inventory { bag, in_hand }` is the one view that answers
count / has / is_equipped over both — `OwnedItems::count` is the bag alone.
`inventory.holds` (the authored condition) asks the bag and then any player or
driven body's hand. Guards: `another_seats_weapon_is_not_the_primary_players_
equipped_item`, `a_wielded_weapon_with_no_stored_copy_is_owned_and_stowable`,
`a_weapon_in_the_players_hand_is_held_with_nothing_in_the_bag`; the two
persistence-authority tests read the grid's count through the projection. No
schema bump: the clone snapshot lost a field the session checksum never saw.

### I2 — held weapon/ability occurrence continuity

The nine held weapon/ability cases are the real customer. A pickup, room
transition, drop, save/load and replay should preserve whatever occurrence
identity/provenance the item's policy says matters while still supporting durable
quantity/entitlement accounting.

Use canonical reconstitution rather than a transition-specific rematerialization
hack.

### I3 — portal-gun/special pickup convergence

Special pickup roads that despawn on pickup and manufacture a replacement on
drop should converge toward the same occurrence/custody model as ordinary held
items when that model can express their semantics.

### I4 — unloaded-room disposition

Persistent dropped items and carried items in nonresident rooms need a durable
location/disposition that construction can reconstitute. This is jointly owned
with [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)
and [`construction-and-reconstitution.md`](construction-and-reconstitution.md).

## Relationship to session and possession

Possession/control does not itself make a physical item participant-owned. If a
possessed body carries an item through a room transition, the item travels
because its holder/custody/lifetime policy says so.

A generic live relationship component must not be written to durable save merely
because another domain adopted the same relationship vocabulary. Persist it only
when the durable road can restore the relation.

## Agent-native surface

Useful inspection eventually includes:

```text
item where <occurrence>
item list --room <room>
item list --body <body>
item audit
item explain <occurrence>
```

The inspector should report definition, quantity/stack identity, physical
occurrence, custody owner, provenance and durable disposition without requiring a
reader to infer them from unrelated components.

## Open design questions — deliberately unresolved

- Which item classes are rematerializable entitlements versus lossable physical
  occurrences?
- When do stack merge/split operations preserve provenance?
- What is the policy for unique-item destruction, recovery and reset?
- What happens to a persistent dropped item in an unloaded room?
- How should possession transfer body inventory, equipment and participant
  entitlements?
- What is authoritative/predicted for item custody in online multiplayer?
