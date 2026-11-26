# Ambition Editor

Status: active.

Ambition Editor starts with asset inspection and review. The Tools page links
to independent inspection and authoring tools. It shows the same owner commands
as `python3 scripts/agent_query.py tools "<task>"`. Start each service with its
shown command before opening a service link. Commands run from the repository
root. No tool service starts when this page opens.

The asset inspector is the one place to browse and review every asset of an
engine game: characters and their sprite sheets, images, music, sound effects,
fonts and worlds. It shows what exists, what is missing or stale, how to make
it, where the game uses it, and how assets relate, including the design lineage
of characters. Editing assets from the page is planned; the local server is the
place it will write.

Run `./scripts/asset_inspector.py` with no arguments. It opens the last index
in a browser at once and refreshes it in the background, in a separate process
at low priority, so the page stays responsive while it runs. The page offers a
reload when the refresh is done. The first run builds before it opens.
When an open page detects a new art publication or a change to the game pack's
sources, the server refreshes the index and the page shows the Reload button.
The supported launcher is `scripts/asset_inspector.py`.

Only one inspector serves a port (8767 unless `--port`). If one is already
running, a second run says so and exits. `./scripts/asset_inspector.py --kill`
stops the running inspector and its refresh, then serves again. It stops only
an asset inspector, never another program on the port.

While a refresh runs, the server shows the previous index: its catalog, with
the page files (HTML, scripts and styles) of the current code, which `serve`
writes before it starts. A server keeps the code it started with. If the inspector changes while a
server runs, the page shows "The inspector server is older than this page",
because a review can fail to save. Restart the server with `--kill`.

To sort a table, click a column header. Click it again to reverse the order.
Empty values sort last.

A problem never stops the index. If one source cannot be read, the page still
shows everything else. A notice names the problem and its fix, such as
rebuilding a stale content check binary.

## What it reads

Everything comes from the selected library manifest (`library.json`) and game
pack, so another engine game is inspected the same way (`--profile`,
`--library`).

- **Overview** joins the library's producer families (`scripts/content_status.py`)
  with a count of each kind and a list of what needs attention.
- **Characters** and **Inventory** read the sprite publication and the game
  catalog, as described below. A character a pack declares as a Smash fighter
  (a `smash_fighter` source naming it) is tagged `smash-fighter`.
  The Characters panel also has temporary boss entries from the content tool's
  boss art bindings, references, and room placements. Select Role: Bosses to
  see them. Entries that share art have links on their detail pages. A shared
  image does not merge identities. Missing art remains visible; character action
  coverage is not asserted for these entries. This bridge does not change
  gameplay. Remove it after the
  [character construction unification](../../docs/planning/engine/boss-character-unification.md).
  Set **Lineage items >** to a whole number to show entries with more than that
  many distinct historical looks. The count includes variations, milestones,
  and earlier lineage branches. It excludes the current published preview and
  counts each commit/target pair once. A blank field shows all entries.
  Sort characters by name, date added to the game, or art generation date.
  Both date sorts offer newest and oldest first. Added dates use the first
  character key in available catalog Git history, across file renames. Boss
  profile dates use the same lookup in the boss profile sources.
  Generation dates use the latest render receipt for the character's art units.
  Without a receipt, the page uses the latest published image file date and
  labels it as a file date. A copy or restore can change that date.
  Unknown dates sort last. A group uses its main character's added date and
  the latest generation date across its forms and parts. Use All catalog entries
  to sort each entry by its own dates. The page build reads catalog history;
  ordinary status queries do not.
- Every image asset has one top-level category, each with its own tab: Props,
  Held items, Effects, Backgrounds, Terrain & tiles, Entities & pickups,
  Icons & UI and Reference art (`asset_index/categories.py`). The renderer
  target's category decides unless a more specific family does (a strike effect
  is a `props` target), then the file's folder. What no rule places is **Other**,
  counted on the overview so a gap is visible. Reduced-tier copies count with
  their originals. A background layer is used where a room's parallax theme
  names it. **All images** lists everything.
