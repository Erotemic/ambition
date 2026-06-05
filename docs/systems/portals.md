# Portals

The portal gun's flagship ability — and, more importantly, a general spatial
primitive: a **portal pair topologically glues two parts of the world together**.
Step into one, emerge from the other carrying your momentum. This document
describes how portals *ideally* work (the pure rules we aim for), then the
deliberate **gameplay accommodations** where we bend those rules for feel.

The guiding principle: **the fewer accommodations, the better.** Pure portal
physics is the target; every accommodation is a debt we take on knowingly and
document here. As of this writing there are two gameplay-feel accommodations
(input warp + auto-orientation) plus a couple of smaller stability/robustness
ones — all listed below.

Code: `crates/ambition_sandbox/src/portal_pieces.rs` (the pure math — the "Core
invariant") and `crates/ambition_sandbox/src/portal.rs` (the ECS systems). The
test lab is the LDtk-authored `portal_lab` room (see the bottom of this doc).

---

## Ideal portal physics

These are the rules a "correct" 2D portal obeys. Items marked **(implemented)**
are live; **(not yet)** are part of the ideal we have not built.

### A portal is an aperture, not a trigger *(implemented)*

Touching a portal does **not** instantly teleport the whole body. A body begins
transit when its leading edge crosses the portal plane, the authoritative
center transfers when its **centroid** crosses, and transit ends when the
trailing edge clears. The body is continuously present on both sides during the
crossing — "feet in, feet out".

### The host surface becomes non-solid inside the opening *(implemented)*

A floor/wall containing a portal cannot stay fully solid at the aperture, or a
body could never sink in. Collision **carves** the host block within the opening
(and only there — the rim and surrounding geometry stay solid). The carve is
*transient*: it exists only while a body is actively transiting that portal, so
there is no permanent walk-in pocket to exploit.

### One logical identity, multiple spatial pieces *(implemented)*

A crossing body is **one** object (health, status, AI, cooldowns, inventory,
ownership are all singular) with **two** spatial pieces: the slice still on the
entry side, and the slice mapped through to the exit. This is the **Core
invariant**: every gameplay query that asks "where is this thing?" should use
those pieces, not the raw body AABB. `compute_body_pieces` is the one function
that produces them.

### Position, velocity, and orientation transform through one tangent-preserving map *(implemented)*

Each portal has a **normal** (out of the surface) and a **tangent** (along the
surface — the "second normal" that fixes which way is "right" along the doorway;
canonically the normal rotated +90°, `portal_tangent`). The portal map
(`portal_map_vec`) sends the component going *into* the entry *out* of the exit,
and carries the **tangent (along-surface) component straight over**. Position,
velocity, the recursive ray, and the warped input all use this one map, so they
can never disagree.

This is the key correctness point vs a *bare rotation*: two same-facing floor
portals are a 180° rotation, and 180° flips *both* axes — so falling in moving
right would come out moving **left** (mirrored). The tangent-preserving map keeps
the horizontal component, so you **keep moving right** (and come out *up* of the
exit floor). That's the intuitive, physically-consistent behavior given a
standard tangent — an ideal property, not an accommodation. The debug overlay
draws the tangent as a green tick so its direction is visible. (If a specific
pair ever needs a flipped tangent, it can become an authored per-portal field;
today it's derived consistently from the normal.)

### Restricted to axis-aligned portals *(implemented; by design for now)*

Floor / wall / ceiling portals (normal is ±x or ±y) compose cleanly with AABB
collision. Arbitrary angled portals need clipped polygons and are deferred until
the mechanic is fully stable.

### Blocked exits behave like ordinary obstruction *(implemented via carve + collision)*

A portal is never rejected just because full-body clearance is unavailable at the
exit. Partial emergence is allowed; ordinary exit-side collision geometry decides
how far the body can come out. (Today this falls out of the carve + the normal
collision sweep; a fully blocked exit simply lets less of the body through.)

### Transit is a latch / state machine, not a bare cooldown *(implemented)*

`PortalTransit` tracks entering / centroid-crossed / clear, so re-entry is
prevented by *state* (the body must clear the plane) rather than only by a timer.

### Recursive ray-like queries *(primitive implemented; not yet wired)*

Line of sight, beams, grapples, and aim traces should cast until a portal, then
continue transformed through the link, bounded by a small recursion depth.
`raycast_through_portals` implements this; it is **not yet** wired into grapple /
aim / LoS call sites.

### Portalized hit volumes + owner dedup *(not yet)*

Hurtboxes are not the only thing that should split — attacks, projectiles,
sensors, pickups, hazards, and interaction volumes should all route through the
same piece logic, so a body half-through a portal is hittable on its visible
piece(s) and **not** on its clipped-away part. A hit overlapping both pieces of
one owner should count **once** (store the logical target + which piece). Impact
FX happen at the visible contact piece. The `compute_body_pieces` primitive
exists; the damage-path wiring does not.

### Sized projectiles transit gradually; point projectiles transform on center *(not yet)*

Thrown/sized projectiles should straddle + transit like bodies (with a per-
projectile policy: pass-through / die / bounce / stick / ignore). Point-like
projectiles can simply transform on center crossing. Today projectiles and ground
items instant-teleport.

### Attached objects use their owner's chart *(not yet)*

Held items, weapons, riders, weak points should use their owner's portal chart —
a sword swing from a half-through actor originates from the visible hand piece,
not the authoritative center.

### AI perception + pathfinding *(not yet)*

Simple → advanced: AI targets authoritative centers (today) → perceives/attacks
visible exit-side pieces → pathfinding treats linked portals as graph edges.

### Placement validity + crush rules *(not yet)*

A portal must fit its host surface, must not overlap invalid geometry, and two
portals must not overlap unstably. Forcing a body into blocked geometry through a
portal should block movement (telefrag avoided unless intentional). Today
placement is "nearest solid face" with no validity gate.

### Chart-aware VFX + camera, recursion bounds, moving platforms *(partial / not yet)*

SFX already fire at the entry/exit portal positions. Still ideal: entry/exit
ripples, a textured portal sprite (vs the rim+core bars), recursion limits on
portals-seeing-portals, and portals on moving platforms (transform the frame over
time + moving-frame velocity correction). The camera should *support*, not
*define*, the effect.

---

## Gameplay accommodations

Where pure portal physics would feel bad in a gravity-bound platformer, we bend
the rules — deliberately and minimally. Each accommodation below notes **what
pure physics says**, **what we do instead**, and **why**.

### 1. Held-input warp + emergence guard

- **Code:** `PortalInputWarp` + `PortalEmission` + `warp_portal_input` (input
  layer), set on transfer in `portal_transit_system`.
- **Pure physics says:** a portal transforms the *body* (position, velocity,
  facing). It has no business touching the player's controller.
- **What we do — the warp:** on a **same-wall turn-around** crossing (both portals
  perpendicular to gravity, where `portal_facing_flips` holds), the held movement
  axis is mirrored at the input layer (`ControlFrame`), before the player brain
  reads it, so a held *right* keeps carrying you *left* out the exit instead of
  fighting the reversed exit velocity and yanking you back. Soft — it drops the
  moment movement is released or clearly redirected.
- **The expressibility rule:** the warp is applied **only** when the warped
  direction is something the controller can express as movement — i.e. the
  same-wall case, where it stays a horizontal mirror. For a floor↔wall (90°)
  crossing the portal map would rotate a *horizontal* hold into *vertical* ("up"),
  which the controller can't use for horizontal movement, so we **do not warp**
  there.
- **What we do — the emergence guard (`PortalEmission`):** for a short window
  after **every** crossing, held input that pushes back *into* the exit wall
  (against the exit normal) is stripped, so the floored exit velocity carries the
  body out — "don't let input cancel the portal emission". This is gravity-general
  (it works off the exit normal vector, not a fixed axis) and covers the cases the
  warp deliberately skips (e.g. holding *right* as you fall a floor portal and
  emerge *left* from a wall portal: physics carries you left, the held-right can't
  re-cancel the emergence, then control returns).
- **Why:** these keep the held input from fighting the portal's transformed
  velocity without ever feeling like a hard input latch.

### 2. Auto-orientation (the somersault + gravity-righting)

- **Code:** `somersault_roll` + `ActorRoll` + `update_actor_roll`.
- **Pure physics says:** the body leaves the exit in whatever orientation the
  portal map dictates and stays there.
- **What we do:** on transfer the body picks up the portal's on-screen turn as a
  transient roll, then continuously eases back to **gravity-upright**. A body that
  goes feet-first into a floor portal tumbles out and rights itself; the character
  never stays upside-down.
- **Sub-rule — no tumble on a wall↔wall turn-around:** when both portal normals
  are perpendicular to gravity (a same-direction wall pair), the transit is a pure
  horizontal *turn-around*, not a tumble, so `somersault_roll` imparts **zero**
  roll — the body comes out already upright. Floor↔floor / ceiling↔ceiling keep
  the full 180° somersault; floor↔wall keeps 90°.
- **Sub-rule — facing mirrors on that same turn-around** (`portal_facing_flips`):
  a 180° somersault rotation inherently mirrors the sprite left↔right. Since the
  wall↔wall case *suppresses* that rotation, the mirror would be lost and the body
  would emerge **back-first** ("face in, back out"). So in exactly the
  suppressed-180° case the horizontal facing is flipped, giving the wanted "face
  in, face out" (really **X-in, X-out**: whatever part led in leads out). Every
  other case carries its orientation in the rotation, so facing is left alone.
- **Why:** a gravity-bound platformer character should read as "which way is
  down" = gravity, regardless of the non-Euclidean geometry. The somersault sells
  the transit; the righting keeps the character controllable; the wall-wall
  suppression avoids a jarring, pointless flip.

### Minor / robustness accommodations

These are smaller deviations — stability and discretization fixes rather than
feel changes — but they are still departures from a purely ideal portal.

- **Minimum exit speed** (`MIN_EXIT_SPEED`, in `transit_step`): pure physics
  conserves momentum exactly, so a very slow walk-in would emerge very slowly and
  could stall inside the opening. We floor the exit speed *along the exit normal*
  so a slow walk still pops cleanly out the far side. Tangential momentum is
  preserved.
- **Anti-ping-pong latch + cooldown** (`PortalTransit` clear rule +
  `TELEPORT_COOLDOWN_S` / `PortalCooldown`): the state machine prevents re-entry
  until the body clears the plane, and a short cooldown backstops it. An ideal
  portal wouldn't need the cooldown; it's belt-and-suspenders against
  discrete-timestep oscillation. (A floor↔floor pair *intentionally* still loops —
  you fall in, fly up out the other, arc back down — that's correct, not ping-pong.)
- **Surface grace on the carve** (`SURFACE_GRACE`, in `carve_hole`): an authored
  portal's face can land a few px off the grid-snapped collision edge (e.g. a
  floor whose IntGrid top is y=896 but the portal face is y=900). The carve
  reaches one grid cell *outward* past the face to clear the thin solid lip that
  would otherwise survive in the opening and hold the body up. This is a
  discretization fix (continuous portal vs grid collision), not a feel change.
- **Begin-on-touch capture box** (`TRANSIT_BEGIN_MARGIN`): transit begins when the
  body touches the opening's thin capture box (face + a small margin), a hair
  before the geometric plane crossing, so the carve can open before the body needs
  to sink. A purely ideal aperture would begin exactly at the plane.
- **Wall abilities suppressed while transiting** (`suppress_ledge_grab_during_transit`):
  the carve splits the host block, and the new edges read as grabbable ledges /
  climbable walls — so without this you can grab or cling "into" a portal (and pop
  back out the entry instead of crossing). While mid-transit, ledge-grab, wall-
  cling, wall-jump, and wall-climb are suppressed (the real abilities are saved and
  restored, and suppression only starts once transit begins — so wall-jumping *up*
  to reach a portal still works). Grabbing the *outer* rim of a wall portal mid-air
  is still possible and is not currently considered wrong.

### Debug aids

- **Portal color labels** + **tangent tick** (green) draw on/near each portal so
  you can refer to a pair precisely and see which way its tangent points.
- **Disorientation indicator** (`sync_portal_disorientation_indicator`): a small
  glyph over the player shows exactly while the held input is portal-warped (the
  "holding left, moving right" state), and vanishes when the warp drops. A
  placeholder for a nicer effect (incl. on the joystick visual) later.

## Gravity

The orientation + input rules are written **gravity-parameterized**, not
hard-coded to "down": `somersault_roll`, `portal_facing_flips`, and the emergence
guard all take the gravity direction / the portal normals, so they generalize to
flipped or sideways gravity. The one place still effectively assuming down-gravity
is the same-wall input *warp itself* (it mirrors the screen-horizontal axis); the
**gate** on it is gravity-aware, so under non-down gravity the warp simply won't
engage where it shouldn't. Correctness under arbitrary gravity is verified for the
down case first; the seams are in place to finish it without a rewrite.

---

## Implementation map

| Concern | Where |
| --- | --- |
| Pure piece math (Core invariant) | `portal_pieces.rs`: `compute_body_pieces`, `map_point`/`map_aabb`/`portal_rotation`, `clip_halfspace`, `straddles`, `front_distance`, `subtract_aabb`/`carve_hole` |
| Transit state machine (shared by player + actors) | `portal.rs`: `transit_step` → `TransitStep`, `portal_transit_system`, `portal_transit_actors` |
| Host-surface carve | `portal.rs::publish_portal_carves` → `FeatureEcsWorldOverlay.portal_carves` → `world_with_sandbox_solids` |
| Partial render (feet in / feet out) | `portal.rs::sync_portal_body_pieces` (draws the sprite twice + masks the invisible slice) |
| Accommodation: input warp | `portal.rs`: `PortalInputWarp`, `warp_portal_input` |
| Accommodation: auto-orientation | `portal.rs`: `somersault_roll`, `ActorRoll`, `update_actor_roll` |
| Recursive ray | `portal.rs::raycast_through_portals` |
| N linked pairs by color | `portal.rs::PortalColor` (5 complementary pairs; `partner()`) |
| LDtk-authored static portals | entity `Portal` (color + normal) → `convert_portal` → `spawn_portal` |

Key constants live next to their use: `CARVE_DEPTH` / `SURFACE_GRACE`
(`portal_pieces.rs`), `MIN_EXIT_SPEED` / `TELEPORT_COOLDOWN_S` /
`TRANSIT_BEGIN_MARGIN` / `PORTAL_OPENING_HALF` (`portal.rs`).

---

## The test lab (`portal_lab`)

`portal_lab` is an LDtk-authored room of pre-placed static portal pairs for
eyeballing the math across orientations. Reached by a door on a basement one-way
platform. Four stations, each a distinct complementary color pair so it's clear
which two link:

| Station | Pair | Case | Expected |
| --- | --- | --- | --- |
| A | purple ↔ yellow | ground ↔ ground | feet-in, tumble + reorient (loops — correct) |
| B | teal ↔ red | wall ↔ wall (same wall) | turn-around, **no** somersault, upright |
| C | green ↔ magenta | ground ↔ ceiling | straight through, no rotation |
| D | cyan ↔ rose | ground ↔ wall | feet into floor, out the wall (90°) |

The portals are authored, not code-spawned: edit
`tools/ambition_ldtk_tools/specs/portal_lab_area.ron` and re-run `area create`,
or `entity add` a `Portal` (fields `color`, `normal`). They work **without** the
portal gun — authored pairs are not gun-owned. The integration test
`tests/portal_lab_usable.rs` guards that a gun-less player can actually enter them.
