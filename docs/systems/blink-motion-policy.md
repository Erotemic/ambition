# Blink Motion Policy

Blink is a topological reposition, not a gravity-preserving dash. The engine now treats a successful blink as a single movement transition with three explicit pieces:

1. destination resolution (`blink_destination*`),
2. post-blink motion policy (`apply_post_blink_motion`),
3. event emission (`BlinkEvent`).

This keeps repeated blinks from inheriting runaway downward velocity. Earlier prototypes directly assigned `player.pos` and damped `player.vel` in the blink handler. That made it easy for quick blink, precision blink, and future blink variants to drift apart. The new helper `complete_blink(...)` is the single success path for all blink variants.

## Current policy

After a blink:

- horizontal velocity is damped, preserving some movement intent;
- downward velocity is clamped to a small tuning-controlled maximum;
- fast-fall and wall states are cleared;
- dash state is cancelled;
- a short `blink_grace_timer` suspends gravity while the blink reads visually.

This gives blink a controlled, intentional feel and prevents the specific class of bugs where a second blink appears to work but the player continues falling at pre-blink speed.

## Testing

The engine has regression coverage for this behavior:

- `repeated_blinks_clamp_downward_velocity_each_time`
- `post_blink_grace_suspends_gravity_for_tiny_window`

Future blink upgrades should route through `complete_blink(...)` unless they deliberately need different post-motion semantics. If they do, add an explicit `PostBlinkMotionPolicy` variant rather than bypassing the helper.
