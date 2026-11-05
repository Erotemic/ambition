# Historical atlases, candidate selection, and source provenance

**Checked 2026-11-05.** Related work for Ambition's History dimension and its
experimental worldbuilding atlas. This page records the external projects we
have compared, the two concrete datasets supplied for inspection, the relevant
attribution requirements, and how source lineage should be preserved if their
records inform future candidates.

## The Ambition question

**How do we assemble a finite, representative superset of world-historical
subjects, with attributable sources, without confusing the reference atlas with
the geography or scope of the playable game?**

The current experimental planner lives in
[`tools/experimental/ambition_worldbuilding_lab/history_atlas.py`](../../tools/experimental/ambition_worldbuilding_lab/history_atlas.py)
and [`data/history_atlas/`](../../tools/experimental/ambition_worldbuilding_lab/data/history_atlas/).
The 2026-11-05 source snapshot has **362 authored nodes**, with **93 built-room
connections** and **466 other links** in `edges.yaml` (559 total). The source
models historical regions, time horizons, statuses, topic and population tags,
and several relationship kinds. Its stated purpose is to reveal missing
historical coverage, not to prescribe all rooms or travel routes. See also
[History dimension](../planning/game/history-dimension.md) and the
[long-range roadmap](../planning/game/history-dimension-long-range-roadmap.md).

The three distinct objects are:

1. **External reference record:** an item in a published dataset, bearing an
   external ID, date range, citations, and reuse terms.
2. **Ambition atlas node:** a deliberate editorial choice to include a subject
   in the finite *candidate superset*, whether built, planned, or candidate.
3. **Game room or episode:** an optional playable implementation chosen from
   that superset. Nothing about an external record or atlas link requires one.

Existing atlas nodes should not be retroactively described as *imported from*
WhenAtlas, Cliopatria, or any other third-party collection: the current YAML
has no per-node external-source provenance, and the datasets listed below were
supplied for this comparison, not merged into it.

## Source register: what has actually been received or consulted

