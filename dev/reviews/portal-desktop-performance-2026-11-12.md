# Portal desktop performance

## Follow-up without Tracy

`desktop-timeline-run-20261112T012923Z` uses source `c635d39e74ca`, the same
desktop and `profiling` build profile, no Cargo features, and no Tracy capture.
Manual input enters the Hall and then the lab. The prior captures use the
`profile` feature. This comparison changes both instrumentation and input;
it is not a controlled measurement of profiler overhead alone.

| Interval in the run without Tracy | Mean frame time | Approximate FPS |
| --- | ---: | ---: |
| Hall, wall time 13.5 to 18.9 s | 8.68 ms | 115.2 |
| Hub, wall time 20.5 to 23.5 s | 3.16 ms | 316.7 |
| Lab, wall time 26 to 67.6 s | 57.72 ms | 17.3 |
| Lab, wall time 55 to 67.6 s | 32.83 ms | 30.5 |

These are frame-count-weighted census means. Window boundaries can include
frames just before the named interval. All selected Hall windows are within
its loaded interval, from 12.401 to 19.490 seconds.

This evidence changes the Hall diagnosis. The earlier 3.8 FPS result is not
a representative measurement of the engine without instrumentation. The
driver costs about 4.5 ms per rendered frame in two 300-frame Hall brackets
without Tracy, compared with 244.58 ms in the earlier traced bracket.
Hall integration still costs about 4.08 ms per simulation tick without Tracy,
compared with 5.62 ms in the traced interior interval.

The traced driver spends about 16.68 ms per simulation tick, near the full
60 Hz budget before rendering and other frame work. Its catch-up loop then
runs many ticks in one rendered frame. This explains how added per-tick
overhead can cause a much larger change in rendered frame time. Do not quote
the ratio between these FPS values as the profiler's direct per-tick cost.

The lab remains expensive without Tracy. Its census reports about 13.15 ms
per tick in `PlayerSimulation` and 1.86 ms in `Integrate` from wall time 26
to 67.6 seconds. The same three portal-pose functions below account for
22.06% of whole-run self CPU samples, including boot, hub and Hall time.
Their repeated work remains the first runtime optimization target. The late
lab interval has more active portal captures than the early interval despite
its better frame rate. Camera count alone does not explain the slowdown.

The maintainer confirms that a memory-pressure incident occurred during the
earlier run. Keep that long stall separate from the sustained simulation cost.
The responsible allocation or process is still not established by these files.

## Captures and limits

The first two captures use an i9-11900K, an RTX 3090 with Vulkan, the `profiling`
profile, and the `profile` feature. VSync and the frame cap are off.

The bundles are under `dev/ambition_dev_measurements/profiles/`:

* `desktop-timeline-run-20261112T010446Z`: source `cc235f30cf3b`.
  Autoplay enters the Hall, returns to the hub, and cannot reach the lab.
* `desktop-timeline-run-20261112T012742Z`: source `c635d39e74ca`.
  Manual input enters the lab. The Rust runtime has no changes between these
  two source commits. The second commit changes the autoplay stop order.

The native reports contain self CPU samples, without call stacks. Their
percentages are not inclusive call costs or frame-time percentages. These
captures have different inputs and populations. They do not establish a
before/after speedup or the size of a historical regression.

The first capture contains a valid Tracy trace and aggregate CSV. Its generated
summary incorrectly says that no Tracy artifacts exist. This analysis exports
system instances from that trace and uses each instance's start time. The
second capture has no Tracy trace: its client disconnected during the initial
handshake. Its native profile and census remain available.

## Hall: repeated simulation steps dominate frame time

The Hall loads at wall time 22.207 seconds and unloads near 110.568 seconds.
The interior census interval from 35 to 105 seconds contains 267 rendered
frames with a weighted mean of 261.36 ms, about 3.83 FPS. The wider interval
from 22.3 to 110.1 seconds averages 224.64 ms. Hub windows from 111 to 198
seconds average 10.58 ms.

The Hall has 524 bodies and no active portal capture cameras. In the Tracy
interval from 35 to 105 seconds, the driver runs 273 times and the simulation
systems run 3,825 times: about 14 simulation ticks per rendered frame.
The interval boundaries use the trace clock; census boundaries use log time.

