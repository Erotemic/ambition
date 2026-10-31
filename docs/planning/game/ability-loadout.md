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
- The demo bag (`OwnedItems::starter`) grants every real item
  (`ItemCatalog::is_real`: not a placeholder). A load keeps the unique
  equipment of the session's starting bag that the save lacks
  (`OwnedItems::keep_unlocks`), so an old save does not lock it again.
- A locked cell still carries the cursor: it has an action that
  `effects::decide` answers `NotOwned`. The library draws it greyed out
  (`MenuNode::Control::dimmed`), and both renderers draw the focus colour
  and the selection corners on it when the cursor is there. Witnesses:
  `a_greyed_out_cell_draws_the_cursor_when_it_has_it` (flat) and
  `a_greyed_out_cell_on_the_cube_draws_the_cursor_when_it_has_it` (cube).

## The grid (`game/ambition_content/assets/data/items.ron`)

| Row | Cells |
|---|---|
| 1 | Fireball, Eye Laser, Lightning°, Sword Stance, Axe Stance°, Whip Stance° |
| 2 | Precision Blink, Contextual Blink°, Dash, Dash II°, Grapple, Mark / Recall |
| 3 | Portal Gun, Javelin, Gun-Sword, Puppy-Slug Gun, Bomb, (empty) |
| 4 | Health Cell, Mana Cell, (Spare Battery\*), (Sealed Note\*), (Field Survey\*), (empty) |

