# Android Hall contact and perception work

The Android capture `android-all-ambition-auto-20261124T045252Z` uses native
build ID `81b89f22cb23bbd0212d0f4906007bd715d35246`. Its matching library is
in the unpacked capture's `symfs`. Check this ID before symbol analysis.

In its 30.1-second CPU sample, navigation graph rollouts account for 52.3%
of process samples. The main thread uses 96% of one core. Its perception
collection accounts for 2.46% of process samples; repertoire reach accounts
for 2.07%. Inclusive costs overlap. These values do not measure GPU time.

Static contact candidates can share the cell's inverse maps and clipped
source area. Keep the same polygon solver, contact order, portal cells,
and numeric refusals. A local optimized-build measurement used 20,000
stationary queries against a 1,024-block grid, with varied body scales,
turns, and positions. Four runs changed from 234.6–235.6 ms to
183.2–187.9 ms. Each produced 267,685 contacts and displacement checksum
`10642793848333044673`. This is a dense-contact CPU measurement.

A perception snapshot can retain repertoire reach when its row still names
the same entity and Bevy reports no moveset change. Recompute after an edit
or row change. A local development-build measurement used 600 actors with
eight bound melee moves and 100 updates. Four runs changed from
218.2–236.2 ms to 21.0–22.0 ms. The reach sum stayed 197,700.
The behavioral test also checks removal and room order changes.

These measurements do not establish Android FPS. Capture the same route on
the phone after the changes. Keep the Hall's content and population intact.
Navigation graph construction and per-tick body integration remain measured
costs. Capture Sanic Act 3 separately with
`scripts/measure/autoprofile_android.sh sanic_act3` before assigning its FPS
drop to portals.

## November 25 capture

`android-all-ambition-auto-20261125T060553Z` recorded 577 admitted Hall roots.
Navigation rollouts account for 54.4% of the CPU samples and material axis
queries for 24.2%. These costs overlap. Autoplay's 27,715 Hall rollouts took
28.5 seconds from request to first movement. After construction, five-second
windows still had frame medians of 462–523 ms until the return to the hub.
The hub's first complete window had a median of 26.9 ms.

The profiles answer different questions. Use
`./scripts/measure/autoprofile_android.sh ambition` for the crossing and
`./scripts/measure/autoprofile_android.sh hall --warmup 40 --duration 45` for
steady play. The second route enters the Hall and stays at the entrance.
It removes the crossing request without changing the room or its actors.

The desktop integration run crossed the gap and returned to the hub with
two measured links and 12 rollouts. The full build of the same 93 surfaces
used 27,715 rollouts. Both directions used the same local graph.

A separate prototype streamed shared range and contact queries. In
360-update desktop development runs with idle input, it took 42.7–44.0 s.
Three baseline runs took 38.0–41.7 s. That general rewrite was removed.
A later matched desktop pair with the retained patch took 35.4 s before
and 33.5 s after. The median of the steady window medians went from
65.0 ms to 60.6 ms. This is one desktop pair. The navigation build result
does not establish steady Android frame time.

The stationary route also reached the Hall in a windowed desktop development
run with software rendering. It admitted 587 roots and stayed at the
entrance. After loading, the frame medians were 983–1031 ms with the census
enabled. The driver advanced 15 simulation ticks per rendered frame.
Body integration took 29–31 ms per tick and feature view publication took
5–6 ms per tick. Profile those costs separately from route preparation on
the phone. The desktop run used an older asset bundle.

## Stationary Android capture

`android-all-hall-auto-20261125T142408Z` admitted 594 Hall roots and stopped
autoplay at the entrance. The median of eight frame-census medians
between 44 s and 90 s was 550.4 ms. None of the 7,831 CPU
samples included navigation rollouts. Body integration accounted for 63.9%
of samples, including work on the compute pool. Material axis queries
accounted for 23.0%, repeated swept bounds for 8.3%, and affine cell
construction for 7.4%. These inclusive shares overlap.

The shared terrain index now retains the bounds used to build it. An
ordinary geometry edit invalidates both the bounds and the index.
The affine builder returns one full cell when the aperture search finds
no reachable links. It applies the same piece and scale limits first.

In 360-update headless desktop comparisons, the median share of measured
simulation time in body integration moved from 48–49% to 43–45%. Absolute
frame times varied between runs. They give no device FPS result.
Two 240-update comparisons with `perf stat` counted 238–244 billion
instructions before and 232–235 billion after. The two paired reductions
were 2.6% and 3.8%. Each run used the same six desktop CPU cores, room,
idle input, and census settings.

