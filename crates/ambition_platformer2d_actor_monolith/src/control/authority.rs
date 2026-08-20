//! **WHO DRIVES THIS BODY** — control authority as its own fact.
//!
//! ⭐⭐ **`Brain::Player(slot)` was carrying TWO meanings and only one of them is
//! a brain.** *"A participant drives this body"* is not an AI backend; it sits in
//! the same enum as `Wanderer` and `BossPattern` because that enum was the only
//! place to say it. Every exhaustive match over `Brain` therefore has an arm for
//! a thing that is not a policy, and — the expensive half — **possession has to
//! MOVE the variant** to change who is driving, which destroys the target's own
//! policy and forces `PossessionState` to stash it in `restore_brain`.
//!
//! ⇒ [`DrivingParticipant`] is that fact on its own. It is **DERIVED**, not
//! written at the possess site, for the reason `InCustodyOf` is: a component
//! reprojected every tick from state that IS in the snapshot needs no snapshot
//! entry of its own, and writing it at a decision site would create a population
//! nothing re-derives — a rewind past the possession would drop it with nothing
//! to put it back.
//!
//! ⭐ **the two inputs, and neither is privileged.** A body carrying
//! `Brain::Player(slot)` has that slot's authority; a live possession REDIRECTS
//! the primary slot's authority onto the driven body. Redirect rather than move:
//! the home avatar keeps its player brain and the target keeps its own policy, so
//! releasing needs nothing put back.
//!
//! ⛔⛔ **NOT `ControlAuthority`, and the near-miss is worth the paragraph.**
//! `character_runtime::prepared_match::ControlAuthority` already exists in this
//! crate — and is re-exported from the `ambition_platformer2d` SDK — for a
//! DIFFERENT fact: what a roster SEAT attaches, `LocalInput { channel, source }`
//! or `Brain { profile }`. That is a binding SPEC, read once when a match is
//! prepared. This is a body's live DRIVER, re-derived every tick. Two types with
//! one name in one crate would make every future reader of this seam ask which
//! one was meant, so the new one takes the new name.
//!
//! ⚠ **this slice does not delete `Brain::Player`.** 194 sites name it across 14
//! crates, and the review's instruction was explicit — *"evidence-driven carve;
//! do not redesign the brain stack at once."* What lands here is the SEAM: one
//! component that answers *who drives*, one arbiter that reads it, and a
//! possession that stops swapping policies around to say something it can now say
//! directly.

use bevy::prelude::*;

use ambition_characters::brain::{Brain, PlayerSlot};

use crate::abilities::traversal::possession::PossessionState;

// ⭐ **the TYPE lives in `ambition_characters::brain`, beside `Brain` and
// `PlayerSlot`.** This module owns the PROJECTION — which needs
// `PossessionState`, an actor-domain resource — but the fact itself is
// vocabulary two sibling crates ask for, and neither the interaction seam nor
// the conversation seam can see the other. Re-exported rather than re-declared:
// two meanings on one word in one crate is the collision this seam already
// walked into once.
pub use ambition_characters::brain::DrivingParticipant;

