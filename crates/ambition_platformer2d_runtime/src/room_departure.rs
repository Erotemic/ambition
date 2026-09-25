//! Leaving a room because the level in it is done: the one road for "go on".
//!
//! A game says WHEN (its goal, its tally, its results card) by calling
//! [`Departure::leave`] on the component its mode owner carries. The engine
//! says HOW, the same way for every game: resolve where to (the active room's
//! authored `next_room`, a named room, or this room again), keep asking the
//! lifecycle commit to move the player there until the room changes, and give
//! up to a replay, with a warning, if it never does.
//!
//! Mary-O and Sanic each wrote this loop by hand (a dwell, a target resolved
//! from `next_room`, a transition intent recorded every tick until arrival, a
//! replay fallback), and each had to learn separately that its memory must be
//! rollback state. This is that loop once, on a registered component.
//!
//! What a game does on ARRIVAL stays the game's: it sees the active room change
//! (its act clock restarts, its level re-arms) exactly as before.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::session::lifecycle_commit::{
    LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
};
use ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested;
use ambition_platformer2d_shared_tangle::schedule::{Platformer2dSimulationPhaseMonolith, SimScheduleExt};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

pub use ambition_platformer2d_shared_tangle::lifecycle::{
    Departure, DepartureState, Destination, DEPARTURE_GIVE_UP_S,
};

/// The set [`drive_departures`] runs in. A game that asks to leave orders its
/// request `.before(DepartureSet)`, so the ask and its resolution fall on the
/// same tick on every timeline.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DepartureSet;

/// Drive every departure one tick.
///
/// Resolves a new request against the active room, records the transition for
/// the primary player each tick until the active room is the target (the
/// lifecycle slot is earliest-sticky and the room transaction dedupes, so
/// asking again is how a refused or dropped ask is retried), and ends the trip
/// on arrival. A destination the session does not hold, or a trip that never
/// arrives, is a warning and a replay: the level still ends and the player
/// still goes somewhere.
///
/// ⛔ A REPLAY IS RETRIED TOO. `admit_room_replay` refuses a request while
/// another lifecycle operation holds the slot, and a refused request is gone.
/// This used to write the request once and drop back to `Staying`, so a refused
/// replay was lost and a game that waits on `is_leaving()` re-armed its level
/// although nothing had been replayed. The intent now waits in
/// [`DepartureState::Replaying`], re-asked every tick, until a replay is
/// ADMITTED (`RoomReplayAdmitted`) — any replay: the room restarting is the
/// fact the level asked for, whoever asked first.
#[allow(clippy::too_many_arguments)]
pub fn drive_departures(
    time: Res<ambition_time::WorldTime>,
    rooms: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    subjects: Query<&SimId, ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    mut pending: Option<ResMut<PendingLifecycleCommit>>,
    boundary: Option<Res<ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    mut replay: MessageWriter<RoomReplayRequested>,
    mut admitted: MessageReader<ambition_combat::RoomReplayAdmitted>,
    mut departures: Query<&mut Departure>,
) {
    let replay_admitted = admitted.read().count() > 0;
    let Some(rooms) = rooms else {
        return;
    };
    let active = rooms.active_spec().id.clone();
    for mut departure in &mut departures {
        let target = match std::mem::take(&mut departure.state) {
            DepartureState::Staying => continue,
            DepartureState::Replaying { asked } => {
                if replay_admitted {
                    // The room is restarting: the trip is over.
                    continue;
                }
                if asked >= DEPARTURE_GIVE_UP_S {
                    bevy::log::warn!(
                        target: "ambition::room_departure",
                        "the level asked to replay `{active}` and no replay was admitted \
                         in {DEPARTURE_GIVE_UP_S}s; staying"
                    );
                    continue;
                }
                replay.write(RoomReplayRequested::manual());
                departure.state = DepartureState::Replaying {
                    asked: asked + time.scaled_dt,
                };
                continue;
            }
            DepartureState::Requested(to) => match to {
                Destination::Replay => None,
                Destination::Room(room) => Some(room),
                Destination::NextRoom => rooms.active_metadata().next_room.clone(),
            },
            DepartureState::Leaving { target, asked } => {
                if active == target {
                    // Arrived. The game sees the room change and starts over.
                    continue;
                }
                if asked >= DEPARTURE_GIVE_UP_S {
                    bevy::log::warn!(
                        target: "ambition::room_departure",
                        "the level asked to leave for room `{target}` and never arrived \
                         there in {DEPARTURE_GIVE_UP_S}s; replaying `{active}` instead"
                    );
                    replay_this_room(&mut departure, &mut replay);
                    continue;
                }
                departure.state = DepartureState::Leaving {
                    target: target.clone(),
                    asked: asked + time.scaled_dt,
                };
                Some(target)
            }
        };
        let Some(target) = target else {
            replay_this_room(&mut departure, &mut replay);
            continue;
        };
        let Some(arrival) = rooms
            .rooms
            .iter()
            .find(|room| room.id == target)
            .map(|room| room.world.spawn)
        else {
            bevy::log::warn!(
                target: "ambition::room_departure",
                "the level asked to leave for room `{target}`, which this session \
                 does not hold; replaying `{active}` instead"
            );
            replay_this_room(&mut departure, &mut replay);
            continue;
        };
        if !matches!(departure.state, DepartureState::Leaving { .. }) {
            bevy::log::info!(
                target: "ambition::room_departure",
                "the level in `{active}` is done; leaving for `{target}`"
            );
            departure.state = DepartureState::Leaving {
                target: target.clone(),
                asked: 0.0,
            };
        }
        // No body or no lifecycle commit this tick: keep the trip and ask next
        // tick, until the give-up replays.
        let (Ok(subject), Some(pending)) = (subjects.single(), pending.as_deref_mut()) else {
            continue;
        };
        let _ = pending.record(
            boundary.as_deref().map_or(0, |boundary| boundary.current),
            LifecycleIntent::Transition(RoomTransitionIntent {
                subject: subject.clone(),
                target_room: target,
                arrival,
                // Finishing a level is not walking off the side of a room.
                edge_exit: false,
                zone_sfx: None,
            }),
        );
    }
}

