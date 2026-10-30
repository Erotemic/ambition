# Asset inspector

The asset inspector is the one place to browse and review every asset of an
engine game: characters and their sprite sheets, images, music, sound effects,
fonts and worlds. It shows what exists, what is missing or stale, how to make
it, where the game uses it, and how assets relate, including the design lineage
of characters. Editing assets from the page is planned; the local server is the
place it will write.

Run `./scripts/asset_inspector.py` with no arguments. It opens the last index
in a browser at once and refreshes it in the background. The page offers a
reload when the refresh is done. The first run builds before it opens.
`scripts/art_inspector.py` is the old name and runs the same tool.

A problem never stops the index. If one source cannot be read, the page still
shows everything else. A notice names the problem and its fix, such as
rebuilding a stale content check binary.

## What it reads

Everything comes from the selected library manifest (`library.json`) and game
pack, so another engine game is inspected the same way (`--profile`,
`--library`).

- **Overview** joins the library's producer families (`scripts/content_status.py`)
  with a count of each kind and a list of what needs attention.
- **Characters**, **Images** and **Inventory** read the sprite publication and
  the game catalog, as described below.
- **Music** reads the music registry, the music tool's active scores, its newest
  renders and its review files. A registered track the game lacks is missing;
  a score no registry names is unregistered. A music review made before the
  score last changed is marked.
- **Sound** reads the sound bank and the sound tool's sources. The page plays
  each clip, extracted from the bank.
- **Fonts** reads the font policy and the downloaded files, and draws a sample.
- **Worlds** reads the pack's LDtk worlds: each room's music, parallax theme and
  biome. A room's music links to its track.
- **Uses** come from quoted ids and asset paths in game and engine text, and
  from world rooms.

A music page can rate a render, as the music tool's review GUI does. It offers
the render the game plays (its preview bytes equal the published file) and the
newest render when that differs. The page saves through the music tool's own
review owner (`ambition_music_renderer.music_reviews`), so the review file, its
score scale and its issue tags are the same, and how far you listened is kept.
A review is refused if the render changed since the page was built.

Any item can take a review note (`serve`). A note records the version of the
files it reviewed, so it stays attached after the asset changes.

The Characters view needs the content check binary. The inspector uses the
newer of its release and debug builds. When the binary is missing or older than
an engine Rust source, a build or the served page's refresh rebuilds it in
release mode, then reads the catalog again. `--no-cargo` turns that off. By hand:

```bash
cargo build --release -p ambition_content_schemas --bin ambition_content_check
```

It needs no window, graphics device, or game host.

## Common queries

```bash
python3 scripts/asset_inspector.py status --current-game --needs-generation --json
python3 scripts/asset_inspector.py inspect npc_tower_voss --animation death --json
python3 scripts/asset_inspector.py coverage --missing death --json
python3 scripts/asset_inspector.py list --kind actor,prop,background,terrain --lineage --json
python3 scripts/asset_inspector.py build --open
python3 scripts/asset_inspector.py serve --open
```

`--profile ambition` selects `game/ambition_content/assets`. To inspect another
provider, pass its pack directory and its prebuilt schema check binary with
`--profile PATH --content-binary BINARY`. `AMBITION_ART_CONTENT_BIN` also selects
the binary. `--renderer-python` selects the renderer interpreter.

Use `--current-game` to limit generation work to referenced bundles and enabled
looks. Without it, the query includes registered units with no known game use.
`--query TEXT`, `--kind LIST`, and `--lineage ID` narrow list, status, and coverage
queries. `inspect ID` also accepts a target, game label, tag, or source trait.
`--animation NAME` limits its coverage table. JSON retains owner warnings and
evidence gaps when a query has no results.

The build writes `.agent/asset_inspector/index.html`, its scripts, and previews.
Open the page from disk. It needs no web server. `--no-previews` updates the
index and names preview paths without drawing new strips. `--jobs N` sets the
preview process count, to a maximum of six. `--prune` removes unused previews.
`--authorship` adds Git blame and model attribution. Normal status queries do
not decode pixels, draw previews, read blame, or walk Git history. `--deep`
rehashes generation inputs and outputs instead of using cached file digests.

