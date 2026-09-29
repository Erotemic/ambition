# Runtime rigged sprite animation — disjoint plan for triage

**State:** DISJOINT / TRIAGE — this document does not change queue priority or commit to a migration.

**Purpose:** evaluate and, if evidence supports it, add an optional runtime character-presentation path that reuses rasterized body parts instead of storing every pose as a complete raster frame.

This plan is separate from the active planning queue on purpose. Triage it later against the asset-residency, presentation, composition and performance programs.

Related current owners:

- [`render-animation-and-vfx.md`](render-animation-and-vfx.md) owns the simulation-to-presentation boundary and drawable scheduling;
- [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md) owns demand, preparation, device materialization, quality and residency;
- [`performance-and-iteration.md`](performance-and-iteration.md) owns runtime and memory evidence;
- [`sprite-renderer.md`](sprite-renderer.md) and the sprite-renderer repository own authoring/publishing contracts;
- [`svg-component-character-migration.md`](svg-component-character-migration.md) covers component-oriented character authoring where it is already useful;
- [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md) owns view-local presentation requirements.

## Motivation

Ambition currently publishes most character animation as baked sprite-sheet frames. The runtime path is efficient and simple: `CharacterSpriteAsset` owns one or more packed atlas pages, and `CharacterAnimator` selects a row/frame and updates one Sprite presentation.

That representation has an important cost. When many poses reuse the same head, torso, limbs, clothing, weapon or accessory, the same pixels are stored again in many packed frames. More poses, forms, costumes and NPC variations can therefore increase texture bytes much faster than they increase genuinely new art.

This matters more as Ambition moves toward:

- a persistent world with many resident or recently used actors;
- larger authored move and social-animation repertoires;
- visible equipment and clothing variation;
- multiview, where more presentation can be visible at once;
- weaker target GPUs with tighter texture budgets;
- runtime content that should not require loading a large full-pose sheet for every small variation.

The current asset-residency plan already records that oversampling and weak-tier texture cost are real. It also requires stage-specific evidence rather than assuming that every hitch is texture I/O. This proposal follows that rule. The working hypothesis is that repeated full-pose raster data is an important part of character texture residency and upload cost. The implementation must measure that hypothesis before a broad migration.

## Existing evidence in the repository

This proposal does not start from a blank design.

### The authoring system already supports plural representations

The sprite renderer deliberately supports procedural drawing, shared parametric character families, rig documents, SVG parts, scene graphs, multipart bosses and hybrids. Its published runtime asset is the stable contract; a rig is explicitly not the universal authoring representation.

Preserve that rule. Runtime part composition must not force every character authoring source into one skeleton format.

### Ambition already has a working part-atlas precedent

The generated `vanity_card_made_this_meme` path already proves the basic representation:

```text
rig / choreography in Python
        ↓
offline solve
        ↓
part images rasterized once
+ packed part atlas
+ per-frame part placements
        ↓
Rust runtime draws the placements
```

The exporter describes the runtime payload as a list of images plus per-frame `(part, centre, rotation)` placements. It verifies the baked placement table by recompositing the parts and pixel-diffing against the direct renderer output.

The runtime card then loads one packed texture and reuses a fixed number of part slots while it plays the transform frames.

This is valuable evidence because it separates two questions that should not be conflated:

1. **Must the engine evaluate the authoring rig?** No. The authoring tool can solve the rig offline.
2. **Can the engine draw reusable parts instead of complete pose rasters?** Yes. One existing presentation already does it.

The first character prototype should reuse this principle before adding runtime IK or a general bone solver.

## Core decision

Add support for **multiple runtime visual realizations**, not one mandatory replacement for sprite sheets.

The target conceptual model is:

```text
semantic actor presentation
        ↓
semantic animation / clip request
        ↓
visual realization
        ├── baked sheet
        ├── rigged parts
        └── hybrid
```

The semantic animation authority stays above all three realizations.

A body may ask for `run`, `idle`, an authored move clip, a social pose or another semantic animation. The presentation implementation decides how to draw that request. Gameplay must not care whether the result came from one atlas frame or twelve reusable parts.

