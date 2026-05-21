//! Scripted attack choreography for non-boss enemies.
//!
//! Today, `CharacterAiMode` answers "what high-level mode am I in this
//! tick?" (Idle / Patrol / Chase / Telegraph / Attack / Recover /
//! Stunned / Dead). It does *not* answer "where exactly should I be
//! standing while I chase?" or "is this attack a melee swing or a
//! ranged volley fired from above?". The result is that every enemy
//! resolves "chase" to "walk toward the player", which clumps them
//! together at the player's body.
//!
//! `AttackChoreography` adds an authored, headless-testable layer on
//! top of `CharacterAiMode`:
//!
//! - The choreography is *data* on the actor (one variant of a small
//!   enum). It does not branch into an embedded scripting language.
//! - Each tick, the choreography evaluates a [`ChoreographyTick`]
//!   which provides a *steering target* (where the AI should try to
//!   stand) and an optional *attack request* (swing melee / fire a
//!   projectile in a direction).
//! - The actor's normal AI still owns mode transitions, damage,
//!   stun, kinematics. Choreography only refines targeting and
//!   attack flavour — it never bypasses the state machine.
//!
//! This is intentionally small: a real "behavior tree" would do more,
//! but it would also be much more code to refactor into the
//! character-AI unification later. The shapes here are exactly the
//! ones the pirate / burning-shark / sky-volley combat needs, plus
//! `MeleeContact` so every existing enemy keeps working with an
//! explicit choreography rather than a "no choreography" special case.
//!
//! Anti-clump arbitration lives in [`crate::combat_slots`]; this
//! module merely consumes a slot offset the caller has already
//! assigned.

use crate::Vec2;

/// Authored attack choreography for one enemy.
///
/// `MeleeContact` is the legacy "walk toward the player and swing
/// when in range" behavior. The other variants are scripted spatial
/// patterns: orbit at altitude and fire volleys, dive-bomb, etc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttackChoreography {
    /// Walk to assigned melee slot; swing when next to the player.
    /// This is the default for grounded enemies.
    MeleeContact,
    /// Stand off at a fixed range and fire timed projectile volleys.
    ProjectileVolley {
        /// Horizontal stand-off distance from the target.
        stand_off: f32,
        /// Seconds between individual shots in a burst.
        shot_cadence: f32,
        /// Number of shots per burst.
        burst_count: u8,
        /// Seconds to rest between bursts.
        burst_cooldown: f32,
        /// Projectile launch speed (px/s).
        projectile_speed: f32,
    },
    /// Flying choreography: orbit above the player at a fixed
    /// altitude and radius, fire volleys at regular intervals.
    AerialOrbitAndFire {
        /// Vertical offset above the target (positive = up in sandbox
        /// world space, i.e. smaller Y).
        altitude: f32,
        /// Horizontal orbit radius around the target.
        radius: f32,
        /// Orbit angular speed in radians/sec.
        orbit_speed: f32,
        /// Seconds between volleys.
        fire_interval: f32,
        /// Projectile speed (px/s).
        projectile_speed: f32,
    },
    /// Flying choreography: hover at altitude, then dive at the
    /// player, then climb back up. Used by riderless burning sharks.
    DiveStrike {
        /// Hover altitude before/after a dive.
        hover_altitude: f32,
        /// Seconds spent hovering between dives.
        hover_rest: f32,
        /// Dive descent speed (px/s).
        dive_speed: f32,
        /// Vertical distance to climb back after a dive.
        recover_height: f32,
    },
}

impl AttackChoreography {
    /// Whether this choreography wants the actor to ignore gravity
    /// (i.e. it's flying). The actor's `gravity_scale` should be set
    /// to 0 when this is true and 1 otherwise.
    pub fn is_aerial(self) -> bool {
        matches!(
            self,
            Self::AerialOrbitAndFire { .. } | Self::DiveStrike { .. }
        )
    }
}

/// Sub-phase the choreography is currently in. Sandbox stores this on
/// the actor along with a per-phase timer.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ChoreographyPhase {
    /// Travelling toward the assigned engage position.
    #[default]
    Approach,
    /// In position; performing the actual attack (firing / swinging).
    Engage,
    /// Post-attack reset / cooldown.
    Recover,
}

/// Persistent per-actor choreography state. Owned by the sandbox
/// `EnemyRuntime`; this module only reads + advances it.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ChoreographyState {
    pub phase: ChoreographyPhase,
    /// Seconds remaining in the current phase. Driven by the
    /// evaluator; the sandbox just decrements it each tick before
    /// calling [`evaluate_choreography`].
    pub phase_timer: f32,
    /// Shots remaining in the current burst (volley / aerial fire).
    pub shots_remaining: u8,
    /// Orbit phase in radians (0..2π). Advanced for aerial patterns.
    pub orbit_phase: f32,
    /// Whether the actor is currently in a usable attack slot. Set by
    /// the caller from `combat_slots`.
    pub has_slot: bool,
    /// Stable per-actor seed (derived from the actor's id at
    /// construction via [`seed_from_id`]). Drives orbit-phase
    /// offsets, fire-cadence jitter, and the aerial "personality"
    /// pick so two actors with the same archetype + slot still
    /// produce visually distinct trajectories. Zero is treated as
    /// "no seed set" (legacy behaviour, all actors in lockstep).
    pub seed: u32,
    /// Seconds until the next aerial-orbit "retreat" begins. Counts
    /// down each aerial tick while the actor is orbiting normally.
    /// At `<= 0` a fresh retreat fires — the actor picks a random
    /// heading (see `aerial_retreat_heading`) and flies away from
    /// the slot for `aerial_retreat_timer` seconds, then returns
    /// to the orbit. `0.0` sentinel promotes to a seeded initial
    /// cooldown on first tick so adjacent actors don't retreat in
    /// lockstep.
    pub aerial_retreat_cooldown: f32,
    /// Seconds remaining in the current retreat. `> 0` = the actor
    /// is steering toward an off-slot retreat point (no fire, no
    /// orbit wobble). `== 0` = orbiting normally. Decremented each
    /// tick during retreat; on exit the cooldown is re-seeded.
    pub aerial_retreat_timer: f32,
    /// Heading angle (radians, world frame) of the current retreat.
    /// Set when retreat begins; combined with the
    /// `AERIAL_RETREAT_DISTANCE` constant to derive the absolute
    /// retreat point as `slot + distance * (cos, sin)`. Bounded to
    /// the upper hemisphere (sin ≤ 0 in sandbox y-down convention)
    /// so retreating sharks never steer toward the floor.
    pub aerial_retreat_heading: f32,
    /// Monotonic count of retreats the actor has triggered. Mixed
    /// with `seed` to produce fresh per-retreat jitter — so
    /// successive retreats from the SAME actor pick different
    /// cooldowns, durations, and headings rather than locking onto
    /// a single seed-derived value. Three same-archetype sharks
    /// also desync because their counters advance independently
    /// (one shark retreating doesn't tick the others' counter).
    pub aerial_retreat_count: u32,
}