/// Re-derive, every tick, which participant drives which body.
///
/// ⭐ **possession is a REDIRECT, not a move.** The primary slot's authority goes
/// to `PossessionState::possessed` while a possession is live and to the body
/// wearing `Brain::Player(PRIMARY)` otherwise. Nothing is stashed, because
/// nothing was taken away.
///
/// ⚠ **compared before writing**, like every other derive on this road: an
/// unconditional insert marks the component changed every tick of a possession,
/// and change ticks do not rewind.
pub fn project_driving_participant(
    mut commands: Commands,
    state: Res<PossessionState>,
    brains: Query<(Entity, &Brain)>,
    existing: Query<()>,
    held: Query<(Entity, &DrivingParticipant)>,
) {
    use std::collections::BTreeMap;

    // ⚠ a `BTreeMap` rather than the query's order: this decides component
    // writes a control arbiter reads, and Bevy's iteration order is an archetype
    // accident.
    let mut wanted: BTreeMap<Entity, PlayerSlot> = BTreeMap::new();
    for (entity, brain) in &brains {
        if let Some(slot) = brain.player_slot() {
            wanted.insert(entity, slot);
        }
    }
    // The redirect. Only the PRIMARY slot possesses — see `possession_trigger_system`.
    if let Some(possessed) = state.possessed {
        if existing.get(possessed).is_ok() {
            wanted.retain(|_, slot| *slot != PlayerSlot::PRIMARY);
            wanted.insert(possessed, PlayerSlot::PRIMARY);
        }
    }

    for (entity, authority) in &held {
        if wanted.get(&entity) != Some(&authority.0) {
            commands.entity(entity).remove::<DrivingParticipant>();
        }
    }
    for (entity, slot) in wanted {
        if held.get(entity).map(|(_, a)| a.0) != Ok(slot) {
            commands.entity(entity).try_insert(DrivingParticipant(slot));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::traversal::possession::PossessionState;

    /// Run the projection once over a world and read back who drives what.
    fn project(app: &mut App) {
        app.add_systems(Update, project_driving_participant);
        app.update();
    }

    fn driver(app: &App, body: Entity) -> Option<PlayerSlot> {
        app.world().get::<DrivingParticipant>(body).map(|d| d.0)
    }

    /// **With nobody possessing anything, authority follows the player brains.**
    #[test]
    fn a_player_brain_carries_that_slots_authority_and_an_ai_brain_carries_none() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let home = app
            .world_mut()
            .spawn(Brain::Player(PlayerSlot::PRIMARY))
            .id();
        let cpu = app.world_mut().spawn(Brain::stand_still()).id();
        project(&mut app);

        assert_eq!(driver(&app, home), Some(PlayerSlot::PRIMARY));
        assert_eq!(
            driver(&app, cpu),
            None,
            "an autonomous body was given a participant's authority — `acting_slot` \
             turns that into the primary seat, so a CPU actor would spend a human's \
             buffered press"
        );
    }

    /// **THE POINT OF THE TYPE: possession REDIRECTS authority without moving a
    /// policy.**
    ///
    /// ⭐ the target keeps its own `Brain::StateMachine` throughout. Today
    /// possession also moves `Brain::Player` and this derive would agree either
    /// way; the fixture deliberately does NOT move it, so what is pinned is the
    /// rule rather than the current spelling of it. When `restore_brain` goes,
    /// this test is what says the behaviour did not.
    #[test]
    fn a_live_possession_moves_the_primary_seats_authority_and_leaves_the_policy() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let home = app
            .world_mut()
            .spawn(Brain::Player(PlayerSlot::PRIMARY))
            .id();
        let target = app.world_mut().spawn(Brain::stand_still()).id();
        app.world_mut().resource_mut::<PossessionState>().possessed = Some(target);
        project(&mut app);

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
            "the projection reached into the target's POLICY — it may only decide \
             who drives, never what the body knows how to do on its own"
        );
    }

    /// **A second seat is untouched by the primary's possession.**
    ///
    /// ⚠ the redirect clears `PRIMARY` from whoever else held it, and a version
    /// that cleared the whole map would silently unseat every co-op partner the
    /// moment player one possessed something.
    #[test]
    fn possession_by_the_primary_seat_does_not_unseat_a_second_participant() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let one = app
            .world_mut()
            .spawn(Brain::Player(PlayerSlot::PRIMARY))
            .id();
        let two = app.world_mut().spawn(Brain::Player(PlayerSlot(1))).id();
        let target = app.world_mut().spawn(Brain::stand_still()).id();
        app.world_mut().resource_mut::<PossessionState>().possessed = Some(target);
        project(&mut app);

        assert_eq!(driver(&app, target), Some(PlayerSlot::PRIMARY));
        assert_eq!(driver(&app, one), None);
        assert_eq!(
            driver(&app, two),
            Some(PlayerSlot(1)),
            "player two lost their body because player one possessed something"
        );
    }

    /// **Authority is RETRACTED, not left behind.**
    ///
    /// ⛔ the release direction, which every latch on this road has got wrong at
    /// least once: a projection that only ever inserts leaves the possessed body
    /// driving forever, and the home avatar never gets its seat back.
    #[test]
    fn releasing_the_possession_returns_the_seat_to_the_body_that_owns_it() {
        let mut app = App::new();
        app.init_resource::<PossessionState>();
        let home = app
            .world_mut()
            .spawn(Brain::Player(PlayerSlot::PRIMARY))
            .id();
        let target = app.world_mut().spawn(Brain::stand_still()).id();
        app.world_mut().resource_mut::<PossessionState>().possessed = Some(target);
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
    }
}
