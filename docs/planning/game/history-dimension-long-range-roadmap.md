# History dimension: long-range worldbuilding roadmap

**State:** PROPOSED WORLD DESIGN (2026-11-03). This is a companion to
[`history-dimension.md`](history-dimension.md), not a replacement for that
built-world record. It does not select engineering work or change
[`docs/planning/queue.md`](../queue.md). Its room and story suggestions are
candidates, not approved canon. Do not treat a missing room as a defect in the
current game.

**Goal:** Build a connected, playable account of human history across the
world. Start with early humans in Africa. Reach the present. Show many
societies at the same time, and show how people, materials, ideas, disease,
violence, and institutions moved between them. Make each region an inhabited
place, not a history quiz or a display case.

## 1. Preserve the existing foundation

The source archive at commit `290f93f` contains:

- The `DYNASTIC` interdimensional gate in `history_cradle`.
- Seven geographic rows and eight loose age columns.
- **36 outdoor locations and 7 interiors: 43 rooms total.** The headline in
  `history-dimension.md` still says 29 outdoor rooms. The source generator,
  its 43 room specs, the LDtk world, and the room-connection test reflect the
  larger layout. Correct the older headline during a focused documentation
  pass; do not make it the target count for new work.
- The first people, places, enemies, route puzzles, dialogue, and music.
- Two routes at the Great Wall with Genghis Can and Genghis Can't.
- Existing planned waves A-I. Those waves remain source material for this
  roadmap. Their order does not force implementation order.

**Current source owners** (relative to the repository root):

- `docs/planning/game/history-dimension.md`: built content and current ideas.
- `game/ambition_map_assets/ambition_content/generators/history/make_specs.py`: region
  positions, room specs, routes, and music assignments.
- `game/ambition_content/assets/worlds/history.ldtk`: the published world.
- `game/ambition_content/assets/data/gate_network.ron`: the dimension gate.
- `game/ambition_content/assets/dialogue/sandbox/history.yarn`: dialogue.
- `game/ambition_app/tests/the_history_dimension.rs`: room, cast, puzzle,
  encounter, and route tests.
- `tools/ambition_sprite2d_renderer/`: biome, character, and prop art.
- `tools/ambition_music_renderer/`: existing regional cues.

This roadmap proposes new narrative and geography work. It does not require
an engine rewrite, a new kind of gate, or new content schemas today.

## 2. The governing idea

**History is many lives at once.** A region is not a step on a single ladder
from primitive to advanced. At any date, people live in many places. They use
different technologies and institutions. They make choices under different
conditions. They often do not know what is happening elsewhere.

The dimension must show three things in each major historical period:

1. **Local life.** Food, homes, labor, travel, play, art, family, and belief.
2. **Contact and difference.** Trade, migration, exchange, cooperation,
   conquest, resistance, and independent invention.
3. **Consequences.** What changed, what continued, who benefited, who paid,
   and what evidence survives.

The dimension should remain a large physical world. Its people must not exist
only to deliver a lecture. A good scene works as a platformer scene before
its historical lesson is understood.

### A recommended in-world rule

Treat History as an **atlas made into terrain**, not as a simulation in which
the player changes Earth's past. Regions from different periods coexist in
this dimension. Crossing a boundary moves the traveler to a different
historical setting. It does not rewrite the fate of the people in another
region.

This choice fits the existing Clio, Lucy, Mary Leaky, and historical parodies.
It also avoids a separate time-travel system. It remains a proposal: the
world's exact origin and the degree to which residents know its nature can
stay open.

Clio is a guide to sources and their limits. She is not an all-knowing
speaker who settles every dispute. Her archive can compare accounts. Facts,
interpretations, propaganda, folklore, and intentional comedy must remain
distinguishable.

## 3. Three maps, not one rectangle

The existing grid gives authors stable coordinates. Keep that useful tool.
Do not require every new location to occupy one cell.

| Map | What it represents | What it must not imply |
| --- | --- | --- |
| **Period map** | Loose order of historical settings; east often means later | Every society changes at the same date or pace |
| **Place map** | The physically walkable region, with rivers, climbs, roads, docks, rooms, and loops | Two countries were geographical neighbors because their rooms touch |
| **Contact map** | Recorded links: trade, migration, conquest, ideas, crops, disease, and travel | Every connection was a direct journey or a cause of every later event |

