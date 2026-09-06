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

/// Does this source produce the beat a freeze is FOR?
///
/// ⛔⛔ A CONNECT, NOT AN INJURY. `ResolvedBodyHit` comes off the resolver, which
/// also serves contact attrition, hazards and the blast zone — a fighter leaning
/// on another was measured producing SEVENTEEN `Contact` resolutions in
/// twenty-three ticks, one damage each. A freeze armed by those alternates
/// frozen and moving forever, and three smash fixtures stopped reaching the gait
/// they were waiting for. Standing in lava is not a hit connecting.
fn is_a_connect(source: &crate::HitSource) -> bool {
    matches!(
        source,
        crate::HitSource::Melee | crate::HitSource::Projectile | crate::HitSource::Pogo
    )
}

/// A RESOLVED connect sets the freeze's expiry, bounded by the authored hitlag.
///
/// ⭐ THE VICTIM'S OWN RESOLVED HITLAG, not a flat rate — a jab barely stops the
/// screen and a smash stops it hard, which is the genre's behaviour. A flat
/// `feel.hitlag_time` per connect was measured and is too much: it is the
/// reference for the HARDEST hit.
///
/// ⛔⛔ AND IT READS THE RESOLUTION, NOT THE VICTIM. This used to take
/// `LandedBodyHit` — which means OVERLAP — and go looking for
/// `BodyCombat::hitstop_timer` on the victim. That timer is written this frame
/// for an actor victim and NEXT frame for a player victim, because player-victim
/// hits are staged into a rollback FIFO the player resolver drains in
/// `PlayerSimulation`. **So the connect that stopped the world for a CPU did not
/// stop it for the human** — measured 2026-08-25 in the real schedule: 0.036s of
/// player hitlag, zero frozen frames, and every existing arm blind to it because
/// they all injected the timer before firing the message.
///
/// ⛔ THE FIX IS NOT A SCHEDULE REORDER. `ResolvedBodyHit` carries the resolver's
/// own answer, so this system no longer has an opinion about which road resolved
/// the hit or on which frame — at the price of having to say which SOURCES are a
/// connect, which `LandedBodyHit` used to answer by construction.
pub fn request_impact_hitstop_on_resolved_hits(
    mut hits: MessageReader<crate::hitbox::ResolvedBodyHit>,
    feel: Option<Res<crate::feel::Platformer2dFeelTuningMonolith>>,
    tick: Option<Res<ambition_time::SimTick>>,
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
        if !is_a_connect(&hit.source) {
            continue;
        }
        wanted = wanted.max(hit.hitlag_seconds.min(feel.hitlag_time));
    }
    if wanted <= 0.0 {
        return;
    }
    let until = tick.0 + (wanted * TICKS_PER_SECOND).round().max(0.0) as u64;
    hold.until_tick = Some(hold.until_tick.map_or(until, |held| held.max(until)));
}

#[cfg(test)]
mod tests;