| Source | Status on 2026-11-05 | Use for the candidate superset | Important limitation | Reuse basis |
|---|---|---|---|---|
| [WhenAtlas open dataset](https://whenatlas.com/dataset) | **File received; inspected; not imported** | Named historical entities, time ranges, broad type labels, source IDs | Strong polity weighting; uploaded CSV contains no relationship table or geometries | [CC BY 4.0; preserve record citations](https://whenatlas.com/terms) |
| [Seshat Cliopatria](https://github.com/Seshat-Global-History-Databank/cliopatria) | **File received; inspected; not imported** | Dated polygons and political-geographic coverage audit | Political boundaries, not a general list of historical life; geometries represent interpretations | [Cliopatria CC BY 4.0](https://raw.githubusercontent.com/Seshat-Global-History-Databank/cliopatria/main/LICENSE.md) |
| [History Atlas](https://www.historyatlas.com/) | **Website consulted; no data received** | Bounded editorial inventory and thematic organization | No verified bulk reuse permission or import feed for this assessment | Reference only; do not assume an open dataset |
| [Seshat Global History Databank](https://seshat-db.com/) | **Website/terms consulted; no Seshat table received** | Comparative social and political history; contextual checks | Separate from Cliopatria; do not conflate their licenses | [Seshat Public Data: CC BY-SA 4.0](https://seshat-db.com/terms/current/) |
| [Wikidata](https://www.wikidata.org/) | **Website consulted; no direct extract received** | Entity IDs, aliases, dates, crosswalks, deduplication | Scale and uneven editorial coverage; not a curated candidate list | [Structured data: CC0](https://www.wikidata.org/wiki/Wikidata:Licensing) |
| [Wikipedia Vital Articles, Level 4](https://en.wikipedia.org/wiki/Wikipedia:Vital_articles/Level/4) | **List consulted; no list imported** | Bounded salience comparator across history, geography, technology, everyday life, etc. | Editorial to-do list for English Wikipedia, not a global canon | Text/list content [CC BY-SA 4.0](https://en.wikipedia.org/wiki/Wikipedia:Vital_articles/Level/4); linked structured Wikidata differs |
| [Met Heilbrunn Timeline of Art History](https://www.metmuseum.org/essays/timeline-of-art-history) | **Website consulted; no content received** | Art, material culture, objects, cultural chronology | Essays and photographs have rights separate from open collection metadata | Essays: check rights individually; [some collection data/images are CC0](https://www.metmuseum.org/policies/image-resources) |
| [UNESCO World Heritage List](https://whc.unesco.org/en/list/) | **Website consulted; no list imported** | Historically situated built environments, landscapes, archaeological sites | A nominated/inscribed heritage list, not a comprehensive selection of cultures or events | Check the [rights and license notices](https://whc.unesco.org/en/licenses) for each contemplated reuse |
| [OpenHistoricalMap](https://www.openhistoricalmap.org/) | **License/docs consulted; no data received directly** | Time-aware historical geographic features | Feature-level licensing exceptions; not all history has mapped geometry | [CC0 by default; exceptions flagged](https://wiki.openstreetmap.org/wiki/OpenHistoricalMap/License) |

**Status terminology is intentional.** Being listed here documents *related
work consulted*. It is not evidence that Ambition derived any already-authored
node, room, text, image, map, or code from that source. Update the status and
record-level lineage if that changes. Conversely, a historically familiar fact
does not by itself establish copyrighted-data derivation.

## 1. WhenAtlas: an open historical entity inventory

- **Publisher:** WhenAtlas, *History in Motion*.
- **First-party sources checked:** [dataset](https://whenatlas.com/dataset),
  [about/method](https://whenatlas.com/about), and
  [terms](https://whenatlas.com/terms) (checked 2026-11-05).
- **Downloaded input inspected:** `when-atlas-entities.csv`, supplied by the
  user on 2026-11-05. Its file identity is preserved below; no URL or revision
  ID for the *specific downloaded bytes* was supplied.
- **Published count and local count:** 1,839 entity records; 1,839 distinct `id`
  values. The publisher also advertises **203 relationships**, but the uploaded
  CSV is the *entities* table; it does **not** contain those edges.
- **Observed content:** 1,327 `polity`, 258 `culture_religion`, 95 `unit`, 94
  `conflict`; the other 65 are people, events, periods, treaties, disasters,
  exploration, economy/demography, or eras.
- **CSV fields:** `id`, `type`, `name_en`, `name_zh`, `start`, `end`, `precision`,
  `lon`, `lat`, `prominence`, `source_ids`, `source_licenses`, `updated_at`.
- **Source trail:** `source_ids` and `source_licenses` occur on every inspected
  row. They can identify upstream Wikidata, Wikipedia, and Commons material;
  those labels are *not* a blanket license for copying linked images, text, or
  maps. The dataset's own published terms require retaining per-record source
  citations when reusing the dataset.
- **License:** WhenAtlas machine-readable exports and original written content
  are offered under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
  Credit WhenAtlas, link its dataset or relevant entity page and the license,
  identify modifications, and retain original record-level citations. Embedded
  third-party assets may have additional terms.

**Comparison with Ambition:** WhenAtlas is valuable for finding named subjects
not yet in our superset and for linking potential matches to identifiers. It is
*not* an authority for deciding that all of its 1,839 rows should become
Ambition nodes. In particular, its heavy polity coverage would create a
political-history bias if treated as the superset's baseline.

**Lineage caution:** WhenAtlas says it itself builds on Cliopatria,
OpenHistoricalMap, and Wikidata. Agreement between WhenAtlas and Cliopatria on a
border or dynasty is not automatically two independent historical sources.

## 2. Cliopatria: historical political geography

- **Project:** Cliopatria, part of Seshat: Global History Databank.
- **Primary publication:** Bennett, James S., Erin Mutch, Andrew Tollefson,
  et al. (2025). "Cliopatria - A geospatial database of world-wide political
  entities from 3400BCE to 2024CE." *Scientific Data* 12, article 247.
  [DOI:10.1038/s41597-025-04516-9](https://doi.org/10.1038/s41597-025-04516-9).
- **Dataset publication:** Bennett, J., Mutch, E., and Chalstrey, E.,
  [Cliopatria (Zenodo dataset DOI:10.5281/zenodo.14714684)](https://doi.org/10.5281/zenodo.14714684).
- **Project repository checked:** [README](https://github.com/Seshat-Global-History-Databank/cliopatria)
  and [license](https://raw.githubusercontent.com/Seshat-Global-History-Databank/cliopatria/main/LICENSE.md)
  (2026-11-05).
- **Downloaded input inspected:** `cliopatria.geojson.zip`; its single member is
  `cliopatria_polities_only_v021.geojson`. `v021` is an *embedded file label*,
  not a release tag independently matched to the published Zenodo record.
- **Observed content:** 13,797 GeoJSON features: 13,412 `POLITY` and 385
  `RELATION`, with 1,637 distinct `Name` strings. Dates range from -3400 to
  2024, and individual named polities may occur in many dated records.
- **Observed properties:** `Name`, `FromYear`, `ToYear`, `Area`, `Type`,
  `Wikipedia`, `Wikidata`, `SeshatID`, `Components`, `MemberOf`, plus polygon or
  multipolygon geometry. The `RELATION` type and `Components`/`MemberOf` fields
  describe the dataset's political/geographic organization; they are **not**
  equivalent to Ambition's contact, diffusion, or comparison edges.
- **License:** the **Cliopatria dataset** is [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/),
  with credit, license link and change indication required for reused data.

**Comparison with Ambition:** Cliopatria is useful for auditing which historical
political formations are missing and where regions touch or overlap over time.
Its selection rules favor recorded and sufficiently large, persistent polities;
its paper describes approximate thresholds of 5,000 km² and 50 years with
exceptions. That makes it a poor standalone inventory of mobile communities,
prehistory, informal institutions, households, language, music, craft, and
other subjects the History dimension should also represent. Its borders should
be interpreted as reconstructions, not unique historical ground truth.

**Separate licensing:** Cliopatria's CC BY 4.0 permission must not be replaced
with the terms for **Seshat Public Data**, which the Seshat website currently
states are [CC BY-SA 4.0](https://seshat-db.com/terms/current/). If a future
pass imports a Seshat table rather than Cliopatria geometry, log that dataset
and its share-alike obligations separately.

## 3. Related approaches that complement the two files

### History Atlas: curation and cross-cutting navigation

[Jeff Moran's History Atlas](https://www.historyatlas.com/) presents a large
collection of events arranged into 12 macro-regions, 60 subregions, periods,
subjects, people, groups and commodities. Its site advertises more than 63,000
events. The useful precedent is not that Ambition needs 63,000 nodes, but that
one subject may be found by several conceptual routes while retaining a
specific historical setting. We have **not** confirmed a reusable dataset or
license for bulk copying this site's event selection or text; use it as a
research reference until that is checked.

### Wikidata and Wikipedia Vital Articles: identifiers versus bounded salience

[Wikidata](https://www.wikidata.org/wiki/Wikidata:Licensing) offers CC0
structured identifiers and claims but does not answer which items are worth
representing in a game. It is most useful as an *external identity layer*.
[Wikipedia's Level 4 Vital Articles](https://en.wikipedia.org/wiki/Wikipedia:Vital_articles/Level/4)
is a bounded, actively maintained selection of approximately 10,000 high-priority
articles (the page reported 10,013 on 2026-11-01). It includes history and
other domains important to the atlas. It is a useful editorial comparator, not
an objective ranking of human experiences or an exhaustive list of historical
sites. The page's own list/text license differs from Wikidata's structured-data
CC0 license.

### Seshat, the Met, UNESCO, and OpenHistoricalMap: dimensions of coverage

- [Seshat Global History Databank](https://seshat-db.com/) contributes
  comparative historical social and political research. Cite a specific dataset
  release when actual data is used, rather than attributing every Seshat-adjacent
  source to one umbrella project.
- [The Met's Heilbrunn Timeline](https://www.metmuseum.org/essays/timeline-of-art-history)
  organizes cultural and material history in essays and regional chronologies.
  Open Access [collection metadata and qualifying artwork images](https://www.metmuseum.org/policies/image-resources)
  are distinct from its essays and separately credited media. Do not treat the
  entire Timeline as CC0.
- [UNESCO's World Heritage List](https://whc.unesco.org/en/list/) identifies
  major cultural sites and landscapes, but inscription is not a measure of a
  people's total historical significance. Review relevant rights before
  reproducing records or images.
- [OpenHistoricalMap](https://wiki.openstreetmap.org/wiki/OpenHistoricalMap/License)
  is a useful precedent for time-stamped places, features and map objects. Its
  records are CC0 by default, with some feature-level license exceptions. The
  rule and the exception should both survive any future import.

## Source lineage and attribution design

No change to the atlas YAML schema or the game is made by this document. If we
start programmatic matching/importing, record *which* external records led to a
proposal instead of attributing every existing atlas node generically to an
entire source. A minimal candidate record should eventually be able to answer:

| Field | Example | Purpose |
|---|---|---|
| `source_key` | `whenatlas` | Stable local name for a publisher/dataset |
| `snapshot` | SHA-256 from the manifest below | Exact input edition, not merely a mutable URL |
| `record_id` | `conflict.ww2` | Original machine-readable identifier |
| `role` | `discovered_candidate` | Distinguish suggestion, historical evidence, geometry, illustration |
| `source_url` | `https://whenatlas.com/entity/conflict.ww2` | Inspectable human-facing record |
| `claim` | `date_range`, `polity_boundary`, etc. | The specific field or statement supported |
| `license` | `CC-BY-4.0` | Applicable license for reused material |
| `modifications` | `dates normalized; names reviewed` | Truthful adaptation notice when relevant |
| `editorial_status` | `accepted`, `rejected`, `needs_review` | Import never silently becomes canon |

**A source is not a claim.** A node may need different authorities for its date,
location, archaeological interpretation, peoples, and narrative text. Likewise,
a modern country tag is a discovery convenience, not evidence that the country
or all its inhabitants are represented by that node. Avoid adding false
independence by citing WhenAtlas *and* its upstream Cliopatria-derived geometry
as if they were unrelated confirmations.

**Proposed selection workflow:** compare external records with the existing
finite candidate superset; crosswalk by stable identifiers plus dates and
places; report genuinely missing subjects, duplicate representations and
underrepresented categories; review additions editorially. Never assume
external entity edges imply physical travel or that a match requires a new game
room. Prefer gaps in lived experience, technology, environment, Indigenous
history, and historical interaction over simply maximizing polity count.

## Attribution patterns for *future actual reuse*

These are templates for release notes, data exports, asset manifests or an
in-game acknowledgments page **when the corresponding data is actually
incorporated**; they are not a claim that the current Ambition atlas has already
imported either dataset.

> Historical entity information adapted from **WhenAtlas**, [open historical
> dataset](https://whenatlas.com/dataset), licensed under
> [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). [Describe changes].
> Per-record sources are retained with the imported records.

> Historical polity boundary data adapted from **Cliopatria**, part of the
> **Seshat Global History Databank**; Bennett, J. S., Mutch, E., Tollefson, A.,
> et al. (2025), [*Scientific Data* 12, 247](https://doi.org/10.1038/s41597-025-04516-9),
> licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
> [Describe geometry/date processing and modifications].

If directly using Seshat Public Data, follow its **separate CC BY-SA 4.0**
terms and cite that particular release. If using Wikidata structured claims,
identify Wikidata as a source even though CC0 does not require attribution.
Keep separate asset-level credits for copyrighted photographs, map tiles,
quotations and other media; an open data license never automatically covers
all adjacent assets. Preserve upstream attribution records in exports, not only
in a single aggregate credits paragraph.

## Snapshot register for the two supplied files

These values were measured from the **user-provided bytes**, not copied from a
publisher's changing live count. The files are **not** bundled into this
repository document or its overlay.

| Supplied filename | SHA-256 | File size | Inspected contents |
|---|---|---:|---|
| `when-atlas-entities.csv` | `17236639a5e255a8d18b1254e4f34575a0fa97a7ee2e70d5ce9a5bc2a92e378b` | 370,073 bytes | 1,839 CSV data rows; 1,839 unique IDs; latest populated `updated_at`: 2026-11-04 |
| `cliopatria.geojson.zip` | `e10a4e429fca708788ff9e7572a95fce801fae55852d217cffabc1c59cd4eed4` | 46,005,176 bytes | one member, `cliopatria_polities_only_v021.geojson` (172,854,269 uncompressed bytes); 13,797 GeoJSON features |

The zip member advertises `crs: OGC:CRS84`. Its 1,637 distinct `Name` values
are **not** 1,637 unique time-stamped features: many names repeat across
historical intervals. The 385 `RELATION` features should not be mistaken for
the 203 relations separately advertised by WhenAtlas; the two measures describe
different data models.

Before transforming either source, register the source URL, release/tag (if
confirmed), download date, file digest, fields used, record IDs carried
forward, and notices required by that exact dataset. Do not copy the bulky
original files into ordinary planning overlays.

## Findings and outstanding design pressure

1. **Two supplied datasets can be audited now, but no import has occurred.**
   This document records exactly what was inspected and where it came from.
2. **Existing source overlap is substantial.** WhenAtlas explicitly uses
   Cliopatria, OpenHistoricalMap and Wikidata. Build a lineage graph, not a
   count of nominally independent websites.
3. **A polity inventory is insufficient as the reference superset.** The
   intended historical breadth also includes everyday life, technology,
   migration, art, trade, health, social change and prehistory.
4. **The editorial cutoff is ours.** External databases are evidence and
   candidate generators. Ambition's finite atlas needs explicit selection
   criteria, including representative gaps and redundancy, not a requirement
   to include every source entity.
5. **Rights need to follow actual reused material.** In particular,
   distinguish Cliopatria CC BY 4.0, Seshat Public Data CC BY-SA 4.0,
   WhenAtlas CC BY 4.0 plus record sources, and Wikidata CC0.
6. **Current nodes need eventual per-claim provenance.** That is future
   authoring work, not something to falsify by backfilling blanket citations.

**Decision recorded 2026-11-05:** Keep the worldbuilding atlas as a curated,
finite reference superset separate from playable routes. Keep the two supplied
third-party datasets as *consulted / received, not imported* until specific
record-level transformations actually happen. No code or atlas content changes
follow from this related-work review alone.

## Verified references

All linked source descriptions and license claims above were checked against
published pages or the supplied files on **2026-11-05**. These are research and
rights references; individual historical claims still need their own sources.

- Bennett, J. S., Mutch, E., Tollefson, A., et al. (2025).
  ["Cliopatria - A geospatial database of world-wide political entities from
  3400BCE to 2024CE."](https://doi.org/10.1038/s41597-025-04516-9)
  *Scientific Data* 12, 247. DOI: `10.1038/s41597-025-04516-9`.
- Bennett, J., Mutch, E., and Chalstrey, E.
  [Cliopatria dataset on Zenodo](https://doi.org/10.5281/zenodo.14714684).
  See also the [GitHub repository](https://github.com/Seshat-Global-History-Databank/cliopatria)
  and its [license](https://raw.githubusercontent.com/Seshat-Global-History-Databank/cliopatria/main/LICENSE.md).
- WhenAtlas. [Open dataset](https://whenatlas.com/dataset),
  [method and credits](https://whenatlas.com/about),
  [reuse terms](https://whenatlas.com/terms).
- Seshat Global History Databank.
  [User agreement and data license](https://seshat-db.com/terms/current/).
- Jeff Moran. [History Atlas](https://www.historyatlas.com/),
  [about and source practices](https://www.historyatlas.com/about/).
- Wikimedia Foundation contributors.
  [Wikidata licensing](https://www.wikidata.org/wiki/Wikidata:Licensing);
  [Wikipedia Vital Articles, Level 4](https://en.wikipedia.org/wiki/Wikipedia:Vital_articles/Level/4).
- The Metropolitan Museum of Art.
  [Heilbrunn Timeline](https://www.metmuseum.org/essays/timeline-of-art-history);
  [Open Access and image/data policies](https://www.metmuseum.org/policies/image-resources).
- UNESCO World Heritage Centre. [World Heritage List](https://whc.unesco.org/en/list/)
  and [license reference](https://whc.unesco.org/en/licenses).
- OpenHistoricalMap contributors.
  [OpenHistoricalMap license and attribution](https://wiki.openstreetmap.org/wiki/OpenHistoricalMap/License).