## Report and browser

The inventory starts with registered publication plans, prepared identities,
curated concepts and lineages, and local products. A missing bundle remains a
normal record. The prepared provider and catalog ID identify a game character.
The game label and art label remain separate values. Explicit `form_of`,
`part_of`, and `derived_from` relations keep their different meanings. A name
suffix does not establish identity or a generation.

**Characters** groups declared forms and parts under their
character. Expand a group to open each entry. **All catalog entries** shows the
flat catalog. Independent characters remain separate when they share art or
ancestry. Click a card to open its character page.
The page includes animation playback, coverage, current products, forms, parts,
derivations, and a visual design lineage. Normal builds show cached historical
looks in date order, followed by the current published sprite. The recorded
variations remain available when the curated era list is empty. Expand the
timeline to see all variations or curated milestones. Open a thumbnail to
inspect the original historical image. The History inventory view uses the
same visual timeline. Lineage does not need the optional authorship scan.
Product claims select the lineage when a publisher serves several characters.
The history can declare `continues` for a design's next rendering method and
`split_from` for a shared origin. Character pages show the shared origin with
previews and links to the other characters from that origin.
Missing art does not remove a catalog character.
Search by name, catalog ID, tag, family, or lineage. Use the family, lineage,
and art status filters to find related characters or art that needs work.

Character previews use one camera derived from the neutral pose's published
body bounds, visible art, and foot anchor. Transparent room for an attack does
not set the roster scale. The camera includes each declared `rest_row`. This
lets a whole character and its body part show different neutral poses from one
sheet. The camera centers the visible subject. When art extends too far below
physical feet, it uses the visible bottom as its preview baseline and retains
the physical anchor in the framing evidence. The camera and baseline stay fixed
across action frames. Playback and comparison offer Body and Full animation frame views.
Full frame preserves the authored room for movement, weapons, and effects.
The preview reports whether its bounds and feet came from metadata or pixels.
Runtime RON supplies page, trim, frame, body, and foot geometry, including sheets
without a YAML sidecar. YAML remains an independent metadata comparison.

Inventory has Current game, Needs generation, Registered with no known use,
All targets, Concepts, History, Unregistered products, and Live snapshot views.
Expand identities to see forms and parts. Filter by visual family, generator
family, species, body base, naming class, lineage, catalog tags, animation gaps,
and metadata state. Tags carry roles and regions where the game authors them.
The raw gallery, file links, generator evidence, and findings remain available.

A bundle page shows required members, optional claims, quality tiers,
dependents, source support, selected rows, the accepted receipt, lineage, and a
repair operation. Coverage separates source support, profile selection, valid
runtime rows and pages, aliases, and fallbacks. The Rust sheet owner supplies
fallback steps. Authored move clip chains use the same row binding as runtime.
YAML preview rows and frame counts are compared with runtime RON metadata.
A separate corpse row is optional: a death can hold its final frame.

Select bundles to compare one published action at a common phase or elapsed
time. The player supports play once with a final hold, loops, scrubbing, and a
death-to-corpse sequence. Source Studio links select the chargen spec and pose.
Start Studio with `./scripts/chargen_studio.sh --open` before using those links.
A source preview does not replace a missing published clip.

Historical images come from the renderer's design-history cache. The inspector
frames review thumbnails from these images; it does not redraw past art with
today's renderer. Missing snapshots stay marked as unavailable. Prepare missing
historical renders with `python -m design_history build LINEAGE` from the sprite
renderer directory, then rebuild the inspector. Status, list, inspect, and
coverage queries do not prepare or read this visual timeline.

## Character review notes

Use `serve --open` to build the page and start its local review service. On a
character page, write a note, select its priority and optional animation, then
save it. Character cards show open-note counts. **Review notes** opens the
backlog, sorted by priority, with an export for offline work.

Notes are JSON files in `tools/asset_inspector/reviews/notes/`. These are authored
review data. Commit them with other review changes. They are outside the
generated page directory and browser storage. The service writes them at once.
The page opened from disk can show the notes included at build time; use the
service to save changes or read new notes.

