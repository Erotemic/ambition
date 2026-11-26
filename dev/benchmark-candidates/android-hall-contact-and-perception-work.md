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

The later Pixel 5 capture `android-all-hall-auto-20261126T041153Z` used
`2ef7f0cc831f`, before the later axis comparisons and inline cast storage.
Across eight full Hall windows after second 95, the median of the frame
medians was 120.5 ms. The fastest window median was 108.8 ms. The game
advanced 59.0 simulation ticks per second and displayed 7.9 frames per
second across those windows. This is 7.5 simulation ticks per frame.

The median sum of the measured phases was 13.19 ms per tick. Integration
used 3.80 ms, combat 1.46 ms, the decision gate 1.30 ms, and feature views
1.21 ms. The native build ID `e652596b93c92d0e5b6af1563fe8eb2ff2aef1d4`
matches the library in `symfs`. Of 6,154 CPU samples, 8.56% included body
encoding for the settled movement check, and 7.34% included a full
movement solve. The capture measures the combined build changes.

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

### Combat participant selection

Strike resolution sorts victims when a strike can reach bodies. Ally rally
sorts the bodies with an engaged target or a grudge. It reads an eligible
ally's position from that body. Both systems keep the order of active rows.

In an isolated desktop schedule with 596 bodies and no strikes, the median
cost decreased from 110 to 23 µs per update. Each median covers nine batches
of 1,200 updates. Three uncapped Hall runs per build used the production
desktop host in headless mode. The run medians for `Combat` were 3.74–4.81
ms per tick before the change and 3.49–4.92 ms after it. The ranges overlap.

The windowed 1280 × 800 software-rendered run had 596 bodies and about
6,280 entities. Its median frame time was 460 ms, with 15 sim ticks per
frame and about 32.5 sim ticks per second. The sum of the measured sim
phases was 27.5 ms per tick. `WorldPrep.Integrate` used 5.9 ms, `Combat`
3.5 ms and `FeatureViewSync` 2.5 ms. The windowless host instead allocated
18,544 rigged part sprites. Its camera had no measured viewport and its
fallback requested parts for the full cast. Use the windowed count to
estimate presentation work on a device.

### Desktop trace and life observations

The all-body trace runs when a tool requests its buffer or automatic dumps.
The player trace still records for F8. NPC life plans build the room presence
lookup when an observation asks if a character is present. This lookup uses
the character identities from that tick.

Three full-cast headless runs with the local rollback host took 37.6–46.6 ms
per tick with the all-body recorder and 34.2–51.0 ms with it removed from the
schedule. These ranges overlap. All runs reused 539 of 596 movement steps
on the last tick.

A separate windowed CPU sample captured 7,160 settled samples from the game
and compute threads. The samples include time in child functions. Body-state
encoding for settled movement used 10.0% of the samples. Encoding the axis
movement parameters accounted for 4.4%. The all-body recorder used 2.5% and
the NPC life executor used 1.0%. The movement kernel probe above used an
optimized build. Use both build profiles when comparing changes to the reuse
check. Keep its exact state comparisons and rollback behavior.

Batching the axis parameter writes in a 596-body probe reduced median cost
from 4.55–4.73 to 4.17–4.65 ms per tick without optimization and from
0.23–0.26 to 0.19 ms with optimization. Both versions reused all steps and
produced the same state digest. The optimized saving was less than 0.1 ms
per tick. The batch is an experiment. Compare prepared movement parameters
before making a change to the codec.

### Contact storage and terrain candidates

When a body reuses its settled step, it shares the contact list. A change
to the returned list cannot change the retained result. Terrain queries read
the grid's ordered candidates directly. Position repair uses one contact
list and copies the boundary identities only when it reports a conflict.

An optimized desktop kernel probe stepped 596 bodies in each test. Reuse
went from four allocations per body step to one. Moving bodies used eight
fewer allocations per body step. With one long floor, median moving ticks
took 3.47–3.68 ms before and 2.95–3.05 ms after. With short floor blocks,
the medians overlapped at 3.37–3.83 ms before and 3.40–3.46 ms after. Each
pair produced the same digest of position, velocity, and contact count.

