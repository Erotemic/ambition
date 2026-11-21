# Mary-o World 1-2: roof jump camera diagnosis

Source baseline: `3989cc361`. Camera policy was not changed.

## Report and expected result

A walk followed by a jump from the ceiling bricks lifts the camera. The camera
moves down again on landing. The upper-road view should stay stable during an
ordinary jump. A transition into the corridor must still change the view.

## Probe

The probe used the native Mary-o headless app and the authored `mary_o_1_2`
room. It placed small Mary-o at x = 2000 with her feet at y = 256. Her body and
base height were both 32 units. It drove right for 30 ticks, pressed jump on
tick 30, held jump for 15 ticks, and continued right. The physics rate was
60 Hz. Her feet rose from 256 to 141.75 and returned to 256.

The resulting body samples were replayed through
[`resolve_follow_camera_snapshot`](../../crates/ambition_sim_view/src/camera_snapshot.rs).
The probe used eased mode, no camera snap, default Combat framing, and a 4:3
viewport. This matches Mary-o's desktop presentation profile. It sampled the
camera at 60, 120, and 144 Hz. It did not run a GPU capture.

A control used the same resolver and body samples with an empty camera-zone
list. This control isolates the zone contribution; it is not a selected fix.

| Zoom preset | Camera rise, current policy | Camera rise, zone disabled |
| --- | ---: | ---: |
| Duel (default) | 95.480 | 11.000 |
| Tight | 82.448 | 0.000 |
| Combat | 64.625 | 0.000 |

All distances are world units. At Duel zoom, the 4:3 view is 426 units high.
The current policy moves about 22% of the view height for this jump. At 120 Hz,
the largest camera step was 16.783 units. Early ascent alternated steps of about
15 units with steps below one unit.

## Cause

The authored corridor zone covers y = 256..672. On the roof, the body's centre
is y = 240. `zone_weights` uses the tick centre and a band equal to half the
base view. At Duel zoom, that band is 160 units high. The corridor's weight is
`smoothstep(0.5 - 16 / 160) = 0.352`, even though the body is outside the zone.
The diagnostic `active_camera_zone` stays `None` throughout the jump: the room
has the larger weight. That label does not mean the zone has no influence.

The 4:3 view is slightly taller than the 416-unit corridor. The zone clamp
therefore centres its candidate at y = 464. The normal target at rest is y = 224. The mixed centre is therefore
`224 * 0.648 + 464 * 0.352 = 308.48`. Jumping removes the corridor contribution
and the camera reaches the room's upper clamp at y = 213. Landing restores the
corridor contribution.

The resolver eases its target before it mixes the clamped zone candidates. The
zone weights themselves follow the simulation ticks. Thus the zone contribution
both moves the view through a large distance and adds a step on each physics
tick. A slower target-easing rate alone would not remove that contribution.

A fix should preserve the upper-road composition through ordinary roof jumps
and retain a controlled transition when the body enters the corridor. The
existing drawn-ahead landing case must stay correct; a change from tick position
to drawn position alone does not establish that contract.

Local probe source, logs, CSV files, and plot are under
`/tmp/mary-o-camera-diagnosis/`. The temporary Cargo example was removed after
the run.
