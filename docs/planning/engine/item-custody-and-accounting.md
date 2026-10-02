# Item custody and accounting

**State:** OPEN / NARROW. Physical custody, the instance/count boundary and
persistent occurrence behavior across residency and restore are built and
guarded. One repair is open (atomic durable-horizon write). This page owns A7
(separate item custody and accounting from lifecycle orchestration) in the
[frontier](actor-monolith-work-frontier.md).

## Scope

This page is the **pressed** half of items: a `GroundItem` taken with a
deliberate `Attack` press. That is `ambition_held_items`. The **touched** half
(a `WorldItem` you walk into) is `ambition_world_items`; see
`crates/ambition_world_items/MODULES.md`. Both are "items" and "pickups".
`ItemPickupSet` belongs to the pressed half only.

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

A body may hold or inventory an item without that fact becoming a
participant-wide entitlement. A participant may unlock a capability without a
particular physical manifestation being indestructible.

## Current shape

| Fact | Owner |
| --- | --- |
| `GroundItem`, `ItemCustody`, held specs, pickup/use/throw/physics/residency chain, `ItemPickupSet::CoreHeldItems` | `ambition_held_items` |
| `WorldItem` and the touched pickup | `ambition_world_items` |
| `OwnedItems` (session `Resource`), `Inventory { bag, in_hand }`, `ItemCategory` | `ambition_items` |
| The three-variant `ItemPickupSet` `.chain()`, `restore_custody_to_checkpoint`, `minted_horizon` | kernel (`ambition_platformer2d_actor_monolith/src/items/pickup/`) |
| The held-item registry | one table, `HELD_ITEMS` (`ambition_characters::brain::action_set`), read by `held_item_by_id` and `held_spec_by_id` |
| Durable whereabouts | `AuthoredOccurrences` and `OccurrenceWhereabouts` (`shared_tangle/src/lifecycle/continuity.rs`) |
| The starter roster | one insert, in `AmbitionContentPlugin` (`game/ambition_content/src/plugin.rs`) |

- **The hand is the record.** `item_in_hand` projects a body's `HeldItem` or
  active `PortalGun` to a catalog `Item`. The menu reads the primary player's
  hand. `Inventory` answers count, has and is-equipped over the bag and the
  hand. There is no process-global "equipped" mirror.
- **`GroundItem` is sealed.** It is `#[non_exhaustive]` with two constructors,
  `at_rest` and `released`. This is a component-construction seal. It does not
  give `ambition_held_items` authority over the occurrence: callers such as
  `drop_held_weapon`, the Smash bomb and mine, and the match spawn still mint
  the identity, room scope, provenance and attempt state.
- **Custody durability is a field.** `InCustodyOf` carries a required
  `CustodyDurability { Restored, SessionOnly }`. Item pickup and custody
  restore write `Restored`. Riders, limbs and possessions write `SessionOnly`.
  The durable custody rows read the field.
- **Death drops.** A death drop that becomes an object carries
  `SimId::death_drop(parent, kind)`. Drops that grant a quantity stay
  anonymous, because `OwnedItems` is their durable record.
- **Unloaded rooms.** A `Placed { room, at }` row crosses the durable horizon.
  Construction reinstates the item in the room it lies in and suppresses it
  elsewhere (`outlook_for`). A runtime mint enters the ledger in the tick it
  appears (`AuthoredOccurrences::admit_mints`).
  `republish_placements` refuses an id whose row is not `InCustody` or
  `Placed`, and returns the refusals `#[must_use]`. Position is kept at integer
  pixels.
- **The portal gun is an entitlement.** Pickup grants `OwnedItems` and equips
  the body. A drop spawns a fresh `PortalGunPickup` token and never revokes
  the grant. The other catalog items take the occurrence road. Q45 is ruled:
  entitlement behavior is acceptable during engine development.
- **Seat-keyed entitlement.** A placed mine (`game/ambition_demo_smash/src/mine.rs`)
  keys detonation to a seat (`PlacedMine { owner_seat }`) while the object is
  an ordinary `GroundItem`. An opponent can hold it while it remains the
  placer's to set off.
- **Possession moves the driver, not the goods.** `OwnedItems` is session
  scope and the hand is body scope, so possession transfers nothing.

## Writer census (A7)

```bash
python3 scripts/measure_state_writers.py --domain item --sites
```

Every number is a lower bound. A text scan cannot see a rollback codec, a
`serde` restore, or a helper that takes `&mut T`. To enumerate one family
reliably, seal the type and let `rustc` list the callers.

The finding: the monolith's checkpoint-baseline family has no writer outside its
own crate. `OwnedItems` is written from four crates, and most of its sites are
outside `ambition_items`. The separation A7 needs is between the crate that
defines inventory and the crates that schedule writes to it, not between session
and item. Custody is already where A7 wants it: nearly all writes are in
`ambition_held_items`.

Do not build a generic item-request bus to reduce the minting count to one.
Centralize occurrence minting only where it centralizes a real invariant
(identity, custody, provenance, rollback ownership).

## Invariants