## Architectural rules

### 1. Presentation only

Runtime rig state is presentation state.

Do not derive simulation authority from bones or part transforms. In particular, do not move these authorities into the rig:

- body size;
- collision shape;
- hurtboxes;
- attack geometry;
- movement/contact state;
- item custody;
- rollback gameplay state.

Those facts keep their current semantic owners.

A visible host may omit the rig renderer without changing simulation outcome.

### 2. Keep semantic pose selection above representation

Current semantic animation selection and `BodyPoseView` / presentation facts remain the input.

Do not introduce a second gameplay-facing vocabulary such as `RigPose::Run` beside `CharacterAnim::Run` when they mean the same thing.

The realization layer maps the current semantic request to either a baked row/clip or a part-animation clip.

### 3. Do not make authoring rigs the runtime contract

The sprite renderer may use bones, SVG groups, procedural code, direct drawing or another method.

A runtime part animation consumes a **published runtime product**. It does not import authoring-time Python concepts into Rust.

The first product should be resolved rigid-part placement data. A later product may contain bones/keyframes if runtime interpolation proves valuable.

### 4. Avoid one ECS entity per body part as the final design

A proof spike may temporarily use ordinary child sprites to establish correctness and collect measurements. That is not the intended shipping architecture.

The shipping path should treat a rigged actor as one presentation owner that emits N part draw instances. The parts should share atlases/materials where possible and should not create N independent gameplay/presentation lifetimes.

### 5. Use the existing quality and residency authorities

Rigged parts do not get a second quality selector, texture cache or residency policy.

Part atlases participate in the same requested/resolved quality-tier and asset-demand architecture as baked character sheets.

### 6. Preserve plural animation techniques

Some animation is cheaper and better as baked art.

The engine should support:

```text
rigid repeated motion       → reusable parts
costume/equipment variation → reusable parts
ordinary social motion      → reusable parts
smears / squash / deformation / perspective redraw → baked frames
mixed effect                → hybrid
```

Do not convert a character or clip when the part representation saves little or damages the visual result.

## Proposed runtime representation

Names below are descriptive placeholders. Triage may rename them to fit the current asset vocabulary.

### `CharacterVisualRealization`

A character presentation declaration should be able to resolve to one of:

```text
BakedSheet(CharacterSpriteAsset)
RiggedParts(RiggedSpriteAsset)
Hybrid(HybridCharacterVisual)
```

This does not require a public Rust enum with exactly these names. The important rule is that the semantic character/animation API does not fork by representation.

### `RiggedSpriteAsset`

The minimum runtime product needs:

```text
part atlas page(s)
part rectangle for each part
part pivot/origin
logical character render basis
clip table
per-clip timing
per-frame or per-keyframe part transforms
part visibility
part draw order / z
optional tint / variant selector if required
quality-tier identity
```

A resolved part instance needs only presentation data such as:

```text
part index
translation
rotation
scale or displayed size
z / draw order
visibility
```

Do not include authoring-only IK constraints unless a later runtime-solver phase proves that they are needed.

### Representation level 1 — transform flipbook

Start here.

The authoring tool solves the rig at publish time and writes the final part placements for each animation sample. Runtime does no bone solving.

Advantages:

- captures most texture deduplication benefit;
- low semantic risk;
- exporter can verify exact visual equivalence;
- easy comparison with baked frames;
- no runtime IK or constraint system;
- deterministic presentation data;
- simple authoring/runtime ownership boundary.

The existing vanity-card exporter is the direct precedent.

### Representation level 2 — keyframed part transforms

If frame tables are large or smoother interpolation is valuable, allow the publisher to reduce resolved frames into keyframes.

Runtime interpolates translation, rotation and scale between keys.

This remains a part-transform player. It is not yet a skeletal solver.

Measure CPU cost and visual equivalence before making interpolation the default.

### Representation level 3 — skeletal clips, only if a customer needs them

A later rig product may contain a small bone hierarchy and part bindings when runtime bone evaluation buys something concrete:

- continuous aiming;
- procedural look/gesture overlays;
- runtime equipment attachment;
- a large reduction in animation transform data;
- procedural NPC variation that cannot be efficiently published as resolved frames.

