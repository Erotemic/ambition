# The orbital station of the Science dimension

A small inhabited station on the gate network (address `GALACTIC`), in the
Science dimension's world file `science.ldtk` with GNU-ton's gravity works
(`../gnu_ton_gravity/`). A spoke joins the two on foot and by tram, so they
are one geography with a gate each.

`make_specs.py` writes each room's `.ron` spec and, with `--apply`, makes the
rooms in `science.ldtk` and checks that no room id is in two world files.
Change the script, not the specs. Run it from `tools/ambition_ldtk_tools`
with that tool's `.venv`, after `../gnu_ton_gravity/make_specs.py --apply`
(the spoke and the works name each other's doors), and then
`../scenery_overlay.py --apply`, which puts back the decks' scenery art and
signs (the rooms are older than their art).

## The rooms

| room | what is in it |
| --- | --- |
| `orbital_station_gate_dock` | the gate's arrival `orbital_station_gate_arrival` (clear floor x 128..640 for the ring and its terminal), the freight track, customs (Officer Okafor), the manifest board, the quarantine pen that shuts the freight lane |
| `orbital_station_docking_bay` | the hauler *Remainder* in its berth, the gantry, a shuttle; Captain Reyes, Bo Lindqvist |
| `orbital_station_observation_deck` | the viewpoint on the planet; Wen Liang, Neil onGras Turfson |
| `orbital_station_habitation` | bunks, galley, hydroponic racks; Gus Pell |
| `orbital_station_service_deck` | the plant, the hatch of the damaged spur, the spoke; Hamid Rahimi |
| `orbital_station_damaged_spur` | the optional dangerous branch: broken deck over live cable, crawlers and mites, the breaker and a cache |
| `orbital_station_spoke` | the walk to the gravity works, a tram over the open gap, the lower end of the works' lift |

Two decks, one over the other, laid out as the world map shows them: on each,
a room opens on the next through its side (no doors between rooms of a deck,
Jon 2026-11-09). Upper deck, west to east: service deck, habitation ring,
observation deck. Lower deck: gate deck, docking bay, tether terminus
(`../eureka/`). Stairs join the gate deck to the ring and the bay's gantry to
the observation deck, each pair standing over each other; the spur's hatch
and the spoke are doors off the service deck.

Loop: gate deck, habitation ring, observation deck, docking bay, and back to
the gate deck by the freight lane once customs clears the crate. The tether
only goes up: the way down to the planet is the gate. The trail's shortcut is
the works' lift from `gnu_ton_works_approach` to the end of the spoke, by the
orchard.

## The errand: consignment 0447

Quest `orbital_consignment` (`quests.ron`); talk in `orbital.yarn` and
Canter's node in `math.yarn`. Okafor holds a crate of survey lenses whose
manifest counts them as "aleph-null". Georg Canter, in the Math town's
Surveyor's Office, signs a finite count. Okafor clears the crate: the
quarantine pen's two gates open (`orbital_consignment_cleared`) and so does
the spur's hatch (`orbital_spur_pass`). The quest completes when the robot is
home in the hub. The breaker in the spur is `switch_orbital_spur_breaker_used`.

Until the gate network's return terminal exists, the arrival zone is also a
one-way door to the hub (`orbital_station_temp_hub_link` in its name).

## Proof

`game/ambition_app/tests/the_orbital_station.rs` walks the loop by the robot's
own inputs, opens the freight lane and the hatch by their flags, rides the
tram, and checks the spur's foes, breaker and live cable.
`game/ambition_app/tests/the_consignment_errand.rs` runs the errand in the
shipped session (station, Math town, station, hub) and reads it back from a
saved file. `the_gnu_ton_handle.rs` walks the spoke and the works' lift.