The place map stays playable. The contact map explains why some paths exist.
Only some contact links need physical passages. A long-distance historical
connection may instead use a ship route, an archive, a matched artifact, or
an account from a traveler.

### Geography rules

- Keep the African opening and `DYNASTIC` gate. Keep existing room IDs unless
  a separate migration plan has good reason to change them.
- Use broad eastward order for time. **Do not use the global x position as an
  exact date.** Each room can have a local date range.
- Branch from Africa into several historical migration routes. The current
  single ladder chain is a navigation device, not a literal migration map.
- Keep parallel societies visible. Use vertical and diagonal links to show
  contact within overlapping periods. A vertical link is not enough proof
  of a real contact; check the route before writing its explanation.
- Build compact clusters, tall climbs, wide rivers, inland branches, island
  chains, and dense towns. Do not fill all blank cells in a row.
- Make loops and return paths inside important clusters. A long corridor
  must offer a shortcut after discovery. Use existing local travel systems.
- Do not create a new dimension for a time period. History remains one
  connected topological dimension, linked to the other worlds by gates.
- Do not treat an empty row or age as a time when no people lived there.
  Empty space in the plan means only that the content is not authored.

### Kinds of connection

Use these terms in design notes before any schema work:

- `succession`: a later setting in the same broad place.
- `contact`: a route between societies that had a documented connection.
- `diffusion`: a practice or object moved through several intermediaries.
- `comparison`: two societies are shown side by side for study, but there
  is no claim that they met.
- `atlas_shortcut`: a fictional route that keeps traversal practical. Make
  its fiction clear through the space, signs, or Clio's notes.

A contact should not automatically be a climb or a door. For example, the
maritime Silk Roads were many land and sea routes, not one merchant's direct
trip from China to Europe.

## 4. Time coverage: horizons, not fixed columns

These are approximate design bands. They overlap. Local histories use their
own dates. A date band is not a global stage of development.

| Horizon | Approximate span | Questions the world can explore |
| --- | --- | --- |
| Human origins | about 300,000 to 50,000 years ago | Different early human populations; climate, care, tools, language, food, mobility |
| Dispersals and foragers | about 70,000 to 12,000 years ago | Several routes across Africa and beyond; adaptation, seafaring, art, settlement |
| Many food revolutions | about 12,000 to 3,000 BCE | Crops, herding, fishing, storage, land, inequality; independent origins |
| Cities, writing, and Bronze Age networks | about 4,000 to 1,200 BCE | Different states, cities, workers, accounting, trade, and collapse |
| Classical and regional empires | about 1,200 BCE to 500 CE | Republics, kingdoms, roads, law, philosophy, faith, urban and rural life |
| Postclassical networks | about 500 to 1450 CE | Sahel, Indian Ocean, Silk Roads, Americas, Pacific, Islamic scholarship, Asian states |
| Oceanic and early modern change | about 1400 to 1750 CE | Maritime exchange, colonialism, disease, land rights, slavery, new states |
| Revolutions and industry | about 1750 to 1914 CE | Labor, abolition, independence, steam, rail, capital, imperialism, resistance |
| World war and mass politics | about 1914 to 1945 CE | World wars, genocide, depression, liberation movements, state power |
| Decolonization and Cold War | about 1945 to 1991 CE | Independence, Partition, reconstruction, civil rights, science, energy |
| Connected contemporary world | about 1991 to present | Globalization, the internet, migration, water, climate, public health, institutions |

The built C0-C7 scheme is a coarse first view of these horizons. Do not
rename or reindex its cells just to match this table. Add local branches and
new areas only as the stories need them.

**Deep-time choice:** Lucy lived about 3.2 million years ago, long before
Homo sapiens. Keep her already-authored appearance, but treat it as a
clearly marked deep-time encounter or Clio-led interpretation. A later
human-origins branch could extend the dimension much farther into the past.

## 5. World coverage map

This is an idea pool, not a fixed room quota. **Built** refers to the
current scaffold. **Candidates** are not authored content.

