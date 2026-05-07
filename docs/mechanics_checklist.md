# Metroidvania Mechanics Expressibility Checklist

This document tracks which metroidvania-style mechanics are currently expressible in the Ambition engine, and which ones require additional backend work.

> Last sync: 2026-05-07. The checklist is read-only documentation;
> `TODO.md` and `FEATURES.md` are the operational sources of truth.
> When marking an entry `[x]` here, also add the matching `FEATURES.md`
> entry (and remove from `TODO.md` if it was tracked there).

The focus is on **player-control-level mechanics** and **engine expressibility**, not story design, campaign structure, or authored progression. A mechanic counts as expressible when the engine has reusable runtime primitives for it: input handling, player state, collision behavior, physics integration, targeting/query support, resource state, or authored data hooks.

A mechanic does **not** need to be a bespoke one-off implementation. Prefer generic backend systems that make whole families of mechanics expressible. For example, a `FunctionalZip` ability with a curve argument can support exponential zip, sine dash, parabolic cast, Bezier rails, and spiral zip without each being a separate controller mode.

## Legend

- `[x]` = already expressible as a real player/backend mechanic
- `[~]` = partially scaffolded, but not a full mechanic yet
- `[ ]` = not yet expressible / needs backend work

---

# Backend expressibility primitives

These are the highest-level engine systems. Many named mechanics below become easy once these exist.

- [x] Single-body kinematic player controller
- [x] Ability-gated movement verbs
- [x] Input buffering / grace timing
- [x] Dash charge resource
- [x] Air-jump resource
- [x] Blink aiming / teleport targeting
- [x] Basic melee / pogo / rebound impulse verbs
- [x] Projectile / damage backend (`ambition_engine::projectile`; sandbox player Fireball + Hadouken)
- [~] Bullet-time-compatible timing

- [x] Explicit `PlayerMode` / `LocomotionState` enum (`ambition_engine::LocomotionState`)
- [x] Alternate player body shapes (`ambition_engine::BodyMode` / `BodyShape`)
- [x] Collision-safe body resize (`BodyShape::fits_at`; the *backend*; gameplay verbs still pending)
- [x] General resource meter backend (`ambition_engine::ResourceMeter`)
- [x] Player projectile action backend (`ProjectileSpawner` + `MotionInputBuffer`)
- [ ] Aim-mode abstraction
- [ ] Moving-platform carry velocity
- [ ] Surface tangent / normal query
- [ ] Generic ray / shape-cast targeting API
- [ ] Grapple / tether constraint backend
- [ ] Parametric curve movement backend
- [ ] Curve preview renderer
- [ ] Collision-safe curve traversal
- [ ] Curve basis library
- [ ] Vector-field sampler
- [ ] Scalar-field / gradient sampler
- [ ] Transform volume backend
- [ ] Local-clock / per-entity time backend
- [ ] Oscillator / phase backend
- [ ] Path-integral accumulator
- [ ] Coordinate-space remapping backend
- [ ] Deterministic randomness / probability backend

---

# Core locomotion

- [x] Walk / run
- [x] Air control
- [x] Ground jump
- [x] Variable-height jump
- [x] Coyote-time jump
- [x] Jump buffering
- [x] Double jump / air jump
- [x] Fast fall
- [x] Drop through one-way platforms
- [x] Wall jump
- [x] Wall cling / wall slide
- [x] Wall climb
- [x] Dash
- [x] Double dash / dash charges
- [x] Free flight / noclip-like fly mode
- [x] Glide / cape / slow-fall — held-jump airborne; `glide_fall_speed` cap + `glide_air_accel`; `AbilitySet::glide`
- [ ] Hover with resource drain
- [~] Ledge grab — `Ability::ledge_grab` + `LedgeGrabState`; sandbox-side, awaiting engine promotion
- [~] Ledge mantle / climb-up — Up+Jump triggers climb to `LedgeContact::climb_target`; animation slot still pending
- [x] Crouch — `BodyMode::Crouching`
- [x] Crawl — `BodyMode::Crawling`
- [x] Slide — `BodyMode::Sliding`
- [ ] Roll / dodge roll
- [ ] Sprint acceleration state
- [ ] Momentum-preserving long jump
- [ ] Charge jump
- [ ] Stomp / ground pound

