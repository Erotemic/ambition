//! **The hit-response kernel** — the pure math of what a landed hit does to a
//! body: launch velocity, directional influence, and hitstun duration.
//!
//! Carved down from `ambition_platformer2d_actor_monolith::features::ecs::damage_apply` (FB6b,
//! fighter-brain.md §12.3 route 1) so ONE formula answers both callers:
//!
//! * the authoritative victim path (`damage_apply`) resolves real hits with it;
//! * the fighter brain's shadow rollout (`brain::fighter::rollout`) predicts
//!   hits with it — and `ambition_characters` cannot name `ambition_platformer2d_actor_monolith`
//!   (the dependency points the other way), which is why the kernel lives on
//!   the floor.
//!
//! Everything here is pure and frame-agnostic. The boss/enemy feel selection
//! stays with the CALLER: [`HitResponseTuning`] carries the already-chosen
//! numbers, so the kernel never learns what a "boss" is.

use crate::reference_frame::AccelerationFrame;
use bevy_math::Vec2;

/// Unit-bearing magnitude for a [`HitKnockback`].
///
/// Contact damage, world damage boxes, hazards, and projectiles tune a
/// multiplier over the struck body's standard feel vector. Authored melee
/// volumes instead store an absolute launch speed in engine units (pixels
/// per second). Keeping these meanings in separate variants prevents an
/// authored value such as `120.0 px/s` from being misread as a `120x` feel
/// multiplier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitKnockbackMagnitude {
    /// Dimensionless multiplier over the victim's standard feel-tuned launch.
    /// `1.0` is the standard enemy or boss reaction.
    FeelScale(f32),
    /// Absolute launch speed in engine units (pixels per second).
    LaunchSpeed(f32),
}

/// Knockback impulse carried by a hit. Producers fill this on hits that
/// should push the victim around (enemy melee, enemy projectile, boss swing);
/// leave `None` for impulse-free hits (player slash, pogo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitKnockback {
    /// Horizontal impulse direction (±1).
    pub dir: f32,
    /// Unit-bearing launch magnitude. Never pass an untyped authored scalar.
    pub magnitude: HitKnockbackMagnitude,
    /// World-space attacker position — used for VFX direction.
    pub source_pos: Vec2,
    /// World-space impact position — used for VFX position.
    pub impact_pos: Vec2,
    /// Authored launch DIRECTION in the victim's gravity frame (CM1): `x` =
    /// lateral (mirrored to point away from the source by the resolver's
    /// side sign), `y` = upward against gravity. `None` = the feel-tuned
    /// default diagonal.
    pub launch_dir: Option<Vec2>,
}

/// **The launch speed a standard authored melee strike carries.**
///
/// The reference `hitstun_reference_launch` defaults to, chosen from what the
/// tree actually authors: shipped melee bases sit in the 40–200 band and the
/// growth term pushes a well-fed strike past it. So an ordinary hit lands near
/// the flat duration this replaces, a weak poke stuns less, and a grown smash
/// stuns more — which is the point.
pub const STANDARD_LAUNCH_SPEED: f32 = 150.0;

/// Ceiling on hitstun's launch scaling. Four standard hits' worth of stun is
/// already a long time to be unable to act; past that a launch is a kill, not a
/// combo starter.
pub const MAX_HITSTUN_SCALE: f32 = 4.0;

