# Shared part cuts under linear filtering

## Contract

A stored half can be drawn twice. A repeated strip can use one raster.
Filtering must keep the joined body continuous. Keep raster sharing and test
what the filter reads at the join.

## Two causes

A linear texture sample reads RGB before alpha blending. Black RGB under
alpha zero can darken the opaque part below a cut. Extend edge colour into
transparent texels inside the frame and its own gutter. Keep alpha and each
covered RGBA texel unchanged. Do this after packing, including an incremental
patch. A logical source frame normalizes hidden RGB before it is packed again.

Two opaque tiles that meet without overlap have another failure. At the join,
each filtered edge can have alpha 0.5. Normal alpha blending gives
`0.5 + 0.5 * (1 - 0.5) = 0.75`, not full coverage. A crop needs source context
across its internal cuts. A repeated strip and its context must stay inside a
region whose rows have the same pixels. Offset the pivot by the crop margin.
Do not replace the shared rasters with whole-body frames to hide a join.

## Evidence

2026-10-28, renderer commit `3243b40`: 39 publisher tests pass. Disabling the
sampling step makes the filtered-half witness fail on its colour assertion.
Alpha and covered-colour preservation and full/incremental pack byte equality
are checked through the real packer.

A controlled bake uses the same authoring revision on both sides. The packing
step keeps the robot and Behemoth part tables and alpha masks unchanged. Older
installed assets had different robot poses and were not a valid control.

The Behemoth crop change keeps 29 rasters, 520 draw records and the same actor
metadata. Its part atlas grows from 43,860 to 53,248 texels. The real engine
captures show a continuous robot door pose and Behemoth idle body. The Hall
body also stays continuous at Low quality. These are software-GPU captures.