A separate fixed-law probe omitted the axis parameters from the reuse key
to measure the maximum saving from this change alone. The median optimized
kernel cost decreased by 0.08–0.12 ms per tick. The development build saved
1.6–2.1 ms per tick. The engine still checks these parameters on each reuse.
These measurements do not give the full-room cost or the Android frame rate.

### Exact axis law comparison

The settled movement check compares retained axis parameters in place.
The snapshot codec and the comparison use the same field declarations.
A change from positive to negative zero causes a new solve in the movement
test. All 12 snapshot fixtures match the previous encoding byte for byte.

The desktop kernel probe stepped 596 bodies on one floor. Each condition
ran twice in reverse order. Use the median thread CPU time from seven
32-tick batches in each run.

| Settled pass | Before, ms/tick | After, ms/tick |
| --- | ---: | ---: |
| Development (`opt-level=0`) | 8.44–9.35 | 6.05–6.49 |
| Optimized (`opt-level=3`) | 0.46–0.49 | 0.22–0.30 |

The development probe retired 22.73 billion instructions before and
14.40 billion after. Both versions reused all 596 steps and produced the
same digest. A walking probe reused zero steps and also matched the
digest. The optimized saving was 0.19–0.24 ms per tick for this workload.
The build volume did not have space for a new full Hall executable.
Measure the net desktop tick cost and phone frame time in the assembled
game before assigning a frame rate to this change.

The next change applies the same comparison to the axis maneuver state.
The retained axis policy stores both its parameters and its state. A change
to the sign of zero in a maneuver vector causes a new solve. Before and
after, 48 wire fixtures match byte for byte. The fixtures include ledge,
wire, and launch state.

With 596 settled bodies, median desktop thread CPU time was 2.69–2.71
ms per tick before and 1.95–1.96 ms after in the development build. In
the optimized build it was 0.148–0.149 ms before and 0.120–0.128 ms after.
All steps reused their proof in both builds. Position, velocity, and
contact-count digests match in the settled and walking probes. The
walking times overlap. These runs measure only the movement kernel.

The fixed-size body clusters also use comparisons from their snapshot
field declarations. Reuse still checks resource changes, live effects,
and signed zero. A comparison of 78 rollback fixtures found the same bytes.

In the 596-body development probe, the settled pass took 1.91–1.95 ms
per tick before and 1.32–1.33 ms after. In the optimized probe the
same pass took 0.118–0.121 ms before and 0.130–0.132 ms after. Both
versions reused all steps and matched the position, velocity, and contact
counts. The result shows a development-build gain and an optimized cost
of about 0.01 ms per tick in this workload.

### Inline collision cast storage

A whole material cell retains its bounds inline. Boxes and segments use
inline vertices in the shared cast and repair code. Partitioned cells and
larger polygons retain storage sized to their geometry. The admission
budget, contact order, and closest-point tie rule still apply.

The 596-body floor probe ran each build twice in reverse order. Both
versions matched the position, velocity, and contact-count digests.
The tests cover bounds, budget refusals, and portal clips. A separate
comparison matched contact records from 3,240 casts across shapes and frames.

| Desktop workload | Before, ms/tick | After, ms/tick |
| --- | ---: | ---: |
| Development, 57 moving and 539 reusing | 5.80–6.73 | 5.44 |
| Optimized, 57 moving and 539 reusing | 0.40–0.41 | 0.35–0.39 |
| Optimized, all moving | 2.84–2.94 | 2.45–2.46 |

Heap allocations per moving body step fell from 47 to 17. In the mixed
development run the instruction count fell from 18.39 to 17.60–17.61
billion. The two active populations show how the saving changes with
movement demand. The phone and full desktop room still need a new capture.

### Pose publication

The actor pose and appearance queries select the same bodies. They read
only the components each projection uses. The pose view reads the session
cast and tick duration once per pass. Overlay clocks update when a timer
or a dash edge needs them. The death look checks bodies that are out of
play or still have a death look to clear (`38af7a48b`).

Two full-cast development runs of the local rollback host had
`FeatureViewSync` medians of 2.48–2.77 ms per tick. With just the body pose,
actor pose, and passage projections removed in a probe, that phase took
0.68–0.71 ms. The same 539 bodies reused their movement step in all four
runs. The probe removes the pose output and only measures its cost. Keep
the pose and the clocks that track transitions when changing when the view
is published.