/// The victim-side feel numbers one hit resolution needs, ALREADY selected by
/// the caller (boss vs. enemy rows, difficulty — none of that is the
/// kernel's business).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitResponseTuning {
    /// Standard feel-launch lateral speed (engine units/s).
    pub knockback_x: f32,
    /// Standard feel-launch rise speed (engine units/s, against gravity).
    pub knockback_y: f32,
    /// Standard hitstun duration (seconds) at reaction scale `1.0`.
    pub hitstun_time: f32,
    /// **The launch speed that counts as a standard hit.**
    ///
    /// An authored melee strike carries an absolute launch speed, and hitstun
    /// scales with it against this reference: a strike launching at exactly
    /// `hitstun_reference_launch` arms `hitstun_time`, one launching twice as
    /// hard stuns twice as long. This is the dial that decides how combo-heavy
    /// the game feels — raise it and every hit stuns less.
    ///
    /// ⚠ **`0.0` disables launch scaling**, restoring the flat behaviour, which
    /// is what a build with no authored launch speeds wants.
    pub hitstun_reference_launch: f32,
    /// Ceiling on the reaction scale, so a launch at kill percent cannot stun
    /// for seconds. Applied after the reference division.
    pub hitstun_max_scale: f32,
    /// **Hitlag at reaction scale `1.0`: the shared freeze a connect buys.**
    ///
    /// ⛔ **ONE number for both bodies.** Attacker and victim used two —
    /// `attack_hitstop_time` (0.055) and `player_damage_hitstop_time` (0.070) —
    /// applied at two sites, neither scaled by how hard the hit was. A landed
    /// strike is one event and the pause it buys is one duration; two constants
    /// that can drift apart are two chances for the connect to read as mushy on
    /// one side of it.
    pub hitlag_time: f32,
    /// DI budget (radians). `0.0` disables directional influence entirely.
    pub di_max_angle: f32,
}

/// THE directional-influence law (CM2): the victim's held control rotates its
/// OWN knockback launch, by at most `max_angle` radians. Pure and
/// frame-agnostic — `launch` is the resolved world-frame launch velocity;
/// `di_input_local` is the victim's `ActorControl.locomotion` (local `x` =
/// side, `y` = gravity-down, magnitude a `[0,1]` throttle); `gravity_dir`
/// places that local intent into the world frame. The rotation turns `launch`
/// TOWARD the held direction, weighted by how PERPENDICULAR the input is to
/// the launch (you cannot DI along your own launch line) and by the throttle —
/// classic smash DI. PARITY: `max_angle == 0.0` (or a null input) returns
/// `launch` unchanged, so DI is inert until a game authors a budget.
/// Frame-agnostic because `launch` and the world-frame input rotate together
/// under any gravity, so the victim-local trajectory conjugates (the C4 law).
pub fn di_adjust(launch: Vec2, di_input_local: Vec2, gravity_dir: Vec2, max_angle: f32) -> Vec2 {
    if max_angle <= 0.0 {
        return launch;
    }
    let speed = launch.length();
    if speed < 1e-6 {
        return launch;
    }
    let frame = AccelerationFrame::new(gravity_dir);
    let di_world = frame.to_world(di_input_local);
    let di_mag = di_world.length();
    if di_mag < 1e-6 {
        return launch;
    }
    let throttle = di_mag.min(1.0);
    let launch_dir = launch / speed;
    let di_dir = di_world / di_mag;
    // Signed sine of the angle FROM launch TO the held direction: its magnitude
    // is the perpendicular fraction, its sign the way to rotate.
    let cross = launch_dir.x * di_dir.y - launch_dir.y * di_dir.x;
    let rot = (max_angle * cross.abs() * throttle).min(max_angle) * cross.signum();
    let (s, c) = rot.sin_cos();
    Vec2::new(launch.x * c - launch.y * s, launch.x * s + launch.y * c)
}

/// Dimensionless hitstun scale for a unit-bearing knockback event.
///
/// A `FeelScale` explicitly scales the whole standard reaction, including
/// hitstun.
///
/// ⭐⭐ **an absolute launch speed scales it too, against
/// [`HitResponseTuning::hitstun_reference_launch`] — and THAT is the mechanic
/// combos are made of.** This arm used to return a flat `1.0`, so a jab and a
/// fully-grown smash armed identical hitstun: a launched fighter recovered just
/// as fast at 150% as at 0%, nothing could ever be followed up, and the
/// knockback growth the strike had already computed reached the victim's
/// VELOCITY and stopped there. Bigger launch, longer stun, follow-up connects —
/// that is the whole platform-fighter loop, and it is standard, documented
/// behaviour rather than a taste call.
///
/// ⚠ a `0.0` reference disables the scaling and restores the flat answer, which
/// is right for a build authoring no launch speeds.
pub fn reaction_scale(knockback: Option<&HitKnockback>, tuning: &HitResponseTuning) -> f32 {
    let Some(knockback) = knockback else {
        return 0.0;
    };
    match knockback.magnitude {
        HitKnockbackMagnitude::FeelScale(scale) => scale.max(0.0),
        HitKnockbackMagnitude::LaunchSpeed(launch) => {
            if tuning.hitstun_reference_launch <= 0.0 {
                return 1.0;
            }
            (launch.max(0.0) / tuning.hitstun_reference_launch).min(tuning.hitstun_max_scale)
        }
    }
}