| Region or network | Built anchors | Strong candidate additions | Longer-range coverage |
| --- | --- | --- | --- |
| African origins | Cradle, southern coast | Other African early-human landscapes; multiple migration branches | Climate and population history without a single-origin-point myth |
| Northeast Africa and Nile | Green Sahara, Nile, Kush/Aksum | Nubian states in their own settings; Red Sea ports; Ethiopian highlands | Mediterranean and Red Sea networks, modern Egypt and the Horn |
| West Africa and Sahel | Timbuktu, Benin City | Inland Niger towns, trans-Saharan caravans, Songhai | Kongo, Atlantic coast, abolition, independence |
| East and southern Africa | No dedicated Kilwa or Zimbabwe room | Kilwa and the Swahili coast; Great Zimbabwe | Southern African societies, African-led trade, colonial rule and resistance |
| Southwest and Central Asia | Levant, Crescent, Uruk, Classical Shore | Achaemenid road; Baghdad; Sogdiana; Ottoman city | Persia and Iran, Central Asian states, twentieth-century changes |
| South Asia | Coastal route, Mehrgarh, Indus | Ashokan roads; Nalanda; Chola ports; Mughal city | Labor, partition, independence, modern urban societies |
| Southeast Asia | No dedicated row | Dong Son, Srivijaya or Malacca, Angkor | Java, mainland states, colonial and modern Southeast Asia |
| East Asia | Yellow River, Shang, Silk Road, steppe | Han or Tang-Song urban life; Hangul; Heian Japan | Ming/Qing, Meiji, colonial Korea, modern China, Japan, Korea |
| Central Eurasian steppe | Mammoth steppe, Mongol steppe, Great Ger | Sogdian networks, Yam relay hubs, Central Asian crossroads | Turco-Mongol and Russian-era transformations |
| Pacific and Sahul | Sahul, New Guinea, Lapita, wayfinders, Rapa Nui | Tonga, Samoa, Aotearoa, Hawaii | Living navigation traditions, imperialism, modern Pacific states |
| Mesoamerica and Caribbean | Maize, Olmec, Maya, Taino | Tenochtitlan, Haiti / Saint-Domingue | Independent Indigenous continuities and modern Caribbean societies |
| Andean South America | No dedicated Andean path | Caral, Moche/Nazca, Tiwanaku/Wari, Inca roads | Colonial Andes, revolutions, Indigenous continuity, Amazonian societies |
| North America | Beringia | Cahokia, Ancestral Puebloans, Haudenosaunee | Arctic, Plains, Northeast and Pacific coast societies; colonialism and modern history |
| Europe | Ice Age, farmers, megaliths, Celts, Vikings, sail, industry | Black Death, print and reform, Enlightenment, revolutions | Labor, fascism, world wars, reconstruction, European integration |
| Indian Ocean (cross-region) | Separate South Asia and African/Pacific points | Kilwa to Arabian and South Asian ports; Southeast Asian straits | Monsoon systems, languages, commerce, empire, migration |
| Atlantic (cross-region) | Benin, Taino, sail | Atlantic slave trade and its resistance; Haitian Revolution | Abolition, Atlantic economies and migration |

**Coverage policy:** Do not equate the number of rulers with representation.
A trade city, farm, school, workshop, village, or home may explain more than a
palace. Let Indigenous and local people explain their own places. Show living
societies as living, not only at their first encounter with an empire.

### Four cross-sections to make simultaneity visible

Use these dates as design exercises. Put separate scenes beside each other
in the atlas. Do not claim direct contact where none is known.

- **About 1200 CE:** Song China, South Indian maritime trade, Angkor,
  Baghdad, Cahokia, Swahili ports, the emerging Great Zimbabwe settlement,
  and Pacific navigation. The subject is a world of many centers.
- **About 1500 CE:** Ming China, West African kingdoms, Ottoman cities,
  Malacca, Benin, the Inca state, Mexica Tenochtitlan, and Taino Caribbean
  communities. The subject is contact, as well as the lives that continued
  outside those contacts.
- **About 1850 CE:** Industrial cities and workers; independent Haiti;
  India under company rule; Qing China; Tokugawa Japan; African states;
  and Latin American republics. The subject is unequal change.
