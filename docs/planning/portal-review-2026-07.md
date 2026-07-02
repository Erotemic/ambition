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

### F5. Dead duplicate modules deleted

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

### D4. Air friction on over-speed flings (feel decision)

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
