# Agent handoff guide

This document is for future AI agents and human contributors who need to get productive quickly without re-learning the entire project history.

## First files to read

1. `README.md`
2. `docs/CURRENT_STATE.md`
3. `docs/GOAL_STATE.md`
4. `docs/adr/README.md`
5. The focused doc for the subsystem you are editing

Historical docs are useful, but ADRs and `CURRENT_STATE.md` supersede older constraints.


## LDtk and world composition

Read `docs/adr/0009-world-composition-and-ldtk-authoring.md` and `docs/ldtk_world_composition.md` before changing sandbox level authoring. The central hub basement is a physical area below the hub, not a separate loading-zone room. The current LDtk adapter composes levels sharing the same `activeArea` field into one runtime active area, and the sandbox also spawns a hidden `bevy_ecs_ldtk` `LdtkWorldBundle` whose `LevelSet` mirrors the active Ambition area. Keep the plugin-owned root hidden until individual LDtk layers/entities are intentionally promoted to typed Ambition runtime bundles; otherwise unregistered LDtk placeholders can render as large dark rectangles over rooms. The old sandbox rooms and feature-lab doors are now represented as LDtk active areas linked by `LoadingZone` entities; do not put the boss directly in the stitched hub/basement area.

Validate LDtk edits with:

```bash
python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

Do not treat LDtk JSON as the canonical gameplay model. Add adapter/validator code when new LDtk entity identifiers or fields become meaningful. The LDtk file should stay first-class editor-shaped JSON: keep root `iid`, `worldLayout`, `defs.layers`, and `defs.entities` in sync with instances. `EdgeExit` zones must be physically reachable: split wall collision around them instead of placing the trigger inside a solid wall. Use `AMBITION_REVIEW(spatial)` around chunk composition, seams, camera bounds, wall openings, and spawn/collision assumptions.

## Repository state and patch packaging

Before producing a patch, verify whether you have a full repo checkpoint or only partial context. A full checkpoint should include the root files, `crates/`, `docs/`, and any relevant assets. If you do not have the full repo state, say so explicitly in the response and keep the patch narrowly scoped to files you can inspect.

Patch zips may contain only modified files to save bandwidth, but name them as patches, for example:

```text
ambition-some-feature-patch.zip
```

Patch zips should preserve repo-relative paths exactly. Crate files belong under `crates/ambition_engine/` or `crates/ambition_sandbox/`; do not create duplicate top-level crate directories such as `ambition_sandbox/`. Documentation belongs under `docs/` unless it is the root `README.md`.

A patch zip cannot reliably delete accidentally created files or directories when applied with `bsdtar -xf ... --strip-components 1`. If cleanup is needed, include an explicit `rm` command in the response.

## Working style

- Prefer small patches with a clear intent.
- Preserve compile logs and user feedback as design information.
- Do not claim `cargo` testing unless you actually ran it.
- Patch responses should include:
  - download link,
  - apply/run commands,
  - what changed,
  - known testing limitations,
  - markdown paragraph commit message.
- When a patch creates or changes a feature, add or update a focused doc.
- When a decision supersedes older guidance, add or update an ADR.

## Environment caveat

Some agent environments do not have `cargo`, `rustc`, or `rustfmt`. If so:

- do not claim compile success,
- do structural/text checks where possible,
- keep patches smaller,
- rely on user compile logs for correction,
- prefer comments/docs/tests that are syntactically low-risk.

## Source-of-truth hierarchy

Use this order when documents disagree:

1. Fresh user instructions in the current conversation.
2. `docs/adr/*.md` for recorded decisions.
3. `docs/CURRENT_STATE.md` for active state.
4. Focused subsystem docs.
5. Historical notes and older patch docs.

If an older doc is misleading, do not delete history by default. Add a supersession note or ADR pointer.

## Spatial reasoning review convention

The project has many geometry-heavy systems where subtle mistakes are easy:

- local/world/Bevy coordinate conversion,
- camera clamping,
- loading-zone placement,
- transition arrival repair,
- AABB strict-overlap vs edge-touch semantics,
- blink shape casts,
- moving platforms and hazards,
- non-Euclidean seams/chart transforms,
- procedural room generation and reachability.

When editing such code, add a nearby comment with this marker if the logic deserves future review:

```rust
// AMBITION_REVIEW(spatial): explain the coordinate/geometry assumption here.
```

Use the marker for code that is correct enough to proceed but would benefit from a stronger spatial-reasoning pass, visualization, or property test later. Do not use it as a substitute for fixing known bugs.

Suggested follow-up searches:

```bash
grep -R "AMBITION_REVIEW" -n crates docs
```

## Testing priorities

Prefer lightweight tests before heavyweight Bevy app tests:

- pure movement step tests,
- collision and blink destination tests,
- room graph validity tests,
- spawn repair tests,
- boss schedule snapshots,
- procedural geometry finite/containment checks,
- RON round-trip tests,
- input-buffer timer tests.

`insta` snapshots are best for reviewable generated outputs. `proptest` is best for invariants over many small randomized cases.

## Data-driven direction

Favor specs that describe gameplay intent:

```text
Enemy(kind: GradientSeeker, attack: TelegraphLunge(...))
```

Avoid making data mirror low-level Bevy bundles unless a presentation system specifically needs that.

## Documentation discipline

- README: stable project portal only.
- `CURRENT_STATE.md`: current truth and known transient areas.
- `GOAL_STATE.md`: long-term direction.
- ADRs: durable decisions and supersessions.
- Patch docs: what a patch changed and why.
- Storyline docs: narrative/worldbuilding candidates.

## LDtk asset loading lesson

Keep `LdtkPlugin` before `init_collection::<loading::SandboxAssetCollection>()`. The asset collection allocates a typed `Handle<bevy_ecs_ldtk::assets::LdtkProject>`, so the LDtk asset type and loader must already be initialized before `bevy_asset_loader` creates the collection.

## LDtk hot reload operational note

For LDtk/world-composition patches, include one pasteable apply/validate/run
bash block. Prefer running the sandbox with:

```bash
cargo run -p ambition_sandbox --features dev_hot_reload --release
```

The runtime supports `F11` to validate/apply the on-disk LDtk project and `F12`
to toggle auto-apply after file changes. Keep the validator as the safety gate;
failed reloads must preserve the currently running world. When adding new map
entities, update both the Rust adapter and `tools/validate_ambition_ldtk.py`, and
make sure hot reload rebuilds the corresponding map-authored runtime state.

## LDtk FieldDef schema shape

Do not only validate entity field definitions. LDtk level fields such as
`activeArea` are also `FieldDef` records and must include the same first-class
schema keys expected by `bevy_ecs_ldtk`, including `allowedRefs`,
`allowedRefTags`, `allowOutOfLevelRef`, `autoChainRef`, `editorDisplayScale`,
`editorLinkStyle`, `editorShowInWorld`, `exportToToc`, `searchable`, and
`symmetricalRef`. A missing key can produce Bevy asset-loader errors even when
Ambition's custom JSON adapter can still read and hot-reload the map.

## LDtk defUid compatibility lesson

Direct `bevy_ecs_ldtk` spawning depends on LDtk instance `defUid` values, not
only on `__identifier` strings. If entity instances use stale `defUid` values
that do not match `defs.entities[*].uid`, `bevy_ecs_ldtk` can panic in
`calculate_transform_from_entity_instance` while spawning a level. Keep entity
instance `defUid`, entity field instance `defUid`, and level field instance
`defUid` synchronized with their definitions whenever generating or patching
`sandbox.ldtk`, and keep `tools/validate_ambition_ldtk.py` strict about this.


## LDtk runtime spine migration

Ambition is moving from a custom LDtk JSON adapter toward `bevy_ecs_ldtk` as the runtime spine. The sandbox now registers every current Ambition LDtk entity identifier as a lightweight plugin-spawned marker bundle, keeps the LDtk world root active, disables LDtk level-background rendering, and records plugin-spawned entity lifecycle in HUD/debug state. The next migration patches should consume those marker entities to attach typed Ambition components, then retire matching portions of the old `LDtk -> RoomManifestSpec -> RoomSet` conversion path.

Official LDtk JSON Schema validation should use Python `jsonschema`, not npm. `tools/validate_ambition_ldtk.py` supports optional `--schema` and `--require-schema` flags while continuing to run Ambition-specific semantic validation without the schema file.

## LDtk / RON ownership update

Do not add new spatial/world content to `assets/ambition/sandbox.ron`. `SandboxDataSpec` no longer has a `rooms` field at runtime; RON owns non-spatial sandbox tuning/audio data only. LDtk owns spatial authoring. If a future patch needs to change rooms, levels, solids, doors, hazards, actors, or debug labels, edit `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk` and update the LDtk validator if needed.

`RoomManifestSpec` still exists as a transitional runtime adapter target, not as the canonical authoring format. Prefer promoting plugin-spawned LDtk marker entities into typed Ambition components over adding more bespoke LDtk-to-manifest conversion logic.

The old checked-in RON room block has been removed from `crates/ambition_sandbox/assets/ambition/sandbox.ron`. Do not reintroduce it as a fallback. If a patch needs fallback/fixture maps, put them in explicit test fixtures instead of the main sandbox tuning/audio manifest.
