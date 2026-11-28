//! Cover the first room until its presentation is ready.

use std::time::Duration;

use bevy::prelude::*;

use crate::platformer::lifecycle::SessionRoot;
use crate::provider::PreparedPlatformerSessions;
use crate::render::rendering::presentation_settling::{PresentationSettling, SettledLatch};
use crate::render::rendering::room_transition_cover::{
    spawn_lifting_cover, RoomTransitionStyles, ROOM_TRANSITION_COVER_Z,
};
use crate::time::SimTick;

use super::room_transition_presentation::RoomTransitionPresentationConfig;

/// How long the cover waits, after the route activated, for the room to be
/// drawn. Past it, showing the room as it is beats a black screen, and the
/// warning names what was still coming.
const PRESENTATION_DEADLINE: Duration = Duration::from_secs(8);

#[derive(Component)]
pub(crate) struct FirstRoomCoverRoot;

#[derive(Component)]
pub(crate) struct FirstRoomLoadingLabel;

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
    covered_at: Duration,
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
    mut labels: Query<&mut Visibility, With<FirstRoomLoadingLabel>>,
    config: Res<RoomTransitionPresentationConfig>,
    styles: Option<Res<RoomTransitionStyles>>,
) {
    cover.update = cover.update.saturating_add(1);
    let preparing = sessions
        .as_deref()
        .is_some_and(|sessions| sessions.published().next().is_some());
    let tick = tick.map_or(0, |tick| tick.get());
    if time.elapsed().saturating_sub(cover.covered_at) >= config.loading_reveal_after {
        for mut visibility in &mut labels {
            visibility.set_if_neq(Visibility::Inherited);
        }
    }
    let lift = |commands: &mut Commands| {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
    };
    match cover.phase {
        Phase::Idle => {
            if preparing {
                commands
                    .spawn((
                        FirstRoomCoverRoot,
                        crate::presentation::gameplay_presentation::PresentationCover,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::BLACK),
                        GlobalZIndex(ROOM_TRANSITION_COVER_Z),
                        Name::new("first room cover"),
                    ))
                    .with_children(|root| {
                        root.spawn((
                            FirstRoomLoadingLabel,
                            Text::new("Loading..."),
                            TextFont {
                                font_size: FontSize::Px(24.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.93, 0.95, 1.0)),
                            Visibility::Hidden,
                        ));
                    });
                cover.covered_at = time.elapsed();
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
                cover.phase = Phase::Presenting {
                    tick,
                    since: time.elapsed(),
                };
            }
        }
        Phase::Presenting {
            tick: activated_at,
            since,
        } => {
            // The activation sets the tick back to 0, so a step ran when it is
            // other than it was and not 0.
            let ticked = tick != activated_at && tick > 0;
            let settled = settling
                .as_deref()
                .is_none_or(PresentationSettling::is_empty);
            let update = cover.update;
            let presented = cover.presented.observe(update, ticked && settled);
            let waited = time.elapsed().saturating_sub(since);
            if !presented {
                if waited < PRESENTATION_DEADLINE {
                    return;
                }
                bevy::log::warn!(
                    target: "ambition_app::first_room_cover",
                    "first room revealed after {:.0} ms (ticked={ticked}): {:?}",
                    waited.as_secs_f64() * 1000.0,
                    settling.as_deref().map(PresentationSettling::pending).unwrap_or_default(),
                );
            }
            bevy::log::info!(
                "[first-room-cover] revealed {:.0} ms after the route activated",
                waited.as_secs_f64() * 1000.0
            );
            lift(&mut commands);
            // The room comes in as a room change's does, in the game's look.
            if !roots.is_empty() {
                spawn_lifting_cover(
                    &mut commands,
                    styles
                        .as_deref()
                        .map(|styles| styles.default)
                        .unwrap_or_default(),
                );
            }
            cover.phase = Phase::Idle;
        }
    }
}
