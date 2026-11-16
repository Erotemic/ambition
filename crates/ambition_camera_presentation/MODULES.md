# `ambition_camera_presentation` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_camera_presentation** — The camera's live presentation state and the systems that drive it: the eased scale, target and roll of each view (`CameraEaseState`), the shake (`CameraShakeState`) and the finishing zoom (`FinishZoomState`).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|

_0 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

The crate is one file. Its state is presentation and never rolls back: the
runtime's engine group installs the request appliers and the session-activation
rest beside the resources, and the windowed host ticks the shake and the zoom
ahead of `camera_follow`. The simulation's half (the requests, the tunings, the
amplitude laws and `PlayerBlinkCameraState`) is
`ambition_platformer2d_shared_tangle::camera_ease`; nothing simulated writes
the state here.
