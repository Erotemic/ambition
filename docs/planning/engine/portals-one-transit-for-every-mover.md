# One portal transit, presented in every view

**Status:** the common mover and drawable roads are built; the view work below
is **PLANNED**, from source review of `b91b29d35935` on 2026-11-21.
[Portal parts, similarity and chirality](portal-parts-similarity-and-chirality.md)
is the delivery owner. This page owns presentation handoff, observer geometry
and capture lifetime. [Moving frames](portals-are-moving-frames.md) owns the
motion interval and continuation details; this page does not define another
crossing machine.

## Product contract and current receipts

The maintainer's October 15–25 direction remains binding: every mover uses
portals, arbitrary drawables work without a game-specific portal adapter,
material is drawn on both sides while crossing, and the view has no seam.
An **open window captures every presented frame at every quality tier**.
Quality tiers retain the configured view-matched pixel density and may cap
the number of open captures. An older picture beside current material is not
a valid cheaper open window. The carry's separate byte budget may reduce its
source-image resolution or use material proxies; that is not permission to
reduce window density by tier. The gauge/seam lab and `capture_scene` remain
the existing tools for visible evidence.

| Current mechanism | Keep; do not rebuild it as new work |
| --- | --- |
| Shared `PortalTransit` / `PortalBodyTransited` road | Kernel, loose-item and projectile integrations use the common transfer authority; remaining-time motion is owned by the moving-frame plan. |
| Canonical material partitions and admitted `Sim2` maps | Body, attack and picture geometry share map/cut operations; physical domains and art extents may differ. |
| Universal drawable carry | Top-level Sprite, Mesh2d and Text2d sources are eligible by geometry; explicit families follow their root. No new effect kind needs portal opt-in. |
| Independent family roots and depths | R4 stopped merging independent bodies into one raster source. Preserve their identity, cut root and order. |
| `ClipQuadPool`, reused targets and material proxies | Entities/materials/meshes already persist; same-size targets reuse storage and idle targets retire. The carry's 32 MiB default budget reduces resolution then draws admitted proxies. |
| Per-view camera state and focused scale | The camera resolver adopts `CameraFocus2d::scale` immediately after ordinary zoom easing. Use this once; do not add a second portal scale multiplier. |
| Cached composed-body images | Source images wait for their own GPU work before resting. Readiness is an existing mechanism, not an assumed one-frame delay. |

Source owners: `ambition_portal2d::{transit,view}`, core `frame::{similarity,thread}`,
`ambition_portal2d_presentation::{carry,clip_material,view_cones}`, and
`ambition_sim_view::camera_snapshot`. The
[November review receipt](../../../dev/reviews/portal-correctness-and-cost-review-2026-11-10.md)
and [GNU limb image receipt](../../../dev/ambition_dev_measurements/summaries/portal-gnu-limb-render-20261111.md)
record completed fixes and their limits. The October seam-lab measurements are
historical controls, retained in Git; they do not certify today's cameras,
multiple views or target hardware.

## One presented chart per observer

**Planned invariant:** a portal window is a projection for a particular observer
through a directed portal path. A room/channel alone cannot identify it.
Two views of one room can have different eyes, facing, viewport, roll and
occlusion. Two paths reaching one endpoint can have different maps and clips.
The endpoint occurrence and topology snapshot come from the topology owner in
[the pair plan](portal-pair-identity-and-aperture.md); rendering must not mint a
parallel pair identity. A new pose does not create a new endpoint occurrence.

The shared presented sample names the observer and scene lifetime, selected
source pose/time, crossing cursor, physical viewport and already-resolved
policy. It keeps the persistent observer chart distinct from the final
projection after shake and finishing zoom. Footprint clipping, source capture
and density use the actual final projection; continuity transports the
persistent chart and applies transient effects once, without accumulating
them into the next anchor. Diagnostic targets and zone names add no authority.

The gameplay visibility eye, body extent and facing are also explicit facts
at the selected source sample. They are not the eased/clamped camera center.
A complete chart must not replace body-eye LOS with camera-origin LOS or
change authored turn-away behavior. These facts and the final projection
travel in one coherent sample, with parallax consuming its declared camera
placement. Resolve a room's portal mode for that observer's room; one
active-room rule resource cannot govern independent rooms correctly.

Current source loses necessary facts:

- `platformer2d_host::portal::sync_portal_viewer` publishes views in id order;
  `PortalViewers::publish` refuses a second eye in the same room.
  `PortalViewRig` identifies `(room, channel)`. Existing rig tests explicitly
  pin that restriction; two-room tests do not establish same-room multiview.
