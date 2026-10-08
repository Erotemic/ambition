//! Which controller each participant seat hears.
//!
//! [`LocalDeviceOrder`] is the one answer to "which controller is pad `n`".
//! A seat listens to a source ([`LocalInputSource`]): the frozen channel plan
//! of a session names it, and without a plan the seat number and the keyboard
//! owner do ([`crate::sources::source_for_seat`]). [`assign_local_seat_devices`]
//! projects that source onto the seat's input map and decides nothing else.
//!
//! No seat is left without an association while pads are connected. leafwing
//! gives an input map with no gamepad the FIRST connected pad, so such a seat
//! follows another seat's controller.

use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;

use crate::channels::{LocalChannelPlan, LocalInputSource};
use crate::participant::ParticipantId;
pub use crate::seating::LocalDeviceOrder;
use crate::seating::PadIdentity;
use crate::{InputParticipant, Platformer2dInputActionMonolith};

/// The association of a seat that hears no pad: a keyboard seat, a seat whose
/// controller is unplugged, and a seat the plan gives no source.
/// `Entity::PLACEHOLDER` is leafwing's own answer when no gamepad exists, so no
/// connected pad matches it.
pub const NO_PAD: Entity = Entity::PLACEHOLDER;

/// Frozen local-device topology for one gameplay session.
///
/// [`LocalDeviceOrder`] is live: a controller connecting mid-match changes
/// it. Roster sizing, rollback handles, and input latches read this snapshot
/// instead, so a connection change cannot make them disagree mid-session.
/// `generation` changes on each recapture so consumers can invalidate cached
/// assignments.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalSeatTopology {
    generation: u64,
    /// The pad table as it was when the session decided its seating.
    pads: LocalDeviceOrder,
    /// Roster-declared mapping from input sources to local channels.
    /// `None` means no roster has declared a plan and device discovery supplies
    /// the fallback topology.
    declared: Option<LocalChannelPlan>,
}

impl LocalSeatTopology {
    /// Freeze the current pad table as this session's seating.
    ///
    /// The generation advances on every capture, also when the seats are the
    /// same. Consumers cache against "the topology was decided again" (as with
    /// `CharacterCatalogGeneration`).
    pub fn capture(&mut self, order: &LocalDeviceOrder) {
        self.generation = self.generation.wrapping_add(1);
        self.pads = order.clone();
        // A recapture is a new decision. A declaration from an old roster must
        // not size this session.
        self.declared = None;
    }

    /// How many local players this session seats. At least one: a
    /// keyboard-only desktop has no connected pad but has a player, and a
    /// session with zero local handles accepts no input.
    ///
    /// The roster's declaration wins when present (see `declared`). The number
    /// of connected pads is the fallback.
    pub fn players(&self) -> usize {
        match &self.declared {
            Some(plan) => plan.channels().max(1),
            None => self.pads.connected_slots().count().max(1),
        }
    }

    /// Freeze the pad table AND the channel plan the roster declared.
    ///
    /// This is a separate entry point because some callers of
    /// [`Self::capture`] have no roster (the rollback observatory, device
    /// probes). A `None` argument would make "nobody declared" look like a
    /// decision.
    pub fn capture_for_roster(&mut self, order: &LocalDeviceOrder, declared: LocalChannelPlan) {
        self.capture(order);
        self.declared = Some(declared);
    }

    /// The channel plan the roster declared, if it spoke.
    pub fn declared_channels(&self) -> Option<&LocalChannelPlan> {
        self.declared.as_ref()
    }

    /// How many channels the roster declared, if it spoke.
    pub fn declared_seats(&self) -> Option<usize> {
        self.declared.as_ref().map(|plan| plan.channels())
    }

    /// The pad a channel drives when no plan was declared: the pads that were
    /// connected at the capture, in slot order, one for each channel.
    fn undeclared_pad(&self, index: usize) -> Option<LocalInputSource> {
        self.pads
            .connected_slots()
            .nth(index)
            .map(|slot| LocalInputSource::Pad(slot as u8))
    }

