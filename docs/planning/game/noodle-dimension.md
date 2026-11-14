# The Noodle dimension (started 2026-11-13)

Jon, 2026-11-13: "Expand the noodle dimension. It should be built around the
noodle temple, but we need to build a noodle dimension around it. There
needs to be a noodle village, and then paths out of the village with one
leading to the temple, and another leading to the volcano (with a fizzy
carbonated liquid instead of lava)."

## The map

The flat map is a valley. The temple is on the hill west of the village. The
volcano is on the east side. Brothlehem, the village, is in the floor of the
valley, and it is the hub of the dimension.

```
                     [temple stair]--door--> noodly_narthex (the Noodly Temple)
    [durum terraces]-'  (high)
           |  (low)
    [village square]==[village market]--[fizzy flats]--[Fizzuvius slopes]==[Fizzuvius caldera]
                         | door                          (the rim and the soda tube)
                     [curry house]
```

`==` is a join at two heights, so each of the two regions has a loop. The
temple and the curry house are buildings, so their ways in are doors. The
central hub's door to the narthex stays.

| Room | Where and what | Who |
| --- | --- | --- |
| `noodle_village_square` | Brothlehem's square: the gate ring (ALDENTE), its terminal, a shrine. The roofs of the houses step up east to the market. | Richard Duckling counts the pilgrims (a meme is a thing you catch). |
| `noodle_village_market` | The drying racks. Their rails are the high road. | |
| `noodle_village_curry_house` | Curry udon. | Marie Curry. She tells what the soda is. |
| `noodle_durum_terraces` | Durum wheat (the wheat of pasta) on terraces, with paddies of broth (clear water). The low field climbs west, and the high field climbs back east. | noodlings |
| `noodle_temple_stair` | The great stair to the temple's door. | a parrot |
| `noodle_fizzy_flats` | Soda springs in a crust of sugar at the foot of the volcano. | The Weird Hermit watches the springs, and he has no mints. |
| `noodle_fizzuvius_slopes` | Shelves of rock candy, with channels of soda between them. | spaghetti events, a parrot |
| `noodle_fizzuvius_caldera` | A lake of soda in the crater. Corks drift on it. A bubble lift goes up to the east rim, and the crust goes back west over the lake. The Mint is on the east rim (a chest). | noodlings, a parrot |

The rooms are written by
`game/ambition_map_assets/ambition_content/generators/noodle_village/make_specs.py`.
Change the script and run it with `--apply`. The rooms are recorded in
`noodle.generated.json`, so a room that is edited in LDtk is not overwritten.
A second `--apply` with no change leaves the world byte-identical. The talk is
`dialogue/sandbox/noodle.yarn`. The area cards are "Brothlehem", "The
Pilgrims' Way" and "Mount Fizzuvius".

## The gate ring

ALDENTE now arrives in the village square (`noodle_village_gate`), not in the
narthex: the village is the hub, as a gate shrine or a plaza is in the other
dimensions. The narthex lost its ring and got a front door to the stair
(`noodly_narthex_to_stair`). The narthex is LDtk-owned, so this edit was made
in the map; the temple's `make_specs.py` writes the same door.

## The soda

A pool of soda is two things in one box. Neither needs a Rust change:

- Murky water (IntGrid `Water`, value 2). A body sinks into it slowly, and
  the water is drawn over the body and over what is under its surface.
- A `DamageVolume` (one damage, a knockback out of the pool). The carbonic
  acid stings. Each name has "acid" in it, so a hit plays the acid splash
  sound (`hazard_sfx_id`). Smoking Mountain's lava is the same
  `DamageVolume`.

## Open

- The soda has no art of its own. It takes the murky water art of the
  `noodly` theme, and the spike sprite of each `DamageVolume` shows through
  it. A `look` on `DamageVolume` (as `LockWall` and `BreakablePlatform`
  have) or a soda water kind would fix this; both need Rust.
- The rooms have no props: no houses, racks, mill or volcano. They have only
  the `noodly` scene and terrain. The scenery needs prop targets in the
  sprite renderer.
- No app test walks the new rooms yet.
- The Mint is a chest. It opens nothing.