Do not add general IK, constraints or runtime vector deformation merely because the authoring tools have them.

## Hybrid clips

A sophisticated character should not need one representation for every animation.

The long-term target should allow a clip or visual layer to choose its realization:

```text
idle / walk / run / talk   → part animation
special attack smear       → baked clip
transform effect           → baked clip
ordinary body + sword glow → part body + effect overlay
```

There are two possible implementation shapes:

1. a character has one primary realization and explicit baked override clips;
2. each published clip declares a realization kind.

Do not choose between them until the first prototype shows the simpler data model.

## Render integration

### One visual owner, many draw instances

The preferred render road is:

```text
semantic pose / animation request
        ↓
rigged character presentation state
        ↓
resolve current part instances
        ↓
render extraction
        ↓
batched quads from shared part atlas
```

The CPU-side character should own one compact animation state. The renderer should emit a dense list of part instances.

Do not give every part an independent update system.

### Batching

The renderer should try to keep one character's parts on one atlas/material where practical.

Measure:

- extracted instances;
- actual draw calls / batches;
- CPU extraction time;
- transform-update time;
- GPU vertex/instance bytes;
- alpha overdraw.

A reduction in texture memory is not automatically a win if it creates unacceptable CPU/render overhead.

### Draw order

Part order must be authored data.

Support stable default z per part and per-frame/keyframe overrides only where needed. Do not reconstruct anatomy order from part names in the engine.

### Facing and mirroring

The part player must obey the same semantic facing result as baked presentation.

Mirroring should happen at the realization boundary without introducing a second facing authority. If an asymmetric rig has distinct left/right art, the published asset must declare that explicitly just as current sheets can use authored mirror rows.

### Portal and multiview presentation

Rigged characters must participate in the current body-owned-drawable finalization and portal-compositing architecture.

Do not solve portals by flattening the rig back into a temporary full-body texture each frame.

The design must establish how N part instances become portal-aware drawable geometry without making each part an authoritative ECS object. Likely options are:

- publish an array of `DeclaredFrame`-like part drawables owned by one body presentation;
- extract virtual per-part draw records directly into the portal/view presentation stage;
- share one body clip relation where mathematically valid and clip the resulting part instances in that view.

This is a required architecture checkpoint before production migration because multiview will multiply presentation cost and portal relationships are per pane/drawable.

## Authoring and publishing

### Preserve authoring plurality

The sprite renderer remains free to create a character with procedural Python, SVG components, a rig document or another family.

A target becomes eligible for runtime part animation only when its publisher can identify reusable rigid raster parts and publish their transforms.

### Publish resolved runtime data

The publisher should emit:

```text
part images
→ pack into one or more part-atlas pages

animation source
→ evaluate poses / rig / procedural placement
→ emit resolved runtime transforms

runtime manifest
→ part metadata
→ clip timing
→ frame/key transforms
→ quality/source metadata
```

The engine should not need the source SVG or Python rig document.

### Rigid parts first

The first exporter supports rigid sprite parts only.

A part that deforms with a bone, uses vector geometry that changes shape, or needs another unsupported operation causes that clip/character to stay baked or use a hybrid fallback.

Do not rasterize an incorrect rigid approximation to make the new road universal.

### Pixel-equivalence verification

Reuse the vanity-card verification rule.

For a published transform animation:

1. render the canonical source frame;
2. composite the exported runtime parts with the exported transforms;
3. compare the images;
4. report pixel/alpha differences and fail when they exceed the selected tolerance.

This test belongs primarily in the sprite-renderer repository because it compares the publisher to its own source representation.

## Measurement before migration

The first implementation packet should be an evidence generator, not a renderer rewrite.

Create a reproducible report for representative characters.

### Current baked representation

Record at least:

- number of semantic rows/clips;
- number of physical frames;
- atlas page count;
- packed rectangle pixels;
- source file bytes;
- decoded CPU bytes where available;
- estimated or measured device-resident bytes;
- requested/resolved quality tier;
- load/decode/materialization timing from the existing asset stages.