---

# Shape / body-state mechanics

This is a major missing backend category. These mechanics require alternate body shapes, collision-safe resizing, and stance/form-specific control rules.

- [x] Morph ball — `BodyMode::MorphBall` driver + double-tap-down trigger
- [ ] Morph-ball tunnels — needs IntGrid pinch-test rooms; backend ready
- [ ] Spring ball
- [ ] Bombs while morphed
- [ ] Spider ball / magnetic ball
- [x] Compact hitbox mode — per-`BodyMode` `BodyShape` resizes the kinematic body
- [x] Collision-safe resize / unmorph validation — `BodyShape::fits_at`
- [~] Alternate hurtbox by stance — `BodyShape` resizes; damage volume still uses player AABB
- [x] Form switching — `BodyMode` transitions wired through `SandboxRuntime`
- [x] Size-changing traversal — `BodyMode::{Crouching, Crawling, MorphBall}` already enable narrow passages
- [ ] Blob / liquid / squeeze-through form

Recommended primitive:

```rust
enum BodyMode {
    Standing,
    Crouching,
    Crawling,
    MorphBall,
    Liquid,
}
```

---

# Teleport / phase / blink mechanics

* [x] Quick blink
* [x] Precision blink aim
* [x] Blink cooldown
* [x] Blink grace / safety handling
* [x] Blink through soft walls
* [x] Blink through hard walls
* [ ] Enemy-target blink
* [ ] Blink strike
* [ ] Portal placement
* [ ] Return-to-marker teleport
* [ ] Telefrag / blink-damage rules
* [ ] Phase dash through enemies
* [ ] Phase dash through walls
* [ ] Time-limited ghost form

---

# Grapple / tether mechanics

This is one of the highest-value missing traversal systems. It requires ray/shape queries, latch targets, attach state, pull/swing motion, detach rules, and collision-aware tether motion.

* [ ] Grappling hook
* [ ] Hookshot pull
* [ ] Swinging rope / tether
* [ ] Grapple-to-point
* [ ] Grapple-to-enemy
* [ ] Grapple-to-moving-object
* [ ] Grapple cancel / detach
* [ ] Grapple reel in / reel out
* [ ] Tether constraint physics
* [ ] Zipline
* [ ] Whip latch
* [ ] Tongue latch / pull

Recommended primitive:

```rust
enum GrappleState {
    Idle,
    Aiming,
    Firing,
    Latched {
        target: GrappleTarget,
        length: f32,
    },
    Pulling,
    Swinging,
}
```

---

# Combat-facing player actions

* [x] Basic melee attack / slash
* [x] Pogo attack
* [x] Rebound impulse from surfaces
* [x] Projectile damage backend
* [x] Player projectile weapon (Fireball + Hadouken via half-circle motion input)
* [x] Charge shot — multi-tier hold-to-charge fireballs; `ResourceMeter` mana drain
* [ ] Beam / gun / ranged primary
* [ ] Missile / ammo weapon
* [ ] Spell cast
* [ ] Directional melee aim
* [ ] Up-slash
* [ ] Down-slash beyond pogo
* [ ] Dash attack
* [ ] Spin attack
* [ ] Parry
* [ ] Counterattack
* [ ] Shield block
* [ ] Reflect projectile
* [ ] Invincibility-frame dodge
* [ ] Weapon switching
* [ ] Charge attack
* [ ] Combo chain
* [ ] Attack cancel windows

---

# Movement resource systems

* [x] Dash charges
* [x] Air-jump resource
* [x] Movement resource refresh on ground
* [x] Resource refresh from pogo / rebound-style interactions
* [ ] Stamina meter
* [~] Mana / soul / magic meter — sandbox-side `mana_current` / `mana_max` drives charge fireballs; awaits engine `ResourceMeter` promotion
* [ ] Ammo meter
* [ ] Heat / overheat meter
* [ ] Hover fuel
* [ ] Oxygen / breath meter
* [ ] Rage / super meter
* [ ] Temporary buff timers
* [ ] Consumable movement charges

