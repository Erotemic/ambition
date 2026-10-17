# Render, animation and VFX — Engine 1.0 program

**State:** OPEN. The built-in semantic presentation road and the body-owned
drawable schedule are built. Remaining work adds persistent emitters and new
providers only when a real effect requires them.

## Scope

This page owns the boundary between simulation facts and visible presentation:

- sprite animation and authored visual clips;
- one-shot VFX;
- persistent body/world presentation derived from semantic state;
- procedural particles and optional richer providers;
- body-owned drawables such as hit flash and status clocks;
- portal-aware presentation;
- presentation quality policy;
- multiview and confirmed/external-effect constraints.

Asset preparation, quality-tier materialization, residency and first-use hitches
are owned by [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).
Runtime/frame-cost measurement is owned by
[`performance-and-iteration.md`](performance-and-iteration.md).
Characters drawn from reusable parts (transform flipbooks, on in the shipped
game) are owned by
[`runtime-rigged-sprite-animation.md`](runtime-rigged-sprite-animation.md).
Per-live-room drawing (each entity drawn in its own live room's band and view)
is owned by [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)
("The view half").

## Authority contract

### Simulation says what happened

Gameplay/domain code publishes semantic facts or requests. It does not depend on
particle entities, sprite materials, view-local clipping pieces, render layers,
or provider-specific components.

A visible host may omit the presentation consumer without changing simulation
outcome.

### Presentation owns representation, never gameplay state

Presentation may create, update and remove derived entities freely. Rewinding or
changing views may rebuild those entities from current semantic state.

Do not put ordinary render/VFX entities into rollback state merely to make them
survive a rewind.

A character- or weapon-specific look is legitimate authored vocabulary (Q42,
2026-10-04): the gauntlet fireball keeps its bespoke sprite
(`GAUNTLET_FIREBALL_VISUAL`, registered in `game/ambition_content/src/projectiles.rs`)
and is not collapsed to the catalog energy ball. It still travels the ordinary
projectile road (`RangedActionSpec::with_visual`); bespoke art never earns
bespoke simulation.

### One-shot and persistent presentation have different lifetimes

A one-shot request carries enough information to draw one effect.

A persistent visual needs:

1. stable semantic ownership (`PresentationOf` or an equivalent domain fact);
2. reconciliation from semantic state;
3. an explicit presentation lifetime/retraction rule.

Do not force persistent sources into a growing one-shot `ParticleKind` enum.

### Body-owned drawable geometry has one finalization boundary

Every body-owned drawable that portals or other late compositors consume must
publish its **current-frame geometry/appearance before candidate publication**.
The ordering model is:

```text
body-owned drawable producer/update
    -> body-owned drawable geometry finalized
    -> portal candidate publication
    -> portal compositing / source visibility resolution
    -> draw
```

The boundary is the `BodyOwnedDrawableSync` set (`ambition_render::rendering`).
`publish_portal_compositing_candidates` runs after it. A new body-owned drawable
writer joins the set; outside it, the writer is composited a frame late at
best. The set edge also flushes commands, so a drawable spawned inside the set
is a candidate on its first frame. Do not order a new overlay against concrete
foreign systems.

### Portal relationships are per pane and per drawable

A single entity z/visibility value cannot express a body that is near one portal
and far from another.

The current compositor therefore reasons from drawable geometry and creates
per-pane representations. Keep these invariants:

- source z is not mutated to solve portal optics;
- a disjoint drawable is unchanged;
- far-side overlap clips only the overlapping representation;
- transit remains its own split representation;
- two panes may produce different relations for the same drawable;
- body-owned secondary presentation participates independently from the base
  sprite when it declares enough geometry.

`DeclaredFrame` is the reusable road for non-Sprite presentation that can be
reconstructed from a texture/UV/size/anchor/tint description. Do not reintroduce
whole-body scalar hiding for a drawable that can describe its own geometry.

### Quality is presentation policy

Visual quality may alter particle counts, expensive shaders, texture tiers,
trail density and similar visual cost, and may in time select a cheaper
implementation of a presentation-only system. Gameplay and simulation never
depend on the highest-fidelity presentation (Q84, see
[`asset-preparation-and-residency.md`](asset-preparation-and-residency.md#quality-is-a-presentation-policy-q84)).
How readable a reduced tier must be is set per product: a potato sprite may be
humorously small (Q69), and a UI product such as a portrait must still do its
job (Q84).

FX sheet tier selection now follows the shared quality authority. The narrow prop
loader that explicitly requests full-resolution art is an authored exception,
not an omitted quality road.

### Reference frames are explicit

Gravity-relative, surface-relative, facing-relative and world-fixed effects must
receive or derive the correct semantic frame. Do not reconstruct orientation
from world axes after the authoritative producer has lost that information.

### Providers are demand-driven

The built-in authored-sprite/procedural path is the default.

Do not add a universal VFX backend trait or a third-party particle dependency in
advance. A second provider must first have a concrete effect the current road
cannot express adequately on the supported targets.

## Current implementation state

The current engine already has:

- authored sprite effects and generic action FX;
- built-in procedural particle fallback;
- presentation quality/budget policy;
- explicit body-presentation ownership (`PresentationOf`);
- portal candidate publication and per-pane clipped pieces;
- non-Sprite frame declaration through `DeclaredFrame`;
- hit-flash presentation using the non-Sprite portal road;
- a generic body-clock read model and visible clock presentation;
- shared render-basis logic for sheet-authored player/actor sprites.

Player and actor presentation share one render-basis authority. A
presentation basis is chosen once, and only when the authored answer is
available: a sheet-backed presentation is final only once `BodyPoseView` exists;
before that the body is drawn provisionally. `capture_mary_o` in a headless
environment renders placeholder rectangles (no decode demand), so it cannot
validate a binding fix.

Normal presentation writes its desired visibility each frame, and portal
resolution reasserts `Hidden` after every other writer while a hide reason
stands. Witnesses: `a_clock_bar_is_composited_on_its_first_frame_and_follows_its_body`,
`a_flashing_silhouette_is_back_the_frame_its_body_returns_to_the_near_side`.

## Current work

### R3 — persistent emitter reconciliation, only with a real customer

When a semantic source needs a persistent particle/emitter representation:

- simulation owns source existence and parameters;
- presentation reconciles provider entities;
- removal follows source lifetime;
- rewind may rebuild representation from source state.

Do not create this abstraction without a current persistent source.

**First customer (2026-10-17): procedural lightning.** Jon asked for an
engine that can do procedural lightning: "a force lightning sort of feel" and
"a DBZ super saiyin 2 power lightning that can be a sort of power up aura".
It is built as a LEASE, not a reconciled component:

- `VfxMessage::Lightning` (a bolt between two ends) and
  `VfxMessage::LightningAura` (arcs over a body), in `ambition_vfx::lightning`;
- the module ports `ambition.feedback.lightning` / `lightning_aura`, offered
  in `boss_conduct`;
- a producer re-asks under a key each tick, which renews the effect and moves
  its ends; when it stops, the effect fades. A rewind that re-asks rebuilds
  it. Nothing is in rollback.

`ambition_render::lightning` builds the geometry on the CPU each flicker (a
jagged trunk with forks, or arcs round an ellipse). It is a vertex-coloured
mesh drawn with additive blending, and the shader draws no shape. Quality:

- with no screen-shader budget, or on Android, only the white core is drawn,
  with no halo, forks, aura or contact light;
- the fork and arc counts follow the particle `spawn_rate_scale`.

A bolt is a tell, so its core is drawn on every device. The first user is
Wolfenstein: ceiling strikes, the arc lash, neck-bolt crackle and his
unleashed aura.

Open:
- portal compositing of bolts;
- the ports in `wielded_use` / a character's power-up state (Empowered), so a
  player can wear the aura;
- coils arcing into him at the throne.

### R4 — optional rich provider, only after a capability failure

Reopen a provider spike only when a named effect cannot be expressed reasonably
by authored sprite + built-in particle presentation. Compare platform support,
compile/runtime cost, ownership and multiview behavior—not screenshot richness.

### R5 — multiview and confirmed external effects

View-local presentation consumes the same semantic state independently. No
camera's presentation entity may become authority for another view.

Irreversible external effects must use the confirmed-frame/netcode contract;
ordinary VFX stays derived and disposable.

## Asset/authoring contract

Effect identities and clips use the same catalog/preparation principles as other
assets. Runtime presentation consumes published clip/frame metadata rather than
maintaining a second hand-written copy of sheet geometry.

If art and collision disagree, establish which semantic coordinate basis is
wrong before editing both code and assets. Measurement tools should report the
logical frame, rendered frame, anchor and final drawable transform together.

### R6 - reconcile semantic screen-field sources by visual ownership

**What landed (2026-10-12):** Mary-O's `reconcile_void_auras` makes the
aura of each visual what its thing is now, each frame and both ways: one
pass over the bodies keyed by (live room, id), so a visual of another live
room of the same definition does not take a void body's field; an aura
follows a sprite's new size; it goes when its body goes or is not void; a
shot's aura follows its view's visual id. Witness:
`ambition_demo_mary_o void_look a_visual_has_the_aura_of_its_own_body_now`;
three poisons (id-only key, no removal, no update) each fail at their own
assertion. On the shipped spawn roads (drawn demo, 1-3), each void body's
visual has its aura: `mary_o_it --features visible`
`each_void_body_of_the_drawn_demo_has_a_visual_with_its_aura`; a body key
with no room fails it. Not done: the many-body scaling curve (the nested scan is gone,
so the cost is one pass over bodies and one over visuals). The text below
is the case as found.

**Real customer (Mary-O, 2026-10-11).**
`game/ambition_demo_mary_o/src/void_look.rs:65-88` scans every body for
*each* `FeatureVisual` without a `VoidAura`: `bodies.iter().any(...)` joins
`FeatureId` strings to the rendered id. In the unmarked case the cost is
O(visuals * bodies) each `Update`. The system adds the marker but does not
retract it when the source brain, bound visual, image size, or projectile
visual id changes. `ward_her_with_the_quasar` already shows the better
pattern: it inserts and removes its ward from current semantic state.

**Contract:** the authoritative body/projectile gives presentation a source
identity, its look/effect recipe and lifecycle. The realized visual declares
which source it presents. Reconcile the resulting `VoidAura`/`VoidWard`
from the current source and current visual size; update, remove or move the
derived effect when binding, form, quality, visibility or room changes. No
render system should search all bodies by a display id for each visual. The
source may be looked up once through a bounded index or an explicit binding.
It must not be a second copy of the gameplay brain table in the renderer.

`crates/ambition_render/src/void_field.rs` already makes the full-screen
pass opt-in per camera. Preserve that path; a theme-specific void shader is
not grounds for a universal field-simulation subsystem. Its current shader
budget is 8 sources and 4 wards per camera (`MAX_VOID_SOURCES`,
`MAX_VOID_WARDS`); excess sources are dropped by distance from view centre.
Specify this loss policy for consumers. Measure the gameplay case where a
ward or relevant aura falls outside the cap before altering its cost or
capacity. Do not silently convert render truncation into simulation logic.

**Tests:** a changed brain and worn form retract an old aura; a changed
sprite size updates its radius; projectile visual replacement removes the
source; two live rooms and two cameras show only their own eligible sources;
no visible host means no shader pass. Measure the body/visual population
curve in the supported visible host. Poison: restore the nested id scan and
show a many-body scaling regression, or leave a stale marker after rebind.

### R7 - full-screen field visibility must match the camera's render layers

**What landed (2026-10-12):** `sync_void_fields` gives a camera only the
auras and wards it draws: `drawn_through` is the renderer's own rule (no
`RenderLayers` is layer 0; `RenderLayers` does not go down to children, so
a thing's layers are its own). Witnesses: `ambition_render void_field`
`a_camera_has_only_the_field_of_the_things_it_draws` (two cameras on two
layers, auras and a ward at one pixel, a layer-0 control) and
`a_camera_that_draws_no_void_thing_has_no_pass`; poison (no layer test)
fails both. Not done: a room-scope filter (live rooms have their own layer
band, so no reuse is known), a quality-tier downgrade case, and nested
visuals (by the rule, a child's own layers decide). The text below is the
case as found.

**Direct code finding:** `crates/ambition_render/src/void_field.rs:158-191`
loops over all `VoidAura` and `VoidWard` entities for each `MainCamera`.
It checks `InheritedVisibility` and viewport projection, but **never**
compares each source's effective `RenderLayers` with those of the camera.
`world_to_viewport` does not express the renderer's layer filter. Two
live rooms can have identical world coordinates but distinct camera
layers, and one room's void could desaturate the other's picture. A ward
from the other room may also suppress the current room's effect. The
shader can therefore contradict the actual visible set of draws even
when there are fewer than 8 auras or 4 wards.

**Fix at the presentation owner:** use the **effective** render-layer
mask of each source/ward and each camera, including the established
inheritance policy, as well as the camera's visibility/frustum test.
Sources that could not render through that camera must not affect its
full-screen pass. A room scope may be a second necessary filter if
layer reuse makes independent live rooms share a layer. Do not add an
app-global `PrimaryPlayer` or compare authored room definitions as a
shortcut. The source-to-visual binding repair in R6 remains separate.

**Witness:** two main cameras with disjoint render layers and two auras
at exactly the same pixel coordinates; each camera receives only its
own aura. Repeat with wards, nested/child visuals, one source becoming
hidden, a quality-tier downgrade and two **instances** of the same
room definition. Poison: remove the layer intersection: both cameras
receive the other room's source. Require the same filter for every
future screen-space effect derived from a world entity; rendering in a
single camera is not enough evidence.

### R8 - portal fragment lifetime and measured render churn

**Measured (2026-10-12), headless, the far-side compositor:** a far-covered
body for 600 frames (300 still, 300 moving) makes 2 pieces a frame (1200
in all) and a new material for each; with the asset store's own handle
tracking (`AssetPlugin`) at most 4 materials are live, so there is churn
and no growth. A test-side leak (keeping each piece's handle) gives 1200
live and fails the guard, so the instrument sees retention. Test:
`ambition_portal2d_presentation far_side
far_side_pieces_are_made_each_frame_and_their_materials_do_not_grow`.
Not measured: frame time and GPU cost (no renderer in this harness), the
transit pieces of `visuals.rs`, two panes and two cameras. The agent VM
cannot measure the GPU half: it has no GPU (checked 2026-10-12: no
`/dev/dri`, and Vulkan lists no device), so its offscreen device is a
software rasterizer. So a retained
reconciliation is not justified by growth; only frame time could justify
it. The text below is the case as found.

**Source, `ed0d1ccd`:**
`ambition_portal2d_presentation/src/far_side.rs:62-94` discards every
`PortalFarSidePiece` before reconstructing the participating fragments;
`visuals.rs:117-151` does the same for `PortalBodyPiece`. Both builders
create `PortalClipMaterial` assets inside their per-frame spawn paths.
This is **definite entity/material churn**, not by itself proof of a GPU
regression or unbounded asset retention. In particular, the far-side
module explicitly explains why its fragments are derived, not simulation
state: pose, atlas and clipping must follow the source each frame.

**Preferred presentation contract:** a source-owned, view/room/pane-scoped
reconciliation record may retain only derived fragment entities/materials.
Do not cache the source's authoritative pose or make a second rollback
component. On a changed `DeclaredFrame`, atlas page, texture, flip, transform,
portal clip plane, quality mode, render layer or camera scope, mutate the
owned fragments' appearance; when a source stops participating, remove
exactly its fragments and associated hide reason. Use source and fragment
roles in the key; an ECS entity alone is not an exported stable save identity.
A removal/reappearance of the source must not accidentally reattach its old
fragments to an unrelated new entity.

**Order of work:** instrument first: steady 600-frame stationary case,
animated character, straddling transit, two panes, two cameras, body
teardown and rollback correction. Record piece spawn/despawn, material and
mesh creation, live asset counts and CPU/GPU frame costs on a documented
quality/target profile. Then choose either a retained-fragment reconciler
where the result is measurable or a consciously bounded rebuild where the
extra complexity is not justified. Preserve the existing source-hiding,
last-pose and portal-candidate publication contracts. The already-known
multi-camera pane selection problem is a *different* issue; this task does
not silently claim to solve it.

**Poison:** after a retained path is chosen, force a fresh `materials.add`
for each unchanged source on every `Update`; the steady-state allocation
witness fails. Delete a portal/source; all derived fragments and hide
claims must retire by the correct presentation boundary.

## Acceptance

This program is healthy for Engine 1.0 when:

1. gameplay emits semantic presentation facts without importing renderer
   backends;
2. built-in presentation covers common feedback and remains optional for
   headless/minimal hosts;
3. body-owned Sprite and non-Sprite drawables use one finalization -> portal
   publication -> compositing schedule;
4. portal behavior is correct for far-side, near-side, transit, disjoint and
   multi-pane cases;
5. persistent presentation has semantic ownership and explicit retraction;
6. quality changes visual cost without removing essential gameplay reads;
7. any richer provider is justified by a demonstrated missing capability.

## Standing prohibitions

- no global-z portal bandaid;
- no gameplay dependency on renderer/provider entities;
- no per-effect foreign private-system ordering;
- no hidden second copy of authored sprite geometry;
- no third-party provider added because a category called “particles” exists.

## Consequences of the ownership reassessment

Simulation/contact authorities publish facts; presentation interprets them.
Moving banners, camera effects or feel presentation out of combat must preserve
body ownership, clock semantics and the established portal publication order.
The presence of a presentation module inside `ambition_combat` does not make its
state combat-authoritative.

A9's render-absent profile still validates mechanical geometry and can step
simulation. It must not require device sprites merely to resolve hurtboxes or
construction. Asset quality, resident visual revision and authored physical size
remain separate. A new contact handoff under A2 must not duplicate damage policy
in VFX or reconstruct target identity from display strings.
