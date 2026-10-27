# Art inspector improvement plan

**State:** death publication corrected; inventory and inspector changes
proposed, 2026-10-26. Parent programs:
[authoring](authoring-and-tools.md) and
[inspection](inspection-diagnostics-and-workbench.md). The
[existing inspector](../../../tools/art_inspector/README.md) remains the tool.

## Goal

An author can enumerate the art required by a selected game revision and find
exactly which bundles are missing or stale without building the game. The
inventory includes actors, props, backgrounds and their lineages. It also
supports related-art comparison, animation coverage and fallback explanations.
The same report serves the offline page and command-line queries. Game data,
renderer targets and design history keep their present ownership. The inspector
stores derived results.

## Evidence and present limits

Source examined: superproject `d2fd61449c58`, renderer `eaacd737c133`.

**Death publication receipt.** The default townsfolk profile omitted hurt,
death and corpse rows. A dead body therefore resolved `Death -> Hit -> Idle`.
Renderer commit `eaacd737c133` adds `hurt`, `death` and `death_idle` to
`TOWNSFOLK_ROWS`. The existing generated-character pose test exercises all
three. All 25 affected targets were published at all quality tiers. All 27
townsfolk have these rows in 108 runtime manifests and their part tables.
The quality freshness check passes. The existing capture binary killed Voss
through projectile hits and showed the fall and the corpse. Keep the
correction in the common profile; do not add a soldier path
or require the full combat profile for a peaceful body.

Current inspection gaps:

- The rebuilt index contains 234 character sheet assets; 141 have a
  description. These are sheet counts, not character counts. A lineage link
  exists for each of these assets, but only 126 link to a lineage with eras.
  A link alone does not establish complete history or concept metadata.
- The gallery's `Family` facet means asset type, such as character, prop or
  effect. Chargen style, species and body base appear only in related links.
  The search text omits chargen traits and animation names. All human
  species links are suppressed in `related.py`.
- `usage.py` finds references through text patterns. It does not expose the
  catalog's full identity, `form_of`, `part_of`, `derived_from` or tags.
  Voss's card says `Voss`, from its sidecar, while the game catalog says
  `Drill Sergeant Voss` and supplies military tags.
- `related.py` uses both evidence and name similarity. Its generation labels
  depend on sidecar character IDs, catalog-use matches and `_vN` names.
  These are navigation hints, not sufficient identity rules.
- Findings cover provenance, freshness and use. They do not compare animation
  coverage with a body's capabilities. The page loops every preview row,
  including death, and does not show the death-to-corpse transition.
- The index starts with files on disk. An unpublished target or a catalog
  entry without local art cannot be represented as a normal character card.

## Debug without a game build today

1. Run `python3 scripts/art_inspector.py build --open`. Search for
   `tower_voss`, then open its asset page. Its animation list is the
   published output, not the full source motion library. With existing
   previews, `--no-previews --open` can update only the index. New or changed
   strips need a build with previews.
2. Run `./scripts/chargen_studio.sh --open`. Open the Voss example under
   `data/townsfolk/`, then use the pose view to play `death` and `death_idle`.
   Studio reads the spec and builds the full source clip set. It does not
   establish that those clips are published or selected by the game.
3. Compare the selected source profile with the published YAML and RON rows.
   After the correction, Voss has 12 rows, including reactions and death.
   Do not edit generated manifests to add a row.
4. After a source correction, publish one target with
   `./scripts/regen/sprites.sh --target tower_voss`, then rebuild the
   inspector. This command is a publication step, not an inspection step.
   It must run apart from a Cargo build or another asset publication.
5. Use an existing desktop development binary for the runtime check.
   `ambition_sprite_sheet/build.rs` leaves sprite content on disk in
   development builds. `SpriteSheetReloadPlugin`, installed by the app,
   reloads pixel changes and stages metadata changes. A change to simulation
   or cast inputs waits until the session ends. End the session, or restart
   the same binary, if the new rows have not appeared. Builds with static
   content, including web and Android builds, embed this data.

No Cargo command is required for the first three steps. Existing Rust behavior
still needs a Rust build when that behavior changes.