Recommended primitive:

```rust
struct ResourceMeter {
    current: f32,
    max: f32,
    regen_rate: f32,
    decay_rate: f32,
}
```

---

# Environment movement modes

Many of these can be implemented as special cases of vector fields, media volumes, surface materials, or gravity transforms.

* [x] Swimming — `Ability::swim` + `IntGrid` `BlockKind::Water`; two-pool `water_world` lab
* [x] Underwater gravity / drag — swim mode swaps gravity / max-speed
* [x] Water surface transition — overlapping-zone fix lets the player surface cleanly
* [ ] Diving
* [ ] Buoyancy
* [ ] Currents
* [ ] Wind zones
* [ ] Conveyor belts
* [ ] Ice / low-friction surfaces
* [ ] Mud / high-friction surfaces
* [ ] Quicksand / sinking volumes
* [ ] Low-gravity zones
* [ ] High-gravity zones
* [ ] Gravity flip
* [ ] Arbitrary gravity direction
* [ ] Walk on ceiling
* [ ] Walk on walls
* [ ] Magnetic surfaces
* [ ] Lava / acid suit traversal
* [ ] Environmental resistance suits

---

# Speed / momentum mechanics

* [ ] Speed booster
* [ ] Shinespark
* [ ] Stored charge launch
* [ ] Momentum carry between rooms
* [ ] Slope running
* [ ] Slope sliding
* [ ] Wall bounce
* [ ] Super dash
* [ ] Rail grind
* [ ] Momentum gates / speed blocks
* [ ] Break blocks by velocity
* [ ] Dive-kick
* [ ] Rocket jump
* [ ] Bomb jump
* [ ] Enemy bounce chaining

---

# Functional / mathematical traversal

These mechanics should be built on a generic **parametric curve movement** backend rather than bespoke one-off abilities.

A single `FunctionalZip` primitive could support exponential zip, parabolic cast, sine dash, spiral zip, Bezier rails, and other curve-driven movement.

## Generic curve movement primitives

* [ ] Parametric path locomotion
* [ ] Collision-safe movement along sampled curves
* [ ] Curve preview / ghost trajectory renderer
* [ ] Functional zip ability
* [ ] Functional zip with arbitrary `f(t)` argument
* [ ] Functional zip with arbitrary `Vec2` function argument
* [ ] Piecewise functional zip
* [ ] Curve cancel / bonk / rebound rules
* [ ] Curve-following resource cost
* [ ] Curve-following cooldown
* [ ] Curve-following target validation
* [ ] Designer-authored curve rails
* [ ] Player-authored curve tracing
* [ ] Curve basis unlocks
* [ ] Curve chaining
* [ ] Reverse curve traversal

## Specific curve movement verbs

* [ ] Exponential zip
* [ ] Logarithmic zip
* [ ] Parabolic / ballistic cast
* [ ] Sine-wave dash
* [ ] Cosine-wave dash
* [ ] Spiral zip
* [ ] Cycloid / rolling-curve dash
* [ ] Bezier zip
* [ ] Spline rail zip
* [ ] Arc / circular zip
* [ ] Ellipse orbit dash
* [ ] Lissajous dash
* [ ] Sawtooth / triangle-wave dash
* [ ] Step-function teleport chain

## Exponential zip

The flagship version of functional zip.

Gameplay behavior:

* Player aims or selects a direction.
* A curve preview appears.
* Releasing the input zips the player along an exponential path.
* Low curvature behaves like a long launch.
* High curvature behaves like a sudden vertical whip or super jump.

Example curve:

```text
y = a * (exp(kx) - 1)
```

Checklist:

