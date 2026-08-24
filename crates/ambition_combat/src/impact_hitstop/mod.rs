//! The match-level impact freeze: an absolute expiry tick, held in rollback
//! state, that stops the sim clock for a connect nobody is playing.
//!
//! Every rung of the clock ladder read slot zero, so a CPU-versus-CPU match
//! produced local hitlag on both bodies and NO screen freeze — the beat that
//! sells a connect was a player-shaped affordance in a game whose fights are
//! frequently between two CPUs. ⛔ the fix is not a fake primary player: a
//! match's freeze is a fact about the MATCH.
//!
//! ⛔⛔ IT CANNOT BE DERIVED FROM `BodyCombat::hitstop_timer`, which is the
//! obvious implementation. A player's reaction timers decay on `wall_dt`; an
//! actor's and a boss's decay on `sim_dt` (a test in the monolith guards that
//! split). A freeze driven off a CPU's hitstop would stop the clock that decays
//! the hitstop — the stuck-at-zero shape this engine has already paid for once.
//!
//! ⛔⛔ AND IT CANNOT BE AN UNREGISTERED WALL-CLOCK VALUE on the camera-shake
//! precedent, which is how it was first written. A shake writes PRESENTATION;
//! this writes the `dt` every body integrates with, so a resimulated frame ran
//! at a different pace and
//! `combat_equipment_switch_and_breakable_survive_forced_rollback_identically`
//! reported a checksum mismatch at frames [18, 19, 20]. Registering the float
//! would not have helped either: wall time is not a deterministic input to a
//! replayed frame.
//!
//! ⭐⭐ SO IT IS AN ABSOLUTE EXPIRY AGAINST `SimTick`, which is already rollback
//! state and already advances while `sim_dt == 0`. That is the whole design:
//!
//! * it cannot freeze its own expiry, because the tick it is measured against
//!   does not stop when the clock is scaled to zero;
//! * it is the same on a replayed frame, because both the expiry and the clock
//!   it is compared to rewind together;
//! * overlapping connects are `max`, which is deterministic and needs no
//!   ordering rule;
//! * and there is NO HAND-BACK. Nothing here writes a scale. The ladder re-reads
//!   every fact every frame and its last rung is `1.0`, so an expired hold
//!   restores the pace by ceasing to be true. The imperative
//!   `set 0 … remember to set 1` lifecycle is what produced the original
//!   stuck-at-zero bug and is deliberately absent.

use bevy::prelude::*;

/// The tick this freeze ends on, or `None` when the world is not frozen.
///
/// ⛔ AN ABSOLUTE EXPIRY, not a decrementing counter. A counter is mutable state
/// touched every tick for no reason; an expiry is written once per connect and
/// read thereafter, and two overlapping hits combine with `max` rather than
/// with an accumulation rule somebody has to remember.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImpactHitstop {
    pub until_tick: Option<u64>,
}

impl ImpactHitstop {
    /// Is the world frozen on this tick?
    pub fn is_freezing(&self, now: ambition_time::SimTick) -> bool {
        self.until_tick.is_some_and(|until| now.0 < until)
    }
}

/// The tick rate the authored hitlag seconds are converted against.
///
/// ⛔ A CONSTANT, not a frame delta. The conversion has to give the same answer
/// on a resimulated frame as it did live.
const TICKS_PER_SECOND: f32 = 60.0;

/// A landed hit sets the freeze's expiry, bounded by the authored hitlag.
///
/// ⭐ THE VICTIM'S OWN RESOLVED HITLAG, not a flat rate — the same number and
/// the same source `shake_camera_on_landed_hits` reads one system over, and this
/// runs in the same phase for the same reason: the frame's damage is resolved.
///
/// ⛔ A FLAT `feel.hitlag_time` PER CONNECT WAS MEASURED AND IS TOO MUCH. It is
/// the reference for the HARDEST hit, so every jab would stop the world as long
/// as a smash. Proportional means a jab barely stops the screen and a smash
/// stops it hard, which is the genre's behaviour.
pub fn request_impact_hitstop_on_landed_hits(
    mut hits: MessageReader<crate::hitbox::LandedBodyHit>,
    feel: Option<Res<crate::feel::Platformer2dFeelTuningMonolith>>,
    tick: Option<Res<ambition_time::SimTick>>,
    bodies: Query<&ambition_characters::actor::BodyCombat>,
    hold: Option<ResMut<ImpactHitstop>>,
) {
    let (Some(feel), Some(tick), Some(mut hold)) = (feel, tick, hold) else {
        // A headless fixture that installed no feel route still runs this
        // schedule; no tuning means no freeze rather than a panic.
        hits.clear();
        return;
    };
    let mut wanted = 0.0f32;
    for hit in hits.read() {
        let landed = bodies
            .get(hit.victim)
            .map(|combat| combat.hitstop_timer)
            .unwrap_or(0.0);
        wanted = wanted.max(landed.min(feel.hitlag_time));
    }
    if wanted <= 0.0 {
        return;
    }
    let until = tick.0 + (wanted * TICKS_PER_SECOND).round().max(0.0) as u64;
    hold.until_tick = Some(hold.until_tick.map_or(until, |held| held.max(until)));
}

#[cfg(test)]
mod tests;
