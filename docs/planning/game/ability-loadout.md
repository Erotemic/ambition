# The ability loadout: equipped variants in the inventory grid (started 2026-10-29)

Jon, 2026-10-29: keep the 24-cell grid, but some cells are mutually
exclusive ability variants that the player equips. To equip one changes
which ability is in that major movement or action slot. This "equip" is
different from a held item, which can also come from the inventory. In the
demo, every ability is unlocked.

## The model (built)

- An item cell is `Held` (an object in the hand: `held_item_id`) or
  `Ability` (a variant the body wears: `wears`, an `EquipmentRow`). The two
  are different facts, and the item schema refuses a row that is both.
- Rows that name the same `exclusive_slot` are mutually exclusive.
  `WornEquipment::equip` replaces the row that holds the slot.
- A row that changes nothing (`EquipmentRow::changes_nothing`) is the body's
  own action on its slot. It is in effect while no other row holds the slot
  (`ambition_items::ability_in_effect`). So the robot's fireball, sword,
  dash and blink need no starting loadout.
- An ability with no row is a placeholder. Nothing equips it, and the demo
  does not unlock it, so the grid shows it as not found.
- The menu asks; the simulation equips. `ItemEquipRequested` is a host
  intent, applied by `apply_item_equips` on its tick (rollback-safe).
- A variant that replaces a slot puts a technique on it
  (`EquipmentGrant::Technique`). A module binds the technique through the
  `ambition.body.technique` trigger (`TechniqueUsePort`).
- The demo bag (`OwnedItems::starter`) grants every ability that has a row.

## The grid (`game/ambition_content/assets/data/items.ron`)

| Row | Cells |
|---|---|
| 1 | Fireball, Eye Laser, Lightning°, Sword Stance, Axe Stance°, Whip Stance° |
| 2 | Precision Blink, Contextual Blink°, Dash, Dash II°, Grapple, Mark / Recall |
| 3 | Portal Gun, Axe, Javelin, Gun-Sword, Puppy-Slug Gun, Bomb |
| 4 | Health Cell, Mana Cell, Spare Battery, Map Fragment, Sealed Note, Field Survey |

° = a placeholder (no row yet). Exclusive groups: `projectile` (Fireball,
Eye Laser, Lightning), `stance` (Sword, Axe, Whip), `blink` (Precision,
Contextual), `burst` (Dash, Dash II). Grapple and Mark / Recall are held
gauntlets.

## Eye Laser (built)

`game/ambition_content_modules/src/eye_laser.rs`. While the Projectile slot
is held: one wide, straight `laser` ray from the whole visor along the
facing, stopped by the first wall; mana for each tick it burns; a new ray generation every
`REHIT_S`, so a body it touches is hit again. While it burns, the body is
drawn in its `eye_laser` row (its `shoot` row on a sheet without one).
Witness: `the_eye_laser_takes_the_projectile_slot` (`app_it`).

## Open

1. **The robot's `eye_laser` row** is authored (sprite renderer `eefd4b0`):
   a braced pose with the whole visor lit red (Jon: two red eyes looked
   "a little bit too evil"). The game draws it once robot v3 is
   published again; until then the body falls back to `shoot`.
2. **The visor point and height are authored numbers** (`eye_laser::VISOR`,
   `VISOR_HEIGHT`, measured from that row). Publish a `visor` landmark with the sheet, as the hand
   landmark places the fireball, and read it.
3. **Contextual Blink** (Jon's notes): level 1 blinks behind an enemy that
   is near; level 2 from further; level 3 adds a double tap that slows time
   to choose the enemy. It needs the Blink slot to route a technique:
   `resolve_control_slots` refuses a technique on `Blink` today.
4. **Precision Blink tiers** (Jon's notes): level 1 directional instant;
   level 2 adds a controlled blink; level 3 adds a time-slow control. Also:
   a quick press does "the intuitive thing", and two quick presses give the
   directional blink.
5. **Tiers.** A variant with levels needs an upgrade fact. None exists.
   Prefer a level on the owned item (the bag counts it) over a second
   authority.
6. **Placeholders:** Lightning, Axe Stance, Whip Stance, Dash II. A stance
   variant replaces the Attack slot's moveset; that needs a row grant that
   swaps a moveset, which `EquipmentGrant` does not have.
7. A laser sound of its own (it uses `boss.wolfenstein.zap` now).
