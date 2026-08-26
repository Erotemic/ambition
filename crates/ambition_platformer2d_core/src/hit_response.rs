//! Carved down from `ambition_damage` (FB6b,
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

impl HitKnockbackMagnitude {
    /// The same launch, harder. Multiplies whichever way this magnitude is
    /// expressed, so a caller scaling a hit does not have to know which.
    pub fn scaled(self, scale: f32) -> Self {
        match self {
            Self::FeelScale(v) => Self::FeelScale(v * scale),
            Self::LaunchSpeed(v) => Self::LaunchSpeed(v * scale),
        }
    }
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
    /// Authored launch DIRECTION, a plain vector in the victim's own
    /// acceleration frame (CM1): `x` = lateral (mirrored to point away from
    /// the source by the resolver's side sign), `y` = toward the feet, the
    /// same `y` [`AccelerationFrame`](crate::reference_frame::AccelerationFrame)
    /// uses everywhere else — so an up-launcher authors `(0, -1)` and a spike
    /// authors `(0, 1)`. `None` = the feel-tuned default diagonal.
    pub launch_dir: Option<Vec2>,
    /// AUTOLINK: this pulse HOLDS the victim near the attacker instead of
    /// launching it away. `None` — the overwhelming majority — is an ordinary
    /// hit, and its cost is one byte in the fingerprint.
    ///
    /// The genre's multi-hit moves work because their intermediate pulses keep
    /// the victim inside the next hitbox; only the LAST one launches. See
    /// [`AutolinkFollow`].
    pub follow: Option<AutolinkFollow>,
    /// WHAT KIND OF REACTION this pulse asks for — a blow, or a gust.
    ///
    /// ⛔⛔ IT WAS A `flinchless: bool`, AND THAT LOST THE THING THE AUTHORING
    /// PROMISES. `VolumeReaction::Windbox` is documented as *"pushes its victim
    /// and does nothing else — no damage, no hitstun, no shield"*, but by the
    /// time the pulse reached the victim all that survived was *don't charge
    /// stun*. Everything else an accepted hit does still happened: the gust
    /// refunded the victim's air dodge, cleared its post-recovery helplessness
    /// and charged hitlag — so a wind pulse could hand a recovering fighter its
    /// dodge back and freeze the match, which is not "push only".
    ///
    /// ⭐ THE KIND SURVIVES; the consequences are read off it once, in
    /// `apply_body_hit_reaction`, rather than each being remembered separately
    /// by whoever happens to look.
    pub reaction: HitReaction,
}

/// A pulse's KIND: what the victim is owed for having been touched by it.
///
/// ⛔ AUTOLINK IS NOT A VARIANT. It is a MODIFIER on an ordinary strike — the
/// victim is still stunned, still takes the damage, still hitlags, and the only
/// difference is where the pulse steers it — so it rides on
/// [`HitKnockback::follow`] beside this. A windbox is a different KIND of thing
/// happening to the body, which is why it is here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitReaction {
    /// A blow. Every ordinary hit, and the default.
    #[default]
    Strike,
    /// A gust: it MOVES the victim and owes it nothing else.
    ///
    /// ⭐ THE LAUNCH IS AN ORDINARY LAUNCH. The wind's strength and direction
    /// are authored the same way a punch's are, on `magnitude` and `launch_dir`;
    /// what a windbox declines is the INJURY, not the physics.
    Windbox,
}

impl HitKnockback {
    /// Is this pulse a gust rather than a blow?
    pub fn is_windbox(&self) -> bool {
        matches!(self.reaction, HitReaction::Windbox)
    }
}

/// How an intermediate multi-hit pulse holds its victim, authored per hit.
///
/// ⛔ THIS IS NOT A CAPTURE. No relationship is formed, the victim keeps its
/// whole control model, and it is free the instant the pulses stop — what holds
/// it is that each pulse re-aims its VELOCITY, which is a hit reaction like any
/// other. A capture would need `CapturedBy`, a hold clock and an escape.
///
/// Everything here is stated in the ATTACKER'S frame and resolved through the
/// victim's `AccelerationFrame`, so a move authored once works under any gravity
/// — a world-axis version would send the victim sideways in a rotated room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutolinkFollow {
    /// Where the victim is steered, in WORLD space, resolved at the pulse.
    ///
    /// ⛔⛔ NOT THE AUTHORED ATTACKER-LOCAL POINT. It was, and the victim side
    /// reconstructed it from `knockback.dir` and the VICTIM'S gravity — two wrong
    /// authorities. `dir` is *"which victim-local side points away from the
    /// attacker"*, which coincides with attacker facing only when the attacker
    /// faces its victim, so a hit that caught somebody BEHIND the attacker
    /// mirrored the anchor toward the victim instead. And `anchor_local.y` means
    /// *toward the ATTACKER'S feet*, which is the victim's `down` only while the
    /// two share a frame — a thing this engine deliberately does not assume.
    ///
    /// ⇒ the producer holds the attacker and resolves it, exactly as it samples
    /// [`Self::source_vel`]: both are facts about the attacker at this pulse, and
    /// neither is the victim's to reinterpret.
    pub anchor_world: Vec2,
    /// Share of the ATTACKER'S own velocity handed to the victim, `0..=1`. This
    /// is what makes a RISING multi-hit work: the correction below only closes a
    /// gap, and a fighter climbing at 600 px/s outruns any gap-closing term.
    pub carry: f32,
    /// Spring gain on the remaining gap, in 1/s. A 30 px gap at `pull: 20.0`
    /// asks for 600 px/s. Authored rather than derived because how HARD a move
    /// grabs is a feel decision per move.
    pub pull: f32,
    /// Ceiling on the corrective term alone, engine units/s. The carry is not
    /// clamped: it is the attacker's real motion, and clipping it would make a
    /// fast attacker's victim fall out of its own move.
    pub max_speed: f32,
    /// The attacker's world velocity at this pulse.
    ///
    /// Sampled by the PRODUCER and carried on the payload because the reaction
    /// holds a victim and no attacker entity. It lives inside this struct rather
    /// than on [`HitKnockback`] so an ordinary hit pays nothing for it.
    pub source_vel: Vec2,
}

