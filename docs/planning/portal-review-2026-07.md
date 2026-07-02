# Portal crates review — findings, fixes, and design decisions (2026-07-02)

Scope: `ambition_portal` (backend), `ambition_portal_presentation` (renderer),
the Ambition adapters (`ambition_gameplay_core::portal`,
`ambition_content::portal`), and the movement-integration seams portals depend
on. Goal per Jon: seamless portals, a reusable Bevy package with a natural
parameterization, and the classic fall-through/re-enter fling puzzle working.

Everything in **Fixed** landed in this review's commit and is test-covered.
Everything in **Design decisions** is written so a weaker agent can implement
whichever direction Jon picks — each entry states the problem, the options,
and a recommendation.

---

## Part 1 — Fixed in this pass

### F1. Preview cones warped at 90° pairs (the reported symptom)

`portal_capture_camera_frame` (view_cones.rs) is the frame source for the
DEFAULT capture mode (`PortalCaptureCameraMode::MappedCameraSnapshot`). It
mapped only the host camera's **center** through the portal map and reused the
host viewport **size unrotated**. A floor↔wall (90°) pair rotates the
viewport's image, swapping width/height — so the capture camera framed a
wide-short region where the content occupied a tall-thin one. Mapped cone
vertices fell outside the capture rect, `vertex_uv` clamped them to the edge,
and the window rendered as smeared/stretched edge texels. Parallel pairs
(floor↔floor, wall↔wall, thin-wall doors) were unaffected — which is exactly
"warped in ways that depend on the angle."

Fix: map the whole host-view rect via the tested `pieces::map_aabb` (exact for
cardinal portals). Since `cone_render` clips the entry polygon to that same
host rect before mapping, every mapped vertex now provably lands inside the
capture frame — the UV clamp can never engage. The `RebuildKey` picks up the
swapped texture dims automatically.

### F2. Horizontal portal flings were braked by held input (the fling puzzle killer)

The player integrator (`ambition_engine_core::movement::integration`) treats
the vertical fall cap as an equilibrium — the comment even says "never
decelerate an over-cap fling like a portal exit" — but the horizontal axis had
no such relax: `approach(along, run * max_run_speed, air_accel * dt)`.
Numbers: `MAX_FALL_SPEED = 1900`, `MAX_RUN_SPEED = 270`, `AIR_ACCEL = 3100`.
A fall converted to horizontal speed by a floor→wall pair was braked from
1900 → 270 px/s in ~0.5 s **if the player held the direction they were
flying** — the natural thing to hold. The fling died almost immediately.

Fix: airborne only, the run cap is now the same relax equilibrium — input
accelerates *up to* the cap and never brakes an over-cap fling in the held
direction; opposing input still brakes at full `air_accel` (air control
preserved); landing restores the ordinary grounded approach (a landed fling
ends, which is the classic behavior). Test:
`an_airborne_fling_above_run_speed_is_preserved_while_holding_into_it`.

Note the remaining friction question in D4 below (no-input decay).

### F3. Fast falls could tunnel past the transfer (the re-enter loop killer)

`transit_step`'s rescue path (the only transfer path while the post-transfer
cooldown blocks `Begin`) required the body to **straddle** the portal plane on
a sampled frame. The straddle window is the body height (~40 px); terminal
fall is 1900 px/s ⇒ 63 px/frame at the 1/30 s sim-step clamp (Android, frame
hitches). A fast body could be above the plane one frame and fully below it
the next — no straddle frame, no transfer — and it grounded at the bottom of
the open carve with its momentum destroyed, "stuck in the floor." This fires
exactly during the speed-building loop: loop period shrinks below the 0.25 s
cooldown as speed grows, so transfers increasingly rely on the rescue, at the
speeds where the rescue's straddle gate starts missing. This is very likely
the "sometimes my infinite-fall doesn't work" bug.

Fix: the rescue gate is now the **open carve volume** itself — transfer when
the centroid is past the plane AND the body intersects `carve_hole(frame)`
AND it is moving inward. The carve is only 60 px deep and aperture-wide, so a
body legitimately under a floor elsewhere is never grabbed, and the only way
into that volume is through the aperture — a deep crossing is still a
crossing. Purely geometric, dt-free (same philosophy as
`APPROACH_CARVE_REACH`). Tests:
`rescue_transfers_a_deep_crossing_inside_the_carve_even_on_cooldown`,
`rescue_never_grabs_a_body_past_the_carve_depth`.

### F4. Item transit only worked for the gun's blue/orange pair

`portal_teleport_ground_items` hardcoded `PortalGunColor::BLUE/ORANGE`:
thrown items sailed straight past authored (purple/yellow…) and link-authored
pairs. Now it iterates every placed portal with a placed partner — one
invariant path for items, like bodies. Also added the missing "moving INTO
the face" gate (`vel · normal < 0`): previously any overlap teleported the
item, even one grazing parallel to the surface.

### F5. Thin-wall pairs: capture self-feedback + window punch-through (round 2, from Jon's report)

Symptom: two portals on opposite faces of a thin wall showed the FAR portal's
preview cone (no line of sight to it) and the near window "recursed
incorrectly" — a spurious nested window with one frame of lag.

Two mechanisms, both fixed:

1. **Every capture camera saw its own window.** With `recursion_depth > 0`
   the capture included the shared `PORTAL_WINDOW_RENDER_LAYER` — which holds
   ALL window meshes, including the rig's own. A window photographing itself
   is never correct optics; on a thin-wall pair the near window's mesh sat
   inside its own capture's source region, so the capture fed back into the
   window as a nested ghost. Fix: each window mesh now carries a per-portal
   layer (`512 + render_slot`) in addition to the shared layer; capture
   cameras include every OTHER portal's per-portal layer (true recursion —
   through a door you legitimately see the door's far face) and never their
   own. The main camera keeps the shared layer — no host change.

