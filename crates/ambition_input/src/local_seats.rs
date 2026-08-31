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
/// Frozen local-device topology for one gameplay session.
///
/// Roster sizing, rollback handles, and input latches read the same snapshot so a
/// connection change cannot make them disagree mid-session. `generation` changes
/// on each recapture so consumers can invalidate cached assignments.
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
    /// Advances the generation on every capture, INCLUDING one that produces
    /// the same seats: "the topology was decided again" is the fact a consumer
    /// caches against, and two identical captures at different times are still
    /// two decisions (the same reasoning as `CharacterCatalogGeneration`).
    pub fn capture(&mut self, order: &LocalDeviceOrder) {
        self.generation = self.generation.wrapping_add(1);
        self.seats = order.devices().to_vec();
        // A recapture is a NEW decision, so a declaration from the previous one
        // does not carry: leaving it would let a roster that has since gone away
        // keep sizing the session.
        self.declared = None;
    }

    /// How many local players this session seats. At least one: a keyboard-only
    /// desktop has no device rows and still has a player, and a session with
    /// zero local handles accepts input from nobody.
    ///
    /// The ROSTER's declaration wins when there is one — see `declared`. The
    /// device count is the fallback for every caller that froze a topology
    /// before rosters could speak.
    pub fn players(&self) -> usize {
        match &self.declared {
            Some(plan) => plan.channels().max(1),
            None => self.seats.len().max(1),
        }
    }

    /// Freeze the device order AND the channel plan the roster declared.
    ///
    /// The separate entry point is deliberate: [`Self::capture`] is called from
    /// paths that legitimately have no roster (the rollback observatory, device
    /// probes), and giving them a `None` to pass would make "nobody declared"
    /// look like a decision somebody made.
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

    /// The controller a CHANNEL drives, if that channel drives one.
    ///
    /// `None` is a channel with no pad: one playing on the KEYBOARD, or one
    /// whose controller is unplugged. Neither is an error.
    pub fn device_for_channel(&self, channel: ParticipantId) -> Option<Entity> {
        let index = match &self.declared {
            Some(plan) => plan.source_for(channel)?.pad_index()?,
            None => channel.slot() as usize,
        };
        self.seats.get(index).copied()
    }

    /// The controller at this index of the frozen device order.
    ///
    ///  a DEVICE index, not a channel — for a caller that is asking about
    /// the hardware order itself. Anything asking "which pad does this seat
    /// drive" wants [`Self::device_for_channel`].
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
    // Write only on a real change: this runs every frame, and an unconditional
    // `ResMut` deref would mark the order changed forever.
    if next != order.0 {
        order.0 = next;
    }
}

/// Which pad each seat is HOLDING, remembered across disconnects.
///
/// Positional assignment (seat `n` → the `n`-th connected pad) transfers ownership the moment a pad
/// leaves, because [`LocalDeviceOrder`] forgets disconnections and every later seat shifts down
/// one. Measured: two seats, two pads, unplug player ONE's — and player one's seat took player
/// TWO's controller while player two was holding it.
///
/// So the assignment is a FACT that is remembered, not a position that is
/// recomputed. A seat keeps its pad while that pad exists; a pad that leaves
/// frees exactly its own seat; and a free pad is only taken by a seat that has
/// none — which is also what makes reconnecting restore the same assignment
/// (milestone 7) rather than reshuffling everybody.
/// What a controller IS, across disconnects.
///
///  an `Entity` cannot answer this: a reconnecting pad is a new entity and Bevy
/// moves the generation, so a remembered id never matches again. The OS-provided
/// name and USB vendor/product are what survive being unplugged.
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

#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct SeatDeviceOwnership {
    held: std::collections::BTreeMap<u8, Entity>,
    /// What each seat's controller WAS, kept after it disconnects so the same
    /// controller coming back finds the same seat.
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

    /// Forget which ENTITY each seat holds when its pad is gone — but remember
    /// WHAT it was, which is the whole point.
    fn retire_missing(&mut self, live: &[Entity]) {
        self.held.retain(|_, pad| live.contains(pad));
    }
}