## Separate the relationships

Use explicit relations with source evidence. Do not place all relations in one
untyped family field.

| Relation | Meaning and owner |
| --- | --- |
| Game identity | Provider and stable catalog ID; game catalog and prepared cast |
| Named person or unnamed archetype | Optional authored classification; a display label alone does not decide it |
| Concept | Authored subject or brief, which can have several candidate drawings; character-authoring data |
| Form | Another look of one identity; catalog `form_of` |
| Part | A component of a character; catalog `part_of` |
| Derived character | Independent identity with provenance; catalog `derived_from` |
| Design lineage and era | Curated history, commits, variations and holistic looks; existing `design_history` |
| Visual family | Shared drawing style, such as chargen `crew`; authoring metadata |
| Generator family | Shared implementation or construction method; renderer registry |
| Body base and species | Source traits where the generator supplies them |
| Region, faction and role | Game-authored associations; use catalog tags and other existing fields |

A redesign does not automatically merge game identities. Robot versions remain
distinct characters. A version suffix or common generator does not prove a
form, a shared style or a concept. A human species is a useful filter even
when it is too broad for a related-assets row.

Apply the existing [generation plan](../game/character-generations.md): show
holistic looks first, with all variations available. Do not add another history
registry or decide which old skins ship as part of this work.

## One derived inspection model

Start with the union of catalog entries, registered targets, known concepts,
design-history entries and local products. Join by declared references. Keep
entries whose art is absent. Keep provider namespaces on game IDs.

Each inspected value or relationship carries its source and evidence kind:
authored, derived or inferred. Missing facts remain unknown. Keep these states
separate: unknown, known absent, not applicable, unavailable on this machine,
and conflicting evidence. Do not treat an absent era as an animation defect.

Keep the source value when a display projection chooses another value. The game
catalog supplies the game label; the sidecar supplies its art label. A difference
can be inspected without making either source overwrite the other.

Use owner-produced descriptions for facts that require interpretation:

- Extend renderer discovery with cheap descriptions of source clip support,
  selected publication rows and source traits. Do not render every target to
  inspect it. A target that cannot describe support reports unknown.
- Export catalog and prepared character facts through their Rust owners, using
  the existing content compile path. A small command-line consumer needs no
  window or full game host. Reuse its binary for data edits.
- Export animation names, aliases and fallback resolution from the sheet owner.
  Do not copy the Rust fallback ladder into Python or JavaScript. Initial
  implementation of this exporter can require a narrow Rust build; ordinary
  art inspection must reuse it without a build.
- Reuse design-history joins and records. Remove the inspector's duplicate
  identity and generation guesses when explicit relations cover them. Keep
  name similarity only as a labelled suggestion.

Reports identify input revisions, local changes, source fingerprints and tool
version. Cached exports must state whether they match their inputs. An offline
file scan cannot claim what a running session has loaded.

## Inventory and freshness for all art

The inventory must cover actors, props, held items, projectiles, effects,
icons, tiles, terrain skins, room dressing, backgrounds, portraits and derived
atlases. A character-only catalog cannot answer the game's art requirements.
The unit is a registered product bundle. A background theme has several layers;
a terrain skin has several parts; an actor can use several bundles.

### Existing surfaces and gaps

- `scripts/regen/sprites.sh --check` reports required missing files and one
  fingerprint for the last full publish. The measured baseline has all 1,986
  required sprite files but a stale global fingerprint. It does not say which
  targets are stale or inspect the quality tiers.
- The full publish already has per-target input records and cache keys:
  `read_record.py`, `scripts/lib/sprite_cache_key.sh`, `.cache/reads` and
  `.cache/sheets`. Expose their decisions. Do not write another freshness
  algorithm in the inspector.
- The focused publish branch exits before the full branch sets
  `AMBITION_SPRITE_READS_DIR`. Its provenance can therefore name only the
  target module and declared data, omitting shared motion and drawing code.
  A recorded output digest proves which bytes were published; it does not
  prove those bytes include later changes to unrecorded inputs.
- Registry output claims include optional companions. The script filters
  these with its own `runtime_required` policy. A list of all claims is not
  a correct list of missing required products.
