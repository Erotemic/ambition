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
//!
//! # Where the beat is DRAWN
//!
//! ⭐ NOT where the body was — where the body was, HELD INSIDE THE FRAME BY THE
//! BEAT'S OWN SIZE. See [`beat_anchor`].
//!
//! The genre's rule is that you can SEE the knockout: the flash lands at the
//! screen edge the body left through, and from Brawl on a corner knockout is
//! drawn differently *only to improve the explosion's visibility*. That is the
//! one thing the games agree on, so that is what this ships — the mechanism
//! (Brawl's re-pointed art) is theirs and does not port to a radial burst.
//!
//! ⛔ AND THE FIX IS NOT A CLAMP INTO THE VISIBLE RECT, which is what this
//! looked like it needed. Measured over 22 knockouts across a two-seat and a
//! four-seat CPU match, the death site is ALWAYS INSIDE the frame — 3.9 to 17.3
//! world units in from the nearest edge, never outside — because the camera's
//! cast framing keeps every live fighter on screen right up to the blast line.
//! A clamp into the rect would have moved none of them. What is off screen is
//! the beat's THROW: the burst reaches ~150 units from a centre sitting ~17
//! units inside the edge, so nine tenths of it is drawn past the frame, and the
//! camera then collapses toward the survivors at a few hundred units a second
//! and leaves the rest behind within five ticks of a beat that lives twenty.
//! Deflating the rect by the burst's own reach answers both at once, because
//! the beat's reach and its lifetime are the same quantity said twice.

use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;
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

/// How big the ring is drawn. Against the scales already in the tree — a blink
/// arrival at 0.35, a grenade at 0.7, a bomb at 1.0 — an ordinary stock loss
/// sits at the top of the ordinary band and an elimination is the largest thing
/// the match draws, which is the read the genre gives the last stock.
const STOCK_RING_SCALE: f32 = 0.8;
const ELIMINATION_RING_SCALE: f32 = 1.25;

/// What one knockout asks for. Pure, so the whole rule is asserted without a
/// renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnockoutBeat {
    pub sparks: u32,
    pub speed: f32,
    pub rgba: [f32; 4],
    /// How big the expanding ring is drawn, as [`VfxMessage::Effect`] scale.
    ///
    /// ⛔⛔ THIS USED TO BE A COUNT OF `VfxMessage::Impact`s, AND THAT WAS TWO
    /// DEFECTS IN ONE. `Impact` is the ordinary hit-marker API: asset-backed it
    /// resolves to `GENERIC_HIT_FX` (`hit_soft`), and only the ART-LESS fallback
    /// is an expanding ring — so a shipped game drew the normal damage marker at
    /// the blast line and called it a knockout. And "an elimination gets a
    /// SECOND one" bought nothing: two copies of one clip at one position on one
    /// tick are coincident, so it doubled the alpha and never the size.
    ///
    /// ⭐ THE VOCABULARY ALREADY HAD THE RIGHT NAME. `ids::SHOCKWAVE`
    /// (`shockwave`) is authored as *"the expanding ring a committed heavy
    /// throws"*, so the beat NAMES its effect and SCALES it, and an elimination
    /// reads bigger by being bigger.
    pub ring_scale: f32,
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
            ring_scale: ELIMINATION_RING_SCALE,
        }
    } else {
        KnockoutBeat {
            sparks: STOCK_SPARKS,
            speed: STOCK_SPARK_SPEED,
            rgba: STOCK_RGBA,
            ring_scale: STOCK_RING_SCALE,
        }
    };
    KnockoutBeat {
        sparks: base.sparks + (SPEED_SPARKS as f32 * hard).round() as u32,
        ..base
    }
}

/// How far this beat throws, in world units.
///
/// ⭐ ASKED OF THE SPAWNER, not modelled here. `spawn_burst` owns the drag and
/// the lifetime spread that decide how far a particle gets, and
/// [`crate::fx::burst_reach`] is that same arithmetic read off those constants —
/// so a beat that is re-tuned, or a drag that moves, carries this with it.
pub fn beat_reach(beat: &KnockoutBeat) -> f32 {
    // ⭐ THE RING IS PART OF THE BEAT, so the anchor has to know how wide it is:
    // an effect clip draws at `FX_DEFAULT_WORLD_SIZE * scale`, so it reaches half
    // that from its centre. Today the sparks are the wider of the two by a long
    // way (~150 units against 35 at the elimination scale) and this `max` picks
    // them; it is written as a max rather than dropped so a bigger ring, or a
    // slower burst, cannot quietly start spilling off the frame edge.
    let ring = crate::fx::FX_DEFAULT_WORLD_SIZE * beat.ring_scale / 2.0;
    crate::fx::burst_reach(beat.speed, ParticleKind::Spark).max(ring)
}

