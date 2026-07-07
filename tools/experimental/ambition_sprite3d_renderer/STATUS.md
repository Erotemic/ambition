# Status: experimental / reference

This is reference / experimental only.

This was a Blender-first character sprite experiment intended to render
3D meshes down to 2D character spritesheets. It is **not** the current
production sprite path. The production path is the procedural Pillow-based
[`tools/ambition_sprite2d_renderer`](../../ambition_sprite2d_renderer/README.md).

Do not install runtime assets from this tool unless it is explicitly
revived. Generated outputs (`generated/`, `assets/`) are gitignored and
must not be committed.

## Why it lives here

- Blender as a hard dependency makes CI/runtime reproduction painful.
- The 2.5D character look the project wants is achievable procedurally.
- Keeping the code as a reference avoids losing the rig/render setup
  experiments if we need them later.

## If you revive it

1. Move it back out of `tools/experimental/` and rename to the
   production layout (`tools/ambition_sprite3d_renderer`).
2. Update `tools/README.md` and the renderer conventions section.
3. Add explicit `install` / `render-publish` modes that copy into
   `crates/ambition_actors/assets/...` in lockstep with the 2D renderer.
