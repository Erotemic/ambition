# Sanic Act 3 Android terrain contact

The capture `android-all-sanic_act3-auto-20261124T141124Z` uses
`5142f9337832`. Its frame windows have median times of 106 to 494 ms.
The symbolized CPU samples show 47.58% in body righting and 43.79%
inside its curved collision solver. These shares overlap. Render asset
preparation accounts for another 6.71% in the system report.

The spatial index selects a full terrain chain. A body near one edge
can send every edge of that chain to the continuous solver. Use each
edge's swept bounds to reject distant edges first. Keep the source order
and the continuous solver for each edge in reach. Keep closed solid
polygons as volumes.

The desktop dev build's headless Act 3 test completes two portal and
replay cycles in 58.45 s with the filter. The previous binary took
174.88 s. The test includes the roster, terrain, and authored portals.
This measures elapsed test time.

## Android follow-up

The capture `android-all-sanic_act3-auto-20261124T144946Z` uses
`989c54f15645` on the same Pixel 5. In complete census windows between
28 and 86.1 s, the average rate increases from 3.23 to 17.81 FPS. The
median frame times in these windows decrease from 106–494 ms to 51–66 ms.
The captures use different source revisions and advance through the act
at different rates.

Body righting accounts for 5.03% of CPU samples, and curved casts
account for 2.31%. The render thread accounts for 29.88% of CPU samples
and uses 54% of one core. The capture has no GPU timing.

The ring sheet first draws before the cover lifts. The measured gap
before the Act 3 music decreases from 9.77 to 1.43 s. Two decoder
starts report the same Ogg stream serial during activation. The
playback log polls on the host clock, so its first playing report
gives an upper bound on when the track started.