/// THE hitstun a landed hit arms: the standard duration, reaction-scaled, with
/// the floor that keeps even a soft hit a readable beat. One expression, two
/// callers — the authoritative victim path and the shadow rollout.
pub fn hitstun_duration(knockback: Option<&HitKnockback>, tuning: &HitResponseTuning) -> f32 {
    tuning.hitstun_time * reaction_scale(knockback, tuning).max(0.35)
}

/// **THE hitlag a landed hit buys — the same freeze for attacker and victim.**
///
/// Scales with the hit exactly as hitstun does, so a jab taps and a smash
/// *lands*; the perceived weight of a connect is mostly this. Floored at half
/// the standard so even the weakest connect is a readable beat rather than
/// nothing, and it rides the same [`reaction_scale`] ceiling.
///
/// ⚠ **both sides freeze for the SAME duration**, which is what makes a connect
/// read as one event rather than two things happening near each other.
pub fn hitlag_duration(knockback: Option<&HitKnockback>, tuning: &HitResponseTuning) -> f32 {
    tuning.hitlag_time * reaction_scale(knockback, tuning).max(0.5)
}

/// THE frame-agnostic knockback velocity for ANY struck body (§A2 step 6):
/// side away from the hit's source (falling back to the stored event dir, then
/// away from facing), launched with a rise against the body's gravity.
///
/// `FeelScale` magnitudes preserve the standard per-source feel vector used by
/// contact damage, hazards, and projectiles. `LaunchSpeed` magnitudes preserve
/// the absolute engine-unit speed authored by melee move volumes. The latter is
/// deliberately NOT multiplied by the feel vector: doing so was the goblin-hit
/// teleport regression (`120 px/s` was interpreted as a `120x` multiplier).
pub fn knockback_velocity(
    victim_pos: Vec2,
    victim_facing: f32,
    gravity_dir: Vec2,
    knockback: Option<&HitKnockback>,
    // The victim's held control (local frame), for directional influence (CM2).
    // `ZERO` == no DI intent; the effect is also inert unless
    // `tuning.di_max_angle` is nonzero, so this is parity-free by construction.
    di_input_local: Vec2,
    tuning: &HitResponseTuning,
) -> Vec2 {
    let frame = AccelerationFrame::new(gravity_dir);
    let side_from_source = knockback.map(|k| (victim_pos - k.source_pos).dot(frame.side));
    let knockback_dir = side_from_source
        .filter(|d| d.abs() > 0.001)
        .or_else(|| knockback.map(|k| k.dir))
        .unwrap_or(0.0);
    let dir = if knockback_dir.abs() <= 0.001 {
        -victim_facing
    } else {
        knockback_dir.signum()
    };
    let magnitude = knockback
        .map(|k| k.magnitude)
        .unwrap_or(HitKnockbackMagnitude::FeelScale(0.0));
    // CM1: a volume-authored launch DIRECTION (smash-style fixed angles)
    // replaces the default feel diagonal. Its magnitude is resolved according
    // to the event's explicit unit: feel-scaled contacts preserve the standard
    // feel speed, while authored melee preserves its absolute launch speed.
    let authored = knockback
        .and_then(|k| k.launch_dir)
        .filter(|ld| ld.length_squared() > 1e-6);
    let local = match (authored, magnitude) {
        (Some(ld), HitKnockbackMagnitude::FeelScale(scale)) => {
            let n = ld.normalize();
            let speed = Vec2::new(tuning.knockback_x, tuning.knockback_y).length() * scale.max(0.0);
            Vec2::new(dir * n.x * speed, -n.y * speed)
        }
        (None, HitKnockbackMagnitude::FeelScale(scale)) => {
            let scale = scale.max(0.0);
            Vec2::new(dir * tuning.knockback_x * scale, -tuning.knockback_y * scale)
        }
        (Some(ld), HitKnockbackMagnitude::LaunchSpeed(speed)) => {
            let n = ld.normalize();
            let speed = speed.max(0.0);
            Vec2::new(dir * n.x * speed, -n.y * speed)
        }
        (None, HitKnockbackMagnitude::LaunchSpeed(speed)) => {
            let default_dir = Vec2::new(dir * tuning.knockback_x, -tuning.knockback_y)
                .normalize_or_zero();
            default_dir * speed.max(0.0)
        }
    };
    let launch = frame.to_world(local);
    // CM2: the victim's held input rotates its own launch, bounded by the
    // authored DI budget. Inert at `di_max_angle == 0` (Ambition today).
    di_adjust(launch, di_input_local, gravity_dir, tuning.di_max_angle)
}#[cfg(test)]
mod hitlag_tests {
    use super::*;