/// Resolve an authored attacker-local follow point into world space.
///
/// ⭐ CALLED BY THE PRODUCER, which is the only place that holds the attacker's
/// facing and its resolved frame. `x` runs forward along the attacker's facing
/// and `y` toward the attacker's feet — the same local convention
/// [`HitKnockback::launch_dir`] uses, interpreted against the body that AUTHORED
/// it rather than the body it lands on.
pub fn autolink_anchor_world(
    anchor_local: Vec2,
    attacker_pos: Vec2,
    attacker_facing: f32,
    attacker_gravity_dir: Vec2,
) -> Vec2 {
    let frame = AccelerationFrame::new(attacker_gravity_dir);
    // Mirrors with the attacker's facing: a follow point in front of a
    // left-facing fighter is to its left.
    let facing = if attacker_facing.abs() <= 0.001 {
        1.0
    } else {
        attacker_facing.signum()
    };
    attacker_pos + frame.side * (anchor_local.x * facing) + frame.down * anchor_local.y
}

/// The velocity an autolink pulse writes: carry the attacker, then close the gap
/// to the resolved anchor, bounded.
///
/// ⛔ NO TELEPORT. The victim is given a velocity and moves under its own body
/// authority, which is what keeps it collidable, gravity-correct and rollback-
/// safe. Setting its position would bypass every one of those.
pub fn autolink_velocity(follow: &AutolinkFollow, victim_pos: Vec2) -> Vec2 {
    let gap = follow.anchor_world - victim_pos;
    let correction = gap * follow.pull.max(0.0);
    let correction = if correction.length() > follow.max_speed.max(0.0) {
        correction.normalize_or_zero() * follow.max_speed.max(0.0)
    } else {
        correction
    };
    follow.source_vel * follow.carry.clamp(0.0, 1.0) + correction
}

/// The launch speed a standard authored melee strike carries.
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
    /// The launch speed that counts as a standard hit.
    ///
    /// An authored melee strike carries an absolute launch speed, and hitstun
    /// scales with it against this reference: a strike launching at exactly
    /// `hitstun_reference_launch` arms `hitstun_time`, one launching twice as
    /// hard stuns twice as long. This is the dial that decides how combo-heavy
    /// the game feels — raise it and every hit stuns less.
    ///
    /// `0.0` disables launch scaling, restoring the flat behaviour, which
    /// is what a build with no authored launch speeds wants.
    pub hitstun_reference_launch: f32,
    /// Ceiling on the reaction scale, so a launch at kill percent cannot stun
    /// for seconds. Applied after the reference division.
    pub hitstun_max_scale: f32,
    /// Hitlag at reaction scale `1.0`: the shared freeze a connect buys.
    pub hitlag_time: f32,
    /// DI budget (radians). `0.0` disables directional influence entirely.
    pub di_max_angle: f32,
}

/// Smash directional influence during hitlag: shift the frozen body in the
/// victim's body-local input direction, transformed through gravity. `step <= 0`
/// or null input is inert. Stick magnitude is clamped rather than normalized so
/// partial deflection produces a proportionally smaller shift.
pub fn smash_di_shift(input_local: Vec2, gravity_dir: Vec2, step: f32) -> Vec2 {
    if step <= 0.0 {
        return Vec2::ZERO;
    }
    let world = AccelerationFrame::new(gravity_dir).to_world(input_local);
    let magnitude = world.length();
    if magnitude < 1e-6 {
        return Vec2::ZERO;
    }
    (world / magnitude) * (step * magnitude.min(1.0))
}

/// Rotate the victim's world-space knockback launch toward held body-local input
/// by at most `max_angle`. Influence scales with input magnitude and the component
/// perpendicular to the launch, so input along the launch direction has no effect.
/// `max_angle <= 0` or null input preserves the launch exactly.
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
/// Bigger launch, longer stun, follow-up connects — that is the whole platform-fighter loop, and it
/// is standard, documented behaviour rather than a taste call.
///
/// a `0.0` reference disables the scaling and restores the flat answer, which
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