| Hall zone | Mean per call | Calls | Total |
| --- | ---: | ---: | ---: |
| GGRS driver | 235.41 ms | 273 | 64.27 s |
| GGRS advance closure | 16.68 ms | 3,825 | 63.79 s |
| Body integration | 5.624 ms | 3,825 | 21.51 s |
| Perception collection | 1.081 ms | 3,825 | 4.14 s |
| Actor brains | 0.807 ms | 3,825 | 3.09 s |
| Target selection | 0.512 ms | 3,825 | 1.96 s |
| Combat geometry view | 0.418 ms | 3,825 | 1.60 s |
| Render schedule | 11.686 ms | 273 | 3.19 s |

The driver and advance closure contain the simulation systems. Do not add
these overlapping rows. Integration alone costs about 79 ms per rendered
frame. A separate census bracket reports 244.58 ms inside the driver over
300 frames ending at wall time 101.733 seconds.

Native Hall samples identify `map_volume`, material-cell construction,
polygon axis contact, grid queries, and ledge probes. The axis movement path
builds material cells for both motion and penetration repair. Cell construction
projects and validates the material at the start and at both interval ends.
These costs apply without portals. Other simulation systems also have material
cost; integration is not the whole driver cost.

## Lab: repeated room-link pose evaluation

The manual capture loads the lab at wall time 15.732 seconds. Frame windows
from 19 to 32 seconds average 323.30 ms. Windows from 40 to 70 seconds average
299.15 ms. The lab has 11 bodies. It has 34 portal capture rigs, with at most
two active in the sampled rows.

In native chunk 07, the following distinct self sample shares total 47.34%:

| Function | Self CPU samples |
| --- | ---: |
| `arcs::Scene::pose` | 23.85% |
| `rotation_ending_at` | 11.96% |
| `PreparedApertureStep::sample_wide` | 11.53% |

That chunk covers 58% to 66% of the native sample span. Whole-run shares for
the same functions total 27.94%, which includes boot and hub time.

The source supports a specific cause. `MaterialCast::cells` selects the curved
builder when any admitted link rotates. `Scene::states` samples every room
link on each evaluation and allocates a new pose vector. The current scene
cache stores prepared aperture steps, but not their sampled poses. Static
links therefore repeat pose calculations during curved queries too.
The lab census attributes about 15.11 ms per tick to `PlayerSimulation`
and 1.58 ms to `Integrate` in wall-time windows from 35 to 70 seconds.
Without a lab call stack or Tracy trace, this does not identify one consumer
system as the sole cause.

## Separate long stall and navigation failure

The first run has a 57.76-second frame ending at wall time 269.760 seconds,
after the Hall has unloaded. A driver call takes 55.27 seconds. The native
windows in this interval show substantial kernel page walking, reclaim, and
spin-lock work. Multiple systems become slow together. This supports memory
pressure as a hypothesis. The bundle has no process RSS or memory-pressure
series to establish the cause or the responsible process.

The original route reports `NoRoute` at wall time 110.700 seconds when it
requests the hub's portal-lab door. Changing the stop order does not fix that
navigation defect. The maintainer reports that the reversed route also fails.
The manual capture proves that the authored door can be used.

## Next work

First remove repeated pose work within the canonical curved solver. Reuse
fixed-link poses and evaluate only the link poses needed by a query region.
Any candidate exclusion must conservatively include moving apertures and
later portal charts. Keep numeric refusals and contact ordering.

Then reduce repeated source projection and temporary allocation in ordinary
material movement and repair. Keep one material collision authority. Do not
restore root-AABB repair or change the Hall population to improve the result.

Measure the Hall and lab separately with matched inputs before and after
each change. Check movement and replay at the affected seam. Fix the generic
navigation failure before using autoplay as the acceptance workload.
Record process memory and pressure during a repeat if the long stall returns.
The first runtime optimization is measured below.

## Fixed-link pose reuse

The curved query scene now keeps one sampled pose for each fixed link. A link
is fixed when both aperture steps have zero displacement and turn, or when
the query samples one world instant. Other links still sample their recorded
motion. Invalid clocks use the full sampler to keep refusal order. The cache
belongs to one immutable query scene and has at most one entry per link.
It does not change the material collision authority or exclude candidates.