The next stationary Android capture, `android-all-hall-auto-20261125T150259Z`
at `cff0179a0d74`, had a median of 541.7 ms across nine frame windows
(499.2–559.5 ms). That is about 1.8 FPS. Of 7,838 CPU samples, 61.3%
included integration, 0.46% included perception collection, and 0.01%
included navigation advice. No sample included a navigation rollout.
Use a route with an active navigation request to measure route preparation.

## Desktop navigation demand sweep

The development `hall_bench` build at `bfb9e94fc` ran on CPUs 0–5.
Each run warmed for 220 updates and measured 240 updates. The roaming
runs used `AMBITION_ACTOR_BRAIN_OVERRIDE=ambition::townsfolk_roam`. Each
cap admitted the stated number of roots. The times are medians of
one-second windows from the headless rollback host.

| Hall roots | Stock ms/tick | Local roaming ms/tick | Approximate roaming ms/tick | Approximate segment checks |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 19.1 | 23.4 | 23.6 | 4,987,816 |
| 250 | 33.3 | 53.3 | 52.3 | 12,414,362 |
| 500 | 64.0 | 73.2 | 71.8 | 23,418,769 |

Stock runs prepared zero navigation maps, measured zero motion envelopes,
and prepared zero peer rows at the sample boundary. Local roaming built
96, 241, and 458 standing maps. It built no traversal graphs or motion
envelopes. Approximate roaming prepared 98, 243, and 464 graphs and ran
zero room-specific rollouts. All of its segment checks occurred during
warm-up. The 500-root run took 91.0 s through warm-up, compared with
33.9 s for local roaming. Both times include harness and room startup.
The stock Hall has fewer than 1,000 authored actors.

The local trip preference in `3bf42f47a` kept all three forced roaming
runs on standing geometry. The benchmark repeated the same caps and
update counts. Each run used zero envelopes, traversal graphs, and
segment checks.

| Hall roots | Time through warm-up before, s | Time through warm-up after, s | After, ms/tick |
| ---: | ---: | ---: | ---: |
| 100 | 29.3 | 20.3 | 29.4 |
| 250 | 56.1 | 21.5 | 39.0 |
| 500 | 91.0 | 27.7 | 64.8 |

These are single desktop runs. The work counters establish what each
run omitted. The timings include startup and are subject to load on
the host. The runs exercised the local destination preference. They
do not measure the change in cost per approximate trajectory.

The same headless harness also forced the shipped hub companion to
each traversal method in separate runs. Each method prepared one graph
and one envelope. The simulation-backed method ran 1,341 rollouts.
The approximate graph checked 359 support pairs and 4,632 clearance
segments with 587 solid-index queries and zero rollouts. Hybrid kept
the same estimated graph. The companion's goals in this short run did
not call for refinement. The graph fixture separately checks a live
jump and drop, a rejected jump, and cold and warm refinement.

At 100 Hall roots, the sighted `melee_brute_striker` sweep offered 101
peer rows per view, including the player. The spatial query left a mean
of 16.8 rows to examine and 15.8 visible rows in each sampled window.
This shows the input work after spatial selection. The separate
viewport test compares the visible order with a full scan.

The headless slow-frame test admitted 594 Hall roots. Autoplay crossed
the Hall and returned to the hub with a 200 ms frame interval. It used
two measured links and 12 rollouts in the Hall.

An Xvfb desktop run rendered the approach to the gap and the far side
of the Hall at 1280 × 800 with 100 roots. The seat arrived at its Hall
destination with two measured links and 12 rollouts. Its first hub
request still used the full graph fallback, with 6,352 rollouts.
Software rendering makes the render-time split unsuitable for
comparison with the Android capture.

## Cost of a live body at rest

`android-all-hall-auto-20261125T185355Z` at `48fd44c9cb78` admitted 595
Hall roots. The median of the eight window medians after 44 s is 526 ms
(1.9 FPS). The library in `symfs` matches the build ID recorded
in `perf.data` (`644fcaa42dc8d98a60e4f0d8c1b56f2d3b879b6a`). The capture
starts in the hub. Use seconds 12 to 29 of the CPU recording for the Hall.
This window contains 2,882 samples.

| Work in the Hall | Inclusive CPU sample share |
| --- | ---: |
| Body integration | 58.74% |
| Movement kernel | 51.35% |
| Material axis collision | 20.61% |
| Actor decisions | 3.12% |
| Contact facts for extensions | 2.81% |
| Group observations for extensions | 2.64% |
| Navigation advice | 0.03% |
| Navigation rollouts | 0.00% |

The shares overlap. They measure CPU work across threads. The capture has
no GPU frame time or count of simulation steps per displayed frame.

One desktop sweep used the existing development `hall_bench` binary on
CPUs 0–5. Each run warmed for 155 updates and measured 120 updates. The
median time was 11.8, 30.6, and 70.8 ms per tick at the 2, 150, and 595
root caps. Each run made zero navigation requests. At 595 roots, the
simulation phase census recorded about 30 ms in integration and 6 ms in
feature view publication per tick. The desktop sweep measures how work
grows with the cast. It gives no estimate of phone frame time.

