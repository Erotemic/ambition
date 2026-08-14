//! Composable movement-ability functions — the limbs of the shared spine.
//!
//! Each `apply_<verb>` is a self-contained step the integration calls in a fixed
//! order. Splitting the movement monolith into these named units is the first
//! move toward the "shared physics spine + composable ability limbs" architecture
//! (see `docs/concepts/one-body-one-path.md`): an ability
//! reads + writes ONLY its own cluster fields, so it can later become an opt-in
//! component+system an actor carries or not — and an actor (enemy, NPC, boss,
//! player) is then a different *instance* of one system, differing only in which
//! ability components + tuning it holds.

use super::events::FrameEvents;
use super::input::InputState;
use super::model::AxisManeuverState;
use super::ops::MovementOp;
use super::tuning::AxisSweptParams;
use crate::body_clusters::{
    BodyAbilities, BodyComboTrace, BodyDashState, BodyDodgeState, BodyFlightState, BodyGroundState,
    BodyKinematics, BodyShieldState,
};
use crate::MotionFrame;

/// **What the shared burst button would actually DO right now.**
///
/// Dodge and dash are one input. Which maneuver a press produces is decided by
/// the body's current state — grounded or not, dodge cooldown, air-dodge budget
/// and endlag, dash charges and cooldown — and not by which abilities the body
/// owns. A body that owns both and is mid-dodge-cooldown DASHES.
///
/// ⛔⛔ **the reason this is a named value and not two booleans**: an autonomous
/// driver was choosing its maneuver from `can_dodge` / `can_dash`, which are
/// CAPABILITIES. So a brain could decide *I am dodging* while the kernel — whose
/// dodge declined on cooldown without consuming the buffered press — dashed
/// instead. The brain's stated intent and the body's action disagreed, and no
/// test could see it because the tests varied the two capability flags, which
/// were never the thing that decides.
///
/// ⚠ **availability, not intent** — the buffered press is deliberately NOT an
/// input here. Perception asks *what would a press mean*, one phase before any
/// press exists; the `apply_` steps ask the same question and add their own
/// buffer check. One rule, two callers, in the shape [`resolve_shield`] already
/// established.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BurstManeuver {
    /// The press produces nothing: no ability, or every route is on cooldown.
    #[default]
    None,
    /// A grounded evade roll (invulnerable, committed, then a cooldown).
    GroundDodge,
    /// An airborne evade in any stick direction — once per airtime.
    AirDodge,
    /// The traversal burst: a fast committed move in the aimed direction.
    Dash,
}

impl BurstManeuver {
    /// Whichever of the two dodges this is, if it is one.
    pub fn is_dodge(self) -> bool {
        matches!(self, Self::GroundDodge | Self::AirDodge)
    }
}

/// See [`BurstManeuver`]. Pure + frame-agnostic; the precedence (dodge outranks
/// dash on a body that owns both) is the call order of [`apply_dodge`] before
/// [`apply_dash`], stated once here instead of emerging from it.
pub fn resolve_burst_maneuver(
    abilities: &BodyAbilities,
    ground: &BodyGroundState,
    dodge: &BodyDodgeState,
    state: &AxisManeuverState,
    dash: &BodyDashState,
    tuning: AxisSweptParams,
) -> BurstManeuver {
    if let Some(evade) = available_dodge(abilities, ground, dodge, state, tuning) {
        return evade;
    }
    if dash_available(abilities, dash) {
        return BurstManeuver::Dash;
    }
    BurstManeuver::None
}

/// The dodge half of [`resolve_burst_maneuver`], split out so [`apply_dodge`]
/// gates on the SAME expression rather than a second copy of it. (It cannot call
/// the whole resolver: it holds no dash state, and needing it would be threading
/// one ability's cluster through another's step.)
fn available_dodge(
    abilities: &BodyAbilities,
    ground: &BodyGroundState,
    dodge: &BodyDodgeState,
    state: &AxisManeuverState,
    tuning: AxisSweptParams,
) -> Option<BurstManeuver> {
    if !abilities.abilities.dodge {
        return None;
    }
    if ground.on_ground {
        return (dodge.cooldown <= 0.0).then_some(BurstManeuver::GroundDodge);
    }
    // ⛔ the budget is checked WITHOUT consuming the buffered press, so a body
    // that has already dodged this airtime leaves it standing and the press goes
    // on to mean what it would have meant without the dodge ability at all —
    // which is a dash. That fall-through is deliberate, and it is exactly why
    // an autonomous driver must ask this resolver rather than `abilities.dodge`.
    (!dodge.air_dodge_spent
        && tuning.abilities.air_dodge_time > 0.0
        && state.air_dodge_timer <= 0.0
        && state.air_dodge_endlag_timer <= 0.0)
        .then_some(BurstManeuver::AirDodge)
}

