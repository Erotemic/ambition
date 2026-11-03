# The History dimension

**State:** FIRST PASS BUILT (2026-11-03). Eight eras, six buildings, eight
biomes, the cast and the gate are in the game. The topics after the
Industrial Age are in "Next passes" at the end of this document: they are
the plan for the dimension, not missing work of the first pass.

Jon (2026-11-03): "Add a history dimension. This will be a great use for
Genghis Can and Genghis Cant. We will need to add other historical figures.
I want to go world history angle with this. So this is a big dimension.
Think Crash Course World History: the game. We don't need to get it all in
the first pass, but this is the idea I'm going for."

## What it is

A timeline. You walk east and you walk forward in time. The gate (DYNASTIC,
`data/gate_network.ron`) is at the far west, at the dawn of farming. The eras
stand side by side on one long road; each is an outdoor room with its own
biome, three creatures and its famous people, and most have a building by a
door. The east end of the first pass is the Industrial Age, with a sign "TO
BE CONTINUED".

It is a world-history dimension, not a European one. The road goes from
Mesopotamia and the Nile through the Mediterranean, along the Silk Road,
over the steppe, to Mali and the Indian Ocean, and only then to the ships
and the mills.

The host is Clio, the Muse of History (an original character). She keeps
the record and sighs about the footnotes.

| | |
| --- | --- |
| Gate | `DYNASTIC`, ring `history_crescent_gate` in `history_fertile_crescent`, terminal `history_gate_terminal` |
| World | `history.ldtk` (`data/worlds.ron`: `world.history_ldtk`) |
| Generator | `tools/ambition_ldtk_tools/specs/history_dimension/make_specs.py` |
| Dialogue | `dialogue/sandbox/history.yarn`, one node per resident, named after its spec |
| Test | `game/ambition_app/tests/the_history_dimension.rs` |
| Art | the sprite tools: `backgrounds/history_eras.py`, `targets/characters/` (the creatures), `data/townsfolk/history_*.chargen.yaml` (the residents), `targets/props/history_*_props.py` |

## Tone

- Parody and puns, never mockery of a people or a faith. The jokes are on
  the famous (kings, generals, explorers, economists) and on how history is
  told: who wrote it, who carved it bigger, who chiselled whose name off.
- Each resident knows their own era well and is often wrong about the
  others. Many lines end on the real lesson of the topic (farming made people
  shorter; the first writing is a receipt; the Silk Road was many roads).
- Serious things are not jokes. The Middle Passage has a plain sign in the
  harbour. Columbust says out loud that what happened to the people already
  in the Americas is not funny. The Luddites are told fairly.
- No religious founder is a character.

## The road

    [crescent][nile][classical][silk road][steppe][timbuktu][sail][industrial]
       door     door   door                 door     door           door
    [granary] [tomb] [colosseum]           [ger]   [library]       [mill]

| era | room | biome | people | creatures | the way through |
| --- | --- | --- | --- | --- | --- |
| The Fertile Crescent: the Agricultural Revolution, Mesopotamia, Debt & Money | `history_fertile_crescent` | `history_crescent` | Clio, Hammurabbi, Gilgamash | locust swarm, aurochs, scorpion man | the sluice (`history_sluice`) raises the reed bridge over the canal |
| The Nile: Egypt | `history_nile_valley` | `history_nile` | Hat-shepsut, Ramesses the Pretty Good | mummy, scarab beetle, asp | a felucca carries you over the river |
| The Classical Shore: Persians & Greeks, Rome, the Fall of Rome, Water & Classical Civilizations | `history_classical_shore` | `history_classical` | Socratease, Herodotoss, Julius Sneezer | hoplite, immortal, vandal | the stoa gate opens when you know that you know nothing (`history_knows_nothing`); the aqueduct is a high road |
| The Silk Road: the Silk Road, China | `history_silk_road` | `history_silk` | Marco Polo-neck, Confusius | terracotta warrior, desert bandit, silk moth | a camel caravan carries you over the quicksand |
| The Mongol Steppe: the Mongols | `history_mongol_steppe` | `history_steppe` | Genghis Can, Genghis Can't | horse archer, golden eagle, steppe wolf | two roads through the Great Wall (below) |
| Timbuktu and the Monsoon Coast: Mansa Musa, Indian Ocean Trade | `history_timbuktu_coast` | `history_monsoon` | Mansa Moolah, Zheng Hey | inflated coin, coral crab, monsoon squall | a dhow carries you over the sea |
| The Age of Sail: 15th-century mariners, the Columbian Exchange, the Atlantic Slave Trade, Captain Cook | `history_age_of_sail` | `history_sail` | Vasco da Gamer, Christopher Columbust, Captain Cookbook | privateer, scurvy ghost, kraken arm | ships' booms rock over the harbour |
| The Industrial Age: the Industrial Revolution, Capitalism, Railroads | `history_industrial_age` | `history_industrial` | Adam Smithy, Karl Marks, General Ludd | steam automaton, soot sprite, robber baron | the boiler in the mill (`history_boiler`) opens the factory gate; a rail wagon carries you over the cutting |

