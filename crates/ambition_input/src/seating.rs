//! Where a session's local seats come from, and who decides.
//!
//! The number of players is an input fact. A lobby, a roster, or a
//! fixed two-player experience decides it; a backend never does. This
//! declaration lives here, not in the rollback backend, so every surface can
//! read it. If a session opens its handles from the device count, a declared
//! second seat gets a participant and a pad but its body does not move,
//! because the session is never resized.
//!
//! A roster is one possible decider. A plaza with no roster can also be
//! two-player. The type means: a decider claimed local seating, and this is
//! its answer.

use bevy::prelude::{Entity, Resource, SystemSet};

/// The `Update` set in which a composition states its seating: the writers of
/// [`LocalSeatOffer`] and [`SessionSeatingSource`]. A host that sizes a session
/// from the seating runs after this set, so a session is built from the seating
/// stated this frame and not from the seating before it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeatingDeclared;

/// Where this session's seats come from, whether they are decided yet, and
/// whose answer it is.
///
/// One value covers the whole chain: an experience claims local seating, its
/// answer becomes decided, the participant topology freezes from that answer,
/// the session builds from that topology, and the claim is released when the
/// experience ends. A roster is the usual decider but not the only one; a
/// two-observer plaza declares two channels with no lobby.
///
/// [`Self::Devices`] is a real answer. Single-player games, headless oracles,
/// and demos declare nothing and seat from connected devices. Declared seating
/// is opt-in, so compositions that never declare do not stall on the gate.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub enum SessionSeatingSource {
    /// Nobody claimed local seating: freeze from connected devices.
    #[default]
    Devices,
    /// `owner` will publish its answer and has not yet. The session does not
    /// start: a topology frozen from devices now could disagree with the
    /// answer, and the session is never resized.
    Pending { owner: String },
    /// `owner` decided `channels`. The maintainer stamps `frozen_topology` with
    /// the generation it captured, so the roster, the handle count, and the
    /// per-seat latches all cite one number.
    ///
    /// A seat count is not enough (see `LocalSeatTopology`). A count opens the
    /// right number of GGRS handles but does not say which controller feeds
    /// each. Consumers that derived that from the roster's sparse source
    /// numbers put a fighter on an unopened channel when a CPU was seated
    /// before a human.
    Decided {
        owner: String,
        channels: crate::LocalChannelPlan,
        frozen_topology: Option<u64>,
    },
}

impl SessionSeatingSource {
    /// `owner` intends to decide local seating and has not yet.
    pub fn pending(owner: impl Into<String>) -> Self {
        Self::Pending {
            owner: owner.into(),
        }
    }

    /// `owner` decided which source drives which channel.
    pub fn decided(owner: impl Into<String>, channels: crate::LocalChannelPlan) -> Self {
        Self::Decided {
            owner: owner.into(),
            channels,
            frozen_topology: None,
        }
    }

    /// Which experience claimed local seating, if any.
    pub fn owner(&self) -> Option<&str> {
        match self {
            Self::Devices => None,
            Self::Pending { owner } | Self::Decided { owner, .. } => Some(owner),
        }
    }

    pub fn is_owned_by(&self, owner: &str) -> bool {
        self.owner() == Some(owner)
    }

    /// The decided channel plan, or `None` while seating is pending or
    /// device-driven.
    pub fn channel_plan(&self) -> Option<&crate::LocalChannelPlan> {
        match self {
            Self::Decided { channels, .. } => Some(channels),
            _ => None,
        }
    }

    /// The decided seat count, or `None` while seating is pending or device-driven.
    pub fn seat_count(&self) -> Option<usize> {
        self.channel_plan().map(|channels| channels.channels())
    }

    /// The topology generation the session was built from, once one was frozen.
    pub fn frozen_topology(&self) -> Option<u64> {
        match self {
            Self::Decided {
                frozen_topology, ..
            } => *frozen_topology,
            _ => None,
        }
    }

