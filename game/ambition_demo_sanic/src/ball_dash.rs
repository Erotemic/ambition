//! Sanic ball-dash technique implemented entirely in content code.
//!
//! While crouched, melee-press edges add charge; releasing crouch launches along the
//! body's gravity-relative facing direction. Grounded launch writes surface `v_t`;
//! airborne launch resolves the local side axis from gravity. Rolling shrinks
//! `BodyKinematics::size` while leaving `BodyBaseSize` as the standing reference.

use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

/// Feel knobs. A `Resource` so a future act can retune per-zone without a
/// component write, and so a test can build an extreme one.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BallDashTuning {
    /// Charge added per rev tap. One tap clears the launch floor; three reach full.
    pub rev_per_tap: f32,
    /// Charge bled off per second while crouched. Holding forever must not
    /// guarantee a max launch — the rev is a rhythm, not a timer.
    pub decay_per_s: f32,
    /// Launch speed (px/s) at `charge == 1.0`. A full rev must OUT-run the
    /// momentum top-speed cap (the launch writes `v_t` directly, which the kernel
    /// permits above the cap) so a spin dash blasts through the loop: clearing a
    /// radius-180 loop under 2250 gravity needs ~1300 px/s at the bottom, so the
    /// full-charge launch sits comfortably above that with room for pre-loop drag.
    pub launch_speed: f32,
    /// Below this charge a crouch-release is just standing up.
    pub min_launch_charge: f32,
    /// The balled-up body box. Square, so the kernel's circle proxy is exact.
    pub ball_size: ae::Vec2,
    /// Roll ends when tangential speed falls below this. Sanic stands up when he
    /// stops, not when a timer says so.
    pub exit_speed: f32,
    /// `locomotion.y` past this reads as a crouch (local down is `+y`).
    pub crouch_threshold: f32,
    /// Grace after losing a ride contact while a rev is already armed.
    pub contact_grace_s: f32,
}

impl Default for BallDashTuning {
    fn default() -> Self {
        Self {
            rev_per_tap: 0.4,
            decay_per_s: 0.55,
            launch_speed: 1600.0,
            min_launch_charge: 0.3,
            ball_size: ae::Vec2::new(24.0, 24.0),
            exit_speed: 90.0,
            crouch_threshold: 0.5,
            contact_grace_s: 0.12,
        }
    }
}

/// Per-body charge state. Attached to whichever body is being driven; a possessed
/// Sanic revs, and the vacated body does not.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BallDash {
    /// `0..=1`.
    pub charge: f32,
    /// Was the body crouched at the end of last tick? The release edge.
    pub crouched: bool,
    /// This is refreshed only by a real riding contact.
    pub contact_grace: f32,
}

/// Mode-local input latched before the generic worn-kit gate strips peaceful
/// personas' combat verbs. This is not a second device map: it is a content-side
/// interpretation of the same controller-neutral `ActorControl` frame.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BallDashInput {
    /// Local-down is held past the authored threshold.
    pub crouch_held: bool,
    /// Falling edge of local-down, captured at the same controller-neutral seam as the rev.
    pub crouch_released: bool,
    /// Rising edge of the default attack verb (X in arrows + Z/X/C).
    pub rev_pressed: bool,
    /// Whether the body had support when input was captured, before crouch
    /// compaction or movement could transiently detach it from a flat block.
    pub grounded_at_capture: bool,
}

/// The body's action scheme declares `spin_dash` on the Attack slot (`ActorTechniques`), so
/// `gate_worn_player_control` routes the Attack device edge into
/// `ResolvedTechniqueEdges["spin_dash"]` (and clears the raw melee verb); this reads that edge.
/// Runs AFTER the gate. Vacated bodies are reset so a possession handoff cannot replay a stale
/// rev edge.
pub fn capture_ball_dash_input(
    subject: Option<Res<ambition_platformer2d::platformer::markers::ControlledSubject>>,
    tuning: Res<BallDashTuning>,
    mut bodies: Query<
        (
            Entity,
            &ambition_platformer2d::characters::control::ActorControl,
            &ambition_platformer2d::characters::action_scheme::ResolvedTechniqueEdges,
            &ambition_platformer2d::actor::MotionModel,
            &ambition_platformer2d::engine_core::BodyGroundState,
            &mut BallDashInput,
        ),
        With<BallDash>,
    >,
) {
    let controlled = subject.and_then(|subject| subject.0);
    for (entity, control, techniques, motion, ground, mut input) in &mut bodies {
        *input = if Some(entity) == controlled {
            let grounded_at_capture = ground.on_ground
                || matches!(
                    motion,
                    ambition_platformer2d::actor::MotionModel::SurfaceMomentum(momentum)
                        if matches!(momentum.state, ae::SurfaceMotion::Riding { .. })
                );
            let crouch_held = control.0.locomotion.y >= tuning.crouch_threshold;
            BallDashInput {
                crouch_released: input.crouch_held && !crouch_held,
                crouch_held,
                rev_pressed: techniques.pressed("spin_dash"),
                grounded_at_capture,
            }
        } else {
            BallDashInput::default()
        };
    }
}

