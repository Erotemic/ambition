//! The knockout beat: the presentation for the moment a stock ends.
//!
//! It draws a bright flash and a spark burst where the body left play, plus a
//! sound. An elimination (last stock) reads bigger than an ordinary stock
//! loss. Star KO and Screen KO variants are not implemented; the games differ
//! on when they apply, and nothing asks for them yet.
//!
//! # Where the body was
//!
//! The intent carries the position; nothing here resolves an entity. The
//! ruleset despawns an eliminated body, so there is no entity to read. D201
//! stops placing a body until its death window closes, so the position is
//! readable where the stock is spent and is published there as a
//! [`ambition_vfx::vfx::KnockoutBeatRequested`]. No previous-frame cache is
//! needed; a non-rollback cache could answer from an abandoned branch after a
//! rewind.
//!
//! # Where the beat is drawn
//!
//! At the death site, held inside the frame by the beat's own size. See
//! [`beat_anchor`].
//!
//! The genre rule is that the knockout is visible: the flash lands at the
//! screen edge the body left through.
//!
//! A clamp into the visible rect is not enough. The camera's cast framing keeps
//! every live fighter on screen up to the blast line, so the death site is
//! already inside the frame (measured: 3.9 to 17.3 units from the edge). What
//! goes off screen is the burst: it reaches ~150 units, and the camera then
//! moves toward the survivors. Deflating the rect by the burst's reach fixes
//! both, because reach and lifetime come from the same numbers.

use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;
use ambition_sfx::{ids, SfxMessage, SfxWriter};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

/// Sparks thrown by an ordinary stock loss, and by an elimination.
///
/// The gap is the read: the knockout that takes a fighter out is the one that
/// matters. `FighterStockSpent::eliminated` is the only distinction the
/// simulation publishes.
const STOCK_SPARKS: u32 = 26;
const ELIMINATION_SPARKS: u32 = 48;

/// Extra sparks at the top of the launch band, on top of whichever base above
/// applies.
///
/// The band is the launch trail's (`flight_intensity`). A knockout ends a
/// flight, so the burst must score the speed the same way as the plume that
/// led into it. This measures the body's flight, not the hit's weight (which
/// drives hitlag, the strong-hit flash, and camera shake).
const SPEED_SPARKS: u32 = 22;

/// How fast the burst leaves, in world units per second. Fast and short:
/// `ParticleKind::Spark` shrinks and falls, where the trail's `Dust` grows
/// and hangs.
const STOCK_SPARK_SPEED: f32 = 520.0;
const ELIMINATION_SPARK_SPEED: f32 = 720.0;

/// Hot white. The elimination moves toward the launch trail's ember, so the
/// hardest outcome has the colour of the hardest launch.
const STOCK_RGBA: [f32; 4] = [1.0, 0.98, 0.92, 0.95];
const ELIMINATION_RGBA: [f32; 4] = [1.0, 0.72, 0.34, 1.0];

/// How big the ring is drawn. Compared with other scales (blink arrival 0.35,
/// grenade 0.7, bomb 1.0), a stock loss is at the top of the ordinary band and
/// an elimination is the largest effect in the match.
const STOCK_RING_SCALE: f32 = 0.8;
const ELIMINATION_RING_SCALE: f32 = 1.25;

/// What one knockout asks for. Pure, so tests check the rule without a
/// renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnockoutBeat {
    pub sparks: u32,
    pub speed: f32,
    pub rgba: [f32; 4],
    /// How big the expanding ring is drawn, as [`VfxMessage::Effect`] scale.
    ///
    /// Not `VfxMessage::Impact`: with assets that is the ordinary hit marker
    /// (`GENERIC_HIT_FX`), and two coincident copies only double the alpha.
    /// The beat names `ids::SHOCKWAVE` ("the expanding ring a committed heavy
    /// throws") and scales it, so an elimination is actually bigger.
    pub ring_scale: f32,
}

/// The beat a knockout asks for.
///
/// `eliminated` is `FighterStockSpent::eliminated`, the simulation's answer
/// to "was that the last stock"; do not compare `remaining` with zero here.
/// `flight_speed` is the body's speed when it left play, scored on the
/// launch trail's band. See [`SPEED_SPARKS`].
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
/// Asks the spawner: [`crate::fx::burst_reach`] uses the same drag and
/// lifetime constants as `spawn_burst`, so retuning carries through.
pub fn beat_reach(beat: &KnockoutBeat) -> f32 {
    // The ring is part of the beat. A clip draws at
    // `FX_DEFAULT_WORLD_SIZE * scale`, so it reaches half that from its
    // centre. Today the sparks are much wider (~150 against 35), but the
    // `max` keeps a bigger ring or slower burst from spilling off the frame.
    let ring = crate::fx::FX_DEFAULT_WORLD_SIZE * beat.ring_scale / 2.0;
    crate::fx::burst_reach(beat.speed, ParticleKind::Spark).max(ring)
}