    /// Give the claim back, if it is this owner's to give.
    ///
    /// Returns whether anything was released.
    pub fn release(&mut self, owner: &str) -> bool {
        if !self.is_owned_by(owner) {
            return false;
        }
        *self = Self::Devices;
        true
    }
}

/// The local seats the active surface offers, and in whose name.
///
/// Unlike [`SessionSeatingSource`], an offer follows surface lifetime and never
/// freezes session topology. A route states its offer to the game shell, which
/// writes this value while the route is active; the owner lets the shell
/// withdraw only an offer that a route stated.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalSeatOffer {
    owner: Option<String>,
    seats: u8,
    policy: crate::sources::InputAssignmentPolicy,
}

impl LocalSeatOffer {
    /// `owner` offers `seats` local seats under `policy`, taking the claim over
    /// from whoever held it.
    pub fn offered(
        owner: impl Into<String>,
        seats: u8,
        policy: crate::sources::InputAssignmentPolicy,
    ) -> Self {
        Self {
            owner: Some(owner.into()),
            seats,
            policy,
        }
    }

    /// How many local seats are on offer. `0` (the default) means no offer,
    /// which is every single-participant route.
    ///
    /// A count only means "seats 0..n, dense". When players are not on the
    /// first n sources (keyboard below a pad), use a
    /// [`crate::LocalChannelPlan`]; do not extend this field.
    pub fn seats(&self) -> u8 {
        self.seats
    }

    /// How local sources become participants while this offer stands. An
    /// unclaimed offer gives the default, which is solo behaviour.
    pub fn policy(&self) -> crate::sources::InputAssignmentPolicy {
        self.policy
    }

    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }
}

/// What a controller is, across disconnects.
///
/// The OS-provided name and the USB vendor and product identifiers stay the
/// same when a controller is unplugged and plugged in again. Two controllers
/// of one model have equal identities.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PadIdentity {
    name: Option<String>,
    vendor: Option<u16>,
    product: Option<u16>,
}

impl PadIdentity {
    pub fn new(name: Option<String>, vendor: Option<u16>, product: Option<u16>) -> Self {
        Self {
            name,
            vendor,
            product,
        }
    }

    /// Whether this identity says anything at all.
    fn is_known(&self) -> bool {
        self.name.is_some() || self.vendor.is_some() || self.product.is_some()
    }
}

/// One numbered place in the pad table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PadSlot {
    /// The connected controller in this slot. `None` while it is unplugged.
    pad: Option<Entity>,
    /// The entity that held this slot last. Bevy keeps the entity of a
    /// disconnected gamepad, so the same controller comes back with it.
    last: Option<Entity>,
    /// What the controller that held this slot last was.
    identity: PadIdentity,
}

impl PadSlot {
    fn remembers(&self, pad: Entity, identity: &PadIdentity) -> bool {
        self.last == Some(pad) || (identity.is_known() && self.identity == *identity)
    }

    fn take(&mut self, pad: Entity, identity: PadIdentity) {
        self.pad = Some(pad);
        self.last = Some(pad);
        self.identity = identity;
    }
}

/// The pad table: the one answer to "which controller is pad `n`".
///
/// [`crate::LocalInputSource::Pad`]`(n)` is slot `n` of this table. A slot keeps
/// its number: a disconnect empties the slot and does not move the other
/// controllers, so a seat, a lobby label and a match plan that all name pad `n`
/// name one controller. A controller that comes back takes the slot it had. A
/// new controller takes the lowest empty slot.
///
/// A resource, not a derived sort: the order in which people picked up their
/// controllers cannot be recovered from the world later.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct LocalDeviceOrder {
    slots: Vec<PadSlot>,
}

impl LocalDeviceOrder {
    /// The connected controller in slot `index`, if one is.
    pub fn pad(&self, index: usize) -> Option<Entity> {
        self.slots.get(index).and_then(|slot| slot.pad)
    }

    /// The slot this connected controller holds.
    pub fn slot_of(&self, pad: Entity) -> Option<usize> {
        self.slots.iter().position(|slot| slot.pad == Some(pad))
    }