/// The rolling flag. Carries the size to restore, so nothing has to re-derive
/// "what was he before" — and a body that somehow rolls twice cannot lose its
/// standing height.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Rolling {
    pub restore_size: ae::Vec2,
}

/// What one tick of the charge machine decided. Pure, so the feel is testable
/// without a world: this is where the verb actually lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BallDashStep {
    /// Nothing to do.
    Idle,
    /// Revving. Carries the charge after this tick, for a HUD or a rev SFX pitch.
    Charging(f32),
    /// Released above the floor. Carries the charge that was spent.
    Launch(f32),
}

/// One tick of the charge machine. `grounded` is the support state sampled at
/// the PlayerInput seam — you cannot start a rev in mid-air, which is Sonic's
/// rule and also the only one that makes the launch's `v_t` meaningful.
///
/// A body must be genuinely riding to ADD charge. Once a charge exists, a short
/// contact grace tolerates the one-tick airborne seams produced by block/chain
/// hand-offs and ramp lips. Staying airborne past that grace still clears the rev,
/// so the player cannot bank a dash across an actual jump.
pub fn ball_dash_step(
    state: &mut BallDash,
    grounded: bool,
    crouch: bool,
    crouch_released: bool,
    rev_pressed: bool,
    dt: f32,
    tuning: &BallDashTuning,
) -> BallDashStep {
    // Release is an INPUT edge, not a statement about the contact resolver. Once
    // a grounded rev has armed charge, the falling edge must spend it exactly
    // once even if body-mode expansion or a surface hand-off changes support in
    // this tick. This is also what makes a ramp lip launch rather than eat the
    // spin dash.
    if crouch_released || (state.crouched && !crouch) {
        state.crouched = false;
        state.contact_grace = 0.0;
        let charge = std::mem::take(&mut state.charge);
        return if charge >= tuning.min_launch_charge {
            BallDashStep::Launch(charge)
        } else {
            BallDashStep::Idle
        };
    }

    if grounded {
        state.contact_grace = tuning.contact_grace_s;
    } else {
        state.contact_grace = (state.contact_grace - dt).max(0.0);
    }
    let contact_is_credible = grounded || state.contact_grace > 0.0;

    if crouch {
        // A rev may only START from a real ride contact. Once armed, a tiny
        // airborne seam is tolerated so crossing a block/chain boundary or a
        // ramp lip cannot delete the charge before the release tick arrives.
        if rev_pressed && grounded {
            state.charge = (state.charge + tuning.rev_per_tap).min(1.0);
        }
        if !contact_is_credible {
            *state = BallDash::default();
            return BallDashStep::Idle;
        }
        state.charge = (state.charge - tuning.decay_per_s * dt).max(0.0);
        state.crouched = true;
        return BallDashStep::Charging(state.charge);
    }

    // No crouch and no falling edge: neutral. The explicit edge above owns
    // launch, so a stale level cannot fire twice.
    if !contact_is_credible {
        state.contact_grace = 0.0;
    }
    BallDashStep::Idle
}

