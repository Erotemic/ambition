# The gate network

**State:** built core. The interdimensional gate system and authored destinations are built and
tested; only the future expansions below remain in planning.
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
   catalog with a duplicate address or ring, an address with a letter on no
   chevron, an unknown facility or posted address, or an empty reason
   (`gate_network_problems`).
   `gate_network_ring_zones` (LDtk validator) and the content tests hold each
   ring to the world files.
2. **The inscription** is the ring's fifteen chevrons, ΛΕΓΑΛΛΥ ΔΙΣΤΙΝΧΤ,
   written as the letters `LEGALLYDISTINCT`. Each chevron is its own glyph
   (a repeated Λ, Ι or Τ wears a small ring). An address is a sequence of
   chevrons (a chevron may repeat). The catalog names it by a word, which
   spells into chevrons by the inscription's rule: the k-th use of a letter
   takes the k-th chevron of it, round again past the last
   (`GateNetworkCatalog::spell`; LEGAL is Λ Ε Γ Α Λ₂). A dial must enter
   exactly those chevrons (Jon, 2026-10-24): Λ Ε Γ Α Λ is no address.
   **An address has at most 16 chevrons, each used at most 4 times**
   (`MAX_ADDRESS_LEN`, `MAX_CHEVRON_USES`), so a ring can show it whole:
   each plaque has a 4x4 position grid, row k the chevron's k-th use, its
   columns that use's place in 4 bits, most significant first
   (`position_grid`). The lit ring spells the address, its order too. The
   pack refuses a longer address; a dial of one is refused `illegal`; the
   terminal enters no fifth use and no seventeenth chevron.
