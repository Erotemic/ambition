# Data-driven entity and sprite publishing

Status: planning target.
First implementation target: clean asset publishing and runtime install boundaries.

---

## Implementation status (2026-07-02)

**Short version: the *tools and types* exist and work in isolation; the
end-to-end pipeline (regen → publish → runtime consumption) is NOT connected.
`./regen_sprites.sh` does not run ultrapacking or emit a real PublishManifest,
and the game still loads per-target `*_spritesheet.ron` sheets. Nothing in the
runtime consumes a shared pack.**

### Done

- **Publish-boundary hygiene (the first-milestone core).** `PublishManifest`
  and the runtime-root hygiene validator are typed + tested in the Rust
  `ambition_gameplay_core::asset_publish` module (classify / manifest / publish /
  hygiene). `scripts/sweep_runtime_diagnostics.py` relocates author diagnostics
  (canonical poses, preview/debug sheets — 156 of them) out of the runtime roots,
  and **is wired into `regen_sprites.sh`**. The `shipped_runtime_roots_have_no_leaked_diagnostics`
  test fails if a diagnostic reappears under a runtime root. So: *diagnostics stay
  outside runtime roots* — done and enforced.
- **Renderer canonicalization (the enabling refactor, in the renderer submodule).**
  The procedural generators are one canonical `CharacterGenerator` (no adapter
  layer); `TackonTarget`/`AdapterTarget` are one `Target`; `build_sheet` and the
  generators feed one `render_sheet(FrameSource)` core; every generator renders
  each frame independently at any resolution (`frame_source` / `render_all_frames`
  + debug contact sheets + per-frame export).
- **SpritePackCatalog / PackPlan — first pass (`authoring/ultrapack.py`).** Pools
  every target's frames and MaxRects-packs them into shared, uniformly-sized
  atlas pages: **5374 frames from 109/120 targets → 41 shared 2048² pages at 93%
  fill**, plus a catalog (`{page_size, pages[], targets → animation →
  [{page,x,y,w,h,off,src,duration}]}`). This realizes "PackPlan can pack many
  small props + one-frame sprites into shared pages" — as a standalone tool.

### NOT done (the gaps that make it not-yet-usable in-game)

1. **`regen_sprites.sh` does not run the publish/pack step.** It still renders +
   installs *per-target* sheets into `assets/sprites/`. The shared pack is never
   produced during a normal regen.
2. **No runtime `SpritePackCatalog` consumer.** The game reads per-target baked
   `*_spritesheet.ron` (`SheetRecord` via `build.rs` → `BAKED_SHEET_RONS`). There
   is no Rust loader for the shared pages + per-frame `(page, rect, off)`. This
   is the keystone missing piece — without it, ultrapacking cannot ship.
3. **`PublishManifest` is not emitted for real assets.** It's a typed artifact +
   fixture test; the actual install is still a direct copy, not manifest-driven.
4. **`EntityCatalog` — not started.** Gameplay truth still lives in
   `character_catalog.ron` + `SheetRecord` geometry.
5. **11 bespoke targets sit out ultrapacking** (multi-file bosses, tilesets, the
   icon grid, multi-variant modules) — they don't emit a standard single-sheet
   manifest.
6. **Ultrapacking has no locality policy and re-renders to extract frames** (no
   pack-groups; not fed from native `frame_source()`).

### Recommendations (ordered)

1. **Build the runtime `SpritePackCatalog` loader first.** A typed Rust schema +
   loader that resolves `(target, animation, frame) → (shared page, rect, off,
   logical size)`. Until the runtime can *read* a shared pack, the packer output
   is unusable. Keep `SheetRecord` as a compatibility path during migration.
2. **Add a publish step to `regen_sprites.sh`** that runs after per-target render:
   produce the shared pages + catalog, install them into a runtime pack root,
   and emit a real `PublishManifest` recording exactly what shipped. Gate it
   behind a publish profile so dev iteration keeps using fast per-target sheets.
