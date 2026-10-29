# External art library migration

**State:** ACTIVE. Jon selected this work on 2026-10-28.

## Contract

A content library can supply images, geometry, animation, SFX, VFX, music,
and supported script artifacts. A game selects zero or more libraries.
All current games select the shared library under `game/ambition_content_library`.
This library is part of this repository. It is not a submodule.
A new game can use the engine
with that library absent. A content edit must not require an engine rebuild.

Use the existing content compiler, domain schemas, prepared generations,
extension host, and publication bundles. Do not add a second content engine.
Generated assets are not checked in. Source files, runtime products, review
products, and quality tiers have named
roots. The engine owns formats and execution. Libraries own named content.

## Problems addressed

The asset source installer adds an implicit monolith fallback. Engine build
scripts select named art. Sprite metadata, rigs, landmarks, portraits, and
packs share one process-wide publication. Some gameplay lookups bypass
App-local sheet declarations. Manifest lookup discards the source identity.
Source-qualified images lose quality variants. Desktop overrides and package
collision rules disagree. Generation and inspection assume the monolith path.
The publisher also depends on superproject quality code and game-local
validator arguments. The sheet mechanics digest is not in session identity
(the existing Q157 ruling requires it).

## Implementation packets

1. Define library identity, explicit game selection, source roots, and product
   locations. Extract shared assets from the actor crate. Preserve current
   generated bundles and authored data. Track only authored sources and recipes.
2. Route source installation, generation, inspection, embedding, and packaging
   through that selection. Remove monolith-relative roots and named core art
   from engine services. Use one collision and override rule.
3. Make selected sprite metadata an App-local immutable generation. Resolve
   geometry, rigs, landmarks, portraits, packs, effects, and quality products
   from that generation. Remove process-global content selection. Pin semantic
   content through existing session preparation and activation.
4. Include the admitted mechanical projection in content identity. Keep image
   packing and quality policy out of simulation identity. Use the same selected
   library facts for native, static, web, Android, and package products.
5. Verify current games with the shared library, an independent composition
   with it absent, and two Apps with different selections. Verify source edits
   without a Cargo build, failed candidate retention, quality changes, and
   package byte identity. Run the assembled app and workspace policy gates.

## Progress

Implementation checkpoint: `e1ad49163`. Main integration: `a03796df6`.
Acceptance fixes: `24095151e`, now on `main`.
The shared library is optional, selected by game packs, and stored under
`game/ambition_content_library`. Generated products are ignored.

The engine no longer selects named art or embeds the shared library. Source
readers, generation, inspection, static embedding, and packaging use declared
libraries. Qualified references retain their library identity. Ambiguous short
references are refused.

Each App owns an immutable `SpriteContent` generation. Sessions retain their
admitted geometry. Files and metadata advance together. Failed candidates keep
the accepted selection. Q157 is complete: one mechanical projection includes
sheet geometry, rigs, landmarks, and portrait vocabulary. Image packing and
quality products do not change simulation identity.

The follow-up fixes immutable image lookup and qualified FX loading. FX images
and frame rectangles now come from matching tier metadata. Candidate construction
and encounter spawns use the session's frozen generation. The old FX folder
cache is removed.

Verification:

- All workspace test targets typecheck. All 35 workspace policy tests pass.
- The assembled native app passes 21 art-resolution and edit-to-play tests.
  The visible app's two-room HUD activation test passes.
- Provider tests (46), sprite scope tests (7), native reload tests (3), font
  tests (4), and five focused rendering acceptance tests pass.
- Sixty-two focused renderer, publication, provenance, inspector, quality,
  symlink, and package tests pass.
- A source-only checkout simulation lists 466 publication units in 2.69 s.
  It creates no generated directory. Exact packed page membership still needs
  layout resolution.
- The repeated preparation leak is fixed. RSS stabilized near 171 MB across
  three preparations instead of retaining about 140 MB per preparation.
  A weak-reference test verifies release after the last session drops.

## Expanded generated-content scope

Jon confirmed on 2026-10-29 that this applies to every generated game product:
SFX, VFX, music, backgrounds, fonts, sprites, and atlases. Audit each producer's
library selection, source and output roots, runtime consumers, package inputs,
and presence/freshness checks. Verify the complete route from
`run_developer_setup.sh` through the generation setup and family scripts.
Generation must be part of developer setup. Generated products stay ignored.

The first GPU capture rendered the shared scene, but its log found remaining
path assumptions: a game startup collection still lived in the engine, relative
LDtk images did not see declared library imports, and three renderers prefixed
qualified image identities. These fixes are in progress. Named game source
readers must use the same import and collision rules as the default reader.
Loaded relative image handles must reload when the imported publication changes.

## Remaining work

