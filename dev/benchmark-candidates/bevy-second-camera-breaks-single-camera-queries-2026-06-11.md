# Benchmark candidate: a new render-to-texture camera silently breaks `With<Camera2d>` queries that assume one camera

Date: 2026-06-11

Tags: `bevy-camera`, `bevy-render-to-texture`, `ecs-query-assumptions`,
`symptom-disambiguation`, `game-architecture`

## Prompt

You are adding "view windows" to a 2D Bevy game (`bevy 0.18`): each portal
shows a live slice of the world in front of its linked partner. The
implementation spawns, per portal, an **offscreen capture camera** that renders
a fixed world region to a texture, which a mesh then displays:

```rust
commands.spawn((
    Camera2d,
    Camera { order: -8, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
    Msaa::Off,                                   // already present
    RenderTarget::Image(ImageRenderTarget::from(image)),
    Projection::Orthographic(OrthographicProjection {
        // Frame EXACTLY the exit-side source rect, independent of the player.
        scaling_mode: ScalingMode::Fixed { width: source_size.x, height: source_size.y },
        ..OrthographicProjection::default_2d()
    }),
    Transform::from_translation(exit_zone_center),
));
```

The main game camera is the usual single `Camera2d` (no `RenderLayers`), and a
follow system pans/zooms it toward the player:

```rust
pub fn camera_follow(
    player: Query<(&Transform, &PlayerBaseSize, &BlinkCam), PrimaryPlayerOnly>,
    // "ignore the #31 cube pause-menu Camera3d"
    mut query: Query<(&mut Transform, &mut Projection), (With<Camera2d>, Without<PlayerVisual>)>,
    // ...zoom inputs...
) {
    // ...compute follow position (x, y) and orthographic_scale...
    for (mut transform, mut projection) in &mut query {
        if let Projection::Orthographic(o) = &mut *projection {
            o.scale = orthographic_scale;
        }
        transform.translation.x = x + shake.x;
        transform.translation.y = y + shake.y;
    }
}
```

Observed behavior once the capture cameras exist: the portal windows DO render
content (not blank), but **what each window shows tracks the main camera — it
pans and zooms with your view, instead of staying locked to a fixed slice of
the partner's exit.** Zoom in and the window's contents zoom with you.

Two RTT gotchas are on the table:

1. The offscreen target is single-sampled, but a camera defaults to
   `Msaa::Sample4`; a 4×-MSAA camera renders nothing into a 1× target.
2. Something else.

Which one explains THIS symptom, and what is the minimal fix? Be specific about
why the symptom rules one of them out.

## Reference answer

It is **(2)**, and the symptom rules out (1). The MSAA mismatch produces a
**blank/transparent** target (the camera renders nothing), so its signature is
"the window shows nothing / you see through it." Here the window renders real
content that is *scale- and pan-dependent on the main camera* — so the capture
IS rendering; its **projection and transform are being mutated** every frame.

Root cause: `camera_follow`'s query `With<Camera2d>` now matches the new capture
cameras too (they are `Camera2d`). The loop `for (..) in &mut query` therefore
overwrites every capture's `Transform.translation` (→ player position) and its
`Projection`'s `scale` (→ the main camera's `orthographic_scale`) each frame.
The capture's `ScalingMode::Fixed` is multiplied by that hijacked `scale`, so
the captured area zooms with the main view and re-centers on the player — never
framing the intended fixed exit rect. The `With<Camera2d>` filter was written to
exclude only the pause-menu `Camera3d`; it never anticipated a second 2D camera.

Minimal fix: scope the mutating system to the actual main game camera, not "any
2D camera." Use a dedicated marker that only the main camera carries:

```rust
mut query: Query<
    (&mut Transform, &mut Projection),
    (With<MainCamera>, Without<PlayerVisual>),   // was: With<Camera2d>
>,
```

The same over-match silently affects every other `With<Camera2d>` system in the
project — e.g. background/foreground parallax that do `camera.single()` and now
get `QuerySingleError` (many `Camera2d`) and bail, so parallax quietly stops
following. Audit all of them and pin each to `MainCamera`. (The pre-existing
front-HUD `Camera2d` was already a latent second match; the capture cameras just
made the bug visible.)

Validation: `cargo check -p ambition_app` compiles; in-game the portal window
stays locked to its exit slice while you pan/zoom, and parallax follows again. A
cheap regression guard is a test asserting exactly one entity carries
`MainCamera` while several carry `Camera2d`.

## Why this matters as a benchmark

- **Symptom disambiguation, not gotcha recall.** The well-known RTT failure is
  the MSAA mismatch, and a model pattern-matching "render to texture + Bevy"
  will reach for it. But the stated symptom (content present, scales with the
  main camera) is the signature of a *mutated projection*, not a blank target.
  The candidate must reason from the precise symptom to the cause and explicitly
  reject the famous one. Models that grab MSAA "fix" a bug that isn't there.
- **Invariant:** adding a second camera of an existing type retroactively
  invalidates every `With<CameraType>` query that assumed a single instance —
  most dangerously the *mutating* ones, which corrupt the new camera rather than
  just reading the wrong entity. A marker component (`MainCamera`) is the
  durable fix; `With<Camera2d>` is a count-dependent filter masquerading as a
  type filter.
- **Project-specific:** the comment on the query (`"ignore the cube Camera3d"`)
  actively misleads toward "the filter is already correct." The model must see
  that excluding a 3D camera says nothing about a second 2D one.
- **Checkable:** the fix is a query-filter change plus an audit; the symptom→cause
  mapping is gradeable from prose, and the single-`MainCamera` invariant is
  testable.
