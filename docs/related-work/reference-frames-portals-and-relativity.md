# Reference frames, portals, and relativity

**Checked 2026-08-07.** Ambition already treats reference frames as gameplay
semantics rather than only transform math. That deserves comparison to robotics
frame systems, ordinary game-engine transform spaces, portal transforms, and
special-relativity toolkits.

## The Ambition capability that already exists

Several independently implemented systems now meet at the same idea:

- [`MotionFrame`](../../crates/ambition_platformer2d_core/src/movement/mod.rs)
  is resolved by the environment and supplied immutably to the movement kernel;
- gravity-relative input and launch primitives deliberately avoid hard-coded
  screen/world axes;
- [`ambition_portal2d`](../../crates/ambition_portal2d/src/lib.rs) models portal
  transit as a spatial mapping over ordinary bodies rather than as a separate
  actor type;
- [`ray_through_apertures`](../../crates/ambition_platformer2d_core/src/cast.rs)
  maps both points and directions through aperture pairs while preserving a
  bounded distance budget;
- [`ambition_relativity`](../../crates/ambition_relativity/src/lib.rs) provides a
  dimension-independent special-relativity foundation;
- [`ambition_relativity2d`](../../crates/ambition_relativity2d/src/lib.rs) adds
  2D optics, signals, causal targeting and bounded rollback-aware worldline
  telemetry.

This is not merely “the game can rotate gravity” or “the game has portals.” The
common engine question is: **what frame is this position, direction, velocity,
input or observation stated in, and how does it transform when the relevant
frame changes?**

---

## ROS `tf2` — frame identity and transforms are first-class data

ROS 2's `tf2` library tracks multiple coordinate frames over time. It maintains
relationships among frames in a time-buffered tree and transforms points,
vectors and other stamped data between frames at a requested time. Its advanced
API can query transforms between past and current poses.

Sources:

- [About tf2](https://docs.ros.org/en/rolling/ROS-Framework/interfaces/About-Tf2/About-Tf2.html)
  (ROS, official).
- [Traveling in time with tf2](https://docs.ros.org/en/humble/Tutorials/Intermediate/Tf2/Time-Travel-With-Tf2-Cpp.html)
  (ROS, official).

### Why this is unusually relevant

`tf2` is closer prior art for Ambition's **architectural vocabulary** than most
game engines. It establishes several useful principles:

1. a vector without an understood frame is incomplete information;
2. frame relationships have identity and may vary over time;
3. tools should be able to inspect the frame graph;
4. transformation should happen at explicit boundaries rather than through
   scattered coordinate arithmetic.

Ambition does not need a general `tf2` clone. Its world is much smaller and a
portal graph is not a tree. But it should borrow the inspectability: a developer
ought to be able to ask which frame a body/input/cast is in, how that frame was
resolved, and which transform edge was applied.

---

## Unreal and Godot — local/world spaces are pervasive but usually structural

Unreal exposes explicit relative transform spaces including world, actor,
component and parent-bone space. Godot's `Transform2D` represents a 2D basis and
origin and supports operations in global and local space.

Sources:

- [Unreal `ERelativeTransformSpace`](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/ERelativeTransformSpace)
  (Epic, official).
- [Godot `Transform2D`](https://docs.godotengine.org/en/stable/classes/class_transform2d.html)
  (Godot, official).

### Comparison

Mature engines already teach users to distinguish local and world transforms.
Ambition therefore should not claim “frames” themselves as novel.

The stronger Ambition distinction is **which gameplay quantities are forced
through frame-aware seams**. The movement kernel carries frame semantics into
input interpretation, gravity, support, launch velocities and movement policies;
portal casts transform a query's direction as well as its point of origin; the
relativity layers use worldlines and observers rather than treating the camera
as a privileged universal viewpoint.

That is a deeper contract than merely storing every object under a parent
transform.

---

## OpenRelativity / *A Slower Speed of Light* — relativity as an engine feature

MIT Game Lab's OpenRelativity is an open-source toolkit developed for *A Slower
Speed of Light* to simulate special-relativistic effects by varying the speed of
light. The project demonstrates that relativistic rendering/experience can be
packaged as reusable game technology rather than a one-off visualization.

Sources:

- [OpenRelativity](https://gamelab.mit.edu/research/openrelativity/)
  (MIT Game Lab, official).
- [A Slower Speed of Light](https://gamelab.mit.edu/games/a-slower-speed-of-light/)
  (MIT Game Lab, official).

### Comparison

OpenRelativity is the obvious direct prior art for Ambition's relativity stack,
but the implementation emphasis is different. Ambition already separates:

- dimension-independent Lorentz/causal mathematics;
- gameplay-owned proper-time clocks;
- observer-relative 2D optics and signal/targeting rules;
- ordinary platformer movement and portal frames.

That decomposition is worth preserving. A relativistic rendering effect should
not become the owner of gameplay time, and a portal should not need to know
about a particular character controller.

---

## Portals as transform edges, not teleports

The portal source makes a useful claim that should become explicit engine
terminology: a portal transit is a **mapping between local frames**. Position,
orientation, velocity/gravity frame and continuation queries all have to cross
the same edge coherently.

This makes portals comparable less to an engine's “teleport actor” call and more
to adding a non-Euclidean edge to the frame graph. That perspective explains
why `ray_through_apertures` belongs in shared geometry and why ordinary bodies
remain ordinary on both sides.

The design opportunity is to generalize only the minimum useful abstraction:

```text
FrameId / FrameObservation
    + TransformEdge(source, destination, mapping, validity)
    + explicit mapping of point / vector / velocity / orientation
```

A full arbitrary frame graph may be unnecessary. But an inspector-visible frame
identity and transform provenance would unify several features that already use
the idea independently.

## What Ambition already distinguishes

| Concern | Typical engine baseline | Ambition's existing shape |
|---|---|---|
| Object transforms | local/world transform hierarchy | same, plus gameplay frame carried into movement semantics |
| Gravity direction | world/project setting or per-body vector | environmental `MotionFrame` consumed by the one movement kernel |
| Input direction | often screen/world assumptions in controller code | directional input is explicitly interpreted relative to the controlled body's frame |
| Portal transit | teleport + render trick is a common implementation | aperture pair maps point/vector and ordinary simulation state through a reusable mechanic |
| Ray query through portal | mechanic-specific recursion | shared collision query recursively transforms the ray through aperture frames |
| Relativistic effects | specialized demo/toolkit | proper-time and SR foundations are engine-domain libraries, with observer-relative 2D gameplay/optics on top |

## Design work the comparison exposes now

### 1. Name frame identity in diagnostics

`MotionFrame` is currently a value. For debugging, causal facts should also be
able to say *which frame relationship produced it*. A stable `FrameId` or
`FrameSource` vocabulary would let an explanation state, for example, that a
jump launch was evaluated in room gravity frame X after portal edge Y.

### 2. Make frame transforms inspectable

Borrow the `tf2` lesson: tooling should render the active frame/portal graph and
answer point/vector conversion queries. This can begin as a text/JSON dump; no
editor is required.

### 3. Specify transform laws by quantity

Points, directions, velocities, accelerations, normals and observer events do
not all transform for the same reasons. Keep typed helpers for each meaningful
quantity rather than exposing one generic matrix call that invites incorrect
semantic reuse.

### 4. Define the portal/relativity interaction explicitly

Both systems already exist. Their composition therefore needs a declared law,
not a future handwave: which observer/worldline frame is used across an aperture,
what happens to signal propagation, and whether portal mappings are instantaneous
spatial identifications or participate in causal delay. Even if the first
answer is intentionally simple, it should be a tested contract.

## What this comparison changed

Reference frames should be a first-class related-work topic. The strongest
outside analogy is not another platformer but ROS `tf2`: Ambition is already
building a smaller gameplay-oriented frame system. The differentiation is the
combination of **frame-explicit platformer movement, portal topology and
observer-relative relativity under one deterministic simulation contract**.