/// Where the beat is drawn: the death site, held far enough inside the frame
/// for the whole beat to be on it.
///
/// ⛔ THE RECT IS ONE FRAME OLD, and it cannot be otherwise here. The camera
/// resolve (`CameraObservationSet`) runs DOWNSTREAM of the presentation visual
/// chain in the shipped app — `RoomTransitionCoverSet` sits after
/// `PresentationVisualSync` and feeds the gameplay presentation layout, which
/// is ordered before the resolve — so asking this system to run after the
/// resolve closes a cycle and every composed test panics at schedule init. The
/// ordering is deterministic rather than racy, and the staleness is small
/// against what it is used for: the frame edge moves at most a few units per
/// tick even during the collapse a knockout triggers, against an inset of a
/// hundred and more.
///
/// `centre` and `visible` are the presented view's own
/// [`ambition_sim_view::CameraViewState::center_world`] / `visible_view` — the
/// published camera rect, never a second copy resolved here.
///
/// A knockout in open air is untouched, because the deflated rect already
/// contains it; only a beat that would spill moves, and it moves the least
/// distance that puts it on screen. When the frame is narrower than the beat is
/// wide the deflated rect is empty, and the beat takes the frame's centre —
/// the same degenerate answer the camera clamp itself gives, rather than an
/// arbitrary edge.
pub fn beat_anchor(pos: Vec2, reach: f32, centre: Vec2, visible: Vec2) -> Vec2 {
    let half = visible / 2.0 - Vec2::splat(reach);
    let axis = |value: f32, mid: f32, half: f32| {
        if half >= 0.0 {
            value.clamp(mid - half, mid + half)
        } else {
            mid
        }
    };
    Vec2::new(axis(pos.x, centre.x, half.x), axis(pos.y, centre.y, half.y))
}