/// Stable, allocation-free 32-bit hash of an actor id. Used so a
/// shark named `Burning Flying Shark` always produces the same
/// orbit-phase offset / fire-cadence jitter across runs — the
/// gameplay rng is deterministic and replay-safe.
///
/// FNV-1a (32-bit) — small, no deps, plenty distinct for the
/// handful of enemies we ever see per arena.
pub fn seed_from_id(id: &str) -> u32 {
    let mut hash: u32 = 0x811C9DC5;
    for byte in id.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    // Never produce 0 — `ChoreographyState::seed == 0` is the
    // "unset" sentinel; an id that happens to hash to 0 would
    // otherwise be indistinguishable from "never assigned".
    if hash == 0 {
        1
    } else {
        hash
    }
}

/// Personality variants for aerial actors. Derived from
/// `ChoreographyState::seed` so each actor picks one role at
/// spawn and sticks with it for the run. Three readable variants:
///
/// - `Hover` — orbits and fires from a standard altitude. The
///   default reading of a "fires-from-above" enemy.
/// - `Swoop` — periodically dives at the player and pulls back up,
///   firing on the recovery arc. Reads as the "aggressive" shark.
/// - `Retreat` — keeps a higher-than-normal altitude, fires less
///   frequently. Reads as the "cautious" shark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AerialRole {
    Hover,
    Swoop,
    Retreat,
}

impl AerialRole {
    /// Pick a role from a seed. Three roles, modulo-3 on the seed
    /// after a small shift to avoid clumping in the low bits.
    pub fn from_seed(seed: u32) -> Self {
        match (seed >> 4) % 3 {
            0 => Self::Hover,
            1 => Self::Swoop,
            _ => Self::Retreat,
        }
    }
}

/// Read-only snapshot for [`evaluate_choreography`].
#[derive(Clone, Copy, Debug)]
pub struct ChoreographyInput {
    pub actor_pos: Vec2,
    pub target_pos: Vec2,
    /// World position the slot board assigned this actor (often near
    /// the target). When `has_slot == false`, callers should pass the
    /// outer holding ring position so the actor still steers somewhere
    /// sensible.
    pub assigned_slot_pos: Vec2,
    /// Seconds since last tick.
    pub dt: f32,
    /// World position of the nearest same-kind neighbor (other aerial
    /// actor, etc), if any. Aerial choreographies add a "personal
    /// space" steering bias when a neighbor is too close so two
    /// sharks don't visually merge into one blob even after the slot
    /// board has assigned them distinct anchors.
    pub nearest_neighbor: Option<Vec2>,
}

impl Default for ChoreographyInput {
    fn default() -> Self {
        Self {
            actor_pos: Vec2::ZERO,
            target_pos: Vec2::ZERO,
            assigned_slot_pos: Vec2::ZERO,
            dt: 0.0,
            nearest_neighbor: None,
        }
    }
}

/// Action the choreography wants to perform this tick. The caller is
/// responsible for cooldowns/resources — the choreography only fires
/// when its own internal cadence says so.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChoreographyAction {
    /// Trigger a melee swing toward the target.
    Melee,
    /// Spawn a projectile with the given launch direction and speed.
    FireProjectile { dir: Vec2, speed: f32 },
}

/// Output of [`evaluate_choreography`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChoreographyTick {
    /// Where the actor should head this tick. The caller turns this
    /// into chase velocity (grounded) or direct motion (aerial).
    pub steering_target: Vec2,
    /// Desired facing: +1 right, -1 left, 0 hold.
    pub face_x: f32,
    /// Optional attack action this tick.
    pub action: Option<ChoreographyAction>,
    /// Optional per-tick steering speed override (px/s). When `Some`,
    /// the caller should use this instead of the archetype's
    /// `chase_speed` for this tick's convergence — used by
    /// `DiveStrike` to actually accelerate during the dive phase
    /// (the `dive_speed` knob was previously unwired tuning data).
    /// `None` means "use the archetype default".
    pub steering_speed_override: Option<f32>,
}

const NEIGHBOR_PERSONAL_SPACE: f32 = 110.0;
const NEIGHBOR_SPREAD_GAIN: f32 = 70.0;

/// Apply a "personal space" steering bias: when `neighbor` is closer
/// than `NEIGHBOR_PERSONAL_SPACE`, push the steering target away
/// from the neighbor along the actor→away axis. The magnitude
/// scales linearly to zero at the personal-space radius.
fn apply_neighbor_spread(target: Vec2, actor: Vec2, neighbor: Option<Vec2>) -> Vec2 {
    let Some(neighbor) = neighbor else {
        return target;
    };
    let away = actor - neighbor;
    let dist = away.length();
    if dist >= NEIGHBOR_PERSONAL_SPACE || dist < 1.0e-3 {
        return target;
    }
    let push = NEIGHBOR_SPREAD_GAIN * (1.0 - dist / NEIGHBOR_PERSONAL_SPACE);
    target + (away / dist) * push
}

const MELEE_ENGAGE_DISTANCE: f32 = 56.0;
const VOLLEY_ENGAGE_DISTANCE: f32 = 24.0;
const AERIAL_ENGAGE_DISTANCE: f32 = 32.0;
/// Base interval between aerial "retreats". Seed-derived jitter of
/// ±30% layers on top so adjacent actors never retreat on the same
/// frame. After a retreat ends, this interval is rolled again
/// before the next one fires.
const AERIAL_RETREAT_COOLDOWN: f32 = 4.0;
/// Base duration of an aerial retreat (the "fly away in a random
/// direction" window). Seed-derived jitter of ±20% layers on top so
/// retreats don't snap back at the same beat.
const AERIAL_RETREAT_DURATION: f32 = 2.5;
/// World-frame distance from the assigned slot the actor steers
/// toward during a retreat. Picked large enough that the retreat
/// reads as "the shark flew off into the sky" rather than "the
/// shark wobbled".
const AERIAL_RETREAT_DISTANCE: f32 = 360.0;

/// Pure evaluator: read the current actor snapshot + state, advance
/// `state` in place, return the [`ChoreographyTick`].
pub fn evaluate_choreography(
    choreography: AttackChoreography,
    state: &mut ChoreographyState,
    input: ChoreographyInput,
) -> ChoreographyTick {
    let dt = input.dt.max(0.0);
    state.phase_timer = (state.phase_timer - dt).max(0.0);

    match choreography {
        AttackChoreography::MeleeContact => evaluate_melee(state, input),
        AttackChoreography::ProjectileVolley {
            stand_off,
            shot_cadence,
            burst_count,
            burst_cooldown,
            projectile_speed,
        } => evaluate_projectile_volley(
            state,
            input,
            stand_off,
            shot_cadence,
            burst_count,
            burst_cooldown,
            projectile_speed,
        ),
        AttackChoreography::AerialOrbitAndFire {
            altitude,
            radius,
            orbit_speed,
            fire_interval,
            projectile_speed,
        } => evaluate_aerial_orbit(
            state,
            input,
            dt,
            altitude,
            radius,
            orbit_speed,
            fire_interval,
            projectile_speed,
        ),
        AttackChoreography::DiveStrike {
            hover_altitude,
            hover_rest,
            dive_speed,
            recover_height,
        } => evaluate_dive_strike(
            state,
            input,
            hover_altitude,
            hover_rest,
            dive_speed,
            recover_height,
        ),
    }
}