3. **An address reaches one ring, and every ring has one and a terminal**
   (Jon, 2026-10-24: "every gate placed needs an address and a gate interface
   so it can always be used"). An important ring's address is a word; any
   other ring's is random glyphs (`glyphs`), with its name as its label. A
   **hidden** address is in no directory, and a dial reaches it the same way.
4. **Any valid address can always be dialed.** Knowledge never changes what a
   dial does. It changes only what a terminal lists.
5. **The dial decision** (`resolve_dial`) checks, in this order: the address
   is not empty and each chevron is on the ring; the dialing ring works; a
   ring answers; that ring is not in the dialing ring's facility; the far
   ring works and its room is in the world; the far ring is not in another
   link. Each refusal has its own kind and message: `empty`, `illegal`,
   `unknown`, `same_facility`, `offline`, `busy`. **No faction refuses a
   dial** (Jon, 2026-10-24): there are no bands, permits or reach limits.
6. **Same facility, same dimension.** Two rings in one facility never dial
   each other: that is a walk. Two facilities in one dimension can: the
   Science dimension's gravity trail (`ASCENT`) and its orbital station
   (`GALACTIC`) are far apart, and each dials the other.
7. **Status is deterministic.** A ring is `Online`, `Offline(reason)`,
   `PoweredBy { switch }` (offline until the save switch is on) or
   `BusyWhile { flag }`. Live links make a ring busy for another link. No
   status reads chance.
8. **A connection works both ways.** Both rings open, and either one reaches
   the other without another dial. Each end shows its own destination colour
   and address. A traveller does not close the connection. It closes after
   `idle_timeout_secs` without use (30 simulation seconds by default). A passage
   or pending crossing keeps it open. Use from either end restarts the shared
   timer. A hangup from either terminal closes both ends; a redial replaces the
   whole connection. Leaving the terminal panel keeps the connection open.
9. **Every terminal away from home posts a way home** (`posted`, here
   `LEGAL`). Its Known view always lists it, so no destination needs a known
   address to leave.
10. **Knowledge** is the save flag `gate.known.<ADDRESS>`. A dial that answers
    sets it; a sign, a person or a document sets it with Yarn
    `<<command "world.set_flag" "gate.known.TANGENT" true>>`. The catalog's
    `start_known` decides what a traveller knows before learning anything:
    `All` (every address, hidden ones too) or `Discovered` (only what was
    learned). **Development uses `All`** (Jon, 2026-10-24: development starts
    with everything unlocked). To release with discovery, write
    `start_known: Discovered` in `gate_network.ron`; nothing else changes.

The connection lifetime guards are
`a_connection_routes_both_ways_and_counts_idle_time_after_use`,
`an_active_crossing_holds_the_connection_and_either_terminal_can_close_it`, and
`a_second_participant_can_follow_the_connection_and_return_without_a_dial`.
Travel intents retain the source zone while a crossing is pending. Schema 415
includes that field and both connection endpoints.

## State and ownership

| what | where | kind |
| --- | --- | --- |
| the catalog | `GateNetworkCatalog` resource, from the pack | authored; reload replaces it |
| terminal use and reports | `GateNetworkState` (`resource.gate_network_state`) | rollback state, keyed by `LiveBodyId` |
| links and recent addresses | `GateNetworkState` | rollback state; one connection keyed by its dialing endpoint; recent addresses shared |
| a terminal's dial, hangup, close | `GateTerminalIntent` host intent | stamped tick, released into the sim |
| what the player knows | save flags `gate.known.*` | persisted |
| the panel's focus, entered chevrons, pointer, list cursor | `TerminalUi` | host presentation |
| the chevrons being entered, lit on the ring | `GateDialPreview` | host presentation |
| a ring's portal phase | `GatePortalPhases`, commanded by `GatePortalCommand::Dialed` | rollback state |

The simulation step is `run_gate_network` (Progression, `WorldSync`, after
the switch drain, before the portal tick). The terminal panel
(`presentation::gate_terminal`) is a **dial wheel**: the ring's chevrons
laid out as on the ring (the first at the lower left, Δ at the top), a big
solid Δ in the bottom slice that engages a dial, the address entered in the
middle, and the known and recent addresses beside it. A gamepad points its
stick at a chevron and presses to enter it; the arrow keys and d-pad step
round the wheel (up: the top, down: the Δ); a mouse or a finger hovers and
presses. There is no typing: only the wheel tells Λ from Λ₂. Back takes the
last chevron back, then leaves; the page buttons swap to the list. The panel
claims the seat's input (`gate_terminal`, between dialogue and inventory)
while a terminal is open; the pause menu outranks it. The chevrons entered
light on the ring as they are entered, and a sending ring keeps its link's
chevrons lit (`gate_portal_visuals::sync_ring_lit_glyphs`, the `lit` row of
`interdimensional_gate_glyphs`), with each one's position grid (its `bit`
and `used` rows: a set bit bright, a used row's clear bit dim). Each wheel
plaque draws the same grid. A ring is 288 x 288 world units (2026-10-24, so
the grids read; its sheets are 384 px frames over 192 design units).

A ring has a **shape** (2026-10-31): the triangle (`triangle_gate_ring`,
`_portal`, `_glyphs`; `targets/props/triangle_gate.py`, the round ring's
parts reshaped: chevrons up and down the sides, the Δ at the apex, the same
wormhole clipped to the triangular opening, set into a pedestal with stairs
up to the opening's foot) is the main one; the round ring (`gate_ring`) is
the Jungle's. A ring lights the glyph sheet of its own shape
(`ring_glyphs_kind`); a generator picks the shape with `gates.ring(...,
shape=)`.

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
| `DESTINY` | `magical_gate_zone` | `magical_gate_garden` | east to Starlight Square, down its well to the Starless Hollow, out along the Dune Road | yes |
| `EASEL` | `art_plaza_gate` | `art_painters_plaza` | east to the Sculpture Garden, west to the Story Stacks, the lift up to the Sketchbook Cliffs (the Amphitheatre west of them, the Resonant Reach east), down to the Inkfall Caverns and on to the Workshop Deeps | yes |
| `DYNASTIC` | `history_cradle_gate` | `history_cradle` | the grid of history: east through the ages, ladders between the regions, from the Cradle out of Africa to every land mass | yes |
| `SENTINEL` | `military_gate_zone` | `military_gate_annex` | back through the General's tower | yes; on foot, the checkpoint wants the clearance or the crawlway |
| `NEST` (hidden) | `primeval_rookery_gate` | `primeval_rookery` | down the trail to the trailhead ring | yes |
| `TESLA` (hidden) | `trail_survey_gate` | `gnu_ton_works_approach` | the tower top by GNU-ton's door; the lift down to the spoke | yes |
| glyphs | `math_gate_home_zone` | `gate_stack_lower` (Ring 7) | the gate stack | when its breaker is on; `TANGENT` is posted on it |
| glyphs | `intro_portal_zone` | `gate_stack_lower` (Ring 1, the intro's gate) | the gate stack; its door to the hub | when its power switch is on; `LEGAL` is posted on it |

## The hub

The central hub's main floor has one public gate (ring, dialer, power
cabinet, traffic board, bags for organic travellers) east of the start. The
door to the gate stack stands on the middle platform by the start, paired
with the stack's door to the hub (`gate_stack_to_hub`). The gate stack
(`home_gate_stack`, its own facility, so the public gate dials it) holds
Ring 1 (the intro's gate) and Ring 7, each with its power switch and its
terminal. The halls of bosses, characters and biomes have their doors on
the main floor (Jon put them back); the doors to test rooms and the old doors
to other dimensions are in the **developer wing** (`central_hub_dev_wing`), a
level of the same room east of the main floor, through the old east exit.
`specs/gate_network/make_gates.py` made all of this; it makes the wing and
the annex once and never replaces them.

## The military gate

The annex (`military_gate_annex`, its own facility) opens off the ground floor
of the General's tower (`military_tower`). Its ring answers `SENTINEL`. No
faction refuses a dial, so any ring dials `SENTINEL` and `TESLA` (Jon,
2026-10-24); what the military guards is the annex itself, on foot.

- **Later, by permission.** The save flag `gate.clearance.military` opens the
  annex's checkpoint (a `LockWall`, `gated_by: world.flag_set
  gate.clearance.military`). The General gives it once Castle Godwin's dog is
  down (`boss_cleared("castle_godwin.wolfenstein")`, `factions.yarn`).
- **Early, by one fair trick.** The checkpoint stops 32 px above the floor:
  the maintenance crawlway. A standing robot (48) does not fit; a crouching
  one (24) walks under. The wall is 64 thick and reaches the ceiling, so no
  blink or flight crosses it. The General's line names the crawlway.

## Knowledge sources

The Kernel Guide (`hub_guide`, the public gate's question) teaches TANGENT,
GENESIS and CITADEL; the gate janitor of the gate stack teaches TANGENT; the
General teaches SENTINEL with the clearance. A content test holds every
address a dialogue teaches to the catalog.

## Startup rendering

Jon reports wormhole flicker on desktop and Steam Deck. Warmed High and Ultra
captures show one opening flare, then continuous steady rendering. Cold startup
remains unverified. Capture the first incorrect frame before changing the art
or shader. [GATE-STARTUP-FLICKER](../queue.md#gate-startup-flicker--diagnose-the-wormhole-at-startup)
owns this issue.

## Future expansion

- Several public gates in one hub (the rules hold already: one facility).
- The dial UI in the gate's own glyphs. The art is ready: each of the ring's
  fifteen chevrons is its own glyph (`ambition_sprite2d_renderer.glyphs.legally_distinct`:
  a repeated Λ, Ι or Τ wears a small ring), drawn by the ring sheet, the
  console's dial and the traffic board. The ring stands in a stepped base that
  hides its bottom slice, so the plaques run clockwise from the lower left
  over the top (Δ at 12 o'clock), each glyph's foot toward the centre; it
  plays `charge` (a pulse round the plaques) while a link opens.
  `interdimensional_gate_glyphs` has a `lit` row, frame k chevron k ablaze
  on its plaque: lay it over the ring to light a glyph as it is dialed. The
  panel still draws the Greek letters as text, and the glyphs are not in the
  font yet. A terminal's switch draws nothing: the console is its art. The
  console's dial has the ring's layout (`arch_angle`), with a big solid Δ,
  the press that engages a dial, in the slice the base takes on the ring.
- More knowledge sources: signs and documents that set `gate.known.*` (the
  addresses on the away terminals are posted already), and the news board.
- A guard at the checkpoint (no fitting NPC character yet; the sign and the
  General carry it now).
- More bands (freight), tolls, schedules of outages from authored time.
- Rings in the other 8 of the 17 dimensions.
