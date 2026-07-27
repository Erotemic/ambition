//! **Which physical device drives which local seat.** (C4 slice 5)
//!
//! Every participant's [`InputMap`] shipped with no associated gamepad, which
//! in leafwing means "whichever pad this finds first":
//!
//! ```ignore
//! let gamepad = self.associated_gamepad.unwrap_or(find_gamepad(gamepads));
//! // find_gamepad: gamepads.iter().next().unwrap_or(Entity::PLACEHOLDER)
//! ```
//!
//! `Query::iter().next()` is not a promise about which controller a person is
//! holding. With one pad connected it is harmless and has been correct for the
//! whole life of the project. With TWO it is the couch-play bug in one line:
//! both seats resolve to the same arbitrary pad, so player two's controller
//! either does nothing or moves player one, and which of those happens depends
//! on archetype order.
//!
//! ## The rule
//!
//! One seat: no association at all. That is deliberately today's behaviour,
//! byte for byte — a single player with a spare controller plugged in must not
//! discover that only one of their two pads works because a rule they never
//! asked for partitioned them. Solo play does not need device ownership and
//! should not pay for it.
//!
//! Two or more seats: partition. Seat `n` owns the `n`-th connected pad, in a
//! deterministic order, and a seat with no pad left over gets its association
//! CLEARED rather than left stale — an association pointing at an unplugged
//! controller is a seat that has silently stopped responding.
//!
//! ## Connection order is REMEMBERED, not derived
//!
//! The obvious key is the gamepad entity's index — Bevy spawns the entity on
//! connection, so it looks like it ascends with connection order. It does not:
//! entity indices are RECYCLED, so a controller plugged in second can be handed
//! the index a despawned entity gave back, and sort to the front. This is not
//! hypothetical; it is what the couch-versus test caught within a minute of the
//! first draft, and the symptom is the worst kind — the two players' controllers
//! swap, sometimes, depending on what else the app spawned and dropped.
//!
//! So arrival order is recorded when it happens, in [`LocalDeviceOrder`], and
//! survives whatever the allocator does with indices afterwards. Player one
//! keeps the controller in their hands when player two joins, which is the whole
//! reason the order matters.
//!
//! (Within a single frame, several pads can appear at once — a fresh launch sees
//! every already-connected controller on the same tick. Those are ordered by
//! index, which is arbitrary but DETERMINISTIC, and no human distinction exists
//! to respect anyway.)

use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;

use crate::{InputParticipant, SandboxAction};

/// Connected controllers, oldest connection first.
///
/// A resource rather than a derived sort because the fact it holds — the order
/// people picked their controllers up in — is not recoverable from the world
/// once it has happened.
#[derive(Resource, Debug, Default)]
pub struct LocalDeviceOrder(Vec<Entity>);

impl LocalDeviceOrder {
    /// The controller a seat in this slot owns, if one is connected.
    pub fn device_for_slot(&self, slot: u8) -> Option<Entity> {
        self.0.get(slot as usize).copied()
    }

    pub fn devices(&self) -> &[Entity] {
        &self.0
    }
}

/// Record connections in the order they happen, and forget disconnections.
pub fn track_local_device_order(
    pads: Query<Entity, With<Gamepad>>,
    mut order: ResMut<LocalDeviceOrder>,
) {
    let live: Vec<Entity> = pads.iter().collect();
    let mut next: Vec<Entity> = order
        .0
        .iter()
        .copied()
        .filter(|pad| live.contains(pad))
        .collect();
    let mut fresh: Vec<Entity> = live
        .iter()
        .copied()
        .filter(|pad| !next.contains(pad))
        .collect();
    fresh.sort_by_key(|pad| pad.index());
    next.extend(fresh);
    // Write only on a real change: this runs every frame, and an unconditional
    // `ResMut` deref would mark the order changed forever.
    if next != order.0 {
        order.0 = next;
    }
}