/// Rev, launch, roll. [`BallDashInput`] was sampled from the controlled body's
/// actor-control frame before the peaceful-kit gate; this system only consumes the
/// mode-local technique input and never reopens generic combat.
#[allow(clippy::type_complexity)]
pub fn tick_ball_dash(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    tuning: Res<BallDashTuning>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut bodies: Query<(
        Entity,
        &BallDashInput,
        // The body's per-tick resolved frame (ADR 0024): the airborne launch
        // direction is the SAME frame the momentum kernel integrates under.
        &ambition_platformer2d::world::ResolvedMotionFrame,
        &mut ambition_platformer2d::actor::MotionModel,
        &mut ae::BodyKinematics,
        &mut BallDash,
        Option<&Rolling>,
        Option<&mut ambition_platformer2d::characters::actor::BodyAnimFacts>,
    )>,
) {
    for (entity, input, resolved_frame, mut motion, mut kin, mut state, rolling, mut anim) in
        &mut bodies
    {
        let frame = resolved_frame.basis();
        let ambition_platformer2d::actor::MotionModel::SurfaceMomentum(m) = &mut *motion else {
            // A Sanic on the AABB path is a Sanic in a different demo. Nothing
            // here reaches for `MotionModel::AxisSwept`, on purpose: the verb is
            // defined against the momentum kernel's `v_t`, and faking it with a
            // velocity write would produce a dash that ignores slopes.
            continue;
        };
        // Use the contact sampled in PlayerInput. Generic crouch compaction and
        // the momentum step run after that seam and can briefly detach a body
        // from a flat block during the same tick. Rev eligibility belongs to the
        // contact the player actually pressed X on, not that later artifact.
        let grounded = input.grounded_at_capture;

        match ball_dash_step(
            &mut state,
            grounded,
            input.crouch_held,
            input.crouch_released,
            input.rev_pressed,
            time.scaled_dt,
            &tuning,
        ) {
            BallDashStep::Idle => {}
            BallDashStep::Charging(charge) => {
                // Reuse the shared dash-startup pose. A Sanic sheet with a
                // dedicated row plays it; a lean sheet walks the standard
                // fallback chain DashStartup -> Dash -> Run -> Walk -> Idle.
                // This is presentation-only state already consumed by the one
                // body animation picker, not a demo-local rendering branch.
                if let Some(anim) = anim.as_deref_mut() {
                    anim.dash_startup_timer = anim.dash_startup_timer.max(0.10);
                }
                if input.rev_pressed {
                    // The rev climbs: the post-tap charge picks one of three
                    // ascending tiers, so a full three-tap rev reads as the
                    // classic "reh-reh-REH" without per-play pitch.
                    sfx.write_for(
                        entity,
                        ambition_platformer2d::sfx::SfxMessage::Play {
                            id: ambition_platformer2d::sfx::SfxId::from_static(crate::rev_tier_id(
                                charge,
                            )),
                            pos: kin.pos,
                        },
                    );
                }
            }
            BallDashStep::Launch(charge) => {
                let speed = tuning.launch_speed * charge;
                let facing = if kin.facing == 0.0 { 1.0 } else { kin.facing };
                // the sign convention (`v_t` shares facing's sign, because the
                // kernel integrates `v_t += run * accel * dt` with
                // `run = locomotion.x`) now lives on the op rather than in a
                // comment here. The `false` branch is the airborne case, which
                // this launch DOES have an answer for.
                if !m.set_tangential_speed(facing * speed) {
                    // No tangent to speak of; the local side axis is the
                    // kernel's own airborne convention.
                    kin.vel = frame.side * facing * speed;
                }
                // The release whoosh — distinct from the generic dash cue.
                sfx.write_for(
                    entity,
                    ambition_platformer2d::sfx::SfxMessage::Play {
                        id: ambition_platformer2d::sfx::SfxId::from_static(crate::SFX_LAUNCH),
                        pos: kin.pos,
                    },
                );
                if rolling.is_none() {
                    commands.entity(entity).insert(Rolling {
                        restore_size: kin.size,
                    });
                    kin.size = tuning.ball_size;
                }
            }
        }
    }
}

/// Stand back up when the roll runs out of speed. Separate from the launch so a
/// body that gains speed some other way (a booster, a slope) keeps rolling, and
/// so the exit reads off ONE quantity.
pub fn tick_rolling(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &ambition_platformer2d::actor::MotionModel,
        &mut ae::BodyKinematics,
        &Rolling,
    )>,
    tuning: Res<BallDashTuning>,
) {
    for (entity, motion, mut kin, rolling) in &mut bodies {
        let ambition_platformer2d::actor::MotionModel::SurfaceMomentum(m) = motion else {
            continue;
        };
        // Airborne, speed is the world velocity's magnitude; riding, it is |v_t|.
        // A ball flying off a ramp must not un-ball at the apex just because its
        // tangential speed no longer exists.
        let speed = match m.state {
            ae::SurfaceMotion::Riding { v_t, .. } => v_t.abs(),
            ae::SurfaceMotion::Airborne => kin.vel.length(),
        };
        if speed < tuning.exit_speed {
            kin.size = rolling.restore_size;
            commands.entity(entity).remove::<Rolling>();
        }
    }
}