/// Which pad the frozen session decided this seat holds.
///
///  the declared plan is the answer when there is one. It says which
/// SOURCE each dense channel listens to, which is exactly this question, and it
/// needs no positional correction: a lobby that seated the human holding pad 1
/// against a CPU declares `channel 0 → pad 1`, and reading the device order at
/// the channel's own index would hand them the pad nobody is touching.
///
/// Without a plan nobody said otherwise, so the seat number indexes the device
/// order — minus the seat playing on keys, which is not a row in it. That
/// subtraction is the shape the plan replaces: it can only express a keyboard
/// player in a seat BELOW a pad player, and it fails by producing `None`, which
/// is a person who is simply inert.
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
/// Runs in `PreUpdate` before leafwing resolves actions, so an association made
/// this frame is honoured by this frame's `ActionState` — a seat that joins is
/// playable on the tick it joins, not the one after.
///  The mapping comes from the frozen topology while a session owns one.
/// `LocalDeviceOrder` is live, and a session that froze
/// `handle 0 → keyboard, 1 → pad A, 2 → pad B` and then let a disconnect reorder
/// the live list would keep its GGRS handle COUNT while quietly changing which
/// physical device drives each handle. Freezing the count and not the mapping is
/// freezing the easy half.
///
/// Live discovery still runs — it is what the NEXT session freezes — it just does
/// not get to redecide this one.
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

    // A seat that already holds the keyboard does not also take a pad.
    //
    // Positional assignment (seat `n` → pad `n`) is right when every seat is a pad seat, and wrong
    // the moment one of them is playing on the keyboard: with one keyboard player and one pad
    // player it hands the ONLY pad to the person already typing and leaves the pad player with
    // nothing.
    //
    //  under the default `UnifiedPrimary` this is `None` and the arithmetic
    // below is `pad_index == slot` — today's behaviour, byte for byte. Couch
    // partitioning is something a session opts into, not something a second
    // controller imposes on a solo player.
    //
    //  A DECLARED PLAN OUTRANKS THE POLICY, because the policy is a guess and
    // the plan is what somebody chose. `keyboard_owner_for` answers
    // `Some(PRIMARY)` for every `JoinToClaim` session — "nobody has claimed the
    // keyboard, so it stays with the seat that has been playing" — and that seat
    // is then bound to `Entity::PLACEHOLDER`, deaf to every controller in the
    // room. Correct for a lobby where somebody IS on keys; wrong for the shipped
    // Smash couch, where two people pick up two pads and player one's pad stops
    // working. A plan says who is on the keyboard, including *nobody*.
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

    // CLAIMED IN SLOT ORDER, not in query order. A free pad goes to the
    // lowest seat that needs one, and archetype iteration is not an order — two
    // runs of the same world could hand the same controller to different people
    // (ADR 0023).
    let mut order_of_seats: Vec<(u8, Entity)> = seats
        .iter()
        .map(|(participant, _)| (participant.id.slot(), participant.id))
        .map(|(slot, _)| (slot, Entity::PLACEHOLDER))
        .collect();
    order_of_seats.sort_by_key(|(slot, _)| *slot);
    let seat_slots: Vec<u8> = order_of_seats.into_iter().map(|(slot, _)| slot).collect();
    // A RETURNING CONTROLLER GOES BACK TO ITS OWN SEAT FIRST.
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

    // Then the seats that still have nothing take whatever is left, in slot
    // order — the write pass below only reads decisions.
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
            // A FROZEN SESSION DECIDES WHICH PAD; OWNERSHIP STILL HAS TO LEARN WHICH ONE. This
            // was a bare `continue`, so in a frozen session no seat ever claimed anything and
            // `remembered` stayed EMPTY — which silently disabled the identity pass above for
            // exactly the sessions that have one. Not a swap — that player is simply deaf for the
            // rest of the match, because a dead id matches no live gamepad and nothing was left to
            // repair it with.
            //
            // This does not let the freeze be REORDERED: the pad comes from the
            // topology's own recorded handle, not from whatever is free. It only
            // records the identity of the controller the session already chose,
            // so that a later reconnect has something to match.
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
            // `Entity::PLACEHOLDER` is what leafwing's own fallback resolves to
            // when no gamepad exists, so associating it means no REAL pad ever
            // matches: a seat playing on keys is deaf to every controller in the
            // room, which is what owning the keyboard has to mean.
            Some(Entity::PLACEHOLDER)
        } else if let Some(topology) = frozen.as_ref() {
            // A frozen session's mapping is the session's own answer and does not
            // move — that is the whole point of freezing it.
            let recorded = frozen_pad_for_seat(topology, slot, keyboard_owner);
            match recorded {
                // Still plugged in: the session's answer stands.
                Some(pad) if live.contains(&pad) => Some(pad),
                // The recorded pad is GONE. If the same controller has come back
                // it reclaimed this seat by identity above, and that is not a
                // reorder — it is the seat getting its own pad again.
                _ => ownership
                    .pad_for(slot)
                    .filter(|pad| live.contains(pad))
                    //  otherwise stay DEAF, never `None`. A dead entity matches
                    // no live gamepad and `None` matches EVERY one, so falling
                    // through to `None` here would promote whatever pad is left
                    // into a seat the session froze — the exact swap the freeze
                    // exists to prevent, and the reason this branch keeps the
                    // dead id rather than clearing it.
                    .or(recorded)
                    .or(Some(Entity::PLACEHOLDER)),
            }
        } else {
            // THE PAD IN THEIR HANDS, decided above. A seat keeps its
            // controller while that controller exists, so somebody else
            // unplugging theirs cannot reshuffle this one — which is what
            // positional assignment did, because `LocalDeviceOrder` forgets the
            // pad that left and every later seat shifts down one.
            ownership.pad_for(slot).filter(|pad| live.contains(pad))
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

///  `cargo test -p ambition_input` does not run any of this. The module is
/// `#[cfg(feature = "input")]`, so a bare per-crate invocation reports 55 tests
/// passing while every device-ownership test here — the couch-multiplayer ones —
/// is compiled out. Ask for them explicitly:
///
/// ```bash
/// cargo test -p ambition_input --features input     # 84, not 55
/// ```
///
/// The suite itself is fine: `cargo test --workspace` unifies the feature on
/// through `ambition_platformer2d_actor_monolith`'s
/// `default → desktop_dev → visible → input`. This note exists because the
/// per-crate number is the one a person reads while iterating, and it is
/// silently partial.
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

    /// A keyboard player beside one pad player, FROZEN, through a reconnect.
    ///
    /// The frozen path shifts the handle index for every seat below the keyboard
    /// owner (`slot - 1`), because the keyboard is not a device row: two players
    /// declare two seats but the topology holds ONE device. That arithmetic was
    /// added along with the identity recording and nothing exercised it — the
    /// frozen tests had no keyboard owner and the keyboard test was not frozen.
    ///
    ///  getting the shift wrong is silent: seat one would claim
    /// `device_for_handle(1)`, which is `None` in a one-device topology, so the
    /// pad player simply never gets a pad and the keyboard player is unaffected.
    /// Nothing crashes and nothing logs — the second player is just inert, which
    /// is indistinguishable from "couch multiplayer does not work".
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

        // TWO declared seats, ONE device — the keyboard player has no row. Frozen
        // before any assignment pass, the way a real match freezes.
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

    ///  the shipped Smash couch, and it did not work. That demo claims
    /// `JoinToClaim` on its own routes, and `keyboard_owner_for` answers
    /// `Some(PRIMARY)` for every `JoinToClaim` session — "nobody has claimed the
    /// keyboard, so it stays with the seat that has been playing". So player
    /// one, holding a controller, was bound to `Entity::PLACEHOLDER` and deaf to
    /// it, while player two's pad worked fine. Two people, one of whom cannot
    /// move.
    ///
    ///  a declared plan is a stated fact and outranks the policy's guess.
    /// It says who is on the keyboard, including *nobody*.
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

    /// A CHANNEL LISTENS TO THE SOURCE IT WAS GIVEN, not to its own number.
    ///
    /// Two people picked up the second and third controllers — the first is on
    /// the desk, plugged in and unclaimed. Positionally, channel 0 would take
    /// the spare and channel 1 would take one of theirs.
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
    /// Same world as the couch test above and no policy resource at all. Seat
    /// one keeps the pad, because a session that never asked for couch
    /// partitioning must not get it — a single player with a controller and a
    /// keyboard is the common case, and the couch work is not allowed to charge
    /// them for it.
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

    /// PROBE: milestone 6 — *"Disconnecting the gamepad does not
    /// transfer ownership."* Two seats, two pads, unplug player ONE's.
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

    ///  it cannot be by ENTITY — a reconnecting pad is a new one, and Bevy moves
    /// the generation so a despawned id is never handed back. What restores the
    /// participant is that the seat which lost its pad is the seat still holding
    /// NONE, so the free pad finds it rather than displacing somebody.
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
    ///  the first reconnect test left exactly ONE seat vacant, so any returning
    /// pad necessarily filled the "correct" one. It proved vacancy filling and
    /// was written up as proving RESTORATION. With both seats vacant the
    /// difference is the whole thing: pad B coming back first took seat 0, and
    /// pad A took seat 1 — the two people swapped characters by unplugging in one
    /// order and plugging in in another.
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
    ///  freezing records ENTITIES, and an entity dies when its pad is unplugged.
    /// The frozen branch skipped the ownership pass entirely, so a seat whose pad
    /// came back kept pointing at a dead id forever: the freeze protected the
    /// mapping from being REORDERED and also from being REPAIRED.
    ///
    /// The freeze's purpose is that nobody else may take this seat's controller.
    /// Handing the seat back the same controller is not a reorder.
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
        //  NOT `None` — that means "any pad" and would promote pad B into this
        // seat, which is the swap the freeze exists to prevent. A dead id is
        // DEAF, which is the truthful state for a seat whose controller is gone.
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

    ///  A frozen session's seats must still REMEMBER which controller they
    /// hold, or reconnecting one is unrepairable.
    ///
    /// Measured end-to-end in `game/ambition_app/tests/rollback_seat_devices.rs`: unplug seat
    /// two's pad and plug it back in, and its map still pointed at the DEAD entity — that
    /// player deaf for the rest of the match.
    ///
    /// So ownership is empty at every real freeze.
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

        // The shipping path never gets that free pass: before a roster exists
        // there is one participant, `players < 2` returns early, and the freeze
        // arrives with ownership still empty. The order below is that order.
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

        // The SAME controller comes back as a new entity — which is what a
        // reconnect is; the old `Entity` cannot return.
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

        // RESERVE the low index up front rather than hoping a despawn recycles
        // one. Under Bevy 0.18 this burned two scratch entities, despawned them
        // and trusted the next spawn to take a freed index. Bevy 0.19's
        // allocator buffers freed entities in a 128-slot LOCAL list that only
        // flushes into the shared free list when it fills, so two despawns
        // recycle nothing inside a test and the premise guard below fired
        // instead (got index 13 then 14). `alloc` hands out an id that is valid
        // but not yet spawned, and `spawn_at` spawns it later — which is the
        // documented way to enforce a property about a group's entity indices.
        let low_index = app.world().entity_allocator().alloc();
        let first_pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        let second_pad = app
            .world_mut()
            .spawn_at(low_index, Gamepad::default())
            .expect("the reserved index is free")
            .id();
        app.update();

        // ⛔ THE PREMISE STAYS A GUARD even though the fixture now constructs it:
        // it is what says this test is still about a RECYCLED, LOWER index, and
        // it is what caught the allocator change in the first place.
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

    /// S34: the roster's declaration beats the device count.
    ///
    /// A keyboard player and one pad player is TWO people. The device list has
    /// one row, because the keyboard is not a device row, so the old
    /// `seats.len()` answered one — and the versus roster and the GGRS session
    /// were both sized from that answer.
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
        //  and the declaration says WHICH source each channel listens to, which
        // is the half a count could not carry: the pad belongs to channel ONE,
        // and channel zero is the person on keys rather than a second claimant
        // on the only controller in the room.
        assert_eq!(topology.device_for_channel(ParticipantId(1)), Some(pad));
        assert_eq!(topology.device_for_channel(ParticipantId(0)), None);
    }

    /// A spare controller does not add a player.
    ///
    /// The failure `seat_input_participants_for_roster` names in its own doc —
    /// "a controller left plugged into a machine silently becomes a second
    /// player in every game on it" — was live one layer below that rule.
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

    /// Callers that never declare anything keep the device count, byte for byte.
    #[test]
    fn an_undeclared_capture_still_counts_devices() {
        let mut topology = LocalSeatTopology::default();
        let a = Entity::from_bits(1 << 32 | 1);
        let b = Entity::from_bits(1 << 32 | 2);
        topology.capture(&LocalDeviceOrder::from_devices(vec![a, b]));
        assert_eq!(topology.players(), 2);
        assert_eq!(topology.declared_seats(), None);
    }

    ///  a recapture is a NEW decision and must not inherit the old declaration.
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
        assert_eq!(
            topology.device_for_channel(ParticipantId(0)),
            None,
            "and it owns no pad"
        );
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
    /// `generation` exists so a consumer can notice a rebuild *"rather than compare vectors"*,
    /// and two independent rebuilds sharing a number is exactly what it cannot notice.
    ///
    ///  that collision was LOAD-BEARING while `declared_seats` counted CPUs:
    /// `reconcile_roster_with_frozen_topology` early-returns on a matching
    /// generation, and the match was the only thing stopping it rebuilding
    /// versus' roster as two HUMAN seats against a one-handle session. It counts
    /// humans now, so the rebuild it suppressed produces the right answer.
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

        // The roster then declares a different answer, on top of the SAME
        // topology — which is the fix: seeding from the existing one rather than
        // from `default()` is what keeps the counter moving.
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