Use actual packed frame rectangles. Do not compare against untrimmed logical frame width × height and claim that as current cost.

### Candidate part representation

Record at least:

- number of unique raster parts;
- packed part-atlas pixels;
- part-atlas page count;
- transform data bytes;
- average visible parts per frame;
- maximum visible parts per frame;
- number of z/order changes;
- percentage of frames/clips that require baked fallback;
- estimated device-resident texture bytes.

### Runtime prototype measurements

With equivalent visible actors, record:

- asset demand-to-ready time;
- texture upload/materialization time;
- resident image bytes;
- presentation CPU time;
- extraction CPU time;
- number of extracted part instances;
- render batches/draw calls if available;
- frame time on a representative desktop and a weaker profile.

Run at more than one actor count. A useful shape is 1, 10, 50 and 100 visible actors, adjusted to the practical benchmark environment.

Do not select a hard required savings ratio before these measurements exist.

## Representative candidates

Use at least three kinds of character.

### Positive candidate — an existing rigid-part/rig-document character

Prefer a shipped actor whose current authoring already exposes reusable rigid parts. `player_robot_v3` is a strong candidate because current authoring already uses rig-document machinery in related tooling and it is a real gameplay character.

The pirate family is also useful because it reuses shared parametric anatomy across several shipped characters and represents the population/variation case this design aims to improve.

### Negative or hybrid candidate

Choose one character or boss with meaningful deformation, silhouette changes, smears or pose-specific redraws.

The purpose is to prove that the architecture can decline to rig a bad candidate and can retain baked clips without special gameplay code.

### Population candidate

Choose several visually related NPCs or variants. Measure whether shared parts/palettes/accessories can reduce incremental residency per actor type without creating a combinatorial authoring system.

## Asset identity, demand and residency

### One semantic visual demand

A character demand should resolve its selected runtime representation through one presentation asset declaration.

Do not make gameplay decide whether to demand `foo_sheet` or `foo_rig`.

### Quality variants

Part atlases should use the same active texture-quality authority as baked sheets.

Possible publishing choices include:

- publish each part atlas at the existing quality tiers;
- publish high-quality parts and generate the same lower tiers during the normal variant pipeline.

Do not invent a runtime-only rig quality scale.

### Residency accounting

The asset census should report rigged presentation with the same lifecycle stages:

```text
declared / demanded
CPU prepared
device materialized
resident use / first draw
```

The report must count part-atlas bytes and transform metadata separately so texture savings are visible without hiding CPU-side growth.

### Eviction

Dropping the realized rigged asset must release its strong image handles just as dropping a `CharacterSpriteAsset` releases baked pages.

Do not add a permanent global part cache without an explicit residency owner.

## Interaction with texture compression

GPU-native texture compression is complementary to this proposal.

A separate measurement may show that BC/KTX2/Basis-style device formats reduce the immediate problem enough that runtime rigs have lower priority. That does not invalidate the part representation; the same compression can apply to part atlases.

Keep the questions separate:

```text
How many pixels do we need to store?
How are those pixels compressed on disk/device?
How many draw instances do we need to render them?
```

Do not use a renderer rewrite to solve a compression problem or a compression change to hide extreme duplicated-pixel growth.

## Implementation phases

### Phase 0 — evidence and format experiment

Goal: determine whether Ambition's real assets have enough repeated raster data to justify runtime part composition.

1. add a sprite-renderer report that compares current packed-frame pixels with reusable-part pixels for candidate characters;
2. export a transform-flipbook manifest for one shipped rigid-part character;
3. verify exporter recomposition against canonical renderer output;
4. record transform payload size and maximum visible part count;
5. add no engine runtime path yet unless a tiny decoder is needed to validate the manifest.

**Gate:** continue only if at least one important character/population shows meaningful texture/residency savings and the format remains simple.

### Phase 1 — minimal runtime part player

Goal: prove one real actor can use reusable parts without changing gameplay semantics.

1. add the minimum runtime asset type for one part atlas + resolved transform frames;
2. load it through existing character visual demand/quality infrastructure;
3. feed it from the same semantic animation request used by baked presentation;
4. draw it in the world;
5. keep collision, hitboxes and rollback unchanged;
6. retain the baked sheet for A/B comparison during the spike.

