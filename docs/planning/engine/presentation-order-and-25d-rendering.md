# Presentation order and optional 2.5D rendering

**State:** PO1 and PO2 landed 2026-11-25; RV1 and the RV2 cart pilot landed 2026-11-25; RV3 in progress (baked frames, group fades and the hit flash landed 2026-11-25). Selected by the maintainer on 2026-11-21. The customers
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

When a room has more than one vessel, both render phases use its room
instance and platform identity to order the assemblies. Each assembly has
rear, rider, and near slots. Held art and body effects with a visual root
follow that root's slot. The card view keeps the physical depth of each
slot. A headless test checks rider selection when two beds overlap. Ultra
captures of two occupied carts show the same overlap in 2D and side-on 2.5D.
When a portal cuts a panel, rider, or item, the piece at the entry keeps
its source slot and physical depth. The piece through the portal uses the
exit layer. An Ultra fixture checks the overlap while a rider, the cart,
and a held item cross a paired opening.
When a rider's limb enters a portal, the local pieces keep the rider's
seat. The card view draws the limb and the body parts above it on the
same plane, in that order and behind the near panel.

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
implements Material2d. The portal cone renderer keeps each window by room,
observer, and channel. P5/P6 own the remaining portal view boundaries.
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
   share one z: each takes a tie fixed for its life (`order_block_tie`,
   0.01 steps by body index mod 16), above its own parts' spread, under a
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
   holder's root when no foot is posed.

   The card view draws the same string sprite and fade. Each end uses the
   depth of the art it touches. The foot uses its body part. The load
   uses the held item. The string keeps its side-view position when its
   ends have different depths.

   **One part-rank law (2026-11-25).** A body's own slots
   (`write_slots`) and a reader's placed parts (`part_set::answer`) each
   rebuilt a part's local transform and its rank `index × SLOT_DEPTH_STEP`.
   Both now take it from `rigged::slot_local`, so a reader's copy of a part
   cannot rank differently from the part it copies. No other actor renderer
   computes a part rank (searched `ambition_render` for depth steps).

   **An active rider sits its mount's order block (2026-11-25).** A rider
   and its mount were two same-plane actors, each with a tie from its own
   drawn entity, so a rider was under its shark whenever the shark's tie
   happened to be higher. The actor row now publishes an `OrderBlock`: a
   body is seat 0 of its own simulation body's block, and an active rider
   (`RidingOn` with `Mounted`; `RidingOn` alone outlives the mount's death)
   is seat 1 of its mount's block. The renderer's `order_block_tie` draws
   the rider one step over the mount it sits, and `OCCURRENCE_SPAN` grows by
   that one seat. A row with no body (a hand-built view) is ordered by its
   drawn entity, as before. The witness is
   `an_active_rider_draws_just_over_the_mount_it_sits`. The capture
   (`capture_scene pirate_sky_lookout player --player-beside
   npc_pirate_raider --warmup 90`) shows all three raiders drawn over their
   sharks.

   **Players and features take one tie law; ropes draw under their owner
   (2026-11-25).**
   - The player road ordered its bodies by its own `occurrence_tie`. It now
     calls the same `order_block_tie` the actor road does: a player's visual
     is its body. The old function is gone.
   - The flyline rope and the grab tether (`place_wire`) were drawn at
     `WORLD_Z_PLAYER - 0.05` whoever owned them. An actor's grab line was
     therefore drawn 4.95 in front of its own body and of every actor of its
     layer.
   - Both are now drawn `WIRE_UNDER_BODY` under the owner's drawn root
     (`wire_z`), and keep the player plane when no root is drawn.
   - In the tether witness the roots sit at z 0. The expected line is at
     −0.05, where the old code drew it at 19.95.

   The morph ball and the trapdoor replace a hidden body, so their absolute
   depths stand, as stated above. The bubble shield is drawn above the
   layer's whole span. PO2 is complete. No capture was taken of an actor's
   tether, because no room authors a grabbing actor at rest.
3. **RV1 — shared sample and view ownership.** Complete the needed PVS-2
   extraction, with the current 2D path as its first consumer. Separate
   camera/application and device roots by view/renderer. Remove the old
   duplicated interpretation; an unused wrapper around it is not completion.

   **The shared sample (2026-11-25).** `RiggedLocalSample`, on each rigged
   presentation owner, is the body's whole local interpretation at this
   frame's cut, made once by `sample_rigged_presentations` before any
   renderer reads it:
   - the clip frame and its tween, legs, head turn, expression, layer,
     wounds, the blend from the last row, the follow-through, a `PartPose`,
     and the track mask, in that order;
   - each draw's pivot place, scale, rotation, part (its texture region)
     and full RGBA (`PartDraw`);
   - the row, frame, the frame's own opacity, and its state (`Parts`,
     `Baked`, or `Unavailable` when not drawn or not sampled).

   Placement is not in it: each renderer applies placement at its own stage.
   The blend and follow-through clocks moved with the interpretation that
   reads them.

   The 2D driver is the first consumer. It places and draws the sample on
   both roads (direct slots and impostor cells), answers part-set readers
   from it, and reads the frame opacity from it. Two duplicated
   interpretations are gone:
   - the driver's own (`frame_draws` inside the driver);
   - `posed_reach`, which re-ran the whole interpretation, without blend or
     follow-through, to decide whether a pose leaves the cell. The cell
     decision now measures the sample itself, so the bounds and the drawing
     read the same final sample.

   Both systems run chained in `RiggedDriveSet`
   (`rigged_presentation_drive()`). Every edge that named the driver names
   the stage, so a writer of the sample's inputs (the animators, a boss's
   pose) is ordered before the sampler, not only before the driver.

   The witness is `the_slots_draw_the_local_sample_and_nothing_else`. Every
   sampled draw shows in a slot. A system between the sampler and the driver
   that removes the last draw removes it from the picture, so the driver has
   no other source.

   **Views own the camera application (2026-11-25).** `camera_follow`
   resolved a view's framing and wrote every `MainCamera` in one system, so
   a 3D camera on the same view would have been overwritten with a 2D XY
   position and a Z-only roll. It is now two systems, chained in
   `ViewCameraSet` (`view_camera_presentation()`):
   - `present_view_frames` frames each presented view once, whatever renders
     it. It applies portal continuity, shake and the finishing zoom to a copy
     of the resolved snapshot, does the gameplay-to-Bevy axis conversion once,
     and publishes a `PresentedViewFrame` (room, plane centre, roll,
     orthographic scale) in `PresentedViewFrames`. A view with no frame this
     frame has no entry, so its cameras hold, as before.
   - `apply_view_frames_to_2d_cameras` is the 2D renderer's adapter. It
     writes only gameplay cameras that are `Camera2d`. A camera of another
     renderer is placed by that renderer's adapter from the same frame (RV2).

   Two cameras on one view now step that view's portal anchor once, not once
   per camera. Every ordering edge that named `camera_follow` names the set;
   the name is gone, so no edge can order against a system that no longer
   runs. The witness `the_2d_adapter_writes_only_the_2d_camera_of_a_view` puts
   a `Camera2d` and a `Camera3d` on one view: one frame is published, the 2D
   camera lands on it, and the 3D camera keeps its transform and perspective.

   Roots by renderer: the 2D slots and impostor cells stay owned by the 2D
   driver's `RiggedPresentation`; the sample they draw is separate from them.
   A second renderer adds its own roots beside the sample and never reads the
   2D slots (RV2). Spawning views' cameras through one adapter-chosen path
   waits for the first non-2D adapter, which RV2 brings.