    fn tuning() -> HitResponseTuning {
        HitResponseTuning {
            knockback_x: 100.0,
            knockback_y: 100.0,
            hitstun_time: 0.24,
            hitstun_reference_launch: STANDARD_LAUNCH_SPEED,
            hitstun_max_scale: MAX_HITSTUN_SCALE,
            hitlag_time: 0.070,
            di_max_angle: 0.0,
        }
    }

    fn launch(speed: f32) -> HitKnockback {
        HitKnockback {
            dir: 1.0,
            magnitude: HitKnockbackMagnitude::LaunchSpeed(speed),
            source_pos: Vec2::ZERO,
            impact_pos: Vec2::ZERO,
            launch_dir: None,
        }
    }

    /// ⭐⭐ **A connect is ONE event, so it buys ONE freeze.**
    ///
    /// Attacker and victim call this same function with the same landed hit, so
    /// there is no arrangement of tuning that lets the two sides of a connect
    /// pause for different lengths. Before it there were two unscaled constants
    /// at two call sites — 0.055 and 0.070 — which is two chances for a hit to
    /// read as mushy on one side of itself.
    #[test]
    fn hitlag_is_one_duration_for_both_bodies_and_scales_with_the_hit() {
        let t = tuning();
        let reference = hitlag_duration(Some(&launch(STANDARD_LAUNCH_SPEED)), &t);
        assert!((reference - t.hitlag_time).abs() < 1e-6);

        // ⭐ a heavier connect freezes longer — this is most of what "weight"
        // feels like, and a flat constant cannot express it.
        let heavy = hitlag_duration(Some(&launch(STANDARD_LAUNCH_SPEED * 3.0)), &t);
        assert!(heavy > reference * 2.5, "a big launch lands hard: {heavy}");

        // …and the weakest connect is still a readable beat, never nothing.
        let poke = hitlag_duration(Some(&launch(1.0)), &t);
        assert!(poke >= t.hitlag_time * 0.5 - 1e-6 && poke < reference);

        // ⛔ the poison: hitlag and hitstun read the SAME scale off the SAME
        // hit. If one grows and the other does not, the pause and the stun have
        // drifted apart and the connect stops reading as a single event.
        for speed in [40.0_f32, STANDARD_LAUNCH_SPEED, 600.0] {
            let k = launch(speed);
            let lag = hitlag_duration(Some(&k), &t) / t.hitlag_time;
            let stun = hitstun_duration(Some(&k), &t) / t.hitstun_time;
            // Floors differ deliberately (0.5 vs 0.35), so compare only where
            // neither is clamped.
            if lag > 0.5 + 1e-6 && stun > 0.35 + 1e-6 {
                assert!(
                    (lag - stun).abs() < 1e-6,
                    "lag {lag} and stun {stun} must ride one scale at {speed}"
                );
            }
        }
    }
}


