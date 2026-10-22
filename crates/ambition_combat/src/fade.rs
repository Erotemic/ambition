//! A body that FADES between its attacks ([`MeleeTactics::fades`]): a spirit
//! that is faint and intangible while it circles you, and solid while it
//! strikes (Jon, 2026-10-22, the spectral wolf: "fades between attacks").
//!
//! [`faded`] is the one rule. Its readers derive their state from it each
//! tick, so it holds no state of its own:
//!
//! - the body's published hurtbox is empty while it is faded
//!   (`refresh_body_damageable_volumes`), so nothing hits it;
//! - its contact threat is withdrawn ([`ContactThreatWithdrawn`]), so its
//!   touch does not hurt;
//! - it is drawn with the rows of its fade ([`fade_row`]).
//!
//! [`MeleeTactics::fades`]: ambition_characters::brain::MeleeTactics::fades
//! [`ContactThreatWithdrawn`]: crate::actor_tuning::ContactThreatWithdrawn

use ambition_characters::actor::BodyCombat;

use crate::actor_tuning::ActorPolicy;
use crate::components::MeleeSwing;
use crate::AttackPhase;

/// Is this body faded now? A body whose tactics fade is faded unless it is
/// in a swing (from the start of its windup to the end of its recovery) or
/// reels from a hit.
pub fn faded(policy: Option<&ActorPolicy>, swing: Option<&MeleeSwing>, combat: Option<&BodyCombat>) -> bool {
    policy.is_some_and(|policy| policy.0.tactics.fades)
        && swing.is_none_or(MeleeSwing::done)
        && combat.is_none_or(|combat| combat.hitstun_timer <= 0.0)
}

/// The sheet rows of a fade.
pub const FADED_ROW: &str = "faded";
/// It comes solid over the windup of its swing: the tell.
pub const FADE_IN_ROW: &str = "fade_in";
/// It sinks and goes faint over the recovery of its swing (solid still: the
/// punish).
pub const FADE_OUT_ROW: &str = "fade_out";

/// The row a fading body is drawn with now, and seconds into it, and whether
/// it loops; `None` when its own rows draw it (the strike of its swing, a
/// hit). `clock` is the seconds of the faded loop.
pub fn fade_row(
    policy: Option<&ActorPolicy>,
    swing: Option<&MeleeSwing>,
    combat: Option<&BodyCombat>,
    clock: f32,
) -> Option<(&'static str, f32, bool)> {
    if !policy.is_some_and(|policy| policy.0.tactics.fades) {
        return None;
    }
    if faded(policy, swing, combat) {
        return Some((FADED_ROW, clock, true));
    }
    let swing = swing.filter(|swing| !swing.done())?;
    match swing.phase()? {
        AttackPhase::Startup => Some((FADE_IN_ROW, swing.elapsed, false)),
        AttackPhase::Active => None,
        AttackPhase::Recovery => Some((
            FADE_OUT_ROW,
            swing.elapsed - swing.spec.startup_seconds - swing.spec.active_seconds,
            false,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AttackIntent, AttackSpec};
    use ambition_characters::brain::{BrainProfile, MeleeTactics};
    use bevy::math::Vec2;

    fn spirit() -> ActorPolicy {
        ActorPolicy(BrainProfile { tactics: MeleeTactics { fades: true, ..Default::default() }, ..Default::default() })
    }

    /// The spectral wolf's bite: 0.36 s windup, 0.10 s strike, 0.42 s recovery.
    fn bite(elapsed: f32) -> MeleeSwing {
        let mut swing = MeleeSwing::new(AttackSpec {
            intent: AttackIntent::Forward,
            startup_seconds: 0.36,
            active_seconds: 0.10,
            recovery_seconds: 0.42,
            hitbox_offset: Vec2::ZERO,
            hitbox_half_size: Vec2::ZERO,
            self_impulse: Vec2::ZERO,
            knockback: Vec2::ZERO,
        });
        swing.elapsed = elapsed;
        swing
    }

    /// Between its swings it is faded; through a swing, windup to recovery,
    /// it is solid; reeling from a hit it is solid. A body that does not fade
    /// never is.
    #[test]
    fn it_is_faded_only_between_its_swings() {
        let policy = spirit();
        let reeling = BodyCombat { hitstun_timer: 0.2, ..Default::default() };
        assert!(faded(Some(&policy), None, None));
        for t in [0.0, 0.2, 0.4, 0.8] {
            assert!(!faded(Some(&policy), Some(&bite(t)), None), "faded {t} s into its swing");
        }
        assert!(faded(Some(&policy), Some(&bite(0.9)), None), "solid after its swing ended");
        assert!(!faded(Some(&policy), None, Some(&reeling)));
        assert!(!faded(Some(&ActorPolicy::default()), None, None));
        assert!(!faded(None, None, None));
    }

    /// It is drawn faint between its swings, coming solid over a windup, its
    /// own strike row in the strike, and going faint over a recovery; each
    /// transition row from its start.
    #[test]
    fn its_rows_follow_its_swing() {
        let policy = spirit();
        assert_eq!(fade_row(Some(&policy), None, None, 3.0), Some((FADED_ROW, 3.0, true)));
        assert_eq!(fade_row(Some(&policy), Some(&bite(0.1)), None, 3.0), Some((FADE_IN_ROW, 0.1, false)));
        assert_eq!(fade_row(Some(&policy), Some(&bite(0.4)), None, 3.0), None);
        let (row, at, looping) = fade_row(Some(&policy), Some(&bite(0.56)), None, 3.0).expect("a row");
        assert_eq!((row, looping), (FADE_OUT_ROW, false));
        assert!((at - 0.1).abs() < 1e-5, "{at}");
        assert_eq!(fade_row(Some(&ActorPolicy::default()), None, None, 3.0), None);
    }
}
