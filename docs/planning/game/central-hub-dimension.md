# The Central Hub dimension: the Gate Authority's capital (started 2026-11-25)

**State:** BEGUN. Three rooms, two biomes, four residents and the gate are
in the game. What is under "Next" is not built.

Jon (2026-11-24): "The Central Hub Dimension (Home of the Gate Authority)".
What the lore already says about the Authority
(`docs/brainstorms/good_llm_ideas/gate_frontier_war_and_world_tone.md`,
`docs/brainstorms/2026-11-20-hobford-gates-and-frontier.md`,
`docs/storylines/cannon_expansions.md` F10):

- The official public network routes through a central hub. The Authority
  restricts what addresses people know; it can control terminals and
  information, not make public routes unknowable.
- Gates are unlocked by nature; the Authority maps them and installs
  restrictions. Bootleg, stolen and unregistered routes still exist.
- The war with the Nazi regime ended in a stalemate, the Authority (or what
  became it) a little the stronger.
- Bureaucracy operates as denial: a Manifest Office that erases by filing,
  classifying and "correcting" records rather than by force.
- Gate staff are **Gatekeepers** (one word). Alice and Bob's usual base is
  in a major Authority city.

The concept art (2026-11-22) is in the assets submodule,
`assets/concept_art/misc/`: `Central Hub Gate Authority.png` (a station
concourse: the ring on a raised floor, an information booth under a blue
dome, signal posts, a catwalk of crates overhead, a baggage hall below with
a conveyor, a claw and carts, monorails beyond tall arched windows) and
`Central Gate Plaza and Portal Ring-2 (1).png` (a plaza: a blossom tree on
a planter with benches, lamp posts with the Authority's banners, a notice
board, a great clock under an arcade with stairs up to it, arched bridges
full of commuters, floating islands, a giant ring in the sky). They are a
reference for the look, not a layout.

| | |
| --- | --- |
| world file | `authority.ldtk` (`data/worlds.ron`: `world.authority_ldtk`) |
| biomes | `authority` (the plaza, outdoors), `authority_hall` (the concourse and the offices) |
| gate address | `LICENSED`, on the Concourse (`data/gate_network.ron`, posted `LEGAL`) |
| generator | `game/ambition_map_assets/ambition_content/generators/authority/make_specs.py` |
| dialogue | `dialogue/sandbox/authority.yarn`; the Hall's nodes in `hall.yarn` |
| art | renderer `backgrounds/authority.py`, `targets/props/authority_props.py`, `data/townsfolk/authority_*.chargen.yaml` |
| music | `terms_and_conditions` (a compliance lounge in 5/4) |

## Home and the capital

The game's home is the sandbox's central hub (`central_hub_complex`, address
`LEGAL`): every terminal away from home posts `LEGAL`, and a new game starts
there. This slice leaves that alone. The Authority's capital is a dimension
of its own with its own address, `LICENSED`, and the Concourse's terminal
posts `LEGAL` like every other. Ruled 2026-11-26
([Q185](../maintainer-decisions.md)): `LEGAL` becomes the capital's public
address, and the developer sandbox gets its own address. Not built yet:
queue GATE-LEGAL.

## The shape of it

```
 [ the Concourse ] ==== [ Authority Plaza ]
   (the baggage              |
    hall below)            door
                      [ the Manifest Office ]
```

- **The Concourse** (`authority_concourse`): the arrival hall. The gate and
  its terminal; the information booth and Marguerite in it; a catwalk of
  freight overhead (a chest on it). A grate in the floor drops to the
  baggage hall below: a conveyor of bags from every dimension, the claw,
  and Lou at Lost Baggage. The lift beside the conveyor returns to the
  main floor under the freight catwalk.
- **Authority Plaza** (`authority_plaza`), east: the arcade with the great
  clock, stairs up to its balcony, Gatekeeper Imre at the podium; the
  blossom tree on its planter, benches round it; the notice board; the
  Manifest Office's door.
- **The Manifest Office** (`authority_manifest_office`), by a door: the
  take-a-number machine, NOW SERVING, the counter, the Registrar, and
  filing cabinets to the ceiling (a chest on top).

The capital is safe: there is nothing to fight in it.

## The people

| Id | Name | Where | Note |
| --- | --- | --- | --- |
| `npc_authority_information` | Marguerite | the Concourse, Information | Can tell you any address you like, so long as it is this one. |
| `npc_authority_baggage` | Lou | the Concourse, Lost Baggage | Bags come back from other dimensions changed. |
| `npc_authority_gatekeeper` | Gatekeeper Imre | the Plaza | Explains what "Authority" means: they were there first, with a clipboard. |
| `npc_authority_registrar` | the Registrar | the Manifest Office | Never deletes a record. Corrects it. |

## Next

- The rest of the capital: the bridges of commuters, the monorail, the
  islands, the great ring in the sky.
- The Authority's own offices: the Gatekeepers' hall, the Bureau of
  Addresses (where addresses are kept, and the bootleg ones hunted).
- Alice and Bob's desk at the Cartographers' Office, for when they get home.
- Its own music.
