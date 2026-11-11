---
status: proposed
source_snapshot: 122f90e8
related_docs:
  - docs/planning/game/art-and-ambiance.md
  - docs/systems/parallax-backgrounds.md
  - docs/systems/room-looks.md
  - docs/recipes/headless-room-verification.md
  - docs/recipes/ldtk-authoring.md
---

# Collision-guided environmental composition

## Intent

Make Ambition's traversable geometry belong visually to its environment. A
platform should read as a branch, balcony, masonry ledge, supported dock, or
spectral structure—not as a textured collision rectangle placed in front of an
illustrated background. Preserve the clean, predictable collision surfaces and
allow the artwork to project beyond them, extend behind them, and occasionally
pass in front of the actor.

The visual reference is the environmental layering and silhouette composition
of *Hollow Knight* and *Silksong*, adapted to Ambition's many different visual
languages. Organic silhouette breakup is appropriate in forests; straight lines
are appropriate in cities, laboratories, and engineered structures. The common
requirement is **coherent construction**, not a universal organic skin.

This is an art-and-presentation improvement built on the **existing** terrain
skin, art publication, room authoring, and prop layering systems. It is not a
request for another terrain simulator or generic procedural-world framework.

## What the current screenshots establish

The November 11 gameplay captures provide useful contrasts:

- **Pirate scaffolding and docks:** plank thickness, posts, diagonal braces,
  ropes, and railings already make rectangular platforms read as constructed
  objects. Preserve this approach and reuse the structural reasoning.
- **Red wasteland:** the grass fringe, bushes, stones, and soil fill integrate
  ground with the scene. The broad soil fill is repetitive, but the collision
  edge remains readable.
- **Ornate hotel halls and mathematical gate:** architectural ornamentation,
  arches, banners, doors, and landmarks are effective; long horizontal
  walking surfaces sometimes still resemble repeated trim strips.
- **Spectral hotel wing:** square checkerboard floating ledges contrast sharply
  with the haunted wallpaper, sconces, furniture, and spectral effects. Their
  structure should look native to that room rather than generic test blocks.
- **Pirate-port gate plaza:** the climb to the harbour balcony is visibly a
  stair made from successive rectangular ledges. It should read as one
  architectural connection, with coherent tread/nosing and supporting mass.
- **Cedar forest:** the ground dressing works, but floating wooden branches
  with repeated oval foliage have a shallow, mechanically repeated silhouette.
  Improve the branch/foliage relationship and variation at the scale of the
  whole platform rather than adding more evenly spaced shrubs.
- **City and lab interiors:** the illustrated environments already contain
  strong large-scale structure. Prefer foundations, support joins, beams,
  railings, drains, and material transitions to indiscriminate roughening.

The three acceptance scenes below intentionally represent three different
problems. The successful pirate scaffold and wasteland ground are comparison
references, not things to rebuild.

## Existing ownership and extension points

| Responsibility | Existing owner | What to reuse |
| --- | --- | --- |
| Collision blocks and one-way surfaces | Authored LDtk rooms, game content, core collision | Remain authoritative; art must not change gameplay contact |
| Room theme | `RoomMetadata::visual_profile.parallax_theme` | Explicit theme selection; do not infer it from biome names |
| Exposed spans and edge coverage | `crates/ambition_render/src/rendering/terrain_skin.rs` (`open_edge_spans`, `open_spans`, `lay_room_skin`) | Derive presentation structure from actual surfaces |
| Terrain pictures and loaded theme parts | `crates/ambition_sprite_sheet/src/game_assets/room_dressing.rs` (`RoomDressingPart`, `RoomDressingSet`) | Existing named, published asset identities and residency |
| Terrain mesh publication | `terrain_skin.rs` (`SkinQuad`, `SkinLayer`, `RoomTerrain`, `lay_room_terrain`) | Room-anchored texture coordinates, chunked meshes, dirty rebuilds |
| Decorative exclusions | `TerrainKeepOut`, `TerrainDecorDensity` | Preserve access to doors, spawns, interactive objects, and hazards |
| Authored room objects and art overrides | LDtk props, `EntityArt`, `PropDraw` | Explicit art direction where geometry does not decide meaning |
| Structural foreground/background placement | `ambition_render/src/rendering/world.rs` (`PropDraw::Structure`, `PropDraw::Enclosure`) and parallax/room looks | Respect established scene ordering and view composition |
| Terrain artwork and publication | `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/terrain/` (`skins.py`, `ornaments.py`, `decor.py`, related generators) | Publish textured assets through the selected content library |