- Backgrounds and terrain already expose product plans
  (`backgrounds.plan`, `parts_plan`, `terrain.plan`) and write provenance
  through `provenance.Book`. Include them through those owners. The sprite
  target registry alone does not enumerate them.
- The inspector scans local products and uses character design history.
  Missing products need records even when no files exist. Props and
  backgrounds also need explicit lineage references where their authors have
  established a history. Absence of that metadata means unknown history.

### Keep use, registration, generation and history separate

| Question | Derived answer |
| --- | --- |
| Does this game revision use it? | Referenced by the selected prepared provider and profile, available as an optional look, registered with no known use, or unresolved |
| Can an owner produce it? | Registered, unregistered, or discovery unavailable/failed |
| Is the required bundle ready? | Current, missing, partial, stale inputs, changed outputs, stale tiers, or unknown |
| What design is it? | Explicit concept, lineage, era, form, part and derivation relations, with unknown values retained |

An old design can be a current game's selectable skin. An unused registered
target is not unregistered. A retired file with no live producer is not a
missing current-game asset. A failed import is not proof of removal.

Do not write `current: true` into generated art. Current use is a relation to
an exact game content revision, provider and profile. Obtain it from the
prepared content reference closure, including world themes, prop bindings,
character forms, portraits and effects. The renderer cannot decide game use.
Keep game revision/content digest separate from renderer revision and source
fingerprints. A newer renderer commit does not make every old output stale.

### Extend the product description and publication record

Have each existing owner export the following facts without drawing:

- Namespaced unit ID, kind, source paths and supported publish operation.
- Required outputs by role and tier, optional outputs and selected parameters.
  Derive this description from the same product plan that publication uses.
- Available source clips, selected sheet rows, actual published clips and
  semantic bindings. Serialize YAML and RON from the same rendered result.
- Explicit concept, visual family and lineage/era references where authored.
  Keep form, part and derived-character relations at their present owners.
  A caller must not infer these from `_vN` or a common module name.

Extend the existing provenance records with a complete generation key:
recorded files, imported drawing code, directory membership, selected render
parameters, effective seed/spec, relevant toolchain versions and product-plan
version. Record the output members and their digests as one accepted bundle.
Quality tiers and atlases name the source bundle digest and their conversion
parameters. Store this evidence in the existing provenance root beside the
runtime assets. Do not add a second registry or copy game catalogs into it.

Focused and full publication must use the same recorder, key and acceptance
path. A failed publication keeps the previous accepted receipt. A cache
backfill without complete inputs remains unknown for source freshness; it
must not gain a claim that it was rendered from today's source.

### Fast questions and focused generation

Extract the existing per-target cache decision into a reusable owner API. Use
it for status, the publication plan and the actual publisher. The inspector
combines owner results; it does not decide how to render a unit.

The fast path discovers units once, hashes each shared input once, and caches
digests for unchanged files. It does not decode pixels, generate previews,
read Git blame or walk old revisions. A separate deep verification rehashes
outputs. Report unsupported owner checks as unknown. Discovery failure makes
the report incomplete, even when the known units are current.

Return structured reasons, affected required members, evidence completeness
and the exact repair operation. A publish of one selected target updates that
target's accepted receipt; it must not claim that the whole game was rebuilt.
Do not regenerate optional historical looks merely because their pixels are
absent. Keep the set requested by the selected game separate from all known
authoring units.

Proposed inspector queries, not implemented commands:

```text
status --profile ambition --needs-generation --json
list --profile ambition --kind actor,prop,background --lineage --json
inspect npc_tower_voss --animation death --json
```

The page should expose the same records as Current game, Needs generation,
Registered with no known use, History and Unregistered products. Unknown and
conflicting metadata remain selectable. Selecting an actor, prop or theme
shows its complete bundle, tiers, dependents, lineage and repair command.

Acceptance: delete one required page, change a shared drawing helper, change
one spec, leave one old skin enabled, add an unreferenced registered target,
and retain an unregistered file. The report must identify each case and name
only the affected generation work. Publication of that work must make the
same owner status current. Use small product fixtures for these behaviors;
do not add roster snapshots or source-text guards.