/// The dash half of [`resolve_burst_maneuver`] — see [`available_dodge`].
fn dash_available(abilities: &BodyAbilities, dash: &BodyDashState) -> bool {
    abilities.abilities.dash && dash.charges_available > 0 && dash.cooldown <= 0.0
}

/// Facing + input buffering: turn to face the stick (only when grounded or
/// flying), and buffer jump/dash presses for the short windows the sim phase
/// consumes them in. The intent step at the head of the control phase.
pub(super) fn apply_intent(
    kinematics: &mut BodyKinematics,
    ground: &BodyGroundState,
    flight: &BodyFlightState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    input: InputState,
    tuning: AxisSweptParams,
) {
    let can_turn = ground.on_ground || flight.fly_enabled;
    let local_stick = input.local_axis();
    if can_turn && local_stick.x.abs() > 0.1 {
        kinematics.facing = local_stick.x.signum();
    }
    if input.jump_pressed() && abilities.abilities.jump {
        state.buffer_jump = tuning.locomotion.jump_buffer;
    }
    if input.dash_pressed() && abilities.abilities.dash {
        state.buffer_dash = tuning.abilities.dash_buffer;
    }
}

/// Flight toggle: flip fly mode; on entering, clear transient wall/dash/blink
/// maneuver state so the body cleanly enters free flight.
pub(super) fn apply_fly_toggle(
    flight: &mut BodyFlightState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    events: &mut FrameEvents,
) {
    if input.fly_toggle_pressed() && abilities.abilities.fly && abilities.abilities.fly_toggle {
        flight.fly_enabled = !flight.fly_enabled;
        if flight.fly_enabled {
            state.fast_falling = false;
            state.wall_clinging = false;
            state.wall_climbing = false;
            state.dash_timer = 0.0;
            state.blink_grace_timer = 0.0;
            state.phased_jump.clear();
        }
        events.op_clusters(combo_trace, MovementOp::FlyToggle);
    }
}

/// **The evade, on the ground and in the air.** A buffered dash spent by a body
/// that owns the dodge ability becomes a roll when its feet are down and an air
/// dodge when they are not (the dodge ability claims the dash buffer before
/// `apply_dash` would).
///
/// The two share the buffer, the ability gate and the i-frame *idea*, and
/// nothing else — see [`AxisManeuverState::air_dodge_timer`] for why they do not
/// share a timer. Their commitments differ in kind:
///
/// ```text
///                  gate                travel            spent against
/// ground roll      on_ground           facing/stick x     a cooldown clock
/// air dodge        airborne, unspent   full 2D stick      this trip through the air
/// ```
///
/// ⚠ **an air dodge with `air_dodge_time <= 0.0` is a body that has none.** The
/// tuning defaults are `#[serde(default)]`, so every authored body baked before
/// the maneuver existed keeps exactly the movement it had; a body opts in by
/// authoring a window.
pub(super) fn apply_dodge(
    kinematics: &mut BodyKinematics,
    dodge: &mut BodyDodgeState,
    state: &mut AxisManeuverState,
    ground: &BodyGroundState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    if state.buffer_dash <= 0.0 {
        return;
    }
    // The ONE availability rule (see [`BurstManeuver`]) — the same expression
    // autonomous perception reads, so a driver that decided "dodge" and a body
    // that performs one cannot disagree.
    let Some(evade) = available_dodge(abilities, ground, dodge, state, tuning) else {
        return;
    };
    let local_stick = input.local_axis();
    if evade == BurstManeuver::GroundDodge {
        let dir = if local_stick.x.abs() > 0.1 {
            local_stick.x.signum()
        } else {
            kinematics.facing
        };
        let descend = kinematics.vel.dot(frame.down()).min(0.0);
        kinematics.vel =
            frame.side() * (dir * tuning.abilities.dodge_roll_speed) + frame.down() * descend;
        state.dodge_roll_timer = tuning.abilities.dodge_roll_time;
        state.phased_jump.clear();
        dodge.cooldown = tuning.abilities.dodge_roll_cooldown;
        state.buffer_dash = 0.0;
        events.op_clusters(combo_trace, MovementOp::DodgeRoll);
        return;
    }
    // ── airborne ────────────────────────────────────────────────────────────
    // The stick aims the evade in the body's own frame: sideways, up, down or
    // any diagonal. A neutral stick dodges in place — a real option, not a
    // degenerate one, because the invulnerability is the point and standing
    // still keeps the body where its drift left it.
    let aim = if local_stick.length_squared() > 0.01 {
        local_stick.normalize()
    } else {
        bevy_math::Vec2::ZERO
    };
    // ⚠ **local `y` points toward the FEET**, the same convention
    // `wants_drop_through` reads — so the stick's y composes with `down()`, not
    // against it. Negating here would have aimed every "dodge down through the
    // stage" upward, which is the exact input a recovering body uses.
    kinematics.vel =
        (frame.side() * aim.x + frame.down() * aim.y) * tuning.abilities.air_dodge_speed;
    state.air_dodge_timer = tuning.abilities.air_dodge_time;
    state.air_dodge_endlag_timer = 0.0;
    state.fast_falling = false;
    state.phased_jump.clear();
    dodge.air_dodge_spent = true;
    state.buffer_dash = 0.0;
    events.op_clusters(combo_trace, MovementOp::AirDodge);
}

