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

use crate::{InputParticipant, Platformer2dInputActionMonolith};

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

    /// Build an order from a known device list. For a caller that already holds
    /// the devices (a session freezing its seating) and for tests; the tracking
    /// system is still the only thing that DISCOVERS them.
    pub fn from_devices(devices: Vec<Entity>) -> Self {
        Self(devices)
    }
}

/// The local seating a SESSION was started with — frozen, and shared by
/// everything that must agree about it.
///
/// [`LocalDeviceOrder`] is LIVE: a controller connecting mid-match changes it.
/// Several consumers need to agree about how many people are playing — the
/// match roster, the rollback session's player count, the handle→device
/// mapping, the per-seat input latches — and each of them sampling the live
/// resource independently means a connection landing between two samples makes
/// them disagree while both read "the same source". The roster would seat three
/// fighters into a two-handle session and nothing would say so.
///
/// So the topology is decided ONCE **per GAMEPLAY session** — not per GGRS
/// session — and every consumer reads the snapshot. Baseline starts, proof
/// pulses and hot-reload rebases are all the same gameplay session RESTARTED and
/// reuse the frozen value; recapturing for one of them would make the topology
/// stable only for a rollback sub-session, which is not the lifetime the roster
/// or the latches use. It is REMOVED when gameplay ends, because a topology that
/// outlives its session is the previous match's seating presented to the next one
/// as a frozen fact.
///
/// `generation` exists so a consumer can notice it was rebuilt rather than
/// compare vectors (GPT 5.6, 2026-07-28; lifetime corrected 2026-07-29).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalSeatTopology {
    generation: u64,
    seats: Vec<Entity>,
}

impl LocalSeatTopology {
    /// Freeze the current device order as this session's seating.
    ///
    /// Advances the generation on every capture, INCLUDING one that produces
    /// the same seats: "the topology was decided again" is the fact a consumer
    /// caches against, and two identical captures at different times are still
    /// two decisions (the same reasoning as `CharacterCatalogGeneration`).
    pub fn capture(&mut self, order: &LocalDeviceOrder) {
        self.generation = self.generation.wrapping_add(1);
        self.seats = order.devices().to_vec();
    }

    /// How many local players this session seats. At least one: a keyboard-only
    /// desktop has no device rows and still has a player, and a session with
    /// zero local handles accepts input from nobody.
    pub fn players(&self) -> usize {
        self.seats.len().max(1)
    }

    /// The controller a handle drives, if this seat has one. A handle past the
    /// connected devices is a CPU or an empty seat, not an error.
    pub fn device_for_handle(&self, handle: usize) -> Option<Entity> {
        self.seats.get(handle).copied()
    }

    /// Bumped on every capture; `0` means never captured.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether a session has decided its seating yet.
    pub fn is_frozen(&self) -> bool {
        self.generation > 0
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
/// ⚠ **The mapping comes from the frozen topology while a session owns one.**
/// `LocalDeviceOrder` is live, and a session that froze
/// `handle 0 → keyboard, 1 → pad A, 2 → pad B` and then let a disconnect reorder
/// the live list would keep its GGRS handle COUNT while quietly changing which
/// physical device drives each handle. Freezing the count and not the mapping is
/// freezing the easy half (GPT 5.6, 2026-07-29).
///
/// Live discovery still runs — it is what the NEXT session freezes — it just does
/// not get to redecide this one.
pub fn assign_local_seat_devices(
    order: Res<LocalDeviceOrder>,
    topology: Option<Res<LocalSeatTopology>>,
    mut seats: Query<(&InputParticipant, &mut InputMap<Platformer2dInputActionMonolith>)>,
) {
    let frozen = topology.filter(|topology| topology.is_frozen());
    // **HOW MANY PEOPLE ARE PLAYING comes from the session, not from how many
    // seat entities have materialized yet.** (GPT 5.6, 2026-07-29)
    //
    // This asked `seats.iter().len() < 2`, which is an observation of ACTIVATION
    // PROGRESS. During activation a two-player topology can already exist while
    // only the primary participant entity does — and in that window the solo
    // branch below cleared the primary's gamepad restriction and restored
    // any-pad behaviour, so a controller meant for handle 1 could drive seat 0
    // until the second entity appeared.
    //
    // A frozen topology is the session's own answer and does not move.
    let players = match frozen.as_ref() {
        Some(topology) => topology.players(),
        None => seats.iter().len(),
    };
    // Solo: leave leafwing's any-pad behaviour exactly as it was.
    if players < 2 {
        for (_, mut map) in &mut seats {
            if map.gamepad().is_some() {
                map.clear_gamepad();
            }
        }
        return;
    }

    for (participant, mut map) in &mut seats {
        let slot = participant.id.slot();
        let wanted = match frozen.as_ref() {
            Some(topology) => topology.device_for_handle(slot as usize),
            None => order.device_for_slot(slot),
        };
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
                InputMap::<Platformer2dInputActionMonolith>::default(),
            ))
            .id()
    }

