# Swept parallel-graze far-edge de-penetration benchmark candidate — 2026-06-04

The X-axis analog of [`movement-edge-touch-y-sweep-question-2026-05-11.md`](movement-edge-touch-y-sweep-question-2026-05-11.md):
the same axis-separated-collision shape-cast trap, on the other axis, ~4 weeks
later. The y-sweep was guarded against edge contacts back in May
(`body_is_side_contact`); the x de-pen never got the symmetric guard, so it sat
latent until a player flew along the ceiling.

## Q: Defer floor/ceiling contacts in the X de-pen regardless of `immediate_contact`

### Level A prompt

A platformer uses axis-separated AABB collision. The X step sweeps the body and,
on a hit, de-penetrates it out of the block. It already tries to recognise a
floor/ceiling contact (where the *vertical* exit is shorter) and let the Y pass
resolve it instead of pushing in X:

```rust
let body = kinematics.aabb();
let immediate_contact = hit.time_of_impact <= 1.0e-5;
let exit_left  = body.right()  - block.left();   // push left  this far
let exit_right = block.right() - body.left();    // push right this far
let exit_up    = body.bottom() - block.top();
let exit_down  = block.bottom() - body.top();
let x_exit = exit_left.min(exit_right);
let y_exit = exit_up.min(exit_down);
// "A floor/ceiling contact -- the vertical exit is shorter; defer to the Y pass."
let vertical_dominant = immediate_contact && y_exit <= x_exit;
if vertical_dominant || horizontal_overlap_moving_away {
    kinematics.pos.x += delta.x * (1.0 - toi_fraction); // keep the swept motion going
} else {
    // push out the nearer X face
    if exit_left < exit_right { kinematics.pos.x -= exit_left; }
    else                      { kinematics.pos.x += exit_right; }
    kinematics.vel.x = 0.0;
}
```

The world has a wide, thin ceiling block spanning the whole room
(`0..1904 × 0..32`) whose right edge coincides with the world's right wall. A
**48-tall** player flies **LEFT** at high speed with its top resting exactly on
the ceiling's bottom edge (top = 32). In one frame it teleports ~900px to the
**RIGHT**, out of the world. Why, and what is the minimal fix?

### Expected answer

Sliding parallel just under the ceiling, the swept shape-cast (Parry
`cast_shapes`) reports a **non-immediate** grazing contact with the ceiling —
`time_of_impact` a hair above `0`, even though the body is ~0.01px *below* the
ceiling and (moving purely in X) never approaches it. Because `vertical_dominant`
is gated on `immediate_contact`, the grazing hit is **not** deferred; it falls
through to the else-branch, which pushes the body out the ceiling's **far X
edge**: `exit_right = block.right(1904) − body.left ≈ 918px`. That is the
teleport. (`wall_normal_x` ends `+1` and `vel.x` is zeroed — the de-pen
fingerprints, even though the body was moving the other way.)

The `y_exit <= x_exit` test is already correct — for the grazing contact
`y_exit ≈ 0` or negative while `x_exit ≈ 918` — but it must be applied
**regardless of `immediate_contact`**. The fix factors the X de-pen into one
helper used by both the swept de-pen and the positional repair, which:

```rust
fn resolve_x_penetration(body: Aabb, block: Aabb, world_w: f32) -> Option<(f32, f32)> {
    let exit_left = body.right() - block.left();
    let exit_right = block.right() - body.left();
    let exit_up = body.bottom() - block.top();
    let exit_down = block.bottom() - body.top();
    if exit_up.min(exit_down) <= exit_left.min(exit_right) {
        return None; // vertical exit shorter -> Y pass owns it (NO immediate gate)
    }
    // ... otherwise push the nearer X face, but never out of the world:
    let half_w = (body.right() - body.left()) * 0.5;
    let cx = body.center().x;
    let left  = ((cx - exit_left)  - half_w >= 0.0).then_some((-exit_left, -1.0));
    let right = ((cx + exit_right) + half_w <= world_w).then_some(( exit_right,  1.0));
    if exit_left <= exit_right { left.or(right) } else { right.or(left) }
}
```

Returning `None` means "not an X contact to resolve here," so the swept de-pen
keeps the motion going. The eject-guard (`left`/`right` only `Some` when the push
stays in the world) handles the top-corner case where the nearer X face of a
boundary-spanning block *is* the world edge.

### Why this was easy to miss

1. **Cross-axis symmetry.** The identical bug class was fixed on the *other* axis
   in May: `body_is_side_contact` rejects edge-touch side contacts before the
   y-sweep. The x-side never got the mirror guard. When you harden one axis of an
   axis-separated collision against spurious shape-cast hits, harden the other.
2. **Untested toi regime.** Two earlier fixes addressed the *deep-penetration*
   and *corner-eject* shapes of the same teleport, but both gated their defer on
   `immediate_contact`. A swept-collision guard that is "mostly right" usually has
   a `time_of_impact` regime (here: small-but-nonzero, the parallel graze) that
   no test exercises.
3. **Repro fidelity.** The bug needs the exact body size (30×48, not the nominal
   24×40) and the exact float trajectory; synthetic repros with round positions
   did not trigger Parry's spurious hit. The reliable repro was a unit test
   calling the X-sweep directly with the captured `(pos, vel, size, dt)` from the
   trace frame. A "delta opposes velocity" teleport (moving left, flung right) is
   the tell for a de-pen firing on a bogus hit.

Validation command:

```bash
cargo test -p ambition_sandbox --lib ceiling_graze_x_sweep_does_not_teleport_body_to_the_far_edge
```

Tags: `game-physics`, `axis-separated-collision`, `shape-cast`, `edge-touching`,
`platformer-collision`, `cross-axis-symmetry`.