/// The ONE shield-activation rule, shared by the player body and every actor body
/// (roadmap S6b convergence / invariant I3 — the body owns the gate, the
/// controller only attempts). Given the controller's held-shield attempt and the
/// body's gates — does it have the shield ability, and is it mid-dash (you can't
/// raise a guard while dashing) — it resolves the raised state and refreshes the
/// parry window on the *rising edge*. Returns `true` iff a FRESH guard was raised
/// this tick (the edge that opens a parry window / emits a `ShieldUp` op), so the
/// caller can fire its own side effect. Pure + frame-agnostic.
///
/// The player's [`apply_shield`] and the actor resolver in `update_ecs_actors`
/// both call this, so "raise the guard" is one implementation, not two.
pub fn resolve_shield(
    active: &mut bool,
    parry_window_timer: &mut f32,
    ability_enabled: bool,
    dash_active: bool,
    shield_held: bool,
    parry_window_time: f32,
) -> bool {
    if !ability_enabled {
        *active = false;
        *parry_window_timer = 0.0;
        return false;
    }
    let want = shield_held && !dash_active;
    let fresh = want && !*active;
    if fresh {
        *parry_window_timer = parry_window_time;
    }
    *active = want;
    fresh
}

/// Shield / parry hold. Can't raise while dashing; opens a parry window on the
/// rising edge. Thin player-side wrapper over the shared [`resolve_shield`] rule.
pub(super) fn apply_shield(
    shield: &mut BodyShieldState,
    state: &AxisManeuverState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    let fresh = resolve_shield(
        &mut shield.active,
        &mut shield.parry_window_timer,
        abilities.abilities.shield,
        state.dash_timer > 0.0,
        input.shield_held,
        tuning.abilities.parry_window_time,
    );
    if fresh {
        events.op_clusters(combo_trace, MovementOp::ShieldUp);
    }
}

/// Variable jump height: cut the rising jump short on an early button release.
pub(super) fn apply_jump_release(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    if !input.jump_released() {
        return;
    }
    cut_ascent_now(kinematics, state, abilities, frame, tuning);
}

/// The variable-jump CUT itself, with the release edge already established by
/// the caller. Split out because a jump-squat swallows the release edge — the
/// button can come up mid-crouch, when there is no ascent to shorten — so the
/// squat's takeoff replays the cut at the instant the body actually leaves the
/// ground. ⛔ that is a deferred release, not a second "short hop" mechanic:
/// whichever [`super::tuning::AxisJumpLaw`] the body authored still decides
/// what shortening means.
pub(super) fn cut_ascent_now(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    if !abilities.abilities.variable_jump {
        return;
    }
    match tuning.locomotion.jump_law {
        super::tuning::AxisJumpLaw::VelocityCut => {
            let ascend_speed = -kinematics.vel.dot(frame.down());
            if ascend_speed > 120.0 {
                let along_down = kinematics.vel.dot(frame.down());
                kinematics.vel += frame.down() * (along_down * 0.54 - along_down);
            }
        }
        super::tuning::AxisJumpLaw::PhasedGravity(_) => {
            // Do not rewrite velocity. The next integration tick observes the
            // latched release and applies the stronger gravity phase in the
            // body's current resolved frame.
            state.phased_jump.cancel_hold();
        }
    }
}