3. **Migrate one runtime consumer** (a simple prop, then a character) from
   `SheetRecord` to the pack catalog to prove the end-to-end path before flipping
   everything.
4. **Fold in the 11 bespoke targets** by giving them uniform frame access
   (native `frame_source()`), which also lets the packer skip the render→extract
   round-trip and pack at pack-optimal resolution.
5. **Then** layer memory-locality pack groups (keep a zone's / always-loaded
   set's frames co-resident) on top of the general packer.
6. **`EntityCatalog`** is the later, separable gameplay-truth migration; it does
   not block shipping the sprite pack.

The dependency to internalize: **runtime consumer → regen publish step →
per-consumer migration.** Everything upstream (packer, manifest, hygiene) is
ready; the runtime loader is what turns it from a tool into shipped assets.

This document defines the target architecture for generated sprite data, visual publishing, runtime asset installation, and eventually uniform entity contracts.

The immediate priority is not to replace the whole character, prop, sprite, and entity runtime in one pass. The immediate priority is to clean up the generated asset pipeline so runtime asset directories contain intentional runtime artifacts only.

The current generated directories are too messy. Runtime assets, diagnostics, previews, sidecars, intermediate manifests, generated sheets, scaled variants, and tool outputs are too easy to mix together. Before the engine can become fully data-driven, the publish boundary needs to be explicit.

The first milestone is:

```text
authoring/generation outputs many things
publish/install step selects runtime things
runtime asset roots contain only installed runtime things
PublishManifest records exactly what was installed
validators fail when diagnostics or accidental outputs leak into runtime roots
```

The broader north star remains:

```text
EntityCatalog is gameplay truth.
SpritePackCatalog is visual storage truth.
PackPlan is quality-specific packing policy.
PublishManifest is the shipping/install boundary.
Diagnostics stay outside runtime assets.
```

---

## Thesis

Every runtime asset should have a purpose.

Generated tools may produce many artifacts:

* source frames
* canonical transparent images
* packed PNGs
* sprite sheet metadata
* measured geometry
* entity-contract fragments
* preview sheets
* labeled debug sheets
* visual diff reports
* authoring diagnostics
* runtime catalogs

But only a subset belongs in runtime asset directories.

The publisher decides what ships. The generator does not directly define the runtime asset root by dumping everything it knows into it.

Runtime systems consume installed catalogs and installed image pages. They should not crawl tool output directories, rely on preview files, or infer gameplay data from diagnostic artifacts.

---

## Immediate goal

Create a professional asset publishing boundary.

This means:

```text
generated artifacts are staged
runtime artifacts are installed
diagnostics remain outside runtime roots
installed files are listed in a PublishManifest
validators check that runtime roots contain only intentional files
```

This cleanup comes before a full runtime migration to `EntityCatalog` and `SpritePackCatalog`.

The first implementation slice should clean the asset pipeline while preserving current runtime behavior.

---

## Current problem

The engine currently has several useful asset concepts, but their boundaries are blurred.

The current runtime visual path is centered around sprite sheet records. These records mix visual sheet metadata with gameplay-adjacent generated geometry such as body metrics, tuning, animation hitboxes, hurtboxes, feet pixels, and frame rects.

The current character and prop loading paths still treat characters, props, one-frame entity sprites, enemies, NPCs, and controlled actors as separate practical classes.

Generated visual tools already produce useful sidecars and diagnostics, but the distinction between runtime-installed data and diagnostic/tooling data is not strong enough.

Visual quality variants already exist, including extreme potato quality, but the eventual target is cleaner:

```text
quality-specific sprite packs
explicit pack plans
runtime install manifest
diagnostics outside runtime roots
```

The immediate cleanup is valuable even before the gameplay schema is fully migrated.

---

## Design principles

### Publish boundary first

Generation and publishing are different stages.

Generation may produce many intermediate artifacts. Publishing installs only runtime artifacts.

The runtime asset roots should be treated as installed outputs, not as a dumping ground.

### Runtime roots are intentional

Every file under runtime asset roots should be either:

* checked-in authored runtime data
* generated runtime catalog data
* generated runtime image page data
* generated runtime manifest data

Files such as previews, debug sheets, measurement reports, visual diffs, canonical source-frame dumps, and temporary diagnostics should not appear under runtime asset roots.

### Manifest the install

The publisher writes a `PublishManifest`.

The manifest records:

* generated timestamp or content hash
* publish profile
* quality profile
* installed files
* source artifacts used
* runtime catalogs installed
* image pages installed
* sidecar/catalog files installed
* diagnostics generated but not installed

The manifest makes accidental asset leakage visible.

### Keep runtime behavior stable during the first slice

The first cleanup pass must not require a flag-day replacement of:

* current character catalog
* current sprite sheet records
* current visual quality switching
* current prop rendering
* current boss sprites
* current entity sprite enum
* current live reload semantics

The first pass creates the publishing spine and proves it with a small vertical slice.

### Contracts, not categories

The long-term runtime model is still contract-driven.

The engine should not ask:

```text
is this a character, prop, enemy, pickup, or projectile?
```

It should ask:

```text
does this entity expose the contract this system consumes?
```

But the first publishing pass does not need to migrate all runtime systems to contracts. It only needs to create the asset pipeline that will support that migration.

### Logical gameplay data is not atlas data

Gameplay geometry should eventually live in entity-local logical coordinates, not atlas pixels.

Sprite packs own visual storage. Entity catalogs own gameplay truth.

The first pass should avoid making the current coupling worse. It does not need to remove every legacy geometry field immediately.

---

## Artifact classes

The target pipeline has four durable artifact classes.

### EntityCatalog

Runtime gameplay truth.

Eventually owns:

* entity ids
* components
* local frames
* volumes
* contacts
* sockets
* timelines
* semantic animation bindings
* presentation references
* tags for tooling and grouping

First-pass status:

```text
planned / optional seed
not required to replace character_catalog.ron yet
```

### SpritePackCatalog

Runtime visual storage truth.

Eventually owns:

* quality profile
* pack id
* page image paths
* visual ids
* clips
* frame rects
* frame page indices
* trim offsets
* frame durations
* render-only anchors

First-pass status:

```text
target name and shape should be introduced
current SheetRecord path may remain as compatibility
```

### PackPlan

Publishing policy.

Owns:

* quality profile
* page size
* padding
* sampling/downsample policy
* grouping by visual id
* grouping by tags
* grouping by zone/load scope
* grouping for always-loaded small objects
* per-quality packing rules

First-pass status:

```text
may be a minimal config or documented placeholder
do not need full atlas repacking rewrite yet
```

### PublishManifest

The immediate first-class artifact.

Owns the exact file set installed into runtime roots.

First-pass status:

```text
required
```

The manifest is the core of the first implementation slice.

---

## Proposed directory model

The pipeline should distinguish four areas.

### Authoring inputs

Human-authored or source data.

Examples:

```text
tools/.../authoring/
tools/.../scores/
crates/ambition_gameplay_core/assets/data/
source art directories
```

### Generated staging

Tool output that may contain runtime candidates, diagnostics, intermediate files, source-frame dumps, previews, and reports.

Target location:

```text
target/ambition_publish/
target/ambition_generated/
tmp/ambition_publish/
```

The exact path can be chosen during implementation. The important rule is that staging is not a runtime asset root.

### Runtime asset roots

Files the game loads.

Current transitional roots may include:

```text
crates/ambition_gameplay_core/assets/sprites/
crates/ambition_gameplay_core/assets/sprites_0_5x/
crates/ambition_gameplay_core/assets/sprites_0_25x/
crates/ambition_gameplay_core/assets/sprites_potato/
crates/ambition_gameplay_core/assets/data/
```

Long-term roots should move toward:

```text
crates/ambition_gameplay_core/assets/data/entities/
crates/ambition_gameplay_core/assets/data/presentation/
crates/ambition_gameplay_core/assets/sprite_packs/<quality>/
```

### Diagnostics

Generated debug output that must not be installed into runtime roots.

Examples:

```text
preview/
reports/
diagnostics/
visual_diffs/
labeled_sheets/
canonical_frames/
tmp/
target/
```

---

## First implementation slice

### Goal

Create an explicit publish/install step and manifest around the existing generated sprite assets.

This pass should clean the generated directory story without replacing all runtime consumers.

### Requirements

The first pass should:

```text
1. Define PublishManifest.
2. Add a publisher/install step that copies selected runtime artifacts from staging to runtime roots.
3. Record installed files in the manifest.
4. Keep diagnostics outside runtime roots.
5. Add validation that known diagnostic files are not installed into runtime roots.
6. Preserve current runtime sprite loading and visual quality behavior.
7. Preserve current generated actor sidecars, but clarify which are runtime-installed and which are transitional.
```

### Non-goals for the first pass

Do not try to complete these in the first pass:

```text
replace character_catalog.ron
remove SheetRecord.body_metrics
replace all SheetRecord loading
migrate all props
replace entity_sprite.rs
rewrite all visual quality generation
introduce full PackPlan atlas grouping
make potato pack every prop into shared pages
move all gameplay geometry to EntityCatalog
remove *_actor.ron sidecars
change live asset reload semantics
```

The first pass makes the install boundary real.

---

## PublishManifest schema

A minimal first schema is enough.

Example:

```ron
(
    schema_version: 1,
    profile: "dev",
    generated_at: "2026-07-01T00:00:00Z",

    runtime_roots: [
        "crates/ambition_gameplay_core/assets/sprites",
        "crates/ambition_gameplay_core/assets/sprites_0_5x",
        "crates/ambition_gameplay_core/assets/sprites_0_25x",
        "crates/ambition_gameplay_core/assets/sprites_potato",
        "crates/ambition_gameplay_core/assets/data",
    ],

    installed: [
        (
            logical_id: "sprite.goblin.basic.high.record",
            kind: "sheet_record",
            quality: "high",
            source: "target/ambition_publish/high/goblin_spritesheet.ron",
            destination: "crates/ambition_gameplay_core/assets/sprites/goblin_spritesheet.ron",
        ),
        (
            logical_id: "sprite.goblin.basic.high.page",
            kind: "image_page",
            quality: "high",
            source: "target/ambition_publish/high/goblin.png",
            destination: "crates/ambition_gameplay_core/assets/sprites/goblin.png",
        ),
    ],

    diagnostics: [
        (
            kind: "preview_sheet",
            path: "target/ambition_publish/diagnostics/goblin_labeled.png",
            installed: false,
        ),
    ],
)
```

The exact fields may change. The manifest must answer:

```text
what did the publisher install?
where did it install it?
what runtime profile / quality was used?
what diagnostics were generated but not installed?
```

---

## Runtime install validation

Add a validator that checks runtime asset roots for accidental diagnostic leakage.

Hard errors:

```text
runtime root contains preview sheet
runtime root contains labeled/debug sheet
runtime root contains visual diff report
runtime root contains temporary dump
runtime root contains known diagnostic metadata
PublishManifest references a missing installed file
PublishManifest says a diagnostic was installed
installed file is outside allowed runtime roots
source path is inside a runtime root when it should be staged
```

Warnings:

```text
installed file not referenced by current loader
runtime root contains legacy file not yet managed by PublishManifest
actor sidecar is installed but not consumed
sprite record contains gameplay geometry still awaiting migration
```

Warnings are acceptable during migration. Broken publish boundaries are not.

---

## Transitional asset layout

The first pass may keep the existing runtime layout:

```text
assets/sprites/
assets/sprites_0_5x/
assets/sprites_0_25x/
assets/sprites_potato/
assets/data/
```

But it should introduce names and manifest records that map cleanly to the future layout:

```text
quality: high
quality: medium
quality: low
quality: potato
kind: sheet_record
kind: image_page
kind: actor_contract_sidecar
kind: entity_contract_fragment
kind: sprite_pack_catalog
kind: publish_manifest
```

Do not require the future `sprite_packs/<quality>/` layout immediately. The publish step should make such a move possible later.

---

## Generated actor sidecars and entity-contract fragments

Generated actor sidecars are transitional but useful.

Current sidecars such as:

```text
*_actor.ron
```

should remain supported if existing tooling emits them.

The forward path is:

```text
*_entity.ron
```

or an equivalent entity-contract fragment.

First-pass requirement:

```text
do not remove *_actor.ron
do not require runtime to consume all entity fragments
do ensure the publisher knows whether a sidecar is runtime-installed, diagnostic-only, or transitional
```

Optional small vertical slice:

```text
emit one *_entity.ron fragment for one generated target
record it in PublishManifest
validate it as a runtime candidate
do not require the sandbox to consume it yet
```

---

## EntityCatalog target

The long-term `EntityCatalog` remains the gameplay target.

It should eventually express entities as contract bundles:

```text
physics.body2d
locomotion.grounded
vitality.damageable
combat.hit_emitter
interaction.inspectable
control.brain
presentation.sprite
inventory.holder
resources.mana
body_mode.morph
```

This supports the current gameplay architecture direction:

```text
controlled body owns capabilities
held items are used by the body holding them
body mode is capability-gated
HUD follows the controlled body
room transitions follow the controlled body
portal guns are held items
```

But the first publish cleanup pass only needs to make room for this model. It does not need to migrate all runtime spawning.

---

## SpritePackCatalog target

The long-term `SpritePackCatalog` should replace visual-storage responsibilities currently mixed into sprite sheet records.

A sprite pack owns:

```text
quality profile
pack id
page image paths
visual ids
clip ids
frame order
frame durations
atlas rects
trim offsets
render-only anchors
```

A sprite pack does not own:

```text
hitboxes
hurtboxes
solid collision
interaction volumes
support contacts
combat timing
controller behavior
entity identity
```

First-pass requirement:

```text
document the target
name installed visual artifacts by manifest kind
do not rewrite all loaders yet
```

---

## PackPlan target

`PackPlan` describes how visual frames become atlases.

It can eventually group by:

```text
explicit visual id
entity tags
visual tags
load scope
room/zone
always-loaded status
frame count
estimated area
quality profile
```

Potato quality should eventually prove the value by packing many small props and one-frame sprites into shared pages.

First-pass requirement:

```text
do not make full PackPlan mandatory
do not regress existing visual quality generation
make the publish step compatible with future PackPlan outputs
```

---

## Publish pipeline target

The eventual publish pipeline has five stages.

### Stage 1: Generate or collect artifacts

Tools produce staged outputs:

```text
visual frames
sheet records
sidecars
entity fragments
diagnostics
preview sheets
reports
```

### Stage 2: Select runtime artifacts

The publisher selects files that belong in runtime roots.

Selection may use:

```text
quality profile
runtime profile
pack plan
known file kinds
explicit install rules
```

### Stage 3: Install runtime artifacts

The publisher copies selected files into runtime roots.

It writes or updates `PublishManifest`.

### Stage 4: Validate runtime roots

The validator checks:

```text
installed files exist
diagnostics are not installed
manifest destinations are legal
runtime roots do not contain accidental generated junk
```

### Stage 5: Runtime loads installed assets

Runtime loaders continue to load from asset roots.

During migration, existing loaders can continue using existing paths. Over time they should consume `EntityCatalog` and `SpritePackCatalog`.

---

## Implementation notes for current code

The current sprite sheet record path is useful and should not be removed immediately.

The first pass should wrap the existing generated asset layout with publishing/install discipline.

Good first targets to inspect:

```text
scripts/generate_visual_quality_variants.py
tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/registry/discovery.py
tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/actor_contract.py
crates/ambition_gameplay_core/build.rs
crates/ambition_gameplay_core/src/character_sprites/
crates/ambition_gameplay_core/src/assets/sandbox_assets/
```

Likely useful existing facts:

```text
current build embeds *_spritesheet.ron from assets/sprites* directories
scaled variants use suffixes such as .0_5x, .0_25x, and .potato
discovery already opportunistically copies *_actor.ron sidecars
SandboxAssetCatalog centralizes path resolution
guardrail tests discourage ad hoc asset walkers
visual quality generation should not blindly resize packed actor sheets
```

The publisher should align with these facts rather than bypass them.

---

## First-pass tests

Add focused tests.

### Manifest tests

```text
PublishManifest parses.
PublishManifest rejects diagnostics marked as installed.
PublishManifest rejects destinations outside allowed runtime roots.
PublishManifest rejects missing installed files when validating a staged fixture.
```

### Runtime root hygiene tests

```text
validator flags preview/labeled/debug files under runtime sprite roots.
validator allows known runtime files.
validator reports legacy unmanaged runtime files as warnings, not hard errors.
```

### Publisher fixture test

Use a tiny fixture staging directory:

```text
staging/
    high/goblin_spritesheet.ron
    high/goblin.png
    diagnostics/goblin_labeled.png
```

Run publisher install into a temporary runtime root.

Assert:

```text
spritesheet RON installed
PNG installed
diagnostic PNG not installed
PublishManifest records installed files
PublishManifest records diagnostic as not installed
```

### Existing runtime preservation test

Run the smallest relevant existing asset/sprite tests to prove current runtime loading still works.

Do not build a huge matrix in the first pass.

---

## Validation commands

Suggested commands for a first implementation slice:

```bash
cargo test -p ambition_gameplay_core publish_manifest
cargo test -p ambition_gameplay_core asset_publish
```

If Python publisher code is added:

```bash
uv run python -m pytest tools/ambition_sprite2d_renderer/tests/test_publish_manifest.py
uv run python -m pytest tools/ambition_sprite2d_renderer/tests/test_asset_publish.py
```

Also run the relevant existing visual-quality or sprite-loader tests if they exist.

Finish with:

```bash
cargo test -p ambition_gameplay_core
cargo fmt --check
```

If a known unrelated app/menu test fails, report it separately.

---

## Migration phases

### Phase 1: Runtime publish boundary — DONE (2026-07-01)

Add:

```text
PublishManifest
publisher install step
runtime-root hygiene validator
fixture tests
documentation of installed vs diagnostic artifacts
```

Keep current runtime behavior.

Landed in `crates/ambition_gameplay_core/src/asset_publish/`:

* `classify.rs` — `ArtifactClass`, the shared brain that decides what a
  generated file *is* from its path shape (runtime vs intermediate vs
  diagnostic). Used by both the publisher and the hygiene validator.
* `manifest.rs` — the typed `PublishManifest` with filesystem-free shape
  validation (no diagnostic marked installed; every destination under a
  declared runtime root) and a staged-source existence check.
* `publish.rs` — the small `install(staging, runtime_root)` step: copies
  runtime-classified files, records diagnostics as `installed: false`.
* `hygiene.rs` — `scan_runtime_root`: diagnostics under a runtime root are hard
  errors, throwaway intermediates are warnings.
* `tests.rs` — publisher fixture + the real-data test
  `shipped_runtime_roots_have_no_leaked_diagnostics`, which fails if a
  `*_canonical` / `*_preview_labeled` / `*_debug` file reappears under a runtime
  sprite root.

The boundary was given teeth immediately: `scripts/sweep_runtime_diagnostics.py`
(the publisher sweep, wired into `regen_sprites.sh`) relocated 156 leaked
diagnostics out of the runtime roots, and the quality-variant generator now
skips diagnostics in its loose-png pass so they stop leaking into the variant
roots. Runtime loaders were untouched.

### Phase 2: Entity-contract fragments

Add:

```text
*_entity.ron fragment emission for one generated target
manifest kind for entity_contract_fragment
validator for fragment shape
no full runtime consumption yet
```

Keep `*_actor.ron` transitional.

### Phase 3: Minimal EntityCatalog runtime spine

Add:

```text
typed EntityCatalog schema
seed actor-like entity
seed prop-like entity
headless parse/validate test
presentation.sprite.visual_id
local frames
volumes
contacts
sockets
bindings
```

Do not replace all spawning yet.

### Phase 4: SpritePackCatalog prototype

Add:

```text
typed SpritePackCatalog schema
one tiny generated pack fixture
quality-specific page/clip data
render-enabled validation mode
```

Keep existing SheetRecord runtime path until a migration slice is ready.

### Phase 5: Runtime consumer migration

Migrate one small runtime consumer:

```text
simple prop presentation
or simple inspectable entity
or generated actor fixture
```

Use:

```text
entity_id -> visual_id -> sprite pack / current compatibility sheet
```

Do not migrate all props/characters in one pass.

### Phase 6: PackPlan and quality-aware packing

Introduce real pack plans and pack grouping.

Prove:

```text
potato quality can pack many small visuals together
high quality can keep larger visuals separate
gameplay geometry is unchanged
```

### Phase 7: Deprecate legacy visual/gameplay coupling

Gradually remove:

```text
SheetRecord gameplay geometry authority
character-specific sprite tables
prop-specific sprite row tables
entity_sprite.rs enum as the only path for one-frame visuals
```

Only remove after replacement consumers exist.

---

## Acceptance criteria for the first pass

The first pass is successful when:

```text
PublishManifest exists as a typed artifact.
There is a publisher/install step, even if small.
Runtime artifacts are selected from staging rather than dumping all generated files into runtime roots.
Diagnostics are kept outside runtime roots.
Runtime-root hygiene validation exists and is tested.
Existing sprite loading and visual quality behavior still work.
Legacy unmanaged files are reported as warnings, not immediate hard failures.
The docs clearly define installed runtime artifacts vs diagnostics.
The next migration step toward EntityCatalog / SpritePackCatalog is obvious.
```

Not accepted:

```text
more generated files dumped directly into assets/sprites*
diagnostics installed into runtime asset roots
publisher copies by broad glob without manifesting installed files
visual quality generation regresses
runtime loaders start ad hoc filesystem crawling
SheetRecord is replaced before a tested compatibility path exists
EntityCatalog migration begins without first cleaning the publish boundary
```

---

## Long-term acceptance criteria

The full design is working when:

```text
A one-frame prop and an animated actor are loaded through the same entity and presentation resolver.
A headless simulation can spawn entities from EntityCatalog without loading PNGs.
Potato quality can pack multiple unrelated visuals into one atlas page without changing gameplay geometry.
Adding a normal prop requires no Rust code.
Adding a normal character using existing behavior requires no Rust code.
Runtime systems consume components, volumes, contacts, sockets, timelines, and bindings instead of entity taxonomies.
Sprite sheet manifests no longer contain authoritative gameplay geometry.
Diagnostics are not installed into runtime asset roots.
Visual quality profiles can change packing topology without changing entity ids or gameplay behavior.
Missing visual data degrades presentation, not simulation.
```

---

## Summary

The target architecture is still:

```text
EntityCatalog is gameplay truth.
SpritePackCatalog is visual storage truth.
PackPlan is quality-specific packing policy.
PublishManifest is the shipping boundary.
Diagnostics stay outside runtime assets.
```

But the implementation order changes.

Do publishing discipline first.

Clean the generated directory mess. Make staging, install, manifesting, and validation explicit. Then migrate entity contracts and sprite packs on top of a clean boundary.

The engine consumes contracts, not categories. The publisher installs runtime artifacts, not diagnostics. The runtime loads intentional assets, not whatever a generator happened to dump.