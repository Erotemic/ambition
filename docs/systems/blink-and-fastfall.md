# Blink and Fast-Fall Notes

This is a draft mechanics note for the endgame sandbox.

## Blink

Blink is now treated as a first-class special movement verb.

There are two modes:

- **Quick blink**: tap/release the blink button to blink a short distance along the current input direction, or facing direction if neutral.
- **Precision blink**: hold the blink button past the short ~0.1s threshold to enter bullet-time, steer a fine-grained blink cursor, and release to blink to that target.

The precision blink target is controlled by accumulating the current input direction while the button is held. This is deliberately different from the first pass, where the destination was just a fixed-length ray. The new version should support much finer placement with keyboard controls.

## Bullet-time ramp

Precision blink should feel like the world is bending around the player, not like the simulation instantly flips into slow motion. The sandbox therefore ramps a time-scale value toward the blink-aim scale instead of jumping there immediately.

Current conceptual scales:

- normal: `1.0`
- early blink hold: about `0.08`
- precision aim bullet-time: about `0.10` (slow enough to aim, fast enough to remain readable after the two-clock fix)
- debug slow motion: about `0.25`

These are presentation-layer values in the Bevy sandbox. The engine still receives a timestep and remains testable without Bevy.

## Blink-through walls

The engine now has blink wall tiers:

- `BlinkWallTier::Soft`
- `BlinkWallTier::Hard`

Blink walls are solid to normal movement. A blink path may cross them only if the player has the matching blink-through ability. In the current sandbox, the player has both soft and hard blink-through upgrades, so every interior blink-wall can be crossed. The outer room boundaries and the central needle pillar remain normal Solid blocks, so blink cannot cross them. The destination must still be open space; the engine should never place the player inside a wall.

Normal `Solid` blocks remain absolute blink blockers.

## Fast-fall

Fast-fall is no longer triggered merely by holding down. That made down+attack/pogo feel bad. Instead, the Bevy input layer recognizes a **double-tap down** gesture and sends an explicit `fast_fall_pressed` event to the engine.

Once triggered, fast-fall persists until the player lands.

This keeps these inputs distinct:

- `Down + Attack` = pogo / downward attack intent
- `Down, Down` = fast-fall intent

## Bullet-time timestep bug fix

Precision blink bullet-time must use the same scaled timestep for the player as for visible time-reference objects.

A bug in the first moving-platform pass made the platform freeze while the player continued to fall quickly. The root cause was in the engine's combined player tick (now the kernel's `ae::step_motion` simulation phase): tiny positive timesteps were clamped upward to `1/240s`. That was safe for normal framerate spikes, but wrong for intentional near-zero bullet-time. The engine now only caps large timesteps and preserves tiny timesteps, so gravity and movement slow with the platform.

Regression coverage: `tiny_dt_preserves_bullet_time_scale` checks that a very small timestep produces proportionally small gravity instead of being rounded up to normal simulation speed.

Current tuning: quick-to-precision transition threshold is about `0.1s`.

## Real-time precision cursor

Precision blink aim now uses unscaled control time. The player, enemies,
particles, and moving platform can run in near-frozen game time while the blink
cursor still moves as a responsive input/UI control. This keeps bullet time
slow enough to read while avoiding a cursor that crawls across the screen.

When a hard blocker prevents the requested blink, the debug overlay now shows
both the raw desired cursor and the safe resolved destination. The resolved
magenta box is where the player will actually land; the red box/line indicates
where the requested target was blocked.


## Moving platforms and blink pathing

The sandbox moving platform is intentionally solid for normal movement, so the
player can collide with it and ride it. For blink pathing it is represented as a
soft blink wall rather than a hard solid. This means an upgraded blink can pass
through the moving platform, while the platform remains a real collision surface
for walking, jumping, and wall-style contact tests.

The debug blink preview must use the same temporary collision world as the actual
player update. Otherwise the preview and the release-time blink resolution can
disagree about sandbox-only geometry.