- **Music** reads the music registry, the music tool's active scores, its newest
  renders and its review files. A song's page shows its score's description and
  notes, key, tempo, meter and sections, and each instrument with the sound that
  plays it: an SFZ or SoundFont sample library, a Surge XT patch, procedural FM,
  or a General MIDI program. A registered track the game lacks is missing;
  a score no registry names is unregistered. The Added column uses the first
  score commit in available Git history, through detected renames. A music review made before the
  score last changed is marked. The player at the bottom of the page draws the
  sections of the file it plays from that file's `CHAPTERnnn` comments (the
  chapters VLC lists), shaded by the score's energy; click a section's name to
  start it, anywhere else on the strip to play from there, and press `[` / `]`
  to step between sections. The score's Form table plays from any section. The
  served page answers byte-range requests, which seeking needs.
- **Sound** reads the sound bank and the sound tool's sources. The page plays
  each clip, extracted from the bank.
- **Fonts** reads the font policy and the downloaded files, and draws a sample.
- **Worlds** reads the pack's LDtk worlds: each room's music, parallax theme and
  biome. A room's music links to its track. A world page draws a map of its
  rooms at their world positions and a preview of each room (collision, water
  and climbable cells and entities), with the LDtk tool's room renderer.
- **Uses** come from quoted ids and asset paths in game and engine text, and
  from world rooms.

