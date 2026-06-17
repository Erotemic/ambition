# Movement edge-touch y-sweep benchmark candidate — 2026-05-11

This additive candidate follows the movement split repair. Keep it separate from
shared benchmark files so the exact failure can be reviewed independently.

## Q: Reject edge-touch side contacts before vertical shape casts

### Level A prompt

A platformer engine represents the player and world blocks as AABBs. Movement is
axis-separated: `sweep_player_x` handles horizontal motion and
`sweep_player_y` handles vertical motion. The engine already has this helper:

```rust
fn body_is_side_contact(body: Aabb, block: Aabb) -> bool {
    const Y_NESTED_EPS: f32 = 1.0e-4;
    body.top() >= block.top() - Y_NESTED_EPS && body.bottom() <= block.bottom() + Y_NESTED_EPS
}
```

The y-sweep predicate rejects blocks the body already strictly intersects:

```rust
if start_body.strict_intersects(block.aabb) {
    return false;
}
```

A full-world wall-cling repro starts with the player exactly touching a tall
wall on x, with the player's y-range fully nested inside the wall's y-range.
The body is not strictly intersecting because the boxes are exact-edge-touching,
but Parry reports a `TOI=0` vertical sweep hit. `sweep_player_y` then treats the
hit as a ceiling/floor collision and snaps the body to the wall's bottom edge,
moving the player by about 215 px in one frame:

```text
FULL WORLD step: pos=(62, 1783) vel=(0, 0) on_ground=false on_wall=true cling=true
dy=215.08752 (initial y=1567.9125, after y=1783)
```

Where should the guard go, and why is `strict_intersects` insufficient?

### Expected answer

The y-sweep predicate should reject non-one-way side contacts before calling the
sweep query or accepting its result:

```rust
if body_is_side_contact(start_body, block.aabb) {
    return false;
}
if start_body.strict_intersects(block.aabb) {
    return false;
}
```

`strict_intersects` only catches penetration. The failing trace is exact-edge
contact on x, so there is no strict overlap even though the vertical sweep has
no legitimate y-axis collision to resolve. Because the player's y-range is fully
nested inside the wall's y-range, the contact belongs to the x-axis wall-contact
state, not to the y-axis floor/ceiling snapper.

The fix must preserve one-way platform logic by checking one-ways first. One-way
platforms use landing-from-above semantics and should not be classified using
the side-wall helper.

Validation command:

```bash
cargo test -p ambition_gameplay_core --test repro_walls
```

### Why this was easy to miss

The existing comments said the fully nested case was caught by
`body_is_side_contact`, but the y-sweep predicate only used
`strict_intersects`. That mismatch was invisible in small synthetic repros and
only appeared in the authored full world, where a tall wall could be exact-edge
touching while still producing a `TOI=0` shape-cast hit.

Tags: `game-physics`, `axis-separated-collision`, `shape-cast`,
`edge-touching`, `platformer-collision`.