- **About 1960 CE:** Decolonization in Africa, civil rights movements,
  Cold War pressure, new states in Asia, and advances in medicine and
  communications. The subject is agency under new global institutions.

These cross-sections are reference plans, not four mandatory mega-levels.

## 6. Historical processes that can become game systems

A region needs a physical activity. Prefer activities that model the work
people did. Reuse platformer movement, contacts, switches, vehicles, item
custody, dialogue, and persistence before asking for new mechanics.

| Thread | Example scene or mechanic | Historical idea | Care point |
| --- | --- | --- | --- |
| Food | Water channels, crop terraces, storage, seed exchange | Farming arose in many places; it changed health and property | Avoid one worldwide invention point |
| Navigation | Coastal crossing, stars, monsoon seasons, port calls | People connected places by land and sea | Credit local navigators and shipbuilders |
| Money and debt | Granary tally, weighed goods, tax claims, gifts | Records and obligations change power | Do not claim all writing came from one receipt |
| Information | Manuscripts, printing, translated texts, archive comparison | Knowledge has many makers and routes | Do not turn one scholar into a sole inventor |
| States | Roads, canals, city walls, courts, tribute | Institutions coordinate and compel | Include workers and the governed |
| Commerce | Caravan relay, port markets, prices, supply disruptions | Trade creates gains and unequal costs | Do not equate trade with peaceful contact |
| Disease and climate | Seasonal hazards, lost harvests, response networks | Environment and public health shape change | Do not use victims as loot or comic enemies |
| Coercion and resistance | Escape routes, mutual aid, surviving archives | Enslavement, empire, labor struggles, and resistance | Center affected people, not perpetrators |
| Industry and energy | Mill, boiler, workshop, rail, urban air | New energy systems reshape labor and ecology | Keep the Luddites' demands fair |
| Modern institutions | Journalism, civil rights, labor, independence | Participation, exclusion, law, reform | Show disagreement without false equivalence |

Make cross-era threads visible through recurring objects. A crop, book,
compass, coin, route map, or machine can appear in several rooms. The object
should change its social meaning in each place. The player does not need to
collect every example.

### Clio's archive: a long-term quest framework

Clio can present several accounts of one event. The player can find a
source, inspect an artifact, meet a witness, and learn which claims survive
comparison. A new finding can update the atlas, a route annotation, or a
character's account. **It must not change the real past.**

Use three source roles when a scene needs an evidence puzzle:

1. A contemporary document, artifact, image, or oral account.
2. A different perspective, including people missing from official records.
3. A later interpretation that the player can compare with the first two.

Where facts are established, do not turn them into arbitrary equal opinions.
Where scholars disagree, label the open question and the kind of evidence.
The current dialogue can stay provisional until a focused narrative pass.

## 7. Expansion waves

**Build a few finished places at a time.** A wave is a group of connected
places that has a reason to exist, not a promise to complete a whole period.
The existing A-I topic inventory remains valid. The order below is a
suggestion for world coverage and strong connections, not a selected queue.

| Wave | Small initial slice | Why it earns space | Main connection |
| --- | --- | --- | --- |
| 0 - Atlas pass | Existing 43 rooms, no new room required | Clean period labels, map claims, and current route assumptions | A trustworthy foundation for later work |
| A - Atlantic histories | A West African port or archive; Atlantic crossing context; Saint-Domingue / Haiti | Enslavement, resistance, revolution, abolition and their long effects | Benin and Taino anchors, then Haiti; sober presentation |
| B - Andes and American cities | One tall Andes region; Tenochtitlan; later Cahokia or an Ancestral Puebloan area | Independent city building, agriculture, ecology, roads, and governance | Mesoamerica / Andes / North America as distinct paths |
| C - Persia and learning | Achaemenid route; Baghdad library and translation district | State roads, scholarship, algebra, and exchange | Crescent to Central and South Asia |
| D - Indian Ocean | Kilwa port; South Indian port; Srivijaya or Angkor | Maritime Africa and Asia as historical centers | Monsoon routes, with indirect links made explicit |
| E - East Asia | Song printing city; Heian literary district or Korean Hangul court | Printing, literacy, statecraft, and literary production | East Asia to steppe, Korea, Japan and maritime routes |
| F - Africa inland and North America | Great Zimbabwe; Kongo or another regional anchor; Cahokia | Major regions missing from the current geography | Inland roads and rivers, not only sea ports |
| G - Pacific | Tonga/Samoa or Aotearoa; Hawaii later | Long ocean history and living navigation | Island chains and returning sea routes |
| H - Revolutions and labor | A European print/workers' quarter linked to Caribbean or Latin American independence | How rights and industry spread unevenly | Atlantic and Industrial Age comparison |
| I - Twentieth century | Two carefully researched regional scenes, then widen coverage | Wars, genocide, decolonization, Partition, Cold War and civil rights | Several continents shown concurrently |
| J - Contemporary | A networked city and a climate or public-health region | Global connections and unresolved problems | Continuity with earlier energy, trade and migration stories |