/// Give each local seat its own controller.
///
/// Runs in `PreUpdate` before leafwing resolves actions, so an association made
/// this frame is honoured by this frame's `ActionState` — a seat that joins is
/// playable on the tick it joins, not the one after.
pub fn assign_local_seat_devices(
    order: Res<LocalDeviceOrder>,
    mut seats: Query<(&InputParticipant, &mut InputMap<SandboxAction>)>,
) {
    // Solo: leave leafwing's any-pad behaviour exactly as it was.
    if seats.iter().len() < 2 {
        for (_, mut map) in &mut seats {
            if map.gamepad().is_some() {
                map.clear_gamepad();
            }
        }
        return;
    }

    for (participant, mut map) in &mut seats {
        let wanted = order.device_for_slot(participant.id.slot());
        // Change detection is not cosmetic here: `InputMap` is a component, and
        // touching it every frame marks it changed for every observer of the
        // input map — including the settings UI, which rebuilds bindings when
        // the map changes.
        if map.gamepad() == wanted {
            continue;
        }
        match wanted {
            Some(pad) => {
                map.set_gamepad(pad);
            }
            None => {
                map.clear_gamepad();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParticipantId;

    fn seat_app() -> App {
        let mut app = App::new();
        app.init_resource::<LocalDeviceOrder>();
        app.add_systems(
            Update,
            (track_local_device_order, assign_local_seat_devices).chain(),
        );
        app
    }

    fn spawn_seat(app: &mut App, id: ParticipantId) -> Entity {
        app.world_mut()
            .spawn((
                InputParticipant::with_id(id),
                InputMap::<SandboxAction>::default(),
            ))
            .id()
    }

    fn assigned(app: &App, seat: Entity) -> Option<Entity> {
        app.world()
            .entity(seat)
            .get::<InputMap<SandboxAction>>()
            .expect("the seat keeps its input map")
            .gamepad()
    }

    #[test]
    fn a_lone_seat_keeps_any_pad() {
        let mut app = seat_app();
        let seat = spawn_seat(&mut app, ParticipantId::PRIMARY);
        app.world_mut().spawn(Gamepad::default());
        app.world_mut().spawn(Gamepad::default());
        app.update();
        assert_eq!(
            assigned(&app, seat),
            None,
            "a solo player with a spare controller plugged in must keep using \
             either one; partitioning devices they never asked to partition \
             would silently kill the pad that happened to sort second"
        );
    }

    #[test]
    fn two_seats_own_two_pads_in_connection_order() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let first_pad = app.world_mut().spawn(Gamepad::default()).id();
        let second_pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();

        assert_eq!(assigned(&app, one), Some(first_pad));
        assert_eq!(assigned(&app, two), Some(second_pad));
        assert_ne!(
            assigned(&app, one),
            assigned(&app, two),
            "two seats sharing one pad is the whole defect: leafwing's \
             unassociated fallback is `gamepads.iter().next()`, so both seats \
             resolve to the same controller"
        );
    }

    #[test]
    fn unplugging_a_pad_clears_the_seat_that_owned_it() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let first_pad = app.world_mut().spawn(Gamepad::default()).id();
        let second_pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, two), Some(second_pad));

        app.world_mut().entity_mut(second_pad).despawn();
        app.update();
        assert_eq!(
            assigned(&app, two),
            None,
            "a seat still associated with an unplugged controller reads a \
             device that does not exist, so it stops responding without ever \
             saying so"
        );
        assert_eq!(
            assigned(&app, one),
            Some(first_pad),
            "player one's controller must not be reshuffled because player \
             two unplugged theirs"
        );
    }

    /// The defect that killed the first draft: entity indices are recycled, so
    /// "sort the pads by index" hands player one whichever controller happens to
    /// have inherited a low index — which can be the one that connected LAST.
    #[test]
    fn a_recycled_entity_index_does_not_reorder_the_controllers() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);

        // Burn an index, then free it, so the pad that connects SECOND can be
        // allocated a lower index than the pad that connected first.
        let scratch = app.world_mut().spawn_empty().id();
        let scratch_two = app.world_mut().spawn_empty().id();
        let first_pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        app.world_mut().entity_mut(scratch).despawn();
        app.world_mut().entity_mut(scratch_two).despawn();
        let second_pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();

        assert!(
            second_pad.index() < first_pad.index(),
            "this test is only meaningful when the second controller really did \
             get a recycled, lower index (got {} then {})",
            first_pad.index(),
            second_pad.index()
        );
        assert_eq!(
            assigned(&app, one),
            Some(first_pad),
            "player one must keep the controller they were already holding when \
             player two joined"
        );
        assert_eq!(assigned(&app, two), Some(second_pad));
    }
}