* [ ] Exponential zip curve basis
* [ ] Exponential zip aim mode
* [ ] Exponential zip curvature parameter
* [ ] Exponential zip preview
* [ ] Exponential zip wall collision behavior
* [ ] Exponential zip cancel into dash
* [ ] Exponential zip cancel into pogo
* [ ] Exponential zip cancel into rebound
* [ ] Exponential zip room-gate validation

Recommended primitive:

```rust
enum CurveSpec {
    Line {
        length: f32,
    },
    Parabola {
        vx: f32,
        vy: f32,
        gravity: f32,
    },
    Exponential {
        scale: f32,
        growth: f32,
        length: f32,
    },
    Sine {
        amplitude: f32,
        frequency: f32,
        phase: f32,
        length: f32,
    },
    Spiral {
        radius: f32,
        turns: f32,
        decay: f32,
    },
    Bezier {
        points: Vec<Vec2>,
    },
    Sampled {
        points: Vec<Vec2>,
    },
}

enum CurveCollisionPolicy {
    StopOnHit,
    SlideOnHit,
    BounceOnHit,
    CancelOnHit,
    DamageOnHit,
    PhaseThroughTagged,
}
```

---

# Curve basis library

The curve basis library is the data layer that makes functional traversal designer-friendly.

* [ ] Line basis
* [ ] Parabola basis
* [ ] Exponential basis
* [ ] Logarithm basis
* [ ] Sine basis
* [ ] Spiral basis
* [ ] Circle / arc basis
* [ ] Ellipse basis
* [ ] Bezier basis
* [ ] Catmull-Rom / spline basis
* [ ] Polynomial basis
* [ ] Fourier basis
* [ ] Piecewise basis
* [ ] Designer-authored sampled curve
* [ ] Runtime curve composition

Potential progression unlocks:

* [ ] Unlock exponential basis
* [ ] Unlock sine basis
* [ ] Unlock spiral basis
* [ ] Unlock piecewise composition
* [ ] Unlock curve chaining
* [ ] Unlock curve inversion / reverse traversal

---

# Field-based movement

A vector-field backend can make wind, currents, gravity wells, vortex surfing, magnetic fields, and projectile-bending rooms expressible through one system.

## Vector-field primitives

* [ ] World vector-field sampler
* [ ] Local vector fields attached to rooms
* [ ] Local vector fields attached to entities
* [ ] Player affected by vector fields
* [ ] Projectiles affected by vector fields
* [ ] Enemies affected by vector fields
* [ ] Platforms affected by vector fields
* [ ] Field strength falloff
* [ ] Field visualization
* [ ] Field composition / superposition
* [ ] Field-gated movement abilities

## Specific vector-field verbs / environments

* [ ] Vector-field surfing
* [ ] Wind zones
* [ ] Water currents
* [ ] Gravity wells
* [ ] Repulsor fields
* [ ] Vortex fields
* [ ] Saddle fields
* [ ] Magnetic attraction fields
* [ ] Magnetic repulsion fields
* [ ] Conveyor fields
* [ ] Orbit fields
* [ ] Flow-map rooms
* [ ] Projectile-bending fields

Recommended primitive:

```rust
trait VectorField {
    fn sample(&self, position: Vec2, time: f32) -> Vec2;
}
```

---

# Scalar-field / gradient mechanics

Scalar fields support gradient climbing, potential wells, hidden field reveals, and route-planning puzzles.

* [ ] Scalar field sampler
* [ ] Gradient computation
* [ ] Gradient-following movement
* [ ] Gradient-climb ability
* [ ] Gradient-descent ability
* [ ] Potential wells
* [ ] Potential hills
* [ ] Equipotential platforms
* [ ] Hidden scalar-field reveal
* [ ] Field extrema as targets
* [ ] Field-line visualization

Specific mechanics:

* [ ] Gradient climb
* [ ] Potential-well trap
* [ ] Potential-well slingshot
* [ ] Harmonic-field room
* [ ] Heat-map traversal
* [ ] Follow-the-gradient puzzle
* [ ] Climb to local maximum
* [ ] Escape local minimum

Recommended primitive:

```rust
trait ScalarField {
    fn sample(&self, position: Vec2, time: f32) -> f32;

    fn gradient(&self, position: Vec2, time: f32) -> Vec2 {
        // Finite difference or analytic gradient.
        todo!()
    }
}
```

---

# Surface-geometry movement

This family makes level geometry mechanically expressive. Rebound/pogo is a partial seed, but general tangent/normal/curvature mechanics require explicit surface queries.

* [~] Rebound impulse from surfaces
* [ ] Surface tangent query
* [ ] Surface normal query
* [ ] Curvature query
* [ ] Tangent dash
* [ ] Normal launch
* [ ] Curvature-dependent launch
* [ ] Curvature-dependent grip
* [ ] Curve rail movement
* [ ] Wall tangent acceleration
* [ ] Surface-following locomotion
* [ ] Surface transition across corners

Specific mechanics:

* [ ] Tangent dash
* [ ] Normal launch
* [ ] Curved-wall rebound
* [ ] Concave focusing launch
* [ ] Convex scattering launch
* [ ] Curvature gate
* [ ] Geometric rail slide
* [ ] Derivative gate

Recommended primitive:

```rust
struct SurfaceContact {
    point: Vec2,
    normal: Vec2,
    tangent: Vec2,
    curvature: Option<f32>,
    material: SurfaceMaterial,
}
```

---

# Transform / linear-algebra movement

Transform volumes allow rooms to alter position, velocity, input, gravity, or collision space. This can express complex velocity rotation, momentum reflection, shear rooms, mirror gates, and eigen-dash mechanics.

## Transform primitives

* [ ] Transform volume
* [ ] Position transform volume
* [ ] Velocity transform volume
* [ ] Input transform volume
* [ ] Gravity transform volume
* [ ] Collision-space transform volume
* [ ] Transform preview / ghost vector
* [ ] Transform composition
* [ ] Transform inverse
* [ ] Transform-gated movement

## Specific transforms

* [ ] Rotate velocity
* [ ] Reflect velocity
* [ ] Scale velocity
* [ ] Shear velocity
* [ ] Invert velocity
* [ ] Rotate gravity
* [ ] Reflect position
* [ ] Scale position
* [ ] Shear room coordinates
* [ ] Complex multiplication transform
* [ ] Matrix-field movement

## Specific mechanics

* [ ] Complex velocity rotation
* [ ] Rotate momentum by 90 degrees
* [ ] Momentum reflection gate
* [ ] Momentum refraction gate
* [ ] Eigen dash
* [ ] Basis-shift room
* [ ] Shear dash
* [ ] Mirror dash
* [ ] Inversion gate

Recommended primitive:

```rust
struct TransformVolume {
    aabb: Aabb,
    position_transform: Option<Affine2>,
    velocity_transform: Option<Mat2>,
    input_transform: Option<Mat2>,
    gravity_transform: Option<Mat2>,
}
```

---

# Oscillation / resonance mechanics

This family bridges traversal, combat, timing windows, pogo, moving platforms, and wave hazards.

* [ ] Oscillator component
* [ ] Phase tracker
* [ ] Frequency-tagged platforms
* [ ] Frequency-tagged enemies
* [ ] Resonance window detection
* [ ] Resonant amplification
* [ ] Damping
* [ ] Driven oscillator behavior
* [ ] Standing wave hazards
* [ ] Wave interference fields

Specific mechanics:

* [ ] Resonance jump
* [ ] Resonance pogo
* [ ] Frequency-matched door
* [ ] Phase dash
* [ ] Wave platform
* [ ] Standing-wave ladder
* [ ] Interference hazard
* [ ] Harmonic slash
* [ ] Fourier slash
* [ ] Harmonic dash

Recommended primitive:

```rust
struct Oscillator {
    frequency: f32,
    phase: f32,
    amplitude: f32,
    damping: f32,
}
```

---

# Integral / accumulated-path mechanics

These mechanics reward route choice and movement optimization. They are useful for charge jumps, charge beams, path-length gates, field harvesting, and risk/reward traversal.