    /// The connected controllers, in slot order.
    pub fn connected(&self) -> Vec<Entity> {
        self.slots.iter().filter_map(|slot| slot.pad).collect()
    }

    /// The slots that hold a connected controller, in slot order.
    pub fn connected_slots(&self) -> impl Iterator<Item = usize> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.pad.is_some())
            .map(|(index, _)| index)
    }

    /// The number of slots up to and including the last connected controller.
    /// An empty slot below a connected controller is counted, because the
    /// controller above it keeps its number.
    pub fn span(&self) -> usize {
        self.connected_slots().last().map_or(0, |index| index + 1)
    }

    /// Whether the controller now in slot `index` is the controller this table
    /// has (or had last) in the same slot. A session that froze this table
    /// asks it of the live table, so a different controller in the slot of a
    /// disconnected one is not heard.
    pub fn still_held(&self, live: &LocalDeviceOrder, index: usize) -> Option<Entity> {
        let frozen = self.slots.get(index)?;
        let now = live.slots.get(index)?;
        let pad = now.pad?;
        frozen.remembers(pad, &now.identity).then_some(pad)
    }

    /// Build a table from a known device list, for a caller that already holds
    /// the devices and for tests. Only [`Self::track`] discovers devices.
    pub fn from_devices(devices: Vec<Entity>) -> Self {
        Self {
            slots: devices
                .into_iter()
                .map(|pad| PadSlot {
                    pad: Some(pad),
                    last: Some(pad),
                    identity: PadIdentity::default(),
                })
                .collect(),
        }
    }

    /// Bring the table in step with the controllers that are connected now.
    /// `live` must be in a stable order (the caller sorts it).
    ///
    /// Returns whether the table changed.
    pub fn track(&mut self, live: &[(Entity, PadIdentity)]) -> bool {
        let before = self.clone();
        for slot in &mut self.slots {
            if slot.pad.is_some_and(|pad| !live.iter().any(|(entity, _)| *entity == pad)) {
                slot.pad = None;
            }
        }
        let mut fresh: Vec<&(Entity, PadIdentity)> = live
            .iter()
            .filter(|(pad, _)| self.slot_of(*pad).is_none())
            .collect();
        // A controller that comes back takes its own slot before a new
        // controller can take any slot: first by its entity, then by what it is.
        let own_slot: [fn(&PadSlot, Entity, &PadIdentity) -> bool; 2] = [
            |slot, pad, _| slot.last == Some(pad),
            |slot, pad, identity| slot.remembers(pad, identity),
        ];
        for is_own in own_slot {
            fresh.retain(|(pad, identity)| {
                let home = self
                    .slots
                    .iter_mut()
                    .find(|slot| slot.pad.is_none() && is_own(slot, *pad, identity));
                match home {
                    Some(slot) => {
                        slot.take(*pad, identity.clone());
                        false
                    }
                    None => true,
                }
            });
        }
        for (pad, identity) in fresh {
            match self.slots.iter_mut().find(|slot| slot.pad.is_none()) {
                Some(slot) => slot.take(*pad, identity.clone()),
                None => {
                    let mut slot = PadSlot::default();
                    slot.take(*pad, identity.clone());
                    self.slots.push(slot);
                }
            }
        }
        *self != before
    }
}

#[cfg(test)]
mod pad_table_tests {
    use super::*;

    fn pad(index: u32) -> Entity {
        Entity::from_raw_u32(index).expect("a valid index")
    }

    fn named(name: &str) -> PadIdentity {
        PadIdentity::new(Some(name.to_string()), None, None)
    }

    fn unnamed() -> PadIdentity {
        PadIdentity::default()
    }

