//! The rows an enemy reads by (Jon, 2026-10-30: enemies that "notice, space,
//! telegraph, attack similar to hollow knight enemies"): its `notice` row
//! while it stands and looks at a foe it has just seen, and its `windup` row
//! held over the startup of its swing, the telegraph, before its attack row
//! plays the strike.
//!
//! Derived from rollback state every tick (the brain's notice beat, the
//! swing's phase), as the fade rows are. A sheet without the row draws as
//! though nothing were pinned ([`PinnedRow`]), so a body with no such art is
//! drawn as before.

use ambition_characters::brain::Brain;
use ambition_combat::actor_tuning::ActorPolicy;
use ambition_combat::moveset::MeleeSwingQuery;
use ambition_combat::AttackPhase;
use ambition_sprite_sheet::character::PinnedRow;
use ambition_time::SimTick;
use bevy::prelude::*;

/// The length of a tick, for the clock of a held row (s).
const TICK_S: f32 = 1.0 / 60.0;

/// The name [`pin_telegraph_rows`] writes its pins under ([`PinnedRow::pin_as`]).
pub const TELEGRAPH_ROW_WRITER: &str = "telegraph";
/// The row of a body that has just seen its foe.
pub const NOTICE_ROW: &str = "notice";
/// The row held over a swing's startup.
pub const WINDUP_ROW: &str = "windup";

/// The row an enemy is pinned to now, and how far into it: its notice beat,
/// else the startup of its swing. (pure)
pub fn telegraph_row(brain: Option<&Brain>, swing: Option<&ambition_combat::MeleeSwing>, clock: f32) -> Option<(&'static str, f32)> {
    if brain.is_some_and(Brain::is_noticing) {
        return Some((NOTICE_ROW, clock));
    }
    let swing = swing.filter(|swing| !swing.done())?;
    matches!(swing.phase()?, AttackPhase::Startup).then_some((WINDUP_ROW, swing.elapsed))
}

/// Pin each enemy's notice and windup rows. A pin another system wrote (a
/// fade, a rest) is left alone. (sim)
pub fn pin_telegraph_rows(
    tick: Res<SimTick>,
    mut bodies: Query<(Entity, Option<&Brain>, MeleeSwingQuery, Option<&mut PinnedRow>), With<ActorPolicy>>,
    mut commands: Commands,
) {
    let clock = tick.get() as f32 * TICK_S;
    for (entity, brain, swing, pinned) in &mut bodies {
        let row = telegraph_row(brain, swing.swing().as_ref(), clock);
        match (row, pinned) {
            (Some((row, elapsed)), None) => {
                let mut pin = PinnedRow::default();
                pin.pin_as(TELEGRAPH_ROW_WRITER, &[row], elapsed, true);
                commands.entity(entity).try_insert(pin);
            }
            (Some((row, elapsed)), Some(mut pinned)) if pinned.written_by(TELEGRAPH_ROW_WRITER) || !pinned.is_pinned() => {
                pinned.pin_as(TELEGRAPH_ROW_WRITER, &[row], elapsed, true);
            }
            (None, Some(mut pinned)) if pinned.written_by(TELEGRAPH_ROW_WRITER) => pinned.clear(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::{AttackIntent, AttackSpec, MeleeSwing};
    use bevy::math::Vec2;

    /// A raptor's swipe: 0.28 s windup, 0.08 s strike, 0.32 s recovery.
    fn swipe(elapsed: f32) -> MeleeSwing {
        let mut swing = MeleeSwing::new(AttackSpec {
            intent: AttackIntent::Forward,
            startup_seconds: 0.28,
            active_seconds: 0.08,
            recovery_seconds: 0.32,
            hitbox_offset: Vec2::ZERO,
            hitbox_half_size: Vec2::ZERO,
            self_impulse: Vec2::ZERO,
            knockback: Vec2::ZERO,
        });
        swing.elapsed = elapsed;
        swing
    }

    /// The windup row is held over the swing's startup only: its strike and
    /// its recovery are its attack row's, and between swings nothing is
    /// pinned.
    #[test]
    fn the_windup_row_is_held_over_the_startup_only() {
        assert_eq!(telegraph_row(None, Some(&swipe(0.1)), 5.0), Some((WINDUP_ROW, 0.1)));
        assert_eq!(telegraph_row(None, Some(&swipe(0.3)), 5.0), None, "the strike");
        assert_eq!(telegraph_row(None, Some(&swipe(0.5)), 5.0), None, "the recovery");
        assert_eq!(telegraph_row(None, None, 5.0), None, "between swings");
    }
}