A child-Sprite implementation is acceptable only as a temporary benchmark/correctness spike. Do not declare Phase 1 production-ready on that architecture.

**Acceptance:** the actor can run through representative locomotion, social and attack clips through the same semantic animation API, with measured memory and frame-cost data.

### Phase 2 — production render extraction

Goal: remove per-part ECS scaling cost.

1. represent one rigged character as one presentation owner/component;
2. resolve its current part instances into a compact buffer;
3. extract those instances for rendering;
4. batch by atlas/material;
5. preserve draw order;
6. integrate facing, tint and quality changes;
7. measure 1/10/50/100-actor loads.

**Acceptance:** the new representation demonstrates a useful memory reduction without unacceptable CPU/frame-time growth.

### Phase 3 — portal and multiview integration

Goal: make rigged characters first-class Ambition drawables.

1. publish rigged part geometry before the existing body-owned-drawable finalization boundary;
2. support near-side, far-side, transit and disjoint portal cases;
3. support two panes with different portal relationships;
4. ensure one pane's presentation never becomes authority for another;
5. verify that headless simulation has no dependency on the rig asset.

**Acceptance:** rigged and baked characters obey the same portal/view semantics.

### Phase 4 — hybrid clip support

Goal: keep baked art where it is the better representation.

1. allow selected clips or layers to use baked frames;
2. define transition behavior between part and baked clips;
3. share the same render basis / feet anchor;
4. keep semantic clip identity above the realization choice;
5. add one real hybrid character witness.

**Acceptance:** a character can use parts for ordinary motion and baked art for one deformation-heavy special without gameplay-side branches.

### Phase 5 — optional transform interpolation

Goal: test whether keyframed part motion improves visual smoothness and shrinks transform data.

1. add keyframe interpolation for translation/rotation/scale;
2. compare against the transform flipbook;
3. verify phase-slaved move clips remain synchronized to gameplay timing;
4. measure CPU cost;
5. retain discrete frames where exact authored timing is required.

Do not add a general skeletal solver in this phase.

### Phase 6 — runtime skeleton only if demanded

Open this phase only if a real feature needs runtime bones, such as procedural aim, look direction, gesture composition or dynamic equipment attachment.

If opened:

1. publish a small runtime bone hierarchy separate from authoring constraints;
2. keep the semantic clip/action authority unchanged;
3. keep IK/procedural constraints capability-specific;
4. keep gameplay geometry independent;
5. benchmark the solver against resolved-part playback.

A runtime skeleton is successful only when it removes more complexity or content cost than it adds.

### Phase 7 — measured migration

Do not migrate the roster by policy.

For each candidate:

1. measure current baked cost;
2. measure proposed part/hybrid cost;
3. verify visual equivalence or intentional improvement;
4. verify runtime cost;
5. migrate only when the result is favorable.

The baked-sheet path remains supported for characters that are cheaper or better that way.

## Testing and acceptance surface

### Publisher tests

- every referenced part exists;
- pivots and part rectangles are valid;
- clip/frame transform tables are deterministic;
- recomposed frames match canonical source rendering for rigid clips;
- unsupported deforming parts fail or select a declared baked fallback;
- quality variants remain fresh.

### Runtime asset tests

- semantic character identity resolves one declared visual representation;
- demand/readiness states distinguish declared from ready just as baked sheets do;
- quality swaps keep the current representation visible until replacement is ready;
- dropping the realization releases its image handles according to existing residency policy.

### Animation tests

- the same semantic `CharacterAnim` / authored clip request selects equivalent content on baked and part realizations;
- phase-slaved attack clips preserve move timing;
- facing/mirroring preserves asymmetric art behavior;
- switching baked ↔ part clips keeps the same logical render basis and feet anchor.

### Presentation tests

- portal near/far/transit/disjoint behavior;
- multiview with different portal relationships;
- hit flash / tint / submerged / other body-owned effects still apply through the intended compositing boundary;
- rigged presentation can be omitted in a headless composition.