4. **RV2 — one orthographic 3D cart view.** Use the same sampled actor parts
   as unlit cards, a simple authored cart mesh and one declared visual-depth
   policy. Draw it beside the 2D view in one App. Preserve the same platform,
   support, actor material, animation clock, held-item facts and input path.

   **The cart pilot (2026-11-25).** `rendering::card_view`, behind the
   render crate's optional `card_view` feature (Bevy PBR, which a 2D-only
   build does not link; the facade forwards it, and the capture tools turn
   it on). `CardViewPlugin` adds a second renderer over the same simulation
   and the same sample:
   - **Camera.** One orthographic `Camera3d` per view (`CardViewCamera`, its
     own `CARD_VIEW_LAYER`), turned 35° and tilted 12°, in a viewport in the
     lower right of its target, above the gameplay rigs and under the HUD.
     Its adapter (`place_card_view_cameras`) places it from the view's
     `PresentedViewFrame`, the frame the 2D adapter reads, so it follows the
     same camera policy, shake and zoom. The viewport is measured on the
     camera's own target, and the camera stays inactive until it is.
   - **Cards.** Each rigged body's sample becomes a `CardBody` of unlit
     textured quads, one per drawn part, made from the part's page region,
     pivot, turn, scale and RGBA. The body is placed by its scene placement
     (the root's transform) times the sample's `feet`. `feet` (the frame's
     quad: size, anchor, squash, mirror row) moved into `RiggedLocalSample`,
     and the 2D direct road reads it there too, so neither renderer
     re-derives it. The atlas road keeps its own cell geometry, which is
     2D-only.
   - **Cart.** Seating publishes the vessel boxes it already finds
     (`PresentedVessels`, only while a renderer asked for them), and each
     becomes a cart of five boards: floor, back, two sides and the near panel.
   - **Depth policy.** It is declared once (`card_view` module docs) and does
     not reuse the 2D z bands. The play plane is 0. A vessel's bed is
     recessed `VESSEL_DEPTH` (40) behind it. A seated body (`SeatedInVessel`)
     stands mid-bed, between the back panel and the near panel. Bodies on one
     depth are ranked by their logical order, 2.5 apart, and a body's parts
     are 0.05 apart in paint order.

   Witnesses (`--features card_view`):
   - `the_cards_and_the_slots_draw_one_sample`: each sampled draw is one
     visible card at its slot's place, and the card body stands where the 2D
     owner stands. A draw removed from the sample before both renderers is
     drawn by neither, so neither renderer reads the other.
   - `a_seated_body_is_between_the_cart_panels_and_others_are_in_front`.

   Capture: `capture_scene intro_wake_room player OUT --card-view --warmup
   120`. One picture holds both views (the capture adopts the card camera
   into its image, as it does the pause cube's). Scout stands in the cart's
   bed behind the near panel, which hides the feet as the rails do in 2D, and
   Dennis and the ice bag stand on the play plane in front.

   The simulation is untouched: the pilot only reads presentation facts, and
   the animation is interpreted once for both renderers (one sampler).

   Not drawn in this view, which is RV3's list: baked-frame bodies, held and
   wielded art, the hit flash, group fades (a fading frame fades each card),
   portals, room tiles and parallax. Opaque parts use an alpha mask at 0.5,
   so filtered edges are hard. The view is a pilot that `capture_scene`
   installs; no game turns it on.
5. **RV3 — integration for rollout.** Carry logical order and the new
   realization through existing portal P5/P6; then add required group effects,
   capture/UI integration and target profiles. State unsupported capabilities
   until proved. A successful cart pilot does not certify all scenes or
   arbitrary perspective views.

   **Baked frames (2026-11-25).** A rigged body on a baked clip (a hybrid's
   baked row) drew nothing in the card view. Its sample now carries the frame
   the animator chose (`BakedFrame`: the page region, the quad the animator
   gave the root, its anchor and mirror), and the card view draws it as one
   card at the root. A card body whose page is not loaded yet shows no card;
   before, it kept the last clip's parts on screen. The witness is
   `a_baked_frame_is_one_card_at_the_root`: on a part clip there is one card
   per draw, on the baked clip one card at the root carrying the animator's
   region and quad, and back on parts one card per draw again. Render lib:
   529 passed (default), 532 with `card_view`.

   **Group fades and the hit flash (2026-11-25).** A fading frame faded each
   part card on its own, so a part showed through the one in front of it. A
   body the 2D view composites (a fade, a flash, a warp, any reader of it as
   one image) is now one card of its composited cell, at the root's quad,
   drawn opaque because the cell already carries the fade: the card view
   follows the 2D compositing decision instead of making its own. A whole
   frame uses the shared flash shader. When a rig draws from parts,
   `PresentedFlash` supplies the tint and opacity for each part's flash.
   `a_rigged_body_flashes_its_parts_only_without_a_whole_frame` checks both
   paths. A capture of `intro_wake_room`
   with `--card-view` is pixel-identical to the same capture at the parent
   commit (no body flashes or fades there).

   That capture seemed to contradict the RV2 receipt: Scout's lower body
   showed above the cart's near panel. Resolved 2026-11-25 (probe, deleted).
   Scout is seated (depth −20), the bed is at plane y −70, and the near
   panel is the published rim, 12.7 units (38 art px at the cart's 1/3
   scale), at z −1 to −4. Pixel columns under Scout in the capture run:
   body, one or two rows of the panel's top edge, then the panel's wood,
   with no body pixel on the wood, so the panel occludes him. What shows
   above it is his legs and hands, not his soles. At the default 12°
   pitch, the 17.5 units between the seat and the panel lift the body
   about 3 units against the panel. The card view therefore shows about 3
   units more of a seated body than the 2D rails do, the declared framing.
   The RV2 sentence "hides the feet" holds; the legs above them show.

   **Held and wielded art (2026-11-25).** Both 2D roads (`sync_held_item_visual`
   and `sync_wielded_item_visuals`) now mark their sprite with the root that
   draws its holder (`HeldArtOf`, from `PosedHands::root`, kept current each
   frame). The card view records each drawn root's card depth
   (`CardRootDepths`) and draws each such sprite as a card at the sprite's
   place on that depth. The sprite's 2D depth offset from the root, counted
   in part slots, becomes the same number of card part steps, so the item is
   between the same two parts in both views. Art at no hand (an absolute 2D
   depth) is held within the body's band (±1). Witness
   `held_art_is_a_card_on_its_holders_depth`: three slots in front of the
   root is three part steps in front of the body; art at no hand stays in the
   band; a hidden item, or one whose holder has no card, shows none. Render
   lib: 529 (default), 534 (`card_view`).

   A `goblin_encounter` capture with `--card-view` shows an empty card view,
   as it did before today's RV3 edits. That is the declared framing, not a
   fault (probe, deleted): the card camera is active and centred on the 2D
   frame's centre (x −515), and Scout's root is at x −675 with his parts
   sampled. At `zoom` 2.5 in a 0.42 viewport the card view shows the middle
   ~227 world units of the 2D view's 568, and the 2D camera, held at the
   room's edge, has Scout 160 units off its centre. Held art is witnessed by
   the unit test only; no capture has a held item in the card frame yet.

   **Bodies drawn from a sheet (2026-11-25).** A body with no rigged
   presentation drew nothing in the card view. An actor (`BoundFeatureKind`
   `Actor`) or a player (`PlayerVisual`) whose 2D sprite is not a rigged
   root is now ranked in the same pass as the rigged bodies, by its 2D z on
   its depth. It is drawn as one card of the cell its sprite shows
   (`realize_card_sprite_bodies`). Its depth goes into `CardRootDepths`, so
   an item in its hand finds its holder. Props, pickups and hidden bodies
   get no card. The held-art road and this one share the sprite-to-card
   helpers. Witness `a_sheet_drawn_body_is_one_card_ranked_on_its_depth`:
   the player and an actor, one depth step apart in logical order, with no
   card for a chest or a hidden actor. Render lib with `card_view`: 535
   passed. Capture `intro_fell player --card-view --player-beside
   npc_fell_sheep`: the sheep stands behind Scout in the card view, in the
   same order as in 2D. The same capture with the old `card_view.rs` has no
   sheep there. The rigged shepherd is drawn in both.

   **Room terrain (2026-11-25).** The card view drew bodies on nothing.
   The room's ground is what the 2D view draws for its tiles: the skin of
   its blocks (`TerrainSkinMesh`, a few textured meshes per room) and the
   earth under its lines (`ChainEarth`, a hill's fill). These are the same
   mesh assets here, drawn as cards (`realize_color_meshes`). Block skins
   draw at `TERRAIN_DEPTH` −0.5 in source layer order. Filled ground draws
   0.1 behind them. Track earth draws in the source order of its lane.
   The card material keeps the texture, vertex colors, and opacity of the
   source mesh. A seated body is recessed behind the play
   plane and is behind them, as the depth policy places it. The 2D meshes
   carry no normals, and the unlit 3D pipeline draws them as they are.
   The terrain card test checks mesh identity, layer order, visibility,
   and removal. An Ultra fixture shows untextured ground and the default
   attack sweep in both renderers. The `intro_fell` capture shows the
   shepherd, the sheep, and Scout on the turf, with the hill behind them.

   **Portals (2026-11-25, `portal_render`).** A portal is a line of light
   on its host surface. The 2D view draws it as the `PortalGlow` sprite,
   placed by `sync_portal_glows`. The card view reads the same glow after
   that system, so it uses the frame's placement, length, channel colour
   and open/close fade. It draws one unlit card per glow
   (`realize_card_portals`) on the play plane at `PORTAL_DEPTH` −0.25,
   between the terrain and the bodies, as wide as the glow's length and 3
   units thick. Materials are cached by colour. Witness
   `a_portal_is_a_line_of_light_on_the_play_plane`: the card takes the
   glow's place, turn, length and channel colour, and leaves with the
   glow. Render lib with `card_view,portal_render`: 5 card-view tests
   passed; with `card_view` alone: 4. Capture `intro_fell player
   --card-view --player-beside npc_fell_sheep` with two upright portals
   placed beside the sheep (`--portal-at`): the card view shows the cyan
   and the pink line where the 2D view draws them, behind the shepherd.
   The portal's view of its far side (P5/P6, the window over the play
   plane) is not drawn here.

   **Room scenery (2026-11-26).** A room's props (`PropVisual`) and the
   trim pieces on its lines (now marked `ChainTurfPiece`) are cards of the
   cells the 2D view draws, at their 2D places (`realize_card_scenery`).
   Their order is the 2D view's, band by band:
   - backdrop props (2D z −12..−10) behind a vessel's back panel
     (`BACKDROP_DEPTH`, −45);
   - the turf (2D z 2) just in front of the block skin's layers
     (`TURF_DEPTH`, −0.4);
   - scenery props, doors and climbables (2D z 10.55..10.97) in front of
     the turf and behind the portal line (`SCENERY_DEPTH`, −0.35, plus their
     2D order);
   - what 2D draws in front of its bodies (a prop that hides a body, a near
     lane's turf) one body step in front of the front-most body.

   They use the 2D blend: a prop picture has soft edges, and a mask would
   make hard cutouts of them. A prop ranked as a body (a bound feature) is
   left to the bodies. Only a prop's own sprite is drawn: a prop that draws
   extra art on child sprites shows its root picture alone. Witness
   `a_rooms_props_and_turf_are_cards_in_the_2d_order` covers each band's
   depth and the order skin < turf < scenery < portal < bodies. No card is
   made for a hidden prop, the actor prop is a body, and a despawned prop
   takes its card with it. Render lib with `card_view,portal_render`: 6
   card-view tests passed. Capture `intro_fell player --card-view
   --player-beside npc_fell_sheep`: the hill in the card view now has its
   grass cap (bare earth before), and the flowering gorse prop stands on
   the slope at the panel's edge.

   **Parallax backdrop (2026-11-26).** The 2D view draws a room's parallax
   panels (`ParallaxLayerVisual`) for each view, relative to that view's
   centre (`sync_parallax_layers`). A flat card behind a turned camera would
   slide sideways with its depth, so a panel is instead a card facing the
   view's card camera (`realize_card_backdrop`, `backdrop_card_place`). It
   stands along the camera's view, 1 500 behind the play plane (further
   back for a layer further back in 2D), inside the camera's far plane.
   A foreground layer stands 1 000 in front of the plane. Scale its size
   and offset from the view's centre by the ratio of the card output width
   to the 2D output width in world units. Use the panels that belong to
   that view.
   Portal capture copies stay with their capture cameras. A background
   card faces the camera's tilt and keeps the 2D orientation and offset
   when the view rolls. A foreground card follows the same rule in front
   of the play plane. A panel kept for another view, or hidden, makes
   no card. Render lib with
   `card_view,portal_render`: 7 card-view tests passed. The `intro_fell`
   capture's card view shows the sky, its clouds, the distant hills,
   windmills and the triangle gate behind the shepherd, the sheep and
   Scout, where before it was black.

   **Desktop rollout.** `./run_game.sh --card-view` starts in side-on 2.5D.
   A build with `card_view` can switch outputs during a session. The card
   camera uses the target, viewport, and sample count of the gameplay
   camera for its view. The comparison inset fits inside that viewport.
   Both outputs read the same observer frame. The plane camera retains
   its sample request while the card camera draws in its place. The same
   gameplay camera supplies the room and view mask for the card output.
   A gameplay camera without a mask draws layer zero.

   World labels use the glyph atlas and layout from Bevy. The plane
   camera supplies the target scale when a card output replaces it.
   The workshop window capture shows signs, nameplates,
   Scout, Dennis, and the pause cube. It shows both renderers in the
   comparison mode. Switching the output kept one workshop session
   active and kept it paused while the cube was open.
   Text cards use each span's color. Control signs show the input glyph
   when its image is ready. Nameplates, speech bubbles, and Mary-O pipe
   signs share one outline path. The outline uses the label's font and
   opacity. A copy of a static sign keeps its outline and live room.
   Medium captures of GNU-ton at tick 350 show the same wrapped speech
   and outline in 2D and side-on 2.5D. Medium captures of the Mary-O Warp
   Zone show all eight pipe signs in 2D, side-on, and oblique views. The
   capture enters through the provider's room setting. Portal fault glyphs
   use the body identity and live room for their drift. Medium captures of
   Portal Scale Lab at tick 152 show the glyphs in 2D, side-on, and oblique
   views.

   A card uses the existing portal capture mesh and texture for the far-side
   window. It stays at the seam and uses the source pane order and opaque
   capture to replace the scene beneath it. The card follows the
   source window's visibility even when the source keeps its image. Portal
   pieces use the existing clip mesh and shader. An entry piece keeps its
   actor's draw order. Exit pieces keep distinct depths when two sources
   have the same depth. Tests cover both cases. In a Medium Portal Lab
   capture, Scout stays visible while
   straddling the floor opening in 2D, side-on 2.5D, and oblique 2.5D.
   An Ultra offscreen fixture checks both clipped pieces, an actor in front
   of the entry piece, two overlapping panes, and a foreground band.
   P5/P6 still own cross-room and observer order. Moving platforms
   use their sprite frames. The cart's near layer covers seated riders and
   its far layer stays behind them. Sprite cards blend texture alpha and
   use plane depth for paint order in tilted views. Card quads use a
   geometry key with region, page size, anchor, and flips.

   The card view uses the hub skin and sprite-effect shaders through their
   3D fragment inputs. It also draws tiled blocks, doors, gate glyphs, blink
   markers, slash sprites, the default attack sweep, and folded line meshes.
   The sweep uses the same mesh and vertex opacity in both views.
   Medium captures show the hub floor, doors, switches, and an open gate
   in side-on view. Wolfenstein's
   throne-room lightning uses the same line mesh in both views.
   At Ultra, both outputs use the same animated tunnel material inside
   the gate membrane.

   A moving platform keeps its depth when its sprite sheet draws from
   parts. Both paths place each cart panel on its own layer. The parts
   stay inside that layer's depth range. The cart test checks that the
   rider stays between the panels and that a moving panel keeps its card.
   An Ultra offscreen fixture shows Scout in the published cart and in
   front of the part-drawn burning shark.

   A room prop uses the same scenery depth for sheet frames and parts.
   Each stack of parts fits between consecutive scenery depths. The
   card view sorts transparent draws relative to a nearby point so
   the camera distance does not round close layers to the same key.
   Card cameras use direct draws to keep the order of an alpha batch.
   An Ultra offscreen fixture checks three close, overlapping layers,
   a prop drawn from parts, and the cart and rider.

   Sprite masks and shader glows follow the depth of the actor that
   owns the effect. The card view also uses the authored shaders for
   the Puppy Slug dream, the Flying Spaghetti Monster's sauce, and the
   Mockingbird's flames. The space art retains its sprite materials as
   the tint changes. An offscreen fixture shows all four effects in
   2D and side-on 2.5D at Medium and High. In-game captures are pending.

   Both renderers use the same damage flash and gate scanline shader on
   sheet-drawn actors and on rigged actors with a whole frame. The card
   uses the whole frame's tint and opacity. When a rig has only parts, each
   part uses a flat flash until a whole frame is available. The shared
   overlay goes away when the cue ends. An Ultra offscreen fixture with
   Scout's published parts shows the damage flash and gate transit in 2D,
   side-on 2.5D, and oblique 2.5D.

   Direct rigs use the body tint and opacity from one sample in both
   renderers. An opacity change refreshes the 2D slots. Each tinted card
   keeps its material as the tint or opacity changes. An Ultra offscreen
   fixture shows these changes on Scout's direct parts in both renderers.

   Flips on either axis apply to the frame and its anchor. The sheet
   animator keeps the mirrored trim as the frame advances. Both views and
   readers of composed rigs use the same sampled placement for the parts.
   The frame and part tests cover both axes. An Ultra offscreen fixture
   shows the upside-down shark as direct parts and as a composed frame.

   World-space sprite producers mark their art with `WorldSprite`. The card
   renderer reads this marker and uses the sprite's world transform and
   visibility. Ladders, vines, water, and lock walls use this path, including
   fixture art that replaces placeholder sprites. A test checks that the
   card view shows the ladder at the same position before and after this
   replacement.

   Morph balls, trapdoors, flylines, tethers, room motes, and limb wisps
   use the same sprite path. The card rig reads the same visibility as
   the 2D rig. A test checks that morphing hides the rig on the same frame
   and that exiting morph shows it again. An Ultra fixture shows the
   replacement sprites and effects in 2D, side-on 2.5D, and oblique 2.5D.

   Worn gear fits use the published sheet target. The fit works for both
   loose assets and library assets. The card view draws the back picture
   behind the wearer and the front picture above its anchor part.
   An Ultra fixture shows Scout's helmet on a rig drawn from parts and
   Kagero's talismans on the Inugami drawn as one image.

   Portal carry includes worn gear in the wearer's capture and includes the
   gear's bounds when it sizes that capture. A held item has its own portal
   cut. Its card uses the item's visibility. The item and its entry piece
   use the same draw depth. An Ultra fixture shows a split helmet and held
   items with independent cuts in all three views.

   Both views read the same prepared planar mesh for lightning, shock,
   and attack marks. 2D binds it when an active camera uses its layers,
   including an offscreen capture. Switching the binding keeps the mesh.
   An Ultra fixture checks additive light and blended colors in 2D,
   side-on 2.5D, and oblique 2.5D.

   Chain fills and room terrain use the same mesh preparation. A room
   keeps its terrain meshes when the quality setting changes their material.
   The 2D bindings are present only when an active 2D camera needs them.
   A Medium fixture checks chain fill and textured terrain in 2D,
   side-on 2.5D, and oblique 2.5D. It checks camera switching with the
   same mesh handles.

   Particles, procedural beams, and physics debris use the shared world
   sprite path. Their cards keep the same material during a fade.
   Medium captures of Mine Pocket show dust and moving fragments in 2D,
   side-on 2.5D, and oblique 2.5D at the same simulation tick.

   The Clockwork Warden's hazard column uses the shared world sprite path.
   Medium captures in the basement arena show the yellow warning at tick
   345, the red strike at tick 435, and the clear lane at tick 480.

   The daylight veil and lamp glow use prepared meshes. The translucent
   fallback uses the same world sprite path. Medium night captures in Math
   Town check the veil in 2D and side-on 2.5D.

   Daylight reads the room of each logical view. Its lamp sample covers the
   combined output bounds and uses light sources from that room. The view
   hides its veil when the light is neutral. The two-view test checks lamp
   isolation and veil removal when an observer goes away. A Medium
   comparison capture shows the same night light in the 2D main view and
   the 2.5D inset.

   Indoor darkness uses the light settings and light sources of each
   view's live room. Low and Medium center the light on the participant
   that the view follows, or the nearest participant in that room. High
   and Ultra share a light map across the outputs of that view. The
   two-view test checks overlapping rooms, a quality change, and veil
   removal on entry to a lit room. Medium captures of Mine Pocket check
   the light in 2D, side-on and oblique 2.5D, and comparison mode.

   Each boss presentation row has a live room. Stage lights and name
   cards read the room that the view frames. Both renderers use the
   same stage lights. A test gives two instances of the same boss
   different light cues and checks that each cue stays in its view.
   Medium captures show Wolfenstein's dark room and wake-up flash in
   2D and side-on 2.5D.

   The feature, boss cue, and boss frame indexes share room-qualified
   storage. Each boss occurrence supplies its own animation frame,
   wounds, and hazard column. Hazard columns use the frame's room and
   geometry table, so they can draw before the boss art is available.

   Actor art binding also uses the live room of each occurrence. The
   sprite bind test gives the same placement ID different character art
   in two live rooms and checks the texture on each sprite.

   Actor pose clocks follow the body when it changes rooms. Sprite
   animation, passage fades, glows, and flashes read the frame in the
   body's live room. The passage tests give two actors the same ID and
   check the pose, scale, and fade of the actor at the door.

   Actor nameplates read the label and bounds in the body's live room.
   A view replaces its plates when it frames a different room. The tests
   check actors and doors with the same ID in two live rooms.

   The Smash fighter bubble uses the primary output's projection and the
   fighter's presented position. A test switches between 2D, side-on,
   oblique, and inset views at the same fighter position.
   The close-up camera starts when the fighter leaves the view. The ring
   and image use the front HUD. The game releases both when the bubble
   declaration ends. At tick 149 with `--player-at 950,260@148`, Medium
   captures of `--route smash_gameplay` show the ring and fighter on the
   first frame in 2D, side-on 2.5D, and inset mode. The tilted view still
   shows part of the fighter and hides the bubble. All four captures use
   the same simulation state and logical view.

   Ground shadows use the shared presented body position. Each shadow uses
   the floor and art of its live room. A test checks the sprite and card
   positions for two actors with the same ID in different live rooms.

   Developer health bars use the shared world-sprite and text paths.
   Each bar keeps its background, fill, and label as health changes.
   A test uses the same feature ID in two live rooms. It checks each
   bar's position, health, and removal.

   Both gameplay outputs run the same spirit, void, and screen filter
   passes. Each output projects a field into its own viewport. The card
   output takes its room mask from the plane camera and uses the depth of
   the field's art. Inactive cameras release their passes.

   A field can extend beyond the camera's depth range. The shared
   projection uses its center to check depth and its edge to measure
   radius. Boss finish bars use the viewport of each active camera that
   frames the boss's room. The bars keep their UI nodes as they change
   height and leave when that camera closes or frames a different room.
   The shared HUD draws above the bars. A Medium capture of Marshal
   Gallus's finish shows the bars and color drain in both views.

   Mary-O's fire glow, cosmic aura, barrier, quasar, and room shimmer use
   the same materials in both views. The shimmer, room sky, and blueprint
   shaders use the card camera axis at the play plane. An Ultra fixture
   checks the five Mary-O materials with the published art.

   The fire glow and cosmic aura keep their subject's position in depth.
   Their paint order follows their source depth between the bodies of the
   same room. A light can overlap another body without moving to its plane.

   Cards and sprite effects use the same texture region. A crop uses
   coordinates relative to the atlas cell. An Ultra capture checks both
   atlas and whole-image crops with an offset anchor and two-axis flips.
   Card meshes use the sprite's fit or fill mode. An Ultra fixture checks
   all six modes with wide and tall bounds, a crop, a flip, and an offset
   anchor. The 2D and side-on images agree within one color value per channel.

   Card caches keep meshes and materials that a draw used recently.
   Hidden sprite cards release their handles. The last view releases the card
   products when it closes. The cache uses the same rounded color for
   the flash key and the flash material.

   Both renderers draw emitted trails and retained debug lines from the same
   line buffers. Each card camera draws the lines for its room and local
   view. Switching to 2D removes the card draws and keeps the line buffers.
   The navigation graph and rollback ghost overlays use these buffers.
   Medium Central Hub captures show the graph in both renderers. A test
   checks that a rollback ghost stays in its source room and disappears
   when the room is removed.

   The card view uses Bevy's sprite slices for lock-wall braces and other
   sliced props. It draws boss-stage veils and both room darkness paths.
   The blueprint overlay, portal glow, and shield lattice use the same
   fragment shaders in both views. The portal channel uses the world text
   layout. The card view keeps the source text order relative to scenery
   and lighting. Gravity zone sprites keep their identity when a zone moves.
   The shield keeps its card mesh and material as its size and color change.
   Medium captures show these fixes in Castle Godwin, the blueprint room,
   the military gate annex, Gravity Lab, Portal Lab, and the central hub.
   The Clean / corrupted exhibit uses the same sky, fog, and front effects
   in both renderers. Medium captures show the front in all three views.
   The Hub sky, blueprint overlay, and Mary-O shimmer use the same mesh
   fragment inputs and camera projection in both renderers. The plane
   renderer loads this shader library with the room presentation. This
   fixes a missing shader import when the card plugin is absent. Medium
   Central Hub captures at tick 160 show the corrupted sky behind the
   developer wing in both renderers. Captures also show the Tech-Bros
   Basement blueprint and the Mary-O starlight shimmer in both renderers.
   Shared mesh effects get their fragment inputs from
   `ambition_sprite_fx::mesh_fragment`. The library uses the Bevy render
   view and clock types. It selects the correct vertex color and clock
   binding locations for each renderer. The sprite-effect GPU test draws
   a silhouette in a composition that has only the 2D render features.
   The default desktop build draws the Tech-Bros Basement blueprint at
   Medium. Medium Portal Lab captures show the clip and glow in 2D,
   side-on, and oblique views. A rendered vertex color check gives the
   same result in 2D and side-on 2.5D.

   `capture_scene --renderer` fills the image with the selected renderer.
   Each camera uses the same presented frame. The card view keeps each
   closed portal mesh and uses the 2D clip warm-up. It prepares bindings
   after the capture image is ready. Both renderers now request
   specialization again when a delayed mesh reaches the GPU.

   To inspect the first frame of the floor opening in Portal Lab, use
   `--at-tick 73 --player-at 1450,884@72:0,100` with focus `1450,850`,
   size `1280x800`, and Medium quality. Scout, the portal picture, the
   channel label, and the ground pickup are visible at tick 73 in 2D,
   side-on 2.5D, and oblique 2.5D. To inspect the approach and exit, use
   `--at-tick 70 --frames 8 --stride 2` for the same player movement.

   A Medium capture at `900x1200 --camera-roll 90` shows the backdrop
   at the same orientation and offset in 2D and side-on 2.5D. A
   `--card-view` capture uses the game inset settings and draws both
   outputs to the selected gameplay target. The live F3 panel can
   switch between the inset and 2D while the simulation is stopped.
   A backdrop uses the routed width of the inset. When the view switches
   from a zoomed inset to full-screen 2.5D, the backdrop uses the same
   size and offset as the 2D view.

   Each portal capture draws its own backdrop from the room it serves.
   Local views, card outputs, menus, and portal captures use separate
   render layer families. The portal backdrop test covers two eyes in one
   live room and a third eye in a room that uses the same portal channels.
   The feature census keeps the live room of each undrawn feature. When
   two rooms use the same authored ID, only the room with missing art
   keeps its loading cover.

   Portal windows use the final view pose, zoom, and pixel dimensions.
   A tilted view projects its viewport onto the play plane. Its capture
   covers the bounds of that projection. The comparison inset and its
   main output share one capture for each portal and observer.
   The capture covers the union of the observer's output bounds. It uses
   the highest projected pixel density along each axis, within the
   capture limits. A projection test checks the density of a zoomed,
   tilted inset on a high-DPI display.

   Portal windows, rims, and exit pieces project from the same play plane
   in 2.5D. The draw order of each stays in its own layer. An Ultra capture
   checks the projection with both tilt and roll. At the exit boundary,
   both outputs filter texels on the visible side of the same capture.
   The card renderer uses the shared portal material and mesh. The
   window, rim, and exit pieces keep their paint order.
   A new portal capture target waits for the 2D draws queued for its view.
   The portal frame stays visible while the target has no picture. A
   Medium cold-start capture in Portal Lab shows the far rim and ledge
   in the first picture in side-on and oblique 2.5D. The warm 2D capture
   shows its picture on the opening frame.

   Text cards use Bevy's layout for the current frame. The plane camera
   supplies the target scale and room mask, including when a card output
   replaces it. The card camera draws the text at the edge of its tilted
   view. Text layout needs no extra visibility pass. The portal input cue
   appears in both views and clears on release.

   Rigged bodies use the same output coverage as portal windows. A tilted
   view samples a body at its own screen edge. A body keeps sprite slots
   when a 2D camera or the compositor needs them. Card draws and part
   readers can use the sample without sprite slots.
   The compositor supplies the finished rig frame to the card renderer.
   A test changes the 2D sprite after composition and checks that the card
   still draws the atlas cell and its group fade.

   Card placement and paint order use separate values. Free bodies stay on
   the play plane. Riders stay inside the cart. Parts and attachments use
   the body's plane and keep their own paint order. A body keeps its place
   when another body enters the view.

   The card mode still uses 2D world preparation and composition. Shared
   placement and appearance products for all families, more portal effects,
   and a performance comparison are open.

PO1 can proceed before RV1/RV2. The scale-law work can proceed independently;
both renderers must consume the resulting authoritative material and poses.
Keep the maintainer's product order in the queue.

## Focused acceptance and cost boundaries

- Compare both outputs with the intended scene behavior. Record or repair
  defects found in either output.
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