Four runs of the pose and appearance passes used one thread and the same
settled Hall with 596 bodies. Median thread CPU time was 2.88–2.90 ms per
update before the change and 2.40–2.61 ms after it. Individual windows
overlapped. The probe matched the pose counts, clips, position, facing,
and defense facts. The isolated result does not give a full tick speedup.

### Body control and move admission

The action scheme indexes each slot when the scheme is built or read from
data. The ordered actions and the index come from the same list. The persona
pass reads each body's cast generation and request before it fetches the
components it can change. The move admission pass checks for a proposal,
a playing move, or a capture throw latch before it reads combat rules.

| Isolated desktop pass for 596 bodies | Before, ms | After, ms |
| --- | ---: | ---: |
| Quiet move admission | 0.51–0.55 | 0.09 |
| Worn persona scan in the loaded Hall | 0.226–0.232 | 0.017–0.018 |
| Slot lookup pass in the development build | 0.119–0.120 | 0.046–0.048 |
| Slot lookup pass in the optimized build | 0.032–0.033 | 0.007 |

The persona probe confirmed that the pass consumed a request and refreshed
a stale generation. The slot probe matched the result digest. In a separate
full-cast run on the earlier build, removing the complete move trigger
reduced the median `Combat` phase from 3.96–4.64 ms to 3.52 ms. Full tick
timings overlapped. The complete removal is a cost probe. The engine still
accepts buffered moves, held follow-ups, movement cancels, and throws.

Control admission uses the slot index to clear ungranted verbs. It reads
declared techniques in canonical slot order to route edges and report
unsupported bindings. A probe on the older desktop host loaded 596 Hall
bodies. None had a technique binding. The isolated gate pass went from
0.758–0.766 to 0.417–0.425 ms, with the same control frames and edges.

A separate 596-body probe used the indexed action scheme. Resolver time
fell from 0.142–0.150 to 0.031–0.035 ms per pass in the development
build and from 0.016–0.022 to 0.009–0.014 ms per pass in the optimized
build. A comparison of 4,096 mixed schemes kept the same control frames,
edge order, and diagnostics. These tests measure the gate and its
resolver. They do not measure an assembled tick.

### Defense projection

The guard pass reads the full body data for guards that exist, bodies with
new or changed forms, and bodies that lose guard state. A new cast can add
guards to forms already in the room. The wallet shield pass checks live
room rules first and clears existing shields when no room grants the rule.
Only timed empowerment grants read the clock of a body's room.

| Isolated development pass in the loaded 596-body Hall | Before, ms | After, ms |
| --- | ---: | ---: |
| Guard volume projection | 0.085–0.122 | 0.027–0.034 |
| Wallet shield projection | 0.063–0.066 | 0.002 |
| Empowerment projection | 0.085–0.086 | 0.075 |

The Hall had one guard, no wallet shields, and no temporary empowerment
grants. The before and after passes gave the same output. The probe also
checked guard reconstruction, form changes, and cast replacement. Three
wallet shield tests covered the rule, leaving its rooms, and two rooms with
different rules. These runs used cached host libraries and current system
source. Full application and phone measurements remain open.

### Actor decision observations

The side contact pass reads the bodies in the contact snapshot. Crowd
observation reads full body data for living fighters and records deaths
without storing a row for every living body.

| Isolated desktop pass in the loaded Hall | Before, ms | After, ms |
| --- | ---: | ---: |
| Development, 595 actors and one contact box | 0.484–0.487 | 0.038–0.039 |
| Optimized, same actors and contact box | 0.026 | 0.005 |
| Development, with 24 hostile actors and contact boxes | 0.784–0.794 | 0.338–0.342 |
| Optimized, same hostile actors and contact boxes | 0.084 | 0.066–0.068 |

Both versions gave the same liveness, contact normals, nearest neighbours,
and crowd pressure. The second case set combat and contact state in the
loaded Hall to test those paths. The system test checks both body
populations, bystanders, and contacts. Two crowd tests check room scope and
order. The probe uses cached host libraries. It measures one system pass.

### Fixed actor decisions

An actor with fixed control checks for a complete body and a resolved
motion frame. It reads the room stamp from the decision query. The pass
fetches full actor body data when a brain evaluates world state or shares
perception with its crew. A fixed brain still clears old beliefs. Empty
memory can keep its last change tick when the actor stays in the room.