/// One expression, two callers — the authoritative victim path and the shadow rollout.
pub fn hitstun_duration(knockback: Option<&HitKnockback>, tuning: &HitResponseTuning) -> f32 {
    tuning.hitstun_time * reaction_scale(knockback, tuning).max(0.35)
}

/// The weakest connect the hitlag law admits, as a fraction of
/// [`HitResponseTuning::hitlag_time`].
///
/// Camera hit shake uses the same floor for its dead zone, so the shared constant
/// prevents independent literals from drifting apart.
pub const MIN_HITLAG_SCALE: f32 = 0.5;

/// Damage at which a hit freezes for exactly [`HitResponseTuning::hitlag_time`].
///
/// The MEDIAN staled damage a landed hit deals, so an ordinary connect lands on
/// the reference — the same rationale [`STANDARD_LAUNCH_SPEED`] was chosen by.
/// Measured 2026-08-23 over 3 x 90s smash CPU-vs-CPU, `smash_george_booul` vs
/// itself (⚠ one matchup), every `HitEvent` that reached a victim: n = 203,
/// `min 1  p25 5  p50 11  p75 14  p90 19  max 20`, mean 10.5.
///
/// It reads the STALED damage, which is what `hitbox` puts on the event after
/// `rules.stale_scale` — the damage actually dealt, which is the number the
/// genre computes its freeze from too.
pub const HITLAG_REFERENCE_DAMAGE: f32 = 11.0;

/// The constant half of the genre's affine hitlag law, as a fraction of the
/// reference freeze.
///
/// Melee and its successors compute hitlag as `damage * 0.65 + 6` frames — a
/// term proportional to damage PLUS a fixed floor, and the floor is most of a
/// weak hit's freeze. At [`HITLAG_REFERENCE_DAMAGE`] = 11 that formula gives
/// 13.15 frames of which the constant 6 is `0.456`, so this is that shape
/// expressed against our own reference rather than a second set of frame
/// numbers to keep in step.
const HITLAG_BASE_SCALE: f32 = 0.46;

/// Ceiling on hitlag's damage scaling.
///
/// ⛔ DELIBERATELY NOT [`MAX_HITSTUN_SCALE`], which is what hitlag used to ride
/// while it was computed from knockback. Decoupling the freeze from the launch
/// is the entire point of reading damage: in the genre a stale hit on a
/// high-percent victim has a SHORT freeze and an enormous launch, and one
/// ceiling shared with hitstun cannot express that.
///
/// `1.5` is the scale the hardest hit this tree authors reaches (damage 20 →
/// 1.44), rounded up so authoring a little past today's hardest move is not
/// silently clipped.
pub const MAX_HITLAG_SCALE: f32 = 1.5;

/// How long a connect freezes, from the DAMAGE it dealt.
///
/// ⭐ FROM DAMAGE, NOT FROM KNOCKBACK, and that is the genre's rule rather than
/// a taste call. Knockback grows with the victim's percent, so a law reading it
/// makes the same move freeze longer as a match goes on: measured before this
/// changed, one authored damage value produced freezes spanning 2.2x to 4.6x
/// across a single match, and an 11-damage hit could freeze for less time than
/// a 3-damage one. What varies hugely in this genre is the LAUNCH; the freeze
/// tracks the hit.
///
/// Floored at [`MIN_HITLAG_SCALE`] so even the weakest connect is a readable
/// beat rather than nothing, and capped at [`MAX_HITLAG_SCALE`].
///
/// both sides freeze for the SAME duration, which is what makes a connect
/// read as one event rather than two things happening near each other.
pub fn hitlag_duration(damage: i32, tuning: &HitResponseTuning) -> f32 {
    tuning.hitlag_time * hitlag_scale(damage)
}

/// The damage term of the hitlag law, in units of the reference freeze.
pub fn hitlag_scale(damage: i32) -> f32 {
    let share = damage.max(0) as f32 / HITLAG_REFERENCE_DAMAGE;
    (HITLAG_BASE_SCALE + (1.0 - HITLAG_BASE_SCALE) * share)
        .clamp(MIN_HITLAG_SCALE, MAX_HITLAG_SCALE)
}

/// HOW HARD the hit currently freezing a body was, in `0..=1`.
///
/// The inverse of [`hitlag_duration`]: hitlag is the one quantity that already
/// scales with a connect's weight and is readable off the victim afterwards, so
/// dividing it back out recovers the strength without anyone re-deriving it
/// from damage, knockback or a move name. `0.0` is the weakest connect the
/// hitlag law admits (or no hitlag at all), `1.0` is the ceiling
/// [`reaction_scale`] rides.
///
/// It lives beside [`MIN_HITLAG_SCALE`] for the reason that constant's own doc
/// gives: camera shake already derives its dead zone from this floor, and a
/// second consumer with its own literal is exactly the drift the shared
/// constant exists to prevent.
pub fn hit_strength_fraction(hitstop_seconds: f32, reference_hitlag_seconds: f32) -> f32 {
    if reference_hitlag_seconds <= 0.0 {
        return 0.0;
    }
    let scale = hitstop_seconds / reference_hitlag_seconds;
    let span = MAX_HITLAG_SCALE - MIN_HITLAG_SCALE;
    if span <= 0.0 {
        return 0.0;
    }
    ((scale - MIN_HITLAG_SCALE) / span).clamp(0.0, 1.0)
}

