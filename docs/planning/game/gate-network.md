# The gate network

**State:** OPEN. The first slice is built and tested (see "Current slice").
Rules for the engine: `ambition_platformer2d_world::rooms::gate_network`.
Ambition's catalog: `game/ambition_content/assets/data/gate_network.ron`.

## What a gate is

A gate is a **ring**: a loading zone that a live **link** opens. A traveller
uses the ring's **terminal** (a `Switch` with action `GateTerminal`), dials an
**address**, and walks into the ring when it is open. The ordinary room
transition (`detect_room_transition_system`, its passage, the confirmed
lifecycle commit, readiness and rollback) takes the traveller to the far
ring's zone. A dial never moves a body.

## The rules

1. **One catalog.** Addresses, facilities and rings are authored in one
   typed file (schema `gate_network`, world capability). The pack refuses a
   catalog with a duplicate address or ring, an address with a symbol that is
   not on the chevrons, an unknown facility, band or address in a reach or a
   posting, or an empty reason (`gate_network_problems`).
   `gate_network_ring_zones` (LDtk validator) and the content tests hold each
   ring to the world files.
2. **The alphabet** is the ring's inscription, ΛΕΓΑΛΛΥ ΔΙΣΤΙΝΧΤ, as the
   letters `A C D E G I L N S T Y`. A symbol can repeat. Matching is exact
   after upper-casing and removing spaces.
3. **An address reaches one ring.** A ring with no address only sends (Ring
   7). A **hidden** address is in no directory, and a dial reaches it the same
   way.
4. **Any valid address can always be dialed.** Knowledge never changes what a
   dial does. It changes only what a terminal lists.
5. **The dial decision** (`resolve_dial`) checks, in this order: the address
   is not empty and uses the alphabet; the dialing ring works; a ring answers;
   that ring is not in the dialing ring's facility; the dialing ring can reach
   it (`reach`); the far ring admits the dialing ring's band (`admits`); the
   far ring works and its room is in the world; the far ring is not in another
   link. Each refusal has its own kind and message: `empty`, `illegal`,
   `unknown`, `same_facility`, `restricted`, `offline`, `busy`.
6. **Same facility, same dimension.** Two rings in one facility never dial
   each other: that is a walk. Two facilities in one dimension can: the
   Science dimension's gravity trail (`ASCENT`) and its orbital station
   (`GALACTIC`) are far apart, and each dials the other.
7. **Status is deterministic.** A ring is `Online`, `Offline(reason)`,
   `PoweredBy { switch }` (offline until the save switch is on) or
   `BusyWhile { flag }`. Live links make a ring busy for another link. No
   status reads chance.
8. **A link** holds both rings open: the sender's portal boots (the existing
   opening animation) and the receiver shows the incoming wormhole. Only the
   sender leads anywhere: a receiving ring sends nobody back. A link closes
   1.5 s after its traveller goes in, or 30 s after the dial when nobody does.
   A hangup shuts it; a redial replaces it.
9. **Every terminal away from home posts a way home** (`posted`, here
   `LEGAL`). Its Known view always lists it, so no destination needs a known
   address to leave.
10. **Bands and permits.** A ring sends in one band (`civil` by default). A
    ring that `admits` only `military` refuses a civil ring as restricted,
    unless the save flag of its `permit` is set. Reaching a military ring early
    lets a traveller use that ring; it sets no permit, so the public gate
    still refuses the military rings.
11. **Knowledge** is the save flag `gate.known.<ADDRESS>`. A dial that answers
    sets it; a sign, a person or a document sets it with Yarn
    `<<command "world.set_flag" "gate.known.TANGENT" true>>`. The catalog's
    `start_known` decides what a traveller knows before learning anything:
    `All` (every address, hidden ones too) or `Discovered` (only what was
    learned). **Development uses `All`** (Jon, 2026-10-24: development starts
    with everything unlocked). To release with discovery, write
    `start_known: Discovered` in `gate_network.ron`; nothing else changes.

## State and ownership