    fn assigned(app: &App, seat: Entity) -> Option<Entity> {
        app.world()
            .entity(seat)
            .get::<InputMap<Platformer2dInputActionMonolith>>()
            .expect("the seat keeps its input map")
            .gamepad()
    }

    /// **A frozen session's device mapping does not follow live discovery.**
    ///
    /// The topology froze the COUNT and, for a day, nothing else: the GGRS handle
    /// count stayed put while `assign_local_seat_devices` kept reading the live
    /// order, so a mid-match disconnect could hand handle 1 a different physical
    /// controller than the one the session was built around — confirmed input
    /// from one pad replayed as another's (GPT 5.6, 2026-07-29).
    ///
    /// Freezing the count and not the mapping is freezing the half that is easy
    /// to test.
    #[test]
    fn a_frozen_session_keeps_its_device_mapping_when_a_pad_disconnects() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app.world_mut().spawn(Gamepad::default()).id();
        let pad_b = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(pad_b));

        // The session starts and freezes what it found.
        let frozen = {
            let mut topology = LocalSeatTopology::default();
            topology.capture(app.world().resource::<LocalDeviceOrder>());
            topology
        };
        app.insert_resource(frozen);

        // Pad A drops out mid-match. Live discovery correctly reports one pad.
        app.world_mut().entity_mut(pad_a).despawn();
        app.update();

        assert_eq!(
            assigned(&app, two),
            Some(pad_b),
            "seat two's controller was reassigned by a disconnect it was not \
             involved in: with live order, pad B slides into slot 0 and seat two \
             gets nothing"
        );
        assert_eq!(
            assigned(&app, one),
            Some(pad_a),
            "handle 0 must keep pointing at the controller the session was built \
             around, even though it is gone: that seat reads nothing, which is \
             the truth. Promoting pad B into it would silently hand seat one's \
             confirmed GGRS inputs to seat two's physical controller — the \
             mapping is frozen precisely so a disconnect cannot do that. (A \
             despawned entity is never recycled into an equal `Entity`; the \
             generation moves, so a new pad cannot inherit this binding.)"
        );
    }

    /// Without a frozen topology, live discovery is still the answer — that is
    /// what the next session freezes.
    #[test]
    fn an_unfrozen_session_still_follows_live_discovery() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app.world_mut().spawn(Gamepad::default()).id();
        let pad_b = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(pad_b));

        app.world_mut().entity_mut(pad_a).despawn();
        app.update();
        assert_eq!(
            assigned(&app, one),
            Some(pad_b),
            "with no session owning the seating, the live order is the authority"
        );
    }

    /// **Activation progress is not the session's player count.**
    /// (GPT 5.6, 2026-07-29)
    ///
    /// The solo branch asked how many seat ENTITIES existed. During activation a
    /// two-player topology can already be frozen while only the primary
    /// participant has materialized — and in that window the solo branch cleared
    /// the primary's gamepad restriction and restored any-pad behaviour, so a
    /// controller meant for handle 1 could drive seat 0 until the second entity
    /// appeared.
    #[test]
    fn a_frozen_two_player_session_binds_the_primary_before_seat_two_exists() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let pad_a = app.world_mut().spawn(Gamepad::default()).id();
        let pad_b = app.world_mut().spawn(Gamepad::default()).id();
        app.update();

        let frozen = {
            let mut topology = LocalSeatTopology::default();
            topology.capture(app.world().resource::<LocalDeviceOrder>());
            topology
        };
        assert_eq!(frozen.players(), 2, "the fixture must freeze two players");
        app.insert_resource(frozen);
        app.update();

        assert_eq!(
            assigned(&app, one),
            Some(pad_a),
            "seat two has not materialized yet, so the entity count says SOLO and \
             the primary was handed any-pad behaviour — pad B could drive it until \
             the second participant appeared"
        );
        let _ = pad_b;
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

