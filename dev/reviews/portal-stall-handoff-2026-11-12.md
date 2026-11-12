# Portal stall handoff

Work is paused at the maintainer's request. The playable stall is unresolved.
The full portal goal remains paused and incomplete. No build or test was
started for the latest case.

## Checkout and completed work

Use the assigned main checkout, `/home/joncrall/code/ambition`.
The starting HEAD for this handoff is `84fe3f9a909d`. All implementation is
committed locally. Do not push; the maintainer explicitly prohibited it.
Do not change slot 1. Do not commit binary files, including profile captures.

`22d36dd3d` caches fixed portal-link poses within each immutable curved query.
Matched lab instruction estimates decrease 50.5%. A short desktop lab interval
without Tracy averages 8.8 ms/frame. This does not establish performance at
all stations or during all crossings.

Two later corrections address narrower station E cases:

* `3d0e411db`: exclude line colliders proved disjoint from a piece by its fixed
  world planes. Preserve nonempty boundary strata and input refusals.
* `0a3be0efc`: prove that an already rejected contact component persists using
  complete world-plane sets and affine collider-point motion. Keep the body
  containment proof and the conservative proof for unsupported cases.

Both old recorded E cases pass in the real app under rollback, with progress
and no resets. All 101 material-cast tests and the 80/40 partial-contact,
reverse-crossing and bit-identical replay guard pass. The broader suite is
not run. These checks did not establish a complete playable fix.

## Latest failure: approach to the slope opening

Bundle:
`dev/ambition_dev_measurements/profiles/desktop-timeline-run-20261112T034409Z`.
It records source `84fe3f9a909d`, no Cargo features, and no Tracy. This confirms
the latest fixes were present. Some frames take about 2.2 seconds. The final
census reports 15 simulation ticks per frame and about 136 ms/tick in
integration. The body is frozen while simulation time continues.

Latest F8 trace:
`debug_traces/ambition_gameplay_trace_1794455416-534364433-000000_20769d03h50m16s.json`.
It contains 240 frames, ends at tick 935, and reports `Manual (F8)`.

| Tick | Position | Velocity |
| --- | --- | --- |
| 897 | (2972.9697, 842.68115) | (0, 580.33185) |
| 898 | (2972.9697, 852.97833) | (0, 617.83185) |
| 899 | (2972.9697, 863.9005) | (0, 655.33185) |
| 900 | (2972.9697, 875.4477) | (0, 692.83185) |
| 901 through 935 | same as tick 900 | same as tick 900 |

Input is neutral at these ticks. The body is airborne, not flying or in
hitstun. Reported size is approximately (30.065935, 48.000004); its reported
AABB is axis-aligned. The trace does not store the full physical material
transform. Unlike the two older witnesses, no portal position jump precedes
this freeze. It occurs while falling toward the slope opening, about 28 pixels
to the right of the older forward approach.

No real-app reproducer or exact refusal has yet been obtained for this newest
case. Do not assume that it has the same internal failure as the older cases.

## Lead, not a confirmed cause

Station E uses channels c130/c131. Its slope aperture is at approximately
(2950.065, 892.1126), normal (-0.4984711, -0.8669063), half-length 46. The wall
aperture is at (3456, 900), normal (-1, 0), half-length 46. The slope segment
is (2832, 960) to (3032, 845).

For an upright body following the latest recorded fall, the projected center
at the entry-plane crossing is about 26.4 units along the aperture tangent.
The body's tangent half-extent is about 25.0. Their sum exceeds the aperture's
46-unit half-length. This suggests a rim or failed-fit case: the body should
stop or slide while it remains controllable. The fit calculation is a lead;
it is not a verified explanation of the refusal or work cost.

A focused next reproduction can use the existing setup in
`game/ambition_app/tests/portal_lab_usable.rs`, but start before tick 900 at the
new recorded position and velocity. Inspect
`BodyFrameOutput.events.motion_refusal`. The current E tests cover one fitted
forward fall and one fitted reverse walk. They do not cover approaches across
the aperture width, rejected fits, or withdrawal from a rim contact. Avoid
another conclusion based only on those two fitted paths.

## Relevant owners and records

* `crates/ambition_platformer2d_core/src/frame/crossing.rs`: root crossing and fit.
* `crates/ambition_platformer2d_core/src/frame/material_cast/arcs.rs`: conditional
  regions, world planes, and subdivision work.
* `crates/ambition_platformer2d_core/src/frame/material_cast/curved.rs`: curved
  collision, rejected components, and overlap-continuity proof.
* `crates/ambition_platformer2d_core/src/movement/kernel.rs`: refuses the candidate
  and retains the old body when movement or righting admission fails.
* `dev/reviews/portal-slope-stall-2026-11-12.md`: older F8 witnesses and corrections.
* `target/portal-e-reverse-final/`: latest completed validation commands and logs.

The earlier station J stop is also unverified. The full portal acceptance
plan and device/performance gates remain open (autoplay navigation was
fixed later on 2026-11-12).

Before Cargo, inspect `scripts/setup/target_bindmount.sh --status`; the target
is bound to local VM storage. No owned build or test remains running. Preserve
unrelated edits in the measurements, map, music and SFX submodules, untracked
sprite work, and `docs/related-work/ideas.md`. Keep profile and F8 files outside
Git tracking.

## Takeover, 2026-11-12

The lab stall had two causes outside any one station. Every body received
every link in the room, so a turning plank anywhere put every body on the
curved builder. And a refused step froze the body with its velocity, so each
later tick repeated the refused search. Material queries now take only the
links a body can reach, and a refusal steps with closed portals or stops the
body. The idle lab falls from 13.0 to 2.6 ms of CPU per tick. The newest E
fall no longer freezes. The PSIM log entry "Reachable links and playable
refusals" records the measurement and the station J defect that the
room-wide link set had hidden.