| Isolated decision pass in the loaded Hall | Before, ms | After, ms |
| --- | ---: | ---: |
| Development, 595 fixed brains | 0.848–0.867 | 0.340–0.352 |
| Optimized, 595 fixed brains | 0.122–0.126 | 0.042–0.044 |
| Development, 24 patrols and one module observer | 1.073–1.092 | 0.638–0.651 |
| Optimized, 24 patrols and one module observer | 0.212–0.215 | 0.140–0.146 |

Both versions produced the same ordered control frames, brain state, and
memory. The mixed case changed 24 of the same 595 brains to patrol, added
a module observer, and removed a resolved motion frame from one body.
The candidate cleared a seeded belief and kept the change tick of the
empty memory after the room stamp was set. The probe uses cached host
libraries and measures the decision pass.

### Hitlag views and rest rows

The body pose, feature view, and frame sampler share a hitlag strength
function. A zero hitstop timer gives zero strength without a room rule
lookup. The frame sampler calculates jitter when a reaction calls for it.
The feature view borrows the combat and shield state for both defense
fields. The rest-row pass indexes the authored rows when the catalog
changes. The tick lookup searches only entries that declare a rest row.

| Isolated pass in the loaded 596-body Hall | Development before → after, ms | Optimized before → after, ms |
| --- | ---: | ---: |
| Body pose per tick | 0.776–0.777 → 0.600–0.630 | 0.261–0.262 → 0.125–0.126 |
| Rest rows per tick | 0.133 → 0.045–0.046 | 0.119–0.120 → 0.009 |
| Feature view per frame | 0.409–0.410 → 0.246–0.255 | 0.161–0.163 → 0.067–0.073 |
| Presented pose per frame with a new tick | 0.175–0.178 → 0.066–0.067 | 0.106 → 0.014 |

The probe measured thread CPU time with cached host libraries. Five of
636 catalog entries declared rest rows. The pose, feature, and frame
samples matched in the quiet Hall and with affine hitlag, including a
paralyzed body and an attacker in hitstop. Three rest-row tests passed.
The cases include catalog replacement and a separate pin writer.

### Character presentation lookups

The pose and appearance passes read the character and cast generation on
the body's geometry grant. A body without a grant checks the cast for a
definition. The audio attribution pass resolves a source when the body
changes character, its source changes or is absent, or the active cast
changes. It compares retained cast versions so a replacement with the
same generation number also refreshes the source.

| Isolated pass in the loaded 596-body Hall | Development before → after, ms | Optimized before → after, ms |
| --- | ---: | ---: |
| Body pose per tick | 0.643–0.665 → 0.535–0.548 | 0.127–0.128 → 0.069–0.070 |
| Actor appearance per frame | 0.185–0.188 → 0.104 | 0.066 → 0.018 |
| Body audio attribution per tick | 0.070–0.071 → 0.039 | 0.046–0.047 → 0.002 |

The probe used cached host libraries and measured thread CPU time. Pose
and appearance agreed when a grant was missing and after a character
change. A focused system test changed the worn character and the active
cast, restored the published cast, and removed the source and the form.
These measurements cover individual passes. The assembled desktop tick
and Android frame rate need a new capture.

### Movement support contacts

The movement result carries the support role and the surface normal.
Righting reads the selected contact from the same list that the result
publishes. A settled step retains the shared contacts. It builds the empty
event fields when a body reuses the step.

In a desktop kernel probe with 596 bodies, each body step used one fewer
allocation. The run with 539 settled bodies and 57 moving bodies used
2.54 allocations per body step before the change and 1.54 after it. All
three workloads matched the positions, velocities, contact counts, and
support point and normal digests. The core test run passed 941 tests,
including reuse after an edit to the returned contact list and after
floor removal.

The optimized kernel medians for the mixed workload overlapped
(0.364–0.396 ms before, 0.354–0.385 ms after). The workload measures one
thread and excludes the desktop host, render, and ECS systems.

### Desktop Hall after the movement and presentation changes

The development build at `34bdc51a25a0` passed the assembled app check.
Two windowed runs used the same Intel i9-11900K VM, llvmpipe Vulkan,
Potato quality, and a 1280 × 800 viewport. Each run started in
`hall_of_characters` with a fresh configuration and no population cap.

