# Semantic Part Rendering and 2D Ragdoll Roadmap

## Status

Selected architectural direction and implementation plan.

This document plans the migration from the current mandatory whole-body part compositor toward a semantic-part presentation model that preserves the advantages of vector-authored parts, supports high-resolution presentation without blurry upscaling, and creates a direct path to future 2D ragdolls.

The current part pipeline was a useful compatibility bridge: it let part-authored characters continue to look like one ordinary body sprite to portals, fades, color effects, and other one-image consumers. The problem is not that render-to-texture composition exists. The problem is that whole-body composition has become the mandatory presentation path, and its current implementation uses ordinary Bevy cameras and atlas-page redraws for work that is fundamentally local character composition.

The long-term engine should not require every semantic body part to be flattened back into one body image every frame. Whole-body or local composition should be a derived presentation operation used only where its mathematics or an unmigrated consumer require it.

## North-star requirements

The resulting architecture must satisfy all of the following:

1. **One semantic body decomposition.** Animation, rendering, attachments, portals, and future physics must refer to the same semantic part identities. Do not create a renderer skeleton and a separate ragdoll skeleton that can drift.
2. **Pose is an input, not an authority split.** Authored animation and future physics/ragdoll simulation are alternative providers of transforms for the same semantic part graph.
3. **SVG/vector art remains the visual source of truth.** Quality modes choose raster realization quality from that source. Ultra must not upscale a lower-resolution raster when a vector source exists.
4. **Semantic parts render directly in world space by default.** A hand, forearm, head, torso, weapon, etc. should be independently drawable when it has world/physical meaning.
5. **Nonphysical visual layers can be composed locally.** Eyes, mouth/expression layers, decals, insignia, face paint, highlights, and similar layers can be cached/composited into the semantic part that owns them.
6. **Whole-body composition is optional derived presentation.** Keep it for effects/consumers that truly require `effect(composite(parts))`, not as the default representation of every part-authored character.
7. **No duplicate transform authority.** A visual flipbook must not independently own another per-frame transform table when the semantic rig/pose already owns the same pose facts.
8. **Quality and physics do not redefine character semantics.** Potato through Ultra and animated through ragdoll all describe the same character/part graph.
9. **Structural memory savings come before fragile raster tricks.** Reuse semantic/vector parts first. Pixel-level tricks such as symmetric-half storage are optional optimizations and must be demonstrably lossless.
10. **Performance must scale with changed visual work, not with compatibility machinery.** No fleet of ordinary scene cameras should be required to update character parts.

## Existing architectural seed to preserve

The repository already has an important piece of the desired authority structure: the body rig/pose side of the engine is simulation-owned, and authoring code can derive rig information from the same SVG/RigDocument source used to draw visual output.

Build on that authority. Do not replace it with a new presentation-only skeleton.

The implementation campaign must identify the current authoritative types and preserve or refine them so that the conceptual road is:

```text
Character definition
        |
        v
Semantic part graph
        |
        +-------------------+
        |                   |
        v                   v
   animation pose      physics/ragdoll pose
        |                   |
        +---------+---------+
                  |
                  v
         semantic part transforms
                  |
          +-------+--------+
          |                |
          v                v
   vector appearance   attachments/layers
          |                |
          +-------+--------+
                  |
                  v
          presentation realization
```

The exact Rust type names may evolve, but there should be one body decomposition and one current pose, not parallel truths.

## Semantic vocabulary

Do not call every independently stored raster fragment a "body part". The migration should establish explicit semantics.

### Semantic part

A semantic part has meaningful identity in the character's structure and may eventually participate in physics, attachments, portals, damage, or gameplay presentation.

Examples:

- torso;
- head;
- upper arm;
- forearm;
- hand;
- thigh;
- shin;
- foot;
- tail segment;
- weapon;
- shield.

A semantic part owns a transform in the current pose.

### Visual layer

A visual layer decorates one semantic part but normally has no independent physical/world meaning.

Examples:

- eyes;
- mouth/expression;
- beard/face detail;
- shirt logo;
- tattoo;
- insignia;
- damage decal;
- highlight;
- small color/lighting layers.

Visual layers inherit their semantic part's transform. They may be drawn directly when cheap, but they are prime candidates for local composition/cache.

### Attachment/accessory

An attachment has semantic identity and a stable attachment relationship but may or may not have independent physics initially.

Examples:

- hat;
- backpack;
- cape;
- hair lock;
- scabbard;
- held equipment.

This category gives the engine room to add secondary physics later without pretending every decorative layer is already a rigid body.

### Physics realization

