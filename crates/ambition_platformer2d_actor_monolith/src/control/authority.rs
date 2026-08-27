//! Live participant-to-body driving authority.
//!
//! [`DrivingParticipant`] states who currently drives a body; [`Brain`] remains
//! AI policy and is not replaced during possession. This differs from prepared
//! match `ControlAuthority`, which specifies how a roster seat is initially
//! bound. This module reconciles runtime possession redirects only.

use ambition_characters::control::{DrivingParticipant};
use bevy::prelude::*;

use ambition_characters::control::PlayerSlot;

use crate::abilities::traversal::possession::PossessionState;


/// Redirect the primary seat from the home body to a possessed body and back.
///
/// Outside an active possession this system leaves authored seat assignments
/// unchanged. Writes are conditional so component change ticks are not generated
/// every frame.
pub fn project_driving_participant(
    mut commands: Commands,
    mut state: ResMut<PossessionState>,
    alive: Query<()>,
    held: Query<(Entity, &DrivingParticipant)>,
) {
    // No possession has taken the primary seat, so nobody's seat is in question.
    let Some(home) = state.home else {
        return;
    };
    let seat_of = |entity: Entity| held.get(entity).map(|(_, seat)| seat.0).ok();
    let possessed = state.possessed.filter(|target| alive.get(*target).is_ok());

    match possessed {
        // Live possession: the primary seat sits on the driven body, and the
        // home avatar is inert until it comes back.
        Some(target) => {
            if seat_of(home) == Some(PlayerSlot::PRIMARY) {
                commands.entity(home).try_remove::<DrivingParticipant>();
            }
            if seat_of(target) != Some(PlayerSlot::PRIMARY) {
                commands
                    .entity(target)
                    .try_insert(DrivingParticipant(PlayerSlot::PRIMARY));
            }
        }
        // Released (or the driven body is gone): the seat goes home, and the
        // record that said where home was has done its job.
        None => {
            // RETRACT BEFORE RESTORING, or the release leaves TWO bodies
            // holding one seat. The release site clears `possessed`, so by the
            // time this runs there is nothing left naming the body that was
            // driven — and an earlier version of this branch therefore restored
            // `home` without taking the seat off the vacated actor. Both then
            // answered the primary seat's press, which is the exact two-writer
            // state this whole component exists to make impossible.
            //
            // the sweep is safe because of the guard at the top. This
            // system returns early unless `state.home` is set, and `home` is set
            // only between a possession starting and this branch clearing it. In
            // that window the primary seat belongs to `home` or to the body it
            // possessed and to nobody else, so "any other holder" names the
            // vacated actor precisely. A session that never possesses — a versus
            // match whose seat-0 fighter legitimately holds PRIMARY — never
            // reaches here at all.
            for (entity, seat) in &held {
                if entity != home && seat.0 == PlayerSlot::PRIMARY {
                    commands.entity(entity).try_remove::<DrivingParticipant>();
                }
            }
            if alive.get(home).is_ok() && seat_of(home) != Some(PlayerSlot::PRIMARY) {
                commands
                    .entity(home)
                    .try_insert(DrivingParticipant(PlayerSlot::PRIMARY));
            }
            state.home = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::traversal::possession::PossessionState;
    use ambition_characters::brain::Brain;

    /// Run the reconcile once over a world and read back who drives what.
    fn reconcile(app: &mut App) {
        app.add_systems(Update, project_driving_participant);
        app.update();
    }

    fn driver(app: &App, body: Entity) -> Option<PlayerSlot> {
        app.world().get::<DrivingParticipant>(body).map(|d| d.0)
    }

    /// With nobody possessing anything, the authored seats stand.
    ///
    /// the reconcile may not have an opinion about a body no possession
    /// touched: a seated versus fighter and an adventure session's home avatar
    /// can both hold the primary seat in one world today, and a system that
    /// recomputed the population would unseat one of them.
    #[test]
    fn outside_a_possession_the_authored_seat_is_left_exactly_as_spawned() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let home = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot::PRIMARY))
            .id();
        let cpu = app.world_mut().spawn(Brain::stand_still()).id();
        reconcile(&mut app);

        assert_eq!(driver(&app, home), Some(PlayerSlot::PRIMARY));
        assert_eq!(
            driver(&app, cpu),
            None,
            "an autonomous body was given a participant's authority — `acting_slot` \
             turns that into the primary seat, so a CPU actor would spend a human's \
             buffered press"
        );
    }

    /// THE POINT OF THE TYPE: possession REDIRECTS authority without moving a
    /// policy.
    ///
    /// the target keeps its own `Brain::StateMachine` throughout. There is no
    /// longer any other way for it to go — a `Brain` cannot name a driver — and
    /// this is what says the behaviour did not change when `restore_brain` died.
    #[test]
    fn a_live_possession_moves_the_primary_seats_authority_and_leaves_the_policy() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let home = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot::PRIMARY))
            .id();
        let target = app.world_mut().spawn(Brain::stand_still()).id();
        {
            let mut state = app.world_mut().resource_mut::<PossessionState>();
            state.home = Some(home);
            state.possessed = Some(target);
        }
        reconcile(&mut app);

        assert_eq!(
            driver(&app, target),
            Some(PlayerSlot::PRIMARY),
            "the possessed body is not being driven by the seat that possessed it"
        );
        assert_eq!(
            driver(&app, home),
            None,
            "the vacated home avatar still holds the primary seat's authority, so \
             two bodies answer one seat's press"
        );
        assert!(
            matches!(
                app.world().get::<Brain>(target),
                Some(Brain::StateMachine(_))
            ),
            "the reconcile reached into the target's POLICY — it may only decide \
             who drives, never what the body knows how to do on its own"
        );
    }

    /// A second seat is untouched by the primary's possession.
    ///
    /// the redirect takes the seat from the HOME avatar and from nowhere else;
    /// a version that cleared every `PRIMARY` holder — or every holder — would
    /// silently unseat a co-op partner the moment player one possessed something.
    #[test]
    fn possession_by_the_primary_seat_does_not_unseat_a_second_participant() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let one = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot::PRIMARY))
            .id();
        let two = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot(1)))
            .id();
        let target = app.world_mut().spawn(Brain::stand_still()).id();
        {
            let mut state = app.world_mut().resource_mut::<PossessionState>();
            state.home = Some(one);
            state.possessed = Some(target);
        }
        reconcile(&mut app);

        assert_eq!(driver(&app, target), Some(PlayerSlot::PRIMARY));
        assert_eq!(driver(&app, one), None);
        assert_eq!(
            driver(&app, two),
            Some(PlayerSlot(1)),
            "player two lost their body because player one possessed something"
        );
    }

    /// Authority is RETRACTED, not left behind.
    ///
    /// the release direction, which every latch on this road has got wrong at
    /// least once: a reconcile that only ever inserts leaves the possessed body
    /// driving forever, and the home avatar never gets its seat back.
    #[test]
    fn releasing_the_possession_returns_the_seat_to_the_body_that_owns_it() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let home = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot::PRIMARY))
            .id();
        let target = app.world_mut().spawn(Brain::stand_still()).id();
        {
            let mut state = app.world_mut().resource_mut::<PossessionState>();
            state.home = Some(home);
            state.possessed = Some(target);
        }
        app.add_systems(Update, project_driving_participant);
        app.update();
        assert_eq!(driver(&app, target), Some(PlayerSlot::PRIMARY));

        app.world_mut().resource_mut::<PossessionState>().possessed = None;
        app.update();

        assert_eq!(
            driver(&app, target),
            None,
            "the released body is still holding the seat"
        );
        assert_eq!(
            driver(&app, home),
            Some(PlayerSlot::PRIMARY),
            "the home avatar never got its seat back"
        );
        assert_eq!(
            app.world().resource::<PossessionState>().home,
            None,
            "the `home` record outlived the possession it described — the next \
             body to hold the primary seat would be unseated by it"
        );
    }

    /// A driven body that VANISHED still hands the seat back.
    ///
    /// the failure this pins is being stranded driving nothing: the target is
    /// despawned mid-possession, and if the reconcile only reacted to
    /// `possessed == None` the human would hold a seat on a dead entity forever.
    #[test]
    fn a_despawned_target_hands_the_seat_back_to_the_home_avatar() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let home = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot::PRIMARY))
            .id();
        let target = app.world_mut().spawn(Brain::stand_still()).id();
        {
            let mut state = app.world_mut().resource_mut::<PossessionState>();
            state.home = Some(home);
            state.possessed = Some(target);
        }
        app.add_systems(Update, project_driving_participant);
        app.update();
        assert_eq!(driver(&app, target), Some(PlayerSlot::PRIMARY));

        app.world_mut().despawn(target);
        app.update();

        assert_eq!(
            driver(&app, home),
            Some(PlayerSlot::PRIMARY),
            "the driven body is gone and the seat did not come home"
        );
    }
}
