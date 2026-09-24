//! Local gamepad ownership for participant seats.
//!
//! A single seat remains unassociated so any connected pad can drive it. With
//! multiple seats, seat `n` owns the `n`-th controller in remembered connection
//! order; unmatched seats clear stale associations. [`LocalDeviceOrder`] records
//! arrival order because Bevy entity indices may be recycled.

use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;

use crate::channels::LocalChannelPlan;
#[cfg(test)]
use crate::channels::LocalInputSource;
use crate::participant::ParticipantId;
use crate::{InputParticipant, Platformer2dInputActionMonolith};

/// Connected controllers, oldest connection first.
///
/// A resource, not a derived sort: the order people picked up their
/// controllers cannot be recovered from the world later.
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

    /// Build an order from a known device list, for a caller that already holds
    /// the devices (a session freezing its seating) and for tests. Only the
    /// tracking system discovers devices.
    pub fn from_devices(devices: Vec<Entity>) -> Self {
        Self(devices)
    }
}

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
    seats: Vec<Entity>,
    /// Roster-declared mapping from input sources to local channels.
    /// `None` means no roster has declared a plan and device discovery supplies
    /// the fallback topology.
    declared: Option<LocalChannelPlan>,
}

impl LocalSeatTopology {
    /// Freeze the current device order as this session's seating.
    ///
    /// The generation advances on every capture, also when the seats are the
    /// same. Consumers cache against "the topology was decided again" (as with
    /// `CharacterCatalogGeneration`).
    pub fn capture(&mut self, order: &LocalDeviceOrder) {
        self.generation = self.generation.wrapping_add(1);
        self.seats = order.devices().to_vec();
        // A recapture is a new decision. A declaration from an old roster must
        // not size this session.
        self.declared = None;
    }

    /// How many local players this session seats. At least one: a
    /// keyboard-only desktop has no device rows but has a player, and a
    /// session with zero local handles accepts no input.
    ///
    /// The roster's declaration wins when present (see `declared`). The device
    /// count is the fallback.
    pub fn players(&self) -> usize {
        match &self.declared {
            Some(plan) => plan.channels().max(1),
            None => self.seats.len().max(1),
        }
    }

    /// Freeze the device order AND the channel plan the roster declared.
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

    /// The controller a channel drives, if any.
    ///
    /// `None` is a channel with no pad: one on the keyboard, or one whose
    /// controller is unplugged. Neither is an error.
    pub fn device_for_channel(&self, channel: ParticipantId) -> Option<Entity> {
        let index = match &self.declared {
            Some(plan) => plan.source_for(channel)?.pad_index()?,
            None => channel.slot() as usize,
        };
        self.seats.get(index).copied()
    }

    /// The controller at this index of the frozen device order.
    ///
    /// This is a device index, not a channel. To ask which pad a seat drives,
    /// use [`Self::device_for_channel`].
    pub fn device_at(&self, index: usize) -> Option<Entity> {
        self.seats.get(index).copied()
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
    // Write only on a real change. This runs every frame, and an
    // unconditional `ResMut` deref would mark the order changed every frame.
    if next != order.0 {
        order.0 = next;
    }
}

/// What a controller is, across disconnects.
///
/// An `Entity` cannot answer this: a reconnecting pad is a new entity with a
/// new generation. The OS-provided name and USB vendor/product survive an
/// unplug.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PadIdentity {
    name: Option<String>,
    vendor: Option<u16>,
    product: Option<u16>,
}

impl PadIdentity {
    fn of(pad: Option<&Gamepad>, name: Option<&Name>) -> Self {
        Self {
            name: name.map(|name| name.as_str().to_string()),
            vendor: pad.and_then(|pad| pad.vendor_id()),
            product: pad.and_then(|pad| pad.product_id()),
        }
    }

    /// Whether this identity says anything at all.
    fn is_known(&self) -> bool {
        self.name.is_some() || self.vendor.is_some() || self.product.is_some()
    }
}