- `PortalCameraContinuityHostView::capture` keeps center and visible extent,
  but drops camera rotation. The camera stage applies that rotation to the
  real camera. `portal_window_clip_rect` still clips an axis-aligned rectangle,
  and `portal_capture_camera_frame` maps global X instead of the observer basis.
- `GameplayScreenDensity` reads the global gameplay rectangle or a first main
  camera, rather than the actual observer's physical viewport.

The missing basis has an exact counterexample: an 800×450 view rolled 90°
sees x±225,y±400 about its center; portal clipping keeps x±400,y±225. A visible
window slice at y=300 disappears before rasterization. Clipping to the actual
convex screen footprint and transporting its basis fixes the cause; extra
AABB padding does not make capture coordinates correct at arbitrary angles.

Bind depth-one observer×endpoint projections first. A view's pane, private
capture layers and parallax copies all belong to that binding. In particular,
`rendering::parallax::sync_portal_capture_parallax_layers` currently copies every
room's root panels into every rig, and channel-derived private masks alias
between rooms. Filter source panels by the projection's scene and give each
live projection a private backend binding. Preserve the rule that foreground
parallax is drawn by the main view only, not copied through a window.

## Ordered crossings and explicit cuts

Let `C` map pre-effect screen coordinates into the persistent observer chart
and let `H` be an admitted crossing map. Instantaneous chart continuity of
corresponding points requires:

`C_after = H ∘ C_before`, or `P_after ∘ H = P_before` for the inverse
pre-effect projection `P`.

Transport the complete chart, not just its center. Apply the selected final
presentation effects once on either side of this relation; their intentional
change over time is separate from a chart discontinuity. The existing focused
camera-scale adoption participates in the chart result exactly once.
Overview/director framing is explicit observer policy; it cannot accidentally
inherit a focus-only assumption.

`apply_portal_camera_continuity` currently drains all transfer messages, selects
one previous camera center, then maps **that same center** for each transfer.
It maps the ease target sequentially but overwrites the anchor. Two translations
of +100 and +200 between presentations therefore store C+200 instead of C+300.
Several fixed ticks can precede one presentation even when a mover crosses
only once per tick. Fold the motion owner's ordered admitted transfers once,
using their sampled maps; do not reconstruct the chain from the last pair,
current endpoints or event/query iteration without its declared ordering.

Resolve each view's source pose and crossing cursor together. Fold only the
accepted transitions between its previous cursor and the chosen presented
sample, not every simulation event available. An interpolated pose can trail
the latest simulation crossing; an extrapolated pose cannot invent an
unadmitted transfer. The view's chart, follow target and foreground carry
consume that same fold. Subject binding, scene lifetime and presentation
branch are part of its identity.

Project compact crossing receipts while motion history is available, and
retain them until the relevant cursors consume them or explicitly cut. This
does not retain every contact cell and full tick history for a slow observer.
Bound retention; if a cursor loses its required history, declare the view cut
rather than reconstructing a path from current endpoints.

On a subject/room/session change, an explicit director cut, missing history,
or abandonment of the presented rollback branch, discard the old anchor and
rebase to the accepted presented sample. A body still straddling a
pair is not a request to replay its transfer. Presentation history is derived;
it adds neither rollback authority nor another crossing event.

### Reflection is a policy boundary, not an angle

**Proposed Continuous contract:** transport the full finite `Sim2`, including
scale and reflection, and retain the transported chart parity for this view.
It remains that chart until another accepted map or an explicit view reframe
changes it. This can change reflected presentation; it never reflects the
shared physical world again and never rewrites authoritative controls.

An orientation-reversing map changes the sign of the camera basis determinant.
No continuous nonsingular interpolation can restore an unmirrored positive
basis: the determinant would have to pass through zero. Angle or quaternion
easing cannot provide this transition. Current `CameraChartTransit` carries
only a roll; `camera_roll_for_portal_transit` returns zero for reflected
opposite-wall pairs even though their map flips a tangent coordinate.

[WorldFixed and SubjectFrame](../../systems/camera-reference-frames.md) remain
per-view orientation goals. A policy that also requires an unmirrored result
must explicitly select a **parity-restoring cut/fade/reframe**, with its own
handoff boundary; it cannot describe that restoration as continuous easing.
Keep the existing **Pop** option for an intentional camera cut. Do not silently
make a requested Continuous reflection fall back to Pop, and do not infer input
frame from camera parity. The first implementation must represent these choices
explicitly and demonstrate the continuity claim of the selected one.

Retire or replace the unused `view::map_viewpoint_frame` angle-only API rather
than moving production onto it: it omits scale/parity and simply adds angles
even when its view map reflects. Use exact composition of the appropriate
maps; a virtual LOS eye construction and a physical body map remain named
different operations where their definitions differ.