* [ ] Path-integral accumulator
* [ ] Area-under-curve charge
* [ ] Field-integral charge
* [ ] Distance-weighted resource gain
* [ ] Velocity-weighted resource gain
* [ ] Hazard-proximity charge gain
* [ ] Route-optimized charge
* [ ] Charge decay
* [ ] Charge release action
* [ ] Integral-based gate

Specific mechanics:

* [ ] Integral charge
* [ ] Charge super jump
* [ ] Charge dash
* [ ] Charge beam
* [ ] Risk-route overcharge
* [ ] Gradient-field harvesting
* [ ] Path-length gate
* [ ] Enclosed-area gate

Recommended primitive:

```rust
struct PathAccumulator {
    value: f32,
    decay_rate: f32,
}

impl PathAccumulator {
    fn accumulate(&mut self, position: Vec2, velocity: Vec2, field_value: f32, dt: f32) {
        self.value += field_value * velocity.length() * dt;
    }
}
```

---

# Coordinate / topology mechanics

These mechanics are more invasive, but they can produce highly distinctive rooms and traversal puzzles.

* [ ] Coordinate wrapping
* [ ] Toroidal room topology
* [ ] One-axis wrap
* [ ] Two-axis wrap
* [ ] Portal-like room seams
* [ ] Non-Euclidean room transition
* [ ] Coordinate projection
* [ ] Coordinate folding
* [ ] Coordinate inversion
* [ ] Local coordinate charts
* [ ] Room-space remapping

Specific mechanics:

* [ ] Modular wrap room
* [ ] Pac-Man horizontal wrap
* [ ] Vertical wrap shaft
* [ ] Toroidal projectile arena
* [ ] Mobius-strip room
* [ ] Projection shift
* [ ] Flatten-to-line traversal
* [ ] Folded-space shortcut
* [ ] Coordinate inversion

Recommended primitive:

```rust
trait CoordinateMap {
    fn world_to_local(&self, world: Vec2) -> Vec2;
    fn local_to_world(&self, local: Vec2) -> Vec2;
    fn remap_velocity(&self, velocity: Vec2, position: Vec2) -> Vec2;
}
```

---

# Probability / uncertainty mechanics

These are potentially fun, but should be used carefully. Randomness in required metroidvania traversal can feel unfair unless it is bounded, previewed, deterministic, or optional.

* [ ] Random variable utility layer
* [ ] Seeded deterministic randomness
* [ ] Position uncertainty ellipse
* [ ] Velocity uncertainty cone
* [ ] Probability-weighted blink
* [ ] Variance-reduction upgrade
* [ ] Entropy resource
* [ ] Luck-biased collision outcome
* [ ] Probabilistic projectile spread
* [ ] Deterministic replay support for random mechanics

Specific mechanics:

* [ ] Probability blink
* [ ] Uncertainty dash
* [ ] Quantum dodge
* [ ] Collapse-to-target ability
* [ ] Entropy bomb
* [ ] Variance field

Recommended primitive:

```rust
struct UncertaintyState {
    mean_position: Vec2,
    covariance: Mat2,
    seed: u64,
}
```

---

# Interaction / targeting mechanics

* [x] Basic interact input — `SandboxAction::Interact` (E / F / RB) + double-tap-up; gates door / NPC / save-point
* [x] Door transitions
* [x] Ability pickups
* [x] Chests / pickups / interactables
* [~] Breakables — attack-break variant landed via `Surface` LDtk component; stand-too-long variant pending
* [ ] Aim mode for ranged weapons
* [ ] Twin-stick aiming
* [ ] Lock-on targeting
* [ ] Enemy grab / pull
* [ ] Object pickup / carry / throw
* [ ] Push / pull blocks
* [ ] Switch activation with attacks
* [ ] Pressure plates
* [ ] Physics crates
* [ ] Rideable platforms as controller state
* [ ] Moving platform velocity inheritance
* [ ] Ladders
* [ ] Poles / vines
* [ ] Lifts / elevators as player-carrying surfaces

---

# Time / state manipulation

