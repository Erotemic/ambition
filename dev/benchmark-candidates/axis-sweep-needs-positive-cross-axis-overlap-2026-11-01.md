# An axis sweep needs positive overlap across the axis

A closed shape cast can report contact when two boxes touch only at an edge.
For separated-axis body movement, that contact is not a blocking face. A body
beside stacked wall blocks can receive a head contact when it jumps past the
upper block, even though the block does not extend over the body.

Before a block can stop an axis sweep, require positive interval overlap on the
perpendicular world axis. Exact edge contact contributes no collision on the
moving axis. This rule needs no added gap or pixel threshold. It keeps real
narrow support and real ceiling overlap.

The rule belongs in the shared body sweep. A character-specific jump exception
would leave other bodies and gravity directions with different collision rules.

The regression test presses a body against a two-block wall and then jumps.
It covers both sides, several body widths and translated coordinates. A real
ceiling is the negative control.

```bash
cargo test -p ambition_platformer2d_core --lib -- a_jump_beside_stacked_wall_blocks
```