Every asset can take a review: characters (bosses too), images (props, effects,
backgrounds, tiles, icons and held items), music, sound effects, fonts and
worlds. All reviews use one format. See [Asset reviews](#asset-reviews).

The Characters view needs the content check binary. The inspector uses the
newer of its release and debug builds. When the binary is missing or older than
an engine Rust source, a build or the served page's refresh rebuilds it in
release mode, then reads the catalog again. `--no-cargo` turns that off. By hand:

```bash
cargo build --release -p ambition_content_check --bin ambition_content_check
```

It needs no window, graphics device, or game host.

## Common queries

### Prepare assets for a scenario

The preflight reads prepared content bindings and producer declarations. It can
list products before any generated files exist. The CLI and the editor's
**Tools → Prepare assets** page use the same result.

After you pull engine changes, update the authoring submodules. A pull of the
parent repository can leave an existing renderer checkout unchanged:

```bash
scripts/setup/submodules.sh
```

```bash
python3 scripts/asset_inspector.py preflight --world art --missing-only
python3 scripts/asset_inspector.py preflight --world art --json
python3 scripts/asset_inspector.py preflight --world art --generate-missing
python3 scripts/asset_inspector.py preflight --room primeval_run --purpose headless --check
python3 scripts/asset_inspector.py preflight --room primeval_run --character player_robot_v3 --purpose capture --json
python3 scripts/asset_inspector.py preflight --effect sonic_boom --json
```

Repeat `--world`, `--room`, or `--character` to state the units a fixture loads.
A world can be selected by its manifest id, file name, or file stem. A character
is a catalog id. A capture must name any additional characters it places.
The preflight does not read test source or maintain a separate test asset list.

`--missing-only` shows required products with no accepted publication.
`--generate-missing` builds that selection. It keeps stale accepted products and
partial products. It does not rebuild the library's shared atlases. The plan
reports partial products and unresolved references separately. Resolve reference
errors before generation. `--check` fails when required products are missing
or partial; stale accepted products remain available.

Only required accepted products are checked for availability and freshness.
Optional products can report `not-checked`. A missing required path or effect
row inside an accepted bundle is partial. It does not count as a completely
missing producer unit.

The default purpose is `play`. `headless` requires gameplay products and lists
presentation products as optional. `capture` also requires presentation. Audio
is listed as optional media and is not generated by these requests. A producer
can make several products and quality tiers in one request; the plan names that
producer scope. The T-rex has a separate gameplay target that measures the same
source geometry and publishes its rig without sheet or image payloads.

Build the content tool once, then use it without Cargo for content edits and
asset generation:

```bash
cargo build -p ambition_content_check --bin ambition_content_check
```

Preflight never starts Cargo. It reports a missing or old protocol with the
build command. `--profile PATH --content-binary BINARY` selects another provider.

### Inspect published assets

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
Open the page from disk. It needs no web server. A sheet published part-first
(no page, `drawn_from_parts`) has no strips: the page draws each frame from the
part atlas and the draw table (`parts/<key>/draws.js`, loaded when the sheet is
opened), as the game does, and the build draws its cover from parts. To look
at such a sheet's every frame painted, run `scripts/regen/sprites.sh
--reference --target <name>`. `--no-previews` updates the
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

## Asset reviews

Every review uses the format of the music tool's reviews
(`ambition_music_renderer.music_reviews`). A review contains:

- a score from 1 to 10, with the label of its rubric band: 1–3 Replace, 3–5
  Major polish, 5–7 Acceptable, 7–9 Strong, 9–10 Standout;
- notes and issue tags. Each kind has its own tags, for example
  `animation-motion` for a character and `mix-level` for a sound effect;
- the exact version that was reviewed. A review stays with that version after
  the asset changes. An edit to a review replaces its `current` values;
- optionally, a part: an animation of a character or a sheet, or a room of a
  world. Each part has its own review;
- for audio, how far the reviewer listened.

Music reviews are the music tool's own files, under its `reviews/music/<cue>/`.
The music page saves through the music tool's review code, so its review window
(`review_music.sh`) and this page show the same reviews. The other kinds have
the same keys under the schema `ambition.asset_review.v1`. They are saved in
`tools/ambition_editor/reviews/<kind>/<asset>/<version>[--<part>].review.yaml`.
These files are authored data: commit them. `tests/test_reviews.py` checks that
the scale and rubric agree with the music tool's.

A version is identified by the bytes of the asset:

- a character: its published products, manifest and bundle ids. Unrelated
  catalog edits do not change it. The sheet of a known character is reviewed
  as that character;
- an image: its files;
- a sound effect: its clip, not the whole bank;
- a font or world: its files;
- music: a render (`render_hash` and the bytes of its preview).

The server refuses a review when the bytes differ from those the page showed.
In that case, reload the inspector.

Use `./scripts/asset_inspector.py` (`serve`) to save reviews. A page opened
from disk shows the reviews saved when it was built. **Reviews** shows the work
queue and every review. The work queue holds the newest review of each asset
(or part) with a score below 6, the music tool's polish threshold:

- **fix**: the review rated the current version.
- **re-review**: the asset changed after the review, so look or listen again.

The CLI reads and writes the same files:

```bash
python3 scripts/asset_inspector.py reviews backlog --json
python3 scripts/asset_inspector.py reviews list --kind sfx
python3 scripts/asset_inspector.py reviews add ambition:npc_general --score 4 --part death --issue animation-motion --notes "The cap clips during the fall."
```

`reviews add` rates the version of the last build; for music, the render the
game plays. For an overnight task, read `reviews backlog --json`. For each
`fix` row, read the notes and the issue tags, change the sources, and publish
again. The review of the old version stays as it is. A person reviews the new
version, and the row then leaves the queue.

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
Use `./scripts/regen/sprites.sh --only-possibly-stale` for the shared-only
selection. Add `--plan` to preview it.
See [sprite publication](../../docs/tools/sprite-publication.md) for planning,
source snapshots, exact membership, and unmanaged files. A focused publish
records evidence for that bundle:

```bash
./scripts/regen/sprites.sh --target tower_voss
./scripts/regen/quality_variants.sh --target tower_voss
python3 scripts/checks/check_quality_variants_are_fresh.py
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

Run the inspector tests with `python3 -m pytest tools/ambition_editor/tests -q`.
Renderer tests cover accepted bundles, source changes, missing pages, and failed
conversion acceptance. Sprite staging tests exercise receipt and pixel
publication together. Rust sheet tests cover the original `Death -> Hit -> Idle`
failure and direct death resolution. No test freezes the cast roster.