/// Which pad each seat holds, kept across disconnects.
///
/// Positional assignment (seat `n` gets the `n`-th pad) is wrong: when a pad
/// leaves, [`LocalDeviceOrder`] drops it, every later seat shifts down one,
/// and a seat takes another player's controller.
///
/// So the assignment is remembered, not recomputed. A seat keeps its pad
/// while that pad exists. A pad that leaves frees only its own seat. A free
/// pad goes only to a seat that has none, so a reconnect restores the same
/// assignment.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct SeatDeviceOwnership {
    held: std::collections::BTreeMap<u8, Entity>,
    /// What each seat's controller was, kept after it disconnects so the
    /// same controller finds the same seat when it comes back.
    remembered: std::collections::BTreeMap<u8, PadIdentity>,
}

impl SeatDeviceOwnership {
    /// The pad this seat holds, if it still has one.
    pub fn pad_for(&self, slot: u8) -> Option<Entity> {
        self.held.get(&slot).copied()
    }

    /// Whether any seat holds this pad.
    pub fn is_held(&self, pad: Entity) -> bool {
        self.held.values().any(|held| *held == pad)
    }

    /// The seat waiting for this exact controller, if one is.
    fn seat_awaiting(&self, identity: &PadIdentity) -> Option<u8> {
        if !identity.is_known() {
            return None;
        }
        self.remembered
            .iter()
            .find(|(slot, remembered)| *remembered == identity && !self.held.contains_key(slot))
            .map(|(slot, _)| *slot)
    }

    fn claim(&mut self, slot: u8, pad: Entity, identity: PadIdentity) {
        self.held.insert(slot, pad);
        self.remembered.insert(slot, identity);
    }

    /// Forget the entity each seat holds when its pad is gone. Keep the
    /// remembered identity.
    fn retire_missing(&mut self, live: &[Entity]) {
        self.held.retain(|_, pad| live.contains(pad));
    }
}

/// Which pad the frozen session decided this seat holds.
///
/// The declared plan is the answer when there is one. It says which source
/// each dense channel listens to. Example: a lobby that seats the human on
/// pad 1 against a CPU declares `channel 0 -> pad 1`. Indexing the device
/// order by channel would give them an unused pad.
///
/// Without a plan, the seat number indexes the device order, minus one for a
/// keyboard seat below it (the keyboard is not a row). This fallback can only
/// express a keyboard player in a lower seat than a pad player. When it fails
/// it returns `None`, and that player has no input.
fn frozen_pad_for_seat(
    topology: &LocalSeatTopology,
    slot: u8,
    keyboard_owner: Option<ParticipantId>,
) -> Option<Entity> {
    if topology.declared_channels().is_some() {
        return topology.device_for_channel(ParticipantId(slot));
    }
    let pad_index = match keyboard_owner {
        Some(owner) if owner.slot() < slot => slot.saturating_sub(1),
        _ => slot,
    };
    topology.device_at(pad_index as usize)
}

