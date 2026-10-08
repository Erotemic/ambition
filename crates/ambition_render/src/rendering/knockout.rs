//! The knockout beat: the presentation for the moment a stock ends.
//!
//! A knockout through the blast envelope is drawn as a blast from the place
//! the body left the screen, shot back along the line it flew in on: a column
//! of light and a jet of streaks, as in the platform fighters this genre
//! names. The body is not drawn after it crosses (`KnockedOutOfTheWorld`), so
//! the blast is the last thing the player sees of it. An elimination (last
//! stock) is the bigger blast.
//!
//! # Where the body was
//!
//! The intent carries the position and the velocity; nothing here resolves an
//! entity. The ruleset despawns an eliminated body, so there is no entity to
//! read. D201 stops placing a body until its death window closes, so the
//! position is readable where the stock is spent and is published there as a
//! [`ambition_vfx::vfx::KnockoutBeatRequested`]. A non-rollback cache could
//! answer from an abandoned branch after a rewind, so there is none.
//!
//! # Where the blast is drawn
//!
//! Its base is the death site, held a few units inside the presented frame
//! ([`blast_base`]): the camera keeps every live fighter on screen up to the
//! blast line, so the body leaves at the frame edge, and the blast comes out
//! of that edge. It points away from the edge, into the frame
//! ([`blast_axis`]), so it does not spill off the screen.

use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;
use ambition_sfx::{ids, SfxMessage, SfxWriter};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};
use ambition_vfx::vfx::VfxWriter;
#[cfg(test)]
use ambition_vfx::vfx::VfxInRoom;

/// What one knockout blast asks for. Pure, so tests check the rule without a
/// renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnockoutBeat {
    /// How far the column of light shoots, in world units.
    pub length: f32,
    /// How wide its glow is; the core is [`CORE_WIDTH_FRACTION`] of it.
    pub width: f32,
    /// How long the column lasts, in seconds.
    pub seconds: f32,
    /// The streaks of the jet, and how fast they leave.
    pub streaks: u32,
    pub speed: f32,
    /// The glow and the jet's colour; the core is near white.
    pub glow: [f32; 4],
    pub core: [f32; 4],
}

/// An ordinary stock loss: a gold blast.
const STOCK_LENGTH: f32 = 380.0;
const STOCK_WIDTH: f32 = 90.0;
const STOCK_SECONDS: f32 = 0.55;
const STOCK_STREAKS: u32 = 28;
const STOCK_STREAK_SPEED: f32 = 1300.0;
const STOCK_GLOW: [f32; 4] = [1.0, 0.80, 0.28, 0.60];
const STOCK_CORE: [f32; 4] = [1.0, 0.99, 0.92, 1.0];

/// The last stock: the largest effect in the match, toward the launch trail's
/// ember, so the hardest outcome has the colour of the hardest launch.
const ELIMINATION_LENGTH: f32 = 560.0;
const ELIMINATION_WIDTH: f32 = 130.0;
const ELIMINATION_SECONDS: f32 = 0.75;
const ELIMINATION_STREAKS: u32 = 44;
const ELIMINATION_STREAK_SPEED: f32 = 1700.0;
const ELIMINATION_GLOW: [f32; 4] = [1.0, 0.38, 0.16, 0.70];
const ELIMINATION_CORE: [f32; 4] = [1.0, 0.93, 0.78, 1.0];

/// What the top of the launch band adds, on the launch trail's own band
/// (`flight_intensity`): a knockout ends a flight, so the blast scores the
/// speed the same way as the plume that led into it.
const SPEED_LENGTH: f32 = 220.0;
const SPEED_STREAKS: u32 = 18;

/// The core's width, as a fraction of the glow's.
pub const CORE_WIDTH_FRACTION: f32 = 0.28;

/// The half-angle of the jet of streaks, and of the wider spray of sparks
/// at its foot, in radians.
const JET_SPREAD: f32 = 0.32;
const SPRAY_SPREAD: f32 = 0.95;