- One live physical occurrence has one authoritative custody/disposition.
- Custody transfers are explicit and atomic at the item/body domain boundary.
- Room unload does not silently delete a persistent occurrence.
- Body despawn follows an explicit drop, transfer or retention policy.
- Pickup may merge into fungible accounting only when identity and provenance
  no longer matter.
- Drop or rematerialization mints or restores an occurrence by item policy, not
  by fabricating an unrelated replacement (entitlement items excepted, Q45).
- Save/load persists only relationships the durable road can reconstruct.
- Rollback reproduces live custody without becoming the durable save format.
- Participant entitlement and physical custody are not inferred from each other
  in the simulation. `MenuAction::Equip` grants custody from the roster; that is
  accepted for the demo inventory under Q45.
- An object the attempt reclaims (`SpawnedThisAttempt`) is not durable. An
  object made durable stops carrying `SpawnedThisAttempt`.

## Open work

| Item | Work | Acceptance |
| --- | --- | --- |
| Atomic durable horizon | `PersistedMintedItem` has its own setter (`set_minted_items`) and writer (`minted_horizon.rs`), separate from `set_durable_horizon`, which takes occurrences and custody together. Fold minted items into `set_durable_horizon` | One write; the guard `no_durable_row_names_an_occurrence_the_save_does_not_hold` becomes a property of the writer, not only a fixture test |
| `durably_held` width | It matches the `ItemCustody` component, so it holds items lying on the ground too. Narrowing it to `Held` is a durability decision | Decide it explicitly; dropping a save row is the dangerous direction |
| Lifetime matrix (before further A7 cuts) | Write the matrix for authored room baseline, runtime mints, inventory, equipment, stock loss, same-room replay, room exit and new-session/durable restore | Each row names its owner and phase |
| Item carve boundary | See below | The proofs below |
| Agent surface | `item where`, `item list --room/--body`, `item audit`, `item explain` | Reports definition, stack identity, occurrence, custody owner, provenance and disposition |

### Item carve boundary

Startup checkpoint restore belongs to session, not to
`ItemPickupSimulationPlugin` (A1b). A checkpoint-only composition resumes
without the item domain
(`a_checkpoint_only_composition_resumes_without_the_item_domain`). Do not move
it back with items, and do not move the room/session slot into
`shared_tangle`. `restore_custody_to_checkpoint` stays a kernel system that
reads the item crate's components: it is checkpoint policy, not item policy.

A later item carve identifies pickup verbs, held-item custody, equipment
capability and minted accounting separately. One generic item service that
receives all world, control and session context would make the crate smaller
without reducing what a change must know.

Prove: item-absent checkpoint restore, two-body interaction, duplicate pickup
suppression, failed construction and consumption, durable occurrence restore,
and rollback. A source guard that forbids an import is supplemental.

## Open design questions

- When do stack merge and split preserve provenance?
- What is the policy for unique-item destruction, recovery and reset?
- What is authoritative or predicted for item custody in online multiplayer?
- "Unique" names two properties. `ItemCategory::is_unique()` is a stacking
  property (all non-consumables clamp at 1). The portal gun's uniqueness is a
  lifecycle property (acquired once, never revoked). Do not classify items in
  terms of the one word.
- Q141 is ruled: durability is per-item and authored, and a runtime-spawned
  item may be durable when authored so.

## Relationship to session and possession

Possession does not make a physical item participant-owned. If a possessed body
carries an item through a room transition, the item travels because its custody
policy says so. A generic live relationship component is not written to the
durable save because another domain uses the same vocabulary. Persist it only
when the durable road can restore the relation.

## Checkpoint and construction boundary

Items own occurrence, custody, entitlement and minted-history semantics. Session
owns the save/replay boundary at which those domains restore or persist. Session
does not copy item fields itself. An owner installer registers known lifetime
work against published phases, without a universal callback service.

Atomic custody transfer means the accepted transition is indivisible to its
consumers. It does not make the raw-Commands construction interface undoable.
Failed construction and consumption have an explicit order and regression. Do
not grant or spend an occurrence because a construction request was made.

## Procedural callers and residency handoff

Modules request acquisition, transfer, cost or custody through the owning
transaction. No extension-owned inventory mirror exists. A submitted request is
not a receipt of acquisition. A paid action uses the owning compound
admission/reservation rule.

[Open-world residency](open-world-runtime-and-residency.md) requires exactly one
writer while a live occurrence becomes dormant or live again. A durable item
reference does not force a body to stay resident. A candidate reconstruction
attempt is not a new persistent identity. FI5/FI9 in
[acceptance](fast-iteration-acceptance.md) exercise refused and accepted
transfer and replay between submission and acknowledgement.

## Forbidden regressions

- No second held-item registry.
  `every_catalog_item_with_a_held_form_resolves_in_the_one_registry` enumerates
  the population from `Item::ALL`.
- No struct-literal `GroundItem` outside `ambition_held_items`.
- No process-global "equipped" mirror.
- A restore that displaces an item keeps the spec it displaced; it does not
  re-derive the spec from an id.
- An authored field that reaches a runtime representation must have a
  consumer, or be deleted. A comment that promises a consumer is worse than an
  unread field.
- A negative claim ("nothing authors this") needs an untruncated scan of every
  spelling (struct literal and assignment).