/// Give each local seat its own controller.
///
/// Runs in `PreUpdate` before leafwing resolves actions, so a seat that joins
/// is playable on the tick it joins.
///
/// While a session owns a frozen topology, the mapping comes from it.
/// `LocalDeviceOrder` is live, and a disconnect that reorders it would change
/// which physical device drives each GGRS handle. The session must freeze the
/// mapping, not only the handle count.
///
/// Live discovery still runs for the next session. It does not change this
/// one.
pub fn assign_local_seat_devices(
    order: Res<LocalDeviceOrder>,
    topology: Option<Res<LocalSeatTopology>>,
    offer: Option<Res<crate::seating::LocalSeatOffer>>,
    keyboard: Option<Res<crate::sources::KeyboardOwner>>,
    mut ownership: ResMut<SeatDeviceOwnership>,
    pads: Query<(Option<&Gamepad>, Option<&Name>)>,
    mut seats: Query<(
        &InputParticipant,
        &mut InputMap<Platformer2dInputActionMonolith>,
    )>,
) {
    let frozen = topology.filter(|topology| topology.is_frozen());
    // Use the frozen topology's player count, not the number of seat
    // entities. During activation a two-player topology can exist while only
    // the primary entity does. Counting entities would take the solo branch,
    // clear the primary's pad restriction, and let handle 1's pad drive seat 0.
    let players = match frozen.as_ref() {
        Some(topology) => topology.players(),
        None => seats.iter().len(),
    };
    // Solo: keep leafwing's any-pad behaviour.
    if players < 2 {
        for (_, mut map) in &mut seats {
            if map.gamepad().is_some() {
                map.clear_gamepad();
            }
        }
        return;
    }

    // A seat that holds the keyboard does not also take a pad. Otherwise,
    // with one keyboard player and one pad player, the only pad goes to the
    // keyboard player.
    //
    // Under the default `UnifiedPrimary` this is `None` and
    // `pad_index == slot`. A session opts in to couch partitioning; a second
    // controller does not impose it.
    //
    // A declared plan outranks the policy. `keyboard_owner_for` answers
    // `Some(PRIMARY)` for every `JoinToClaim` session, which binds that seat
    // to `Entity::PLACEHOLDER` and deafens it to all pads. That is wrong for
    // the Smash couch, where both players hold pads. A plan says who is on
    // the keyboard, including nobody.
    let keyboard_owner = match frozen.as_ref().and_then(|t| t.declared_channels()) {
        Some(plan) => plan.keyboard_channel(),
        None => crate::sources::keyboard_owner_for(
            offer.map(|offer| offer.policy()).unwrap_or_default(),
            keyboard.map(|keyboard| *keyboard).unwrap_or_default(),
            players,
        ),
    };

    // Pads that still exist. A seat's claim survives only while its pad does.
    let live: Vec<Entity> = order.devices().to_vec();
    ownership.retire_missing(&live);

    // Claim in slot order, not query order. A free pad goes to the lowest
    // seat that needs one. Archetype iteration order is not stable (ADR 0023).
    let mut order_of_seats: Vec<(u8, Entity)> = seats
        .iter()
        .map(|(participant, _)| (participant.id.slot(), participant.id))
        .map(|(slot, _)| (slot, Entity::PLACEHOLDER))
        .collect();
    order_of_seats.sort_by_key(|(slot, _)| *slot);
    let seat_slots: Vec<u8> = order_of_seats.into_iter().map(|(slot, _)| slot).collect();
    // A returning controller goes back to its own seat first.
    {
        for pad in &live {
            if ownership.is_held(*pad) {
                continue;
            }
            let identity = pads
                .get(*pad)
                .map(|(gamepad, name)| PadIdentity::of(gamepad, name))
                .unwrap_or_default();
            if let Some(slot) = ownership.seat_awaiting(&identity) {
                ownership.claim(slot, *pad, identity);
            }
        }
    }

    // Then seats with no pad take the remaining pads, in slot order. The write
    // pass below only reads these decisions.
    for slot in &seat_slots {
        let slot = *slot;
        if keyboard_owner.map(|owner| owner.slot()) == Some(slot) {
            continue;
        }
        if ownership
            .pad_for(slot)
            .is_some_and(|pad| live.contains(&pad))
        {
            continue;
        }
        if let Some(topology) = frozen.as_ref() {
            // In a frozen session the topology decides the pad, but ownership
            // must still record its identity. Without this, `remembered` stays
            // empty, the identity pass above does nothing, and a reconnected
            // pad never returns to its seat.
            //
            // This does not reorder the freeze: the pad comes from the
            // topology's recorded handle, not from the free pads.
            if let Some(pad) = frozen_pad_for_seat(topology, slot, keyboard_owner)
                .filter(|pad| live.contains(pad) && !ownership.is_held(*pad))
            {
                let identity = pads
                    .get(pad)
                    .map(|(gamepad, name)| PadIdentity::of(gamepad, name))
                    .unwrap_or_default();
                ownership.claim(slot, pad, identity);
            }
            continue;
        }
        if let Some(free) = live.iter().copied().find(|pad| !ownership.is_held(*pad)) {
            let identity = pads
                .get(free)
                .map(|(gamepad, name)| PadIdentity::of(gamepad, name))
                .unwrap_or_default();
            ownership.claim(slot, free, identity);
        }
    }

    for (participant, mut map) in &mut seats {
        let slot = participant.id.slot();
        let wanted = if keyboard_owner == Some(participant.id) {
            // `Entity::PLACEHOLDER` is leafwing's fallback when no gamepad
            // exists, so no real pad matches it. A keyboard seat hears no pad.
            Some(Entity::PLACEHOLDER)
        } else if let Some(topology) = frozen.as_ref() {
            // A frozen session's mapping does not move.
            let recorded = frozen_pad_for_seat(topology, slot, keyboard_owner);
            match recorded {
                // Still plugged in: the session's answer stands.
                Some(pad) if live.contains(&pad) => Some(pad),
                // The recorded pad is gone. If the same controller came back,
                // the identity pass gave it back to this seat.
                _ => ownership
                    .pad_for(slot)
                    .filter(|pad| live.contains(pad))
                    // Otherwise stay deaf, never `None`. A dead entity matches
                    // no pad, but `None` matches every pad and would put
                    // another player's pad into this frozen seat.
                    .or(recorded)
                    .or(Some(Entity::PLACEHOLDER)),
            }
        } else {
            // The pad this seat holds, decided above. Another player's
            // unplug does not move it.
            ownership.pad_for(slot).filter(|pad| live.contains(pad))
        };
        // Skip unchanged maps. `InputMap` is a component, and writing it every
        // frame marks it changed for every observer, including the settings
        // UI, which rebuilds bindings on change.
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
        app.init_resource::<SeatDeviceOwnership>();
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

        // Two declared seats, one device. Frozen before any assignment pass,
        // as in a real match.
        let frozen = {
            let mut topology = LocalSeatTopology::default();
            topology.capture_for_roster(
                &LocalDeviceOrder::from_devices(vec![pad]),
                LocalChannelPlan::from_sources([
                    LocalInputSource::Keyboard,
                    LocalInputSource::Pad(0),
                ]),
            );
            topology
        };
        app.insert_resource(frozen);
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
        assert_eq!(assigned(&app, two), None);
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
        assert_eq!(assigned(&app, one), None);
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
        assert_eq!(assigned(&app, one), None);
        assert_eq!(assigned(&app, two), None);

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
            None,
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
        // Not `None`: that means "any pad" and would put pad B in this seat.
        // A dead id hears nothing, which is correct for a missing controller.
        assert_ne!(
            assigned(&app, one),
            Some(pad_b),
            "seat one was handed player two's controller"
        );
        assert_ne!(
            assigned(&app, one),
            None,
            "an unset gamepad answers every pad"
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
        assert_eq!(assigned(&app, two), None);

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
            None,
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
    /// primary participant exists. Counting entities would take the solo
    /// branch and let any pad drive seat 0.
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

        // In the shipped path, ownership is empty at the freeze: before a
        // roster exists there is one participant, and `players < 2` returns
        // early. This test uses that order.
        let frozen = {
            let mut topology = LocalSeatTopology::default();
            topology.capture_for_roster(
                &LocalDeviceOrder::from_devices(vec![pad_a, pad_b]),
                LocalChannelPlan::from_sources([0, 1].map(LocalInputSource::Pad)),
            );
            topology
        };
        assert!(frozen.is_frozen(), "the fixture must actually freeze");
        app.insert_resource(frozen);
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
        assert_eq!(live.devices().len(), 3);
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
        assert_eq!(
            topology.device_for_channel(channel(0)),
            Some(live.devices()[0])
        );
        assert_eq!(
            topology.device_for_channel(channel(1)),
            Some(live.devices()[1])
        );
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