/// How far inside the frame edge the blast's base is held, in world units.
pub const EDGE_INSET: f32 = 6.0;

/// The blast a knockout asks for.
///
/// `eliminated` is `FighterStockSpent::eliminated`, the simulation's answer
/// to "was that the last stock"; do not compare `remaining` with zero here.
/// `flight_speed` is the body's speed when it left play.
pub fn knockout_beat(eliminated: bool, flight_speed: f32) -> KnockoutBeat {
    let hard = super::launch_trail::flight_intensity(flight_speed);
    let base = if eliminated {
        KnockoutBeat {
            length: ELIMINATION_LENGTH,
            width: ELIMINATION_WIDTH,
            seconds: ELIMINATION_SECONDS,
            streaks: ELIMINATION_STREAKS,
            speed: ELIMINATION_STREAK_SPEED,
            glow: ELIMINATION_GLOW,
            core: ELIMINATION_CORE,
        }
    } else {
        KnockoutBeat {
            length: STOCK_LENGTH,
            width: STOCK_WIDTH,
            seconds: STOCK_SECONDS,
            streaks: STOCK_STREAKS,
            speed: STOCK_STREAK_SPEED,
            glow: STOCK_GLOW,
            core: STOCK_CORE,
        }
    };
    KnockoutBeat {
        length: base.length + SPEED_LENGTH * hard,
        streaks: base.streaks + (SPEED_STREAKS as f32 * hard).round() as u32,
        ..base
    }
}

/// The direction the blast shoots, a unit vector: back along the flight that
/// ended, into the frame it left. A body that left with no speed has no
/// flight to point back along; its blast points at the frame's centre, or up
/// with no frame.
pub fn blast_axis(launch: Vec2, base: Vec2, frame_centre: Option<Vec2>) -> Vec2 {
    let back = -launch;
    if back.length() > 1.0 {
        return back.normalize();
    }
    frame_centre
        .map(|centre| centre - base)
        .filter(|toward| toward.length() > 1.0)
        .map(Vec2::normalize)
        .unwrap_or(Vec2::new(0.0, -1.0))
}

/// Where the blast comes out: the death site, held [`EDGE_INSET`] inside the
/// presented frame (`centre`, `visible` are the view's
/// [`ambition_sim_view::CameraViewState::center_world`] and `visible_view`).
/// A knockout inside the frame is not moved.
///
/// The rect is one frame old: the camera resolve runs after the presentation
/// visual chain, and ordering this after it would make a schedule cycle. The
/// frame edge moves a few units per tick, and the blast is hundreds long.
pub fn blast_base(pos: Vec2, centre: Vec2, visible: Vec2) -> Vec2 {
    let half = (visible / 2.0 - Vec2::splat(EDGE_INSET)).max(Vec2::ZERO);
    Vec2::new(
        pos.x.clamp(centre.x - half.x, centre.x + half.x),
        pos.y.clamp(centre.y - half.y, centre.y + half.y),
    )
}