/// THE frame-agnostic knockback velocity for ANY struck body (§A2 step 6):
/// side away from the hit's source (falling back to the stored event dir, then
/// away from facing), launched with a rise against the body's gravity.
///
/// `FeelScale` magnitudes preserve the standard per-source feel vector used by contact damage,
/// hazards, and projectiles. `LaunchSpeed` magnitudes preserve the absolute engine-unit speed
/// authored by melee move volumes.
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
    // Its magnitude is resolved according to the event's explicit unit: feel-scaled contacts
    // preserve the standard feel speed, while authored melee preserves its absolute launch
    // speed.
    //
    // the authored vector IS the local launch direction — `n * speed`, with
    // only `x` mirrored by the away-from-source side. No `y` negation: local `y`
    // means toward the feet here exactly as it does in every other local vector
    // the engine passes around (see [`HitKnockback::launch_dir`]).
    let authored = knockback
        .and_then(|k| k.launch_dir)
        .filter(|ld| ld.length_squared() > 1e-6);
    let local = match (authored, magnitude) {
        (Some(ld), HitKnockbackMagnitude::FeelScale(scale)) => {
            let n = ld.normalize();
            let speed = Vec2::new(tuning.knockback_x, tuning.knockback_y).length() * scale.max(0.0);
            Vec2::new(dir * n.x, n.y) * speed
        }
        (None, HitKnockbackMagnitude::FeelScale(scale)) => {
            let scale = scale.max(0.0);
            Vec2::new(
                dir * tuning.knockback_x * scale,
                -tuning.knockback_y * scale,
            )
        }
        (Some(ld), HitKnockbackMagnitude::LaunchSpeed(speed)) => {
            let n = ld.normalize();
            Vec2::new(dir * n.x, n.y) * speed.max(0.0)
        }
        (None, HitKnockbackMagnitude::LaunchSpeed(speed)) => {
            let default_dir =
                Vec2::new(dir * tuning.knockback_x, -tuning.knockback_y).normalize_or_zero();
            default_dir * speed.max(0.0)
        }
    };
    let launch = frame.to_world(local);
    // CM2: the victim's held input rotates its own launch, bounded by the
    // authored DI budget. Inert at `di_max_angle == 0` (Ambition today).
    di_adjust(launch, di_input_local, gravity_dir, tuning.di_max_angle)
}
#[cfg(test)]
mod hitlag_tests {
    use super::*;

    /// The strength read is the hitlag law run backwards, so a jab and a smash
    /// come back apart — and neither escapes `0..=1`.
    #[test]
    fn hit_strength_recovers_the_weight_of_the_connect() {
        let t = tuning();
        let weakest = hitlag_duration(0, &t);
        let ordinary = hitlag_duration(HITLAG_REFERENCE_DAMAGE as i32, &t);
        let heaviest = hitlag_duration(1_000, &t);

        let strength = |stop| hit_strength_fraction(stop, t.hitlag_time);
        assert_eq!(strength(weakest), 0.0, "the weakest connect is the floor");
        assert_eq!(strength(heaviest), 1.0, "the ceiling is the ceiling");
        let middle = strength(ordinary);
        assert!(
            middle > 0.0 && middle < 1.0,
            "a standard hit sits between: {middle}"
        );

        // Monotone across the whole band — a harder hit never reads softer.
        let mut previous = -1.0;
        for damage in [0, 1, 5, HITLAG_REFERENCE_DAMAGE as i32, 20, 500] {
            let now = strength(hitlag_duration(damage, &t));
            assert!(
                now >= previous,
                "{damage} went backwards: {now} < {previous}"
            );
            previous = now;
        }
    }

