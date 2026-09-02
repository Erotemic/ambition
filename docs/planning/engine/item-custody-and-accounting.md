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

⚠ **TWO OF THESE ARE WRITTEN AS UNIVERSALS AND HOLD ONLY FOR OCCURRENCE-MODEL
ITEMS** — noticed while measuring I3, and worth fixing when question 45 is
answered rather than guessed at now:

- *"drop/rematerialization … not by fabricating an unrelated replacement"* — the
  portal gun's drop spawns a fresh `PortalGunPickup` unrelated to the token that
  was consumed. Under the ENTITLEMENT reading nothing is fabricated in place of
  an occurrence, because there is no occurrence; under the OCCURRENCE reading it
  is exactly what the invariant forbids;
- *"participant entitlement and physical custody are not inferred from each
  other"* — ⛔ and this one is looser than it looks for EVERY item, not just the
  gun: `MenuAction::Equip` grants physical custody straight from the roster
  (`equip_portal_gun`, or `held_spec_for_item` → `equip_held_spec`), which is
  inferring custody from entitlement unless the invariant is read as being about
  the SIMULATION only.

⇒ Neither is a defect to fix today; both are the same under-specification, and
answering question 45 is what makes them precise.

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

⭐⭐ **MEASURED 2026-09-02, and the row's own escape clause — *"when that model
can express their semantics"* — is the whole answer. It cannot, because the
portal gun is not an occurrence.** Read side by side:

| | ordinary held item | portal gun |
|---|---|---|
| in the world | `GroundItem` with a `SimId` | `PortalGunPickup`, no identity |
| pickup | `HeldItem` + `ItemCustody::Held { holder }`; the object PERSISTS | pickup entity despawned; `PortalGun { active }` + `StashedActionSet` on the body; `owned.grant(Item::PortalGun, 1)` |
| drop | the same object returns to the ground | `unequip_portal_gun`, then `spawn_room_scoped(PortalGunPickup { … })` — a FRESH token |
| durable record | custody + the whereabouts ledger, rebuilt by `restore_custody_to_checkpoint` | `OwnedItems`, granted on pickup and **never revoked on drop** |

⇒ **The gun is an ENTITLEMENT with a cosmetic world token.** "Dropping" it is
unequip-plus-spawn-a-re-pickup, and it never loses you the gun: the menu
re-equips straight from `OwnedItems`
(`menu/effects.rs` → `equip_portal_gun`, no check that a token exists), and the
dropped token is room-scoped so leaving the room destroys it with no
consequence. The code says this out loud where it is decided — *"The gun is a
single item: it doesn't exist until you pick it up — picking up the one world
item IS getting the portal gun."*

✔ **AND THE ACCOUNTING IS SAFE, checked rather than assumed:**
`OwnedItems::grant` clamps a `is_unique()` category to 1, so the two roads
cannot inflate a count. What they can do is coexist — after a drop there are two
independent ways to hold the gun again (the menu, and the token), which is
harmless today precisely BECAUSE the gun is an entitlement.

⛔ **SO I3 IS A DECISION, NOT AN IMPLEMENTATION.** Converging the gun onto the
occurrence/custody model would give it an identity its semantics never use, and
would make "drop" mean something it does not mean for this item. The question —
is a unique capability item an ENTITLEMENT or an OCCURRENCE? — is recorded in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).
⚠ Whoever answers it should note that the two readings differ observably in one
place only: whether dropping the gun and walking away can ever lose it.

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
