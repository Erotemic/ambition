# Local geometry queries in the movement kernel

A body should pay for the geometry its query can reach. A full scan of room
blocks for every hazard or rebound check makes unrelated geometry increase
movement and navigation cost.

Use the existing spatial index for endpoint queries. For a swept hazard, use
the same full-path candidate query as solid sweeps. An endpoint box cannot
replace the travelled path. Keep the exact contact predicate after broad phase:
strict endpoint overlap, the hazard interior and grazing rules, and rebound
source priority are separate parts of the contact contract.

Compare indexed results with a visible ordered scan after layers hide and add
geometry. Include fast traversal of thin hazards and overlapping rebound
surfaces. Test through the common movement kernel as well as query functions.

A material cast's source polygon is fixed. Contact candidates can share it;
clipping needs owned storage only when it changes that polygon. An unclipped
cell has exactly eight space-time bound vertices and can keep them on the
stack. Preserve projection order and outward rounding.

Measure small-world controls as well as large populations. Index overhead can
cost more than a short scan. Do not infer a frame-time gain from a kernel
benchmark, or select a new grid representation from asymptotic cost alone.