    /// A body that is not in hitlag has no strength to report, and a
    /// degenerate reference cannot divide by zero into a flash.
    #[test]
    fn hit_strength_is_zero_without_a_connect_to_measure() {
        assert_eq!(hit_strength_fraction(0.0, tuning().hitlag_time), 0.0);
        assert_eq!(hit_strength_fraction(-1.0, tuning().hitlag_time), 0.0);
        assert_eq!(hit_strength_fraction(10.0, 0.0), 0.0);
        assert_eq!(hit_strength_fraction(10.0, -1.0), 0.0);
    }

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
            // An ordinary hit: it stuns.
            reaction: HitReaction::Strike,
            dir: 1.0,
            magnitude: HitKnockbackMagnitude::LaunchSpeed(speed),
            source_pos: Vec2::ZERO,
            impact_pos: Vec2::ZERO,
            launch_dir: None,
            follow: None,
        }
    }

    /// A connect is ONE event, so it buys ONE freeze — and the freeze is the
    /// HIT's, not the launch's.
    #[test]
    fn hitlag_is_one_duration_for_both_bodies_and_scales_with_the_damage() {
        let t = tuning();
        let reference = hitlag_duration(HITLAG_REFERENCE_DAMAGE as i32, &t);
        assert!(
            (reference - t.hitlag_time).abs() < 1e-6,
            "the reference damage freezes for exactly the reference time"
        );

        // A heavier connect freezes longer — this is most of what "weight"
        // feels like, and a flat constant cannot express it.
        let heavy = hitlag_duration(20, &t);
        assert!(heavy > reference, "a big hit lands harder: {heavy}");

        // …and the weakest connect is still a readable beat, never nothing.
        let poke = hitlag_duration(1, &t);
        assert!(poke >= t.hitlag_time * MIN_HITLAG_SCALE - 1e-6 && poke < reference);

        // …and nothing runs away with it.
        assert!(hitlag_duration(10_000, &t) <= t.hitlag_time * MAX_HITLAG_SCALE + 1e-6);
    }

    /// ⭐ THE POINT OF THE LAW: the freeze is DECOUPLED from the launch.
    ///
    /// Knockback grows with the victim's percent, so a freeze computed from it
    /// makes the same move freeze longer as a match goes on. Measured on the
    /// old law over one match, a single authored damage value produced freezes
    /// spanning 2.2x to 4.6x, and an 11-damage hit could freeze for LESS time
    /// than a 3-damage one.
    ///
    /// This is the assertion the old law could not satisfy, and it is the
    /// reason `hitlag_duration` no longer takes a `HitKnockback` at all — the
    /// signature is what makes the mistake unavailable rather than merely
    /// discouraged.
    #[test]
    fn one_damage_is_one_freeze_however_hard_the_launch() {
        let t = tuning();
        let stale_jab_at_high_percent = hitlag_duration(3, &t);
        let fresh_jab_at_zero = hitlag_duration(3, &t);
        assert_eq!(stale_jab_at_high_percent, fresh_jab_at_zero);

        // And the ordering is the DAMAGE ordering, at every rung.
        let mut previous = 0.0;
        for damage in [1, 3, 5, 11, 14, 20] {
            let now = hitlag_duration(damage, &t);
            assert!(now >= previous, "damage {damage} froze less: {now}");
            previous = now;
        }

        // HITSTUN still rides the launch, and that is deliberate: a body thrown
        // harder stays helpless longer. The two laws now read different terms,
        // which is the whole separation.
        let soft = hitstun_duration(Some(&launch(STANDARD_LAUNCH_SPEED)), &t);
        let hard = hitstun_duration(Some(&launch(STANDARD_LAUNCH_SPEED * 3.0)), &t);
        assert!(hard > soft, "hitstun must still grow with the launch");
    }
}

#[cfg(test)]
mod di_tests {
    use super::di_adjust;
    use crate::Vec2;

    const DOWN: Vec2 = Vec2::new(0.0, 1.0);
    /// ~18°, the budget a platform fighter authors (Smash Ultimate's).
    const BUDGET: f32 = 0.31;

    fn angle_between(a: Vec2, b: Vec2) -> f32 {
        (a.x * b.y - a.y * b.x).atan2(a.x * b.x + a.y * b.y)
    }

    /// you cannot DI along your own launch line — and that is the whole
    /// shape of the mechanic, not a detail. A victim launched away who holds
    /// straight away steers NOTHING; the influence is the PERPENDICULAR part of
    /// the stick. Without this, DI would be a speed dial and holding away from
    /// the blast zone would be strictly correct, which is the opposite of the
    /// read it exists to create.
    #[test]
    fn di_is_the_perpendicular_part_of_the_stick() {
        // Launched along +x. Holding +x is perfectly parallel.
        let launch = Vec2::new(300.0, 0.0);
        let along = di_adjust(launch, Vec2::new(1.0, 0.0), DOWN, BUDGET);
        assert!(
            angle_between(launch, along).abs() < 1e-4,
            "⛔ holding ALONG the launch steered it by {} rad",
            angle_between(launch, along)
        );

        // Holding perpendicular (local +y is gravity-down) spends the whole
        // budget, which is the maximum a launched body can ever buy.
        let across = di_adjust(launch, Vec2::new(0.0, 1.0), DOWN, BUDGET);
        let turned = angle_between(launch, across);
        assert!(
            (turned.abs() - BUDGET).abs() < 1e-4,
            "a fully perpendicular hold should spend the whole {BUDGET} rad \
             budget, turned {turned}"
        );
        assert!(turned > 0.0, "the launch turns TOWARD the held direction");
    }