The current system already hides covered internal edges, anchors texture
patterns in room space, batches terrain quads, and rebuilds changed chunks when
terrain changes. It also gives individual blocks with `EntityArt` priority over
generated dressing. Preserve these behaviors. Some evergreen prose still
claims newly exposed edges are not re-trimmed after block removal; the current
`lay_room_terrain` implementation explicitly supports that invalidation.

**Dependency direction:** the gameplay/collision authority supplies geometry;
presentation derives visual structure and decoration. Presentation must not
write alternate collision, gameplay material, or traversal decisions.

## Design: structural composition before small detail

A visually convincing surface has three related aspects:

1. **Structural form:** the apparent mass and support relationship—a stone
   staircase, a suspended platform, a root-held shelf, a steel truss, a pier.
   This has the greatest influence on the silhouette.
2. **Surface treatment:** material fill, walkable cap, end treatment, underside,
   corners, joints, damage, or overgrowth. The current system is strongest here.
3. **Depth and accents:** supports and continuations behind the playable
   geometry; selected foreground railings, vines, roots, or posts; restrained
   atmospheric details. These reinforce the structure.

Avoid treating decoration count as a measure of quality. Several harmonious
large forms can be more effective than many small repeated ornaments.

### Geometric derivation: build only what the pilot needs

Start with the existing per-block exposed-edge logic. When a visual needs to
span blocks, derive **presentation-only continuous features**:

- coalesce adjacent collinear exposed spans where material and elevation match;
- identify exterior corners, ledge ends, steps, and interruptions;
- associate nearby faces so one formation can bridge multiple colliders;
- keep useful provenance to the source surfaces for invalidation;
- anchor deterministic variation to room coordinates or stable feature
  geometry, so subdividing one collision rectangle does not reshuffle the art.

This representation may be an intermediate value during `RoomTerrain`
preparation. It does not need a persistent ECS entity for every edge, tile, or
decoration. Do not implement a general polygon/topology library in advance.
Use the minimum grouping required by the first visible artifact.

Not every stair-shaped collection is a staircase, and not every isolated
platform is a balcony. Infer straightforward geometry, but use existing
**authored props/overrides** to select the intended structural identity when
it is ambiguous. Only introduce a small, presentation-only semantic authoring
field if the existing authoring vocabulary proves insufficient. Roles should
be independent of themes: the same `stair_run` or `supported_span` concept can
receive pirate masonry, hotel ornament, or mechanical construction from the
selected art palette. Do not create room- or biome-specific renderer branches.

### Artwork and composition

Prefer strong authored transparent silhouettes over synthetic geometric
noise. Existing rectangular `SkinQuad`s can carry transparent images with
irregular edges, so a new shader or generalized mask is not a prerequisite.
Explore:

- substantial underside pieces, corners, brackets, roots, stone masses, and
  structural continuations that give apparent thickness and support;
- large/medium/small variants with deliberate negative space and asymmetry;
- materials that transition coherently across adjacent blocks and through
  concave/convex corners;
- accents placed as clusters with controlled rhythm, not a repeated item at
  every fixed interval;
- background extensions and occasional foreground details whose depth
  relationship is authored intentionally.