Each note records the catalog identity, exact product hashes, manifest, bundle
sources, repair commands, lineage IDs, priority, and optional animation. The
generation key includes required and optional bundle members, including part
atlases. Unrelated catalog edits do not change it. If published bytes change
before a save, rebuild the inspector first. A note on older art stays open
until someone reviews the fix and resolves it.

Read and update the same files from the CLI:

```bash
python3 scripts/asset_inspector.py notes backlog --json
python3 scripts/asset_inspector.py notes add ambition:npc_general --animation death --priority high --text "The cap clips during the fall."
python3 scripts/asset_inspector.py notes resolve NOTE_UUID --resolution "Fixed the cap joint and reviewed the new fall."
python3 scripts/asset_inspector.py notes reopen NOTE_UUID
```

For an overnight task, read `notes backlog --json`, inspect each note's recorded
generation and current bundle sources, make the source edits, and publish the
affected bundles. Rebuild the inspector after publication to compare against the
new generation. Resolve a note after the result has been checked. Backlog art
facts describe the last inspector build; the note files remain live shared data.

## Generation evidence

Game use, registration, generation state, and design history are separate axes.
Game use comes from the selected prepared pack and explicit references in its
worlds. Reports include content fingerprints, world digests, repository
revisions, and local changes. Compiled bindings that an owner does not export
remain outside this reference closure. A registered unit with no known use is
not proof that no runtime path can use it.

The renderer owns the current/missing/partial/stale/unknown decision. Publication
and inspection call that owner. Accepted sprite bundles record source inputs,
source membership, parameters, toolchain versions, and every output digest.
One publication manifest selects immutable bundles and their exact files. Quality conversions
and atlases record their inputs and conversion parameters in the same provenance
root, outside runtime asset roots.

Legacy receipts with no complete input record stay unknown. A known changed
input or changed output can still prove staleness. `--needs-generation` selects
positive missing or stale evidence; unknown generation evidence needs review.

Strictly stale is not always worth a rebuild. A unit stale only because shared
renderer code, the toolchain or the set of renderer files changed is shown as
**possibly stale**. Its page lists the changed shared files, the most specific
first. `--needs-generation` leaves possibly stale units out; add `--strict` to
count them. `./scripts/regen/sprites.sh --loose` rebuilds only directly stale
art, and a plain run rebuilds everything that is not strictly current.
See [sprite publication](../../docs/tools/sprite-publication.md) for planning,
source snapshots, exact membership, and unmanaged files. A focused publish
records evidence for that bundle:

```bash
./scripts/regen/sprites.sh --target tower_voss
./scripts/regen/quality_variants.sh --target tower_voss
python3 scripts/check_quality_variants_are_fresh.py
```

The quality check reports legacy evidence gaps. Add `--require-evidence` to fail
on those gaps. Do not run asset publication with a Cargo build.

Metadata is sparse. Chargen reports source traits and profile rows. The crypto
crew and training equipment supply authored concepts and naming classes. The
other groups can remain unknown. Conflicting facts retain their candidates.
A failed discovery does not classify its omitted products as unregistered.

## Live evidence

An opt-in capture writes a read-only art snapshot beside the image:

```bash
target/debug/capture_scene military_tower player /tmp/art.png --art-snapshot
python3 scripts/asset_inspector.py build --runtime-snapshot /tmp/art.0000.art.json --open
```

Use an existing room ID and the snapshot file the capture produced. The snapshot
contains stable actor IDs, worn character IDs, session and tick, liveness, death
beat, requested pose and clip, selected row and frame, realization, quality,
loaded sheet generation, and build/content identity. The page compares the
loaded manifest fingerprint with local manifests. It does not claim that local
pixel bytes are already uploaded to the live graphics device.

## Validation

Run the inspector tests with `python3 -m pytest tools/asset_inspector/tests -q`.
Renderer tests cover accepted bundles, source changes, missing pages, and failed
conversion acceptance. Sprite staging tests exercise receipt and pixel
publication together. Rust sheet tests cover the original `Death -> Hit -> Idle`
failure and direct death resolution. No test freezes the cast roster.