/// Draw the knockout blast for every knockout the simulation published.
///
/// Reads a quarantined message, so delivery follows the message channel:
/// journalled by producing frame, replaced on resimulation, released when
/// confirmed, and discarded with an abandoned branch. Each knockout is drawn
/// once, and only for a confirmed timeline.
pub fn emit_knockout_beat(
    mut knockouts: MessageReader<ambition_vfx::vfx::KnockoutBeatRequested>,
    // The presented view only. The blast is world-space, so one knockout gets
    // one blast however many views watch. `PresentedViewState` refuses to
    // guess with several cameras; then the blast comes out of the death site.
    presented: ambition_sim_view::PresentedViewState,
    mut vfx: VfxWriter,
    mut sfx: SfxWriter,
) {
    let frame = presented
        .get()
        .map(|view| (view.center_world, view.visible_view));
    for knockout in knockouts.read() {
        let beat = knockout_beat(knockout.eliminated, knockout.launch.length());
        let base = match frame {
            Some((centre, visible)) => blast_base(knockout.pos, centre, visible),
            None => knockout.pos,
        };
        let toward = blast_axis(knockout.launch, base, frame.map(|(centre, _)| centre));
        let mut vfx = vfx.for_room(knockout.room);
        vfx.write(VfxMessage::Beam {
            pos: base,
            toward,
            length: beat.length,
            width: beat.width,
            color: beat.glow,
            seconds: beat.seconds,
        });
        vfx.write(VfxMessage::Beam {
            pos: base,
            toward,
            length: beat.length * 0.9,
            width: beat.width * CORE_WIDTH_FRACTION,
            color: beat.core,
            seconds: beat.seconds * 0.8,
        });
        vfx.write(VfxMessage::Jet {
            pos: base,
            toward,
            spread: JET_SPREAD,
            count: beat.streaks,
            speed: beat.speed,
            color: [beat.glow[0], beat.glow[1], beat.glow[2], 1.0],
            kind: ParticleKind::Streak,
        });
        vfx.write(VfxMessage::Jet {
            pos: base,
            toward,
            spread: SPRAY_SPREAD,
            count: beat.streaks / 2,
            speed: beat.speed * 0.45,
            color: beat.core,
            kind: ParticleKind::Spark,
        });
        sfx.write(SfxMessage::Play {
            id: ids::WORLD_EXPLOSION,
            pos: base,
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
    const MEASURED_LAUNCH: Vec2 = Vec2::new(1000.0, 768.0);
    const MEASURED_FRAME_CENTRE: Vec2 = Vec2::new(320.0, 369.0);
    const MEASURED_FRAME_SIZE: Vec2 = Vec2::new(1242.0, 699.0);

    /// The blast shoots back along the flight it ended, so it comes out of
    /// the edge the body left through and into the frame.
    #[test]
    fn the_blast_points_back_along_the_flight() {
        let toward = blast_axis(MEASURED_LAUNCH, MEASURED_KO, Some(MEASURED_FRAME_CENTRE));
        assert!((toward.length() - 1.0).abs() < 1e-4, "a unit axis: {toward:?}");
        assert!(
            toward.dot(MEASURED_LAUNCH.normalize()) < -0.999,
            "{toward:?} is not opposite the launch {MEASURED_LAUNCH:?}"
        );
        assert!(
            toward.dot(MEASURED_FRAME_CENTRE - MEASURED_KO) > 0.0,
            "a knockout at the bottom right blasts into the frame"
        );
    }

    /// A body that crossed with no speed has no flight to point back along:
    /// the blast points at the frame's centre.
    #[test]
    fn a_still_knockout_blasts_toward_the_frame() {
        let toward = blast_axis(Vec2::ZERO, MEASURED_KO, Some(MEASURED_FRAME_CENTRE));
        let expected = (MEASURED_FRAME_CENTRE - MEASURED_KO).normalize();
        assert!((toward - expected).length() < 1e-4, "{toward:?} for {expected:?}");
        assert_eq!(blast_axis(Vec2::ZERO, MEASURED_KO, None), Vec2::new(0.0, -1.0));
    }

    /// The base is at the frame edge the body left through, and a knockout
    /// inside the frame is not moved.
    #[test]
    fn the_blast_comes_out_of_the_frame_edge() {
        let outside = MEASURED_FRAME_CENTRE + MEASURED_FRAME_SIZE;
        let base = blast_base(outside, MEASURED_FRAME_CENTRE, MEASURED_FRAME_SIZE);
        let corner = MEASURED_FRAME_CENTRE + MEASURED_FRAME_SIZE / 2.0 - Vec2::splat(EDGE_INSET);
        assert!((base - corner).length() < 1e-3, "{base:?} for the corner {corner:?}");
        let inside = MEASURED_FRAME_CENTRE + Vec2::new(30.0, -40.0);
        assert_eq!(blast_base(inside, MEASURED_FRAME_CENTRE, MEASURED_FRAME_SIZE), inside);
    }

    /// An elimination is the bigger blast in every term.
    #[test]
    fn the_last_stock_reads_bigger_than_an_ordinary_one() {
        let stock = knockout_beat(false, 0.0);
        let out = knockout_beat(true, 0.0);
        assert!(out.length > stock.length);
        assert!(out.width > stock.width);
        assert!(out.seconds > stock.seconds);
        assert!(out.streaks > stock.streaks);
        assert!(out.speed > stock.speed);
        // It is also hotter: it moves toward the trail's ember.
        assert!(out.glow[0] - out.glow[2] > stock.glow[0] - stock.glow[2]);
    }

    /// A knockout scores the flight it ended, on the same band as the plume,
    /// and saturates where the trail's band does.
    #[test]
    fn a_knockout_scores_the_flight_that_ended_it() {
        let crawl = knockout_beat(false, 0.0);
        let hard = knockout_beat(false, 10_000.0);
        assert!(crawl.streaks > 0 && crawl.length > 0.0, "a knockout is always a blast");
        assert!(hard.length > crawl.length && hard.streaks > crawl.streaks);
        assert_eq!(hard, knockout_beat(false, 100_000.0), "it saturates");
        assert!(knockout_beat(true, 10_000.0).length > hard.length);
    }

    /// What one knockout asked the effect layer for: the messages and their
    /// rooms. `with_view` adds a presented camera.
    fn emitted(with_view: bool, room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>) -> Vec<VfxInRoom> {
        use ambition_sim_view::{CameraViewState, LocalView};
        use ambition_vfx::vfx::KnockoutBeatRequested;
        use bevy::prelude::App;

        let mut app = App::new();
        app.add_message::<VfxInRoom>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<KnockoutBeatRequested>();
        app.world_mut().write_message(KnockoutBeatRequested {
            pos: MEASURED_KO + Vec2::new(500.0, 0.0),
            eliminated: false,
            launch: MEASURED_LAUNCH,
            room,
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
        app.world_mut()
            .resource_mut::<bevy::prelude::Messages<VfxInRoom>>()
            .drain()
            .collect()
    }

    /// The blast is two beams and two jets, all aimed one way, and none of the
    /// round explosions (no `Burst`, no `Effect`).
    #[test]
    fn a_knockout_is_a_directional_blast_not_a_round_explosion() {
        let messages = emitted(true, None);
        let mut beams = 0;
        let mut jets = 0;
        for message in &messages {
            match &message.vfx {
                VfxMessage::Beam { toward, .. } | VfxMessage::Jet { toward, .. } => {
                    assert!(toward.dot(MEASURED_LAUNCH.normalize()) < -0.999, "aimed back along the flight");
                    if matches!(message.vfx, VfxMessage::Beam { .. }) {
                        beams += 1;
                    } else {
                        jets += 1;
                    }
                }
                other => panic!("a knockout drew {other:?}, which is not part of the blast"),
            }
        }
        assert_eq!((beams, jets), (2, 2));
    }

    /// The system reads the camera: with a view, the base is held inside the
    /// frame; without one, it is the death site.
    #[test]
    fn the_blast_is_placed_by_the_presented_views_own_rect() {
        let base = |messages: Vec<VfxInRoom>| match messages[0].vfx {
            VfxMessage::Beam { pos, .. } => pos,
            ref other => panic!("the first message is the glow beam, not {other:?}"),
        };
        let death_site = MEASURED_KO + Vec2::new(500.0, 0.0);
        assert_eq!(base(emitted(false, None)), death_site);
        let framed = base(emitted(true, None));
        assert_eq!(framed, blast_base(death_site, MEASURED_FRAME_CENTRE, MEASURED_FRAME_SIZE));
        assert_ne!(framed, death_site, "the death site is off the frame, so the base must move");
    }

    /// A knockout in the second of two live rooms is drawn in that room.
    #[test]
    fn the_blast_is_drawn_in_the_room_the_body_left_play_in() {
        use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
        let second = LiveRoomInstance::ACTIVATION.next();
        let rooms: Vec<_> = emitted(false, Some(second)).into_iter().map(|m| m.room).collect();
        assert_eq!(rooms, vec![Some(second); 4]);
    }
}
