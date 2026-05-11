# 2026-05-11 movement edge-touch y-sweep lesson

Additive journal entry for the final failure after the movement split compiled
and headless fuzz passed.

## Symptom

`cargo test -p ambition_engine --lib` passed, and the random-walker fuzz passed,
but two full-world wall-cling repros still failed:

```text
FULL WORLD step: pos=(62, 1783) vel=(0, 0) on_ground=false on_wall=true cling=true
dy=215.08752 (initial y=1567.9125, after y=1783)
```

The smaller arena and mob-lab repros passed. The failure only appeared in the
full authored square-arena world on iteration 0.

## Root cause

The y-sweep predicate rejected blocks that `start_body.strict_intersects`, but
that only catches penetration. In the full-world repro, the player was exactly
edge-touching a tall wall on x while the player's y-range was fully nested
inside the wall's y-range. Parry could still report a `TOI=0` hit for the
vertical sweep, and `sweep_player_y` treated the side-wall contact as a vertical
floor/ceiling collision.

The resulting snap moved the player to the wall's bottom edge, causing a large
single-frame y displacement.

## Fix

Reject side-wall contacts in the y-sweep predicate before accepting Parry's hit:

```rust
if body_is_side_contact(start_body, block.aabb) {
    return false;
}
if start_body.strict_intersects(block.aabb) {
    return false;
}
```

Keep one-way platform handling before this guard so one-way landing semantics
remain unchanged.

## Takeaway

For axis-separated collision, "not strictly overlapping" does not mean "valid
contact for this axis." Exact-edge contacts can still produce immediate
shape-cast hits. If the body's y-range is fully nested in a tall wall, the
contact belongs to horizontal wall state, not vertical snap resolution.