/// Ask for this room again, and keep asking until a replay is admitted.
fn replay_this_room(departure: &mut Departure, replay: &mut MessageWriter<RoomReplayRequested>) {
    replay.write(RoomReplayRequested::manual());
    departure.state = DepartureState::Replaying { asked: 0.0 };
}

/// Installs [`drive_departures`] in the simulation. Part of
/// [`crate::PlatformerEnginePlugins`].
pub struct RoomDeparturePlugin;

impl Plugin for RoomDeparturePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.configure_sets(
            sim,
            DepartureSet.in_set(Platformer2dSimulationPhaseMonolith::GameplayEffects),
        );
        app.add_systems(sim, drive_departures.in_set(DepartureSet));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

    fn room(id: &str, next: Option<&str>) -> RoomSpec {
        let world = ae::World::new(id, ae::Vec2::new(640.0, 480.0), ae::Vec2::new(64.0, 400.0), Vec::new());
        let mut spec = RoomSpec::new(id, world);
        spec.metadata.next_room = next.map(str::to_string);
        spec
    }

    /// A session in room `first`, whose `next_room` is `next`, with a player
    /// and a mode owner that has asked to leave for `to`.
    fn app_leaving(next: Option<&str>, to: Destination) -> App {
        let mut app = App::new();
        app.init_resource::<ambition_time::WorldTime>();
        app.init_resource::<PendingLifecycleCommit>();
        app.add_message::<RoomReplayRequested>();
        app.add_message::<ambition_combat::RoomReplayAdmitted>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            RoomSet::from_parts_or_panic("first", vec![room("first", next), room("second", None)], Vec::new()),
        );
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            SimId::player_slot(0),
        ));
        let mut departure = Departure::default();
        departure.leave(to);
        app.world_mut().spawn(departure);
        app.add_systems(Update, drive_departures);
        app
    }

    fn recorded(app: &App) -> Option<String> {
        app.world()
            .resource::<PendingLifecycleCommit>()
            .peek()
            .and_then(|pending| match &pending.kind {
                LifecycleIntent::Transition(transition) => Some(transition.target_room.clone()),
                _ => None,
            })
    }

    fn replays(app: &mut App) -> usize {
        let messages = app.world().resource::<bevy::prelude::Messages<RoomReplayRequested>>();
        messages.get_cursor().read(messages).count()
    }

    #[test]
    fn a_level_leaves_for_the_room_its_next_room_names() {
        let mut app = app_leaving(Some("second"), Destination::NextRoom);
        app.update();
        assert_eq!(recorded(&app).as_deref(), Some("second"));
        assert_eq!(replays(&mut app), 0);
    }

    #[test]
    fn a_level_that_names_no_next_room_replays() {
        let mut app = app_leaving(None, Destination::NextRoom);
        app.update();
        assert_eq!(recorded(&app), None, "nowhere to go is not a transition");
        assert_eq!(replays(&mut app), 1, "it is the same level again");
    }

    fn state(app: &mut App) -> DepartureState {
        let mut q = app.world_mut().query::<&Departure>();
        q.iter(app.world()).next().expect("the departure").state.clone()
    }

    /// A replay the lifecycle slot REFUSES is asked again, through the real
    /// admission, until one is admitted — and only then does the level stop
    /// leaving. The game's own wait (`is_leaving()`) spans the refusal.
    #[test]
    fn a_refused_replay_is_asked_again_until_one_is_admitted() {
        let mut app = app_leaving(None, Destination::Replay);
        // The real admission, ahead of the driver as in the schedule
        // (`PlayerInput` runs before `GameplayEffects`).
        app.add_systems(
            Update,
            crate::sandbox_reset::admit_room_replay.before(drive_departures),
        );
        // Another lifecycle operation owns the earliest-sticky slot.
        let competing = LifecycleIntent::Transition(RoomTransitionIntent {
            subject: SimId::player_slot(0),
            target_room: "second".into(),
            arrival: ae::Vec2::ZERO,
            edge_exit: false,
            zone_sfx: None,
        });
        let _ = app.world_mut().resource_mut::<PendingLifecycleCommit>().record(0, competing);

        app.update(); // the driver asks
        app.update(); // the admission refuses it; the driver asks again
        assert!(
            matches!(state(&mut app), DepartureState::Replaying { .. }),
            "a refused replay is still wanted: {:?}",
            state(&mut app)
        );
        assert_eq!(recorded(&app).as_deref(), Some("second"), "the other operation kept the slot");

        // The other operation completes.
        app.world_mut().resource_mut::<PendingLifecycleCommit>().take();
        app.update(); // admitted this time
        assert_eq!(state(&mut app), DepartureState::Staying, "the room is restarting: the trip is over");
        let pending = app.world().resource::<PendingLifecycleCommit>().peek().cloned();
        assert!(
            pending.is_some_and(|pending| pending.kind.target_room() == "first"),
            "and what holds the slot now is this room's replay"
        );
    }

    #[test]
    fn a_room_the_session_does_not_hold_replays_instead_of_stranding_the_level() {
        let mut app = app_leaving(Some("second"), Destination::Room("nowhere".into()));
        app.update();
        assert_eq!(recorded(&app), None);
        assert_eq!(replays(&mut app), 1);
    }
}
