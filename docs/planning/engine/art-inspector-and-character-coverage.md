# Art inspector and character coverage

**State:** proposed implementation plan, 2026-10-26. Investigation only; the
death-animation defect is still present. Parent programs:
[authoring](authoring-and-tools.md) and
[inspection](inspection-diagnostics-and-workbench.md). The
[existing inspector](../../../tools/art_inspector/README.md) remains the tool.

## Goal

An author can find a character, compare related art, find missing animations,
and explain a fallback without building the game. The same report serves the
offline page and command-line queries. Game data, renderer targets and design
history keep their present ownership. The inspector stores derived results.

## Evidence and present limits

Source examined: superproject `c70860bef889`, renderer `5fe52bfe3272`.
The checkout has other local submodule work. No runtime or authored art was
changed for this investigation. The inspector was rebuilt from local files
with `python3 scripts/art_inspector.py build --no-previews`.

- All seven `tower_*` townsfolk sheets omit `hurt`, `death` and `death_idle`.
  Of the 27 specs under the renderer's `data/townsfolk/`, 25 select the
  default nine-row `townsfolk` profile. Only `armsmith_odile` and
  `village_nagisa` select `full`; their published sheets have all three rows.
- `chargen/sheets.py::TOWNSFOLK_ROWS` omits those reactions.
  `PUBLISH_ROWS` includes them. `targets/characters/townsfolk.py` selects the
  profile from `game.rows`, with `townsfolk` as its default.
- `chargen/motion.py` supplies an eight-frame death and a four-frame corpse
  loop. A fresh source render of Voss shows a fall and a body lying down.
  These poses exist in the source but are absent from the published bundle.
- The shared animation picker selects `Death` for a dead actor
  (`ambition_character_sprites::anim`). The sheet resolver follows
  `Death -> Hit -> Idle` (`CharacterAnim::base_pose`,
  `CharacterSheetSpec::resolve_anim`). Voss has neither reaction row, so this
  resolves to idle. This explains the reported standing body. An in-game
  kill was not replayed in this investigation.
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
   For Voss, both published manifests have nine rows and no death row. Do
   not edit these generated manifests to add a row.
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

## Animation coverage and preview

For each relevant semantic action, show separate facts:

1. Can this source produce it?
2. Does the selected publish profile request it?
3. Does the published runtime manifest contain a valid clip and its pages?
4. Does the prepared body or move request it, and what does the actual resolver
   select, including aliases and fallback steps?

Show direct clip, alias, fallback, missing product and unknown as separate
outcomes. For Voss, the report must explain that source death exists, the
profile excludes it, the published row is absent, and death resolves to idle.
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
2. **Correct the shared publication profile.** Include hurt, death and corpse
   poses in the ordinary townsfolk profile, through the common chargen path.
   Keep optional combat and item rows driven by capabilities. Do not set every
   soldier to `full` or add a military-only renderer. Publish one representative
   first, inspect its bundle and death sequence, then publish affected targets
   and all quality tiers. Verify freshness with the existing quality checker.
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

The death correction needs a real lethal-damage witness through the shared
actor path: death is requested, its clip plays once, and the corpse pose stays.
Exercise a brain-driven body and a controlled body at that seam. A source GIF
proves only source art. Python inspector checks do not prove Bevy rendering;
runtime changes need the narrow owning tests and the assembled-app compile
gate from the [verification recipe](../../recipes/cheapest-sufficient-check.md).

## First deliverable

Select steps 1 and 2 together. The inspector should explain this failure before
the corrected publish makes it disappear. Then build the character browser on
the same inspection model. This keeps the first result useful while broader
concept metadata remains incomplete.