1. Complete the generation phase and all freshness checks. Preserved bundles
   do not prove freshness. Do not commit generated assets.
2. Repeat native and static acceptance after generation. Inspect a new capture
   and verify the independent Outlander package. Measure the page-size failure.
3. Merge new `origin/main` changes, resolve conflicts, publish accounting, and
   push the final receipt.

## Resume

Inspect Git status, this page, and `docs/tools/sprite-publication.md`.
Preserve user edits in the map submodule and `scripts/audit_test_corpus.py`.
The latest gate logs are `/tmp/ambition-library-last-*.log` and
`/tmp/ambition-library-commit-*.log`. The latter batch stopped on an isolated
session fixture's undeclared test feature. That fixture now uses an empty
library selection, which is sufficient for its music-path claim.

The target volume filled during verification. Only this migration's compiled
test binaries and recent incremental sessions were removed. The cleanup record
is `/tmp/ambition-library-build-cleanup.json`. Check target storage before another
large feature-cell build. Never delete another worktree's data.

## Generated-content follow-up

The shared library now declares each producer. Font selection and music naming
policy live under `authored/`. Generated music registry bytes move to the ignored
runtime root. Pack source declarations can select a library by ID. Shared SFX
definitions use this path. Pack embedding, native reads, source watches, exports,
and package input reports use that source owner.

Background themes use authored keys. The library defines alternate states and
mote styles. The engine no longer holds the named theme enum or mote table.
Loose runtime products have named full, half, quarter, and potato roots.
The SFX packer takes explicit input and output paths. The root generator packs
only the selected authored cue IDs. Old rendered cue directories have no
membership authority. The music registry follows authored scores and policy.

Compiler tests (47), theme and renderer tests (23), producer tool tests (18),
font/package tests (8), all 35 workspace policy tests, and all 53 absence
contracts pass. The assembled app and its test targets typecheck. A scratch
background conversion verifies the declared half-tier root and current receipt.

Remaining: complete native/static acceptance checks, move measurement readers to
selected files, run each producer and setup verification, inspect a new capture,
and commit the verified result. The loading fixes and Jon's repertoire changes
are merged and pushed in `3767f3d23`.

## All producer publication

A further audit found that loose media directories still define package
membership. A retired music render can remain in the package after its registry
entry is removed. Use the existing immutable store for fonts, backgrounds,
background variants, music, and SFX. Do not add a second publisher.

Each producer must render into a private staging root, verify its outputs, and
admit one complete selection. A failed or changed-source build must keep the
accepted files. Authored runtime metadata must remain distinct from generated
roots when a publication manages the same logical directory. Required setup
checks must inspect accepted files. Keep generated bytes ignored.

The current packet updates non-image reloads, selected-file diagnostics,
quality receipts, and the independent Outlander package. The package carries
only its own assets. It has no inherited engine asset list. The expanded native acceptance run
passed 22 tests, with one separate GPU retirement test ignored. The static
assembled app check passed.


The media publisher now uses the existing store for every declared family.
Producer roots can name one file or one directory. Authored metadata stays
visible beside generated audio. Font replacement and failed-download tests
verify retirement and accepted-byte retention. Sheet images retain their
metadata generation. Backgrounds and other independent media use live paths.

SFX bank handles stay loaded. A shared bank replaces all borrowing providers in
one transaction. A conflicting bank keeps the accepted providers. The separate
startup SFX cache is removed. All 63 audio core tests, 50 audio adapter tests,
and 35 workspace policy tests pass. The assembled app and its test targets
compile. Render examples now require an explicit library selection.

Remaining: run the complete generation phase and its freshness checks, repeat
native/static acceptance after generation, and verify the independent package.
The selected-file page survey reports 95 full-tier pages above 2048 pixels,
against a recorded limit of 83. Keep this failure visible until the refreshed
output is measured. No generated bytes are committed.


Checkpoint `4fcab904d` publishes every media family through the shared store.
The first full setup run accepted fonts and backgrounds. It was stopped before
sprite publication because full sprite replacement would remove other media
units. The renderer now replaces only target and atlas units. Its replacement
and provenance tests pass (13). The accepted selection was kept.

The music CLI now keeps the caller's Python interpreter for child renders. The
root music batch uses six workers by default. Set `AMBITION_MUSIC_JOBS` to change
that budget. A profile also found a repeated large PNG encode in the giant's
sheet generator. Its alias now copies the encoded body bytes.

Resume generation with `scripts/setup/generated_content.sh --skip-navigation`.
Read `/tmp/ambition-library-full-generation.log` and
`target/regen_assets/latest.jsonl` for the phase result. The interrupted run's
recorded temporary worker paths were removed. Completed immutable bundles were
kept. The complete audio dependency scan passed for 125 cues in 447.5 seconds.