#[cfg(test)]
mod local_seat_topology_tests {
    use super::*;
    use bevy::prelude::Entity;

    fn order(count: usize) -> LocalDeviceOrder {
        LocalDeviceOrder::from_devices(
            (0..count)
                .map(|i| Entity::from_raw_u32(i as u32 + 1).unwrap())
                .collect(),
        )
    }

    /// **A session's seating is decided once, and every consumer reads that.**
    ///
    /// The roster and the rollback session both need to know how many people
    /// are playing. Sampling the LIVE device order independently means a
    /// controller connecting between the two samples makes them disagree while
    /// both cite the same source — the roster seats a fighter the session has
    /// no handle for.
    #[test]
    fn a_frozen_topology_does_not_follow_a_later_connection() {
        let mut topology = LocalSeatTopology::default();
        assert!(!topology.is_frozen(), "nothing has decided the seating yet");

        topology.capture(&order(2));
        assert_eq!(topology.players(), 2);
        assert!(topology.is_frozen());

        // A third pad joins mid-match. The LIVE order changes; the session's
        // seating does not, because the session cannot grow a handle.
        let live = order(3);
        assert_eq!(live.devices().len(), 3);
        assert_eq!(
            topology.players(),
            2,
            "a controller connecting mid-session must not silently add a seat \
             the rollback session has no handle for"
        );
    }

    /// Zero devices is one player: a keyboard-only desktop has no device rows
    /// and still has somebody playing, and a session with zero local handles
    /// accepts input from nobody.
    #[test]
    fn a_keyboard_only_desktop_is_still_one_player() {
        let mut topology = LocalSeatTopology::default();
        topology.capture(&order(0));
        assert_eq!(topology.players(), 1);
        assert_eq!(topology.device_for_handle(0), None, "and it owns no pad");
    }

    /// Re-capturing ADVANCES the generation even when the seats are identical.
    /// "The topology was decided again" is the fact a consumer caches against,
    /// and two identical decisions at different times are still two decisions.
    #[test]
    fn recapturing_the_same_seats_is_still_a_new_generation() {
        let mut topology = LocalSeatTopology::default();
        topology.capture(&order(2));
        let first = topology.generation();
        topology.capture(&order(2));
        assert!(
            topology.generation() > first,
            "a rebase that happens to reproduce the same seating is still a \
             rebase, and a consumer comparing generations must see it"
        );
    }

    /// Each handle maps to the device that seat owns, in connection order.
    #[test]
    fn handles_map_to_devices_in_connection_order() {
        let live = order(2);
        let mut topology = LocalSeatTopology::default();
        topology.capture(&live);
        assert_eq!(topology.device_for_handle(0), Some(live.devices()[0]));
        assert_eq!(topology.device_for_handle(1), Some(live.devices()[1]));
        assert_eq!(
            topology.device_for_handle(2),
            None,
            "a handle past the connected pads is a CPU or an empty seat, not an error"
        );
    }
}
