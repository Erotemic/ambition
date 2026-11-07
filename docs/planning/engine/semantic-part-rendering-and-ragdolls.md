# Semantic Part Rendering and 2D Ragdoll Roadmap

**Authority:** semantic-part identity and pose belong to the character rig, not to
an independent renderer skeleton. A ragdoll pose provider is optional cosmetic
presentation; gameplay collision for a persistent corpse or prop belongs to
region mechanics. See [ADR 0038](../../adr/0038-persistent-objects-and-optional-physics.md)
and [architecture reassessment](architecture-reassessment-and-composable-multiplayer.md).

## Current implementation and remaining work

This status is a consolidation of the October 5–26 implementation notes, **not
fresh verification against HEAD**. Recheck the relevant code and published art
before making changes. Completed implementation diaries and detailed benchmark
runs remain available in Git history.

**Shipped baseline:** semantic parts normally render directly as world sprites,
following the root's transform, mirror, visibility and tint. The body requests
an optional composed image only while a consumer needs one (and for a 30-frame
hold to prevent thrash). Known consumers include hit flash, portal transit and
far-side pieces, puppy-slug dream, Mary-O star power and whole-frame fades.
`FrameInSprite` describes a composed frame's in-cell bounds. Cell margins
include the authored art overhang and tween envelope; pages do not regrow in
place and unused last pages retire. `AMBITION_PART_PRESENTATION=impostor`
remains an A/B comparison knob. Owner:
`crates/ambition_render/src/rendering/actors/rigged/`.

**Measured decisions to preserve:** scene and composed images use compatible
linear-light blending. The attempt to put only the world camera in sRGB
compositing broke the multi-camera window stack; do not reintroduce it without
fixing the camera format contract. Whole-frame translucency needs group
composition (`fade(composite(parts))` is not equivalent to fading loose parts).
`measure_rigged_parity.py` and `measure_composition_switch.py` established the
parity/transition baseline; the historical host used llvmpipe. Some thin-outline
differences from the SVG/PIL gamma reference remained an art-direction trade.
Preserve the common camera-format witness, composition-switch coverage and
runtime-law parity rather than treating the early gamma experiment as an open
migration. **Still needs human visual acceptance:** compare the remaining
thin-outline differences against published SVG/PIL art on the target GPU before
changing the agreed blending policy.

**Measured throughput (historical, 100 actors):** direct parts 9.65 ms per
frame versus composited 56.3 ms in `examples/rigged_sprite_bench.rs --render`.
These are not current hardware performance claims. Whole-page compositor
redraws remain a possible cost even though demand is sparse.

### Open: held-item pose and strike agreement

`rendering/posed_hand.rs` reads the active posed hand from `PartSetDraws` for
both in-hand and over-hand visual roads. The item follows the animated hand;
non-part or missing-hand bodies keep the fallback. Pirate heavies and the robot
`item_swing` provide authored customers. `moveset_render --hold ITEM` exposes
this for inspection. Authored hand tracks include `near_hand`, `front_hand`,
`right_hand` and their far-hand counterparts. An unturned hand holds an item
at `ITEM_REST_ANGLE` (24 degrees above body facing); the artist turns the
hand part to swing it, rather than the game inventing another swing angle.

- Publish/review `item_swing_up`, `item_swing_down` and `item_swing_back`
  instead of silently using their fallback row.
- Align the **simulation hit volume** of a swung item with the posed hand;
  visual attachment alone does not make its gameplay geometry correct.
- Preserve mirrored-hand selection and the gun-sword's posed aiming direction.

### Open: part-group geometry and barrier lifecycle

`PartSetRequest` / `PartSetDraws` in
`crates/ambition_render/src/rendering/actors/rigged/part_set.rs` expose ordered
named tracks as placed parts. Multiple independent readers now use named
`ask_for_parts` / `forget_parts` leases; each sees only its requested tracks,
and one reader's departure preserves the others. Mary-O's cosmic barrier is
the first effect consumer. **This fixes destructive single-consumer ownership,
not general geometry or lifetime semantics.**

- The barrier shader supports at most **four parts from one atlas page with
  unrotated/unscaled geometry**. Unsupported geometry now refuses drawing,
  rather than silently truncating. The remaining implementation should render
  a complete correct group or provide an intentional visible fallback, not
  misrepresent a fifth part, a second page or a transformed part.
- A reusable part-group result should resolve all requested tracks in current
  body-pose space, support multi-page/rotated/scaled coverage and offer a
  deliberate draw-list/mask/contour form. Keep Mary-O-specific color, timing
  and pixel effects outside `ambition_render`.
- Make barrier presentation body-owned: despawn, recharacterization, rewind,
  visibility, room and `RenderLayers` changes must follow the body. Publish
  geometry before the portal-candidate boundary. Avoid a global orphan scan.
- Verify two consumers surviving independent create/teardown and cover form
  change, quality-tier change, hybrid baked row, rewind and despawn. Existing
  tests prove independent readers and safe refusal, **not** that full lifecycle.
