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

/// The beat a knockout asks for. `eliminated` is
/// `FighterStockSpent::eliminated` — the simulation's own answer to "was that
/// their last stock", never a comparison of `remaining` against zero here.
pub fn knockout_beat(eliminated: bool) -> KnockoutBeat {
    if eliminated {
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
        let beat = knockout_beat(knockout.eliminated);
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
        let stock = knockout_beat(false);
        let out = knockout_beat(true);
        assert!(out.sparks > stock.sparks);
        assert!(out.speed > stock.speed);
        assert!(out.rings > stock.rings);
        // And it is hotter: the elimination shifts toward the launch trail's
        // ember rather than simply throwing more white.
        assert!(out.rgba[0] - out.rgba[2] > stock.rgba[0] - stock.rgba[2]);
    }
}