```sh
cargo build -p ambition_app --bin ambition_game_bin -j 4
AMBITION_PROFILE_CENSUS=1 xvfb-run -a -s '-screen 0 1280x800x24 -nolisten tcp' \
    target/debug/ambition_game_bin --start-room hall_of_characters
```

The sum of measured simulation phases had medians of 20.52 and 20.48
ms/tick for the census window from 30 to 55 seconds. The earlier desktop
receipt recorded 27.5 ms/tick. Integration used 4.2 ms/tick, combat 2.4 ms,
player simulation 2.1 ms, and feature view sync 2.0 ms. The headless
population check on the same build found 596 movement bodies. Of these,
539 reused their settled movement step on the last tick. The desktop
target is 13.75 ms/tick.

### Terrain motion and cast preparation

Each terrain base and overlay records whether it has moving entries. A
zero-axis step can skip the swept range search for terrain that has no
motion. It still records the queried region so a new moving entry can
invalidate a settled step. A whole material cell maps each corner once
when source travel is zero. The two translations give the swept bounds.
Settled frames compare the same float bits with direct field reads.

The windowed development comparison used the saved `34bdc51a25a0`
executable and a new native build in reference, candidate, candidate,
reference order. All four runs used the same viewport and quality as the
previous desktop measurement. The census reported 596 bodies in each run.

| Median over census windows after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 20.07, 20.02 | 19.42, 19.38 |
| World preparation and movement integration | 4.20, 4.28 | 3.79, 3.89 |

The 942 core tests passed. The movement probe matched positions,
velocities, contact counts, and support point and normal digests with
settled, mixed, and moving bodies.

### Room queries for pending work

Conversation selection checks reach and facing before it reads a candidate's
live room. An ambient bark clock stores its timer and line rotation together
and reads room metadata when it starts or comes due. Death beats read room
pace when they advance an active death clock.

The windowed comparison used the terrain preparation build as the reference.
The four runs used 596 bodies and the same viewport, quality, and run order.

| Median after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 20.59, 19.52 | 20.39, 19.64 |
| Progression | 0.84, 0.79 | 0.54, 0.51 |
| World preparation | 0.60, 0.56 | 0.45, 0.44 |

The total tick results overlap. Focused checks used the built libraries
to verify talk reach across rooms, death and revival under two room clocks,
and bark timing and rotation after a catalog replacement.

### Shared room pace and pending combat

Movement and reaction passes check for a local room pace once per pass.
Shared-pace bodies need no room lookup. Landing recovery, special turns,
and buffered actions read room rules or clocks when those inputs apply.
The placement pass checks for a custody or placement record before it
resolves an authored body's room and feet.

An isolated development probe read 596 clocks in one room. The batch
reader took about 4 µs per pass with the shared clock. Individual reads
took 99–101 µs. Both methods used the local reading path during a hold.
Focused checks covered a held room, rooms at different paces, a room
hidden for replay, a buffered grab, special turns, and landing recovery.

The four windowed runs used 596 bodies and the same quality, viewport, and
run order as the previous desktop comparison.

| Median after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 19.16, 20.70 | 19.34, 20.25 |
| Combat | 2.36, 2.56 | 2.23, 2.34 |
| Movement integration | 3.85, 3.96 | 3.82, 4.02 |

The combat reduction was consistent across the four runs. Total time and
integration varied across the runs.

Two consecutive headless observations of the settled Hall found the same
kinematics, control, motion, ground, combat, animation, damageable volumes,
and body bounds for all 596 bodies. Every one of these components received
a new change tick. Check the owners of these writes before using change
detection to retain derived views. Keep explicit samples that identify a
simulation tick.

### Body geometry and weapon readiness

Damageable body boxes keep the published shape when its bounds and axes
match. Movement feedback reads an arrival or action before it builds
effects. The contact pass collects portal geometry when a body straddles
a pair. The weapon readiness pass counts shots once per owner for all
readers in that pass.

Isolated checks on the loaded 596-body Hall used the same simulation
state for each comparison. The damageable-volume pass kept 592 change
ticks and published the same shapes. The probe checked a resize, an
authored shape and facing change, and death and revival. The readiness
probe matched all results with zero shots and with 400 test shots.