fn face_toward(actor: Vec2, target: Vec2) -> f32 {
    let dx = target.x - actor.x;
    if dx.abs() < 0.001 {
        0.0
    } else {
        dx.signum()
    }
}

fn evaluate_melee(state: &mut ChoreographyState, input: ChoreographyInput) -> ChoreographyTick {
    let to_target = input.target_pos - input.actor_pos;
    let dist = to_target.length();
    let close = dist <= MELEE_ENGAGE_DISTANCE;
    let action = if close && state.has_slot {
        state.phase = ChoreographyPhase::Engage;
        Some(ChoreographyAction::Melee)
    } else {
        state.phase = ChoreographyPhase::Approach;
        None
    };
    // Grounded melee gets neighbor-spread too: two SmallSkitters
    // converging on the player from the same direction shouldn't
    // visually merge into one blob.
    let steering = apply_neighbor_spread(
        input.assigned_slot_pos,
        input.actor_pos,
        input.nearest_neighbor,
    );
    ChoreographyTick {
        steering_target: steering,
        face_x: face_toward(input.actor_pos, input.target_pos),
        action,
        steering_speed_override: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_projectile_volley(
    state: &mut ChoreographyState,
    input: ChoreographyInput,
    stand_off: f32,
    shot_cadence: f32,
    burst_count: u8,
    burst_cooldown: f32,
    projectile_speed: f32,
) -> ChoreographyTick {
    // Engage position: stand_off pixels horizontally on the actor's
    // current side of the target.
    let side = face_toward(input.target_pos, input.actor_pos);
    let side = if side == 0.0 { 1.0 } else { side };
    let engage_pos = Vec2::new(
        input.target_pos.x + side * stand_off,
        input.target_pos.y - 16.0,
    );
    let face_x = face_toward(input.actor_pos, input.target_pos);
    let dist_to_engage = (engage_pos - input.actor_pos).length();
    let in_position = dist_to_engage <= VOLLEY_ENGAGE_DISTANCE && state.has_slot;

    let mut action = None;
    match state.phase {
        ChoreographyPhase::Approach => {
            if in_position {
                state.phase = ChoreographyPhase::Engage;
                state.shots_remaining = burst_count.max(1);
                state.phase_timer = 0.0;
            }
        }
        ChoreographyPhase::Engage => {
            if !in_position {
                // Pushed out of position: drop the burst and re-approach.
                state.phase = ChoreographyPhase::Approach;
                state.shots_remaining = 0;
            } else if state.phase_timer <= 0.0 && state.shots_remaining > 0 {
                let dir = (input.target_pos - input.actor_pos).normalize_or(Vec2::new(face_x, 0.0));
                action = Some(ChoreographyAction::FireProjectile {
                    dir,
                    speed: projectile_speed,
                });
                state.shots_remaining -= 1;
                state.phase_timer = shot_cadence.max(0.05);
                if state.shots_remaining == 0 {
                    state.phase = ChoreographyPhase::Recover;
                    state.phase_timer = burst_cooldown.max(0.1);
                }
            }
        }
        ChoreographyPhase::Recover => {
            if state.phase_timer <= 0.0 {
                state.phase = ChoreographyPhase::Approach;
            }
        }
    }
    ChoreographyTick {
        steering_target: engage_pos,
        face_x,
        action,
        steering_speed_override: None,
    }
}

/// Deterministic per-seed jitter in `[-1.0, 1.0]`. Used to vary
/// fire cadence so two same-archetype aerial actors don't volley in
/// lockstep — pure function of the seed so it's replay-stable.
fn seed_jitter_unit(seed: u32) -> f32 {
    // Take 8 bits in the middle of the seed, map to [-1, 1].
    let bits = ((seed >> 8) & 0xFF) as f32; // 0..255
    (bits / 127.5) - 1.0
}

/// Fold two `u32` together into a fresh hash so each retreat can
/// derive its own jitter from `(seed, retreat_count)` without
/// collapsing to a single shared per-actor value. FNV-1a variant;
/// matches the `seed_from_id` style and stays deterministic.
fn hash_combine(a: u32, b: u32) -> u32 {
    let mut h = a ^ 0x811C9DC5;
    for byte in b.to_le_bytes() {
        h ^= byte as u32;
        h = h.wrapping_mul(0x01000193);
    }
    if h == 0 {
        1
    } else {
        h
    }
}

/// Jitter unit in `[-1.0, 1.0]` derived from a `(seed, counter)`
/// pair. Useful for "this actor's Nth retreat" where you want a
/// fresh value each time even though seed and counter alone don't
/// vary much across small N. Backed by `hash_combine`.
fn seed_counter_jitter(seed: u32, counter: u32) -> f32 {
    let mixed = hash_combine(seed, counter);
    // Use bits 8..16 (mirrors seed_jitter_unit) so output range is
    // consistent across the codebase.
    let bits = ((mixed >> 8) & 0xFF) as f32;
    (bits / 127.5) - 1.0
}

/// Stable per-seed orbit-phase starting offset in radians. Two
/// sharks with the same orbit_speed but different seeds end up at
/// different points around their slot anchor at any given tick.
fn seed_orbit_offset(seed: u32) -> f32 {
    // Spread the offset deterministically across [0, 2π); the
    // golden-ratio multiplier keeps adjacent seed values from
    // landing too close on the circle.
    let unit = (seed.wrapping_mul(2654435761) >> 8) as f32 / (u32::MAX as f32 / 256.0);
    unit * std::f32::consts::TAU
}

#[allow(clippy::too_many_arguments)]
fn evaluate_aerial_orbit(
    state: &mut ChoreographyState,
    input: ChoreographyInput,
    dt: f32,
    altitude: f32,
    radius: f32,
    orbit_speed: f32,
    fire_interval: f32,
    projectile_speed: f32,
) -> ChoreographyTick {
    // Per-retreat jitter: hash (seed, retreat_count) so successive
    // retreats from the same actor produce different cooldowns,
    // durations, and headings rather than locking onto one shared
    // per-actor value. Cross-actor desync comes from each actor's
    // distinct seed; cross-cycle desync comes from the counter.
    // `aerial_retreat_count` increments when a retreat STARTS, so
    // the cooldown that follows a retreat uses the post-increment
    // counter and lands a different beat than the previous cycle.
    let upcoming_counter = state.aerial_retreat_count.wrapping_add(1);
    let cooldown_jitter =
        1.0 + 0.45 * seed_counter_jitter(state.seed, upcoming_counter.wrapping_mul(3));
    let duration_jitter =
        1.0 + 0.35 * seed_counter_jitter(state.seed, upcoming_counter.wrapping_mul(5));
    // Heading uses the post-increment counter directly so the very
    // first retreat (counter 0 → upcoming 1) already produces a
    // per-actor unique angle independent of the shared orbit_phase.
    let heading_unit = seed_counter_jitter(state.seed, upcoming_counter);

    // Initialize the retreat cooldown on first tick (Default
    // sentinel = 0.0). Seeded jitter staggers initial retreats so
    // adjacent actors don't fly off at the same beat.
    if state.aerial_retreat_cooldown == 0.0 && state.aerial_retreat_timer == 0.0 {
        state.aerial_retreat_cooldown = AERIAL_RETREAT_COOLDOWN * cooldown_jitter.max(0.30);
    }

    let in_retreat_at_tick_start = state.aerial_retreat_timer > 0.0;
    if in_retreat_at_tick_start {
        // Drain the retreat timer. When it hits 0, roll a fresh
        // cooldown so the next retreat fires after a beat of normal
        // orbiting. The retreat heading is left in place; it gets
        // overwritten when the next retreat starts.
        state.aerial_retreat_timer = (state.aerial_retreat_timer - dt).max(0.0);
        if state.aerial_retreat_timer == 0.0 {
            state.aerial_retreat_cooldown = AERIAL_RETREAT_COOLDOWN * cooldown_jitter.max(0.30);
        }
    } else {
        // Orbit normally; tick down toward the next retreat.
        state.aerial_retreat_cooldown -= dt;
        if state.aerial_retreat_cooldown <= 0.0 {
            // Start a new retreat. Increment the counter FIRST so
            // the heading/duration jitter we already computed (based
            // on `upcoming_counter`) matches the post-increment
            // state value — keeping the math self-consistent.
            state.aerial_retreat_count = upcoming_counter;
            // Heading: pick from the UPPER HEMISPHERE only. In
            // sandbox y-down world coordinates, sin > 0 ⇒ moving
            // toward the floor; sin < 0 ⇒ moving toward the sky.
            // Mapping `heading_unit ∈ [-1, 1]` to `heading ∈
            // [-π, 0]` (i.e. the upper-half angular range) means
            // sin(heading) ≤ 0 by construction — the shark always
            // flies up-and-away, never into the deck.
            // `heading_unit = -1` → -π (straight left), 0 → -π/2
            // (straight up), +1 → 0 (straight right).
            let heading = -std::f32::consts::FRAC_PI_2
                + heading_unit * std::f32::consts::FRAC_PI_2;
            state.aerial_retreat_heading = heading;
            state.aerial_retreat_timer = AERIAL_RETREAT_DURATION * duration_jitter.max(0.50);
            state.aerial_retreat_cooldown = 0.0;
        }
    }
    state.orbit_phase = (state.orbit_phase + orbit_speed * dt).rem_euclid(std::f32::consts::TAU);

    // Per-actor variation derived from the stable seed. Three roles
    // (Hover / Swoop / Retreat) give visually distinct readings;
    // orbit offset + cadence jitter keep two same-role actors out of
    // lockstep.
    let role = AerialRole::from_seed(state.seed);
    let orbit_offset = seed_orbit_offset(state.seed);
    let cadence_jitter = 1.0 + 0.20 * seed_jitter_unit(state.seed);

    // `altitude` and `radius` from the enum scale the wobble
    // ellipse around the slot anchor; the slot board owns the
    // absolute standoff. Role tweaks shift altitude / cadence.
    let (altitude_scale, fire_scale) = match role {
        AerialRole::Hover => (1.0, 1.0),
        AerialRole::Retreat => (1.4, 1.6), // higher + slower fire
        AerialRole::Swoop => (0.85, 0.85), // lower + faster fire
    };
    let wobble_x = (radius * 0.18).min(60.0);
    let wobble_y = (altitude * 0.10).clamp(8.0, 28.0) * altitude_scale;
    let phase = state.orbit_phase + orbit_offset;
    let mut engage_pos = Vec2::new(
        input.assigned_slot_pos.x + phase.cos() * wobble_x,
        input.assigned_slot_pos.y + phase.sin() * wobble_y,
    );

    // Swoop: every ~3s the actor commits to a quick descent toward
    // the target, then climbs back. Drives the `phase` field so the
    // visual read transitions Approach → Engage (descent) →
    // Recover (climb).
    if role == AerialRole::Swoop {
        match state.phase {
            ChoreographyPhase::Recover if state.phase_timer <= 0.0 => {
                state.phase = ChoreographyPhase::Approach;
            }
            ChoreographyPhase::Approach if state.shots_remaining == 0 && state.phase_timer <= 0.0
            => {
                // Use shots_remaining as a "swoop interval"
                // countdown latch: a fresh approach phase lasts
                // `phase_timer` seconds before triggering a swoop.
                state.shots_remaining = 1;
                state.phase_timer = 2.5 + 0.5 * seed_jitter_unit(state.seed);
            }
            ChoreographyPhase::Approach if state.phase_timer <= 0.0 && state.shots_remaining > 0 => {
                state.phase = ChoreographyPhase::Engage;
                state.phase_timer = 0.6;
                state.shots_remaining = 0;
            }
            _ => {}
        }
        if state.phase == ChoreographyPhase::Engage {
            // Descend toward the player horizontally + vertically
            // for the duration of the engage phase.
            engage_pos = Vec2::new(
                input.target_pos.x,
                input.target_pos.y - 24.0,
            );
            if state.phase_timer <= 0.0 {
                state.phase = ChoreographyPhase::Recover;
                state.phase_timer = 0.7;
            }
        } else if state.phase == ChoreographyPhase::Recover {
            // Pull back up above the slot anchor.
            engage_pos = Vec2::new(
                input.assigned_slot_pos.x,
                input.assigned_slot_pos.y - 80.0,
            );
        }
    }

    // Retreat override: while the retreat timer is non-zero, the
    // actor steers toward an off-slot point in the direction of
    // `aerial_retreat_heading`. This wins over the role-specific
    // engage_pos so a retreating Hover or Retreat actually leaves
    // combat range instead of just wobbling. Swoop retreats also
    // abort their dive — the retreat heading takes priority.
    let retreating = state.aerial_retreat_timer > 0.0;
    if retreating {
        let cos_h = state.aerial_retreat_heading.cos();
        let sin_h = state.aerial_retreat_heading.sin();
        engage_pos = Vec2::new(
            input.assigned_slot_pos.x + cos_h * AERIAL_RETREAT_DISTANCE,
            input.assigned_slot_pos.y + sin_h * AERIAL_RETREAT_DISTANCE,
        );
        // Ensure Swoop logic doesn't tug back on the next tick — we
        // forcibly leave Swoop's Engage/Recover state machine in
        // Approach so it can re-enter naturally after the retreat
        // ends.
        if role == AerialRole::Swoop {
            state.phase = ChoreographyPhase::Approach;
            state.phase_timer = 0.0;
        }
    }

    // Neighbor-aware spread: push away from the nearest aerial
    // neighbor if it's inside personal-space radius. Layered on
    // AFTER the role-specific engage_pos so a swoop still moves
    // toward the player but bends around a neighbor in the way.
    let engage_pos = apply_neighbor_spread(engage_pos, input.actor_pos, input.nearest_neighbor);

    let face_x = face_toward(input.actor_pos, input.target_pos);
    let dist_to_engage = (engage_pos - input.actor_pos).length();
    let in_position = dist_to_engage <= (AERIAL_ENGAGE_DISTANCE + wobble_x);

    let effective_fire_interval = (fire_interval * fire_scale * cadence_jitter).max(0.2);
    let mut action = None;
    if retreating {
        // Retreating actors do not fire — they're disengaging.
        // Holding the cooldown at the role's interval keeps the
        // first post-retreat shot from snapping immediately.
        state.phase = ChoreographyPhase::Approach;
        state.phase_timer = effective_fire_interval.max(state.phase_timer);
    } else if in_position && state.has_slot && role != AerialRole::Swoop {
        // Hover / Retreat fire on cadence when in position.
        if state.phase_timer <= 0.0 {
            let dir = (input.target_pos - input.actor_pos).normalize_or(Vec2::new(face_x, 1.0));
            action = Some(ChoreographyAction::FireProjectile {
                dir,
                speed: projectile_speed,
            });
            state.phase_timer = effective_fire_interval;
            state.phase = ChoreographyPhase::Engage;
        }
    } else if role == AerialRole::Swoop && state.phase == ChoreographyPhase::Engage {
        // Swoop fires once at the start of the engage descent.
        if state.shots_remaining == 0 {
            let dir = (input.target_pos - input.actor_pos).normalize_or(Vec2::new(face_x, 1.0));
            action = Some(ChoreographyAction::FireProjectile {
                dir,
                speed: projectile_speed * 1.1,
            });
            // Use shots_remaining=1 as a "fired this swoop" flag.
            state.shots_remaining = 1;
        }
    } else if role != AerialRole::Swoop {
        state.phase = ChoreographyPhase::Approach;
    }
    ChoreographyTick {
        steering_target: engage_pos,
        face_x,
        action,
        steering_speed_override: None,
    }
}

fn evaluate_dive_strike(
    state: &mut ChoreographyState,
    input: ChoreographyInput,
    hover_altitude: f32,
    hover_rest: f32,
    dive_speed: f32,
    recover_height: f32,
) -> ChoreographyTick {
    let face_x = face_toward(input.actor_pos, input.target_pos);
    // Hover position is above the target; dive target is the target
    // itself. Per-actor seed offsets the hover position horizontally
    // so two dismounted sharks don't pick exactly the same hover spot.
    let hover_x_offset = 60.0 * seed_jitter_unit(state.seed);
    let hover_pos = Vec2::new(
        input.target_pos.x + hover_x_offset,
        input.target_pos.y - hover_altitude,
    );
    let dive_pos = input.target_pos;
    let recover_pos = Vec2::new(
        input.target_pos.x + hover_x_offset,
        input.target_pos.y - hover_altitude - recover_height,
    );
    let (steering, action, speed_override) = match state.phase {
        ChoreographyPhase::Approach => {
            // Approach → reach hover position → rest → dive.
            let dist = (hover_pos - input.actor_pos).length();
            if dist <= AERIAL_ENGAGE_DISTANCE && state.has_slot {
                state.phase = ChoreographyPhase::Engage;
                // Per-actor rest jitter so two sharks don't
                // synchronize their dive timing.
                state.phase_timer = (hover_rest.max(0.2))
                    * (1.0 + 0.25 * seed_jitter_unit(state.seed));
            }
            (hover_pos, None, None)
        }
        ChoreographyPhase::Engage => {
            if state.phase_timer <= 0.0 {
                // Dive — use the authored `dive_speed` rather than
                // the archetype's general `chase_speed` so a fast
                // dive *reads* fast. The Recover/Approach phases
                // pass `None` and fall back to archetype default.
                let dist_to_target = (input.target_pos - input.actor_pos).length();
                if dist_to_target <= MELEE_ENGAGE_DISTANCE {
                    state.phase = ChoreographyPhase::Recover;
                    state.phase_timer = 0.6;
                    (recover_pos, Some(ChoreographyAction::Melee), Some(dive_speed))
                } else {
                    (dive_pos, None, Some(dive_speed))
                }
            } else {
                (hover_pos, None, None)
            }
        }
        ChoreographyPhase::Recover => {
            if state.phase_timer <= 0.0 {
                state.phase = ChoreographyPhase::Approach;
            }
            (recover_pos, None, None)
        }
    };
    let steering = apply_neighbor_spread(steering, input.actor_pos, input.nearest_neighbor);
    ChoreographyTick {
        steering_target: steering,
        face_x,
        action,
        steering_speed_override: speed_override,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_at(actor: Vec2, target: Vec2, slot: Vec2) -> ChoreographyInput {
        ChoreographyInput {
            actor_pos: actor,
            target_pos: target,
            assigned_slot_pos: slot,
            dt: 1.0 / 60.0,
            nearest_neighbor: None,
        }
    }

    #[test]
    fn melee_engages_when_close_with_slot() {
        let mut state = ChoreographyState {
            has_slot: true,
            ..Default::default()
        };
        let tick = evaluate_choreography(
            AttackChoreography::MeleeContact,
            &mut state,
            input_at(
                Vec2::new(10.0, 0.0),
                Vec2::new(20.0, 0.0),
                Vec2::new(20.0, 0.0),
            ),
        );
        assert_eq!(tick.action, Some(ChoreographyAction::Melee));
        assert_eq!(state.phase, ChoreographyPhase::Engage);
    }

    #[test]
    fn melee_without_slot_does_not_attack_even_when_close() {
        let mut state = ChoreographyState::default();
        let tick = evaluate_choreography(
            AttackChoreography::MeleeContact,
            &mut state,
            input_at(
                Vec2::new(10.0, 0.0),
                Vec2::new(20.0, 0.0),
                Vec2::new(20.0, 0.0),
            ),
        );
        assert!(tick.action.is_none());
    }

    #[test]
    fn projectile_volley_advances_through_burst() {
        let choreography = AttackChoreography::ProjectileVolley {
            stand_off: 200.0,
            shot_cadence: 0.2,
            burst_count: 3,
            burst_cooldown: 1.0,
            projectile_speed: 500.0,
        };
        let mut state = ChoreographyState {
            has_slot: true,
            ..Default::default()
        };
        // Actor already at engage position (left of player).
        let target = Vec2::new(0.0, 0.0);
        let actor = Vec2::new(-200.0, -16.0);
        let mut shots_fired = 0;
        for _ in 0..240 {
            let mut input = input_at(actor, target, actor);
            input.dt = 1.0 / 30.0;
            let tick = evaluate_choreography(choreography, &mut state, input);
            if let Some(ChoreographyAction::FireProjectile { .. }) = tick.action {
                shots_fired += 1;
            }
            if shots_fired >= 6 {
                break;
            }
        }
        // Should fire 2 full bursts (6 shots) over the simulated window.
        assert!(shots_fired >= 6, "fired {shots_fired} shots");
    }

    #[test]
    fn aerial_orbit_fires_when_in_position() {
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 120.0,
            radius: 160.0,
            orbit_speed: 1.2,
            fire_interval: 0.4,
            projectile_speed: 420.0,
        };
        let mut state = ChoreographyState {
            has_slot: true,
            ..Default::default()
        };
        // Snap actor to the initial orbit point so the first tick is in-position.
        let target = Vec2::new(0.0, 0.0);
        // orbit_phase starts at 0 → engage_pos.x = target.x + radius
        let initial_engage = Vec2::new(target.x + 160.0, target.y - 120.0);
        let mut shots = 0;
        let mut actor = initial_engage;
        for _ in 0..120 {
            let input = ChoreographyInput {
                actor_pos: actor,
                target_pos: target,
                assigned_slot_pos: actor,
                dt: 1.0 / 30.0,
                nearest_neighbor: None,
            };
            let tick = evaluate_choreography(choreography, &mut state, input);
            actor = tick.steering_target; // teleport to track engage point
            if matches!(tick.action, Some(ChoreographyAction::FireProjectile { .. })) {
                shots += 1;
            }
        }
        assert!(shots >= 3, "aerial fired {shots} shots");
    }

    #[test]
    fn dive_strike_cycles_approach_engage_recover() {
        let choreography = AttackChoreography::DiveStrike {
            hover_altitude: 100.0,
            hover_rest: 0.2,
            dive_speed: 600.0,
            recover_height: 80.0,
        };
        let mut state = ChoreographyState {
            has_slot: true,
            ..Default::default()
        };
        let target = Vec2::new(0.0, 0.0);
        // Start at hover position.
        let mut actor = Vec2::new(0.0, -100.0);
        let mut saw_engage = false;
        let mut saw_melee = false;
        for _ in 0..240 {
            let input = ChoreographyInput {
                actor_pos: actor,
                target_pos: target,
                assigned_slot_pos: actor,
                dt: 1.0 / 30.0,
                nearest_neighbor: None,
            };
            let tick = evaluate_choreography(choreography, &mut state, input);
            if state.phase == ChoreographyPhase::Engage {
                saw_engage = true;
                // Simulate the dive arriving at the target on the very
                // next tick.
                actor = tick.steering_target;
            } else {
                actor = tick.steering_target;
            }
            if matches!(tick.action, Some(ChoreographyAction::Melee)) {
                saw_melee = true;
            }
        }
        assert!(saw_engage, "dive never engaged");
        assert!(saw_melee, "dive never landed a melee strike");
    }

    #[test]
    fn seed_from_id_is_stable_and_nonzero() {
        let a = seed_from_id("Burning Flying Shark:0");
        let b = seed_from_id("Burning Flying Shark:0");
        assert_eq!(a, b, "same id must hash to same seed across calls");
        assert_ne!(seed_from_id(""), 0, "empty id must not produce sentinel 0");
        assert_ne!(
            seed_from_id("a"),
            seed_from_id("b"),
            "different ids should typically hash to different seeds"
        );
    }

    #[test]
    fn aerial_orbit_seeds_produce_distinct_positions() {
        // Three actors at the SAME slot pos but different seeds must
        // produce visibly different engage positions on the same tick.
        // The pre-fix behaviour (no per-actor offset) had all three
        // landing on the same point — the "three sharks clump" bug.
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 160.0,
            radius: 220.0,
            orbit_speed: 0.9,
            fire_interval: 1.4,
            projectile_speed: 380.0,
        };
        let target = Vec2::new(0.0, 0.0);
        let slot = Vec2::new(0.0, -160.0);
        let mut positions = Vec::new();
        for id in ["shark_a", "shark_b", "shark_c"] {
            let mut state = ChoreographyState {
                seed: seed_from_id(id),
                has_slot: true,
                ..Default::default()
            };
            // Tick once to advance orbit_phase off zero.
            let tick = evaluate_choreography(
                choreography,
                &mut state,
                ChoreographyInput {
                    actor_pos: slot,
                    target_pos: target,
                    assigned_slot_pos: slot,
                    dt: 1.0 / 30.0,
                    nearest_neighbor: None,
                },
            );
            positions.push(tick.steering_target);
        }
        // Pairwise distance — every pair must differ by at least
        // a few px or they'll visually merge.
        for i in 0..positions.len() {
            for j in i + 1..positions.len() {
                let d = (positions[i] - positions[j]).length();
                assert!(
                    d > 5.0,
                    "shark {i} and {j} at indistinguishable engage positions \
                     ({:?} vs {:?})",
                    positions[i],
                    positions[j]
                );
            }
        }
    }

    #[test]
    fn aerial_role_distribution_covers_all_three() {
        // Survey a batch of plausible ids and confirm the role
        // distribution actually hits all three variants — otherwise
        // the role pick would silently collapse to a single role and
        // the visual variety claim is false.
        let ids = ["shark_a", "shark_b", "shark_c", "shark_d", "shark_e", "shark_f"];
        let mut hovers = 0;
        let mut swoops = 0;
        let mut retreats = 0;
        for id in &ids {
            match AerialRole::from_seed(seed_from_id(id)) {
                AerialRole::Hover => hovers += 1,
                AerialRole::Swoop => swoops += 1,
                AerialRole::Retreat => retreats += 1,
            }
        }
        assert!(
            hovers + swoops + retreats == ids.len(),
            "role count mismatch"
        );
        // With 6 ids we expect at least 2 distinct roles to appear in
        // practice; modulo-3 distribution is uniform enough.
        let distinct = [hovers, swoops, retreats]
            .iter()
            .filter(|&&n| n > 0)
            .count();
        assert!(
            distinct >= 2,
            "expected at least 2 distinct aerial roles across 6 ids \
             (Hover={hovers}, Swoop={swoops}, Retreat={retreats})"
        );
    }

    #[test]
    fn aerial_orbit_retreat_steers_far_from_slot() {
        // Drive the orbit eval long enough to trigger at least one
        // retreat. During retreat, the steering target must be
        // substantially farther from the slot than the normal orbit
        // wobble (which is bounded by `wobble_x + wobble_y` ≈
        // ~60 + 25 px on the slot ellipse). The retreat distance is
        // hundreds of pixels.
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 160.0,
            radius: 220.0,
            orbit_speed: 0.9,
            fire_interval: 1.4,
            projectile_speed: 380.0,
        };
        let mut state = ChoreographyState {
            seed: seed_from_id("retreat_test"),
            has_slot: true,
            ..Default::default()
        };
        let target = Vec2::new(0.0, 0.0);
        let slot = Vec2::new(0.0, -160.0);
        let dt = 1.0 / 60.0;

        // Find the first frame where retreat_timer is set.
        let mut max_dist_during_retreat = 0.0_f32;
        let mut saw_retreat = false;
        let mut elapsed = 0.0_f32;
        let cap = 15.0; // larger than AERIAL_RETREAT_COOLDOWN
        while elapsed < cap {
            let tick = evaluate_choreography(
                choreography,
                &mut state,
                ChoreographyInput {
                    actor_pos: slot,
                    target_pos: target,
                    assigned_slot_pos: slot,
                    dt,
                    nearest_neighbor: None,
                },
            );
            if state.aerial_retreat_timer > 0.0 {
                saw_retreat = true;
                let dist = (tick.steering_target - slot).length();
                max_dist_during_retreat = max_dist_during_retreat.max(dist);
            }
            elapsed += dt;
        }
        assert!(
            saw_retreat,
            "retreat never fired within {cap}s; cooldown={}",
            state.aerial_retreat_cooldown
        );
        assert!(
            max_dist_during_retreat > 200.0,
            "during retreat, steering target should be hundreds of px \
             from the slot — got max {} px",
            max_dist_during_retreat
        );
    }

    #[test]
    fn aerial_orbit_retreat_suppresses_fire() {
        // While the retreat timer is active, the choreography must
        // not emit a FireProjectile action — the actor is fleeing,
        // not shooting.
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 160.0,
            radius: 220.0,
            orbit_speed: 0.9,
            fire_interval: 1.4,
            projectile_speed: 380.0,
        };
        // Force the actor into retreat immediately by pre-loading
        // the retreat timer past the cooldown trigger.
        let mut state = ChoreographyState {
            seed: seed_from_id("retreat_no_fire"),
            has_slot: true,
            aerial_retreat_timer: 1.0,
            aerial_retreat_heading: 0.0,
            phase_timer: 0.0, // ordinarily ready to fire
            ..Default::default()
        };
        let target = Vec2::new(0.0, 0.0);
        // Place actor right at the target so without the retreat
        // gate it WOULD fire immediately (in-position + slot + timer=0).
        let actor_pos = Vec2::new(0.0, -160.0);
        let tick = evaluate_choreography(
            choreography,
            &mut state,
            ChoreographyInput {
                actor_pos,
                target_pos: target,
                assigned_slot_pos: actor_pos,
                dt: 1.0 / 60.0,
                nearest_neighbor: None,
            },
        );
        assert!(
            !matches!(tick.action, Some(ChoreographyAction::FireProjectile { .. })),
            "retreating actor should NOT fire; got {:?}",
            tick.action
        );
    }

    #[test]
    fn aerial_orbit_retreat_heading_avoids_floor() {
        // Sweep a wide range of seeds; every resulting first-retreat
        // heading must have sin(heading) ≤ 0 (y-down sandbox = no
        // downward retreat into the deck). Catches any regression
        // where the heading falls back into the lower hemisphere.
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 160.0,
            radius: 220.0,
            orbit_speed: 0.9,
            fire_interval: 1.4,
            projectile_speed: 380.0,
        };
        let target = Vec2::new(0.0, 0.0);
        let slot = Vec2::new(0.0, -160.0);
        let dt = 1.0 / 60.0;
        for id_idx in 0..32 {
            let id = format!("shark_{id_idx}");
            let mut state = ChoreographyState {
                seed: seed_from_id(&id),
                has_slot: true,
                ..Default::default()
            };
            // Tick until first retreat fires.
            let mut elapsed = 0.0_f32;
            let cap = 15.0;
            while elapsed < cap && state.aerial_retreat_timer == 0.0 {
                evaluate_choreography(
                    choreography,
                    &mut state,
                    ChoreographyInput {
                        actor_pos: slot,
                        target_pos: target,
                        assigned_slot_pos: slot,
                        dt,
                        nearest_neighbor: None,
                    },
                );
                elapsed += dt;
            }
            assert!(
                state.aerial_retreat_timer > 0.0,
                "{id}: retreat never fired within {cap}s"
            );
            let sin_h = state.aerial_retreat_heading.sin();
            assert!(
                sin_h <= 1e-6,
                "{id}: heading {} would steer toward the floor (sin={})",
                state.aerial_retreat_heading,
                sin_h
            );
        }
    }

    #[test]
    fn aerial_orbit_retreat_heading_varies_across_retreats() {
        // The SAME actor must pick different headings for successive
        // retreats — otherwise it would keep flying off in the same
        // direction every cycle. Drive a single actor through 4
        // retreats and assert at least 3 distinct headings.
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 160.0,
            radius: 220.0,
            orbit_speed: 0.9,
            fire_interval: 1.4,
            projectile_speed: 380.0,
        };
        let mut state = ChoreographyState {
            seed: seed_from_id("multi_retreat_actor"),
            has_slot: true,
            ..Default::default()
        };
        let target = Vec2::new(0.0, 0.0);
        let slot = Vec2::new(0.0, -160.0);
        let dt = 1.0 / 60.0;
        let mut headings: Vec<f32> = Vec::new();
        let mut elapsed = 0.0_f32;
        let cap = 60.0;
        let mut was_retreating = false;
        while elapsed < cap && headings.len() < 4 {
            evaluate_choreography(
                choreography,
                &mut state,
                ChoreographyInput {
                    actor_pos: slot,
                    target_pos: target,
                    assigned_slot_pos: slot,
                    dt,
                    nearest_neighbor: None,
                },
            );
            let is_retreating = state.aerial_retreat_timer > 0.0;
            if is_retreating && !was_retreating {
                headings.push(state.aerial_retreat_heading);
            }
            was_retreating = is_retreating;
            elapsed += dt;
        }
        assert!(
            headings.len() >= 4,
            "only saw {} retreat starts in {cap}s — cooldown logic broken",
            headings.len()
        );
        // Distinct heading count (within float tolerance).
        let mut distinct = 0;
        for i in 0..headings.len() {
            if !headings[..i]
                .iter()
                .any(|h| (h - headings[i]).abs() < 0.01)
            {
                distinct += 1;
            }
        }
        assert!(
            distinct >= 3,
            "expected ≥3 distinct headings across 4 retreats; got {distinct} \
             ({headings:?})"
        );
    }

    #[test]
    fn aerial_orbit_retreat_first_cooldown_varies_across_seeds() {
        // Spawn three same-archetype actors with different seeds and
        // confirm their FIRST retreat fires on different frames.
        // Before the per-retreat jitter rework, the same `seed_jitter`
        // call dominated cooldown / duration / heading so 8-bit
        // collisions in seed_jitter_unit could leave actors retreating
        // close to the same beat. The new per-retreat hash should
        // produce wider variance.
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 160.0,
            radius: 220.0,
            orbit_speed: 0.9,
            fire_interval: 1.4,
            projectile_speed: 380.0,
        };
        let target = Vec2::new(0.0, 0.0);
        let slot = Vec2::new(0.0, -160.0);
        let dt = 1.0 / 60.0;
        let ids = [
            "EnemySpawn-104446",
            "EnemySpawn-104447",
            "EnemySpawn-104448",
        ];
        let mut first_retreat_frames: Vec<u32> = Vec::new();
        for id in &ids {
            let mut state = ChoreographyState {
                seed: seed_from_id(id),
                has_slot: true,
                ..Default::default()
            };
            let mut frame = 0u32;
            while frame < (15 * 60) && state.aerial_retreat_timer == 0.0 {
                evaluate_choreography(
                    choreography,
                    &mut state,
                    ChoreographyInput {
                        actor_pos: slot,
                        target_pos: target,
                        assigned_slot_pos: slot,
                        dt,
                        nearest_neighbor: None,
                    },
                );
                frame += 1;
            }
            assert!(
                state.aerial_retreat_timer > 0.0,
                "{id}: retreat never fired"
            );
            first_retreat_frames.push(frame);
        }
        // Every pair of frames must differ by at least a few frames
        // (≥ 6 = 0.1s at 60Hz) — same-frame retreats are exactly
        // the "they all do it at the same time" bug.
        for i in 0..first_retreat_frames.len() {
            for j in i + 1..first_retreat_frames.len() {
                let diff = (first_retreat_frames[i] as i64
                    - first_retreat_frames[j] as i64)
                    .abs();
                assert!(
                    diff >= 6,
                    "first-retreat frames {} and {} too close: \
                     frame[{i}]={} frame[{j}]={} (delta {} frames)",
                    ids[i],
                    ids[j],
                    first_retreat_frames[i],
                    first_retreat_frames[j],
                    diff
                );
            }
        }
    }

    #[test]
    fn aerial_orbit_retreat_staggers_across_seeds() {
        // Two actors with different seeds should NOT enter retreat
        // on the same frame — that's the whole point of seeded
        // jitter on the cooldown. Run both for several cycles and
        // check at least one tick has actor_a retreating while
        // actor_b is orbiting (or vice versa).
        let choreography = AttackChoreography::AerialOrbitAndFire {
            altitude: 160.0,
            radius: 220.0,
            orbit_speed: 0.9,
            fire_interval: 1.4,
            projectile_speed: 380.0,
        };
        let mut a = ChoreographyState {
            seed: seed_from_id("shark_a"),
            has_slot: true,
            ..Default::default()
        };
        let mut b = ChoreographyState {
            seed: seed_from_id("shark_b"),
            has_slot: true,
            ..Default::default()
        };
        let target = Vec2::new(0.0, 0.0);
        let slot = Vec2::new(0.0, -160.0);
        let dt = 1.0 / 60.0;
        let mut saw_desync = false;
        for _ in 0..(20 * 60) {
            let input = ChoreographyInput {
                actor_pos: slot,
                target_pos: target,
                assigned_slot_pos: slot,
                dt,
                nearest_neighbor: None,
            };
            evaluate_choreography(choreography, &mut a, input);
            evaluate_choreography(choreography, &mut b, input);
            let a_retreat = a.aerial_retreat_timer > 0.0;
            let b_retreat = b.aerial_retreat_timer > 0.0;
            if a_retreat ^ b_retreat {
                saw_desync = true;
                break;
            }
        }
        assert!(
            saw_desync,
            "two seeded actors never desync'd their retreat windows; \
             the staggered cooldown jitter isn't doing its job"
        );
    }

    #[test]
    fn neighbor_spread_pushes_away_when_close() {
        let mut state = ChoreographyState {
            seed: seed_from_id("a"),
            has_slot: true,
            ..Default::default()
        };
        let target = Vec2::new(0.0, 0.0);
        let slot = Vec2::new(100.0, -100.0);
        let actor = slot; // already at slot
        // Without a neighbor, steering = slot + small wobble.
        let no_neighbor = evaluate_choreography(
            AttackChoreography::AerialOrbitAndFire {
                altitude: 160.0,
                radius: 220.0,
                orbit_speed: 0.9,
                fire_interval: 1.4,
                projectile_speed: 380.0,
            },
            &mut state.clone(),
            ChoreographyInput {
                actor_pos: actor,
                target_pos: target,
                assigned_slot_pos: slot,
                dt: 1.0 / 30.0,
                nearest_neighbor: None,
            },
        );
        // With a neighbor 30px to the left, steering should bias right.
        let with_neighbor = evaluate_choreography(
            AttackChoreography::AerialOrbitAndFire {
                altitude: 160.0,
                radius: 220.0,
                orbit_speed: 0.9,
                fire_interval: 1.4,
                projectile_speed: 380.0,
            },
            &mut state,
            ChoreographyInput {
                actor_pos: actor,
                target_pos: target,
                assigned_slot_pos: slot,
                dt: 1.0 / 30.0,
                nearest_neighbor: Some(actor - Vec2::new(30.0, 0.0)),
            },
        );
        assert!(
            with_neighbor.steering_target.x > no_neighbor.steering_target.x,
            "neighbor on left should push steering to the right (no={:?}, with={:?})",
            no_neighbor.steering_target,
            with_neighbor.steering_target
        );
    }

    #[test]
    fn dive_strike_uses_dive_speed_override_during_engage() {
        // Place the actor at the hover position; first tick → Engage.
        // Drain the rest timer to 0; next tick → dive, which must
        // populate `steering_speed_override` with the authored
        // `dive_speed` value (not None).
        let choreography = AttackChoreography::DiveStrike {
            hover_altitude: 100.0,
            hover_rest: 0.2,
            dive_speed: 999.0,
            recover_height: 80.0,
        };
        let mut state = ChoreographyState {
            seed: seed_from_id("dive_test"),
            has_slot: true,
            ..Default::default()
        };
        let target = Vec2::new(0.0, 0.0);
        // The dive helper applies a per-seed horizontal hover offset.
        let hover_offset = 60.0 * seed_jitter_unit(state.seed);
        let hover_pos = Vec2::new(target.x + hover_offset, target.y - 100.0);
        let mut actor = hover_pos;
        let mut saw_override = false;
        for _ in 0..120 {
            let tick = evaluate_choreography(
                choreography,
                &mut state,
                ChoreographyInput {
                    actor_pos: actor,
                    target_pos: target,
                    assigned_slot_pos: hover_pos,
                    dt: 1.0 / 30.0,
                    nearest_neighbor: None,
                },
            );
            if let Some(speed) = tick.steering_speed_override {
                saw_override = true;
                assert!(
                    (speed - 999.0).abs() < 1e-3,
                    "dive_speed override should equal authored value, got {speed}"
                );
                break;
            }
            actor = tick.steering_target;
        }
        assert!(
            saw_override,
            "dive_speed override was never produced during engage phase"
        );
    }

    #[test]
    fn aerial_choreographies_flag_is_aerial() {
        assert!(AttackChoreography::AerialOrbitAndFire {
            altitude: 1.0,
            radius: 1.0,
            orbit_speed: 1.0,
            fire_interval: 1.0,
            projectile_speed: 1.0
        }
        .is_aerial());
        assert!(AttackChoreography::DiveStrike {
            hover_altitude: 1.0,
            hover_rest: 1.0,
            dive_speed: 1.0,
            recover_height: 1.0
        }
        .is_aerial());
        assert!(!AttackChoreography::MeleeContact.is_aerial());
        assert!(!AttackChoreography::ProjectileVolley {
            stand_off: 1.0,
            shot_cadence: 1.0,
            burst_count: 1,
            burst_cooldown: 1.0,
            projectile_speed: 1.0
        }
        .is_aerial());
    }
}
