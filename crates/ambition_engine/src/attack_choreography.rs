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
}

const MELEE_ENGAGE_DISTANCE: f32 = 56.0;
const VOLLEY_ENGAGE_DISTANCE: f32 = 24.0;
const AERIAL_ENGAGE_DISTANCE: f32 = 32.0;

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
            dive_speed: _,
            recover_height,
        } => evaluate_dive_strike(state, input, hover_altitude, hover_rest, recover_height),
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
    ChoreographyTick {
        steering_target: input.assigned_slot_pos,
        face_x: face_toward(input.actor_pos, input.target_pos),
        action,
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
    }
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
    state.orbit_phase = (state.orbit_phase + orbit_speed * dt) % std::f32::consts::TAU;
    // Spatial anchor is the slot-board-assigned position, NOT the
    // player. The slot board has already spread aerial actors across
    // a wide arc above the target, so anchoring here is what keeps
    // sharks from clumping at a single orbit point. The choreography
    // adds a small per-actor wobble around its slot anchor so the
    // sprite still reads as "hovering / circling" rather than static.
    //
    // `altitude` and `radius` from the enum no longer set the absolute
    // standoff (the slot board owns that). They now scale the wobble
    // ellipse so aggressive aerial archetypes can still feel jittery
    // while tank-flyers feel anchored. The horizontal wobble caps at
    // a fraction of `radius` so neighboring slots don't overlap.
    let wobble_x = (radius * 0.18).min(60.0);
    let wobble_y = (altitude * 0.10).min(28.0).max(8.0);
    let engage_pos = Vec2::new(
        input.assigned_slot_pos.x + state.orbit_phase.cos() * wobble_x,
        input.assigned_slot_pos.y + state.orbit_phase.sin() * wobble_y,
    );
    let face_x = face_toward(input.actor_pos, input.target_pos);
    let dist_to_engage = (engage_pos - input.actor_pos).length();
    // Wider engage threshold for aerial actors: they're flying, not
    // settling on a tile, and the wobble keeps the target moving
    // ±wobble_x px every tick — a tight threshold would have the
    // shark forever "approaching" and never firing.
    let in_position = dist_to_engage <= (AERIAL_ENGAGE_DISTANCE + wobble_x);

    let mut action = None;
    if in_position && state.has_slot {
        if state.phase_timer <= 0.0 {
            let dir = (input.target_pos - input.actor_pos).normalize_or(Vec2::new(face_x, 1.0));
            action = Some(ChoreographyAction::FireProjectile {
                dir,
                speed: projectile_speed,
            });
            state.phase_timer = fire_interval.max(0.2);
            state.phase = ChoreographyPhase::Engage;
        }
    } else {
        state.phase = ChoreographyPhase::Approach;
    }
    ChoreographyTick {
        steering_target: engage_pos,
        face_x,
        action,
    }
}

fn evaluate_dive_strike(
    state: &mut ChoreographyState,
    input: ChoreographyInput,
    hover_altitude: f32,
    hover_rest: f32,
    recover_height: f32,
) -> ChoreographyTick {
    let face_x = face_toward(input.actor_pos, input.target_pos);
    // Hover position is above the target; dive target is the target
    // itself.
    let hover_pos = Vec2::new(input.target_pos.x, input.target_pos.y - hover_altitude);
    let dive_pos = input.target_pos;
    let recover_pos = Vec2::new(
        input.target_pos.x,
        input.target_pos.y - hover_altitude - recover_height,
    );
    let (steering, action) = match state.phase {
        ChoreographyPhase::Approach => {
            // Approach → reach hover position → rest → dive.
            let dist = (hover_pos - input.actor_pos).length();
            if dist <= AERIAL_ENGAGE_DISTANCE && state.has_slot {
                state.phase = ChoreographyPhase::Engage;
                state.phase_timer = hover_rest.max(0.2);
            }
            (hover_pos, None)
        }
        ChoreographyPhase::Engage => {
            if state.phase_timer <= 0.0 {
                // Dive! The "action" is a melee strike — caller
                // converts that into contact damage.
                let dist_to_target = (input.target_pos - input.actor_pos).length();
                if dist_to_target <= MELEE_ENGAGE_DISTANCE {
                    state.phase = ChoreographyPhase::Recover;
                    state.phase_timer = 0.6;
                    (recover_pos, Some(ChoreographyAction::Melee))
                } else {
                    (dive_pos, None)
                }
            } else {
                (hover_pos, None)
            }
        }
        ChoreographyPhase::Recover => {
            if state.phase_timer <= 0.0 {
                state.phase = ChoreographyPhase::Approach;
            }
            (recover_pos, None)
        }
    };
    ChoreographyTick {
        steering_target: steering,
        face_x,
        action,
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