## Animation coverage and preview

For each relevant semantic action, show separate facts:

1. Can this source produce it?
2. Does the selected publish profile request it?
3. Does the published runtime manifest contain a valid clip and its pages?
4. Does the prepared body or move request it, and what does the actual resolver
   select, including aliases and fallback steps?

Show direct clip, alias, fallback, missing product and unknown as separate
outcomes. A fixture of the original Voss failure must explain that source death
exists, the selected profile excludes it, the published row is absent and death
resolves to idle. The corrected bundle must report a direct death clip.
The inspector also compares YAML preview metadata with runtime RON metadata;
a preview row alone does not prove runtime availability.

Build expectations from prepared capabilities, death/corpse policy, moves and
their clip requests. Do not require every actor to have every fighter row.
Peaceful behavior does not mean a body cannot be hurt or killed. A separate
`death_idle` is optional when the final death frame is the intended corpse.
Old skins retain the generation plan's allowance for visible fallbacks.

The offline player needs play-once, hold-last-frame, loop and frame scrubbing.
Add a death sequence that plays death once, then uses the corpse row or holds
the final frame, as the owner describes. Support comparison of one action
across a selected group, with common phase or elapsed time and visible scale
choices. Keep a raw clip preview available.

Source preview uses the existing renderer and Studio. Published preview uses
the installed bundle. Link between them with target, spec and clip selected;
do not silently replace a missing published clip with a fresh source render.

## Implementation sequence and acceptance

1. **Expose the failure.** Add source publication descriptions, published clip
   queries and JSON output to the existing inspector. Add owner-based fallback
   explanation and a coverage table. Proposed commands are `inspect <id>
   --animation death --json` and `coverage --missing death --json`; these
   commands do not exist yet. Voss and one complete townsfolk target must
   produce different, correct reports without launching the game.
2. **Unify status and publication.** Complete the cross-art owner descriptions
   and receipts above. Make focused and full publication share the recorded
   input path. Expose exact missing or stale units without rendering. Keep
   optional combat and item rows driven by capabilities.
3. **Add the character browser.** Use the derived identity model. Lead with
   game identities, expandable forms, parts and generations. Offer separate
   tabs for targets, concepts, history and raw files. Add facets for visual
   family, generator family, species, body base, role, region, animation gap
   and metadata state. Preserve old looks as accessible records. Voss must be
   reachable from its game name, ID, military region and crew style.
4. **Fill metadata at its owner.** Add sparse concept, naming classification
   and visual-family facts where no existing source states them. Begin with
   chargen, the crypto crew and one unrelated family. Show an explicit unknown
   group for the rest. Do not infer unnamed status from a missing description,
   or make a complete cast annotation a prerequisite for the browser.
5. **Close the runtime evidence gap.** Add an opt-in read-only snapshot of
   stable actor/character identity, worn form/skin, liveness/death beat,
   requested pose/clip, resolved row/frame, realization, quality tier and
   sheet generation. Include session/frame and build/content identity. Import
   it into the inspector to compare live selection with local products.
   Simulation stays authoritative; the snapshot changes no body state.

Use focused behavior tests for owner resolution and publication, and small
synthetic data tests for inspection joins. Exercise forms, independent derived
characters, parts, shared sheets, missing local products and unknown metadata.
Do not add source-text guards or tests that freeze the current roster.

A change to runtime death selection needs a lethal-damage witness through the
shared actor path: death is requested, its clip plays once and the corpse pose
stays. Exercise a brain-driven body and a controlled body at that seam. The
publication correction above changes source art; its in-game witness uses
projectile hits on the brain-driven Voss. A source GIF proves only source art.
Python inspector checks do not prove Bevy rendering; runtime changes need the
narrow owning tests and the assembled-app compile gate from the
[verification recipe](../../recipes/cheapest-sufficient-check.md).

## First deliverable

The shared townsfolk profile correction is the first delivered slice. Next,
select owner status and the missing/stale report. Give actors, props and
backgrounds the same query before adding more gallery controls. Then build the
character browser on that inspection model. Sparse concept and history metadata
must not prevent useful generation queries.