1. Measure how many bodies stay on unchanged static support with no pending
   physical work. Count full steps, eligible bodies, and reasons a body
   needs a full step. Count committed simulation ticks and their wall time
   per displayed frame on the phone.
2. Probe the exact at-rest step in the shared movement kernel. Reuse the
   accepted support and motion facts while the inputs that support the body
   stay valid. Keep the body live for combat, dialogue, and nearby queries.
   New control or impulse, a changed frame or body, and changed collision
   inputs can require a full step on the same tick. Compare state and emitted
   facts with the full step. Include a support removal and a rewind in the
   comparison. Use the moving crowd to check the cost of the full step.
3. Measure again before selecting the next cut. Contact and group observers
   can select subscribers in the live room before they read the cast.
   Render view owners can retain rows when their source facts stay the same.
   Simulation consumers still need their facts at the declared tick boundary.

Use the full Hall on the phone for frame time. Keep the population caps in
the desktop scaling probe. At 20, 30, and 40 FPS the frame budgets are
50, 33.3, and 25 ms. Record the rate of simulation advance with each result.

### Settled movement results

The shared movement kernel can retain a result when the complete step
leaves the body state unchanged. It checks the next step's input and the
terrain that the original step queried. Moving mount terrain elsewhere in
the room can change each tick without making these bodies solve their
contacts again. A change in local terrain, control, or body state can
require a full step immediately.

For each desktop run, `hall_bench --warmup 140 --ticks 140` ran on CPUs
0–5 with `AMBITION_PROFILE_CENSUS=1`. `--full-steps` removed the derived
cache after warm-up. Both modes used the same binary and the full cast.

| Movement steps | Median ms/tick, two runs | Median integration ms/tick |
| --- | ---: | ---: |
| Solve each step | 71.54, 72.23 | 28.12, 28.93 |
| Reuse settled results | 48.36, 64.17 | 8.83, 12.08 |

The two reuse runs skipped 539 of 596 solves on their last tick. All four
runs recorded zero navigation requests. Host load varied across runs.
These measurements precede the fixed-control brain change. Android frame
time and simulation advance need a new capture. The frame census now logs
`sim_ticks` for each window.

### Rigged part allocation

A 1280 × 720 desktop run with software rendering admitted the full cast.
Before the renderer allocated parts on demand, the loaded Hall had 18,545
rigged part slots and 30,717 spawned entities. With allocation based on the
active views and readers, the same entry view had 13,115 spawned entities.
The captured frame showed Scout and the nearby exhibits. Use these counts
to compare entity work. Host load and software rendering varied between runs.

### LDtk layer selection

The Hall had 6,948 cells in its unpainted `Collision` layer. The LDtk
adapter spawned these cells even with its color overlay off. Presentation
now selects the adapter's layer exclusions from the loaded projects. If a
layer name has painted tiles or entities in any project, keep that name.
Simulation still reads the cells when it prepares the room geometry.

The same 1280 × 720 Hall entry view admitted all 595 roots and settled at
6,168 spawned entities, down from 13,115 after the rigged part change. The
captured frame showed the platforms, exhibits, and Scout on the entry floor.

### Frame observations

The frame reads combat geometry, boss cursors, attack art, and pooled shield
and launch views after the simulation steps. Pooled effects use the current
presented pose. Pose clocks and passage state still advance at each tick.

Two 1280 × 720 software-rendered runs used the full cast and 15 simulation
ticks per displayed frame. The median settled `FeatureViewSync` cost fell
from 4.58 to 2.58 ms per tick. The two settled frame windows had median frame
times of 529–541 ms before the change and 505–513 ms after it. This comparison
measures work in the desktop build. The Android rate needs a device capture.

### Combat target selection

Build the spatial index when a seeker can have a foe under the current
faction, team, or grudge rules. Use the same relationship rule to select
work and to evaluate a candidate. A new grudge or hostile faction can
request the index on the same tick. An actor with no foe still clears its
target and updates its combat standing.

In a third full Hall run, the targeting phase took 1.24 ms per settled tick
after this change. The frame observation build took 2.07 ms in that phase.
The third run had 541–548 ms median frame times in its two settled windows.
Other phases also took longer in this run. The phase time identifies less
targeting work. The frame time does not establish a gain from this change.

### Room collision inputs and rebound geometry

Prepare each live room's collision inputs before body steps start. Each
body uses its room's walls, gate grants, and barriers. The integration
phase no longer scans the cast to collect its room IDs before it steps
the cast. Sampled hurt regions reuse their storage. Ordinary pogo contacts
use the resolved hit. Only declared world surfaces project hurt regions
into the collision overlay.