**Do not turn A-I into 9 giant parallel projects.** Choose one wave, make a
small connected slice, and learn what the atlas needs. A wave can remain
partial while another fills a larger gap.

### The first four recommended design packets

The packets below are good world-building targets. Their contents do not
need to enter the executable queue now.

**Packet 0: Make the atlas legible.** Keep all 43 built room IDs. Draft
short period labels, place labels, and boundary notes. Mark which current
connections are historical and which are intentional atlas shortcuts.
Allow the current rooms to remain playable. Do not require new source
code or art to finish this review.

**Packet A1: The Atlantic is more than the Age of Sail.** Develop a small
history of the Atlantic slave trade, with its African, shipboard, Caribbean,
and revolutionary contexts. One sober crossing or archive space can do more
than many combat rooms. Give Saint-Domingue an inhabited town and the
Haitian Revolution a story about the people who won their freedom. The
planned Door of Return can be a symbolic atlas route, not a claim about an
actual ship route. Do not reduce people to cargo, collectibles, targets,
or a puzzle reward.

**Packet D1: An Indian Ocean connection.** Draft Kilwa, one South Asian
port, and one Southeast Asian strait or port. Each has ordinary work,
local political choices, and a distinct landscape. The monsoon controls a
trade route. A reader can see that long-distance exchange used a network
of ports and intermediaries. Use this packet to test a non-rectangular
regional cluster and a safe return path.

**Packet B1: The Andes are a climb, not a blank cell.** Draft an extended
Andean route with mountain terraces, a high road, and an inhabited town.
Start with one specific period, then reserve branches for Caral, Moche or
Nazca, Tiwanaku or Wari, and Inca administration. Treat these as distinct
societies and periods. Connect them by historical succession and shared
landscape, not as one undifferentiated 'Inca' room.

### What can wait

- A full scene for every state, dynasty, and century.
- A single encyclopedic world map with perfect date alignment.
- Fully simulated economies or global population movements.
- New dedicated historical game engines, global time-travel mechanics,
  or another dimension to hold historical philosophy.
- Finished modern war arcs before the tone and source rules are ready.
- Final dialogue and art for every proposed resident.

## 8. One repeatable room or region brief

Before creating an LDtk area, make a brief with these fields. They are
**design-note fields**, not a request for new runtime schema fields.

```yaml
id: candidate_history_kilwa
status: candidate  # built | candidate | deferred
place: Kilwa Kisiwani and the Swahili coast
approximate_dates: 1200-1500 CE
period_relation: overlaps several Indian Ocean trade centers
setting: coast, harbor, stone-town streets and an inland market path
inhabitants: merchants, sailors, craft workers, local families
ordinary_life: boats, food stalls, craft, languages, civic and family life
historical_focus: African-led maritime trade and regional institutions
material_trace: imported ceramics and local architecture
traversal: tide pools, docks, two harbor routes, seasonal sailing
contact_links:
  - Arabia and Persian Gulf ports through intermediaries
  - South Asian port networks through intermediaries
fictional_shortcuts: []
interpretive_mode: grounded_scene  # or parody | myth | archive | comparison
sensitivity: source_review
needs: focused historical sources, people, art, sound, dialogue, test
```

The brief for a completed playable slice should answer:

1. What can a visitor do here without speaking to an NPC?
2. Who lives here, and what do they want today?
3. What physical work or institution can the player observe?
4. What did this place change, receive, or preserve?
5. Which other region does it connect to, and why?
6. Which historical claim needs a source or a disputed label?
7. Is the space safe to discuss through parody, or must it be sober?
8. How does the traveler return without walking a long linear corridor?

**Definition of a good first slice:** A distinct biome, at least two local
inhabitants with different concerns, one everyday activity, one major
historical idea, a route with a loop or return, source notes, and enough
visual and musical identity to be memorable. More enemies are not a
requirement.

## 9. Historical framing repairs to consider before expansion

These are **review notes**, not claims that the current game is broken.
Preserve existing IDs and working interactions until a focused content pass.

1. **Lucy and the Cradle.** Lucy is about 3.2 million years old and was not
   a contemporary of Homo sapiens at 300,000 years ago. Keep her, but mark
   the encounter as deep time, an interpretation, or a separate older area.
2. **The first fire.** Control of fire predates the current Cradle date.
   The authored `history_first_fire` switch can remain, but dialogue should
   frame it as learning to tend or carry fire, not humanity's first fire.
3. **The single point of origin.** Africa is essential to the human-origin
   story. Do not claim that a single modern region contains the one proven
   place where all Homo sapiens began. Use a broad African origin account.
4. **The dispersal ladder.** The present route passes from the Levant into
   Europe and then South Asia and Sahul. That is a compressed game route,
   not a known human migration sequence. Future branches should separate
   these corridors and show the Pacific crossings and Beringian route.
5. **Maize and llamas.** Maize domestication belongs in Mesoamerica; llamas
   are Andean domesticates. The current `history_first_maize` scene combines
   them. Keep that possibility as an explicit comparison or move the llama
   to a later Andean area.
6. **The Green Sahara.** Cattle herding is not the same claim as independent
   cereal domestication. Treat pastoralism and crop farming as different
   innovations with different histories.
7. **Uruk and the Bronze Age collapse.** Uruk's great urban rise and the
   late Bronze Age crises happened far apart in time. The raider encounter
   can stay if framed as a later layer or an atlas comparison, not a single
   contemporary event.
8. **Timbuktu and the coast.** Timbuktu is an inland city near the Niger
   and Saharan routes, not a coastal harbor. The existing
   `history_timbuktu_coast` ID does not need an immediate rename. An Indian
   Ocean port such as Kilwa is a better setting for a dhow and maritime
   trade. Zheng He's voyages also belong with the Indian Ocean network,
   not as a direct visit to inland Timbuktu.
9. **Kush and Aksum.** These are distinct states, places, and historical
   periods. The combined room can act as a regional introduction. Later
   work can separate the Nile and Horn of Africa histories.
10. **The Great Wall.** Different walls were built and rebuilt across long
    periods. Keep Genghis Can and Genghis Can't and both paths. Avoid a
    caption that treats every visible wall section as a single medieval
    construction.
11. **Anachronistic character meetings.** Confucius, Marco Polo, and other
    people sometimes share a room although they lived centuries apart.
    These can be deliberate parody or Clio's guest encounters. Mark that
    frame so the game does not imply they met historically.
12. **A person's first mention is not a people group's first appearance.**
    Avoid 'discovered' unless the viewpoint is named. Show societies before
    and after an outsider arrives, especially in the Americas, Africa,
    Oceania, and South and Southeast Asia.

### Source starting points

Use these as starting references, not the only authorities for a place:

- Smithsonian Human Origins: <https://humanorigins.si.edu/evidence/human-fossils/species/homo-sapiens>
- Smithsonian Lucy specimen: <https://humanorigins.si.edu/evidence/human-fossils/fossils/al-288-1>
- Smithsonian introduction to early human migration: <https://humanorigins.si.edu/education/introduction-human-evolution>
- UNESCO on the Silk Roads as land and maritime networks: <https://www.unesco.org/en/silk-roads/about-silk-roads>
- UNESCO on inland Timbuktu: <https://whc.unesco.org/en/list/119/>
- OpenStax *World History Volume 1, to 1500* maps and timelines: <https://openstax.org/books/world-history-volume-1/pages/b-world-history-volume-1-to-1500-maps-and-timelines>
- OpenStax *World History Volume 2, from 1400* maps and timelines: <https://openstax.org/books/world-history-volume-2/pages/b-world-history-volume-2-from-1400-maps-and-timelines>