### Matched simulation measurement

Baseline source is `2a63bf880`. The optimized arm changes only
`crates/ambition_platformer2d_core/src/frame/material_cast/arcs.rs`.
Both binaries use rustc 1.95.0, the `profiling` build profile, no Tracy,
and the same content hashes on the local VM. No build runs during measurement.
The sprite watcher is off in both arms. Each arm has a 60-tick warmup in
each room, followed by three interleaved pairs of 60- and 360-tick processes.
Low/high order alternates. The estimate is `(total_360 - total_60) / 300`.
The table gives the median of the three estimates.

| Room | Baseline million user instructions/tick | Reuse | Change |
| --- | ---: | ---: | ---: |
| Portal lab | 219.597 | 108.701 | -50.50% |
| Hall | 80.329 | 80.371 | +0.05% |

The lab estimates range from 219.297 to 219.646 million instructions/tick
before the change and 108.461 to 108.781 after it. Median process CPU time
falls from 16.694 to 9.259 ms/tick. These are startup-subtracted process
estimates, not direct counters around individual ticks or desktop FPS.
The Hall's baseline ranges from 78.150 to 80.751 million instructions/tick.
Do not interpret its small median change as a demonstrated regression.

All 24 measured processes complete with 100% instruction-counter coverage.
Every process matches content hashes and the expected simulation census:
524 bodies and zero portals in the Hall; 11 bodies and 34 portals in the lab.
All requested ticks and shared-clock active steps run.
Raw counters, binary hashes, source metadata and commands remain under
`target/portal-lab-pose-cost/`. Binary files are not committed.

The 101 material-cast tests pass, including the fixed/moving sample and invalid
clock comparison. The assembled 80/40 crossing and replay check passes. It
checks stationary partial support, persistent scale and reflection, reverse
travel, and bit-identical repeated input under rollback. Validation commands:

```bash
cargo test -j12 -p ambition_platformer2d_core --lib frame::material_cast
cargo test -j12 -p ambition_app --test app_it -- portal_similarity_slice::the_authored_80_40_pair_carries_partial_contact_scale_parity_and_reverse_travel
```

The app command runs one test. The broader suite is not run for this change.
These checks do not measure displayed frame pacing or GPU cost. The autoplay
navigation failure remains open.

### Desktop confirmation

`desktop-timeline-run-20261112T020509Z` uses source `f0c5a90beaf1`, the same
desktop, the `profiling` profile, and no Tracy feature or capture. Manual input
enters the Hall at wall time 13.951 seconds, leaves it near 73.383 seconds,
and enters the lab at 77.958 seconds. The capture ends near 90 seconds.

Lab census windows from 79.5 seconds to the end contain 1,139 frames with
a weighted mean of 8.803 ms, about 113.6 FPS, and a maximum of 32.61 ms.
The lab still has 11 bodies and 34 capture rigs. Sampled active captures
average 0.6. The Hall interval from 20 to 70 seconds averages 10.161 ms,
about 98.4 FPS, with 524 bodies and zero portal capture rigs.

In the earlier run without Tracy, the late lab interval averages 32.832 ms,
about 30.5 FPS, with one active capture. The manual inputs and camera activity
differ. These frame-time values are an observed comparison, not a controlled
speedup estimate. The matched headless measurements above establish the CPU
cost reduction with equal inputs.

Lab `PlayerSimulation` decreases from 12.587 to 8.024 ms per tick across
these selected desktop intervals. `Integrate` decreases from 1.684 to
1.185 ms per tick. The reduced tick cost leaves room for rendering without
the earlier accumulation of simulation steps in each rendered frame.

The last native window in each run lies within the lab. The old self sample
shares for `Scene::pose`, `rotation_ending_at`, and aperture sampling total
37.66%. The new shares for `Scene::pose`, its extracted `sample_pose`, rotation,
and aperture sampling total 9.35%. Sample shares have different denominators;
they support the mechanism but are not a direct cost ratio.
Memory copies now lead the last window at 8.78% of self CPU samples. Repeated
pose/state vector construction remains a candidate for a later measurement.

This is a short displayed confirmation of the optimization. It does not cover
a long lab session, every station, all camera positions, or mobile performance.
The autoplay navigation defect and the remaining PSIM gates stay open.