    /// DI changes the ANGLE and nothing else. a launch whose speed moved
    /// would make holding a direction a damage-mitigation dial, and survival
    /// would stop being about the angle to the blast zone.
    #[test]
    fn di_rotates_the_launch_without_changing_its_speed() {
        let launch = Vec2::new(180.0, -240.0);
        for hold in [
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.7, -0.7),
        ] {
            let out = di_adjust(launch, hold, DOWN, BUDGET);
            assert!(
                (out.length() - launch.length()).abs() < 1e-2,
                "hold {hold:?} changed the launch SPEED: {} → {}",
                launch.length(),
                out.length()
            );
            assert!(
                angle_between(launch, out).abs() <= BUDGET + 1e-4,
                "hold {hold:?} exceeded the authored budget"
            );
        }
    }

    /// POISON: an unauthored budget is inert. Ambition's PvE answers
    /// `0.0` on purpose — being hit there is a punishment, not the opening of a
    /// negotiation — so every body in every non-fighter game rides this path
    /// with a zero budget and must come out byte-identical.
    #[test]
    fn a_zero_budget_or_an_idle_stick_returns_the_launch_untouched() {
        let launch = Vec2::new(180.0, -240.0);
        assert_eq!(di_adjust(launch, Vec2::new(0.0, 1.0), DOWN, 0.0), launch);
        assert_eq!(di_adjust(launch, Vec2::ZERO, DOWN, BUDGET), launch);
    }

    /// A partial stick spends a partial budget, so DI is a continuous control
    /// rather than a three-position switch.
    #[test]
    fn a_half_held_stick_spends_less_than_the_whole_budget() {
        let launch = Vec2::new(300.0, 0.0);
        let full = angle_between(launch, di_adjust(launch, Vec2::new(0.0, 1.0), DOWN, BUDGET));
        let half = angle_between(launch, di_adjust(launch, Vec2::new(0.0, 0.5), DOWN, BUDGET));
        assert!(
            half > 0.0 && half < full - 1e-4,
            "half a stick bought {half} of the {full} a full one does"
        );
    }

    /// frame-agnostic, and the C4 law is what makes that testable. Under
    /// flipped gravity the same body-local hold against the same body-local
    /// launch must produce the same body-local trajectory — the whole thing
    /// conjugates. A DI that read world axes would steer the wrong way the
    /// moment a stage inverted.
    #[test]
    fn di_conjugates_under_a_flipped_frame() {
        let hold = Vec2::new(0.0, 1.0);
        let upright = di_adjust(Vec2::new(300.0, 0.0), hold, DOWN, BUDGET);
        // Flip gravity: the same LOCAL launch is a different world vector.
        let flipped = di_adjust(Vec2::new(-300.0, 0.0), hold, Vec2::new(0.0, -1.0), BUDGET);
        let a = angle_between(Vec2::new(300.0, 0.0), upright);
        let b = angle_between(Vec2::new(-300.0, 0.0), flipped);
        assert!(
            (a - b).abs() < 1e-4,
            "the same local hold turned {a} upright and {b} inverted"
        );
    }
}

#[cfg(test)]
mod launch_direction_tests {
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

    fn launched(launch_dir: Option<Vec2>, gravity_dir: Vec2, speed: f32) -> Vec2 {
        let victim = Vec2::new(100.0, 200.0);
        let frame = AccelerationFrame::new(gravity_dir);
        let knockback = HitKnockback {
            // An ordinary hit: it stuns.
            reaction: HitReaction::Strike,
            dir: 0.0,
            magnitude: HitKnockbackMagnitude::LaunchSpeed(speed),
            // Struck from the local left, so the away-from-source side is +x.
            source_pos: victim - frame.side * 40.0,
            impact_pos: victim,
            launch_dir,
            follow: None,
        };
        knockback_velocity(
            victim,
            1.0,
            gravity_dir,
            Some(&knockback),
            Vec2::ZERO,
            &tuning(),
        )
    }

    /// THE CONVENTION, and it is the whole of.
    ///
    /// `launch_dir` is a plain vector in the victim's acceleration frame, where `y` points TOWARD
    /// THE FEET — the authoring contract's own words (`HitVolume::launch_dir`: *"(+x = facing, +y =
    /// gravity-down)"*), which every authored volume in the tree wrote against.
    ///
    /// the poison is the SPIKE half. A test that only checked the up-launcher
    /// would also pass on a resolver that ignored `launch_dir`'s sign entirely
    /// and always launched up.
    #[test]
    fn an_authored_up_launcher_rises_and_an_authored_spike_drives_down() {
        let down = Vec2::new(0.0, 1.0);
        let rise = launched(Some(Vec2::new(0.0, -1.0)), down, 400.0);
        assert!(
            rise.y < -399.0 && rise.x.abs() < 1e-3,
            "an authored (0,-1) up-launcher must throw the victim AGAINST \
             gravity at the authored speed, got {rise:?}"
        );
        let spike = launched(Some(Vec2::new(0.0, 1.0)), down, 400.0);
        assert!(
            spike.y > 399.0 && spike.x.abs() < 1e-3,
            "an authored (0,1) spike must drive the victim INTO the floor, got \
             {spike:?} — if this and the rise agree, the sign is being dropped"
        );
    }