    /// The controller a channel drove when the session froze, if any.
    ///
    /// `None` is a channel with no pad: one on the keyboard, or one whose
    /// controller was unplugged. Neither is an error.
    pub fn device_for_channel(&self, channel: ParticipantId) -> Option<Entity> {
        let source = match &self.declared {
            Some(plan) => plan.source_for(channel)?,
            None => self.undeclared_pad(channel.slot() as usize)?,
        };
        self.pads.pad(source.pad_index()?)
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

/// Keep the pad table in step with the connected controllers.
pub fn track_local_device_order(
    pads: Query<(Entity, &Gamepad, Option<&Name>)>,
    mut order: ResMut<LocalDeviceOrder>,
) {
    let mut live: Vec<(Entity, PadIdentity)> = pads
        .iter()
        .map(|(entity, pad, name)| {
            (
                entity,
                PadIdentity::new(
                    name.map(|name| name.as_str().to_string()),
                    pad.vendor_id(),
                    pad.product_id(),
                ),
            )
        })
        .collect();
    // Query order is not stable. Two controllers that connect in one frame
    // take their slots in the order of their entity indices.
    live.sort_by_key(|(pad, _)| pad.index());
    // Write only on a real change. This runs every frame, and an
    // unconditional `ResMut` deref would mark the table changed every frame.
    let mut next = order.clone();
    if next.track(&live) {
        *order = next;
    }
}

/// The pad a lone seat hears: the controller its player used last.
///
/// One player can pick up any connected controller. The seat follows the pad
/// that shows input, and keeps it while it stays connected.
fn pad_in_use(current: Option<Entity>, order: &LocalDeviceOrder, pads: &Query<&Gamepad>) -> Option<Entity> {
    const AXIS_DEFLECTION: f32 = 0.5;
    let connected = order.connected();
    let used = connected.iter().copied().find(|pad| {
        pads.get(*pad).is_ok_and(|pad| {
            pad.get_just_pressed().next().is_some()
                || pad.get_analog_axes().any(|axis| {
                    pad.get(*axis)
                        .is_some_and(|value| value.abs() >= AXIS_DEFLECTION)
                })
        })
    });
    used.or(current.filter(|pad| connected.contains(pad)))
        .or(connected.first().copied())
}

/// Give each local seat the controller of its source.
///
/// Runs in `PreUpdate` before leafwing resolves actions, so a seat that joins
/// is playable on the tick it joins.
///
/// While a session owns a frozen topology, a seat hears only the controller
/// the session froze for it. A controller that disconnects leaves its seat with
/// no input, the same controller gets its seat back, and a different controller
/// does not. Without a frozen topology (a lobby, a menu) the live pad table
/// decides, so controllers connect and disconnect freely.
pub fn assign_local_seat_devices(
    order: Res<LocalDeviceOrder>,
    topology: Option<Res<LocalSeatTopology>>,
    offer: Option<Res<crate::seating::LocalSeatOffer>>,
    keyboard: Option<Res<crate::sources::KeyboardOwner>>,
    pads: Query<&Gamepad>,
    mut seats: Query<(
        &InputParticipant,
        &mut InputMap<Platformer2dInputActionMonolith>,
    )>,
) {
    let frozen = topology.filter(|topology| topology.is_frozen());
    // The larger of the two counts. During activation a two-player topology can
    // exist while only the primary entity does, and a lobby can hold several
    // seats while a one-player topology still stands. Neither is one player.
    let players = frozen
        .as_ref()
        .map_or(0, |topology| topology.players())
        .max(seats.iter().len());

    let plan = frozen.as_ref().and_then(|t| t.declared_channels());
    // A declared plan outranks the policy. It says who is on the keyboard,
    // including nobody.
    let keyboard_owner = match plan {
        Some(plan) => plan.keyboard_channel(),
        None => crate::sources::keyboard_owner_for(
            offer.map(|offer| offer.policy()).unwrap_or_default(),
            keyboard.map(|keyboard| *keyboard).unwrap_or_default(),
            players,
        ),
    };

    for (participant, mut map) in &mut seats {
        let slot = participant.id.slot();
        let source = match plan {
            Some(plan) => plan.source_for(participant.id),
            None => Some(crate::sources::source_for_seat(keyboard_owner, slot)),
        };
        let wanted = match (source, frozen.as_ref()) {
            // One player with no plan hears the pad that player uses.
            (Some(LocalInputSource::Pad(_)), _) if players < 2 && plan.is_none() => {
                pad_in_use(map.gamepad(), &order, &pads)
            }
            (Some(LocalInputSource::Pad(index)), Some(topology)) => {
                let index = match plan {
                    Some(_) => Some(index as usize),
                    // No plan: the pads connected at the freeze, one each.
                    None => topology
                        .undeclared_pad(index as usize)
                        .and_then(LocalInputSource::pad_index),
                };
                index.and_then(|index| topology.pads.still_held(&order, index))
            }
            (Some(LocalInputSource::Pad(index)), None) => order.pad(index as usize),
            (Some(LocalInputSource::Keyboard), _) | (None, _) => None,
        }
        .unwrap_or(NO_PAD);
        // Skip unchanged maps. `InputMap` is a component, and writing it every
        // frame marks it changed for every observer, including the settings
        // UI, which rebuilds bindings on change.
        if map.gamepad() != Some(wanted) {
            map.set_gamepad(wanted);
        }
    }
}

/// `cargo test -p ambition_input` does not run these tests. The module is
/// `#[cfg(feature = "input")]`, so enable the feature:
///
/// ```bash
/// cargo test -p ambition_input --features input     # 84, not 55
/// ```
///
/// `cargo test --workspace` enables the feature through
/// `ambition_platformer2d_actor_monolith`
/// (`default -> desktop_dev -> visible -> input`).
#[cfg(test)]
mod tests {
    use super::*;
    use ParticipantId;

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

    /// Freeze the session's seating from the pad table the app tracked, as a
    /// session does. `plan` is the roster's declaration, if it made one.
    fn freeze(app: &mut App, plan: Option<LocalChannelPlan>) {
        app.update();
        let mut topology = LocalSeatTopology::default();
        let order = app.world().resource::<LocalDeviceOrder>().clone();
        match plan {
            Some(plan) => topology.capture_for_roster(&order, plan),
            None => topology.capture(&order),
        }
        app.insert_resource(topology);
    }

    #[test]
    fn a_single_pad_beside_a_keyboard_player_drives_the_second_seat() {
        let mut app = seat_app();
        app.insert_resource(crate::seating::LocalSeatOffer::offered(
            "a couch surface",
            2,
            crate::sources::InputAssignmentPolicy::JoinToClaim,
        ));
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, two), Some(pad), "the pad player is seat two");
        assert_eq!(
            assigned(&app, one),
            Some(Entity::PLACEHOLDER),
            "seat one plays on the keyboard and must not answer any pad"
        );
    }

