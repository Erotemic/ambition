# The Ninja Village (gate address `SILENT`)

A mountain village of the ninja dimension. The world file is
`ninja_village.ldtk`. The gate arrives at the gate shrine, at the zone
`ninja_village_gate_arrival`.

`make_specs.py` writes each room's `.ron` spec and, with `--apply`, makes the
rooms, the Shadow Dojo's door to the forest (`sandbox.ldtk`) and the ladder.
`props.py` gives each room its scenery. Change the scripts, not the specs.
Run them from `tools/ambition_ldtk_tools`.

## The rooms

| room | what it is | who lives there |
| --- | --- | --- |
| `ninja_village_gate_shrine` | the gate shrine; a stone stair up to the torii terrace | Sato (shrine keeper), Nagisa the Gatekeeper (keeps the gate), Kenji (gate-sick courier) |
| `ninja_village_street` | the main street, the tea house, the roofs as a road | Daichi (wants to pass the exam), Ume (tea house) |
| `ninja_village_dojo` | the village dojo and its rank exam | Sensei Hayashi (teacher), Ren (top student and rival) |
| `ninja_village_forest` | the cedar forest; the door to the Shadow Dojo | |

The village is ordinary people. The joke is the institutions (the exam's
forms, the rival form, the shouted names of moves), not the culture.

## The roads

- The low road: shrine floor, street, forest floor.
- The high road: the forest's branches (each 64 px over the one east of
  it), the roofs of the street (gaps of 80 px, steps of 64 px or less), the
  torii terrace, the shrine's stair. Each room joins the next at both
  heights, so the two roads are one loop.
- The way on: the forest's east door goes to `ninja_dojo` (two ways), the
  hall of the Shadow Clan (the Oni leader, the Duelist, the Heavy), and from
  there to the mockingbird arena.

## The rank exam

The dojo's exam floor is an encounter (`ninja_village.rank_exam`): its lock
wall, its trigger strip, a bell (`ResetEncounter`) to take it again, three
training dummies (`sandbag`) and the examiner (the Ninja Swordsman, who shows
his eyes). The encounter has no waves in the wave book, so these spawns are
its one wave. After it, Sensei Hayashi awards the rank
(`ninja_village_rank_awarded`). The quest is `ninja_village_rank_exam`
(`quests.ron`); the talk is `dialogue/sandbox/ninja_village.yarn`.

## The gate

Agent GATE places the ring and its terminal on the shrine's clear floor
(x 160..512, floor y 704, clear to 224 over the floor). Put those entities in
`gate_interface.yaml` here (an `entity add` spec): `--apply` adds that file
again after it makes the rooms. Until the ring stands, the arrival zone is a
one-way door to the hub.

## Proof

`game/ambition_app/tests/the_ninja_village.rs` walks the loop by keys from
the shrine, talks to each resident, passes the exam by Attack presses, and
goes through the door to the Shadow Dojo and back.