## Capture demand and ready pictures

A desired view, an allocated target, a scheduled capture and a completed
usable picture are distinct. Keep this state in the presentation owner.
Desired work and storage admission precede allocation; readiness gates
exposure or dependent consumption, not allocation. A source producer may
need an allocated target before it can produce a ready picture.
A render dependency may produce the picture before its consumer in the same
presented frame; readiness does not prescribe an extra CPU round trip or a
fixed one-frame delay. A scheduled but unavailable pipeline is not ready.

| State | Meaning |
| --- | --- |
| Desired | The observer/path has a nonempty clipped domain and the authored policy wants a window. No texture allocation is implied. |
| Admitted | Current-frame capture work and storage have been assigned for this projection. Required source images/dependencies are identified. |
| Ready/open | The matching completed picture can be composited for this presented sample. Every open view is current. |
| Non-view/retired | The pane uses the explicitly selected non-view fallback or no window, and releases obsolete dependencies/targets. The physical portal remains unchanged. |

Today `sync_portal_view_cones` allocates a target for every paired endpoint,
even shut ones. Its global active-camera cap does not bound retained bytes.
Existing rigs claim it in ECS iteration order. A newly created over-budget rig
can have an opaque nonempty pane, an inactive camera, and `captured=None`:
there is no usable picture behind the material. Later rigs can also remain
stale indefinitely while earlier rigs remain open. These are concrete lifetime
and admission problems, not evidence that fragments lack a pool.

Admit open work from explicit observer/path demand, with deterministic
priority and required crossing views first. A tier may bound the admitted
open set; excess views must use the declared non-view presentation, never an
uninitialized or stale opaque picture pretending to be a live view. Preserve
current seam priority, including an explicitly accounted temporary capture
allowance if a crossing needs it. Budget loss must not close a physical portal,
remove an admitted material piece, or change collision.

A completed image carries its source scene, observer/path, sampled map,
capture frame and content/dependency version. Publish it only to the request
that still owns those facts. Retirement/replacement, another scene, observer
rebinding or an abandoned branch cannot adopt an old completion. Current
`PortalViewRig::captured` already retains the capture frame and adjusts UVs
when showing a held picture; retain that provenance fact, but remove stale
held pictures as an open-window quality policy. Reuse without a new raster
only when the relevant source semantics are actually unchanged and the result
is equivalent to this frame's capture. Age alone is never such evidence.

### Visibility and nested views

Keep authored Static/Dynamic modes, turn-away behavior and the near-face
opening response. Separate their appearance mask from geometric coverage.
`compute_cone` currently reduces 1/3 aperture sample rays per body corner to
one fraction, then grows a full-aperture trapezoid. Cover only the left end,
then only the right: equal fractions can generate identical windows although
the visible mouth regions differ. Increasing ray count after discarding its
locations cannot recover those regions. This is approximate visibility, even
with the half-plane assist off; avoid calling it exact LOS geometry.

A geometric visibility result retains visible aperture intervals/regions and
maps their clipped beams into the source scene. It can use core similarity
and half-plane operations, with its own sight-blocking policy. It is not a
physical material partition, and art overhangs/visibility do not inherit body
snag rules. Shared immutable room occluder data and spatial candidates replace
per-eye copies and a whole-room AABB scan for every sampled ray. Retain sample
time and geometry revisions so moving hosts/occluders invalidate the right
result; topology identity alone is insufficient.

True nested views expand these observer/path projections with a finite
admitted depth/work bound. A child is captured from the parent's transported
chart and clipped domain; dependencies are rendered before their consumers.
Current positive `recursion_depth` enables texture feedback of windows made
for the original eye, not a deeper view, and is disabled for multiple rooms
to avoid channel-layer aliasing. Keep default depth zero until the path model
exists. Do not make depth-one correctness depend on implementing arbitrary
recursion, or preserve stale feedback as the implementation of depth two.

## One piece geometry, two image sources

The current common carry is the correct fallback for arbitrary drawables.
Its source family, material domain, partition/cuts and transformed output are
the same whether its image comes from a live raster capture or an already
complete local image. This is a producer specialization, not object opt-in.

`declare_portal_body_demand` currently requests composed images for every
animated root in every portal room, and carry then rasterizes each carried
family again. The November GNU witness records 12.125 MiB of carry targets
and 51.2578125 MiB of upstream image-pair descriptors outside that budget;
its six upstream image cameras were already resting. Those are historical
software-renderer resource descriptors, not device timings or a new baseline.

