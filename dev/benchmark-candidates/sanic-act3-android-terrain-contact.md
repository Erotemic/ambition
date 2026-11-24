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
This measures elapsed test time. Measure the phone again for frame
times and GPU cost.