° = a placeholder (no row yet). \* = a key item: its cell is blank (see
below). The Axe and the Map Fragment were removed on 2026-10-30 (Jon: "get
rid of the javelin, axe, map fragment, sealed note"). The Javelin and the
Sealed Note stay until Jon decides: the save tests use the Javelin (the
only item that a throw spends), and the intro quest uses the note. Exclusive groups: `projectile` (Fireball,
Eye Laser, Lightning), `stance` (Sword, Axe, Whip), `blink` (Precision,
Contextual), `burst` (Dash, Dash II). Grapple and Mark / Recall are held
gauntlets.

## Equipment, and the QUEST page (built, 2026-10-30)

Jon, 2026-10-30: items "that you just have, that you can't equip or do not
change your abilities in some way should not be in this 24 cell inventory
page, they can be stored in the quest page". The menu page that was called
"Inventory" is the EQUIPMENT page.

- The tab says "Equipment" and the page title says "EQUIPMENT"
  (`host_text.ron`). The page id stays `MenuPage::Items`.
- Equipment is an ability, a held item or a consumable
  (`model::is_equipment`). A consumable is used from the grid (a health
  cell heals).
- A key item is not equipment. Its cell is blank: no name, picture, count
  or panel text. The cursor still passes it. The Spare Battery is a key
  item, because it does nothing when used.
- The QUEST page (`model::build_quest_page`) lists each key item that the
  bag holds, with its count and description, under "CARRIED".
- Key items are not in the demo bag: the quests give them.
- Witness: `a_carried_key_item_is_on_the_quest_page_not_the_equipment_grid`.

## Eye Laser (built)

`game/ambition_content_modules/src/eye_laser.rs`. While the Projectile slot
is held: one wide, straight `laser` ray from the whole visor along the
facing, stopped by the first wall; mana for each tick it burns; a new ray generation every
`REHIT_S`, so a body it touches is hit again. While it burns, the body is
drawn in its `eye_laser` row (its `shoot` row on a sheet without one).
Witness: `the_eye_laser_takes_the_projectile_slot` (`app_it`).

## Stocked bombs (built, 2026-10-29)

Jon, 2026-10-29: a bomb is an item in the inventory that you have a stock
of. You equip it as a held item. Each throw takes one from the stock. At
zero the hand is empty and the normal attack comes back. A thrown bomb must
feel like a bomb: weight, a bounce, and a blast on a hard contact.

- **Stacks are a row fact.** `ItemMeta::stack` (default 1) is the most of an
  item that the bag holds; the category does not decide it. Bomb: 9. The
  consumables: 99. `OwnedItems::grant` stops at the stack, and the shop
  refuses a full stack. The `item_catalog` schema refuses `stack: 0`. The
  menu shows the count of a stack (`InventoryItemNode::count`).
- **The demo bag** (`OwnedItems::starter`) holds `STARTER_BOMBS` (5).
- **A stock re-arms the hand.** A bomb equipped from the bag is a quantity
  of the bag in the hand. `throw_held_item_system` mints the object, takes
  one from the bag, and, when the bag holds more, puts the next one in the
  hand. A drop empties the hand. A bomb picked up off the floor is an
  object: its throw takes nothing and re-arms nothing.
- **Weight and bounce.** `ItemBody` (gravity scale, bounce, slide, rest
  speed) is how a loose item moves; an item without it stops dead, as
  before. `weigh_bombs` gives each bomb `BOMB_BODY` (1.5× gravity, bounce
  0.4, slide 0.7, rest at 60 px/s) before its first step. Each bounce
  publishes `ItemStruckSurface` (the speed into the surface).
- **Detonation.** On the first of: the fuse (1.2 s; the first bounce cuts
  what is left to 0.5 s); a strike on a surface at 650 px/s or more; a
  strike on a body at 300 px/s or more. A tilt throw lands under the surface
  bar and bounces; a smash throw into a wall or the floor goes off there.
  The blast is a large burst, smoke, a ring, `WORLD_EXPLOSION` and a 9 px
  camera shake. A hard bounce makes a sound.
- **Fuse flash.** `GroundItemFact::fuse_left_s` carries the fuse to the
  view. The sprite flashes red and swells, from 3 to 14 flashes a second
  as the fuse burns down (`fuse_flash`).
- The Smash item bombs use the same `bomb` item, so they have the same
  weight, bounce and detonation.

Witnesses: `bombs_are_a_stock_in_the_bag` (`app_it`: the stock, the re-arm,
the empty hand and the normal attack; a hard throw goes off on contact and a
soft one on its fuse), the `ranged::bomb` and `ambition_held_items` unit
tests (bounce, one-way landing, strike rules).

Open for bombs:

- **Pocket a bomb.** A bomb picked up off the floor cannot go into the
  bag. To pocket it (+1 to the bag, the object ends) is the reverse of the
  throw's mint: it needs a record of bag grants since the checkpoint, as
  `BagSpendsSinceCheckpoint` records spends, or a checkpoint restore loses
  the bomb. Not built.
- **The menu equips a held item from the host.** Abilities equip through
  `ItemEquipRequested` on a sim tick; a held item (the bomb too) is still
  put in the hand by `menu::effects::apply_menu_action`, outside the tick.
  Move held equips and stows onto the sim road.

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

## Where this stands (2026-10-30, before a reboot)

Branch `inventory-unlocks-cursor` (commit c0c091931) is NOT on main yet.

- Done on it: every real item unlocked in the demo bag; a load keeps the
  unlocks (`OwnedItems::keep_unlocks`); the cursor shows on a locked cell;
  each cell states each fact once. Unit, menu and bag app tests pass.
- To land it: merge `origin/main` (27 commits ahead). The merge stopped on a
  submodule conflict in `game/ambition_map_assets`: take main's pin (the
  branch does not change that submodule), then run `cargo check -p
  ambition_app --tests --features rl_sim` and `cargo test -p
  ambition_workspace_policy`. The policy test `engine_policies` failed
  on the half-merged tree; check it again on the clean merge, and on
  `origin/main` alone, before deciding whose it is.
- Reds on main that are NOT from this work (they fail the same on
  d91b5c2b6): 5 tests in `death_restores_the_checkpoint` and 2 in
  `two_persistence_authorities_for_one_item`, all "pressed Attack ... never
  picked it up" / "the throw mints exactly one instance".
- Next task (Jon, 2026-10-30): REMOVE the unused items Javelin, Axe, Map
  Fragment and Sealed Note from the inventory (enum, `items.ron`, held-item
  specs, placements, tests; the Javelin is the minted item of several
  checkpoint fixtures and needs a replacement there). Jon also said he does
  not know what most items are: list each remaining item and what uses it,
  so he can choose more to remove.