/// Dash: a buffered, charge-gated burst that REPLACES the velocity vector and
/// opens a timed window during which the integrator skips normal physics (see
/// `integrate_velocity_clusters`'s `dash.timer > 0` branch). Picks Dash vs
/// DoubleDash by the charge count before decrement. No-op unless the actor has
/// the dash ability + a buffered press + a free charge + the cooldown clear, so
/// an actor without dash (no buffered press / `abilities.dash == false`) pays
/// only the gate check.
///
/// Order: runs in the CONTROL phase after the input buffer is populated and
/// after dodge (which consumes the same buffer on the ground first).
pub(super) fn apply_dash(
    kinematics: &mut BodyKinematics,
    dash: &mut BodyDashState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    // Same shared rule as the dodge step above — see [`BurstManeuver`].
    if state.buffer_dash > 0.0 && dash_available(abilities, dash) {
        let fallback = bevy_math::Vec2::new(kinematics.facing, 0.0);
        let aim = input.local_axis().normalize_or(fallback);
        kinematics.vel = frame.to_world(aim) * tuning.abilities.dash_speed;
        state.dash_timer = tuning.abilities.dash_time;
        state.phased_jump.clear();
        dash.cooldown = tuning.abilities.dash_cooldown;
        state.buffer_dash = 0.0;
        let before = dash.charges_available;
        dash.charges_available = dash.charges_available.saturating_sub(1);
        let op = if before >= 2 {
            MovementOp::DoubleDash
        } else {
            MovementOp::Dash
        };
        events.op_clusters(combo_trace, op);
    }
}

#[cfg(test)]
mod burst_maneuver_tests {
    use super::*;
    use crate::body_clusters::{
        BodyAbilities, BodyComboTrace, BodyDashState, BodyDodgeState, BodyGroundState,
        BodyKinematics,
    };
    use crate::movement::events::FrameEvents;
    use crate::movement::input::InputState;
    use crate::movement::model::AxisManeuverState;
    use crate::movement::tuning::AxisSweptParams;
    use crate::MotionFrame;

    /// A body that owns BOTH verbs — the shipped Smash fighter (P4.30).
    fn both_abilities() -> BodyAbilities {
        let mut abilities = BodyAbilities::default();
        abilities.abilities.dodge = true;
        abilities.abilities.dash = true;
        abilities
    }

    /// Run the kernel's two burst steps in their real order against a buffered
    /// press, and report which maneuver the BODY performed.
    fn what_the_body_does(
        abilities: &BodyAbilities,
        ground: &BodyGroundState,
        dodge: &mut BodyDodgeState,
        dash: &mut BodyDashState,
        tuning: AxisSweptParams,
    ) -> BurstManeuver {
        let mut kinematics = BodyKinematics::default();
        let mut state = AxisManeuverState::default();
        state.buffer_dash = 0.1;
        let mut combo_trace = BodyComboTrace::default();
        let mut events = FrameEvents::default();
        let frame = MotionFrame::from_direction(bevy_math::Vec2::new(0.0, 1.0), 900.0);
        let input = InputState::default();

        apply_dodge(
            &mut kinematics,
            dodge,
            &mut state,
            ground,
            abilities,
            &mut combo_trace,
            input,
            frame,
            tuning,
            &mut events,
        );
        let dodged = if state.dodge_roll_timer > 0.0 {
            Some(BurstManeuver::GroundDodge)
        } else if state.air_dodge_timer > 0.0 {
            Some(BurstManeuver::AirDodge)
        } else {
            None
        };
        apply_dash(
            &mut kinematics,
            dash,
            &mut state,
            abilities,
            &mut combo_trace,
            input,
            frame,
            tuning,
            &mut events,
        );
        match dodged {
            Some(evade) => evade,
            None if state.dash_timer > 0.0 => BurstManeuver::Dash,
            None => BurstManeuver::None,
        }
    }

