//! The cover over a shell route's first room, until the room is drawn.
//!
//! The route's barrier completes when the first room's art is decoded and
//! uploaded (`first_room_art`), and the route activates: the room is built
//! that frame, and its look comes in over the frames after it (the terrain's
//! skin, the doors' art, what the first tick spawns, pipelines compiling).
//! With no cover, the player saw the room in its placeholder art first.
//!
//! An opaque cover goes up when a platformer session is published and its
//! first room is being prepared, and stays after the route activates until
//! a sim tick has run and nothing of the room is still coming in
//! (`PresentationSettling`), as the cover of a room transition does.

use std::time::Duration;

use bevy::prelude::*;

use ambition_platformer2d::platformer::lifecycle::SessionRoot;
use ambition_platformer2d::provider::PreparedPlatformerSessions;
use ambition_platformer2d::render::rendering::presentation_settling::{PresentationSettling, SettledLatch};
use ambition_platformer2d::time::SimTick;

/// How long the cover waits, after the route activated, for the room to be
/// drawn. Past it, showing the room as it is beats a black screen, and the
/// warning names what was still coming.
const PRESENTATION_DEADLINE: Duration = Duration::from_secs(8);

#[derive(Component)]
pub(crate) struct FirstRoomCoverRoot;

#[derive(Default, Debug)]
enum Phase {
    #[default]
    Idle,
    /// A session is published and its first room is being prepared.
    Preparing,
    /// The route activated: waiting for the room to be drawn.
    Presenting { tick: u64, since: Duration },
}

#[derive(Resource, Default, Debug)]
pub(crate) struct FirstRoomCover {
    phase: Phase,
    update: u64,
    presented: SettledLatch,
}

pub(crate) fn cover_first_room(
    mut commands: Commands,
    mut cover: ResMut<FirstRoomCover>,
    sessions: Option<Res<PreparedPlatformerSessions>>,
    session_roots: Query<(), With<SessionRoot>>,
    settling: Option<Res<PresentationSettling>>,
    tick: Option<Res<SimTick>>,
    time: Res<Time<Real>>,
    roots: Query<Entity, With<FirstRoomCoverRoot>>,
) {
    cover.update = cover.update.saturating_add(1);
    let preparing = sessions.as_deref().is_some_and(|sessions| sessions.published().next().is_some());
    let tick = tick.map_or(0, |tick| tick.get());
    let lift = |commands: &mut Commands| {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
    };
    match cover.phase {
        Phase::Idle => {
            if preparing {
                commands.spawn((
                    FirstRoomCoverRoot,
                    ambition_platformer2d::render::rendering::presentation_settling::PresentationCover,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                    // Under the load screen and the menu (1000), as the cover of a
                    // room transition is.
                    GlobalZIndex(900),
                    Name::new("first room cover"),
                ));
                cover.phase = Phase::Preparing;
            }
        }
        Phase::Preparing => {
            if preparing {
                return;
            }
            if session_roots.is_empty() {
                // Cancelled, or failed: nothing to draw.
                lift(&mut commands);
                cover.phase = Phase::Idle;
            } else {
                cover.presented = SettledLatch::default();
                cover.phase = Phase::Presenting { tick, since: time.elapsed() };
            }
        }
        Phase::Presenting { tick: activated_at, since } => {
            // The activation sets the tick back to 0, so a step ran when it is
            // other than it was and not 0.
            let ticked = tick != activated_at && tick > 0;
            let settled = settling.as_deref().is_none_or(PresentationSettling::is_empty);
            let update = cover.update;
            let presented = cover.presented.observe(update, ticked && settled);
            let waited = time.elapsed().saturating_sub(since);
            if !presented {
                if waited < PRESENTATION_DEADLINE {
                    return;
                }
                bevy::log::warn!(
                    target: "ambition_app::first_room_cover",
                    "first room revealed after {:.0} ms with the room still coming in (ticked={ticked}): {:?}",
                    waited.as_secs_f64() * 1000.0,
                    settling.as_deref().map(PresentationSettling::pending).unwrap_or_default(),
                );
            }
            eprintln!("[first-room-cover] revealed {:.0} ms after the route activated", waited.as_secs_f64() * 1000.0);
            lift(&mut commands);
            cover.phase = Phase::Idle;
        }
    }
}
