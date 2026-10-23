# GNU-ton gravity handle blockout

This world is a remote part of the Math manifold. It is separate from Math Town. The central hub has a door to the first room. The last room stops before the GNU-ton boss encounter.

## Room sequence

1. **`gnu_ton_outer_handle`** - Cross gaps, reach an upside-down roof, walk against a wall, and cross an oscillating up-gravity column.
2. **`gnu_ton_inversion_gallery`** - Use ceiling paths and a moving platform between gravity columns.
3. **`gnu_ton_vector_well`** - Climb a tall space by moving through right, up, left, and up gravity fields.
4. **`gnu_ton_proofworks`** - Cross a moving platform, use a linked floor-to-wall portal pair, and reach an empty boss threshold.

The geometry and entity positions are first-pass puzzle scaffolds. The rooms have collision, moving platforms, gravity zones, switches, arrivals, and return routes. The sequence has not been tuned for complete traversal or combat.

## Current mechanics and remaining work

- `GravityZone` fields work with the existing engine. An oscillating zone moves left and right.
- The `SetGravity*` switches change **ambient gravity**. They do not turn `GravityZone` fields on or off. Dynamic field control is future puzzle work.
- A door targets a separate arrival-only `LoadingZone`. This prevents an immediate return transition. The arrival zones use `Walk` and have no target.
- The proofworks portal pair uses the existing `Portal.link` field. It is a test pair. It is not part of the GNU-ton boss moveset.
- Enemy placement, art, music, encounter state, reward, and the story boss connection remain open. The Hall of Bosses GNU-ton route is unchanged.

Each `.ron` file in this directory is the editable source for one room. The `.ldtk` world is the shipped map. To rebuild one room, run `ambition_ldtk_tools area create <spec.ron> --ldtk game/ambition_content/assets/worlds/gnu_ton_handle.ldtk --replace-existing` with `PYTHONPATH=tools/ambition_ldtk_tools`.
