# The History dimension

**State:** THIRD PASS BUILT, ATLAS PLANNED (2026-11-04). Built: 47 rooms (40
outdoor rooms on a grid of regions and ages, 7 buildings), with the start in
Africa, waves A (the Atlantic world) and B (the Andes, Caral) included. The
plan for the rest is the History atlas, an ideal graph of world history in
the worldbuilding lab, and "The road ahead" below. The rooms that are not
built yet are the plan for the dimension, not missing work of a pass.

Jon (2026-11-03): "Add a history dimension. This will be a great use for
Genghis Can and Genghis Cant. We will need to add other historical figures.
I want to go world history angle with this. So this is a big dimension.
Think Crash Course World History: the game. We don't need to get it all in
the first pass, but this is the idea I'm going for."

Jon (2026-11-03, the second pass): "History is nonlinear, so I want different
floors / heights to be different contemporaneous eras. We need to start in
Africa with nomadic history and branch to effectively every major land mass,
and that's before we get to the crescent. I think we can do quite a lot with
a 2d grid layout here."

## What it is

A grid. East is later: each column is an age. Each floor is a region of the
world, so the rooms of one column are contemporaneous. A way up and down
joins two floors of one column only where the two regions were in contact
at that time: a hole with a one-way hatch in the upper room's floor, over a
tower of ledges in the lower room (no ladders: Jon, 2026-11-04, "Ladders
should be used very sparingly in the game. Platforms and 'leveled' level
designs are what we want"). A cell is 2800 by 640: a level to cross, not a
shaft to climb. Each room has a development blurb under its title, where
and when and one checked fact (`blurbs.py`), and dates are BCE and CE. The gate (DYNASTIC, `data/gate_network.ron`) is in the Cradle, the
Great Rift Valley 300,000 years ago, at the bottom left. Everyone starts in
Africa. The way out is the great dispersal: up column 1, from
the southern coast through the Levant to Europe, South Asia, Sahul, the
mammoth steppe and Beringia. Only after that do the rooms of farming begin.

Each room is an outdoor room with its own biome, two or three kinds of
creature and its people; some have a building by a door. The east end of
Europe is the Industrial Age, with a sign "TO BE CONTINUED".

It is a world-history dimension, not a European one. Farming is invented
five times in the grid (the Crescent, the Yellow River, the New Guinea
highlands, the first maize, and the Green Sahara's cattle), and each time by
the people who were there.

The host is Clio, the Muse of History (an original character). She keeps
the record and sighs about the footnotes.

| | |
| --- | --- |
| Gate | `DYNASTIC`, ring `history_cradle_gate` in `history_cradle`, terminal `history_gate_terminal` |
| World | `history.ldtk` (`data/worlds.ron`: `world.history_ldtk`) |
| Generator | `game/ambition_map_assets/ambition_content/generators/history/make_specs.py` (the map submodule; it writes its area specs to the temp folder) |
| Dialogue | `dialogue/sandbox/history.yarn`, one node per resident, named after its spec |
| Test | `game/ambition_app/tests/the_history_dimension.rs` |
| Art | the sprite tools: `backgrounds/history_eras.py` and `backgrounds/history_origins.py` (the biomes), `targets/characters/` (the creatures), `data/townsfolk/history_*.chargen.yaml` (the residents; Lucy, who is not a modern human, is her own drawing, `targets/characters/history_lucy.py`), `targets/props/history_*_props.py` |

## Tone

- Parody and puns, never mockery of a people or a faith. The jokes are on
  the famous (kings, generals, explorers, economists) and on how history is
  told: who wrote it, who carved it bigger, who chiselled whose name off.
- Each resident knows their own era well and is often wrong about the
  others. Many lines end on the real lesson of the topic (farming made people
  shorter; the first writing is a receipt; the Silk Road was many roads).
- Prehistory has no famous names. Its people are original (the first
  painters, farmers and navigators) and are told with respect, with a few
  scientists who dug them up as parodies. The Aboriginal and Native American
  rooms show skill and age, with no sacred designs.
- Serious things are not jokes. The Middle Passage has a plain sign in the
  harbour. Columbust says out loud that what happened to the people already
  in the Americas is not funny. The Luddites are told fairly.
- No religious founder is a character.

## The grid

             C0       C1            C2              C3           C4          C5          C6     C7
    R0 Americas       beringia      maize           olmecs       maya        taino       haiti
    R½ S. America                   andes           caral
    R1 East Asia      mammoth st.   yellow river    shang        silk road   steppe
    R2 Oceania        sahul         highlands       lapita       wayfinders  rapa nui
    R2½ SE Asia       sulawesi      austronesians   dong son     srivijaya   angkor
    R3 South Asia     south. route  mehrgarh        indus
    R4 Europe         ice age       first farmers   megaliths    celts       vikings     sail   industrial
    R5 Med./SW Asia   levant        crescent        uruk         classical
    R6 Africa cradle  south. coast  green sahara    nile         aksum       timbuktu    benin  the fort

The columns: C0 origins (300,000 years ago), C1 the great dispersal (70,000
to 12,000 years ago), C2 the first farmers (10,000 to 3,000 BCE), C3 the
first cities and the Bronze Age, C4 the classical age, C5 the middle ages,
C6 the age of sail, C7 the industrial age.

The ways up and down (each is a hole with a one-way hatch in the upper room's
floor, over a tower of ledges in the lower room):

| column | floors | why |
| --- | --- | --- |
| C1 | Africa and the Levant, the Levant and Europe, Europe and South Asia, South Asia and Sulawesi, Sulawesi and Sahul, Sahul and the mammoth steppe, the mammoth steppe and Beringia | the great dispersal: one road out of Africa to every land mass; every island of Wallacea was reached by sea |
| C2 | the Austronesian islands and the New Guinea highlands | the newcomers from Taiwan meet the first farmers of New Guinea |
| C2 | the Green Sahara and the Crescent, the Crescent and Europe | cattle and wheat go both ways; farming walks into Europe |
| C3 | the Shang court and the Lapita voyagers | the Austronesians sail from Taiwan into the Pacific |
| C3 | the Nile and Uruk, Uruk and the megalith builders | Egypt and Sumer trade; tin comes south from the north |
| C4 | Aksum and Rome, Rome and the Celts | the Red Sea trade; the frontier |

The rooms:

| room | biome | people | creatures | the way through |
| --- | --- | --- | --- | --- |
| `history_cradle` | `history_savanna` | Clio, Lucy, Mary Leaky | hyena | the fire drill (`history_first_fire`) opens the thorn boma |
| `history_southern_coast` | `history_savanna` | the ochre artist | hyena, cave lion | the way up: out of Africa |
| `history_green_sahara` | `history_green_sahara` | the cattle herder | hippo, locust swarm | a reed boat carries you over the lake |
| `history_nile_valley` | `history_nile` | Hat-shepsut, Ramesses the Pretty Good | mummy, scarab beetle, asp | a felucca carries you over the river |
| `history_aksum_and_kush` | `history_nile` | Kandake Can-Do | asp, mummy, scarab beetle | the way up: the Red Sea road to Rome |
| `history_timbuktu_coast` | `history_monsoon` | Mansa Moolah, Zheng Hey | inflated coin, coral crab, monsoon squall | a dhow carries you over the sea |
| `history_benin_city` | `history_benin` | the brass caster of Igun Street, Iyoba Idia, Lord Finders-Keepers | driver ants, asp | over the bank of the city walls (the iya); a plain sign for 1897 |
| `history_door_of_no_return` | `history_gold_coast` | Ottobah Cugoano, the castle guide | (none: a sober room) | through the castle gate and the low dungeon to the Door of No Return, which goes one way, to Saint-Domingue; the way home from there is the ring ANTILLES in Haiti |
| `history_levant` | `history_crescent` | Neville the Neanderthal | aurochs, cave lion | the ways: down to Africa, up to Europe |
| `history_fertile_crescent` | `history_crescent` | Hammurabbi, Gilgamash | locust swarm, aurochs, scorpion man | the sluice (`history_sluice`) raises the reed bridge over the canal |
| `history_uruk` | `history_crescent` | Enheduanna | scorpion man, sea raider | the Bronze Age collapse: three Sea Peoples raiders |
| `history_classical_shore` | `history_classical` | Socratease, Herodotoss, Julius Sneezer | hoplite, immortal, vandal | the stoa gate opens when you know that you know nothing (`history_knows_nothing`); the aqueduct is a high road |
| `history_ice_age_europe` | `history_ice_age` | (the cave painter, inside) | cave bear, woolly mammoth, cave lion | the door to the painted cave |
| `history_first_farmers_of_europe` | `history_megalith` | Ötzi the Nice-man | aurochs, steppe wolf | the way down: where the wheat came from |
| `history_megalith_builders` | `history_megalith` | Not-a-Druid | aurochs, steppe wolf | the sun stone (`history_solstice`) opens the trilithon |
| `history_celtic_frontier` | `history_northwoods` | Boudic-car | vandal, steppe wolf | the hillfort's ledges |
| `history_viking_north` | `history_northwoods` | Leif Errandson | viking raider | a longship carries you over the fjord |
| `history_age_of_sail` | `history_sail` | Vasco da Gamer, Christopher Columbust, Captain Cookbook | privateer, scurvy ghost, kraken arm | ships' booms rock over the harbour |
| `history_industrial_age` | `history_industrial` | Adam Smithy, Karl Marks, General Ludd | steam automaton, soot sprite, robber baron | the boiler in the mill (`history_boiler`) opens the factory gate; a rail wagon carries you over the cutting |
| `history_southern_route` | `history_indus` | the coast walker | coral crab, tiger | the ways: up to Sulawesi, down to Europe |
| `history_sulawesi` | `history_wallacea` | the pig painter | Sulawesi warty pig | up the karst to the painted cave: a pig and three hunters, 51,200 years old, and hand stencils; the ways: up to Sahul, down to the southern route |
| `history_austronesian_islands` | `history_austronesia` | the word keeper | saltwater crocodile, coral crab | an outrigger canoe crosses the lagoon; the stilt houses; MATA, one word across half the globe; the way up to New Guinea |
| `history_dong_son` | `history_red_river` | Trưng Trắc, Trưng Nhị, the drum caster | saltwater crocodile, tiger | the paddies (mud and water); the bronze drum; the Trưng sisters' rising of 40 CE |
| `history_srivijaya` | `history_strait` | Yijing, the harbour master of Palembang | water monitor | a jong carries you over the Musi; then the low covered wharf of the warehouses, with a way over their roofs (the Kedukan Bukit stone, 682 CE); the door to Java |
| `history_angkor` | `history_angkor` | the baray engineer, Zhou Da-Guide | king cobra, saltwater crocodile | the West Baray's dike, the sandstone with its peg holes, the causeway over the moat, and Angkor Wat's three tiers |
| `history_mehrgarh` | `history_indus` | the dentist of Mehrgarh | locust swarm, tiger | the terraces |
| `history_indus_cities` | `history_indus` | the Priest-King (Probably) | indus unicorn, tiger | the unicorn seal on the granary roof (`history_unicorn_seal`, a pickup) opens the citadel gate |
| `history_sahul` | `history_sahul` | the raft builder | diprotodon, megalania | a bamboo raft carries you over the strait |
| `history_new_guinea_highlands` | `history_highlands` | the Kuk gardener | cassowary | the garden terraces |
| `history_mammoth_steppe` | `history_ice_age` | Denny the Denisovan | woolly mammoth, cave bear, steppe wolf | the ways: up to Beringia, down to Sahul |
| `history_yellow_river` | `history_yellow_river` | Leizu | silk moth, coral crab | a reed boat carries you over the river |
| `history_shang_court` | `history_yellow_river` | Lady Fu Hao | taotie spirit, steppe wolf | the oracle bone (`history_oracle_bone`) opens the bronze gate |
| `history_silk_road` | `history_silk` | Marco Polo-neck, Confusius | terracotta warrior, desert bandit, silk moth | a camel caravan carries you over the quicksand |
| `history_mongol_steppe` | `history_steppe` | Genghis Can, Genghis Can't | horse archer, golden eagle, steppe wolf | two roads through the Great Wall (below) |
| `history_beringia` | `history_ice_age` | Dr. Clovis-First | woolly mammoth, smilodon | the way down: back to Asia |
| `history_first_maize` | `history_maize` | the teosinte breeder | green iguana, howler monkey, jaguar | the way down: maize goes south to the Andes |
| `history_andean_highlands` | `history_andes` | the potato farmer, Antoine Parmen-tater | llama, Andean condor | up the rock steps to the high plain and its chuno; a plain sign for the Irish famine |
| `history_caral` | `history_supe` | the Caral flautist, the net maker of Aspero | (none: no signs of war are found there) | up the stepped platform pyramid; the quipu that may be the oldest |
| `history_olmec_heartland` | `history_olmec` | the head sculptor | jaguar, howler monkey | a pad of rubber throws you up to the terrace (a jade earspool) |
| `history_maya_city` | `history_maya` | the Zero Hero, Lady Six Sky-High | howler monkey, jaguar, green iguana | the stone ring (`history_ballcourt_ring`) opens the ballcourt gate |
| `history_taino_islands` | `history_caribbean` | the Cacica of Boriken, the canoe maker | coral crab | the hammock throws you into the ceiba; a dugout canoe crosses the sea; a plain sign for 1492 |
| `history_haitian_revolution` | `history_saint_domingue` | Toussaint Louverture, Catherine Flon, the soup joumou cook | grenadier of Leclerc's army | from the cane of the northern plain up and over the mornes, where Napoleon's army lost; the first flag; 1 January 1804; a plain sign for 1825-1947 |
| `history_lapita_voyagers` | `history_lapita` | the Lapita potter | coconut crab, coral crab | an outrigger canoe crosses the lagoon; the way up to the Shang court |
| `history_wayfinders` | `history_open_ocean` | the wayfinder | frigatebird, coconut crab | a voyaging canoe crosses the open sea |
| `history_rapa_nui` | `history_rapa_nui` | the moai carver, Thor Hey-I'm-Right-Dahl | Pacific rat | ride a walking moai over the quarry ditch; a plain sign for 1862 |

The buildings, in a row under the grid:

| building | off | in it |
| --- | --- | --- |
| `history_painted_cave` | Ice Age Europe | the cave painter; hand stencils; chest "the first signature" |
| `history_temple_granary` | the crescent | the Scribe of Debts; writing was made here to count barley; chest "the first receipt" |
| `history_pyramid_tomb` | the Nile | a vigil (`history_tomb_vigil`: scarabs, asps, mummies) shuts the room; when it is over, the treasure room's grille opens |
| `history_colosseum` | the Classical Shore | the games (`history_the_games`: hoplites, immortals, vandals); the emperor's box opens after |
| `history_great_ger` | the steppe | Kublai Khan't-Wait; the Yam relay; chest "a paiza" |
| `history_timbuktu_library` | Timbuktu | Ibn Batootah; manuscripts worth more than gold |
| `history_cotton_mill` | the Industrial Age | the beam engine and the boiler lever |
| `history_borobudur` | Srivijaya (the Sailendra kings ruled both) | two rows high: six galleries of relief panels, joined by stairs at alternate ends, then the round terraces of bell stupas; the relief carver; the hidden foot (found 1885) and the ship (sailed to Ghana, 2003). No images of the Buddha: the stupas are closed |

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

## Music

Jon, 2026-11-03: "I also think we will want new music for different
locations in the history dimension." Each floor (region) has its own cue, so
the music stays with the player who walks east through the ages
(`MUSIC` in the generator, `generators/history/make_specs.py`). Each cue is a
composer script, `tools/ambition_music_renderer/dev/compose_<cue>.py`, which
writes `scores/active/<cue>.music.yaml`. All melodies are original. The
instruments are real samples, most of them from the Versilian VCSL library.

| floor | cue | title | colour |
| --- | --- | --- | --- |
| Africa | `history_africa_first_footsteps` | First Footsteps | G Mixolydian, 12 pulses felt as 4×3 and 3×4; mbira dzaVadzimu and nyunga nyunga interlock, balafon and kalimba call and answer, wordless choir |
| Mediterranean / SW Asia | `history_crescent_between_the_rivers` | Between the Rivers | D Dorian; a plucked lyre (folk harp), oboe as the reed, darbuka and frame drum; the Uruk hymn on male choir |
| Europe | `history_europe_solstice_stones` | Solstice Stones | E Dorian in 6/8; recorder as the bone flute, plucked tagelharpa, frame drum; a sunrise to E major among the stones |
| South Asia | `history_south_asia_monsoon_road` | The Monsoon Road | D Lydian in 7/4 (3+2+2) over a drone; bowed psaltery and flute; a "grid" section for the Indus streets |
| East Asia | `history_east_asia_yellow_earth` | Yellow Earth, Long Sky | G# minor pentatonic; đàn tranh zither, ocarina as the xun, bronze bells for the Shang court, a gallop for the steppe |
| Southeast Asia | `history_southeast_asia_bronze_and_rain` | Bronze and Rain | the idea of a gamelan: a gong closes each 16-beat cycle; marimba core melody, two glockenspiel parts that interlock, recorder as the bamboo flute; a pélog-like E F G B C |
| Oceania | `history_oceania_sahul` | Sahul | C# minor (the key of the didgeridoo samples), clapsticks, slit drum; E major marimba for the highlands |
| Americas | `history_americas_land_bridge` | Over the Land Bridge | A minor to A major; ocarina, slit drum, wine-glass ice for Beringia |

The buildings take the cue of their region (the Granary, the Great Ger,
the Timbuktu Library), except where the place has its own mood: the
tomb and the painted cave (`vault_in_the_dark`), the Colosseum
(`fighters_guild_oath_of_steel`), the cotton mill
(`luddites_guild_loom_and_liberty`). The Age of Sail and the Industrial Age
are not one region's music any more; they keep the cues of their eras.

Samples to measure before you use them again (the composer scripts correct
them per note with `tuning_correction`): the Horse Pulse tagelharpa is up to
70 cents out on some keys; the VCSL đàn tranh, mbira, kalimba, nyunga nyunga
and balafon are 20 to 45 cents out in places; VCSL Frame Drum key 60 is
almost silent. `cue tuning-audit <cue>` measures them.

## The road ahead

Jon, 2026-11-03: "Let's make this dimension stupidly comprehensive. Let's
make sure we prioritize representation and impact. We don't have to have
this be a perfect grid either, in fact, we should bias away from that."

Jon, 2026-11-04: "Polish the history dimension, make it much more
interesting, fun, expansive, immersive, accurate, authentic, aesthetic. Are
we giving proper representation to the different populations? ... Imagine
different players around the world playing and then asking themselves:
where's my history? Do we have an answer? Let's make sure there is an answer
and it is a good one. ... build something open and sprawling. That will
require a very good plan so we aren't going into it blind." And: "not every
room needs to be big and open, we can have corridors."

The long-range ideas (the atlas as terrain, the kinds of link, the source
rules, the tone) are in
[`history-dimension-long-range-roadmap.md`](history-dimension-long-range-roadmap.md).
This section is the plan: what the world will be, and the order to build it.

### The atlas: the reference for what is missing

The History atlas is an ideal graph of world history, kept beside the game
in the worldbuilding lab:

| | |
| --- | --- |
| Tool | `tools/experimental/ambition_worldbuilding_lab/history_atlas.py` (`check`, `report`, `plot`, `where`) |
| Data | `tools/experimental/ambition_worldbuilding_lab/data/history_atlas/`: `frame.yaml` (clusters, regions, horizons, room shapes), `countries.yaml`, `peoples.yaml`, `topics.yaml`, `nodes.yaml`, `edges.yaml` |
| Report | [`history-atlas-report.md`](history-atlas-report.md), generated: do not edit it by hand |
| Tests | `tools/experimental/ambition_worldbuilding_lab/tests/test_history_atlas.py` |

A node is a place in a time (Kilwa, 1100-1505 CE), with its region, the
modern countries and the peoples whose history it is, its topics, its mode
(grounded, parody, sober, archive) and its shape (room, corridor, hub,
pocket, tall). It is built (a room of the game), planned (in the build order
below) or a candidate (the pool). An edge is a historical link: succession,
contact, diffusion, comparison, or an atlas shortcut (a way of the game that
is not history, and must say so).

The tests hold these promises, so that a new room cannot quietly break them:

- The data is whole, and its built part is the game: every room of
  `make_specs.py` is a built node, and the ways of `history.ldtk` are exactly
  the built edges.
- Every country (the UN members, the observer states and the territories
  with large histories: 210) has at least a planned place. So do the 53
  peoples that a passport does not answer for (Aboriginal Australians,
  Māori, Pacific Islanders, Inuit, the Roma, the Kurds, the African, Indian,
  Chinese, Filipino, Lebanese and Irish diasporas, and others).
- Every topic, obvious or neglected, has at least a planned place.
- The built and planned places are one connected world.
- Each part of the world reaches at least 7 of the 11 horizons, from human
  origins to the present.
- No parody touches a sober topic.

`history_atlas.py where <country or people>` answers one player:
`where "Puerto Rico"` gives the Taíno islands (BUILT), and what is planned.

What the atlas says now (2026-11-04: 362 places, 47 built, 188 planned):

- **28% of the world's people have no built room.** The biggest unanswered
  audiences: Indonesia, Brazil, Bangladesh, Japan, the Philippines, DR Congo,
  Vietnam, Thailand, Myanmar, Colombia, South Korea, Uganda. Southeast Asia
  alone is about 680 million people and has no room.
- **The modern world is almost empty.** Only Haiti and the Industrial Age
  are built after 1750. The revolutions, both world wars, the Holocaust,
  decolonization, the Cold War, civil rights, space flight and the internet
  are planned but not built: the "obvious non-neglected topics that we are
  neglecting".
- **Many peoples are planned but not built:** the Indigenous nations of North
  America, the Sámi, the Roma, Jewish history, the Armenians, the diasporas.
- **Five ways of the game are not history** (atlas shortcuts): Aksum to
  Timbuktu, Uruk to the classical shore, Ice Age Europe to the southern
  route, Sahul to the mammoth steppe, the Shang court to the Lapita. Each gets
  a sign that says it is the atlas's road, until the real link is built.

### The shape of the map: clusters, corridors, hubs

The grid stays as the spine: the dispersal out of Africa, column by column.
Around it the world becomes eleven **clusters**, one per part of the world
(Africa, Southwest Asia, Europe, Central Asia and the steppe, South Asia,
East Asia, Southeast Asia, Oceania, North America, Mesoamerica and the
Caribbean, South America), plus the seas and networks that join them. Inside
a cluster, time still runs east, and its regions are its floors.

The atlas finds the order of the clusters as floors that makes the most
historical links walkable (`report`, "Can it be one map?"): Oceania / South
Asia / Southeast Asia / East Asia / Central Asia / Europe / Southwest Asia /
Africa / Mesoamerica / North America / South America. That order walks 51
of the 96 links between clusters. The rest are long contacts (Africa and
Europe across the Mediterranean, Europe and North America across the
Atlantic), and they become:

- **Corridors** (`shape: corridor`). Jon: "not every room needs to be big and
  open, we can have corridors." A narrow, low room (2800 by 320 to 480) that
  is quick to cross and is a journey in itself: the Royal Road, the Silk Road
  (built), the Qhapaq Ñan, the monsoon sea, the strait of Srivijaya, the
  Manila galleons, the indenture ships, Windrush, the Underground Railroad,
  the Western Front's trench. A corridor can join two clusters that do not
  touch, so the flat map stays faithful where it can and honest where it
  cannot.
- **Ships** (the `crossing` mechanic: a felucca, a dhow, a reed boat).
- **Gates.** One ring per cluster for fast travel (Jon, 2026-11-03: "You
  can add gates for fast travel"); Haiti's ANTILLES is the first.
- **Doors**, for a one-way passage with a reason (the Door of No Return).

The other room shapes:

- **Hubs** (`shape: hub`): a town with doors and several ways out; the loops
  and return paths of a cluster meet there (Baghdad, Istanbul, Malacca,
  Kilwa, Tenochtitlan, Chang'an, Lagos and Nairobi now).
- **Tall rooms** (`shape: tall`): a room in levels, of two or more floors
  joined by terrain, never by a ladder (Beringia is the first; Great
  Zimbabwe, Borobudur, Cahokia, Potosí, Hampi, a pā of Aotearoa, the
  Cockpit Country of the Jamaican Maroons, Moshoeshoe's mountain).
- **Pockets** (`shape: pocket`): a small room by a door. The buildings are
  pockets, and so are the memorials: the Holocaust, Hiroshima, 1915,
  Rwanda, the residential schools. A memorial is a pocket so that the player
  chooses to go in, and it is never on the only road.

What the generator (`make_specs.py`) already does beyond a grid: a wide
room takes several cells of one row (an age that lasted, a long road); a row
can go in between two others (its key sorts there: South America under the
Americas); `EXTRA_WAYS` joins rooms that are not one above the other; and a
room can take cells of two rows (Beringia: the land bridge at its foot, down
to Asia; along its floor the Pacific coast under the ice, east to the Andes;
up the ice, the inland way, east at the top to the maize).

The generator becomes data, one module per cluster, when Southeast Asia is
built (the first cluster that is not a grid row): a room table (id, cluster,
place in the cluster, size, shape) and a link table (side, hole, door,
corridor, gate) that the atlas can be checked against.

### The accuracy pass on what is built (first)

Cheap, and it makes the built rooms trustworthy before they are copied:

1. The Cradle: Lucy is deep time (3.2 million years), a guest of Clio's,
   not a neighbour; the fire is carried and tended, not the first fire.
2. The Green Sahara: herding cattle is not farming grain; say which it is.
3. Uruk: the Sea Peoples came 2,000 years after Uruk's height; the raiders'
   sign says it is a later layer of the same ground.
4. The classical shore and the Silk Road: a sign for Clio's guest list
   (Confucius, Marco Polo and the others did not meet).
5. A sign at each of the five atlas shortcuts.
6. Timbuktu: the dhow and Zheng He move to Kilwa when it is built; until
   then, a sign says the coast is the Swahili coast, far to the east.

### The build order

A wave is a few connected rooms, each one finished: its biome, its people,
its music, its blurb, and its test. The order comes from the atlas: the
most players without a built room, then the most obvious topics without a
built room, then the links that make the world one. Each wave starts with
`history_atlas.py report` and ends with its nodes marked built.

| wave | rooms | why now |
| --- | --- | --- |
| A, B | the Atlantic world; the Andes and Caral | BUILT |
| C | **Southeast Asia** (C1 BUILT: Sulawesi, the Austronesian islands, Dong Son; C2 BUILT: Srivijaya, Borobudur, Angkor), the first cluster off the grid: Sulawesi's painted story (51,200 years), the Austronesians come to the islands, Dong Son and its drums (the Trưng sisters), the strait of Srivijaya (a corridor), Angkor (a hub), Borobudur (tall) | 680 million players with no room; it also replaces the Shang-to-Lapita shortcut with Taiwan, the true homeland |
| D | **East Asia's other half**: Taiwan, the Austronesian homeland; Jōmon, the first pots; Silla and Queen Seondeok's star tower; Heian-kyō; Kaifeng and Hangzhou (printing, paper money, the compass); King Sejong's letters | Japan and the two Koreas have no room; printing is an obvious topic |
| E | **Bengal and the cotton**: Dhaka muslin (the other end of the cotton mill), the Company and 1857, the language movement and 1971 | Bangladesh (174 million); colonialism in South Asia |
| F | **Brazil and Amazonia**: the garden cities of the Amazon, Palmares (Zumbi and Dandara), Rio 1888 | Brazil (212 million); Afro-Brazilian and Amazonian peoples |
| G | **Central Africa**: Ishango's counting bone, the kingdom of Kongo, Queen Nzinga, the Congo Free State (sober) | DR Congo (109 million), Angola; the Scramble |
| H | **The revolutions** (a new column, H7): Philadelphia 1776, Paris 1789, the liberators (a corridor), Mexico 1810, Manila 1896, the Underground Railroad | four obvious topics at once, on four continents |
| I | **The world wars** (H8, sober, care first): the Western Front (a corridor; the colonial soldiers), Ethiopia 1935, the war in Asia and the Pacific, the code talkers, the Holocaust (a pocket), Hiroshima (a pocket) | the most obvious gap of all; needs the tone rules and sources ready |
| J | **Decolonization and the Cold War** (H9): the year of Africa, 1947 (sober), the salt march, the Wall, the Korean War, Montgomery to Selma, Baikonur and the Moon, Mabo | civil rights, decolonization, space flight |
| K | **The present** (H10): Lagos and Nairobi now, Shenzhen now, Bengaluru now, the rising sea, the network, the warming world | every cluster's story reaches today |
| L | **Oceania and the Pacific**: Lake Mungo, Budj Bim's eel traps (tall), Aotearoa (tall), Waitangi's two texts, Tonga and Samoa, Nan Madol and Yap's stone money, the Hawaiian kingdom | Australia, New Zealand and the island nations have only deep time now |
| M | **North America's nations**: Watson Brake and Poverty Point, Cahokia (tall), Chaco, the Great Law of Peace, the cedar coast, the Thule and the Inuit, Sequoyah and the Trail of Tears | the Indigenous nations of North America |
| N | **The rest of each cluster**, in the atlas's order: Persia and Baghdad (wave C of the old list), the Indian Ocean (Kilwa, the monsoon sea, the Chola fleets), Great Zimbabwe, Europe's middle (1348, Florence and Mainz, Constantinople), the steppe and Central Asia (the Oxus civilization, Samarkand, the Great Game) | the candidates become planned as the clusters fill |

Waves C to G are representation first; H to K are the obvious topics; L
and M are both. They need not go in this order: a wave can be built when its
art helper and its sources are ready. The atlas's tests say when the plan has
a hole; its report says how big.

### How a room earns its place (in order)

1. Representation: a people or a region the dimension does not show yet,
   or shows only through someone else's eyes (the atlas's "Where's my
   history?").
2. Impact: what still shapes the world now. Zero and algebra, paper and
   printing, the plough and the potato, the Atlantic slave trade and its
   abolition, empires and their ends, revolutions, the world wars.
3. Told from inside: the people it happened to tell it. The Taino room's
   plain sign for 1492 is the model. A catastrophe gets a sober room with
   people and choices, and no joke.
4. Nothing contrived (Jon, 2026-11-03: "try to avoid things that are
   contrived"). The place gives the way through: a wall of earth, the
   mountains, a sea. No lever that needs a story to say why it opens a
   gate, and no symbolic door.
5. Faithful and entertaining (Jon, 2026-11-04: "Don't be afraid of puns.
   Puns are good. Just make them good puns. History is messy. We are going
   to do our best to be faithful and entertaining. ... We can be bolder and
   confront reality. We just need to be careful to observe reality, and not
   define it."). Good puns on the famous; hard facts said plainly, with who
   did what; evidence with its limits ("may be the oldest quipu"), and a
   legend called a legend.
6. Levels, not shafts (Jon, 2026-11-04): platforms and rooms in levels;
   ladders only where nothing else will do. A new room uses the shapes
   above, and a room of the grid is 2800 by 640.
7. Names a player knows. A native name gets the familiar one beside it the
   first time it appears (Boriken, Puerto Rico; Ayiti, Haiti and the
   Dominican Republic). Each room has its blurb: where, when, one fact.

### Ideas for mechanics

- The Yam relay as fast travel along the steppe (a horse at each station).
- Herodotoss's tall tales as false signs (a sign that lies, as in the Art
  dimension's stacks), and Clio's archive: three accounts of one event, side
  by side (the Crusades, 1948, 1857, Waitangi).
- Mansa Moolah's gold: give it away and the price of things in the shops
  falls.
- The monsoon as a switch of the season: the dhow sails one way in summer
  and the other way in winter.
- A crop, a coin or a script that travels: the player meets the same thing
  in several rooms (the potato in the Andes, Ireland and the market of the
  Columbian exchange; cowries in the Maldives and Benin; the alphabet at
  Ugarit, Tyre and the classical shore).
- The ballgame's rubber ball, the stone money of Yap, the cowrie and the
  silver of Potosí as the dimension's currencies, each in its own place.