- **Acceptance:** at least five parts over two atlas pages, a 30-degree turn,
  mirrored/scaled part, second live room, portal transit, recharacterization
  and owner despawn. Exercise the actual material/mesh, not only bounds math;
  keep headless builds valid.
- **Promotion trigger:** build a general GPU mask/contour cache only after a
  second real part-group effect needs it or published art exceeds the current
  barrier limits. Measure CPU, GPU passes, draw calls, cached bytes and quality
  tiers first. A correct named-reader API is required independently of that
  optimization trigger.

### Open: pose authority, attachments and future physics

`rigged::PosedParts` can bind flipbook tracks to body-rig joints, and
`rigged::PartPose` lets a provider drive parts from joint frames without
changing the drawing road. Mary-O's three forms measure near-exact agreement;
three tracks had small residual placement drift (0.86–0.95 px) to explain.
Pirate sheets still compute independent rotations for several parts: nine of
about 26 tracks aligned with rig joints in the earlier measurement. In
particular, face and hat cannot both use the head joint's one angle.

- Converge the pirate painter/body rig without moving authored gameplay
  attachments. Add accessory joints where parts need independent turns, then
  gate with `scripts/measure/measure_track_joints.py`.
- Publish each flipbook track's owning `track_joints` binding in authoring;
  the current `PosedParts::bind` inference is not a permanent source of truth.
- Build an optional physics/joint pose provider on a suitable existing body
  (Mary-O is a candidate). It should feed the **same** `PartPose` and renderer,
  without requiring every death to use articulated rigid bodies or changing
  the custom player movement authority.

### Open: effects, composition, quality and presentation verification

- Hit flash still requires temporary group composition to mix a white
  silhouette; a part material could eventually remove that need. Color shift
  without a production inserter remains composition-only. Audit remaining
  readers before changing their route.
- Atlas composition still redraws every body in a changed page. Improve only
  with a measured real customer; see the local semantic-part caching plan.
- A page allocated mid-play can leave its first body temporarily without a
  composed image. Validate a composed reader across that creation frame.
- A `PartPose` extending beyond its composed cell is drawn directly and loses
  composed readers (flash and portal pieces). If ragdoll/posed content needs
  those effects, select a cell from the current pose reach or an authored
  envelope. This is already selected as
  [RIG-IMPOSTOR-CONTAINMENT](../queue.md#rig-impostor-containment--a-part-drawn-body-is-drawn-whole-or-refused);
  do not spin it into a duplicate campaign.
- The part-visual `Ultra` realization still needs deliberate vector-source
  resolution/quality policy; do not upscale lower-tier rasters when SVG truth
  exists. Measure caching, package size, startup, memory and display quality.
- Recheck portal/transit clipping and far-side panes with actual part-drawn
  bodies, including posed or transformed parts. Normal direct rendering does
  not by itself prove part-aware portal correctness.
- The composition-switch study found second-sampling differences on thin
  lines. Maintain parity tests at game scale, not only geometry-unit tests.

## Enduring design and acceptance contract

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

## Migration phases (current disposition)

The numbered phases describe design dependencies, **not eight unstarted
campaigns**. Do not reimplement an already-shipped road merely because an
older phase used imperative wording.

| Phase | Disposition | Remaining gate |
|---|---|---|
| 0 — discovery/measurement | Baselines captured in October | Refresh host/room/render-thread, GPU, memory, loading and image-parity measurements before replacing a mechanism; Tracy overhead alone is not baseline |
| 1 — single semantic pose authority | Measured, partially converged | Pirate joints/accessory identity and authored `track_joints`; confirm no independent second pose truth |
| 2 — direct semantic-part world rendering | Shipped default | Preserve anchor, mirror, ordering, quality and parity checks as regression coverage |
| 3 — inherited simple effects | Partly handled by demand-driven composition | Classify flash, HSV, star power, opacity, visibility and tint; migrate only when parity/customer supports it |
| 4 — local visual-layer cache | Planned | Dirty semantic-part/cell invalidation, shared cache keys, bounded atlas targets, no per-character compositor cameras and no duplicate pose authority |
| 5 — portals and one-image readers | Composition on demand shipped | Prove geometry/transit behavior and classify each residual reader as parts, inherited presentation or required group composition |
| 6 — remove mandatory impostors | Shipped for normal world presentation | Do not remove optional group composition required by fades/portals; avoid default compositor regression |
| 7 — ragdoll proof | Pose-provider interface shipped, physics provider open | Bounded animation-to-physics provider proof using the same semantic part IDs/renderer; full gameplay ragdolls and rollback policy deferred |

For Phase 4, measure blend/color-space parity, premultiplied-alpha behavior,
per-cell dirty updates, GPU passes, page/cell containment and renderer-thread
cost before choosing a custom pass, compute path or atlas layout. Phase 7
must not become an unsolicited rigid-body dependency on every character. A
realized physics profile may choose only some semantic parts as physical;
articulated recovery, networking, limb damage and portal-aware rigid bodies
are later, customer-driven work.

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