For Indigenous histories and living cultures, add focused work from the
people and institutions involved. Do not assume a broad world-history
textbook is enough for a costume, ritual, sacred symbol, or local account.

## 10. Tone, agency, and serious history

The established tone remains: parody of famous people, leaders, and how
history is told. Do not mock a people or faith. No religious founder needs
to appear as a character. Make the ordinary people of a region competent,
specific, and important to its story.

Use three treatments:

- **Comic:** anachronistic rivalry, the boasts of rulers, faulty archives,
  bureaucrats, the competing methods of historians, Genghis Can and Can't.
- **Reflective:** famine, debt, inequality, labor conflict, conquest, and
  contested claims. Show material causes and human choices.
- **Sober:** enslavement, genocide, mass atrocity, forced removal, and other
  large-scale harms. Do not make suffering the joke, an enemy resource, or
  a quiz gate. Show resistance, survival, care, and consequences.

These are not global labels for entire peoples or eras. A region can have
warm daily life and also contain a memorial. Keep the memorial's tone clear.

**Cross-dimension separation:** The Evil dimension can explore fictional
ideology and its institutions. History still needs its own accurate,
serious account of Nazi rule, the Holocaust, World War II, and resistance
when that later work is ready. Math, Art, Science, and Pirate/Ninja worlds
can hold related gameplay and ideas without replacing their histories here.
A gate link between worlds must be an explicit topology decision, not a
shortcut inferred from shared subject matter.

## 11. Selection rules and stopping rules

Do not rank rooms by fame alone. Review each proposal in this order:

1. **Representation gap:** Is this society or region absent, or visible
   only through another society's account?
2. **Historical impact:** Does it explain a durable change or a major
   experience that the atlas otherwise misses?
3. **Inside view:** Can the scene give agency to people who lived there?
4. **Connections:** Does it explain a route, comparison, or consequence
   between existing parts of the world?
5. **Physical identity:** Is there a landscape and an activity worth
   exploring in a platformer?
6. **Production cost:** Can this become one complete scene with reusable
   art, movement, and content systems?
7. **Source readiness:** Can key claims be supported, and can sensitive
   content receive the review it needs?

Use these as a decision review, not a numerical gate. A small culture with
few surviving written records must not score poorly just because a famous
empire left more documents.

A slice is ready to join the game when:

- Its authored rooms form a reachable path with a way back.
- The room and route labels do not create a false migration or chronology
  claim without an explicit fiction marker.
- The ordinary inhabitants have reasons to be present beyond teaching.
- The important physical interaction works using existing capabilities.
- Sensitive material has an appropriate tone and local viewpoint.
- The source notes record important uncertainty or disputed claims.
- The existing `DYNASTIC` arrival and old room IDs still function.

These checks apply when a slice is selected for production. They are not a
reason to prevent exploratory worldbuilding or to reject older content.

## 12. A long view without an enormous first build

A complete History dimension will not be a list of every century. It will
be a connected set of **representative places and historical processes**.
The same story may appear in more than one time and continent. The player
should be able to follow a crop, a book, a ship, a currency, a labor
movement, or an idea across several lives.

A possible long-range completion state is:

- Every major inhabited land mass has multiple periods and local views.
- History before large states has real space, not only a brief tutorial.
- Africa, Asia, the Pacific, and the Americas each have complex histories
  before their contacts with European empires.
- Several dates have playable, concurrent settings in different regions.
- The Indian Ocean, Silk Roads, Atlantic, Pacific, and inland networks
  each feel different and connect multiple societies.
- Industrial and contemporary history are not limited to Europe or the US.
- Players can follow at least a few recurring themes through the whole
  dimension, but do not have to see every room to enjoy it.
- Clio's archive remains incomplete. New evidence and new regions can be
  added without changing the world's basic rule.

**Next design action:** Refine one existing region and one missing regional
connection. Start with an atlas review and a small Atlantic or Indian Ocean
slice. Keep the rest as deliberately open world-building space.