| Isolated pass | Reference, ms | Candidate, ms |
| --- | ---: | ---: |
| Damageable volumes | 0.111–0.121 | 0.070–0.077 |
| Weapon readiness, no shots | 0.063–0.069 | 0.042 |
| Weapon readiness, 400 shots | 2.257–2.314 | 0.102–0.120 |

The assembled desktop development build passed. The four windowed runs
with the same 596 bodies had total medians of 18.47 and 18.96 ms/tick
for the reference and 19.63 and 19.15 ms/tick for the candidate. Their
windows overlapped. This comparison established no total tick reduction.
Three focused checks matched feedback and physical contacts, including a
straddler with contact geometry at the far end of its portal.

### Stationary flight

The movement solver can retain an exact result for a stationary flyer.
The reuse check still compares the body state, control, forces, and
terrain. After a step with transient events, the full solver waits for a
quiet result before it records terrain reads again. A movement test
compares against the full solve during repeated contact events and checks
that new terrain and input require a new solve.

An isolated integration pass used the loaded Hall on six compute workers.
Four idle runs with the default batches took 3.04–3.43 ms per pass.
Making the participant pass serial took 3.29–5.23 ms. Increasing actor
batches to at least 128 bodies took 3.73–4.03 ms with that serial pass.
With velocity given to each body, the default batches took 11.80–13.63 ms.
Batches of at least 128 took 12.34–15.87 ms. The engine keeps the default
batching.

An optimized kernel probe used 38 flyers and 558 grounded bodies.
Median times were 0.265–0.277 ms before and 0.237–0.238 ms after.
Both versions had the same measured allocations per step and the same
position, velocity, and contact digests. Moving-body times overlapped.

The loaded Hall had 38 motionless flyers. Ten reused the full step after
the change. The other 28 still needed full solves. Grounded reuse stayed
at 539. The windowed desktop development comparison used 596 bodies, a
1280 × 800 viewport, and the same CPU renderer and quality. The four runs
used reference, candidate, candidate, reference order.

| Median after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 18.62, 18.86 | 18.18, 19.10 |
| Movement integration | 3.89, 3.84 | 3.51, 3.54 |

Total tick windows overlapped. The Android capture for this change is
pending.

### Parallel body pose publication

In 18,826 CPU samples from the windowed Hall, the two pose builders had
the largest named system counts on the main thread. The body pose pass
now updates existing poses in parallel for groups of at least 256 bodies
when compute workers are available. First publication inserts the new
components in query order. Both paths use the same pose calculation and
animation clocks.

A six-worker probe compared all 596 poses through first publication,
movement, hitlag, and a late arrival. The pose records matched. The
development pass fell from 0.483–0.505 ms to 0.163–0.207 ms. With the
pose crate optimized, it fell from 0.077–0.092 ms to 0.050–0.057 ms.
The optimized 64-body pass took longer with workers. The 192-body
results overlapped. At 256 bodies the measured pass took 0.036–0.038 ms
with serial updates and 0.026–0.033 ms with workers.
The assembled pose system also matched the serial reference with 400
bodies in two rooms with distinct hitlag laws, with and without a pool.

Four windowed desktop runs used the same 596 bodies, viewport, and
quality in reference, candidate, candidate, reference order.

| Median after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 18.09, 19.05 | 18.92, 18.73 |
| Feature view publication | 1.88, 1.97 | 1.76, 1.76 |

Total tick windows overlapped. Device performance still needs a capture.

### NPC rally and bark clocks

Rally selects actors with a target or a body grudge before it reads their
combat state. It checks for prior rallies even after targets clear so
allies can return to their usual behavior. New rallies check for active
fights in dangerous live rooms. Bark clocks use direct room stamps and
hash lookups. Emission keeps the query order.

Five isolated desktop comparisons used the loaded Hall cast. Each row
shows the range of run times per pass.

| System and fixture | Reference, ms | Candidate, ms |
| --- | ---: | ---: |
| Rally with no fight | 0.183–0.197 | 0.021–0.022 |
| Rally with one active foe and prior rally | 0.324–0.340 | 0.163–0.171 |
| Ambient bark clocks | 0.194–0.213 | 0.120–0.134 |