    /// A keyboard player beside one pad player, frozen, through a reconnect.
    ///
    /// The frozen path shifts the handle index by one for seats below the
    /// keyboard owner, because the keyboard is not a device row. Two declared
    /// seats, one device. A wrong shift fails silently: the pad player gets
    /// no pad.
    #[test]
    fn a_frozen_keyboard_and_pad_pair_survives_the_pad_reconnecting() {
        let mut app = seat_app();
        app.insert_resource(crate::seating::LocalSeatOffer::offered(
            "a couch surface",
            2,
            crate::sources::InputAssignmentPolicy::JoinToClaim,
        ));
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("the only pad")))
            .id();

        // Two declared seats, one device.
        freeze(
            &mut app,
            Some(LocalChannelPlan::from_sources([
                LocalInputSource::Keyboard,
                LocalInputSource::Pad(0),
            ])),
        );
        app.update();
        assert_eq!(
            assigned(&app, two),
            Some(pad),
            "the pad player is seat two, and the frozen handle for seat two is \
             handle ZERO — the keyboard owner above them is not a device row"
        );
        assert_eq!(
            assigned(&app, one),
            Some(Entity::PLACEHOLDER),
            "seat one plays on the keyboard and must stay deaf to every pad"
        );

        // The pad player's controller dies and comes back.
        app.world_mut().entity_mut(pad).despawn();
        app.update();
        assert_eq!(
            assigned(&app, one),
            Some(Entity::PLACEHOLDER),
            "the keyboard seat must not be handed anything by a disconnect"
        );
        let pad_again = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("the only pad")))
            .id();
        app.update();
        assert_eq!(
            assigned(&app, two),
            Some(pad_again),
            "the reconnected pad must come back to the seat that was holding it, \
             not stay pointed at the dead entity"
        );
        assert_eq!(
            assigned(&app, one),
            Some(Entity::PLACEHOLDER),
            "and it must never land on the keyboard seat"
        );
    }

    /// The Smash couch: `JoinToClaim`, with both players on pads.
    /// `keyboard_owner_for` gives the keyboard to `PRIMARY` for every
    /// `JoinToClaim` session, which would deafen player one's pad.
    ///
    /// A declared plan outranks the policy. It says who is on the keyboard,
    /// including nobody.
    #[test]
    fn a_declared_couch_with_nobody_on_the_keyboard_gives_both_seats_their_pads() {
        let mut app = seat_app();
        app.insert_resource(crate::seating::LocalSeatOffer::offered(
            "a couch surface",
            2,
            crate::sources::InputAssignmentPolicy::JoinToClaim,
        ));
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad a")))
            .id();
        let pad_b = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad b")))
            .id();

        let frozen = {
            let mut topology = LocalSeatTopology::default();
            topology.capture_for_roster(
                &LocalDeviceOrder::from_devices(vec![pad_a, pad_b]),
                LocalChannelPlan::from_sources([0, 1].map(LocalInputSource::Pad)),
            );
            topology
        };
        app.insert_resource(frozen);
        app.update();

        assert_eq!(
            assigned(&app, one),
            Some(pad_a),
            "player one is holding a controller and the plan says so — binding \
             them to the keyboard makes the person who started the match the one \
             who cannot move"
        );
        assert_eq!(assigned(&app, two), Some(pad_b));
    }

    /// A channel listens to the source it was given, not to its own number.
    ///
    /// Two people hold the second and third controllers; the first is unused.
    /// By position, channel 0 would take the unused pad.
    #[test]
    fn a_declared_plan_hands_each_channel_the_pad_its_person_is_holding() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let spare = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("the spare on the desk")))
            .id();
        let pad_b = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad b")))
            .id();
        let pad_c = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad c")))
            .id();

        let frozen = {
            let mut topology = LocalSeatTopology::default();
            topology.capture_for_roster(
                &LocalDeviceOrder::from_devices(vec![spare, pad_b, pad_c]),
                LocalChannelPlan::from_sources([1, 2].map(LocalInputSource::Pad)),
            );
            topology
        };
        app.insert_resource(frozen);
        app.update();

        assert_eq!(
            assigned(&app, one),
            Some(pad_b),
            "channel zero was given pad ONE; handing it pad zero is handing it a \
             controller nobody is holding"
        );
        assert_eq!(assigned(&app, two), Some(pad_c));
    }

    /// The default policy leaves solo behaviour exactly where it was.
    ///
    /// No policy resource: seat one keeps the pad. A solo player with a
    /// controller and a keyboard must not get couch partitioning.
    #[test]
    fn without_a_declared_policy_the_pad_still_goes_to_seat_one() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, one), Some(pad));
        assert_eq!(assigned(&app, two), Some(NO_PAD));
    }

    /// Two seats, two pads, unplug player one's: ownership must not transfer.
    #[test]
    fn unplugging_one_pad_does_not_hand_its_seat_the_other_players_pad() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app.world_mut().spawn(Gamepad::default()).id();
        let pad_b = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(pad_b));

        app.world_mut().despawn(pad_a);
        app.update();

        assert_ne!(
            assigned(&app, one),
            Some(pad_b),
            "player one's pad was unplugged and their seat took player TWO's \
             controller — a disconnect must not transfer ownership"
        );
        assert_eq!(
            assigned(&app, two),
            Some(pad_b),
            "player two kept playing on the pad in their hands"
        );
    }

    /// A reconnecting pad is a new entity. The seat that lost its pad is the
    /// only seat with none, so the free pad goes there.
    #[test]
    fn a_reconnecting_pad_comes_back_to_the_seat_that_lost_one() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app.world_mut().spawn(Gamepad::default()).id();
        let pad_b = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(pad_b));

        // Player one's controller drops out.
        app.world_mut().entity_mut(pad_a).despawn();
        app.update();
        assert_eq!(assigned(&app, one), Some(NO_PAD));
        assert_eq!(assigned(&app, two), Some(pad_b));

        // ...and comes back. A DIFFERENT entity, as a real reconnection is.
        let pad_again = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(
            assigned(&app, one),
            Some(pad_again),
            "the seat that lost a pad is the seat that gets the returning one"
        );
        assert_eq!(
            assigned(&app, two),
            Some(pad_b),
            "and player two was never disturbed by any of it"
        );
    }

    /// Two pads reconnecting in the OTHER order must not swap the players.
    ///
    /// With both seats empty, a pad must return to its own seat, not the first
    /// empty one. Otherwise, reversed reconnect order swaps the players.
    #[test]
    fn two_pads_reconnecting_in_reverse_order_keep_their_own_seats() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad-a")))
            .id();
        let pad_b = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad-b")))
            .id();
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(pad_b));

        // Everybody unplugs.
        app.world_mut().entity_mut(pad_a).despawn();
        app.world_mut().entity_mut(pad_b).despawn();
        app.update();
        assert_eq!(assigned(&app, one), Some(NO_PAD));
        assert_eq!(assigned(&app, two), Some(NO_PAD));

        // Player TWO plugs back in first.
        let pad_b_again = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad-b")))
            .id();
        app.update();
        assert_eq!(
            assigned(&app, two),
            Some(pad_b_again),
            "player two's controller came back and landed in player ONE's seat"
        );
        assert_eq!(
            assigned(&app, one),
            Some(NO_PAD),
            "seat one is still waiting for its pad"
        );

        // ...and then player one.
        let pad_a_again = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad-a")))
            .id();
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a_again));
        assert_eq!(assigned(&app, two), Some(pad_b_again));
    }

    /// A frozen session must still survive a reconnection.
    ///
    /// Freezing records entities, and an entity dies on unplug. The frozen
    /// branch must still repair the seat when the same pad returns. Giving a
    /// seat its own controller back is not a reorder.
    #[test]
    fn a_frozen_session_rebinds_a_seat_whose_pad_came_back() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad-a")))
            .id();
        let pad_b = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad-b")))
            .id();
        app.update();
        let frozen = {
            let mut topology = LocalSeatTopology::default();
            topology.capture(app.world().resource::<LocalDeviceOrder>());
            topology
        };
        app.insert_resource(frozen);
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(pad_b));

        // Player one's controller drops and comes back.
        app.world_mut().entity_mut(pad_a).despawn();
        app.update();
        // Not `None`: leafwing gives such a seat the first connected pad,
        // which is pad B.
        assert_ne!(
            assigned(&app, one),
            Some(pad_b),
            "seat one was handed player two's controller"
        );
        assert_ne!(
            assigned(&app, one),
            None,
            "an unset gamepad answers the first connected pad"
        );
        let pad_a_again = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad-a")))
            .id();
        app.update();
        assert_eq!(
            assigned(&app, one),
            Some(pad_a_again),
            "a frozen session left seat one pointing at a pad that no longer exists"
        );
        assert_eq!(
            assigned(&app, two),
            Some(pad_b),
            "and player two was never disturbed"
        );
    }

    /// A frozen session's device mapping does not follow live discovery.
    ///
    /// The session must freeze the mapping, not only the player count.
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
            Some(NO_PAD),
            "handle 0 reads nothing while its controller is gone. Promoting pad \
             B into it would hand seat one's confirmed GGRS inputs to seat \
             two's physical controller. The mapping is frozen so that a \
             disconnect cannot do that."
        );
    }

    /// Without a frozen topology, discovery may fill an empty seat but must not
    /// redistribute an existing controller assignment after a disconnect.
    #[test]
    fn an_unfrozen_seat_keeps_its_pad_and_a_free_one_still_finds_an_empty_seat() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        // Discovery still works: the first seat takes the only pad.
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(NO_PAD));

        // A second pad arrives and finds the seat that has none.
        let pad_b = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(
            assigned(&app, one),
            Some(pad_a),
            "seat one did not change hands"
        );
        assert_eq!(assigned(&app, two), Some(pad_b));

        // Player one unplugs. Their seat empties; player two keeps playing.
        app.world_mut().entity_mut(pad_a).despawn();
        app.update();
        assert_eq!(
            assigned(&app, one),
            Some(NO_PAD),
            "player one's seat reads nothing, which is the truth"
        );
        assert_eq!(
            assigned(&app, two),
            Some(pad_b),
            "player two was holding this controller and a disconnect elsewhere \
             must not take it away"
        );
    }

    /// During activation a two-player topology can be frozen while only the
    /// primary participant exists. Counting entities would take the branch
    /// for one player, and the primary would follow pad B when it shows input.
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
        push_stick(&mut app, pad_b, 1.0);
        app.update();

        assert_eq!(
            assigned(&app, one),
            Some(pad_a),
            "seat two has not materialized yet, so the entity count says one \
             player, and the primary followed pad B until the second \
             participant appeared"
        );
    }

    /// A frozen session's seats must remember which controller they hold, or
    /// a reconnect cannot be repaired. End-to-end coverage:
    /// `game/ambition_app/tests/rollback_seat_devices.rs`.
    #[test]
    fn a_frozen_seat_remembers_its_pad_so_a_reconnect_can_come_home() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad a")))
            .id();
        let pad_b = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad b")))
            .id();

        freeze(
            &mut app,
            Some(LocalChannelPlan::from_sources([0, 1].map(LocalInputSource::Pad))),
        );
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));
        assert_eq!(assigned(&app, two), Some(pad_b));

        // Seat two's controller dies.
        app.world_mut().entity_mut(pad_b).despawn();
        app.update();
        assert_eq!(
            assigned(&app, one),
            Some(pad_a),
            "seat one lost nothing and must not move"
        );

        // The same controller comes back as a new entity.
        let pad_b_again = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad b")))
            .id();
        app.update();
        assert_eq!(
            assigned(&app, two),
            Some(pad_b_again),
            "the reconnected pad did not come home: a frozen seat that never \
             recorded WHICH controller it held has no identity to match, so it \
             keeps pointing at a dead entity forever"
        );
        assert_eq!(
            assigned(&app, one),
            Some(pad_a),
            "the reconnect moved the seat that never lost its pad"
        );
    }

    /// One player with two controllers connected can use either one: the
    /// seat follows the pad that shows input. Measured before: the seat had
    /// no association, leafwing gave it the first pad, and the second pad did
    /// nothing.
    #[test]
    fn a_lone_seat_follows_the_pad_its_player_uses() {
        let mut app = seat_app();
        let seat = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let first = app.world_mut().spawn(Gamepad::default()).id();
        let second = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        assert_eq!(assigned(&app, seat), Some(first));

        push_stick(&mut app, second, 1.0);
        app.update();
        assert_eq!(assigned(&app, seat), Some(second), "the player picked up the other pad");
        push_stick(&mut app, second, 0.0);
        app.update();
        assert_eq!(assigned(&app, seat), Some(second), "and keeps it when the stick is at rest");

        app.world_mut().entity_mut(second).remove::<Gamepad>();
        app.update();
        assert_eq!(assigned(&app, seat), Some(first), "its pad is gone, and one is left");
    }

    fn push_stick(app: &mut App, pad: Entity, x: f32) {
        app.world_mut()
            .get_mut::<Gamepad>(pad)
            .expect("a connected pad")
            .analog_mut()
            .set(bevy::input::gamepad::GamepadAxis::LeftStickX, x);
    }

    /// Jon's report, 2026-10-08: one stick moved every cursor. A lobby of the
    /// keyboard and three pads, and the middle pad disconnects as Bevy does it
    /// (the entity stays, the component goes). Measured before: the seat of
    /// the missing pad had no association and followed pad A, and pad C drove
    /// no seat, because the seat above it was gone and its claim was not.
    /// Poison: give the seat of a missing pad no association (`None`).
    #[test]
    fn a_disconnect_in_a_lobby_moves_no_other_controller() {
        let mut app = seat_app();
        app.insert_resource(crate::seating::LocalSeatOffer::offered(
            "a lobby",
            4,
            crate::sources::InputAssignmentPolicy::JoinToClaim,
        ));
        let seats: Vec<Entity> = (0..4).map(|slot| spawn_seat(&mut app, ParticipantId(slot))).collect();
        let pad_a = app.world_mut().spawn(Gamepad::default()).id();
        let pad_b = app.world_mut().spawn(Gamepad::default()).id();
        let pad_c = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        let heard = |app: &App| seats.iter().map(|seat| assigned(app, *seat)).collect::<Vec<_>>();
        assert_eq!(
            heard(&app),
            [Some(NO_PAD), Some(pad_a), Some(pad_b), Some(pad_c)],
            "the keyboard seat and one seat for each pad"
        );

        app.world_mut().entity_mut(pad_b).remove::<Gamepad>();
        app.update();
        assert_eq!(
            heard(&app),
            [Some(NO_PAD), Some(pad_a), Some(NO_PAD), Some(pad_c)],
            "a seat with no association follows the first connected pad, so \
             one stick moves two seats"
        );

        // The pad comes back on its own entity, as a real controller does.
        app.world_mut().entity_mut(pad_b).insert(Gamepad::default());
        app.update();
        assert_eq!(heard(&app), [Some(NO_PAD), Some(pad_a), Some(pad_b), Some(pad_c)]);
    }

    /// In a match a controller cannot take the seat of another one. Player
    /// one's pad disconnects and a different controller connects: the table
    /// puts it in the empty slot, and the frozen seat does not hear it. Then
    /// player one's own pad comes back and the seat hears it again.
    #[test]
    fn a_frozen_seat_does_not_hear_a_different_controller() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);
        let pad_a = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad a")))
            .id();
        let pad_b = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("pad b")))
            .id();
        freeze(
            &mut app,
            Some(LocalChannelPlan::from_sources([0, 1].map(LocalInputSource::Pad))),
        );
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a));

        app.world_mut().entity_mut(pad_a).remove::<Gamepad>();
        let other = app
            .world_mut()
            .spawn((Gamepad::default(), Name::new("another pad")))
            .id();
        app.update();
        assert_eq!(
            app.world().resource::<LocalDeviceOrder>().pad(0),
            Some(other),
            "premise: the new controller is in the slot of the missing one"
        );
        assert_eq!(assigned(&app, one), Some(NO_PAD), "a different controller took the seat");
        assert_eq!(assigned(&app, two), Some(pad_b));

        app.world_mut().entity_mut(other).despawn();
        app.world_mut().entity_mut(pad_a).insert(Gamepad::default());
        app.update();
        assert_eq!(assigned(&app, one), Some(pad_a), "its own controller came back");
    }

    /// One person plays a CPU and holds the SECOND pad. The plan names pad 1,
    /// so the seat hears pad 1. Measured before: a session of one player
    /// cleared the association, leafwing gave the seat the first pad, and the
    /// pad in the player's hands did nothing.
    #[test]
    fn a_lone_player_on_the_second_pad_is_heard_on_that_pad() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let spare = app.world_mut().spawn(Gamepad::default()).id();
        let held = app.world_mut().spawn(Gamepad::default()).id();
        freeze(
            &mut app,
            Some(LocalChannelPlan::from_sources([LocalInputSource::Pad(1)])),
        );
        push_stick(&mut app, spare, 1.0);
        app.update();
        assert_eq!(assigned(&app, one), Some(held));
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
            Some(NO_PAD),
            "the seat of an unplugged controller hears no pad"
        );
        assert_eq!(
            assigned(&app, one),
            Some(first_pad),
            "player one's controller must not be reshuffled because player \
             two unplugged theirs"
        );
    }

    #[test]
    fn a_recycled_entity_index_does_not_reorder_the_controllers() {
        let mut app = seat_app();
        let one = spawn_seat(&mut app, ParticipantId::PRIMARY);
        let two = spawn_seat(&mut app, ParticipantId::SECONDARY);

        // Reserve the low index first. Bevy's allocator buffers freed
        // entities in a local list, so despawning does not reliably recycle an
        // index in a test. `alloc` returns a valid id that is not spawned yet,
        // and `spawn_at` spawns it later.
        let low_index = app.world().entity_allocator().alloc();
        let first_pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        let second_pad = app
            .world_mut()
            .spawn_at(low_index, Gamepad::default())
            .expect("the reserved index is free")
            .id();
        app.update();

        // Keep the premise guard: it proves the test still covers a recycled,
        // lower index.
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

    /// The roster's declaration beats the device count.
    ///
    /// A keyboard player and one pad player is two people, but the device
    /// list has one row. The roster and GGRS session must both see two.
    #[test]
    fn a_declared_roster_seats_two_even_with_one_pad_connected() {
        let mut topology = LocalSeatTopology::default();
        let pad = Entity::from_bits(1 << 32 | 1);
        topology.capture_for_roster(
            &LocalDeviceOrder::from_devices(vec![pad]),
            LocalChannelPlan::from_sources([LocalInputSource::Keyboard, LocalInputSource::Pad(0)]),
        );
        assert_eq!(topology.players(), 2);
        assert_eq!(topology.declared_seats(), Some(2));
        // The declaration also says which source each channel uses: the pad
        // belongs to channel one, and channel zero is on keys.
        assert_eq!(topology.device_for_channel(ParticipantId(1)), Some(pad));
        assert_eq!(topology.device_for_channel(ParticipantId(0)), None);
    }

    /// A spare controller does not add a player.
    ///
    /// A controller left plugged in must not become a second player (the rule
    /// in `seat_input_participants_for_roster`).
    #[test]
    fn a_spare_pad_does_not_inflate_a_declared_solo_session() {
        let mut topology = LocalSeatTopology::default();
        let a = Entity::from_bits(1 << 32 | 1);
        let b = Entity::from_bits(1 << 32 | 2);
        topology.capture_for_roster(
            &LocalDeviceOrder::from_devices(vec![a, b]),
            LocalChannelPlan::from_sources([LocalInputSource::Pad(0)]),
        );
        assert_eq!(topology.players(), 1, "one declared seat is one player");
    }

    /// Callers that never declare anything keep the device count.
    #[test]
    fn an_undeclared_capture_still_counts_devices() {
        let mut topology = LocalSeatTopology::default();
        let a = Entity::from_bits(1 << 32 | 1);
        let b = Entity::from_bits(1 << 32 | 2);
        topology.capture(&LocalDeviceOrder::from_devices(vec![a, b]));
        assert_eq!(topology.players(), 2);
        assert_eq!(topology.declared_seats(), None);
    }

    /// A recapture is a new decision and does not inherit the old declaration.
    #[test]
    fn recapturing_without_a_roster_drops_the_previous_declaration() {
        let mut topology = LocalSeatTopology::default();
        let pad = Entity::from_bits(1 << 32 | 1);
        topology.capture_for_roster(
            &LocalDeviceOrder::from_devices(vec![pad]),
            LocalChannelPlan::from_sources([0, 1, 2, 3].map(LocalInputSource::Pad)),
        );
        assert_eq!(topology.players(), 4);
        topology.capture(&LocalDeviceOrder::from_devices(vec![pad]));
        assert_eq!(
            topology.declared_seats(),
            None,
            "a stale roster must not size a new session"
        );
        assert_eq!(topology.players(), 1);
    }

    use bevy::prelude::Entity;

    fn order(count: usize) -> LocalDeviceOrder {
        LocalDeviceOrder::from_devices(
            (0..count)
                .map(|i| Entity::from_raw_u32(i as u32 + 1).unwrap())
                .collect(),
        )
    }

    /// A session's seating is decided once, and every consumer reads that.
    ///
    /// The roster and rollback session both need the player count. If each
    /// sampled the live order, a connection between samples would make them
    /// disagree, and the roster would seat a fighter with no session handle.
    #[test]
    fn a_frozen_topology_does_not_follow_a_later_connection() {
        let mut topology = LocalSeatTopology::default();
        assert!(!topology.is_frozen(), "nothing has decided the seating yet");

        topology.capture(&order(2));
        assert_eq!(topology.players(), 2);
        assert!(topology.is_frozen());

        // A third pad joins mid-match. The live order changes; the session's
        // seating does not, because the session cannot add a handle.
        let live = order(3);
        assert_eq!(live.connected().len(), 3);
        assert_eq!(
            topology.players(),
            2,
            "a controller connecting mid-session must not silently add a seat \
             the rollback session has no handle for"
        );
    }

    /// Zero devices is one player: a keyboard-only desktop still has a player,
    /// and a session with zero local handles accepts no input.
    #[test]
    fn a_keyboard_only_desktop_is_still_one_player() {
        let mut topology = LocalSeatTopology::default();
        topology.capture(&order(0));
        assert_eq!(topology.players(), 1);
        assert_eq!(
            topology.device_for_channel(ParticipantId(0)),
            None,
            "and it owns no pad"
        );
    }

    /// Recapturing advances the generation even when the seats are the same.
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
        let channel = ParticipantId;
        assert_eq!(topology.device_for_channel(channel(0)), live.pad(0));
        assert_eq!(topology.device_for_channel(channel(1)), live.pad(1));
        assert!(live.pad(1).is_some());
        assert_eq!(
            topology.device_for_channel(channel(2)),
            None,
            "a handle past the connected pads is a CPU or an empty seat, not an error"
        );
    }
}