Use `RoomDressingSet` and existing publication/catalog selection for normal
terrain pieces. The current `RoomDressingPart` is a finite named vocabulary;
do not add a new enum case for every variant, structural role, or biome. Use
existing parts and authored props first. If the pilot demonstrates a genuine
need for multiple theme-specific structural variants, introduce a small data
representation in the existing art/presentation ownership boundary, with
minimal additions to asset publication.

### Layering and gameplay readability

`SkinLayer` currently orders quads **relative to their block**. That does not
by itself provide arbitrary foreground occlusion of actors. For selected
large occluders, use or extend the existing world-prop draw-depth vocabulary
rather than assuming another `SkinLayer` value will render in front of all
characters. Respect room/camera selection and existing portal-capture
presentation when applicable; no portal-specific terrain implementation.

Preserve legible landing edges, hazards, one-way-platform tops, doors, and
interactables. Decorative overhangs may extend beyond colliders, but should
not systematically misrepresent where a character can stand, wall-jump, or
pass through. Foreground objects may partially hide an actor for depth, but
should not repeatedly conceal critical combat or movement information.

### Lifecycle and cost

- Compute structural grouping and composition on room preparation, affected
  terrain changes, or relevant art/theme changes. Do not do procedural
  placement in the per-frame actor update path.
- Reuse `RoomTerrain`'s batched quads, chunks, texture handles, and dirty
  updates. Keep ordinary rooms without dressing inexpensive.
- Redress the affected region when blocks are broken or replaced; preserve
  edge uncovering and neighboring visual continuity.
- Keep decoration deterministic for the same room geometry and authored
  inputs. Do not use world simulation RNG for presentation placement.
- Prefer atlas/mesh reuse and bounded decorative density to thousands of
  independent Bevy entities. Measure only if the pilot shows a real cost.
- Respect generated-versus-editor-owned LDtk room authority. Change the
  actual authoring source, not a generated copy that will be overwritten.

## Three visual acceptance scenes

Use **the same practical camera compositions before and after**. Source room
identities and likely authoring entrypoints are given to avoid reconnaissance
work; verify ownership before editing LDtk outputs.

### A. Pirate-port staircase — coherent adjacent geometry

- **Room:** `pirate_port_gate_plaza`.
- **Authoring source:**
  `game/ambition_map_assets/ambition_content/generators/pirate_port/make_specs.py`;
  see `stair_1`, `stair_2`, `stair_3`, `harbour_balcony`.
- **Current appearance:** a sequence of stacked, plain rectangular blocks
  climbing into the balcony.
- **Target:** one visually continuous architectural stair/landing assembly,
  with aligned nosings, edge ends, supporting wall or brackets, and material
  continuity into the balcony. The physical step layout is unchanged.
- **Demonstrates:** grouping across neighboring blocks, intentional joints,
  correct structural end treatments, an authored stair role if needed.
- **Success:** the player can visually understand the route and the assembly
  reads as constructed architecture rather than a pile of independently
  decorated collision rectangles.

### B. Spectral wing — a thematic suspended-platform language

- **Rooms:** `math_hotel_eigen_gallery` and/or
  `math_hotel_spectral_gap` (start where the camera shows several floating
  ledges clearly).
- **Authoring source:**
  `game/ambition_map_assets/ambition_content/generators/math_town/haunted_wing.py`;
  see `gallery_ledge_*` and `gap_step_*`.
- **Current appearance:** checkerboard platforms with sharp rectangular
  silhouettes, disconnected from the haunted interior's ornate visual
  language.
- **Target:** authored spectral/hotel structural treatment: substantial
  undersides, ornamental edge caps, brackets or other support language,
  restrained supernatural accents, and optionally a little depth-crossing
  foreground detail. Avoid making every platform visually identical.
- **Demonstrates:** one reusable structural role dressed differently by a
  theme, readable platform silhouettes, controlled foreground/background
  overlap.
- **Success:** these platforms look like pieces of the haunted hotel, while
  their traversable tops remain immediately legible.

### C. Cedar forest — organic silhouette and variation

