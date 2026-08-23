//! The knockout beat — the one moment in a match that had no presentation.
//!
//! Every other beat in this demo has a cue: the charge pulse, the i-frame
//! blink, the impact flash, the launch trail, the parry snap, the dizzy ring,
//! the crash splat. The one that ENDS A STOCK had none — a body simply stopped
//! being where it was.
//!
//! What the genre ships, and what this ships: a bright flash and a spark burst
//! at the point the body left play, plus a sound, and an ELIMINATION reads
//! bigger than an ordinary stock loss because in the genre the last one is the
//! big one. ⛔ Star KO and Screen KO are deliberately absent: the games differ
//! on which of the three an upward knockout picks and how often (Screen KOs are
//! rare in the first three games and about as common as Star KOs from Smash 4
//! on), and a variant nobody has asked for is a knob invented ahead of its
//! customer.
//!
//! # Where the body was
//!
//! ⭐ THE KO POSITION IS DESTROYED BEFORE ANY CONSUMER CAN LOOK.
//! `FighterStockSpent` carries an `Entity`; `place_respawning_fighters` reads
//! the same message inside `CombatSet::Settle` and teleports that body onto the
//! respawn platform on the same tick, and an eliminated body is despawned
//! outright. A burst drawn at the entity's position would appear over the
//! respawn platform — plausible, and wrong.
//!
//! So the position is captured sim-side, at the seam that still has it:
//! [`ambition_sim_view::KnockoutsView`] publishes the place the body was on the
//! tick before the knockout resolved. Nothing here resolves an entity.

use bevy::prelude::*;

use ambition_sfx::{ids, SfxMessage, SfxWriter};
use ambition_time::SimTick;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

/// Sparks thrown by an ordinary stock loss, and by an elimination.
///
/// The gap is the read: in the genre the knockout that takes a fighter OUT is
/// the one the room reacts to, and it is the only distinction this module
/// draws because `FighterStockSpent::eliminated` is the only one the
/// simulation publishes.
const STOCK_SPARKS: u32 = 26;
const ELIMINATION_SPARKS: u32 = 48;

/// Extra sparks at the top of the launch band, on top of whichever base above
/// applies.
///
/// ⭐ THE BAND IS THE LAUNCH TRAIL'S, not a second opinion. A knockout is the
/// end of a flight, and the plume that led into it read the body's speed off
/// `flight_intensity`; a burst that scored the same launch on its own scale
/// would contradict the trail on screen. The two facts a hit can be measured by
/// — the HIT's weight (what hitlag, the strong-hit flash and the camera shake
/// all derive from) and the BODY's flight — are different questions, and a
/// knockout is squarely the second.
const SPEED_SPARKS: u32 = 22;

/// How fast the burst leaves, in world units per second. Fast and short-lived:
/// a knockout is a bang, not a plume — `ParticleKind::Spark` shrinks and falls
/// where the launch trail's `Dust` grows and hangs.
const STOCK_SPARK_SPEED: f32 = 520.0;
const ELIMINATION_SPARK_SPEED: f32 = 720.0;

/// Hot white, with the elimination pushed toward the launch trail's ember so
/// the hardest thing that can happen to a body is the same colour as the
/// hardest launch.
const STOCK_RGBA: [f32; 4] = [1.0, 0.98, 0.92, 0.95];
const ELIMINATION_RGBA: [f32; 4] = [1.0, 0.72, 0.34, 1.0];

/// What one knockout asks for. Pure, so the whole rule is asserted without a
/// renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnockoutBeat {
    pub sparks: u32,
    pub speed: f32,
    pub rgba: [f32; 4],
    /// How many expanding rings. An elimination gets a second one, which is the
    /// cheapest way to make it read as bigger without a new effect.
    pub rings: u32,
}