    /// Three controllers, and the middle one disconnects. The third keeps its
    /// number. Poison: compact the table, and pad `c` becomes pad 1.
    #[test]
    fn a_disconnect_leaves_the_other_controllers_in_their_slots() {
        let (a, b, c) = (pad(1), pad(2), pad(3));
        let mut table = LocalDeviceOrder::default();
        table.track(&[(a, unnamed()), (b, unnamed()), (c, unnamed())]);
        assert_eq!([table.pad(0), table.pad(1), table.pad(2)], [Some(a), Some(b), Some(c)]);

        assert!(table.track(&[(a, unnamed()), (c, unnamed())]));
        assert_eq!([table.pad(0), table.pad(1), table.pad(2)], [Some(a), None, Some(c)]);
        assert_eq!(table.span(), 3, "the empty slot below pad c is counted");
        assert_eq!(table.connected(), vec![a, c]);
    }

    /// Bevy keeps the entity of a disconnected gamepad. Two controllers of one
    /// model disconnect, and the second comes back first: it takes its own slot.
    #[test]
    fn a_controller_that_comes_back_with_its_entity_takes_its_own_slot() {
        let (a, b) = (pad(1), pad(2));
        let same_model = named("the same model");
        let mut table = LocalDeviceOrder::default();
        table.track(&[(a, same_model.clone()), (b, same_model.clone())]);
        table.track(&[]);
        assert_eq!(table.span(), 0);

        table.track(&[(b, same_model.clone())]);
        assert_eq!([table.pad(0), table.pad(1)], [None, Some(b)]);
        table.track(&[(a, same_model.clone()), (b, same_model)]);
        assert_eq!([table.pad(0), table.pad(1)], [Some(a), Some(b)]);
    }

    /// A controller that comes back as a new entity is known by what it is.
    #[test]
    fn a_controller_that_comes_back_as_a_new_entity_takes_its_own_slot() {
        let mut table = LocalDeviceOrder::default();
        table.track(&[(pad(1), named("pad a")), (pad(2), named("pad b"))]);
        table.track(&[]);
        table.track(&[(pad(3), named("pad b"))]);
        assert_eq!([table.pad(0), table.pad(1)], [None, Some(pad(3))]);
    }

    /// A new controller takes the lowest empty slot, and a controller that
    /// came back in the same frame keeps its own slot.
    #[test]
    fn a_new_controller_takes_the_lowest_empty_slot() {
        let mut table = LocalDeviceOrder::default();
        table.track(&[(pad(1), named("pad a")), (pad(2), named("pad b"))]);
        table.track(&[]);
        // The new controller sorts first, and pad b comes back with it.
        table.track(&[(pad(3), named("pad x")), (pad(4), named("pad b"))]);
        assert_eq!([table.pad(0), table.pad(1)], [Some(pad(3)), Some(pad(4))]);
    }

    /// A frozen table hears a slot only from the controller it froze there.
    #[test]
    fn a_frozen_table_does_not_hear_a_different_controller_in_a_slot() {
        let mut live = LocalDeviceOrder::default();
        live.track(&[(pad(1), named("pad a")), (pad(2), named("pad b"))]);
        let frozen = live.clone();
        assert_eq!(frozen.still_held(&live, 0), Some(pad(1)));

        live.track(&[(pad(2), named("pad b"))]);
        assert_eq!(frozen.still_held(&live, 0), None, "unplugged");

        live.track(&[(pad(2), named("pad b")), (pad(5), named("pad x"))]);
        assert_eq!(live.pad(0), Some(pad(5)), "premise: the new pad is in slot 0");
        assert_eq!(
            frozen.still_held(&live, 0),
            None,
            "a different controller took the seat of the disconnected one"
        );

        live.track(&[(pad(2), named("pad b"))]);
        live.track(&[(pad(2), named("pad b")), (pad(6), named("pad a"))]);
        assert_eq!(frozen.still_held(&live, 0), Some(pad(6)), "it came back");
    }

    #[test]
    fn tracking_the_same_controllers_changes_nothing() {
        let mut table = LocalDeviceOrder::default();
        assert!(table.track(&[(pad(1), unnamed())]));
        assert!(!table.track(&[(pad(1), unnamed())]));
    }
}
