# The Pirate Port (gate address `SAILING`)

A seafaring town of the pirate dimension. The world file is
`pirate_port.ldtk`. The gate arrives in the gate plaza, at the zone
`pirate_port_gate_arrival`.

`make_specs.py` writes each room's `.ron` spec and, with `--apply`, makes the
rooms, the pirate cove's door to the docks (`sandbox.ldtk`) and the ladders.
`props.py` gives each room its scenery. Change the scripts, not the specs.
Run them from `tools/ambition_ldtk_tools`.

## The rooms

| room | what it is | who lives there |
| --- | --- | --- |
| `pirate_port_gate_plaza` | the gate plaza; the ring's freight booth; a stair up to the harbour balcony | Ines (dock clerk), Tomas (gate-sick traveller) |
| `pirate_port_waterfront` | the market street; the galleries and roofs of the houses over it | Agnes (chandler, sells) |
| `pirate_port_tavern` | The Drowned Compass | Wren (keeper), Broadside Bess of the cove |
| `pirate_port_docks` | the pier, the brig Persistence, the warehouse and the gantry; the door to the pirate cove | Jory (fisherman), Bram (harbour master) |

## The roads

- The low road: plaza floor, market street, pier. Two gaps of open water in
  the pier have a ladder out of each.
- The high road: the brig's yards to her crow's nest (each step 112 px, a
  jump and a double jump), the rope to the warehouse roof, the gantry, the
  roofs and galleries of the waterfront, the harbour balcony, the plaza's
  stair. Each room joins the next at both heights, so the two roads are one
  loop.
- The way on: the docks' east door goes to `pirate_cove` (two ways). The
  cove's sky hatch goes to `pirate_sky_lookout`, the raiders' skyship lookout.

## The problem

The wind took page eleven of Ines's freight log to the crow's nest of the
brig. The page is a `PickupSpawn` (`flag:pirate_port_manifest_found`). Ines
takes it back in her talk (`pirate_port_manifest_returned`). The quest is
`pirate_port_lost_manifest` (`quests.ron`); the talk is
`dialogue/sandbox/pirate_port.yarn`.

## The gate

Agent GATE places the ring and its terminal on the plaza's clear floor
(x 160..512, floor y 704, clear to 224 over the floor). Put those entities in
`gate_interface.yaml` here (an `entity add` spec): `--apply` adds that file
again after it makes the rooms. Until the ring stands, the arrival zone is a
one-way door to the hub.

## Proof

`game/ambition_app/tests/the_pirate_port.rs` walks the loop by keys from the
plaza, talks to each resident, climbs to the page and gives it back, and goes
through the door to the cove and back.
