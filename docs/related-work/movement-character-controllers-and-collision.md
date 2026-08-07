# Movement kernels, character controllers, and collision

**Checked 2026-08-07.** Ambition already has enough movement and collision
architecture that it should be compared against mature character-controller and
collision-query systems as an implemented engine subsystem, not only as future
roadmap work.

## The Ambition capability that already exists

The strongest source-level contract is in
[`ambition_platformer2d_core::movement`](../../crates/ambition_platformer2d_core/src/movement/mod.rs):

> `step_motion` is the ONLY movement entry.

Every movable body selects an explicit `MotionModel`; every model receives the
same immutable `MotionFrame`, typed local `InputState`, body-state authority and
world context. Axis-swept platformer movement, surface momentum and the adhesive
crawler are sibling policies rather than separate actor pipelines.

That matters more than the feature list. The implementation already separates
three concerns that are often fused in a game-specific controller:

1. **collision/query semantics** — sweeps, contacts, one-way rules, aperture-aware
   casts;
2. **movement policy** — axis-swept, surface-momentum or crawler behavior;
3. **control law** — the semantic action/input that asks the current body to move.

The same crate exposes gravity-relative primitives such as
`gravity_descend`, `set_jump_velocity` and `integrate_normal_spine`, explicitly
so gravity changes do not create a second controller path. Fast trigger contact
uses [`aabb_path_contacts`](../../crates/ambition_platformer2d_core/src/cast.rs)
instead of endpoint overlap, and portal-aware ray queries can recursively map a
ray through aperture pairs while preserving one distance budget.

This is already a credible comparison target.

---

## Godot `CharacterBody2D` — a compact integrated platformer controller

Godot's `CharacterBody2D` is a user-controlled body intended for characters and
other kinematic bodies. `move_and_slide()` performs collision-aware motion,
classifies contacts as floor/wall/ceiling using `up_direction`, supports slopes,
floor snapping and moving-platform following.

Sources:

- [Using CharacterBody2D/3D](https://docs.godotengine.org/en/4.5/tutorials/physics/using_character_body_2d.html)
  (Godot, official).
- [CharacterBody2D](https://docs.godotengine.org/en/4.3/classes/class_characterbody2d.html)
  (Godot, official).

### Comparison

Godot is a useful **ergonomics bar**. A small amount of script gets floor
classification, sliding, slope behavior and platform following. Ambition's
current API is lower-level and more policy-explicit.

The architectural difference is that Godot's floor semantics are properties of
one `CharacterBody2D` motion API, while Ambition is already structuring the
movement policy as a replaceable arm over shared body/frame/collision facts.
That makes Ambition's target broader: Sanic-like surface momentum, an adhesive
crawler and a conventional action-platformer body should remain different
motion policies over one kernel, not become bespoke node subclasses or parallel
physics implementations.

**Pressure on Ambition:** preserve the kernel split, but develop an equally
small author-facing configuration surface. Architectural cleanliness is not a
competitive advantage if ordinary platformer motion takes substantially more
work to author or tune.

---

## Rapier Kinematic Character Controller — queries plus reusable controller policy

Rapier's kinematic character controller adjusts a requested trajectory using ray
casts and shape casts. It supports arbitrary collider shapes and built-in
features including slopes, autostep and snap-to-ground. The same configured
controller instance can be reused across bodies that share parameters.

Sources:

- [Character controller](https://rapier.rs/docs/user_guides/rust/character_controller/)
  (Rapier, official).
- [Character controller setup](https://rapier.rs/docs/user_guides/rust/character_controller_setup/)
  (Rapier, official).
- [Shape casting](https://rapier.rs/docs/user_guides/rust/scene_queries_shape_casting/)
  (Rapier, official).

### Comparison

Rapier is close to Ambition's **mechanism boundary**: character motion is built
from geometric queries instead of requiring dynamic rigid-body simulation to own
the controller. That validates Ambition's decision to keep a deterministic
platformer movement kernel independent from a general-purpose dynamics solver.

Rapier also highlights a gap. Its controller parameters expose conventional
movement-surface behavior as one reusable object. Ambition has richer policy
variety, but the configuration story should converge on equally inspectable,
reusable motion specifications rather than a collection of tuning resources and
special-case component clusters.

---

## Box2D collision queries and continuous collision — collision as a reusable layer

Box2D exposes collision primitives independently from its full physics
simulation. Its collision API includes ray casts, shape casts and time-of-impact
queries; continuous collision advances bodies to first impact rather than only
checking their endpoints.

Sources:

- [Collision](https://box2d.org/documentation/md_collision.html)
  (Box2D, official).
- [Box2D overview — continuous collision](https://box2d.org/documentation/)
  (Box2D, official).

### Comparison

This is directly relevant to Ambition's existing `aabb_path_contacts` rule and
axis-swept movement. The lesson is not “replace Ambition collision with Box2D.”
It is that **collision queries should remain first-class reusable geometry**,
not private implementation detail of one controller.

Ambition has an additional semantic requirement that general collision
libraries do not provide by themselves: one-way behavior, gravity-relative
surface classification, moving-geometry frames, portal aperture traversal and
rollback-stable causal facts all have gameplay meaning. Keeping low-level casts
separate from those higher-level interpretations is therefore the right split.

---

## What Ambition already distinguishes

| Concern | Common mature-engine shape | Ambition's existing shape |
|---|---|---|
| Character movement | one integrated kinematic controller | one `step_motion` kernel dispatching explicit sibling motion models |
| “Up” / floor | controller property such as Godot `up_direction` | immutable `MotionFrame` supplied by the environment to every policy |
| Collision query | physics-engine ray/shape cast | engine-owned geometry query layer usable by movement, triggers and portals |
| Fast trigger contact | CCD / shape cast | `aabb_path_contacts` makes swept trigger contact explicit |
| Surface locomotion | slope handling inside a controller | surface-momentum is a separate policy with its own private state over shared authority |
| Actor kind | often controller/class-specific | `integrate_normal_spine` is explicitly actor-generic; controlled actor, NPC and enemy can share the same gravity/run spine |

The important differentiator is not “we have a character controller.” Every
engine does. It is **policy plurality over one deterministic, frame-aware body
contract**.

## Design work the comparison exposes now

### 1. Make contact output a public semantic product

The kernel already computes support/contact facts. The external engine surface
should converge on a stable `MotionStepResult`/contact report that tools,
AI, tests and alternate movement policies can consume without reaching into
solver-private state.

### 2. Measure the kernel against standard controller scenarios

Add a shared conformance suite comparable to the scenarios mature controllers
advertise:

- shallow and steep slopes;
- step-up / step-down;
- snap-to-ground;
- moving platform carry and departure momentum;
- high-speed thin obstacles and trigger volumes;
- one-way surfaces under non-default gravity;
- body-shape transitions under obstruction;
- surface-to-air and wall-to-floor transitions;
- portal transit during or immediately after contact.

Those fixtures should run every `MotionModel` where the scenario is meaningful.
The result would be much stronger than feature prose: one executable matrix of
what “platformer movement” means in this engine.

### 3. Keep tuning declarative and inspectable

Rapier and Godot demonstrate the usability value of a compact controller
configuration. Ambition should preserve distinct motion models while making
each model's authored specification serializable, printable and suitable for a
live tuning inspector.

### 4. Define the extension point for new motion models

The source proves multiple policies can coexist, but the public SDK contract for
a third-party movement model is not yet as clear as the internal enum dispatch.
Decide whether external games may add new policies, parameterize only engine
policies, or contribute policies through a capability boundary. This decision
belongs to the SDK surface, not to an accidental visibility choice in
`ambition_platformer2d_core`.

## What this comparison changed

The movement subsystem should be described as a current engine strength, not a
roadmap placeholder. The competitive bar is now clearer:

- **match Godot/Rapier controller ergonomics for ordinary cases;**
- **retain Box2D/Rapier-style separation between queries and higher-level
  movement policy;**
- **differentiate with one frame-explicit deterministic body kernel supporting
  materially different locomotion models without actor-specific physics paths.**