Physics is not the definition of a semantic part. A ragdoll profile chooses which semantic parts participate as rigid bodies, how they are joined, and which visual/semantic parts follow another physical body.

For example, a visually rich torso may contain multiple layers while the physics realization contains one torso rigid body.

## Target presentation architecture

The intended normal path is:

```text
semantic part graph
      + current pose
      + quality realization
              |
              v
        world-space parts
              |
        normal 2D renderer
```

No whole-character offscreen render target is required for ordinary opaque presentation.

### Local composition/cache

A B-like compositor remains useful at the **semantic-part** level.

Example:

```text
head
  + face base
  + hair
  + eyes = angry
  + mouth = yell
  + scar
        |
        v
local resolved-head cache
        |
        v
one world-space head quad
```

This is a cache/realization detail, not another semantic body definition.

A cache key must be composed only from facts that materially change that resolved semantic part, such as:

- character/appearance realization;
- semantic part identity;
- visual-layer selections;
- expression state;
- quality/pixel-density realization;
- equipment/skin variants that are actually baked into the local result.

Per-instance effects that can be applied downstream should stay downstream so they do not explode cache cardinality.

### Optional whole-body composition

Retain a general group-composition capability for operations where the mathematics require the whole body to be flattened temporarily.

The clearest example is whole-body translucency:

```text
fade(composite(parts)) != composite(fade(part_1), fade(part_2), ...)
```

Other possible customers include screenshots/portraits, an unmigrated portal path, or a specialized effect.

This facility must be opt-in. It must not silently become the default rendering path again.

## Effects migration

Classify existing one-image consumers instead of preserving them all through whole-body composition.

### Effects that should normally become part-aware/inherited

Likely examples:

- hit flash;
- HSV/color shift;
- star-power coloration;
- opaque tint;
- ordinary visibility;
- per-character material parameters.

The semantic authority remains character-level. All parts merely consume the same derived presentation parameters. This is not duplicate authority.

### Effects that may require group composition

Likely examples:

- whole-body translucent fade;
- effects whose result depends on compositing overlaps before applying the effect.

### Portals

Portals are the largest architectural customer and require deliberate treatment.

Do not let the portal compositor force the entire character renderer to remain one-image-based forever. The implementation campaign should establish a part-aware portal road. A semantic-part representation is actually advantageous for future ragdolls because different limbs/attachments can occupy different sides of a portal.

A semantic part that itself crosses a portal plane may still need clipping. That is a local geometry/presentation problem; it does not justify flattening every body in every frame.

## Ragdoll seam

The renderer should accept semantic-part transforms without caring whether they came from animation or physics.

Normal operation:

```text
animation state
     -> semantic pose
     -> part transforms
     -> render
```

Ragdoll operation:

```text
rigid bodies + joints
     -> semantic pose
     -> part transforms
     -> render
```

A later blended/partial ragdoll can combine pose sources before presentation.

Do not introduce a second renderer representation specifically for ragdolls. Do not make every semantic part a rigid body today. Instead define a future ragdoll/physics profile that references semantic part IDs and determines:

- physical bodies;
- mass/inertia;
- joint topology and limits;
- which semantic parts follow which physical body;
- attachment behavior;
- transition from animation to physics and back, if supported.

A small development proof should eventually demonstrate that one existing semantic rig can switch from animation-supplied transforms to physics-supplied transforms without swapping rendering representations.

## Ultra quality and vector-derived realization

Ultra should use the information available in vector sources instead of magnifying already-rasterized parts.

The desired authority chain is:

```text
SVG / vector source
      |
semantic visual part/layer
      |
quality realization policy
      |
requested raster resolution / filtering
      |
GPU image
```

Quality is a presentation realization, not a different logical asset.

The implementation must ensure that high-quality modes rasterize from the SVG/vector source at a resolution appropriate to their intended screen size. A lower-quality raster must not become the source for a higher-quality realization when vector data exists.

Potato mode remains free to use tiny realizations or cheaper presentation algorithms. The semantic part graph stays identical.

### Resolution policy still to discover

The implementation agents must determine the right realization strategy for the current asset pipeline rather than assuming one now:

- offline publication of a small number of quality-specific raster realizations;
- runtime/on-demand vector rasterization with caching;
- a hybrid where common tiers are published and exceptional high-DPI realizations can be generated/cached.

Measure CPU time, load latency, texture memory, package size, and visual quality before selecting the mechanism. The architectural requirement is only that vector truth is not discarded and later upscaled.

## Symmetry and raster micro-optimizations

The symmetric-half seam found on Hunny Horror is evidence that raster storage tricks can violate visual assumptions even when an offline comparison appears valid.