In two full-cast desktop runs at 1280 × 720, the median integration phase
fell from 6.20 to 5.71 ms per fixed tick. Both runs advanced 15 ticks per
displayed frame in the settled windows. Frame medians were 502–528 ms
before and 512–524 ms after. The new build also contains the portal contact
change `526983e9c`. This comparison does not establish a frame-rate gain.

### Android baseline

The Android capture at `48fd44c9cb78` spent 57% of inclusive CPU
samples in the parallel body integration worker. The settled frame median
was 497–547 ms in the last six windows. That build had about 30,000
entities. The desktop runs with the later work retained about 6,160 entities.
The captures ran different builds on different hardware. Use a new phone
capture to measure the combined changes.

### Portal material queries

The portal publisher stores apertures and transit occupancy. Contact and
folded mesh consumers get the material map from the body they query.
In a desktop schedule probe with 596 bodies and no portals, the two
publication passes took 373 µs per tick before the change and 28 µs after it.
Each median covers seven batches of 300 ticks.

Two uncapped desktop Hall runs on each build used the headless host and
measured the settled windows. The median `WorldPrep.BeforeIntegrate` time
changed from 0.56–0.57 ms to 0.26–0.30 ms per tick. Other merged changes
were present in the comparison. The isolated schedule probe measures the
publisher change.

### Extension observations

Contact collection first finds bodies with a bound brain or status selector
and an active room. It copies body IDs and prepares material in the rooms
those receivers can observe. A desktop schedule probe with 596 bodies and
no matching receiver took 228 µs per tick before this change and 89 µs after
it. Both times are medians of seven batches of 300 ticks.

Group observation prepares the matching tags for each character form when
the catalog or admitted rules change. Each tick reads that mapping before
it gathers the live members and items of each group.

The headless desktop Hall admitted 596 bodies on both builds. The median
`WorldPrep.AfterIntegrate` time fell from 1.03 to 0.57 ms per tick in the
settled windows. `WorldPrep.Decision.Observe` stayed near 0.95 ms. The builds
also differ by merged portal and rendering changes.

### Material cast storage probe

The prototype stored the single cell of an unpartitioned cast inline. It
also stored up to four polygon points inline during contact and penetration
repair and computed static bounds at one endpoint.

An optimized desktop probe stepped 596 walking bodies on a floor with no
apertures. Allocations per body step fell from 52 to 23. The positions,
velocities, and contact counts gave the same digest on both builds. The
median tick times from three interleaved runs on each build were 3.17 ms
before and 3.15 ms with the prototype. The run-to-run ranges overlap.
The median headless Hall update times across four runs were 37–46 ms on the
existing build and 40–44 ms with the prototype. The prototype was removed
after this comparison.

A second probe let all 596 bodies come to rest. With settled reuse active,
all 596 reused their results and the complete kernel pass took 0.22–0.24 ms
per tick. With reuse disabled, it took 3.11–3.15 ms. Both modes produced the
same position, velocity, and contact-count digest. This probe covers only
the movement kernel. Measure the warm Hall outside that boundary before
changing the proof's state encoding.

### Combat geometry requests

Frame presentation shares a view of live strikes. The fallback strike art
reads the collision envelope and facing of its owner. F1 and headless
inspection sample hurt geometry for the bodies they request.

With 596 bodies, no live strikes, and no inspector, the median publication
time in an isolated desktop schedule fell from 1.49 to 0.018 ms per update.
Each median covers seven batches of 300 updates. In two full-cast headless
Hall runs on each development build, the median `FeatureViewSync` time
changed from 6.02–8.25 ms to 4.05–4.89 ms per tick. Use a phone capture to
measure the effect at the Android frame rate.

### Full-cast budget

At 60 simulation ticks per second, a 30 FPS frame needs two ticks on
average. If rendering and frame work take 5 ms, each tick has about 14 ms.
The earlier desktop run took about 32 ms per tick in the measured phases
and ran 15 ticks per displayed frame. Use elapsed time and committed ticks
to check the simulation rate. Collect the full-cast Android phase census
with `./scripts/measure/autoprofile_android.sh hall --warmup 40 --duration 45
--launch-env AMBITION_PROFILE_CENSUS=1`.

Select work by domain. A grounded body can retain an exact movement result.
Confirm the cost of its surrounding tick systems before tuning the cache
key. Contact and targeting should select the active sources before they
gather possible recipients in their live rooms. Body combat geometry should
be projected when an observer asks for it. Rendering live strikes has its
own active population. Each owner must accept changes on the tick that
applies them. Measure each boundary with the same full cast and include a
change that requires the full path. Use the shared execution plan in
[execution specialization and selection of active work](../../docs/planning/engine/execution-specialization-and-work-selection.md).