Start with a plain Sprite or existing `DeclaredFrame` whose whole appearance
is represented by its texture, UVs, anchor, flip, tint and alpha semantics.
A completed composed-body image can follow when it proves coverage of the
whole requested family. Unrepresented children, gear, glow, custom materials
or time-dependent shaders retain the universal family capture. Do not merge
independent roots/depths, silently omit an overlay, or recreate per-game
portal renderer branches. Source-domain image bounds may exceed the collider.

World placement and a portal's changing cuts do not by themselves change
local source pixels. Animation, relative member transforms and material/image
inputs do. Reuse the image with those dependencies, then feed the same clipped
material path. Early demand comes from available geometry/appearance facts
before the image producer; asking whether the image already exists to decide
whether to request it recreates the dependency cycle. Preserve first-use
readiness and the explicit proxy/non-view fallback before narrowing the
current room-wide eager demand.

## Shared appearance order and a second rendering consumer

The [presentation composition plan](presentation-order-and-25d-rendering.md)
is selected from the cart, part/effect ordering and 2.5D customers at
`f41218979625`. Keep three facts separate: source appearance order,
observer-relative pane overlap, and pass/capture dependency. A local
appearance's stable rank and occurrence tie must survive projection.
Current carry compresses roots to an exit z band and collapses equal source
z; its free-drawable loop also uses `z <= PORTAL_EXIT_COPY_TOP` to exclude
ground/copies. Migrate eligibility as a source/pass role when changing rank
lowering. It must not be inferred from the new order value.

The current Sprite/Mesh2d/Text2d source recognition, Camera2d capture and
Material2d clipping do not provide Mesh3d support. P5/P6 remain the integration
owner: each installed realization consumes the same material partition,
path, view scope, source order and readiness meaning. Keep complete family
capture and PV4's planned complete-image specialization. A flat color image
cannot supply independently interleavable internal depth to a 3D consumer.

The first 3D cart pilot can explicitly omit portals. General rollout must
cover two observers of one room and cannot silently omit 3D carried objects.
Do not replace the world's window/foreground composition policy with a
private depth adjustment on each drawable.

## Delivery packets

These are slices of the main portal program, not a second queue.

1. **PV1 — complete observer binding.** Publish persistent chart, final
   projection/viewport and distinct gameplay eye/facing together; keep every
   observer of a room; bind depth-one panes,
   parallax, pixel density and governing rules to it. Delete first-eye room
   grouping and channel-derived private ownership. Witness two observers of
   one room with different rolls and cover, plus two live rooms with identical
   channel names and different backdrops. The 90° footprint counterexample
   must keep the visible slice and correct UVs.
2. **PV2 — ordered chart handoff.** Consume the motion owner's ordered admitted
   maps with the cursor of the same presented source pose/time. Retain compact
   receipts until consumption or an explicit cut. Carry view state consistently;
   separate full Continuous transport, ordinary orientation response and an
   explicit parity-restoring transition. Delete map-every-event-from-old-center
   and angle-only chart authority. Witness two translations between frames,
   unequal apertures using the existing focused scale, a reflected asymmetric
   landmark, and view rebinding/rollback correction. No physical/input state
   changes because of the camera policy.
3. **PV3 — admitted capture dependencies.** Make desired/admitted/ready/retired
   images explicit before material exposure. Admit demand before allocation;
   gate exposure on readiness. Refresh every open view at its configured
   view-matched density, retire completion receipts by their owner,
   and apply an explicit non-view fallback under pressure. Delete all-endpoint
   eager targets, query-order allocation and stale-open feedback. Establish
   depth-one readiness before adding path-derived nested views. An allocated
   but never drawn target must never become an opaque view.
4. **PV4 — reuse local image sources.** Feed a proven complete source image
   through the common cut/map path, leaving universal family capture intact.
   Then reduce eager composed-body demand and redundant raster passes.
   Compare the same asymmetric source with overlays, independent family depths,
   scaled/reflected cuts, first publication and teardown. Delete only producer
   work whose full appearance is already represented.

The existing seam lab measures crossings and grid continuity; app camera and
rig tests cover the source-level examples. Pixel area must be interpreted
through physical scale and camera projection. Record current visible-host
CPU/GPU/resource evidence for the selected slice; the October timings and
November llvmpipe films do not establish hardware frame-time savings.
[Render/VFX R8](render-animation-and-vfx.md#r8---portal-fragment-lifetime-and-measured-render-churn)
and [performance](performance-and-iteration.md) own integration and cost
attribution. Preserve `BodyOwnedDrawableSync` and carry's current-frame
PostUpdate-before-extraction boundary throughout.