The rally commands and ordered bark messages matched. A separate test
used two dangerous live rooms. A fight with only a body grudge drew in
an ally in its own room. The other room's ally stayed peaceful. The three
NPC combat integration cases passed with the desktop application's libraries.

The next four windowed runs used the same 596 bodies, viewport, and
quality in reference, candidate, candidate, reference order.

| Median after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 22.03, 18.39 | 22.43, 18.62 |
| Gameplay effects, including rally | 1.37, 1.12 | 1.11, 0.98 |
| World preparation, including barks | 0.53, 0.42 | 0.42, 0.36 |

Host load varied between runs. These runs show lower phase costs and
overlapping total tick windows. They give no phone FPS result.

### Grounding before step retention

The movement kernel applies the caller's grounding policy before it checks
for an unchanged step. A caller can keep a flyer airborne for locomotion
and retain its physical contacts and support normal. A caller can also
set the air-jump count for a grounded body. Retention compares the policy
and the published state as well as the other step inputs.

The full Hall ran in the desktop development build with the same 596
bodies, 1280 × 800 viewport, Potato quality, and CPU renderer. The runs
used reference, candidate, candidate, reference order.

| Median after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 18.35, 18.31 | 16.90, 17.00 |
| Movement integration | 3.51, 3.56 | 2.15, 2.19 |

After 165 headless updates, 589 of 596 bodies reused their step, up from
549. The 40 new bodies were 28 flyers with physical floor contact and 12
grounded actors with a jump refill. The 46 NPCs that solved a step in the
reference build had the same positions, velocities, and ordered movement
events in both builds.

An optimized kernel fixture used 38 flyers and 558 grounded bodies, with
40 extra solves in the reference arm. Median cost across two runs fell
from 0.323–0.344 ms to 0.122–0.130 ms per tick. The movement and contact
digests matched. Both arms solved every step when all bodies moved and
their times overlapped. This test measures no phone frame time.

All 943 active core tests passed. The flight test keeps contact with a floor
then removes it, adds a wall, and applies steering. The grounded test
compares the air-jump refresh with a full step and changes the policy on a
settled body.

Reusing the validation of body material axes had no repeatable effect in
the development fixture with 57 moving and 539 settled bodies. The
prototype stayed out of the build measured above.

### Settled comparisons and prepared rooms

The settled step checks the same snapshot fields and floating-point bits.
The exact comparisons inline in the development build. A material record
copies its five bit values directly. Movement reads room and session stamps
from the body query. The prepared room can return shared walls when it has
no body-specific gates or barriers and no body has a solids exemption.

In a development kernel fixture with 596 settled bodies and four additions
to the terrain each tick, median times fell from 0.854–0.885 ms to
0.670–0.678 ms. The state and contact digests matched. In the all-moving
case the two builds each retired about 45.6 billion instructions. Optimized
kernel results overlapped. A combined bounds check on additions had no
repeatable gain and stayed out of the patch.

Four windowed desktop runs kept all 596 bodies and the same viewport,
quality, and CPU renderer. The run order was reference, candidate,
candidate, reference.

| Median after 30 s | Reference runs, ms/tick | Candidate runs, ms/tick |
| --- | ---: | ---: |
| All measured simulation phases | 16.94, 16.69 | 16.46, 16.24 |
| Movement integration | 2.19, 2.18 | 1.99, 1.92 |

The application build, 943 active core tests, and 17 collision tests
passed. The room test checks per-body collision exceptions across two
live rooms, including an exemption added after the first step. Positions,
velocities, and ordered movement events matched for all 595 Hall NPCs
after 165 headless updates. The live Smash gate test also passed.

### Full-cast budget

At 60 simulation ticks per second, a 30 FPS frame needs two ticks on
average. If rendering and frame work take 5 ms, each tick has about 14 ms.
The earlier desktop run took about 32 ms per tick in the measured phases
and ran 15 ticks per displayed frame. Use elapsed time and committed ticks
to check the simulation rate. With 5 ms of frame work, 20, 30 and 40 FPS
give the simulation 15.0, 14.2 and 13.3 ms per tick. The old 526 ms phone
capture would imply about 35 ms per tick if it also ran 15 ticks per frame
and spent 5 ms outside those ticks. The capture did not count its ticks.
Collect the full-cast Android phase census
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