    /// **THE RESOLVER AND THE BODY ANSWER THE SAME QUESTION.**
    ///
    /// ⛔⛔ this is the whole reason [`resolve_burst_maneuver`] exists. An
    /// autonomous driver was choosing its maneuver from the body's
    /// CAPABILITIES, and a dodge on cooldown declines without consuming the
    /// buffered press — so `apply_dash` takes it and the body dashes while the
    /// brain goes on saying "dodge". Every state below is a state a Smash
    /// fighter is in several times a match.
    ///
    /// ⭐ the second row is the poison: on a body owning both, the ONLY thing
    /// separating a roll from a dash is the cooldown, so a resolver that
    /// answered from capabilities would give the same answer for both rows and
    /// this test would be measuring nothing.
    #[test]
    fn what_the_resolver_says_is_what_the_body_does() {
        let abilities = both_abilities();
        // ⛔⛔ **NOT `AxisSweptParams::default()` UNMODIFIED.** Its
        // `air_dodge_time` is 0.0, so on default tuning the air-dodge branch can
        // never fire and both airborne rows below would agree for the wrong
        // reason — the `AirDodge` variant would go entirely unexercised while
        // the test read as covering it. A fixture that cannot reach a case is
        // not covering it.
        let mut tuning = AxisSweptParams::default();
        tuning.abilities.air_dodge_time = 0.3;
        let grounded = BodyGroundState {
            on_ground: true,
            ..Default::default()
        };
        let airborne = BodyGroundState::default();

        let cases: [(&str, BodyGroundState, BodyDodgeState, BodyDashState); 5] = [
            (
                "grounded, everything ready",
                grounded.clone(),
                BodyDodgeState::default(),
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "grounded, the dodge is on cooldown — THE PRESS DASHES",
                grounded.clone(),
                BodyDodgeState {
                    cooldown: 0.4,
                    ..Default::default()
                },
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "airborne with the air dodge in hand",
                airborne.clone(),
                BodyDodgeState::default(),
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "airborne, the air dodge is spent — THE PRESS DASHES",
                airborne.clone(),
                BodyDodgeState {
                    air_dodge_spent: true,
                    ..Default::default()
                },
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "grounded, dodge on cooldown and no dash charge — nothing happens",
                grounded.clone(),
                BodyDodgeState {
                    cooldown: 0.4,
                    ..Default::default()
                },
                BodyDashState::default(),
            ),
        ];

        let mut seen = Vec::new();
        for (label, ground, dodge, dash) in cases {
            let predicted = resolve_burst_maneuver(
                &abilities,
                &ground,
                &dodge,
                &AxisManeuverState::default(),
                &dash,
                tuning,
            );
            let (mut dodge, mut dash) = (dodge, dash);
            let performed = what_the_body_does(&abilities, &ground, &mut dodge, &mut dash, tuning);
            assert_eq!(
                predicted, performed,
                "{label}: the resolver said {predicted:?} and the kernel did \
                 {performed:?} — a brain that trusted the resolver would have \
                 named a maneuver its own body did not perform"
            );
            seen.push(predicted);
        }

        assert!(
            seen.contains(&BurstManeuver::Dash),
            "poison: at least one case must resolve to Dash on a body that OWNS \
             the dodge, or availability and capability are not being told apart \
             and this test proves nothing: {seen:?}"
        );
        assert!(
            seen.contains(&BurstManeuver::GroundDodge) && seen.contains(&BurstManeuver::AirDodge),
            "and BOTH evades must be reached, or one of the two is unexercised \
             while the test reads as covering it: {seen:?}"
        );
    }
}

#[cfg(test)]
mod resolve_shield_tests {
    use super::resolve_shield;

    /// The shared rule's contract (the one both the player wrapper and the actor
    /// resolver depend on): ability-gated, rising-edge parry, dash-blocked, sustain.
    #[test]
    fn resolve_shield_is_the_one_rule() {
        // Disabled ability forces the guard down and clears the parry window.
        let (mut active, mut parry) = (true, 0.5);
        let fresh = resolve_shield(&mut active, &mut parry, false, false, true, 0.2);
        assert!(!active && parry == 0.0 && !fresh, "no ability → no guard");

        // Rising edge: a held shield with the ability raises a FRESH guard and opens
        // the parry window.
        let (mut active, mut parry) = (false, 0.0);
        let fresh = resolve_shield(&mut active, &mut parry, true, false, true, 0.2);
        assert!(
            active && parry == 0.2 && fresh,
            "rising edge opens a fresh parry"
        );

        // Held across a second tick: still raised, but NOT a fresh edge (no re-arm).
        let fresh = resolve_shield(&mut active, &mut parry, true, false, true, 0.2);
        assert!(active && !fresh, "sustained hold is not a fresh parry");

        // Can't raise while dashing — the gate that binds the player AND the actor.
        let (mut active, mut parry) = (false, 0.0);
        let fresh = resolve_shield(&mut active, &mut parry, true, true, true, 0.2);
        assert!(!active && !fresh, "dashing blocks the guard");

        // Release drops the guard (sustain re-evaluated every tick).
        let (mut active, mut parry) = (true, 0.2);
        resolve_shield(&mut active, &mut parry, true, false, false, 0.2);
        assert!(!active, "releasing the button drops the guard");
    }
}