2. **Window meshes punched through thin walls.** The finite window recedes up
   to `dynamic_depth_close` (280 px) into the host surface — through a ~20 px
   door wall and out the other side, into the partner's room. That tail was
   on the main-camera layer: the far portal's cone was literally visible
   sticking out of the wall on the viewer's side, and the near window's tail
   sat inside its own source rect (feeding mechanism 1). The config comment
   even says the depth was tuned down to avoid "punching through thin door
   walls" — tuning can't fix geometry. Fix: `host_depth_limit` measures the
   solid material directly behind the face (interval-merge along −normal
   against the viewer's occluder snapshot, `SURFACE_GRACE`-tolerant at the
   face, gap-terminated behind it) and the finite + minimum window depths
   clip to it. The half-plane doorway takeover deliberately stays unclipped —
   crossing the aperture, the whole view becomes the exit chart. A portal
   whose host isn't in the occluder snapshot (one-way platforms) stays
   unclipped rather than degenerate.

Also pinned as a test: the far-side portal's window stays CLOSED for a
near-side viewer under the default visibility mode (the wormhole admission
route is correctly gated on face LOS — the admission logic itself was sound;
the artifacts were pure render-layer/geometry bugs).

### F6. "No drag" — zero-input air braking killed ballistic flight (round 3, Jon's c130/c131 report)

Jon: falling through the c130/c131 pair (the `corner_portal_link1`
floor↔wall 90° corner pair in portal_lab) should sustain at high speed, but
the path decays with no drag in sight. Diagnosis: it IS the integrator, twice
over — with **no input held**, the horizontal block ran
`approach(along, 0, air_accel·dt)` (3100 px/s² toward zero!) plus
`air_friction` (650 px/s²). Neither is aerodynamic drag conceptually — both
are run-stop assists — but together they bled any ballistic horizontal leg at
~3750 px/s². The corner pair converts fall speed into a horizontal leg every
cycle, so each pass decayed visibly. (F2 had already fixed the
*held-direction* brake; this closes the *hands-off* one.)

Fix: airborne with the stick released, speed above `max_run_speed` is
ballistic — neither the zero-target approach nor air friction touches it.
Below the run cap, both behave exactly as before (normal jump drift still
stops), opposing input still brakes at full air control, and landing restores
ground friction. There is now genuinely no air drag in this world. Tests:
`a_hands_off_fling_above_run_speed_has_no_air_drag` (+ the held-direction pin
from F2).

### F7. Capture resolution + the ConeRect parallax issue (round 3)

- **Windows were inherently blurry**: capture density was budgeted in texels
  per WORLD pixel and hard-capped at 1.0, but the main camera renders each
  world pixel at `window_px / visible_view` ≈ 2–3 physical pixels. A
  "pixel-perfect" (1.0) capture was therefore 2–3× under-sampled versus the
  world around it. `capture_dims` (MappedCameraSnapshot path) now multiplies
  in the measured screen scale (primary-window physical size over the host
  visible view, clamped 1–4×), still capped by the quality budget's
  `max_resolution` — so the top quality tier (2048) reaches full screen
  density, High (1024) lands ~1.3×, and mobile tiers keep their VRAM caps.
- **Parallax anchoring decoupled from camera framing**: the per-rig parallax
  copies were positioned from the capture camera's own transform. That is why
  `ConeRect` has "the fundamental parallax issue" — its camera centers on the
  tight source rect, so the background was evaluated at the wedge's center
  instead of the viewer's mapped position. `PortalViewRig` now carries a
  `parallax_anchor` (the HOST camera center mapped through the pair, falling
  back to the source-rect center without a host view), and
  `sync_portal_capture_parallax_layers` anchors copies there. This makes
  parallax correct under ANY framing policy — ConeRect included, so ConeRect
  (tight rect = maximum texel density for free) is now a genuinely viable
  default candidate. Worth an A/B against the snapshot mode.

On **AllowClip**: it is a diagnostic escape hatch, not a real mode — it skips
the entry-polygon clip, so in MappedCameraSnapshot mode the mapped vertices
land outside the fixed capture frame and the UVs clamp (looks "broken" by
construction), and with the default full half-plane preview the unclipped quad
is world-sized. See D12.

### F8. Dead duplicate modules deleted

`src/shot.rs` and `src/pickup.rs` in `ambition_portal` were orphaned earlier
copies of `gun_projectile.rs` / `gun_pickup.rs` — never declared in `lib.rs`,
never compiled. Deleted. (They both defined `PortalShot` /
`portal_fire_system` etc.; any future grep would have found two "truths.")

---

## Part 2 — Design decisions (report for implementation later)

### D1. The portal-map convention is a process-wide mutable global — remove it

`ambition_platformer_primitives::math::PORTAL_MAP_ROTATION` is an
`AtomicBool` read by `portal_map_vec` and everything built on it (transit
position/velocity, pieces, carves, views, copies). Problems:

- **Two disagreeing defaults.** The global defaults to `false` (Reflection);
  `PortalTuning::default()` is `Rotation`. They're reconciled only by
  `sync_portal_tuning_convention` running each frame. Any pure call before
  the first sync — headless tests, tools, a host that forgets the sync system
  — silently uses the other convention. (`PortalConvention::default()` is
  also `Reflection`, a third inconsistent default.)
- **Reusability.** A "fold into any 2D game" crate cannot own a process
  global: two Bevy `World`s (tests, server+client, portal-in-portal minigame)
  can't have different conventions; `cargo test` threads race on it (the
  existing tests already tiptoe around this with `_for_convention` variants).

Direction (recommended): delete the global. The pure layer already has the
right shape — `portal_map_vec_reflection` / `portal_map_vec_rotation` and
`*_for_convention` variants exist for every consumer. Make the convention a
parameter everywhere: add it to `PortalFrame`-consuming entry points or —
simpler — pass `&PortalTuning` (already threaded into `transit_step_with_tuning`)
down to `map_point`/`map_aabb`/`view` call sites. The presentation crate reads
`Res<PortalTuning>` (it already can). `portal_map_vec(v, n_in, n_out)`
becomes `portal_map_vec(v, n_in, n_out, convention)`. Mechanical, ~40 call
sites, no behavior change once defaults are unified to `Rotation`.

Cheap interim if the full threading is deferred: change the atomic's default
to match `PortalTuning::default()` (Rotation) and fix the one test that pins
the untouched global.

### D2. Reusable-crate parameterization (the "natural parameterization")

What a host should be able to set without editing the crate, currently
hardcoded as consts:

