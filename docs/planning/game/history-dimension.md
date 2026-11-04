# The History dimension

**State:** SECOND PASS BUILT (2026-11-03). The first pass was one road of
eight eras. The second pass makes it a grid of 29 rooms (7 regions, 8 ages)
and 7 buildings, with the start in Africa. The topics that are not built yet
are in "Next passes" at the end of this document: they are the plan for the
dimension, not missing work of this pass.

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
| Generator | `tools/ambition_ldtk_tools/specs/history_dimension/make_specs.py` |
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
| C1 | Africa and the Levant, the Levant and Europe, Europe and South Asia, South Asia and Sahul, Sahul and the mammoth steppe, the mammoth steppe and Beringia | the great dispersal: one road out of Africa to every land mass |
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
| `history_southern_route` | `history_indus` | the coast walker | coral crab, tiger | the ways: up to Sahul, down to Europe |
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
(`MUSIC` in `specs/history_dimension/make_specs.py`). Each cue is a
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

The long-range plan (the atlas idea, coverage, the review notes on the
built rooms) is [`history-dimension-long-range-roadmap.md`](history-dimension-long-range-roadmap.md).
This section is the order of the build.

Jon, 2026-11-03: "Let's make this dimension stupidly comprehensive. Let's
make sure we prioritize representation and impact. We don't have to have
this be a perfect grid either, in fact, we should bias away from that. But I
figure it's easy to start there." And: "let's do this slowly."

### How a room earns its place (in order)

1. Representation: a people or a region the dimension does not show yet,
   or shows only through someone else's eyes. Southeast Asia, South America,
   North America, southern and eastern Africa, Persia, Korea and Japan have
   no room yet; the Islamic world has one building's worth of scholars.
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

### Not a perfect grid

The generator (`make_specs.py`) reads the grid as a start, not a rule:
- A wide room takes several cells of one row: an age that lasted, or a
  long road (the maize road north, the Qhapaq Nan).
- A row can go in between two others (its key sorts there): Southeast Asia
  between Oceania and South Asia.
- `EXTRA_WAYS` joins rooms that are not one above the other.
- A room can take cells of two rows: Beringia is the land bridge at its
  foot (down to Asia), so it touches the mammoth steppe under South
  America's row, and it has the two ways south that are argued over: along
  its floor, the Pacific coast under the ice, east to the Andes (Monte Verde
  is older than Clovis); up the ice, the inland way, east at the top to the
  maize.
Still to build: more rooms in levels (a climb up the Andes, the hill of Great
Zimbabwe), pockets that hang off one room, narrow rooms and rooms of other
heights, and doors that join far places (the Door of No Return opens on
Saint-Domingue).

### The waves (slowly: a few rooms at a time, each one finished)

- A. The Atlantic world: Benin City and its bronzes (C6, Africa: BUILT);
  the Haitian Revolution (C6, the Americas, beside the Taino room: Ayiti is
  the same island, and Haiti took back its Taino name): the people who freed
  themselves (BUILT); the Door of No Return, a sober room of the Atlantic
  slave trade (C7, Africa), a fort whose door goes one way only, to
  Saint-Domingue (BUILT). Nothing brings you back that way; Haiti's ring
  (ANTILLES) is the way home. Gates are the dimension's fast travel (Jon,
  2026-11-03: "You can add gates for fast travel"): a far region can have
  its own ring.
- B. South America, a new floor under the Americas (row ½, so the way
  down from the first maize goes to it): the Andean highlands (C2: the
  potato, quinoa, the llama) and Caral (C3) are BUILT. Then:
  Caral, the oldest city of the Americas (C3); Nazca and the Moche (C4);
  Tiwanaku and the Wari (C5); the Inca and the Qhapaq Nan (C6). The Aztecs
  and Tenochtitlan (C5, the Americas: a row of its own, or a pocket of the
  Maya city; Haiti has C6 of the Americas floor).
- C. The Islamic Golden Age and Persia: the House of Wisdom in Baghdad
  (C5: al-Khwarizmi's algebra, the translators; no founders); the
  Achaemenids and the Royal Road (C4); the Ottomans and the coffee house
  (C6).
- D. South and Southeast Asia: Ashoka and his pillars (C4; no founder as a
  character); Nalanda (C5); the Cholas at sea (C5); the Mughals (C6). A new
  row: Dong Son and its drums (C4), Srivijaya (C5), Angkor (C5).
- E. East Asia: the Tang and the Song (printing, gunpowder, the compass);
  Heian Japan (Murasaki Shikibu, the first novel); Sejong and Hangul; the
  Meiji.
- F. Africa south and east, and North America: Great Zimbabwe and Kilwa
  (C5 and C6); Lalibela; Kongo. Cahokia, the Haudenosaunee and the Great Law
  of Peace, the Ancestral Puebloans (a new row).
- G. Oceania: Aotearoa and the Maori, Hawaii, Tonga.
- H. Europe and the revolutions: printing and the Renaissance, the
  Reformation as history of states and print (no founders), the
  Enlightenment, the American, French and Latin American revolutions,
  abolition.
- I. A new column, the twentieth century: the world wars, decolonization
  (the year of Africa, 1960), partition, the Cold War, civil rights. These
  need the most care: sober rooms, people and choices, few or no jokes.

The rest of Jon's list (Crash Course World History, both seasons) is the
pool these waves take from: the Bronze Age collapse as a mechanic in Uruk;
the Little Ice Age, drought, famine and disease as mechanics (a frozen
canal, a failed harvest); Alexander ("Alexander the Late", the empire that
ran east); Venice; Russia; Charles V and the Spanish Empire; the V.O.C. (the
first corporation as a boss: the robber baron's ancestor); nationalism and
imperialism; the Congo; Iran's and China's revolutions; Israel and
Palestine; non-violence; globalization, population, energy, water, debt;
the West and historical method, and Clio's own quest (each era's people
disagree about what happened; Clio reads the accounts side by side).

Ideas for mechanics:
- A second ring at the east end, so the grid need not be walked back.
- The Yam relay as fast travel along the steppe (a horse at each station).
- Herodotoss's tall tales as false signs (a sign that lies, as in the Art
  dimension's stacks).
- Mansa Moolah's gold: give it away and the price of things in the shops
  falls.
- The monsoon as a switch of the season: the dhow sails one way in summer
  and the other way in winter.
