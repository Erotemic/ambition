# Portals are moving frames - an engine plan

**Status:** PLANNED 2026-10-15. Jon asked for this after the 10-15 portal lab playtest of the
spinning and swinging plank portals: "is there a refactor that gives us a more elegant and
correct design? how much extra processing power does a portal with a position and angle that
can actually change cost? do we need to special case the static portals to make them faster?"

## The answer, short

- **A moving or turning portal costs microseconds of CPU, so static portals need no special
  case.** Every portal system already runs every tick for every portal, so a still portal is a
  moving portal whose motion is zero. A second code path for still portals is how the
  10-15 bugs got in, because eviction compared normals with `==`.
- **Moving portals cost GPU time, through their captures,** and the 10-15 fixes (`56dc7a240`)
  bought correctness with GPU time:
  - A turned pair captures a square as wide as the view's diagonal, about 2.3x the pixels at
    16:9.
  - A moving pair refreshes its capture every frame, bypassing the quality tier's budget,
    Potato included.
  
  Both work, but neither is the right design (phase 4).
- **The portal does not yet treat its own motion as one rigid motion.** Position, normal, last
  position, last normal and velocity are five fields that each reader combines differently:
  - Transit removes the portal's translation from a body's sweep but not its turn.
  - The exit velocity adds the portal's linear velocity but not its spin.
  - Eviction tells a host's motion from a jump by comparing geometry within a tolerance.

## Measured (2026-10-15)

Measured in `portal_lab` (about 9 pairs, 2 of them on planks) over 900 ticks, on the
`dev` profile (optimized plus debuginfo), at the high tier, with temporary timing probes
that have since been removed. Times are means per tick.

| What | Mean | Note |
|---|---|---|
| All portal sim systems together | ~0.2 ms | out of a 16.7 ms frame |
| Refresh of one portal on a line (plank) | 4.3 µs | looks the line up by name |
| Refresh of one portal on a face (wall) | 0.2 µs | |
| Building the room's surfaces for the refresh | 11 µs | **clones the room's whole collision world** when it has a moving platform; still wall portals pay it too |
| `sync_portal_host_depths` | 45 µs | casts for every portal every tick |
| `resolve_portal_links` / `equalize_pair_apertures` | 31 / 11 µs | |
| `publish_portal_carves` / `portal_transit` / eviction | 28 / 11–19 / 5–7 µs | do not depend on motion |
| `sync_portal_view_cones` (CPU) | 0.33 ms | the same for still and moving pairs |
| Capture render (GPU) | not measured | the same per refresh for still and moving pairs; the size and the rate differ (above) |

## The design

**One idea: a portal is a frame that moves rigidly.** Every reader asks one question: what is
the rigid motion of this portal this tick?

1. **A pose pair replaces five fields.** `PortalPose { origin, normal }`, and the portal holds
   `now`, `prev` and an `epoch`.
   - Velocity and spin are derived from `prev → now` and `dt`. Nothing writes them.
   - A still portal has `prev == now`. There is no branch for it.
   - The opening's length and thickness stay as landed on 10-15.
2. **The epoch is the placement identity.** Only an owner bumps it: spawning, the gun, an
   extension that re-places a portal. A host carrying the portal never does.
   - Eviction fires when the epoch changes or the portal is gone. It drops the geometric test
     and `prev_normal`.
   - The capture rig resets on an epoch change, not on motion.
3. **Transit happens in the portal's own frame.**
   - The body's start sample is `now.from_local(prev.to_local(p))`, which is exact for a slide,
     a spin or a swing. Today the code computes `p + Δorigin`, which misses a plank that turns
     up into a still body.
   - The exit velocity follows rigid motion: `v_rel = v − (v_o + ω × r)` at the entry, and
     `R·v_rel + (v_o′ + ω′ × r′)` at the exit. Today `ω` is missing, so a spinning plank
     neither flings nor catches.
4. **Captures stay registered and stay inside the budget.**
   - The capture camera turns by the portal map's rotation, so its texture is the view's size
     at every angle. This deletes the diagonal square and the special case for a quarter turn.
   - A capture remembers the camera pose it was taken with, and the window samples through that
     remembered pose. A capture the budget throttled is then a slightly old picture in the right
     place, the same as a still pair's capture on a low tier. Today it lags the window.
   - A moving pair goes back under the tier's budget.
   - The capture is padded by the motion it can see in one refresh interval, or refreshed early
     when the window's footprint leaves it.
5. **A host is a handle, resolved once.** When a portal attaches, it records the index of its
   block or line. Refresh then reads that one item's pose. This drops the room-world clone and
   the lookup of a line by name.
   - Host depth is measured when the pose or epoch changes, not every tick. This is a check of
     whether anything changed, not a branch for still portals.
6. **`PlacedPortal::fixed` takes a length.** Callers stop building a box for it to decode.

## Phases

Each phase closes with focused tests that are poison-checked, then a portal lab capture.

1. Pose pair and epoch; eviction by epoch (simulation only; the rollback clone component gains
   fields).
2. Transit in the moving frame and the rigid exit velocity. Tests: a spinning portal scoops up
   a still body, and a body leaves a spinning portal with `ω × r`.
3. Host handles: refresh in O(1) per portal, no clone, depth measured when something changes.
4. A turned capture camera and registered captures. Revert the diagonal square and the budget
   bypass. Measure on Jon's GPU host with `profile_desktop.sh`, and on Potato.
5. `fixed()` takes a length, and the box decoding goes.

## Open

- **Should a spinning plank throw what goes through it?** Rigid motion says yes: the exit
  velocity carries `ω × r`, as it already carries the plank's linear velocity. This is a
  gameplay call. The default is yes, because it is the same rule as linear motion.
