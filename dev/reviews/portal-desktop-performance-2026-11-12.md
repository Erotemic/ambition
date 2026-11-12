# Portal desktop performance

## Captures and limits

Both captures use an i9-11900K, an RTX 3090 with Vulkan, the `profiling`
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
No runtime optimization is implemented by this analysis.