| Knob | Today | Proposal |
|---|---|---|
| `PORTAL_OPENING_HALF` (46) | `pub(crate)` const | field on a `PortalDefaults` resource; authored portals already override via `half_extent` |
| `PORTAL_THICKNESS_HALF` (9) | const | same resource; couples capture box, visuals, exit clearance |
| `CARVE_DEPTH` (60) / `SURFACE_GRACE` (16) | pub consts in `pieces` | move to `PortalTuning`; carve depth should scale with the largest transiting body (it's "just past a body's half-depth" by design — make that relationship explicit: `carve_depth = max_body_half_depth * 1.5` or a plain knob) |
| `APPROACH_CARVE_REACH` (96) | const, budgeted for Ambition's 1900 px/s + 1/30 s clamp | `PortalTuning` field; document the budget formula `max_speed * max_dt` |
| `PORTAL_SHOT_SPEED` / `PORTAL_MAX_RANGE` | consts | gun module tuning — fine to leave, gun is compat |
| `TELEPORT_COOLDOWN_S` / `MIN_EXIT_SPEED` | already in `PortalTuning` | ✓ done |

Channel identity: `PortalChannel` bakes Ambition's palette (8 named colors +
gun slots) into the core API (the file's own FIXME agrees). Proposal: core
pairs portals by an opaque `PortalKey(u64)` with `partner()` defined by the
link layer; the color palette becomes a presentation-side naming/display map.
`PortalLink` + `resolve_portal_links` is already 90% of this — make the link
path primary and derive gun/authored channels through it.

`PlacedPortal`/`PortalFrame` normals are cardinal-only in practice (pieces
are AABB-backed). The frame math already names normals/tangents; arbitrary
angles need polygon clipping + non-AABB pieces. That's a real project — keep
it on the roadmap (the module docs already say this), don't fake it with
bounding boxes.

### D3. Transit cooldown scope

`PortalTransitCooldown` blocks `Begin` at **every** portal for 0.25 s after
any transfer. Consequences: entering a *different* pair within 0.25 s is
blocked (chained-portal rooms will hit this), and the fast re-entry loop
leans entirely on the rescue path (now robust, but the Begin path is the one
that opens the carve latch early and plays the ENTER cue).

Options:
1. **Pair-scoped cooldown** (recommended): `PortalTransitCooldown { channel,
   remaining }` — only the pair just crossed is latched. Ping-pong through
   the same pair is still prevented; chained pairs and fast loops flow.
2. Keep global, shorten to ~0.1 s. Less surgery, less correct.
3. Replace with a pure geometric guard (no cooldown at all): the transfer
   maps the centroid onto the exit plane and the rescue's `vel·n < 0` gate
   already prevents the immediate re-grab; the cooldown mostly guards the
   *Begin* path. Needs a careful headless soak (floor↔floor bounce, wall↔wall
   turnaround) before trusting it.

### D4. Air friction on over-speed flings — RESOLVED by F6 (hands-off flings are ballistic)

After F2, a held-direction fling is preserved, but with **no input** air
friction (650 px/s²) decays a fling toward zero — a 1900 px/s fling dies in
~3 s. Portal-the-game preserves ballistic flight with hands off the stick.

Options:
1. Exempt over-cap speed from friction: friction only acts on the portion of
   `|along|` below `max_run_speed` (friction's job is stopping *run*
   momentum, not ballistic momentum). One-line change next to the F2 relax.
2. Keep as-is: hands-off decay reads as air drag; holding the direction (the
   natural input) preserves the fling anyway.

Recommendation: option 1 for frame-agnostic purity (a ballistic body's decay
shouldn't depend on whether a *run* input happens to be held), but this is
pure feel — playtest.

### D5. `equalize_pair_apertures` destructively shrinks the authored opening

It writes the min opening back into `PlacedPortal::half_extent` every frame.
Once shrunk, the authored length is gone: if the partner is later removed or
re-authored larger, the survivor keeps the shrunk aperture forever. Static
authored rooms never notice; anything dynamic (script-moved portals, the gun
re-firing onto a link channel someday) will.

Proposal: keep the authored opening as the source of truth (either an
`authored_half_extent` field or compute the effective opening at read time:
`effective_opening(portal, partner) = min(...)` used by carve/transit/visual
call sites). The "min of the pair, centered" rule itself is right.

### D6. Capture-camera `order` is derived from query iteration order

`Camera { order: -8 - i as isize }` where `i` is the index in the portals
query — Bevy query order is not stable (see the standing query-order rule).
Rig respawns can shuffle capture order between cameras, which with the
1-frame-lag recursion changes which window sees fresh vs stale content —
subtle shimmer when multiple pairs are live. Fix: derive order from the
channel's stable render slot (`portal_channel_render_slot`), e.g.
`order = -8 - slot as isize`.

### D7. Presentation player-centrism (relativity violations, flagged FIXMEs)

- `sync_portal_body_pieces` uses `body_visual.single_mut()` — only **one**
  `PortalSceneBody` gets an exit copy. Any NPC/enemy/item mid-transit shows
  no emerging copy (it pops at the centroid instead). The decomposition is
  already per-body pure math; loop over all tagged bodies.
- `sync_portal_disorientation_indicator` queries
  `PlayerEntity + PrimaryPlayer` directly inside the reusable presentation
  crate (its own FIXME). Should read a host-tagged focus marker (the
  `PortalCameraContinuityFocus` pattern already exists — reuse it).

### D8. Item transit is a poorer cousin of body transit

Even after F4, `PortalTransitable` items teleport on overlap with an
instant pop to `exit.pos + clearance`: no aperture sinking, no centroid
crossing, the **lateral offset along the face is discarded** (everything
exits at the portal center), and no cooldown (fast items between two facing
portals can multi-teleport within a frame sequence). Bodies get the real
aperture machine.

Direction: fold items onto the same `transit_step` path — they already have
pos/vel/half-extent, which is all `transit_step` needs. `PortalPolicy
{ reorient: false, carry_velocity: true }`, no `ActorRoll`. This deletes
`portal_teleport_ground_items` entirely (convergence: smaller code, one
invariant path). The only reason it exists separately is history.

### D9. View-cone tint default contradicts its own docs

`PortalViewConeConfig::tint` doc explains the recursion-attenuation design
("slightly below white … 1.0 brings back the chaos") but the default IS
`1.0`. If facing-portal recursion ever looks like a full-brightness fractal,
this is why. Decide: either the doc is aspirational (set `0.94`-ish) or the
default is deliberate (trim the comment). Recommendation: `srgb(0.93, 0.95,
0.97)` — convergent recursion with a barely-visible cool cast that also
subtly distinguishes "through a portal" from direct view.

### D10. Stale numbers in load-bearing comments

`APPROACH_CARVE_REACH`'s budget comment says "950 px/s terminal fall" and
"~700 px/s shot"; the actual tuning is `MAX_FALL_SPEED = 1900` and
`PORTAL_SHOT_SPEED = 1900`. The 96 px reach still covers the actor case at
1/30 s (63 px) but NOT a 1900 px/s body on a 100 ms hitch (190 px) — the
carve could be closed for one frame under a hard hitch at max speed. The F3
rescue now recovers the crossing regardless (the transfer no longer depends
on the carve being open on the exact crossing frame), so this is mitigation
rather than correctness — but recompute the budget when parameterizing
per D2, and fix the comment.

### D11. Link-group index cap

`resolve_portal_links` clamps group index to 63 (`gi.min(63)`); a 65th
distinct link id in one room silently shares channels with the 64th —
cross-linked portals. Unlikely soon; assert/log at the clamp when touched.

---

## Part 3 — The fling puzzle, end to end (why it should now work)

The loop "fall into floor portal A, exit elevated portal B, fall back into A,
each pass faster, then redirect through a wall portal to launch":

1. **Speed builds** up to `MAX_FALL_SPEED` (1900 px/s): gravity's fall cap is
   relax-style, so portal-carried speed above the cap is preserved and speed
   below it grows each pass. ✓ (already worked)
2. **Every pass transfers**, even when the loop period is inside the 0.25 s
   cooldown and even at 63 px/frame: the rescue now fires on the carve-volume
   gate, not a lucky straddle frame. ✓ (F3)
3. **The launch survives**: horizontal exit speed above run speed is no
   longer braked by holding the flight direction. ✓ (F2)
4. Placement: nothing in the mechanic requires special placement anymore.
   The remaining practical constraint is geometric — the exit must be
   positioned so re-entry is fallable (B above A / B on the ceiling). If a
   specific room still fails, dump `debug_traces` (OOB flight recorder) and
   check whether the transfer fired (`ambition::portal` log target prints
   "transferred through the portal pair" per crossing).

Residual risks to playtest: D3 (cooldown blocking a *different* chained pair),
D4 (hands-off fling decay), and landing mid-loop (grounded braking ends a
fling by design).

### D12. Source-clip policy enum has one real variant

`PortalViewConeSourceClipPolicy`: `ClampToFrame` is the correct behavior,
`FitToFrame` is currently an exact alias of it, and `AllowClip` is incoherent
under the default MappedCameraSnapshot mode (unclipped vertices fall outside
the fixed capture frame → clamped UVs → smear) and degenerate under the full
half-plane preview (world-sized quad). Recommendation: delete `AllowClip` and
`FitToFrame` (pre-release, no compat tax) and collapse the enum — or keep
`FitToFrame` only if an aspect-preserving fit is actually planned soon.

### D13. Quality-tier capture caps vs screen density

With F7 the capture targets true screen density; the per-tier
`portal.max_resolution` caps now decide the delivered sharpness: 2048 (top
tier) is fully crisp at 1080p-class windows, 1024 (High) lands ~1.3 texels
per world px, lower tiers keep their VRAM budgets. If High should be crisp on
desktop, raise its cap to 2048 and let the tier system differentiate by
platform instead. The `texels_per_world_px` knob now reads naturally as a
fraction of screen density (1.0 = pixel-perfect), matching its original doc.

---

## Part 4 — Round 4 (Jon's lab feedback, 2026-07-02): landed + the approved work program

Jon's decisions on Part 2 are now RECORDED here; items marked LANDED shipped
in this round's commits, the rest form the approved queue below.

### Landed this round

- **ConeRect is the default capture mode.** Its "fundamental parallax issue"
  was the parallax anchor, not the mode: rig parallax copies anchored to the
  capture camera's own framing center. They now anchor at
  `PortalViewRig::parallax_anchor` (the host camera mapped through the pair),
  so parallax is framing-independent and ConeRect's tight rect wins on
  density.
- **View cones are cones again.** The half-plane takeover was a `view_strip`
  whose NEAR edge spanned the wall laterally (the "growing trapezoid"), and
  the wedge clamped its far corners to a lateral limit, bending the true rays
  through the aperture endpoints. Every wedge now keeps its near edge pinned
  to the aperture and its far corners on the real rays (`RAY_LATERAL_CLAMP =
  1e5` is f32 comfort, not geometry); the viewport clip is the only lateral
  bound. Correct at every distance: the fan's rays steepen toward vertical as
  the eye reaches the opening — the half-plane limit — with the apex always
  at the aperture.
- **D3 geometric guard (thin-wall wrong-side entry FIXED).** New
  `PortalHostDepths` seam: the host measures the solid material behind each
  face (`measure_host_depth`, shared with the window depth clip) and portal
  core bounds the aperture volume by it. Begin and the carve engagement gate
  on the FRONT side of the plane (the capture box reaches through thin
  material); the rescue and the mid-fall-through carve use
  `carve_hole_with_depth`. A body behind a thin wall can no longer open,
  enter, or be grabbed by a portal it cannot see.
- **D3 pair-scoped cooldown.** `PortalTransitCooldown { remaining, pair }`
  blocks re-Begin only into the crossed pair; chained pairs flow. The rescue
  still ignores it.
- **D4 CLOSED for real: zero air drag.** Hands-off airborne motion is fully
  ballistic at ANY speed (the sub-run-speed stop assist was the c138/c139
  bounce decay — the lateral component of the back-and-forth). New
  conservation audit: real integrator + real transit_step, 40 crossings, no
  drift. Marked blind for feel (release-stick air drift now persists).
- **D6 LANDED** (stable capture order), **D9 LANDED as doc** (seamless =
  pure-white tint is intentional; field stays as a recursion attenuator),
  **D10 LANDED** (comment corrected), **D11 LANDED** (overflow link groups
  refuse + warn instead of clamping into cross-links).

### Approved queue (Jon's direction, for implementation)

**Q1 — Per-portal map convention (D1, upgraded).** Jon: remove the process
global; rotate-vs-reflect should be PER PORTAL PAIR (scale stays constrained
for now). Plan:
1. Add `convention: PortalConvention` to `PlacedPortal` (and `PortalFrame`),
   defaulting from `PortalTuning.convention` at spawn/authoring; LDtk `Portal`
   entity gains an optional `convention` field; `resolve_portal_links` copies
   the authored value onto both ends (a pair must agree — warn + prefer the
   first end if authored inconsistently).
2. Thread it: `portal_map_vec(v, n_in, n_out)` → takes the ENTRY portal's
   convention (the `_for_convention` pure variants already exist for every
   consumer — transit velocity/position, pieces `map_point`/`map_aabb`, view
   maps, copy transforms, somersault/facing/input-warp policies). Delete
   `PORTAL_MAP_ROTATION`, `set_portal_map_rotation`,
   `sync_portal_tuning_convention`, and the F10 global toggle (retarget F10
   to flip the AIMED pair's convention instead).
3. Unify the three defaults (enum default, tuning default) to `Rotation`.
   Every literal `PlacedPortal { .. }` in tests gains the field — mechanical.

**Q2 — Parameterization for reuse (D2, approved).** Move
`PORTAL_OPENING_HALF`, `PORTAL_THICKNESS_HALF`, `CARVE_DEPTH`,
`SURFACE_GRACE`, `APPROACH_CARVE_REACH` into `PortalTuning` (defaults =
today's constants; document the `max_speed × max_dt` budget rule for the
approach reach). Then the opaque-channel step: pair by `PortalLink`-style key
everywhere, palette becomes presentation-side naming.

**Q3 — One aperture path for EVERYTHING (D7+D8, approved).** All bodies —
actors, projectiles, thrown items — transit as apertures via `transit_step`.
Items: replace `portal_teleport_ground_items` + `PortalTransitable` with
`PortalBody` + `BodyKinematics`-shaped sync (item layer mirrors pos/vel/size
each frame as it does today); delete the instant-teleport path. Presentation:
`sync_portal_body_pieces` loops over ALL `PortalSceneBody` entities (host
tags every actor's visual, not just the player's) so every straddler gets an
exit copy; the disorientation indicator reads a host focus marker instead of
`PrimaryPlayer` (reuse the `PortalCameraContinuityFocus` pattern).

**Q4 — Authored aperture is the source of truth (D5, approved).**
`equalize_pair_apertures` must stop destructively shrinking
`PlacedPortal::half_extent`. Either store `authored_half_extent` alongside,
or (cleaner) compute the effective opening at READ time:
`effective_opening(portal, partner) = min(...)` used by carve/transit/visual
call sites, `half_extent` stays authored.

**Q5 — Ledge-grab THROUGH a portal.** Jon wants wall abilities to stay
enabled during transit AND for a ledge-grab to work through an aperture. The
current suppression exists because the carve's cut edges read as grabbable
ledges. Direction: make the ledge/wall probes PORTAL-AWARE instead of
suppressed — a probe ray that enters an aperture continues in the exit chart
(`raycast_through_portals` already exists for rays); a carve edge itself is
identified by overlap with the active carve holes and excluded from
grab/cling candidates (geometric exclusion, not ability toggling). Then
delete `suppress_ledge_grab_during_transit` + the
`SuppressWallAbilitiesInPortal` toggle. Depends on Q3's "one aperture path"
only weakly; can proceed independently.

**Q6 — AllowClip/FitToFrame removal (D12, pending Jon).** Recommended delete;
one enum + dev-UI labels.

---

## Part 5 — Round 5: carried momentum, arbitrary rotations, and the Opus triage

### Q7 — LANDED: carried momentum (the tight-control ↔ conserved-fling middle ground)

Jon wanted Hollow-Knight tightness (release the stick → fall straight down)
without bleeding portal momentum. The parameterization:

- `BodyFlightState::carried_run` — signed run-axis velocity the WORLD
  imparted. `apply_portal_carried_momentum` (actor-generic, content adapter)
  sets it to the mapped exit velocity's run component on EVERY transfer.
- `MovementTuning::air_stop_assist` (default 3750 px/s² = the pre-ballistic
  hands-off feel) decays the run component toward the CARRIED FLOOR, not
  zero. Ordinary jumps: floor 0 → tight. Flings: conserved.
- Opposing input brakes at full air control and eats the floor; the per-frame
  clamp of carried to the actual velocity makes walls and landing consume it
  with no special cases. `MovementTuning::carried_decay` (default 0)
  optionally bleeds it. Both knobs are in the F3 editable-tuning mirror.

The same channel is the natural home for knockback and wind later.

### Q8 — Arbitrary portal rotations: what's already correct, what's Fable-hard

Audit of the crate by layer (pinned at 45° by
`slanted_normals_are_exact_in_the_vector_layer`):

**Already fully general (correct today for any unit normal):**
`portal_map_vec_{reflection,rotation}`, `portal_rotation`, `portal_tangent`,
`map_point` (exact inverse, depth→front at any angle), velocity transit,
`portal_transit_roll` / somersault / facing / input-warp policies (all
dot/tangent algebra), `front_distance`, `window_eye`, `aperture_wedge_multi`
+ the whole view-cone construction (built in the (normal, tangent) frame),
`PortalViewMap` / `copy_transform` factorization, per-vertex UV mapping
(affine — handles rotated maps by construction), `measure_host_depth`.

**Cardinal-only but MECHANICALLY generalizable (Opus-safe, with the
acceptance test "exactly equivalent for cardinal normals"):** `straddles`
(rewrite as |front(center)| < body support along n + lateral overlap via
tangent projection), `portal_fits` (project body size onto the tangent),
`capture_box`/`approach_box` overlap tests (replace AABB intersects with
front/lateral projection intervals), `clip_to_aperture` bounds,
`portal_half_extent` consumers (already produce bounding boxes of the tilted
face — documented). `map_aabb` stays a conservative bounding box for
non-90° maps — fine for capture rects, NOT for collision.

**Genuinely hard (Fable-tier):** the two places where AABB is load-bearing:
1. `compute_body_pieces` — slanted planes cut AABBs into pentagons; the piece
   model needs convex polygons (parry is already a dep via CombatVolume).
2. The host carve — `subtract_aabb` can't cut a slanted hole out of an AABB
   collision world; needs either polygon collision solids or a stepped-AABB
   approximation of the slanted aperture (ugly). This is really an ENGINE
   collision-representation decision, not a portal one.

Recommendation: do the mechanical generalizations now (they cost nothing for
cardinal play and remove most of the cliff), keep the two hard walls
documented, and design slanted CARVING together with any future slope/ramp
collision work — they're the same problem.

### Opus triage of the queue

| Item | Verdict |
|---|---|
| Q1 per-portal convention | **Opus-safe.** Mechanical threading; `_for_convention` variants exist; tests pin everything. |
| Q2 consts → tuning + opaque channels | **Opus-safe.** The channel-key step needs the plan followed exactly (link-first pairing). |
| Q3 one aperture path (items/actors) | **Opus-safe with care.** The item↔kinematics sync is specified; content tests cover regressions. |
| Q4 authored aperture | **Opus-trivial** (read-time min). |
| Q5 ledge-grab through portals | **Fable-tier** (or Fable writes the spec first). Portal-aware probes touch collision internals + feel; carve-edge exclusion needs geometric judgment. |
| Q6 delete AllowClip/FitToFrame | **Opus-trivial.** |
| Q8 mechanical generalizations | **Opus-safe** with the cardinal-equivalence acceptance tests. |
| Q8 polygon pieces / slanted carve | **Fable-tier**, couple to the engine slope/collision decision. |

---

## Part 6 — Round 6: window compositing order (thin-wall c136/c137 double-sprite)

### F9 — LANDED (blind): the view window draws over portal frames + the exit copy

Jon: on the thin-wall door pair the portals/sprites always drew OVER the cone,
so the exit body copy read as a second sprite laid on top and the far portal's
frame punched through the window — breaking the illusion. Root cause was the
draw order: window mesh z ≈ 8.55, portal rim/core/label 9.0–9.2, and the exit
body copy at `WORLD_Z_PLAYER` = 20 — the window sat under everything. The exit
copy is ALSO captured by the portal cameras (it's on the world layer), so it
appeared once in world space AND once inside the window texture — the literal
"drawn twice."

Fix (one declared z band, `PORTAL_WINDOW_Z` = 9.5 / `PORTAL_EXIT_COPY_Z` =
9.4 in the presentation lib):
- The window now draws OVER the portal rims/labels and over the exit copy. It
  is a captured composite of the far side, so it reads as the single seamless
  source rather than sitting beneath the far portal's frame.
- The exit copy sits just BELOW the window: an OPEN window captures it on the
  far side (one seamless body) and hides the redundant world draw behind
  itself — killing the double — while a CLOSED window (LOS blocked / windows
  off) still shows it over the rim as the emerging-body visual.
- Both stay BELOW actors (`WORLD_Z_PLAYER` = 20), so a near-side actor in
  front of the aperture still correctly occludes the window (the window
  recedes INTO the surface; a body in front of the surface is nearer).

Relies on the window material being `AlphaMode2d::Opaque` (Opaque2d phase
writes depth; Transparent2d sprites depth-test), so z ordering crosses the
mesh/sprite boundary. Marked BLIND — verify in the lab; if the artifact
persists it is the phase interaction, not the z value, and the window would
need a transparent material sorted by z instead.

Tradeoff: the entry portal's own rim/label are now covered by its open window
(fully seamless glass, per D9). If a thin identifying border is wanted back,
draw it as a dedicated overlay ON TOP of the window — a small follow-up.

### Q9 — Thin-wall overlapping windows are fundamentally painter's-ambiguous

Two portals on a thin wall have windows that overlap in screen space and share
the `PORTAL_WINDOW_Z` band; they sort only by viewer proximity, and each
portal is simultaneously "entry" (own frame wanted on top) and "far" (should
be hidden inside the partner's window) — a contradiction no single z-per-entity
resolves. The current fix makes the common case read correctly; a fully
unambiguous thin-wall composite needs per-window stenciling (mask each
window's pixels to its own aperture and composite in explicit order) or a
depth-prepass portal id. Fable-tier; defer until thin-wall doors are a shipping
mechanic.

---

## Part 7 — Round 7: crossing flicker at the thin-wall seam

### F10 — LANDED: the crossed pair always refreshes its capture

Jon: a small flicker remains crossing the c136/c137 thin-wall door. Prime
suspect (tier-dependent): the quality tiers below High set
`max_active_captures: 1` with `min_refresh_interval_s` 0.05–0.25s, so a
thin-wall pair's TWO windows can't both refresh every frame — they alternate /
go stale, which reads as a flicker exactly while you move across the seam.
Slots were handed out in unstable query order with no regard for which portal
the viewer is at.

Fix: `portal_at_seam` — a portal within the viewer's own reach of the eye (and
its partner beside it on a thin wall) ALWAYS refreshes, bypassing the slot cap
and the refresh interval. Away from portals the ordinary budget applies, so it
costs nothing except at the moment it matters. Applied in both the update and
spawn allocation passes. This removes the throttle-induced seam flicker on
constrained tiers.

### Q10 — Residual crossing flicker on UNTHROTTLED tiers (High/Ultra)

If the flicker persists on High/Ultra (where both windows already refresh
every frame), F10 is not the cause and two tier-independent candidates remain,
both cross-specific:

1. **The real sprite's authoritative snap.** At the centroid crossing the body
   position jumps by the portal separation (map_point). For a thick-wall
   single portal this jump is hidden inside the aperture; on a thin wall
   (~32px, both portals on screen) you SEE the whole real sprite pop ~32px
   sideways, because the real sprite is drawn UNCLIPPED at the raw kinematic
   pos while the exit copy handles the other side. The seamless fix is
   texture-clipped body pieces: draw the real sprite clipped to its `here`
   slice and the copy to its `through` slice (both via `Sprite.rect` for
   cardinal portals, or a clip material), so the two slices tile continuously
   across the seam and nothing pops. This is the standing "texture-accurate
   atlas clipping" TODO — Fable-tier (anchor/flip/trimmed-frame interactions).

2. **`window_eye` nearest-end handoff.** `compute_cone`'s through-portal route
   resolves the eye through the NEARER end; at the thin-wall midpoint the
   nearer end flips (door1↔door2), so the resolved eye — and thus the unioned
   wedge shape — can jump discontinuously in one frame. Fix: blend the two
   ends' resolved eyes across a small band around the midpoint instead of a
   hard nearest-pick (the face-continuity guarantees they nearly coincide
   there, so a short crossfade removes the pop). Opus-safe with a
   continuity pin test.

Disambiguation for Jon: does the flicker scale with the Visual Quality tier?
If lowering the tier worsens it and raising it removes it → it was the capture
throttle (F10 fixes it). If it is tier-independent → it is Q10, and the
1 (texture clipping) is the real seamless-crossing fix.

---

## Part 8 — Round 8: texture-clipped body pieces (Q10 candidate 1)

### F11 — LANDED (blind): the transiting body draws as two shader-clipped pieces

Jon confirmed the crossing flicker is tier-independent (Potato / Low / High all
show it), which per Q10's disambiguation makes candidate 1 — the UNCLIPPED
real sprite — the cause. His symptoms match: the sunk slice of the real sprite
popping onto the far side of the thin wall reads as "a second player actor
sprite drawn for just a second," and the ~32px authoritative snap at the
centroid crossing is the click.

Fix: the standing "texture-accurate atlas clipping" TODO, implemented as a
clip SHADER rather than `Sprite.rect` (Jon's suggestion, and the right one —
`rect` would re-derive anchor/flip/trim per frame; a fragment-shader
half-plane test against final world positions is exact for all of them for
free). New `PortalClipMaterial` (`Material2d`) in `ambition_portal_presentation`
(`clip_material.rs` + embedded `shaders/portal_clip.wgsl`): it draws the
sprite's current atlas frame on a mesh quad — the hit-flash overlay's proven
sibling-mesh pattern — and discards fragments behind up to three world-space
clip half-planes (plain vec4 uniforms, WebGL2-friendly).

`sync_portal_body_pieces` now realizes the Core invariant in the renderer:
while a `through` piece exists the real sprite is HIDDEN and both charts draw
as clip-material quads —

- **here**: the real pose, clipped to the front of the entry plane (the sunk
  slice no longer draws over the wall / the far side);
- **through**: the `copy_transform` exit pose, clipped to the front of the
  exit plane plus the exit aperture span (only the emerged slice draws).

Because the portal map is an isometry the two slices tile continuously across
the seam: at the centroid snap the charts swap roles but their union is the
same set of pixels, so nothing pops. Both pieces draw at the actor z (the F9
hide-the-copy-below-the-window compositing is unnecessary once the copy
contains only emerged pixels — a piece is a body in front of the surface, and
it draws crisp instead of only appearing through the blurry capture at low
tiers). The pieces are on the world layer, so portal captures show the clipped
charts too.

Fallback: texture/atlas not yet loaded, or a headless host that never
registered the material (the system's asset params are `Option`al) → the old
behavior exactly (visible real sprite + unclipped exit copy at
`PORTAL_EXIT_COPY_Z` below the window).

Verification: piece decomposition already pinned in `ambition_portal::pieces`;
new unit tests pin the plane render-space conversion (y-flip), the
anchor/size/rotation quad pose, the atlas UV basis, and three headless app
tests pin transit → hidden sprite + 2 clip pieces at the correct poses /
planes, no-transit → untouched sprite, missing texture → fallback copy. The
WGSL itself is parsed + validated by naga in `cargo test` (same front-end wgpu
uses at runtime). Marked BLIND for the visual outcome — verify crossing
c136/c137 slowly on Potato and High.

Known gaps (follow-ups, filed in TODO.md): non-player actors (nothing but the
tagged scene body transits visually today), and sibling overlays — a hit-flash
mid-transit draws its whole silhouette unclipped, the held gun sprite doesn't
decompose. Q10 candidate 2 (the `window_eye` nearest-end handoff) remains
open — if a residual WINDOW-shape pop survives F11 (Jon's "only half of one
side of the exit" moments), the midpoint eye-blend is the next fix.

### F11 iteration 2 (Jon's screenshots): the through slice goes back UNDER the window

Jon verified in the lab: the clipping itself is right (with the cones off the
crossing reads correctly — "the shader looks great without the cones"), but
with windows on, the composite was chaotic — doubled robots over the glass and
half-portals (one rim color missing) depending on crossing depth.

Cause: iteration 1 moved the through slice up to the actor z. While crossing,
the entry window's doorway TAKEOVER is a glass pane showing the whole exit
chart, and that capture already contains the through slice; drawing the slice
on top painted a second, parallax-offset copy over the glass AND covered the
exit portal's front rim half (the half-portals). Jon's diagnosis — "the sprite
is being drawn on top of the cone, even though it doesn't need to be" — was
exactly it.

Fix: the through slice returns to `PORTAL_EXIT_COPY_Z` (the F9 compositing),
keeping only the clipping from F11. The open window's glass is the single
source of the far-side image (it captures the clipped slice, so the capture
tiles correctly too); a closed window still shows the slice over the rim. The
`here` slice stays in the actor band (a body in front of the entry surface
correctly occludes the glass).

Also DELETED the legacy Transit Masks effect (`effect_transit_masks` feature,
`PortalVisualEffect::TransitMasks`, the mask-box spawns) per Jon: the clip
shader is the finished version of what the opaque boxes approximated —
strictly better, nothing left to A/B. The dev-menu cycle is now
View Cones / Off, where Off = bare clipped pieces (still a valid profiling
baseline for the windows' capture cost).

Any residual thin-wall weirdness after this is the standing window-side queue:
Q9 (overlapping windows are painter-ambiguous without per-window stenciling)
and Q10.2 (`window_eye` nearest-end handoff pop at the midpoint).

## Part 10 — Round 10: c136/c137 residual flicker + "a portal only half appearing"

Jon (with screenshot, standing IN the thin-wall portal): the crossing flicker
persists and at least one symptom is a portal only half appearing. Two
mechanisms found in the window compositor, both structural:

### F15 — LANDED (blind): pairwise pane dominance + the frame as an overlay

**Flicker:** a pair's two takeover panes are OPAQUE meshes whose z came from
`proximity_z` — inverse RADIAL distance to each portal. Around a thin-wall
seam the two distances are near-tied everywhere (they cross exactly at the
midpoint where you stand while transiting), so the depth sort could hand the
top pane back and forth frame-to-frame between two DIFFERENT composites —
the flicker. Fix: `pane_z` — between a pair's own two panes the winner is
now decided by PAIRWISE FRONT-SIDE DOMINANCE (signed front distance to this
face minus the partner's — antisymmetric, zero exactly at the material
midpoint) with a 6px hysteresis band carried on the rig (`pane_dominant`), so
sub-pixel eye jitter while standing in the seam can never alternate the
panes; a decisive crossing hands the pane over exactly once, at the same
midpoint where the F12 `window_eye` crossfade already lives. Across DIFFERENT
pairs (and for same-plane pairs, whose fronts coincide) the proximity ramp
still orders windows as before. Pinned by four unit tests (antisymmetry +
midpoint zero, exactly-one-flip sweep, jitter stability, same-plane
proximity order).

**Half-portal:** rims (z 9.0), cores (9.1) and labels (9.2) all drew UNDER
the glass band (9.5–9.85), so an open takeover pane hid the partner's rim
and the back color-half of its own — literally half a portal. Fix: the
identifying frame is now an OVERLAY at `PORTAL_RIM_OVERLAY_Z` = 10.0
(core +0.05, label +0.1) — above every pane, below actors. A portal always
draws whole; a body in front still occludes it; the emerging `through` slice
passes behind the thin rim bar (the ring reads as in front of the body it
emits). This consciously revises F9's "seamless glass covers the entry's own
rim" tradeoff — Jon's half-portal report is the vote for always-whole
frames. The glass remains the single source of the far-side IMAGE (exit
copy + captures below it, unchanged). Pinned by a headless visuals test
(every rim/core/label z above the window band top, below the actor band).

Marked BLIND: verify crossing + standing in c136/c137 on a low tier and
High. If a residual artifact survives, the remaining known ambiguity is Q9
proper (per-window stenciling for overlapping panes).

## Part 9 — Round 9: "fall in from a tall height, don't come all the way back up"

Jon: falling through a ground portal pair from a tall distance doesn't return
to the original height — something damps the speed; feels like a regression;
"I thought we had a test for it." Two independent causes found, both real:

### F13 — LANDED: carried momentum must not launder controller drift

The `floor_portal_bounce_does_not_thud_onto_the_open_floor` app test was
FAILING at HEAD (125/240 frames grounded — a genuine uncaught regression from
the Round 5 carried-momentum work). Tracing the bounce showed a constant
±20 px/s lateral velocity surviving every transfer: at the moment of transfer,
`apply_portal_carried_momentum` floored `carried_run` at the WHOLE exit run
velocity — including the controller-owned walk-in residual the stop assist was
mid-way through braking. That reclassified controller drift as permanent world
momentum, so a vertical ground-pair bounce marched sideways ~20px per leg
until it landed OFF the aperture, grounded, and died — the "momentum killed at
the portal" feel.

Fix (the transfer ROTATES momentum, it must not RECLASSIFY it): recover the
pre-transfer velocity through the inverse map (the map is an isometry; the
swapped-normal map is its inverse, pinned at 45°), split off the
controller-owned run (pre-transfer run minus the old carried floor), map that
through the portal, and floor `carried_run` at only the WORLD-imparted
remainder. A genuine fling (gravity-earned fall speed rotated out of a wall
portal) is all world-imparted and still floors at full strength — the Round 5
promise holds; a walk-in residual decays to zero after exit exactly like
ordinary hands-off drift. The thud test passes again; the spike-frame
momentum audit and every conservation test stay green.

### F14 — LANDED (blind, feel): no terminal velocity for the player

With F13 in place, the remaining tall-drop shortfall is arithmetic, not a bug:
the transfer is lossless (new round-trip pins prove a sub-terminal drop
returns to its height exactly), but the shipped player tuning had
`max_fall_speed: 950` (gravity 2250), so ANY fall saturates at 950 px/s and
the rebound apex saturates at `950²/2g ≈ 200px` no matter how tall the drop.
Terminal velocity IS air drag on the down-leg — it contradicts the Round 4
"ZERO air drag, pure ballistic at ANY speed" decision that already governs the
lateral axis.

Fix: `sandbox.ron` `max_fall_speed` 950 → 100000 (effectively none): a
hands-off fall is pure ballistic, so a ground-pair bounce returns to the drop
height from ANY distance. Marked BLIND for feel — long falls now keep
accelerating past 950; if landing readability suffers, lower the knob again
(the comment in the RON documents the tradeoff: a finite cap re-shortens tall
portal bounces to cap²/2g). Engine default (1900) and per-actor caps are
untouched; the boss `max_fall_speed: 0` float contract is unaffected.

Pinned in `ambition_content` portal tests (`floor_floor_round_trip_*`):
sub-terminal round trip conserves height; uncapped round trip conserves ANY
height; capped round trip saturates at exactly cap²/2g (the diagnostic number
for future "portal damped my fall" reports). The app spike-frame harness now
sizes its strike-zone band from the live fall speed instead of hardcoding
terminal 950.

### F12 — LANDED: `window_eye` crossfades the nearest-end handoff (Q10.2)

`window_eye` now resolves BOTH ends and, when both resolve, crossfades the two
resolved eyes across a 24px `EYE_HANDOFF_BAND` around the equidistance
midpoint; outside the band the nearest end wins exactly as before, and
single-end resolution is unchanged. Both inputs sit cleanly in front of the
entry (front ≥ MIN_FRONT/2) and front is affine, so every blend is a valid
wedge eye; the wormhole flag stays the discrete nearest end. This removes the
one-frame wedge-shape jump when the nearer end flips — the thin-wall doorway
crossing and the walk between a same-plane pair (where the direct and
via-partner images are a full pair-separation apart). Pinned by two
continuity sweeps in `ambition_portal::view` tests (same-plane pair: the old
hard-pick's ~400px jump fails the bound; thin-wall doorway: centered and
off-center walks stay smooth).
