# Presentation order and optional 2.5D rendering

**State:** PO1 landed 2026-11-25; PO2 and RV1-RV3 PLANNED. Selected by the maintainer on 2026-11-21. The customers
are Scout inside and outside the cart, consistent order for body parts and
effects, and Bevy 2D and 3D presentation tools that can coexist. Production
source reviewed: `f41218979625`; planning includes the preceding runtime
contract overlay. No build, image capture or performance measurement ran.

**Execution:** [PRESENTATION-COMPOSITION](../queue.md#presentation-composition--order-appearances-and-add-a-25d-view).
This page owns scene/local ordering and the interface between shared sampled
appearance and concrete renderers. [PVS](runtime-rigged-sprite-animation.md)
owns visual sampling and demanded composition; [portal views](portals-one-transit-for-every-mover.md)
own portal-view projection and capture; [semantic parts](semantic-part-rendering-and-ragdolls.md)
own part identity. Keep their existing authorities.

## Product direction

Keep the simulation in its current 2D world. Add an optional presentation
that uses Bevy's 3D mesh, camera and lighting tools. Existing sprite sources,
2D previews, captures and runtime rendering remain supported. Begin with a
side-on orthographic view and a cart mesh beside the current 2D view of the
same simulation. Oblique views and richer spatial art follow explicit assets
and a measured customer; a raster image contains no unseen back faces.

The immediate cart repair belongs in the current renderer. It must not wait
for the 3D path. A change of renderer cannot change combat depth, support,
control, collision, pose clocks, portal passage or rollback outcomes.

The source pins Bevy 0.19.1. Its [Camera3d](https://docs.rs/bevy/0.19.1/bevy/camera/struct.Camera3d.html)
selects the 3D render graph, and [Mesh3d](https://docs.rs/bevy/0.19.1/bevy/prelude/struct.Mesh3d.html)
uses a 3D material realization. The official
[manual Mesh2D example](https://bevy.org/examples/2d-rendering/mesh2d-manual/)
also has depth and opaque/mask passes. Therefore depth is not a reason to
replace all 2D rendering. The selected 3D consumer needs spatial geometry,
camera and lighting tools; shared presentation meaning makes that choice
composable within Bevy.

## Evidence: one floating-point z carries unrelated meanings

| Current source | Consequence and scope of evidence |
| --- | --- |
| [Cart placement](../../../crates/ambition_render/src/rendering/moving_platforms.rs): panel z 14.5, seated root z 14.45 | A fixed 0.05 gap describes a root, not the whole visible appearance. |
| [Hit flash](../../../crates/ambition_render/src/rendering/hit_flash.rs) adds 1.5 to source z | A seated flash reaches 15.95, in front of the rail. This is an arithmetic consequence, not a captured frame. |
| [Wielded item](../../../crates/ambition_render/src/rendering/wielded_item_visuals.rs) uses player z plus a bias, even when x/y follow a hand | Moving the holder behind a panel does not move this item into the same order. |
| [Rig slots](../../../crates/ambition_render/src/rendering/actors/rigged.rs) use local index times `1e-4`; [same-plane actors](../../../crates/ambition_render/src/rendering/actors/depth_plane_tests.rs) retain equal root z | Overlapping appearances can interleave by part index: A's first part is below B's second, and B's first below A's second. Whole-body tie handling alone cannot fix this. |
| Cart seating asks whether any vessel holds a body and writes one absolute z | It loses the containing occurrence and the body's prior scene order. Two carts cannot own independent interior placements. |
| That actor branch uses [unqualified feature lookup](../../../crates/ambition_sim_view/src/view_index.rs) | Duplicate authored IDs in two live rooms return no feature. Normal actor placement already uses room-qualified lookup. |
| [Render scheduling](../../../crates/ambition_render/src/rendering/mod.rs) orders seating after actor placement, but has no seating-to-rig-driver edge | The driver copies root placement into a separate presentation owner. Source does not guarantee this frame's seating reaches its parts. This is an ordering risk, not a reproduced flicker. |
| [Portal carry](../../../crates/ambition_portal2d_presentation/src/carry.rs) skips free drawables at z <= 9.5 as ground or carried copies | Numeric order also decides eligibility. Changing ranks alone can silently exclude a drawable from portal carry. |

The cart's current local test checks row names and inequalities among z
constants. The [cart gameplay fixture](../../../game/ambition_app/tests/the_cart_of_the_wake_room.rs)
checks walking on its bed/ramp. Neither establishes final order of actual
rig parts, held items and effects.

The engine also has a real mechanical
[`DepthPlane`](../../../crates/ambition_platformer2d_core/src/body_clusters.rs).
Nonzero depth excludes a body from several combat/contact/targeting roads.
Surface-motion lanes also affect movement. Do not change either fact merely
to draw Scout behind a panel. Room/view `RenderLayers` control visibility,
not local paint order. None of these becomes a universal z coordinate.

## Scene placement, local composition and membership

Use a small hierarchy of ordered appearances. These are responsibilities,
not required public type names or a new scene database.

| Fact | Owner and meaning |
| --- | --- |
| Source occurrence and room | Existing live identity; every relationship selects the exact object and room, not only its authored name. |
| Scene placement | Coarse visual plane/order and an explicit sort policy. Mechanical depth is an input where relevant, not a value the renderer rewrites. |
| Local composition | Prepared ordered slices and named insertion slots. Ordinary bodies retain their published part order. |
| Placement relation | A selected member appearance occupies a named slot of an exact owning occurrence. Its reason comes from an existing domain relation or a bounded visual-region rule. |
| Observer/projection | Existing local view and scene lifetime, with directed path/clip identity for portal images. |
| Backend order | A derived order for the actual fragments this renderer draws. Authored content does not allocate float epsilon ranges. |

For the cart, local order is `rear`, then `interior`, then `near_panel`.
Scout's appearance occupies the interior slot while inside. On leaving,
it resumes independent scene placement, which this room authors in front
of the cart. The same operation applies to an NPC, possessed actor or mount
rider. Player and feature presentation must not maintain different order laws.

An illustrative source binding is:

```ron
composition: [
    Layer(name: "rear", row: "idle"),
    Slot(name: "interior"),
    Layer(name: "near_panel", row: "front"),
],
places: {
    "interior": (region: "bed_interior", support: "bed"),
},
```

This is a proposed meaning, not current RON syntax. Extend the existing
`PlatformLook` and art-owned place/binding roads only as the cart requires.
The art source owns its panel and interior landmarks; room content binds
them to the physical platform occurrence. Avoid another copied pixel table.
The [art-place plan](the-art-says-where-a-body-stands.md) and
[anchor bindings](body-anchor-bindings.md) retain their geometry authority.

### Membership is a presentation relation with a stated source

Use accepted support, active mount, capture or passage facts when they
establish the intended relationship. An authored visual compartment can
instead derive membership from a local region and a stated support/body
point. Map that point into the owner's local material frame; do not use
world-down feet as the general rule. Use one coherent presented sample for
the body and moving owner. Render interpolation must not invent mechanical
support or make an art rectangle into a collider.

For the cart, jumping while still inside must not move the whole appearance
in front of the panel solely because ground support ended. Author the local
compartment and its entry/exit rule to express that case. A support fact can
establish a relationship without being its complete visual lifetime rule.

Retain the actual selected owner and slot, not an `inside_anything` bit.
For mounts, use the active relationship; `RidingOn` can remain after mount
death. A purely visual slot needs no new rollback passenger state. If a
gameplay feature later owns occupancy, consume that authority rather than
keeping a competing renderer decision.

Room membership, physical attachment, transform parenting and appearance
placement are distinct. Inserting an appearance into a cart's order does
not change the actor's physical parent or movement authority. Membership
rebuilds from the selected state after rewind, removal or a presentation cut.

### Whole appearance order does not require one raster image

An ordinary body occupies one order block containing its ordered parts and
applicable surface effects. Its hit flash remains inside that placement.
A held item can share the placement and still sit before/after its named
hand. An intentional foreground effect states a different presentation role;
it does not escape by adding an arbitrary large z offset.

Independent limbs and long weapons can require explicit fragments or local
relations. Do not flatten every attachment into a rigid body or erase the
existing policy for a fist intentionally drawn above bodies. World labels
and screen HUD also have explicit presentation roles. A descendant alone
does not decide whether something is a surface effect, independent object
or screen overlay.

Keep direct part rendering and batching. A contiguous order block is a
logical ordering guarantee; it is not a demand for another compositor camera.
A flattened color image can preserve one resolved painter result but loses
independently interleavable depth. If a panel must sit between parts of an
appearance, retain the parts, separate slices, a mask or depth-bearing
geometry. Do not reconstruct depth from the flat color image.

### Order assemblies before their internal slots

Do not define every panel as behind every nonmember. If two occupied carts
overlap, that rule asks each cart's panel to be below the other's occupant
and above its own occupant. It can form a cycle.

Choose scene order first. With A behind B, the valid local expansion is A's
rear, occupant and panel, then B's rear, occupant and panel. A's occupant is
outside B without being in front of B. A free actor has its own scene
placement. True interleaving between these assemblies requires split
geometry, masks or different authoring; silently deleting an order relation
does not produce the requested picture.

Default local order is a prepared list/tree. A member selects one owner slot
per projection scope. Bound nesting and reject static missing references or
cycles at preparation. Resolve competing region candidates through their
support/relation owner and declared scene priority. Residual ambiguity needs
a stated policy, not Bevy query order.

Before accepting a changed instance placement, check its exact-owner ancestry
for a cycle and the nesting bound. A valid prepared template does not prevent
two live objects from selecting each other. Refuse that local placement with
its reason and declared fallback; do not partially apply it or delete the
last visited relation. This is a bounded ancestry check on changed placement,
not a whole-room graph solve.

Equal scene placements use stable occurrence identity and authored local
ordinal as the final tie. Numeric 2D lowering must reserve complete blocks,
including effects, and detect unrepresentable rank/capacity. It can use a
render-phase sort key or a checked numeric mapping; it cannot assume ever
smaller `f32` offsets remain distinct. Do not sort all object pairs each
frame. Re-evaluate local membership candidates when their source relation,
member/owner presented pose, scene placement, relevant generation or
projection changes. Resolve affected assemblies that are visible or explicitly
demanded by a consumer. Ordinary objects keep a direct scene-order path.

## One shared sample, two concrete Bevy renderers

The existing foundation is useful: `BodyPoseView`, `FeatureViewIndex`,
`PresentedPose`, local views, and immutable `RiggedSpriteAsset`/`PartDraw`.
Gameplay already supplies facts without requiring a visible renderer.

The current [rig driver](../../../crates/ambition_render/src/rendering/actors/rigged.rs)
still requires mutable `Sprite` and `Anchor` on its roots. `RiggedPresentation`
mixes sampled draws and blend/follow state with 2D slots and compositor cells.
Its `frame_draws` is the shared sampling nucleus. The existing
[`PartSetDraws`](../../../crates/ambition_render/src/rendering/actors/rigged/part_set.rs)
is useful but is produced by that 2D driver, contains placed transforms,
omits tint/opacity, and has no complete baked-row answer. It is not yet a
complete interface for another renderer.

Extend [PVS-2](runtime-rigged-sprite-animation.md#pvs-2--sample-once-for-consumers-at-the-same-presentation-cut)
so one complete local sample exists before backend materialization. Keep:

- source/program generation, exact presentation cut and selected clip/phase;
- ordered parts or the admitted baked-frame alternative, with local
  transforms, pivots, texture regions, complete RGBA and final bounds;
- active appearance/group operations, including whole-body opacity and
  warps that cannot be distributed over loose parts;
- presented placement, physical material, visual basis and visibility as
  explicit inputs at their own stage;
- unavailable/not-ready state and reader lifetime, rather than a blank
  sprite standing in for a valid sample.

These can remain small domain values and borrowed admitted arrays. No giant
public scene DTO, property bag or universal renderer trait is required.
Preserve the current operator order and one set of presentation clocks and
blend/follow state. A second renderer must not reimplement that interpretation
or read its input back from the first renderer's output entities.

Share samples only for equal program, cut, phase, space and operator inputs.
Simulation geometry can use a different clock. Sharing the pure sampler
does not make rendered output the authority for hits or attachments.
Keep the existing demand-driven group compositor where its semantics are
needed. PVS-3's later raster/finishing optimization is not a pilot prerequisite.

The two concrete adapters are the existing Sprite/Mesh2d realization and an
optional Mesh3d/card/mesh realization. Their slot entities, GPU materials,
targets and completion state remain separately owned. Physical `BodyMaterial`
is a room-space similarity map; it is not a GPU surface material. Preserve
its scale/parity once at the chosen scene embedding, independently of texture
density and the logical paint rank.

## Views own camera and rendering choices

Two views in one App can select different installed renderers over the same
simulation. A single mixed scene needs one world depth/compositing owner:
realize existing art as textured 3D cards, or use an explicit image surface.
Stacking independent Camera2d and Camera3d passes does not interleave their
object depths. UI and other explicit overlays can still have separate passes.

The current [camera follower](../../../crates/ambition_render/src/rendering/camera.rs)
writes every `MainCamera` to an XY position and Z-only rotation. Adaptive
view construction in
[`platformer_presentation`](../../../crates/ambition_render/src/platformer_presentation.rs)
always creates Camera2d. Split observer/layout policy from concrete camera
creation and projection application. Only the selected adapter writes its
camera. Do not repair a 3D camera after a 2D system overwrites it.

Reuse `LocalView`, `PresentsView`, `PresentedForView` and room isolation.
Derived roots name their source and owning view/realization. Shared 2D views
need not duplicate ordinary world sprites. Per-view materialization must
not create another simulation body. Render layers need explicit descendant
propagation under the existing isolation contract; they are visibility masks.

The plane-framed orthographic pilot can consume `CameraSnapshot2d`. Keep
that name and scope: it does not become a complete perspective/3D camera
model. Apply the gameplay-to-Bevy axis conversion once. Logical ordering,
physical XY material scale and authored visual thickness have different
units; do not reinterpret old global z bands as mesh dimensions.

The existing cube menu already composes 3D and 2D cameras. Preserve its
target format, MSAA, clear/order and color-composition lessons. Prove the
pilot in one real window with both views; separate offscreen images alone
do not establish window-stack compatibility.

## Depth, transparency and portal composition

Opaque or cutout geometry can use a shared depth buffer. Blended card
intersections, group fades, filtering fringes and layered art still need
an explicit material/order policy. Bevy's
[AlphaMode](https://docs.rs/bevy/0.19.1/bevy/prelude/enum.AlphaMode.html)
distinguishes masks, blending and alpha-to-coverage; the last depends on
MSAA. Do not turn all existing antialiased art into hard cutouts and call
the appearance preserved. Start unlit for the comparison; add lighting and
normals only with authored meaning and visible acceptance. More costly
transparency techniques need a measured case rather than a default mandate.

`fade(composite(parts))` differs from fading every part. A required group
image retains that meaning but is one depth unit unless it also supplies
the required depth/layer information. The renderer must state which consumers
can use that image and which need the original fragments.

Portals have three separate order contracts: source appearance order,
observer-relative pane overlap, and capture/pass dependency. Preserve all
three. Source ranks must survive projection; equal source z must not collapse
distinct occurrence ties. Keep the existing world-window replacement and
foreground-atmosphere policy at its compositor stage, rather than turning
every drawable's depth into a portal-window number.

Carry eligibility is a separate source/pass role. Replace the current
`z <= PORTAL_EXIT_COPY_TOP` classification when order lowering changes, or
retain that numeric contract explicitly until its consumers migrate. An
eligible body's new rank must not make it disappear from portal carry.
This needs an owned role at the existing source boundary, not a global
taxonomy for every render entity.

Current [carry](../../../crates/ambition_portal2d_presentation/src/carry.rs)
recognizes Sprite/Mesh2d/Text2d, capture cameras are Camera2d, and
[`clip_material`](../../../crates/ambition_portal2d_presentation/src/clip_material.rs)
implements Material2d. The current observer registry also admits only one
eye per live room. The existing P5/P6 work owns those view boundaries.
A 3D adapter needs its own capture/clip realization of the same admitted
material partitions, path, art coverage and presentation order; a new mesh
bundle does not inherit the 2D shader contract.

The first non-portal cart pilot can report portals unsupported. General
coexistence cannot silently omit carried Mesh3d objects or reuse one room's
eye for both views. Before rollout, prove two observers of the same room,
ready images at the selected cut, reflected/scaled parts and independent
source roots. Retain current pools and complete family capture. Add the
portal owner's planned PV4 direct-image specialization only where its complete
image/order contract holds. Do not flatten a room to avoid this work.

## Authoring and installation

The current cart source is
[`targets/props/intro_cart.py`](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/props/intro_cart.py),
with a provider-owned look in
[`platform_looks.ron`](../../../game/ambition_content/assets/data/platform_looks.ron).
Add named composition slots and the pilot mesh/material binding beside
that owned source. A small code/data-authored mesh is enough initially;
do not require a universal editor/export format or another authoring repo.

Reuse the [prepared media](asset-authoring-and-prepared-media.md) source
identity, immutable publication, quality and residency roads. Existing part
textures can feed both adapters. Add mesh/material demand only for a view or
other consumer that selects and needs that realization. Do not preload every
richer realization or make gameplay depend on its highest visual tier.
Geometry anchors remain independent of raster density.

Features select compiled capabilities; a view selects an installed renderer.
Keep the current 2D default and make 2.5D an explicit optional capability.
Both can compile together; headless excludes both. Name each adapter's Bevy
features instead of relying on another tool's feature unification. Move the
shared quality/readiness/startup anchors that the second consumer needs out
of the 2D installation path, without duplicating asset installation.
Do not parameterize the gameplay engine by a renderer type or copy a large
generic implementation for each backend. Two concrete plugin/adapters and
shared value preparation are sufficient until source proves otherwise.

## Delivery and deletion

1. **PO1 — cart and appearance order.** Implement scene/local placement for
   Scout, another actor beside the cart, actual rig parts, flash and held/
   wielded item. Preserve mechanical depth and exact room identity. Resolve
   placement before part and effect realization with an explicit schedule edge.
   Those writers finalize geometry in `BodyOwnedDrawableSync`; folded meshes
   and portal carry consume the finalized result. Delete the post-hoc absolute
   seating write and `IN_VESSEL_Z` once this path replaces them.

   **Landed (2026-11-25).** `IN_VESSEL_Z` is gone. A vessel owns its seats
   (`Vessel::seat_z`): a body in it is placed one whole `BODY_DEPTH_BAND`
   under the vessel's own near side, so its rig parts, flash (+1.5) and the
   art in its hands, all drawn within the band, are hidden with it. Bodies
   in one vessel take its seats in entity order, `SEAT_DEPTH_STEP` (0.01)
   apart, so two seated bodies are two order blocks. Seating runs after
   `actors::sync_visuals` and before `BodyOwnedDrawableSync` and
   `HeldItemPresentationSet`, so every drawable placed from the root reads
   the seat in the same frame. An actor is matched in its own live room
   (`FeatureViewIndex::get_in`). A posed wielded item takes the hand's z plus
   its bias, not `WORLD_Z_PLAYER`, and its bias must lie within the band.
   Captures (`capture_scene intro_wake_room ... --player-flash 20 --warmup
   120`, new option): seated, the flash silhouette shows only between the
   rails; at the old depth (control, front − 0.05) it covered them; beside
   the cart it is in front of the wheels.

   A limb drawn apart (`LimbTwin`) of a seated body stays in its seat
   (2026-11-25): seating marks the body's visual (`SeatedInVessel`, removed
   when it leaves the cart or the cart goes), and the twin then takes the
   root's seat depth plus its twin step instead of flooring at
   `LIMB_OVER_BODIES_Z`. The twin follows the seating in schedule order. A
   wielded item placed by its support takes its holder's hand depth too
   (2026-11-25); only a holder with no posed hand keeps `WORLD_Z_PLAYER`.

   **Same-plane actors (2026-11-25).** Two actors of one plane no longer
   share one z: each takes a tie fixed for its life (`occurrence_tie`,
   0.01 steps by entity index mod 16), above its own parts' spread, under a
   depth plane and with its flash under an enclosure. Players take the same
   tie; the bubble shield, the one overlay drawn over every body of the
   layer, is drawn above the whole span (`OCCURRENCE_SPAN` + 0.05). The
   trapdoor and morph ball replace a hidden body and the rope is under it,
   so their absolute depths stand.
2. **PO2 — common placement consumers.** Move player/feature bodies,
   attachment order and a real active rider onto the same placement authority.
   Keep explicit effect roles and independent limbs. Replace absolute player-z
   item biases and repeated private part-rank calculations as their consumers
   migrate. Keep local lists compiled and changed membership work bounded.

   **Item depths follow their holder (2026-11-25).** Two consumers still
   placed at the player plane. A wielded item whose holder has no posed hand
   took `WORLD_Z_PLAYER`; it now takes the depth the holder's root is drawn
   at (`VisualRoots::drawn_z`, `PosedHands::root_z`), so a seated or tied
   holder's item is in its seat. A shared load's string (`supported_carry`)
   was drawn at `WORLD_Z_PLAYER - 0.01` whatever the holder's foot depth; it
   is drawn just under the foot it hangs from (`UNDER_HOLDER`), or under the
   holder's root when no foot is posed. Still open: bodies and features,
   attachment order and an active rider on the vessel's placement, and the
   private part-rank calculations.
3. **RV1 — shared sample and view ownership.** Complete the needed PVS-2
   extraction, with the current 2D path as its first consumer. Separate
   camera/application and device roots by view/renderer. Remove the old
   duplicated interpretation; an unused wrapper around it is not completion.
4. **RV2 — one orthographic 3D cart view.** Use the same sampled actor parts
   as unlit cards, a simple authored cart mesh and one declared visual-depth
   policy. Draw it beside the 2D view in one App. Preserve the same platform,
   support, actor material, animation clock, held-item facts and input path.
5. **RV3 — integration for rollout.** Carry logical order and the new
   realization through existing portal P5/P6; then add required group effects,
   capture/UI integration and target profiles. State unsupported capabilities
   until proved. A successful cart pilot does not certify all scenes or
   arbitrary perspective views.

PO1 can proceed before RV1/RV2. The scale-law work can proceed independently;
both renderers must consume the resulting authoritative material and poses.
Keep the maintainer's product order in the queue.

## Focused acceptance and cost boundaries

- Exercise actual visible parts/effects for Scout on the bed, another actor
  alongside, jumping inside, leaving, flashing and holding an item. Check
  same-plane bodies
  as separate order blocks, two cart assemblies at different scene placements,
  and two live rooms with the same authored actor ID. Constant inequalities
  alone cannot prove any of these boundaries.
- At the local owner, exercise one valid nested slot, stable ties and a
  conflicting membership/cycle. If numeric lowering has a bound, exercise it.
  A selected-object readout can explain source, room, scene order, slot,
  membership reason and final view rank from the same resolved values.
- Simulation and semantic facts remain unchanged with either renderer,
  both, or neither. For an explicit sample request at equal admitted inputs
  and cut, the result is independent of the installed renderer set. With no
  reader, no sample materialization is required. Create/retire each view
  independently; exercise
  content/quality replacement and material reflection/scale. A 2D view's
  composed-image demand cannot remove the 3D view's sample, or vice versa.
- Capture one real window containing both views. Inspect the panel edge,
  held art, flash, alpha filtering and a group fade. If portals are claimed,
  use two views of the same room with transformed carried parts.
- Measure sampling count, changed-assembly/order work, draws/batches,
  main/render CPU, GPU passes and retained bytes. Ordinary objects must not
  pay for a global relation solve; a second view must not double animation
  interpretation merely because it has another backend. No new broad test
  matrix, source-text guard or frame-time claim follows from this plan alone.