* [x] Bullet-time-compatible control timing
* [ ] Time stop
* [ ] Time rewind
* [ ] Clone / afterimage replay
* [ ] Slow-time ability affecting world entities
* [ ] Fast-forward local object
* [ ] Freeze enemy
* [ ] Freeze projectile
* [ ] Freeze water / lava into platforms
* [ ] World-state toggle as live mechanic
* [ ] Dimension shift with collision changes

---

# Relativistic / local-time mechanics

This extends ordinary bullet time into local clocks and Lorentz-inspired mechanics. Current bullet-time scaffolding is useful, but true local-time mechanics require per-entity or per-system clocks.

* [~] Bullet time / global slow motion
* [x] Split control clock and simulation clock
* [ ] Per-entity local clocks
* [ ] Per-system local clocks
* [ ] Time dilation zones
* [ ] Velocity-dependent proper time
* [ ] Player-relative time fields
* [ ] Object-relative time fields
* [ ] Time-dilated projectiles
* [ ] Time-dilated platforms
* [ ] Time-dilated enemy AI
* [ ] Time-dilated hazards
* [ ] Time-dilated cooldowns
* [ ] Time-dilated animation
* [ ] Time-dilated audio / VFX
* [ ] Causal delay / finite signal speed
* [ ] Collision reconciliation across different local times
* [ ] Deterministic tests for multi-clock simulation

Specific mechanics:

* [ ] Lorentz dash
* [ ] Time-well room
* [ ] Slow-projectile field
* [ ] Fast-platform field
* [ ] Proper-time puzzle
* [ ] Velocity-aging projectile
* [ ] Causal switch delay
* [ ] Light-cone door
* [ ] Near-light-speed zip

Recommended primitive:

```rust
struct TimeContext {
    real_dt: f32,
    world_dt: f32,
    player_control_dt: f32,
}

struct LocalClock {
    scale: f32,
    proper_time: f32,
    last_dt: f32,
}

enum TimeDomain {
    PlayerControl,
    PlayerPhysics,
    Enemies,
    Projectiles,
    Platforms,
    Hazards,
    Cooldowns,
    Animation,
    Vfx,
    Audio,
}
```

---

# Recommended implementation priorities

The most valuable next backend systems are the ones that unlock whole mechanic families.

## Tier 1: high-leverage movement expressibility

* [x] Explicit `PlayerMode` / `LocomotionState`
* [x] Alternate body shapes
* [x] Collision-safe body resize
* [ ] Generic ray / shape-cast targeting API — partial via `parry2d` shape-cast in `sweep_player_x` / `sweep_player_y`; not yet a public targeting API
* [ ] Surface tangent / normal query — `parry2d` returns normals on hit; not yet plumbed through the snap path (see TODO S "Parry contact-normal")
* [ ] Parametric curve movement backend
* [ ] Collision-safe curve traversal
* [ ] Curve preview renderer

These unlock:

* ✅ Morph ball (landed)
* ✅ Crouch / crawl / slide (landed)
* ~ Ledge grab (sandbox-side; engine promotion pending)
* Grapple targeting
* Tangent dash
* Normal launch
* Functional zip
* Exponential zip
* Sine dash
* Parabolic cast

## Tier 2: mathematical traversal identity

* [ ] Curve basis library
* [ ] Vector-field sampler
* [ ] Scalar-field / gradient sampler
* [ ] Transform volume backend
* [ ] Path-integral accumulator

These unlock:

* Exponential zip
* Vector-field surfing
* Gradient climb
* Gravity wells
* Complex velocity rotation
* Momentum reflection / refraction gates
* Integral charge
* Route-optimized super jumps

## Tier 3: advanced systems

* [ ] Grapple / tether constraint backend
* [ ] Local-clock / per-entity time backend
* [ ] Oscillator / phase backend
* [ ] Coordinate-space remapping backend
* [ ] Deterministic randomness / probability backend

These unlock:

* Swinging grapple
* Lorentz-style local time
* Resonance jump
* Fourier slash
* Modular wrap rooms
* Probability blink
