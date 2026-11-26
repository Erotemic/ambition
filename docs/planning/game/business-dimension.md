# The Business dimension: Capital (started 2026-11-24)

**State:** BEGUN. Three rooms, two biomes, five residents, the gate and the
dimension's art are in the game. What is under "Next" is not built.

Jon's direction (`docs/brainstorms/story_and_world_directions_2026-11-18.md`,
"Business Dimension: everyone is doing business"): a maximalist world of
businessmen doing business: Business College, business trips, deadlines,
TPS reports, ledgers, corporate ladders, and the real costs too, worker
exploitation, unions, cruelty justified as "only business". Satirize the
institutions, not every worker. Intellectual property goes to extremes:
directions and maps are licensed. Zorlack 65536 makes his first trip here,
looking for a career.

The concept art (2026-11-22) is in the assets submodule,
`assets/concept_art/misc/`: `Business Dimension_ Corporate Cityscape.png`,
`... Corporate Climb.png`, `... Paperwork Metropolis (1).png` and the four
`ChatGPT Image Nov 22, 2026, 08_0{8,9}_*` sheets (the paperwork works, the
records hall, the airport, the open-plan floors, the college campus). They
are a reference for the look, not a layout.

| | |
| --- | --- |
| world file | `business.ldtk` (`data/worlds.ron`: `world.business_ldtk`) |
| biomes | `business` (the city, outdoors), `business_office` (inside) |
| gate address | `DEALINGS`, Founders' Plaza (`data/gate_network.ron`, posted `LEGAL`) |
| generator | `game/ambition_map_assets/ambition_content/generators/business/make_specs.py` |
| dialogue | `dialogue/sandbox/business.yarn`; the Hall's nodes in `hall.yarn` |
| art | renderer `backgrounds/business.py`, `targets/props/business_props.py`, the skins and decor rows, `data/townsfolk/business_*.chargen.yaml` |
| music | `series_a_bloodbath` in every room for now |

## The names

| What | Name | Why |
| --- | --- | --- |
| The city | **Capital** | It is the capital of the dimension, and it is money. |
| The gate's address | **DEALINGS** | Every letter is on the ring; it is what the city does. |
| The plaza | **Founders' Plaza** | The gilt statue of the Founder, SELF-MADE*, with a small loan in the small print. |
| The café | **The Bottom Line** | Where every table is pitching another. The cups' shape is patented. |
| The union | **Local 17**, the records clerks | Paid by the form; the forms got longer. |

## The rooms

```
               door
    [the Business College]
              |
    [ Founders' Plaza ] ==== [ Records & Processing ]
```

- **Founders' Plaza** (`business_founders_plaza`): the gate and terminal at
  the west end; The Bottom Line, whose awning is a step up to its balcony and
  a chest; Dirk Quarterly on his call; the fountain with the Founder on his
  pedestal (both are steps); Solly Darity's picket line; the portico of the
  Business College and its door. The east edge goes on to Records.
- **Records & Processing** (`business_records_processing`): the conveyor and
  the APPROVED press on the floor, the shredder's pit (a `HazardBlock` at its
  foot). The service stairs on the east lead to the folder carrier
  (`MovingPlatform`, look `business_folder_carrier`). It crosses to the west,
  where the intake shelf leads to Marguerite Pell's desk and the archive
  chest.
  The background is the records hall: arched bays of binders, brass tubes.
- **Lecture Hall 101** (`business_college_lecture_hall`): Professor Gail
  Margin teaches Synergy from the stage. Visitors can climb the seating
  tiers or take the aisle below them and use the rear ladder. Both routes
  reach Zorlack 65536 (`npc_ninja_shadow_oni_leader`) in the back row. He
  has the best attendance in the class and no interview.

The Corporate Ladder is in the city's background, rising into the clouds
with no top in sight. It is a landmark there and not a room yet.

## The people

| Id | Name | Where | Note |
| --- | --- | --- | --- |
| `npc_business_dirk_quarterly` | Dirk Quarterly | plaza | An executive always on a call; he speaks the city's jargon on purpose. |
| `npc_business_solly_darity` | Solly Darity | plaza | Picket captain of Local 17. |
| `npc_business_barista` | Ines Vale | plaza | The Bottom Line's barista. |
| `npc_business_records_clerk` | Marguerite Pell | Records | Eleven years on Form 27-B; still inside, processing the strike notices. |
| `npc_business_professor` | Gail Margin | college | Professor of Synergy. |
| `npc_ninja_shadow_oni_leader` | Zorlack 65536 | college | His first trip to the Business Dimension (renamed from "Shadow Oni Leader", Jon, 2026-11-24). |

## Next

Ideas in the concept art and the brainstorm, not built:

- **The Corporate Ladder** as a place: rungs named for job titles, and a
  glass ceiling. A room of tiers and platforms, not a shaft
  ([levels, not shafts](../../authoring/level-design-guide.md)).
- **Capital International Airport**: the business trip, the moving walkway,
  baggage conveyors, security.
- **The open-plan floors** and the executive suite over them.
- **The patent court**: trial by combat for GNU-ton's gravity patent
  (user-proposed, open).
- **Licensed cartography**: Alice and Bob arrested for sharing a map.
- Enemies of the place, and its own music.
