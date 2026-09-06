---
id: camera-reference-frames
aliases: []
status: current
authority: system-behavior
last_verified: 2026-08-30
related_docs:
  - docs/architecture/spatial-model.md
  - docs/concepts/input-and-game-modes.md
  - docs/planning/engine/multiplayer-and-multiview.md
---

# Camera reference frames

Ambition supports two gameplay camera reference-frame policies:

- **world-fixed** — the observer remains aligned to the world;
- **subject-relative** — the observer follows the presented body's resolved
  reference frame so gravity/orientation changes can appear as world rotation.

The setting is exposed to the player as **Camera Frame**. World-fixed remains the
default.

## Authority boundary

Camera rotation is presentation policy. It does not mutate simulation geometry,
gravity, movement or aiming.

Input control has its own frame policy. A product may deliberately pair a
subject-relative camera with body-relative input, but camera code does not make
that gameplay decision by rotating authoritative input after the fact.

## Per-view state

The camera state used by presentation is already view-indexed:

- `LocalView` / `LocalViewId` identify local observer views;
- `CameraReferenceFrame` is carried by a local view;
- `CameraEaseState` and `ResolvedCameraSnapshot` are per-view state;
- presentation reads the resolved snapshot rather than a process-global camera
  authority.

The one-view game is therefore the first `LocalView`, not a separate camera
architecture.

## Composition

Camera framing/clamp/easing/portal continuity operate in the resolved observer
frame. Effects such as shake compose on presentation state and do not become
simulation truth.

A shared view whose subjects disagree about orientation must have an explicit
shared-view policy. It must not adopt participant zero's frame merely because
that participant was queried first.

## Remaining work

The remaining architecture is multiview/product work rather than a camera-frame
migration:

- independent split views may use different reference frames;
- adaptive shared/split policy needs explicit handling when subjects disagree on
  orientation;
- HUD/prompt/safe-area and other presentation state must follow the relevant
  local view where the product requires it.

Those items are owned by
`docs/planning/engine/multiplayer-and-multiview.md`.