/// The beat a knockout asks for.
///
/// `eliminated` is `FighterStockSpent::eliminated` — the simulation's own
/// answer to "was that their last stock", never a comparison of `remaining`
/// against zero here. `flight_speed` is how fast the body was going when it
/// left play, scored on the launch trail's own band so the plume and the burst
/// that ends it agree — see [`SPEED_SPARKS`].
pub fn knockout_beat(eliminated: bool, flight_speed: f32) -> KnockoutBeat {
    let hard = super::launch_trail::flight_intensity(flight_speed);
    let base = if eliminated {
        KnockoutBeat {
            sparks: ELIMINATION_SPARKS,
            speed: ELIMINATION_SPARK_SPEED,
            rgba: ELIMINATION_RGBA,
            rings: 2,
        }
    } else {
        KnockoutBeat {
            sparks: STOCK_SPARKS,
            speed: STOCK_SPARK_SPEED,
            rgba: STOCK_RGBA,
            rings: 1,
        }
    };
    KnockoutBeat {
        sparks: base.sparks + (SPEED_SPARKS as f32 * hard).round() as u32,
        ..base
    }
}

/// Draw the knockout beat for every knockout published this tick.
///
/// Runs on the render clock and samples on the SIM clock, like the launch
/// trail: a frame that advanced no tick must not draw the same knockout twice.
pub fn emit_knockout_beat(
    tick: Res<SimTick>,
    mut last_sampled: Local<Option<u64>>,
    knockouts: Res<ambition_sim_view::KnockoutsView>,
    mut vfx: MessageWriter<VfxMessage>,
    mut sfx: SfxWriter,
) {
    if *last_sampled == Some(tick.0) {
        return;
    }
    *last_sampled = Some(tick.0);
    for knockout in &knockouts.0 {
        let beat = knockout_beat(knockout.eliminated, knockout.speed);
        let pos = knockout.pos;
        for _ in 0..beat.rings {
            vfx.write(VfxMessage::Impact { pos });
        }
        vfx.write(VfxMessage::Burst {
            pos,
            count: beat.sparks,
            speed: beat.speed,
            color: beat.rgba,
            kind: ParticleKind::Spark,
        });
        sfx.write(SfxMessage::Play {
            id: ids::WORLD_EXPLOSION,
            pos,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An elimination is the bigger beat, and every term of it says so.
    #[test]
    fn the_last_stock_reads_bigger_than_an_ordinary_one() {
        let stock = knockout_beat(false, 0.0);
        let out = knockout_beat(true, 0.0);
        assert!(out.sparks > stock.sparks);
        assert!(out.speed > stock.speed);
        assert!(out.rings > stock.rings);
        // And it is hotter: the elimination shifts toward the launch trail's
        // ember rather than simply throwing more white.
        assert!(out.rgba[0] - out.rgba[2] > stock.rgba[0] - stock.rgba[2]);
    }

    /// A knockout scores the flight it ended, on the SAME band the plume that
    /// led into it used.
    ///
    /// Both directions matter. A body crawling over the line is a knockout and
    /// still gets its beat; a body thrown out at full launch gets a bigger one;
    /// and past the trail's own saturation neither gets denser, because a
    /// knockout that kept growing would disagree with the plume beside it.
    #[test]
    fn a_knockout_scores_the_flight_that_ended_it() {
        let crawl = knockout_beat(false, 0.0);
        let hard = knockout_beat(false, 10_000.0);
        assert!(crawl.sparks > 0, "a knockout is always a beat");
        assert!(
            hard.sparks > crawl.sparks,
            "and a hard one is a bigger beat"
        );
        assert_eq!(
            hard.sparks,
            knockout_beat(false, 100_000.0).sparks,
            "it saturates where the trail's own band does"
        );
        // The elimination premium survives the speed term at both ends.
        assert!(knockout_beat(true, 0.0).sparks > crawl.sparks);
        assert!(knockout_beat(true, 10_000.0).sparks > hard.sparks);
    }
}