### Performance acceptance

Do not accept the architecture on texture-byte savings alone.

Report, side by side:

```text
baked:
    resident texture bytes
    source/decode/upload cost
    presentation CPU
    extraction CPU
    draw instances / batches
    frame time

part-based:
    resident texture bytes
    transform metadata bytes
    source/decode/upload cost
    presentation CPU
    extraction CPU
    draw instances / batches
    frame time
```

Retain the raw evidence in the normal benchmark/evidence location rather than copying volatile numbers into this plan.

## Risks and tradeoffs

### More quads and extraction work

A full-pose sheet can render one actor as one quad. A part actor can require 8–30 or more quads.

Mitigation: packed shared atlas, compact instance extraction, no per-part gameplay ECS, measured batching.

### Alpha overdraw

Overlapping limbs/clothes can increase overdraw.

Mitigation: measure on weak targets; trim part rasters; avoid invisible oversized part bounds.

### Seams and pivots

Separate raster parts can reveal cracks, filtering seams or pivot drift.

Mitigation: author/publish padding rules, pixel-equivalence tests, stable pivots, appropriate texture filtering.

### Draw-order complexity

Crossing limbs, props and clothing can require changing z order.

Mitigation: publish order explicitly. Keep overrides data-driven and sparse.

### Deformation does not fit rigid parts

Some art depends on redraw rather than rigid transform.

Mitigation: hybrid baked clips are a first-class requirement, not an escape hatch added later.

### Runtime bones can become a second simulation skeleton

A convenient presentation skeleton may attract gameplay queries.

Mitigation: enforce the presentation-only boundary. Gameplay geometry stays with its current prepared/simulation authority.

### Variation can create a combinatorial material system

Runtime parts make costume variation possible, but a generic paper-doll framework can become a new project by itself.

Mitigation: first prove static character parts. Add swappable equipment/clothing only when a real character/population requires it.

### Memory may not be the actual bottleneck

Current packed frames are trimmed and split, and quality tiers/lazy realization already reduce cost. Device compression may be a larger win.

Mitigation: Phase 0 measures current packed/resident cost and compares alternative representations before engine migration.

## Explicit non-goals

- no removal of baked character sheets;
- no mandatory rig authoring for all characters;
- no gameplay collision/hitbox derivation from bones;
- no rollback registration for disposable rig presentation state;
- no runtime Python, SVG or authoring-tool dependency;
- no general IK/constraint engine in the first implementation;
- no one-ECS-entity-per-part shipping design;
- no global permanent part cache without residency ownership;
- no broad costume/equipment paper-doll system without a real customer;
- no roster-wide migration before measurements.

## Triage questions

Before this plan enters the active queue, answer:

1. What fraction of current character GPU residency is baked character sheets versus other textures?
2. For representative shipped characters, what is the ratio of packed full-frame pixels to unique reusable-part pixels?
3. Is the primary pain device residency, upload time, decode time, page count, or first-draw materialization?
4. Can the existing vanity-card transform manifest be generalized into a character runtime product without importing UI-specific assumptions?
5. Which shipped actor is the best first positive candidate: `player_robot_v3`, a pirate-family member, or another rigid-part character?
6. Which shipped actor is the best negative/hybrid control?
7. Can portal compositing consume virtual part drawables efficiently without per-part ECS entities?
8. Does Bevy's existing sprite batching make a compact part-instance path sufficient, or is a custom extraction/render phase justified?
9. How much would GPU-native texture compression reduce the same measured residency cost?
10. Should the first production form be transform flipbooks only, with keyframes/runtime bones deferred until a concrete need appears?

## Recommended first triage packet

If this document is accepted into active planning, start with one bounded evidence packet:

1. add a reproducible baked-versus-parts size report in the sprite-renderer repository;
2. export one shipped rigid-part character as a verified transform flipbook;
3. record its part atlas and transform payload;
4. compare that against the current packed baked atlas at Full/Half/Potato where available;
5. estimate the runtime visible-part count;
6. do not modify Ambition's character renderer yet.

That packet should be cheap and should answer whether a runtime implementation deserves priority.