The buildings:

| building | off | in it |
| --- | --- | --- |
| `history_temple_granary` | the crescent | the Scribe of Debts; writing was made here to count barley; chest "the first receipt" |
| `history_pyramid_tomb` | the Nile | a vigil (`history_tomb_vigil`: scarabs, asps, mummies) shuts the room; when it is over, the treasure room's grille opens |
| `history_colosseum` | the Classical Shore | the games (`history_the_games`: hoplites, immortals, vandals); the emperor's box opens after |
| `history_great_ger` | the steppe | Kublai Khan't-Wait; the Yam relay; chest "a paiza" |
| `history_timbuktu_library` | Timbuktu | Ibn Batootah; manuscripts worth more than gold |
| `history_cotton_mill` | the Industrial Age | the beam engine and the boiler lever |

### The two roads through the Great Wall

Genghis Can and Genghis Can't (catalog `npc_genghis_can`, `npc_genghis_cant`,
built before this dimension) stand at the west face of the Wall.

- Ask Genghis Can, and he says yes before you finish: the gate in the Wall
  opens (`history_can_says_yes`). It is the short road, and the garrison
  (horse archers and wolves) waits at its foot.
- Ask Genghis Can't, and he has found another way: a bamboo scaffold up the
  west face (`history_cant_says_maybe`; three planks that are LockWalls gated
  by `not world.flag_set`), along the rampart and down over the garrison's
  heads. "We arrive at the same place, later."

Each flag opens its own road and not the other's
(`each_brother_at_the_great_wall_opens_his_own_road`).

## Next passes

The rest of Jon's list (Crash Course World History, both seasons), grouped by
where it goes on the road. Each group is a room, or a building, or a mechanic.

Between the eras that exist:
- The Indus Valley (a quarter of the crescent: Mohenjo-daro's drains and
  seals that nobody can read yet).
- Buddha and Ashoka (Ashoka's pillars and his turn to non-violence after
  Kalinga; no founder as a character).
- Alexander (a room between the Classical Shore and the Silk Road: the
  empire that ran east; "Alexander the Late").
- Christianity and Islam, Islam and Politics (as history of states and
  scholarship: the House of Wisdom, the libraries; no founders).
- The Dark Ages, the Crusades, Vikings, Venice and the Ottomans, Russia, the
  Heian aristocracy, the Mughals.
- The Bronze Age Collapse (a ruin room: the Sea Peoples, a collapse you can
  feel as falling platforms).
- The Little Ice Age, Drought & Famine, Disease (mechanics more than rooms:
  a frozen canal, a failed harvest).

After the Industrial Age (the road goes on east):
- Nationalism, Imperialism, Asian Responses to Imperialism, Congo, the
  V.O.C. (the first corporation as a boss: the robber baron's ancestor).
- The American, French, Haitian and Latin American Revolutions, Nation
  Building in Latin America (a revolutions room with many flags and doors).
- The Seven Years War, Charles V, the Protestant Reformation, the Spanish
  Empire.
- World War I (How it began, Who started it), World War II (as a war for
  resources), the Cold War, Decolonization, Israel and Palestine, Iran's
  Revolutions, China's Revolutions, China and Democracy, Non-Violence.
  These need care: a sober room, people and choices, few jokes.
- Globalization I and II, Population and Malthus, Energy, Railroads (more),
  Water, Debt and Money (more), War.
- The West and Historical Methodology, Rethinking Civilization: Clio's own
  quest. Each era's people disagree about what happened; the player
  collects the accounts and Clio reads them side by side.

Ideas for mechanics:
- A second ring at the east end, so the road need not be walked back.
- The Yam relay as fast travel along the steppe (a horse at each station).
- Herodotoss's tall tales as false signs (a sign that lies, as in the Art
  dimension's stacks).
- Mansa Moolah's gold: give it away and the price of things in the shops
  falls.
- The monsoon as a switch of the season: the dhow sails one way in summer
  and the other way in winter.