    /// The authored vector IS the local launch, `x` mirrored to point away from
    /// the source: no other transform sits between the table and the velocity.
    #[test]
    fn the_authored_vector_is_the_local_launch_under_every_gravity() {
        let n = Vec2::new(0.6, -0.8);
        let speed = 250.0;
        for gravity_dir in [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
        ] {
            let vel = launched(Some(n), gravity_dir, speed);
            let frame = AccelerationFrame::new(gravity_dir);
            let local = frame.to_local(vel);
            assert!(
                (local - n * speed).length() < 1e-3,
                "authored {n:?} at {speed} must resolve to exactly that local \
                 launch under gravity {gravity_dir:?}, got {local:?}"
            );
        }
    }

    /// DI STEERS A REAL LAUNCH, and opposite holds go opposite ways.
    ///
    /// The rotation law itself is `di_tests`; this is the whole-launch seam —
    /// the same authored hit, the same victim, two opposite held directions, two
    /// materially different trajectories at the SAME speed. `di_max_angle == 0`
    /// is the parity arm, so a game that authors no budget is untouched.
    ///
    /// live-verified too: in the composed smash host a CPU victim steered a `3269 px/s` up-tilt
    /// launch by `0.308 rad` against the declared `SMASH_DI_MAX_ANGLE` of `0.31` — essentially
    /// the whole budget — with the speed preserved to four figures.
    #[test]
    fn opposite_held_directions_steer_one_launch_two_ways() {
        let down = Vec2::new(0.0, 1.0);
        let victim = Vec2::new(100.0, 200.0);
        let launched_holding = |hold: Vec2, budget: f32| {
            let mut tuning = tuning();
            tuning.di_max_angle = budget;
            let knockback = HitKnockback {
                // An ordinary hit: it stuns.
                reaction: HitReaction::Strike,
                dir: 0.0,
                magnitude: HitKnockbackMagnitude::LaunchSpeed(400.0),
                source_pos: victim - Vec2::new(40.0, 0.0),
                impact_pos: victim,
                launch_dir: Some(Vec2::new(0.0, -1.0)),
                follow: None,
            };
            knockback_velocity(victim, 1.0, down, Some(&knockback), hold, &tuning)
        };
        let left = launched_holding(Vec2::new(-1.0, 0.0), 0.31);
        let right = launched_holding(Vec2::new(1.0, 0.0), 0.31);
        assert!(
            left.x < -50.0 && right.x > 50.0,
            "holding away from each other must send the victim two different              places: {left:?} vs {right:?}"
        );
        assert!(
            (left.length() - right.length()).abs() < 1e-3 && (left.length() - 400.0).abs() < 1e-3,
            "DI rotates a launch, it never resizes one: {left:?} vs {right:?}"
        );
        // PARITY: the same two holds with no budget are the same launch.
        assert_eq!(
            launched_holding(Vec2::new(-1.0, 0.0), 0.0),
            launched_holding(Vec2::new(1.0, 0.0), 0.0),
            "a game that authors no DI budget must not have acquired one"
        );
    }

    /// (Ambition's PvE hits are all this arm.)
    #[test]
    fn an_unauthored_launch_still_rises_away_from_the_source() {
        let vel = launched(None, Vec2::new(0.0, 1.0), 300.0);
        assert!(
            vel.y < 0.0 && vel.x > 0.0,
            "the default diagonal throws up and away from the source: {vel:?}"
        );
    }
}

#[cfg(test)]
mod autolink_tests {
    use super::*;

    const DOWN: Vec2 = Vec2::new(0.0, 1.0);

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

    fn follow_at(anchor_world: Vec2) -> AutolinkFollow {
        AutolinkFollow {
            anchor_world,
            carry: 1.0,
            pull: 20.0,
            max_speed: 900.0,
            source_vel: Vec2::ZERO,
        }
    }

    /// The victim side does ONE thing: close the gap to a point somebody else
    /// resolved.
    #[test]
    fn autolink_steers_a_victim_toward_the_resolved_anchor() {
        let anchor = Vec2::new(120.0, 206.0);
        let victim = Vec2::new(80.0, 240.0);
        let v = autolink_velocity(&follow_at(anchor), victim);
        let gap = anchor - victim;
        assert!(
            v.dot(gap) > 0.0,
            "the pulse did not aim at the anchor: {v:?}"
        );
        assert!(
            v.length() > 100.0,
            "a 52px gap at pull 20 asks for real speed"
        );
    }

