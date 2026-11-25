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
