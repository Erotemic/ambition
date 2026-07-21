# Portal camera continuity: a live RED, bisected and diagnosed (2026-07-21)

**Status: OPEN.** Two tests fail on `main`. This document is the diagnosis, not
a fix — the fix was attempted, got the primary assertion green, exposed a second
failure from the same root cause, and was reverted rather than landed half-done.

## The failure

```
cargo test -p ambition_app --test app_it -- portal_translation_camera_continuity
```

```
c135_to_c134_preserves_screen_position_and_keeps_falling   FAILED
c141_to_c140_preserves_screen_position_and_continues_right FAILED
thin_wall_walk_keeps_apparent_player_position_smooth       ok
```

```
body screen-space continuity: got (474.010, -595.792), expected (474.010,  76.125), delta (0.000, -671.917)
body screen-space continuity: got (593.916, -158.439), expected (829.416, -158.439), delta (-235.500, 0.000)
```

Deterministic, and each delta is ≈ the separation of the portal pair it crossed
(c141@2792 → c140@2552 is 240px; observed 235.5). This is not a tolerance flake:
**the camera does not follow the body through the portal**, so the subject's
screen position jumps by the whole portal separation at transit.

This is the `workspace (default features)` job in `./run_tests.sh`; the gate is
16/17 because of it.

## Attribution — NOT the K2a manifest work

The failure reproduces byte-identically (same two deltas) at `09838a679`, the
commit before K2a. Bisected in the worktree:

| commit | result |
|---|---|
| `8a545077b` FIX presentation-camera schedule handoff | **3 passed** |
| `bbeb68658` (parent of the suspect) | **3 passed** |
| `294d7c85c` FIX moving bodies shook against the camera; mint the frame-clock presented pose | **2 FAILED** |

**`294d7c85c` is the regressing commit.**

## Mechanism

`294d7c85c` made the camera follow the PRESENTED (frame-clock, extrapolated)
body pose instead of the tick pose — `camera_snapshot.rs`, in the live resolve:

```rust
if let Ok(presented) = presented.get(followed) {
    player_body.pos = presented.presented();
}
```
and that value becomes `ResolvedCameraSnapshot::follow_world`
(`camera_snapshot.rs:824`).

Meanwhile `apply_portal_camera_continuity` (`ambition_host/src/portal.rs`)
computes `body_screen_offset_world` from the AUTHORITATIVE `BodyKinematics`,
post-transit, and `camera_follow` (`ambition_render/src/rendering/camera.rs:159`)
combines the two:

```rust
snapshot.center_world = follow_world - screen_offset;
```

Instrumenting `camera_follow` on the anchor frame (the only frame where
`active_weight() == 1`) shows the two operands are in DIFFERENT frames of
reference:

```
weight=1 offset=Some((829.416, -158.439)) follow=(2790.9165, 264.0)
```

`follow = 2790.9` is the PRE-teleport position (the entry portal is at 2792);
the body is authoritatively at 2555.4. So the camera lands at
`2790.9 - 829.4 = 1961.5` instead of the correct `1726.0` — off by the portal
separation, exactly the observed error.

The continuity pass itself is correct and even self-reports the miss:

```
portal camera continuity constraint: kind=host_camera_recovery_gap
  desired_minus_host=(-240.0,0.0) ...
portal camera continuity start:
  body_before=(2795.4,264.0) body_after=(2555.4,264.0)
  prev_cam=(1966.0,422.4) desired_cam=(1726.0,422.4)
```

`desired_cam` is right. The host just never applies it.

**Why `thin_wall_walk` still passes:** its portal pair is nearly coincident, so
the pre/post-teleport mismatch is small enough to sit inside the 1.5px epsilon.
The passing test is not evidence the path works — it is evidence the error
scales with portal separation.

To see the logs yourself, the harness needs a subscriber — it builds on
`MinimalPlugins`, which has no `LogPlugin`, so `config.debug_log = true` alone
prints nothing:

```rust
app.add_plugins(bevy::log::LogPlugin::default());   // then set config.debug_log = true
```

## What was tried

1. **Wrong guess.** `portal.rs` clears the effect when
   `active_focus_transits.is_empty()`, AFTER the transit loop set the anchor —
   suspected the anchor was wiped in its own frame by a body fast enough to
   clear the aperture. Gating that clear changed nothing: the anchor does
   survive to `camera_follow` (`weight=1` is observed there). Reverted.

2. **Partial fix — correct, but incomplete.** `start_screen_anchor` already
   stores the exact absolute answer in `target_camera_world`, and
   `camera_follow` throws it away to recompute from the mismatched
   `follow_world`. Adding a one-frame `anchor_frame` marker to
   `PortalCameraContinuityState`, consumed by `camera_follow` to use
   `target_camera_world` verbatim on that frame, **made the primary continuity
   assertion pass**. The test then failed at the NEXT assertion:

   ```
   portal_translation_camera_continuity.rs:266
   screen-space motion should stay continuous through transit clear;
   max visible step 362.952px
   ```

   i.e. the anchor frame is now right and the RELEASE frame pops instead.
   Reverted — an unverified camera change in a subsystem whose own module doc
   says it is "under active portal-lab debugging" is worse than a red test that
   states the truth.

## The likely root fix

Both failures are the same defect seen at two moments: **the presented pose is
stale across a discontinuity**, so anything that mixes it with authoritative
positions disagrees by the size of the jump.

`presented_pose.rs` already has the right idea — `travelled_under_own_power()`
judges a move a teleport and `push(.., continuous: false)` collapses the history
so the jump is drawn as a jump. Verify why that guard does not produce a current
pose on the transit frame here. Prime suspect: `advance_presented_body_poses`
only pushes when `presented.tick != tick.0`, so if the sim tick that performed
the transit is not the tick observed on that render frame, the pose stays one
frame behind — and one frame behind a teleport is the entire teleport.

Fixing that would fix the anchor frame and the release frame together, and is
strictly better than patching either consumer. Patch #2 above is the fallback if
the root fix proves too invasive, but it needs the release phase solved too.

## Wider consequence

This is not only a test failure. The camera visibly lags through every portal
transit whose pair is more than a few pixels apart, and any other consumer that
mixes the presented pose with authoritative sim positions has the same class of
bug at any teleport (room change, respawn, possession swap).