    /// ⭐ THE CARRY IS WHAT MAKES A RISING MULTI-HIT WORK. The correction only
    /// closes a gap; a fighter climbing at 600 px/s outruns any gap-closing term
    /// and its victim falls out of the move.
    #[test]
    fn the_attackers_own_motion_is_carried_into_the_victim() {
        let anchor = Vec2::new(118.0, 206.0);
        let victim = Vec2::new(118.0, 208.0);
        let still = autolink_velocity(&follow_at(anchor), victim);
        let rising = AutolinkFollow {
            source_vel: Vec2::new(0.0, -600.0),
            ..follow_at(anchor)
        };
        assert!(
            autolink_velocity(&rising, victim).y < still.y - 500.0,
            "a rising attacker did not carry its victim up"
        );
        let half = AutolinkFollow {
            carry: 0.5,
            ..rising
        };
        assert!(
            (autolink_velocity(&half, victim).y - (still.y - 300.0)).abs() < 1.0,
            "carry 0.5 of -600 should contribute -300"
        );
    }

    /// The correction is BOUNDED, and the carry is not: clamping the attacker's
    /// own motion would drop its victim out of its own move.
    #[test]
    fn the_correction_is_bounded_and_the_carry_is_not() {
        let capped = AutolinkFollow {
            max_speed: 300.0,
            ..follow_at(Vec2::new(100.0, 200.0))
        };
        let far = Vec2::new(100.0, 2000.0);
        assert!(autolink_velocity(&capped, far).length() <= 300.0 + 1e-3);
        let fast = AutolinkFollow {
            source_vel: Vec2::new(0.0, -1200.0),
            ..capped
        };
        assert!(
            autolink_velocity(&fast, far).length() > 300.0,
            "the bound clamped the ATTACKER'S OWN motion"
        );
    }

    /// ⛔⛔ THE ANCHOR IS THE ATTACKER'S, AND THE ATTACKER'S ALONE.
    ///
    /// The victim side used to rebuild it from `knockback.dir` — *"which
    /// victim-local side points away from the attacker"* — and from the VICTIM'S
    /// gravity. Both coincide with the attacker's own facing and frame in the
    /// ordinary case, which is why every front-contact same-gravity test passed
    /// while a back-side hit mirrored the hold point toward the victim.
    #[test]
    fn the_anchor_follows_the_attackers_facing_and_not_the_victims_side() {
        let attacker = Vec2::new(100.0, 100.0);
        let anchor = Vec2::new(18.0, 0.0);
        let facing_right = autolink_anchor_world(anchor, attacker, 1.0, DOWN);
        let facing_left = autolink_anchor_world(anchor, attacker, -1.0, DOWN);
        assert!(
            facing_right.x > attacker.x && facing_left.x < attacker.x,
            "the anchor did not mirror with the attacker's facing"
        );
        assert!(
            ((facing_right.x - attacker.x) + (facing_left.x - attacker.x)).abs() < 1e-3,
            "the mirror is not symmetric"
        );
        // ⭐ AND IT DOES NOT DEPEND ON WHERE THE VICTIM IS. A back-side catch is
        // the same anchor as a front-side one; only the gap differs.
        assert_eq!(
            autolink_anchor_world(anchor, attacker, 1.0, DOWN),
            facing_right,
            "the anchor is a function of the ATTACKER only"
        );
    }

    /// ⛔ AND IT ROTATES WITH THE ATTACKER'S FRAME, not the victim's. Two bodies
    /// in different gravity is a thing this engine deliberately supports, and
    /// "toward the feet" means the ATTACKER'S feet.
    #[test]
    fn the_anchor_rotates_with_the_attackers_frame() {
        let attacker = Vec2::new(100.0, 100.0);
        let anchor = Vec2::new(20.0, 6.0);
        let upright = autolink_anchor_world(anchor, attacker, 1.0, DOWN);
        let sideways = autolink_anchor_world(anchor, attacker, 1.0, Vec2::new(1.0, 0.0));
        assert!(
            ((upright - attacker).length() - (sideways - attacker).length()).abs() < 1e-2,
            "rotating the attacker's frame changed the DISTANCE, so the anchor \
             is being reinterpreted rather than rotated"
        );
        assert!(
            (upright - attacker).dot(sideways - attacker).abs()
                < (upright - attacker).length_squared() * 0.5,
            "the rotated anchor points the same way as the upright one, so the \
             attacker's frame was ignored"
        );
    }

    /// ⭐ THE POISON: an ordinary hit is unchanged, and `follow: None` is what
    /// every existing hit in the tree authors.
    #[test]
    fn an_ordinary_hit_still_resolves_through_the_launch_road() {
        let kb = HitKnockback {
            // An ordinary hit: it stuns.
            reaction: HitReaction::Strike,
            dir: 1.0,
            magnitude: HitKnockbackMagnitude::LaunchSpeed(200.0),
            source_pos: Vec2::ZERO,
            impact_pos: Vec2::new(20.0, 0.0),
            launch_dir: Some(Vec2::new(0.0, -1.0)),
            follow: None,
        };
        assert!(kb.follow.is_none());
        let v = knockback_velocity(
            Vec2::new(20.0, 0.0),
            1.0,
            DOWN,
            Some(&kb),
            Vec2::ZERO,
            &tuning(),
        );
        assert!(v.y < -100.0, "an authored up-launcher stopped launching up");
    }
}