#[cfg(test)]
mod generation_tests {
    use super::*;

    /// A rebuild advances the generation, whoever rebuilds.
    ///
    /// `generation` lets a consumer detect a rebuild without comparing
    /// vectors, so two rebuilds must never share a number.
    /// `reconcile_roster_with_frozen_topology` returns early on a matching
    /// generation.
    #[test]
    fn capturing_twice_advances_the_generation_rather_than_repeating_it() {
        let order = LocalDeviceOrder::from_devices(Vec::new());
        let mut topology = LocalSeatTopology::default();
        assert_eq!(
            topology.generation(),
            0,
            "a fresh topology has captured nothing"
        );

        topology.capture(&order);
        let device_generation = topology.generation();
        assert!(
            device_generation > 0,
            "a device capture is a rebuild and has to say so"
        );

        // The roster declares a different answer on the same topology.
        // Seeding from the existing topology, not `default()`, keeps the
        // counter moving.
        topology.capture_for_roster(
            &order,
            LocalChannelPlan::from_sources([0, 1].map(LocalInputSource::Pad)),
        );
        assert!(
            topology.generation() > device_generation,
            "the roster's capture must advance past the device capture ({} vs {}) \
             — a consumer keyed on the generation cannot see a rebuild that reuses \
             the number",
            topology.generation(),
            device_generation
        );
        assert_eq!(topology.declared_seats(), Some(2));
    }
}