| what | where | kind |
| --- | --- | --- |
| the catalog | `GateNetworkCatalog` resource, from the pack | authored; reload replaces it |
| open terminal, links, reports, recent addresses | `GateNetworkState` (`resource.gate_network_state`) | rollback state, keyed by ring zone ids |
| a terminal's dial, hangup, close | `GateTerminalIntent` host intent | stamped tick, released into the sim |
| what the player knows | save flags `gate.known.*` | persisted |
| the panel's tab, entered symbols, cursor | `TerminalUi` | host presentation |
| a ring's portal phase | `GatePortalPhases`, commanded by `GatePortalCommand::Dialed` | rollback state |

The simulation step is `run_gate_network` (Progression, `WorldSync`, after
the switch drain, before the portal tick). The terminal panel
(`presentation::gate_terminal`) reads the shared menu input (keyboard,
gamepad, touch buttons), the alphabet's letter keys and presses on its own
buttons, and claims the seat's input (`gate_terminal`, between dialogue and
inventory) while a terminal is open. The pause menu outranks it.

## The address table

New game = a fresh save; with `start_known: All` every address is listed
from the start.

| address | ring (zone) | arrival room | local routes from the arrival | reachable from a new game |
| --- | --- | --- | --- | --- |
| `LEGAL` | `hub_public_gate_zone` | `central_hub_complex` | the hub's local doors, the basement, the developer wing | yes (home) |
| `TANGENT` | `math_gate_return_zone` | `math_gate_hall` | up to `math_town_street`, its cafe, surveyors' office, observatory | yes |
| `GENESIS` | `primeval_trailhead_gate` | `primeval_trailhead` | the trail east to the tyrant's hall | yes |
| `CITADEL` | `evil_gate_return_zone` | `evil_gate_forest` | the road to Castle Godwin's gatehouse | yes |
| `ASCENT` | `orchard_gate` | `gnu_ton_orchard` | the gravity trail toward GNU-ton, the spoke to the station | yes |
| `GALACTIC` | `orbital_station_gate_arrival` | `orbital_station_gate_dock` | the station's decks | yes |
| `SAILING` | `pirate_port_gate_arrival` | `pirate_port_gate_plaza` | the port's docks and the cove | yes |
| `SILENT` | `ninja_village_gate_arrival` | `ninja_village_gate_shrine` | the village, the forest, the dojo | yes |
| `ALDENTE` | `noodly_narthex_gate` | `noodly_narthex` | the Noodly Temple | yes |
| `SENTINEL` | `military_gate_zone` | `military_gate_annex` | back through the military tower | restricted (military band) |
| `NEST` (hidden) | `primeval_rookery_gate` | `primeval_rookery` | down the trail to the trailhead ring | yes, receive only |
| `TESLA` (hidden) | `trail_survey_gate` | `gnu_ton_inversion_gallery` | the trail's end, GNU-ton's door | military band only |
| (none) | `math_gate_home_zone` | `gate_stack_lower` (Ring 7) | the gate stack | sends `TANGENT` only, when its breaker is on |

## The hub

The central hub's main floor has one public gate (ring, dialer, power
cabinet, traffic board, bags for organic travellers) east of the start. The
door to the gate stack stands on the middle platform by the start: the
intro's gate comes out of it. The doors to test rooms and the old doors to
other dimensions are in the **developer wing** (`central_hub_dev_wing`), a
level of the same room east of the main floor, through the old east exit:
each door keeps its id and target, so tests and far doors still find it.
`specs/gate_network/make_gates.py` makes all of this.

## Current slice

Built and tested (`game/ambition_app/tests/the_gate_network.rs`, the
world crate's and the content crate's unit tests):

- the catalog, its validation, the dial decision, links and statuses;
- dialing in the simulation through host intents, the crossing through the
  shipped room transition, no crossing while the ring is still opening, one
  transition per crossing, the link closing after its traveller;
- knowledge through save flags, its save/load round trip, the
  `start_known` switch;
- the terminal panel (Known, Recent, Manual; keyboard, gamepad, touch);
- the public hub gate, the developer wing, Ring 7 and its breaker, the
  Math town and Castle Godwin forest rings.

## Future expansion

- Several public gates in one hub (the rules hold already: one facility).
- Chevron art in the panel from the ring's sheet; the panel draws the Greek
  glyphs as text now.
- More bands (freight), tolls, schedules of outages from authored time.
- Rings in the other 8 of the 17 dimensions.