/// Sanic's answer to "this body starts again".
///
/// The engine's own reset clears the engine's own clusters, which is everything
/// it can honestly clear: a charge stored in [`BallDash`], the crouch edge in
/// [`BallDashInput`] and the [`Rolling`] form are Sanic's state, authored here,
/// and no generic ruleset can know they exist. Without this, a body restarted
/// mid-rev came back holding the charge and fired it on the next release edge,
/// or came back still balled up.
///
/// An observer rather than a system, so it lands inside whichever tick did the
/// restarting no matter which schedule slot that caller occupies — and it is
/// inert for every body that is not Sanic, which is why it can be registered
/// unconditionally.
///
/// Standing up restores the size the roll borrowed, exactly as [`tick_rolling`]
/// does: [`Rolling`] carries that size precisely so nobody has to re-derive it.
pub fn clear_ball_dash_on_restart(
    restart: On<ae::BodyRestarted>,
    mut commands: Commands,
    mut charges: Query<(&mut BallDash, &mut BallDashInput)>,
    mut rolls: Query<(&Rolling, &mut ae::BodyKinematics)>,
) {
    let body = restart.entity;
    if let Ok((mut dash, mut input)) = charges.get_mut(body) {
        *dash = BallDash::default();
        *input = BallDashInput::default();
    }
    if let Ok((rolling, mut kin)) = rolls.get_mut(body) {
        kin.size = rolling.restore_size;
        commands.entity(body).remove::<Rolling>();
    }
}

/// Mirror the physical roll authority (the [`Rolling`] component that shrinks the
/// body) onto the presentation fact the ONE animation picker reads, so a balled-up
/// Sanic plays his looping `ball` row instead of a squished run. Per-frame
/// re-derivation, like `aim_anim_active`: `BodyAnimFacts` booleans are
/// presentation mirrors of live state, not latched timers. There is ONE authority
/// for "is a ball" — the `Rolling` component `tick_ball_dash`/`tick_rolling` own —
/// and this only projects it.
pub fn mirror_ball_anim_fact(
    mut bodies: Query<(
        Option<&Rolling>,
        &mut ambition_platformer2d::characters::actor::BodyAnimFacts,
    )>,
) {
    for (rolling, mut anim) in &mut bodies {
        let is_rolling = rolling.is_some();
        // Change-detection courtesy: only touch the component on a real edge.
        if anim.rolling != is_rolling {
            anim.rolling = is_rolling;
        }
    }
}

/// The spin-dash's ACTION-SCHEME declaration: it claims Sanic's Attack slot and
/// names the on-screen button "Spin Dash". The technique's BEHAVIOR stays in
/// this module (the crouch-rev-release chord on `ActorControl`); this only gives
/// the mechanic an identity so the control prompt can label it honestly.
///
/// Handed to `declare_sanic_techniques`, which owns `ActorTechniques` for the
/// whole demo. Attaching it HERE alongside `BallDash` would make two systems
/// insert the same component and the later one would silently drop the other's
/// declaration.
pub(crate) fn spin_dash_technique(
) -> ambition_platformer2d::entity_catalog::action_scheme::ActionSpec {
    use ambition_platformer2d::entity_catalog::action_scheme as sch;
    sch::ActionSpec {
        id: sch::ActionId::new("spin_dash"),
        slot: sch::ControlSlot::Attack,
        display_name: Some("Spin Dash".to_string()),
        visual: None,
        gate: sch::ActionGate::Technique("spin_dash".to_string()),
    }
}

/// Give the controlled body its charge state. Idempotent; runs every tick because
/// possession can hand control to a body that has never revved.
pub fn attach_ball_dash(
    mut commands: Commands,
    subject: Option<Res<ambition_platformer2d::platformer::markers::ControlledSubject>>,
    without: Query<(), (With<ae::BodyKinematics>, Without<BallDash>)>,
) {
    let Some(entity) = subject.and_then(|s| s.0) else {
        return;
    };
    if without.get(entity).is_ok() {
        commands
            .entity(entity)
            .insert((BallDash::default(), BallDashInput::default()));
    }
}

#[cfg(test)]
mod tests;