Keep the transparent-border fix, but also adopt this policy:

> Structural reuse of semantic/vector parts is the primary memory optimization. Raster-level deduplication is optional and must prove visual equivalence under the runtime's filtering and transforms.

A symmetric-half optimization must have a runtime-representative parity test including:

- filtering at the seam;
- transparent borders;
- rotation;
- scaling;
- alpha/fade if applicable;
- mirroring;
- neighboring atlas content.

If a part cannot prove equivalence, store the full part. Do not accumulate an opaque blacklist without a testable reason.

## Migration phases

### Phase 0 - discovery and measurement

Before changing the primary renderer, answer the discovery questions listed later in this document and capture baseline measurements for:

- Hall render-thread time;
- main-thread part-presentation time;
- atlas/render-target memory;
- total GPU texture memory attributable to part characters;
- loading/preparation time for representative part characters;
- visual parity against baked/vector references;
- draw-call/batch behavior for direct loose parts.

Do not use Tracy instrumentation overhead as the sole performance baseline.

### Phase 1 - make the semantic authority explicit

Establish/document the authoritative mapping from authored/vector rig data to semantic part IDs and current semantic pose.

Remove or de-authorize any independent visual per-frame transform table that duplicates the semantic pose. Published visual data may cache derived transforms if necessary for performance, but the derivation and invalidation must be explicit and there must be one semantic source of truth.

Add validation that visual semantic-part IDs, attachments, and pose channels resolve against the same character rig authority.

### Phase 2 - first-class direct semantic-part rendering

Implement a direct world-space rendering path for semantic parts without using the current body-impostor cameras.

Start with a small representative set of characters and prove:

- correct anchoring;
- flip/mirror behavior;
- animation parity;
- ordering of overlapping body parts;
- quality-tier selection;
- stable batching/performance;
- no new simulation authority.

Do not remove the compatibility compositor yet.

### Phase 3 - migrate simple whole-character effects

Move effects that can naturally be inherited by parts to character-level presentation parameters consumed by every relevant semantic part.

At minimum audit:

- hit flash;
- HSV/color shifting;
- star-power/character color overlays;
- visibility;
- ordinary tint;
- sprite flipping/mirroring assumptions.

Every migrated effect should have an equivalence regression against the existing body presentation.

### Phase 4 - local visual-layer composition

Introduce or refactor the compositor so it can resolve **local semantic parts** such as a head with expression layers.

Requirements:

- no ordinary Bevy camera per character or atlas page merely to composite parts;
- dirty semantic-part/cell invalidation rather than whole-page redraw as the primitive;
- shared cache entries where identical visual-layer states are actually reusable;
- conservative bounds/containment for any atlas-backed cache;
- no second semantic pose authority.

The exact GPU implementation is a discovery/measurement decision. A custom render pass, batched offscreen compositor, compute path, or another explicit mechanism is acceptable if it is simpler and measured. Do not prematurely optimize around the current camera topology.

### Phase 5 - part-aware portals and remaining one-image consumers

Audit every consumer that still requires a resolved whole-body image.

Migrate portals deliberately. For each remaining consumer, decide whether it should:

1. operate on semantic parts directly;
2. consume shared character-level presentation parameters;
3. request optional temporary/group composition because the operation mathematically requires it.

Do not retain mandatory whole-body composition merely because a consumer has not yet been migrated.

### Phase 6 - demote/remove mandatory body impostors

Once ordinary world presentation and the important consumers no longer require mandatory whole-body flattening:

- remove the per-page Bevy-camera compositor road from normal character rendering;
- remove page-generation/cell machinery that has no remaining optional-composition customer;
- retain only a smaller explicit composition/cache facility for the cases that still need it.

Do not keep two equally supported default rendering authorities indefinitely.

### Phase 7 - ragdoll proof and later physics work

After semantic-part rendering is stable, build a bounded ragdoll proof on one suitable character using the same semantic IDs and renderer.

The proof should establish the interface between a physics pose provider and the semantic pose without committing every character to one ragdoll topology.

Full gameplay ragdolls, recovery animations, networking/rollback policy, limb damage, and portal-aware physics can be subsequent tasks. The renderer architecture should no longer block them.

## Discovery gates for the implementation agents

The following questions are deliberately left for implementation-time investigation. Answer them before locking in mechanisms.

### Current authority census

1. Which exact types currently own semantic rig identity, current pose, visual flipbook transforms, attachment points, and rendered part ordering?
2. Where do the semantic rig and visual transform tables duplicate the same facts today?
3. Which authored targets already derive semantic and visual data from one SVG/RigDocument source, and which do not?
4. Are there characters whose visual decomposition cannot currently map cleanly onto semantic parts? Classify why before inventing exceptions.

