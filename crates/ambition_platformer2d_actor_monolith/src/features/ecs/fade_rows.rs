//! The bodies that fade between their attacks (`ambition_combat::fade`):
//! their touch, and the rows they are drawn with.
//!
//! Both are derived from the fade rule every tick, from rollback state (the
//! body's policy, its swing and its hitstun), so a rewind draws the same
//! frame and withdraws the same touch. The empty hurtbox of a faded body is
//! published by `refresh_body_damageable_volumes`.

use ambition_characters::actor::BodyCombat;
use ambition_combat::actor_tuning::{ActorPolicy, ContactThreatWithdrawn};
use ambition_combat::fade::{faded, fade_row};
use ambition_combat::moveset::MeleeSwingQuery;
use ambition_sprite_sheet::character::PinnedRow;
use ambition_time::SimTick;
use bevy::prelude::*;

/// The length of a tick, for the clock of the faded loop (s).
const TICK_S: f32 = 1.0 / 60.0;

/// The name [`pin_fade_rows`] writes its pins under ([`PinnedRow::pin_as`]).
pub const FADE_ROW_WRITER: &str = "fade";

/// A faded body's touch does not hurt. (sim, before the contact pass)
///
/// Only bodies whose tactics fade are written; a body that gets its first
/// [`ContactThreatWithdrawn`] here has it from the next tick on.
pub fn withdraw_faded_contact(
    mut bodies: Query<(Entity, &ActorPolicy, MeleeSwingQuery, Option<&BodyCombat>, Option<&mut ContactThreatWithdrawn>)>,
    mut commands: Commands,
) {
    for (entity, policy, swing, combat, withdrawn) in &mut bodies {
        if !policy.0.tactics.fades {
            continue;
        }
        let now = faded(Some(policy), swing.swing().as_ref(), combat);
        match withdrawn {
            Some(mut withdrawn) => withdrawn.0 = now,
            None => {
                commands.entity(entity).try_insert(ContactThreatWithdrawn(now));
            }
        }
    }
}

/// Pin the row of its fade on each body that fades: faint between its
/// swings, coming solid over a windup, going faint over a recovery. Its
/// strike and its hits draw with its own rows. A pin another system wrote
/// is left alone. (sim)
pub fn pin_fade_rows(
    tick: Res<SimTick>,
    mut bodies: Query<(Entity, &ActorPolicy, MeleeSwingQuery, Option<&BodyCombat>, Option<&mut PinnedRow>)>,
    mut commands: Commands,
) {
    let clock = tick.get() as f32 * TICK_S;
    for (entity, policy, swing, combat, pinned) in &mut bodies {
        if !policy.0.tactics.fades {
            continue;
        }
        let row = fade_row(Some(policy), swing.swing().as_ref(), combat, clock);
        match (row, pinned) {
            (Some((row, elapsed, looping)), None) => {
                let mut pin = PinnedRow::default();
                pin.pin_as(FADE_ROW_WRITER, &[row], elapsed, looping);
                commands.entity(entity).try_insert(pin);
            }
            (Some((row, elapsed, looping)), Some(mut pinned))
                if pinned.written_by(FADE_ROW_WRITER) || !pinned.is_pinned() =>
            {
                pinned.pin_as(FADE_ROW_WRITER, &[row], elapsed, looping);
            }
            (None, Some(mut pinned)) if pinned.written_by(FADE_ROW_WRITER) => pinned.clear(),
            _ => {}
        }
    }
}