- **Room:** `ninja_village_forest`.
- **Authoring source:**
  `game/ambition_map_assets/ambition_content/generators/ninja_village/make_specs.py`;
  see `BRANCHES`, `branch_*`, and `forest_floor`.
- **Current appearance:** wooden ledges with regular overlapping green ovals
  underneath; the foreground ground is better integrated.
- **Target:** convincingly branching wooden masses, varied foliage clusters,
  sparse dangling vegetation or roots, tapered and asymmetric edges. Preserve
  a clean readable walking surface and plausible connection to the trees.
- **Demonstrates:** larger-scale irregular art composed along exposed spans,
  variation based on the whole formation, and stable deterministic placement.
- **Success:** the player reads the upper route as tree branches rather than
  horizontal planks with repeated ovals pasted underneath.

**Positive comparison scenes:** the existing pirate scaffold/docks and red
wasteland ground. Do not regress their visual quality or impose organic
silhouettes on straight architectural materials.

## Implementation order

1. **Capture baseline views.** Play or use `capture_scene` for the three
   named rooms. Keep camera, viewport, and visual quality comparable for
   after-images. Determine which assets and presentation mechanisms are
   actually being selected. See `docs/recipes/headless-room-verification.md`.
2. **Deliver one visible pilot with existing mechanisms.** Start with the
   spectral wing's floating ledges (strongest contrast), producing and
   publishing genuinely better art. Use the current edge logic, skin pieces,
   and authored props. Identify the *specific visible defect* that cannot
   be solved this way, if any.
3. **Extract a minimal reusable composition mechanism.** Only if needed by
   the pilot, coalesce exposed spans into structural features and let a
   theme/palette choose treatments. Reuse the current `RoomTerrain`
   batching/invalidation and avoid alternate render or collision authority.
4. **Apply the same mechanism to the harbor steps and forest branches.**
   Favor one reusable mechanism with different authored assets. Add the
   smallest optional authoring controls required to distinguish structures.
5. **Art-direct and playtest.** Review screenshots at gameplay scale, walk
   and jump through the scenes, and iterate on edge readability, silhouettes,
   repetition, support logic, depth, and composition. Prefer changing
   artwork/placement over adding algorithmic infrastructure when the visual
   problem is primarily artistic.
6. **Finish integration.** Verify broken/replaced blocks, asset publication
   and reload, default art in unthemed rooms, and normal quality profiles.
   Address measured performance issues if they arise. Leave unrelated
   scenery and generic renderer code intact.

The ordering is a guide to producing useful work, not a gating bureaucracy.
An agent may combine steps and continue autonomously through reasonable
reversible decisions. Do not pause an overnight task over a minor art choice.

## Acceptance and completion

The feature is ready for maintainer playtesting when:

- The three named scenes visibly improve in a side-by-side gameplay capture;
  the improvement is in **structural composition and layering**, not just
  the number of small decorations.
- Collision, platforming, camera behavior, hazards, door access, and game
  state remain unchanged. The visuals communicate gameplay affordances.
- The same presentation concepts work for architectural, spectral, and
  organic environments without room-specific renderer branches.
- Published art and optional manual overrides fit the existing game/content
  library pipeline; missing dressing retains the existing fallback.
- Dynamic changes preserve sane terrain boundaries and nearby visual joins.
- Ordinary rooms do not acquire a per-frame dressing or collision cost.

Use screenshots and real playtesting as the main acceptance evidence. Run a
focused existing check if changes affect a consequential engine invariant.
Do not manufacture a permanent suite of tests for individual decoration
placements, source-text policy, or pixel-exact composition.

## Out of scope for this pass

No new physics geometry; no generic procedural level generator; no second
asset registry or ECS terrain hierarchy; no per-biome rendering plugin; no
shader-dependent foundation; no automatic redesign of every room; no broad
revision of unrelated backgrounds, dialogue, UI or portal mechanics. Future
large-format structural art or animated foreground props are welcome if the
pilot reveals a concrete need, but are not prerequisites for delivery.
