# Render target zeroes can defer dynamic meshes

A camera that clears its target does not need CPU clear pixels. Use an
uninitialized image with render-attachment usage. Keep the upload budget for
assets that have data to transfer.

In the portal lab, opening a 2048-square carry target consumed the full
16 MiB upload budget at impact. Bevy 0.19 removes an old GPU mesh before it
checks the budget for the replacement. Eight changed clip meshes were absent
on the first render at tick 88. The carry texture still contained the body;
the final view lost its clipped picture for that frame.

`0f64b41bf` uses `Image::new_uninit` for cleared carry targets. The clean GPU
capture retains the body and far foot at the same impact tick. The probe log
is `target/portal-lab-mesh-probe.log`; the film is under
`target/portal-visual-j/final/`. Compare equal simulation ticks. An image from
a later tick cannot prove that a transient render fault was fixed.


Reproduce the impact film after building `capture_scene`:

```bash
AMBITION_QUALITY_PROFILE=low target/debug/capture_scene portal_lab 2450,800 \
  target/portal-visual-j/recheck/f.png 1280x720 \
  --player-at 2304,872@60 --player-holds attack@61 --player-holds none@62 \
  --player-at 2324,872@70 --player-holds attack@75 --player-holds none@76 \
  --warmup 55 --frames 70 --stride 1 --tick-per-frame --camera-zoom debug \
  --portal-effect off --portal-convention reflection --portal-budget 4,32,12 \
  --art-snapshot
```


## Cached body images must wait for their GPU passes

An unchanged CPU pose does not prove that a cached image has rendered.
A new finishing quad can wait for its mesh, material or pipeline while the
source image already has pixels. If both cameras then rest, the finished
image can stay transparent indefinitely.

`9d6f9da4f` keeps the image active until its source images and its own GPU
passes are ready. It then restores normal caching. The GNU-ton readback has
pixels in both targets, with all six image cameras at rest at the inspected
tick. Compare source and finishing targets before changing the physical cut.
A source-target image alone cannot prove that the final picture is present.