/// Where the beat is drawn: the death site, held far enough inside the frame
/// for the whole beat to be on it.
///
/// The rect is one frame old. The camera resolve (`CameraObservationSet`)
/// runs after the presentation visual chain in the shipped app
/// (`RoomTransitionCoverSet` is after `PresentationVisualSync` and feeds the
/// layout that runs before the resolve). Ordering this system after the
/// resolve would create a cycle and panic at schedule init. The lag is
/// deterministic, and the frame edge moves only a few units per tick against
/// an inset of a hundred or more.
///
/// `centre` and `visible` are the presented view's
/// [`ambition_sim_view::CameraViewState::center_world`] and `visible_view`:
/// the published camera rect.
///
/// A knockout in open air is not moved. A beat that would spill moves the
/// least distance that keeps it on screen. When the frame is narrower than
/// the beat, the deflated rect is empty and the beat takes the frame centre,
/// like the camera clamp.
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

/// Draw the knockout beat for every knockout the simulation published.
///
/// Reads a quarantined message, so delivery follows the message channel:
/// journalled by producing frame, replaced on resimulation, released when
/// confirmed, and discarded with an abandoned branch. Each knockout is drawn
/// once, and only for a confirmed timeline. Sampling a per-advance resource
/// once per render frame would miss intermediate advances and draw
/// speculative ones.
pub fn emit_knockout_beat(
    mut knockouts: MessageReader<ambition_vfx::vfx::KnockoutBeatRequested>,
    // The presented view only. The beat is world-space, so one knockout gets
    // one burst however many views watch. `PresentedViewState` refuses to
    // guess with several cameras; then the beat draws at the death site.
    presented: ambition_sim_view::PresentedViewState,
    mut vfx: MessageWriter<VfxMessage>,
    mut sfx: SfxWriter,
) {
    let frame = presented
        .get()
        .map(|view| (view.center_world, view.visible_view));
    for knockout in knockouts.read() {
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

    /// The geometry of a real knockout from a CPU match (`match_shots --on-ko`):
    /// a body leaving bottom-right at 1261 units/s, 12 units in from the nearest
    /// frame edge.
    const MEASURED_KO: Vec2 = Vec2::new(929.0, 701.0);
    const MEASURED_FRAME_CENTRE: Vec2 = Vec2::new(320.0, 369.0);
    const MEASURED_FRAME_SIZE: Vec2 = Vec2::new(1242.0, 699.0);

    /// How far past the frame's edge `pos` is, given a beat of `reach`.
    /// Negative means the whole beat is on screen.
    fn spill(pos: Vec2, reach: f32, centre: Vec2, visible: Vec2) -> f32 {
        let half = visible / 2.0;
        ((pos.x - centre.x).abs() - half.x).max((pos.y - centre.y).abs() - half.y) + reach
    }

    /// A beat clamped onto the deflated rect's edge has almost zero spill, and
    /// this test computes it another way, so it needs a tolerance. Half a world
    /// unit is under half a pixel at the fixture framing (1242 units across
    /// 960 px).
    const SUB_PIXEL: f32 = 0.5;

    /// The beat is drawn where all of it is on screen, and that is not the death
    /// site.
    ///
    /// The second half is the falsifier: a clamp into the visible rect would not
    /// move this point, because the death site is already inside the frame. The
    /// burst's reach is what spills.
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

    /// A knockout with room around it is drawn exactly where the body left.
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

    /// A frame too small for the beat centres it, instead of pinning it to an
    /// edge of the inverted deflated rect.
    #[test]
    fn a_frame_narrower_than_the_beat_centres_it() {
        let tiny = Vec2::new(40.0, 40.0);
        assert_eq!(
            beat_anchor(MEASURED_KO, 200.0, MEASURED_FRAME_CENTRE, tiny),
            MEASURED_FRAME_CENTRE
        );
    }

    /// An elimination is the bigger beat in every term.
    #[test]
    fn the_last_stock_reads_bigger_than_an_ordinary_one() {
        let stock = knockout_beat(false, 0.0);
        let out = knockout_beat(true, 0.0);
        assert!(out.sparks > stock.sparks);
        assert!(out.speed > stock.speed);
        assert!(out.ring_scale > stock.ring_scale);
        // It is also hotter: it moves toward the trail's ember.
        assert!(out.rgba[0] - out.rgba[2] > stock.rgba[0] - stock.rgba[2]);
    }

    /// A knockout scores the flight it ended, on the same band as the plume.
    ///
    /// A body crawling over the line still gets a beat; a full launch gets a
    /// bigger one; past the trail's saturation neither grows, so the burst
    /// agrees with the plume.
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
        // The elimination premium holds at both speed extremes.
        assert!(knockout_beat(true, 0.0).sparks > crawl.sparks);
        assert!(knockout_beat(true, 10_000.0).sparks > hard.sparks);
    }

    /// The system reads the camera.
    ///
    /// The same knockout is emitted into a composition with a presented view and
    /// one without. Wiring that ignored the camera would put both bursts at the
    /// death site.
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
        use ambition_sim_view::{CameraViewState, LocalView};
        use ambition_vfx::vfx::KnockoutBeatRequested;
        use bevy::prelude::App;

        let mut app = App::new();
        app.add_message::<VfxMessage>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<KnockoutBeatRequested>();
        app.world_mut().write_message(KnockoutBeatRequested {
            pos: MEASURED_KO,
            eliminated: false,
            speed: 1261.0,
        });
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
