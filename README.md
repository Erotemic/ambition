Portal camera continuity v10 compile fix overlay.

Apply this on top of portal_camera_continuity_v10_diagnostics_overlay_20260629.

Fixes:
- Removes `Copy` from `PortalCameraContinuityHostView` now that diagnostics store `Option<String>`.