### One-image consumer census

Find every production consumer of the resolved whole-body image/sprite and classify it as:

- trivially part-aware;
- character-parameter inheritance;
- local semantic-part composition;
- true whole-body group composition;
- portal/special geometry work;
- obsolete compatibility code.

The known examples above are not assumed to be exhaustive.

### Render ordering and transform semantics

Determine the existing rules for:

- z-order within one character;
- anchor conventions;
- facing/mirror semantics;
- interpolation/tweening;
- pixel snapping;
- nested attachment transforms;
- per-view render isolation;
- culling/bounds.

Direct part rendering must have one explicit rule for each rather than inheriting accidental behavior from the compositor.

### Vector/quality realization

Measure and decide:

- offline versus runtime rasterization;
- target pixel density for Ultra;
- whether mipmaps improve or harm the current art style;
- texture filtering rules;
- cache key/lifetime;
- how quality switches invalidate/reuse realizations;
- how much high-resolution part memory the real characters consume after structural reuse.

### Local compositor mechanism

Before choosing a custom renderer, answer:

- what exact blend/color-space behavior is required for parity with SVG/baked references;
- whether premultiplied alpha removes the current unpremultiply road without changing edge appearance;
- whether one render pass can target all dirty cache cells efficiently with the current Bevy/WGPU version;
- whether texture arrays, atlas pages, standalone textures, or another layout is simplest;
- whether CPU-side copies of render targets exist and why;
- what containment/bounds guarantees are required to prevent one cached semantic part from contaminating neighbors.

Use measured results rather than preserving the current atlas layout by inertia.

### Ragdoll/physics seam

Before adding physics components broadly, determine:

- whether the current physics stack supports the desired 2D joint behavior cleanly;
- which semantic IDs are stable enough to reference from a ragdoll profile;
- how rollback/network simulation should eventually treat ragdoll bodies;
- whether ragdoll is simulation-authoritative gameplay, presentation-only death physics, or both in different contexts;
- how animation-to-ragdoll handoff seeds velocities and transforms;
- how attachments and held items should transfer.

The renderer migration must not wait for all of these answers, but it must leave a clean pose-provider seam.

## Required tests and evidence

The implementation campaign should grow a small number of strong tests rather than preserving accidental implementation details.

### Semantic authority

- one authored semantic part identity maps consistently through rig, visual realization, and attachments;
- no visual flipbook is accepted when its semantic channels disagree with the authoritative rig;
- quality switching changes presentation realization, not semantic IDs or pose.

### Visual parity

Representative characters should cover:

- asymmetric characters;
- symmetric-half optimized characters;
- center-anchored characters;
- mirrored animation;
- expressions/face layers;
- held equipment;
- large/tall parts;
- transparent/faded presentation.

Compare against SVG/baked references at multiple quality tiers, including Ultra.

### Performance

Measure Hall and a smaller representative room with:

- current mandatory compositor;
- direct semantic parts;
- local-composition cache where implemented.

Report render-thread CPU, main-thread CPU, GPU time, memory, draw/batch counts, and loading/preparation cost. Do not accept a more complicated design solely on an estimated win.

### Future-physics proof

A development-only proof should show the same semantic-part renderer consuming transforms from both:

- ordinary animation;
- a simple physics/joint pose.

It need not be production ragdoll gameplay yet.

## Non-goals for this campaign

- Do not redesign combat around limb damage.
- Do not make every visual layer a physics object.
- Do not freeze a universal humanoid skeleton into the engine.
- Do not remove baked rendering; baked realization remains a valid presentation strategy where it is cheaper or more appropriate.
- Do not require every character to become part-based.
- Do not add a generic property bag for part semantics.
- Do not solve networked ragdoll policy before the rendering/pose boundary exists.

## Completion criteria

This roadmap is complete when:

1. semantic part identity and current pose have one authoritative road;
2. ordinary part-authored characters can render directly from semantic parts without mandatory body-impostor cameras;
3. Ultra uses vector-derived high-resolution realizations rather than blurry upscale paths where vector truth exists;
4. nonphysical visual layers can be resolved/cached locally without becoming physical parts;
5. one-image consumers are migrated or explicitly use optional group composition;
6. the mandatory whole-body compositor is removed from normal part presentation;
7. a bounded ragdoll/physics proof can drive the same semantic part renderer through the pose-provider seam;
8. measured performance and visual parity are at least as good as the replaced road, with the architecture materially simpler and with fewer duplicate authorities.