/// Draw the knockout beat for every knockout published this tick.
///
/// Runs on the render clock and samples on the SIM clock, like the launch
/// trail: a frame that advanced no tick must not draw the same knockout twice.
pub fn emit_knockout_beat(
    tick: Res<SimTick>,
    mut last_sampled: Local<Option<u64>>,
    knockouts: Res<ambition_sim_view::KnockoutsView>,
    // ⛔ THE PRESENTED view, not every view. The beat is world-space vfx, so one
    // knockout owes ONE burst however many views are watching; iterating views
    // would write a burst per view at the same place. `PresentedViewState`
    // refuses to guess when several cameras exist, and an unframed beat draws
    // where the body was, which is the honest answer with no screen to hold it
    // on.
    presented: ambition_sim_view::PresentedViewState,
    mut vfx: MessageWriter<VfxMessage>,
    mut sfx: SfxWriter,
) {
    if *last_sampled == Some(tick.0) {
        return;
    }
    *last_sampled = Some(tick.0);
    let frame = presented
        .get()
        .map(|view| (view.center_world, view.visible_view));
    for knockout in &knockouts.0 {
        let beat = knockout_beat(knockout.eliminated, knockout.speed);
        let pos = match frame {
            Some((centre, visible)) => {
                beat_anchor(knockout.pos, beat_reach(&beat), centre, visible)
            }
            None => knockout.pos,
        };
        vfx.write(VfxMessage::Effect {
            pos,
            fx: ambition_vfx::fx::ids::SHOCKWAVE,
            scale: beat.ring_scale,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
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

    /// The geometry of a REAL knockout, read off a CPU match rather than chosen.
    ///
    /// `match_shots --on-ko` photographed this one: a body leaving through the
    /// bottom-right at 1261 units/s, 12 units in from the nearest frame edge.
    /// It is the fixture because a hand-picked corner would be a guess about how
    /// close to the edge a knockout actually lands, and the measured answer —
    /// single digits — is closer than anyone would have guessed.
    const MEASURED_KO: Vec2 = Vec2::new(929.0, 701.0);
    const MEASURED_FRAME_CENTRE: Vec2 = Vec2::new(320.0, 369.0);
    const MEASURED_FRAME_SIZE: Vec2 = Vec2::new(1242.0, 699.0);

    /// How far past the frame's edge `pos` is, given a beat of `reach`.
    /// Negative means the whole beat is on screen.
    fn spill(pos: Vec2, reach: f32, centre: Vec2, visible: Vec2) -> f32 {
        let half = visible / 2.0;
        ((pos.x - centre.x).abs() - half.x).max((pos.y - centre.y).abs() - half.y) + reach
    }

    /// A beat clamped exactly onto the deflated rect's edge lands within a
    /// rounding step of zero spill, and this recomputes the same quantity by a
    /// different route, so it needs a floor. Half a world unit: at the framing
    /// these fixtures were measured in — 1242 world units across 960 pixels —
    /// that is under half a pixel, which is smaller than the thing being drawn
    /// can express.
    const SUB_PIXEL: f32 = 0.5;

    /// The beat is drawn where ALL OF IT is on screen — and the death site is
    /// not that place.
    ///
    /// The second half is the falsifier: without it this asserts nothing a
    /// no-op clamp would fail, which is exactly what a clamp INTO the visible
    /// rect would have been here. The camera's cast framing already keeps the
    /// death site inside the frame; it is the burst's reach that spills.
    #[test]
    fn a_knockout_at_the_frame_edge_is_drawn_where_its_whole_burst_fits() {
        let beat = knockout_beat(false, 1261.0);
        let reach = beat_reach(&beat);
        assert!(
            spill(
                MEASURED_KO,
                reach,
                MEASURED_FRAME_CENTRE,
                MEASURED_FRAME_SIZE
            ) > 0.0,
            "the fixture's own death site already fits the frame, so this test \
             cannot tell a clamp from a no-op"
        );
        let drawn = beat_anchor(
            MEASURED_KO,
            reach,
            MEASURED_FRAME_CENTRE,
            MEASURED_FRAME_SIZE,
        );
        assert!(
            spill(drawn, reach, MEASURED_FRAME_CENTRE, MEASURED_FRAME_SIZE) <= SUB_PIXEL,
            "a {reach:.0}-unit beat drawn at {drawn:?} still spills out of a \
             {MEASURED_FRAME_SIZE:?} frame centred {MEASURED_FRAME_CENTRE:?}"
        );
    }

    /// A knockout with room around it is drawn EXACTLY where the body left.
    #[test]
    fn a_knockout_in_open_air_is_not_moved() {
        let beat = knockout_beat(true, 900.0);
        let inside = MEASURED_FRAME_CENTRE + Vec2::new(30.0, -40.0);
        assert_eq!(
            beat_anchor(
                inside,
                beat_reach(&beat),
                MEASURED_FRAME_CENTRE,
                MEASURED_FRAME_SIZE
            ),
            inside
        );
    }

    /// A frame too small to hold the beat centres it rather than pinning it to
    /// an edge the deflation has turned inside out.
    #[test]
    fn a_frame_narrower_than_the_beat_centres_it() {
        let tiny = Vec2::new(40.0, 40.0);
        assert_eq!(
            beat_anchor(MEASURED_KO, 200.0, MEASURED_FRAME_CENTRE, tiny),
            MEASURED_FRAME_CENTRE
        );
    }

    /// An elimination is the bigger beat, and every term of it says so.
    #[test]
    fn the_last_stock_reads_bigger_than_an_ordinary_one() {
        let stock = knockout_beat(false, 0.0);
        let out = knockout_beat(true, 0.0);
        assert!(out.sparks > stock.sparks);
        assert!(out.speed > stock.speed);
        assert!(out.ring_scale > stock.ring_scale);
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

    /// THE SYSTEM READS THE CAMERA, and the pair is the proof.
    ///
    /// The same knockout is emitted twice, into a composition that has a
    /// presented view and into one that has none. A wiring that never looked at
    /// the camera would put both bursts at the death site — which is what four
    /// green-and-inert mechanics on this stage did before somebody ran them.
    #[test]
    fn the_beat_is_placed_by_the_presented_views_own_rect() {
        let framed = emitted_burst_pos(true);
        let unframed = emitted_burst_pos(false);
        assert_eq!(
            unframed, MEASURED_KO,
            "with no view to hold it on, the beat belongs at the death site"
        );
        assert_ne!(
            framed, MEASURED_KO,
            "the beat was drawn at the death site even with a camera rect on \
             hand, so nothing consumed it"
        );
        let beat = knockout_beat(false, 1261.0);
        assert!(
            spill(
                framed,
                beat_reach(&beat),
                MEASURED_FRAME_CENTRE,
                MEASURED_FRAME_SIZE
            ) <= SUB_PIXEL
        );
    }

    /// Where one knockout's spark burst was asked for, with or without a view.
    fn emitted_burst_pos(with_view: bool) -> Vec2 {
        use ambition_sim_view::{CameraViewState, KnockoutFact, KnockoutsView, LocalView};
        use bevy::prelude::App;

        let mut app = App::new();
        app.init_resource::<SimTick>();
        app.add_message::<VfxMessage>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.insert_resource(KnockoutsView(vec![KnockoutFact {
            pos: MEASURED_KO,
            eliminated: false,
            speed: 1261.0,
        }]));
        if with_view {
            app.world_mut().spawn((
                LocalView,
                CameraViewState {
                    center_world: MEASURED_FRAME_CENTRE,
                    visible_view: MEASURED_FRAME_SIZE,
                    ..Default::default()
                },
            ));
            app.world_mut()
                .spawn(ambition_platformer2d_shared_tangle::camera_layers::MainCamera);
        }
        app.add_systems(bevy::prelude::Update, emit_knockout_beat);
        app.update();
        let drawn = app
            .world_mut()
            .resource_mut::<bevy::prelude::Messages<VfxMessage>>()
            .drain()
            .find_map(|message| match message {
                VfxMessage::Burst { pos, .. } => Some(pos),
                _ => None,
            });
        drawn.expect("a knockout owes a burst")
    }
}
