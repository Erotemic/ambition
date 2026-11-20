//! What a participant's death costs (PLAYER-DEATH, Q174's default): the gate
//! network's insurance takes its fee ([`DeathFee`]) from the wallet, and the
//! coins it took lie where the body was taken until someone fetches them. A
//! wallet that cannot pay gives all it holds and goes into debt for the rest;
//! the next currency earned pays the debt first (`BodyWallet::charge`).
//!
//! The fee is owed at the death and taken when the body is back in play: a
//! level that goes back puts the wallet back to its checkpoint, and a fee taken
//! before that would be undone by it.
//!
//! [`DeathFees`] is the authority. The pickup in the world is drawn from it:
//! placed while its room is live, placed again after a room goes back, and
//! taken off the ledger when it is collected. Each participant has one parcel;
//! a second death before the first is fetched loses the first.

use bevy::prelude::*;
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;

use ambition_characters::actor::BodyWallet;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_combat::components::FeatureId;
use ambition_combat::death_rules::{ActorDiedMessage, DeathFee, OutOfPlay};
use ambition_platformer2d_core::Vec2;
use ambition_platformer2d_items::pickups::{Collected, PickupFeature};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, LiveRooms, RoomInstanceRoot, RoomWrite, SessionSpawnScope,
};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_platformer2d_world::rooms::{LiveRoomDefinition, LiveRoomSpecs};

const PARCEL_ID_PREFIX: &str = "death_parcel:";

/// A fee owed by a death, taken once its body is back in play.
#[derive(Clone, Debug, PartialEq)]
struct OwedFee {
    participant: PlayerSlot,
    room: String,
    pos: Vec2,
    coins: i32,
    art: Option<String>,
}

/// The coins one death's fee took, lying where the body was taken.
#[derive(Clone, Debug, PartialEq)]
pub struct DeathParcel {
    pub participant: PlayerSlot,
    /// The room it lies in, by its definition id: it outlives the live room.
    pub room: String,
    pub pos: Vec2,
    pub coins: i32,
    /// Names the pickup drawn from it.
    pub serial: u32,
    /// What it is drawn with (`DeathFee::parcel_art`).
    pub art: Option<String>,
}

/// The fees owed and the parcels lying in the world. Rollback state.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct DeathFees {
    owed: Vec<OwedFee>,
    parcels: Vec<DeathParcel>,
    next_serial: u32,
}

impl DeathFees {
    pub fn parcels(&self) -> &[DeathParcel] {
        &self.parcels
    }

    /// The checksum projection: every fee owed and parcel lying, in order.
    pub fn probe(&self) -> u64 {
        let text = format!("{:?}", self);
        text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }
}

/// One participant's fee and parcel as they stood before a room changed them.
type ParticipantFees = (Option<OwedFee>, Option<DeathParcel>);

/// The changes rooms made to [`DeathFees`] within the rewind horizon, each
/// with the participant's fee and parcel as they were before it (ISL-2d). A
/// room replay restores the body's wallet, so a fee or parcel a speculative
/// death left behind would otherwise outlive the death the replay does not
/// repeat, and the coins would be both in the wallet and on the floor.
/// Rollback state with a real value: a write changes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct DeathFeesInRooms {
    writes: Vec<(RoomWrite, PlayerSlot, ParticipantFees)>,
}

impl DeathFeesInRooms {
    /// The checksum projection, as [`DeathFees::probe`].
    pub fn probe(&self) -> u64 {
        let text = format!("{:?}", self);
        text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    /// Note what `participant` has in `ledger` before `written` changes it.
    fn before(&mut self, ledger: &DeathFees, written: Option<RoomWrite>, participant: PlayerSlot) {
        let Some(at) = written else { return };
        self.writes.retain(|(write, ..)| !write.beyond_the_horizon(at.tick));
        self.writes.push((at, participant, ledger.of(participant)));
    }
}

impl DeathFees {
    fn of(&self, participant: PlayerSlot) -> ParticipantFees {
        (
            self.owed.iter().find(|owed| owed.participant == participant).cloned(),
            self.parcels.iter().find(|parcel| parcel.participant == participant).cloned(),
        )
    }

    /// Put `participant`'s fee and parcel back as `fees` says.
    fn restore(&mut self, participant: PlayerSlot, (owed, parcel): ParticipantFees) {
        self.owed.retain(|owed| owed.participant != participant);
        self.owed.extend(owed);
        self.owed.sort_by_key(|owed| owed.participant);
        self.parcels.retain(|parcel| parcel.participant != participant);
        self.parcels.extend(parcel);
        self.parcels.sort_by_key(|parcel| parcel.participant);
    }
}

/// The room retractor: `room` rewinds to `saved_at`, so each participant's
/// fee and parcel go back to what they were before that room's later writes.
pub fn retract_room_death_fees(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    let Some(log) = world.get_resource_mut::<DeathFeesInRooms>() else { return };
    let taken = ambition_platformer2d_shared_tangle::lifecycle::take_rewound(
        log.map_unchanged(|log| &mut log.writes), room, saved_at, |(write, ..)| *write);
    if taken.is_empty() {
        return;
    }
    let Some(mut ledger) = world.get_resource_mut::<DeathFees>() else { return };
    for (_, participant, before) in taken {
        ledger.restore(participant, before);
    }
}

/// The write stamp of a change made from `body`'s live room on this tick.
fn written_from(rooms: &LiveRooms, tick: Option<&ambition_time::SimTick>, body: Entity) -> Option<RoomWrite> {
    rooms.of(body).zip(tick).map(|(room, tick)| RoomWrite { room, tick: tick.0 })
}

fn parcel_feature_id(serial: u32) -> String {
    format!("{PARCEL_ID_PREFIX}{serial}")
}

fn parcel_serial(id: &FeatureId) -> Option<u32> {
    id.0.strip_prefix(PARCEL_ID_PREFIX)?.parse().ok()
}

/// Owe the fee of each participant's death, in its own room's rules, with the
/// room and the place it lies (the last safe ground, so a fall into a pit
/// leaves it where a body can stand).
pub fn owe_death_fees(
    mut deaths: ReplayMessageReader<ActorDiedMessage>,
    fees: ambition_platformer2d_world::governing_rules::RulesOf<DeathFee>,
    specs: LiveRoomSpecs,
    players: Query<(&DrivingParticipant, Option<&PlayerSafetyState>), (With<PlayerEntity>, With<BodyWallet>)>,
    mut ledger: ResMut<DeathFees>,
    mut log: ResMut<DeathFeesInRooms>,
    rooms: LiveRooms,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    for death in deaths.read() {
        let (Ok((driver, safety)), Some(fee), Some(room)) =
            (players.get(death.victim), fees.of(death.victim), specs.spec_of(death.victim))
        else {
            continue;
        };
        let participant = driver.0;
        log.before(&ledger, written_from(&rooms, tick.as_deref(), death.victim), participant);
        ledger.owed.retain(|owed| owed.participant != participant);
        ledger.owed.push(OwedFee {
            participant,
            room: room.id.clone(),
            pos: safety.map_or(death.pos, |safety| safety.last_safe_pos),
            coins: fee.coins,
            art: fee.parcel_art.clone(),
        });
        ledger.owed.sort_by_key(|owed| owed.participant);
    }
}

/// Take each owed fee once its body is back in play. The coins taken become
/// the participant's parcel, in place of any parcel it had.
pub fn take_death_fees(
    mut players: Query<(Entity, &DrivingParticipant, &mut BodyWallet, Has<OutOfPlay>), With<PlayerEntity>>,
    mut ledger: ResMut<DeathFees>,
    mut log: ResMut<DeathFeesInRooms>,
    rooms: LiveRooms,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    if ledger.owed.is_empty() {
        return;
    }
    let owed = std::mem::take(&mut ledger.owed);
    for fee in owed {
        let body = players.iter_mut().find(|(_, driver, ..)| driver.0 == fee.participant);
        let Some((entity, _, mut wallet, out_of_play)) = body else {
            // Its participant left the session: nobody owes it.
            continue;
        };
        if out_of_play {
            ledger.owed.push(fee);
            continue;
        }
        // What this participant had before this take, with the fee still
        // owed: a rewind of the body's room puts the fee back to be taken
        // again from the wallet the same rewind restores.
        let mut before = ledger.of(fee.participant);
        before.0 = Some(fee.clone());
        if let Some(at) = written_from(&rooms, tick.as_deref(), entity) {
            log.writes.retain(|(write, ..)| !write.beyond_the_horizon(at.tick));
            log.writes.push((at, fee.participant, before));
        }
        let paid = wallet.charge(fee.coins);
        ledger.parcels.retain(|parcel| parcel.participant != fee.participant);
        if paid > 0 {
            let serial = ledger.next_serial;
            ledger.next_serial = ledger.next_serial.wrapping_add(1);
            ledger.parcels.push(DeathParcel {
                participant: fee.participant,
                room: fee.room,
                pos: fee.pos,
                coins: paid,
                serial,
                art: fee.art,
            });
            ledger.parcels.sort_by_key(|parcel| parcel.participant);
        }
    }
}

/// Keep the world's parcel pickups equal to the ledger: a collected one comes
/// off the ledger, one the ledger no longer has goes, and a parcel whose room
/// is live and has none is placed.
pub fn place_death_parcels(
    mut commands: Commands,
    mut ledger: ResMut<DeathFees>,
    pickups: Query<(Entity, &FeatureId, Has<Collected>), With<PickupFeature>>,
    roots: Query<(&LiveRoomInstance, &LiveRoomDefinition), With<RoomInstanceRoot>>,
    specs: LiveRoomSpecs,
    active_session: Option<Res<ActiveSessionScope>>,
    mut log: ResMut<DeathFeesInRooms>,
    rooms: LiveRooms,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let mut present = Vec::new();
    for (entity, id, collected) in &pickups {
        let Some(serial) = parcel_serial(id) else { continue };
        let owed = ledger.parcels.iter().any(|parcel| parcel.serial == serial);
        if collected {
            if let Some(participant) = ledger.parcels.iter().find(|parcel| parcel.serial == serial).map(|parcel| parcel.participant) {
                log.before(&ledger, written_from(&rooms, tick.as_deref(), entity), participant);
                ledger.parcels.retain(|parcel| parcel.serial != serial);
            }
        } else if owed {
            present.push(serial);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if ledger.parcels.iter().all(|parcel| present.contains(&parcel.serial)) {
        return;
    }
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    for parcel in ledger.parcels.iter().filter(|parcel| !present.contains(&parcel.serial)) {
        let live = roots
            .iter()
            .find(|(_, definition)| specs.rooms().spec(**definition).id == parcel.room)
            .map(|(live, _)| *live);
        let Some(live) = live else { continue };
        let feature_id = parcel_feature_id(parcel.serial);
        ambition_platformer2d_items::loot::drop_death_parcel(
            &mut commands,
            scope.in_room(Some(live)),
            &SimId::singleton("death_parcel", &parcel.serial.to_string()),
            &feature_id,
            parcel.pos,
            parcel.coins,
            parcel.art.as_deref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fee(participant: u8, coins: i32) -> OwedFee {
        OwedFee { participant: PlayerSlot(participant), room: "hall".into(), pos: Vec2::ZERO, coins, art: None }
    }

    /// A rewound room puts back the fee changes it made after the tick it was
    /// saved at, and only its own: the other room's participant keeps its
    /// parcel. Without the log a speculative death's parcel outlives the
    /// rewind that restores the wallet it was taken from.
    #[test]
    fn a_rewound_room_puts_back_its_own_death_fees() {
        let first = LiveRoomInstance::ACTIVATION.next();
        let second = first.next();
        let mut ledger = DeathFees::default();
        let mut log = DeathFeesInRooms::default();
        // Alice (seat 0) in the first room dies at tick 12; Bob (seat 1) in
        // the second room has a parcel from tick 12 too.
        log.before(&ledger, Some(RoomWrite { room: first, tick: 12 }), PlayerSlot(0));
        ledger.owed.push(fee(0, 10));
        log.before(&ledger, Some(RoomWrite { room: second, tick: 12 }), PlayerSlot(1));
        ledger.parcels.push(DeathParcel { participant: PlayerSlot(1), room: "yard".into(), pos: Vec2::ZERO, coins: 5, serial: 0, art: None });
        let mut world = World::new();
        world.insert_resource(ledger);
        world.insert_resource(log);
        retract_room_death_fees(&mut world, first, 10);
        let ledger = world.resource::<DeathFees>();
        assert!(ledger.owed.is_empty(), "the rewound room's fee is taken back: {ledger:?}");
        assert_eq!(ledger.parcels.len(), 1, "the other room's parcel stays: {ledger:?}");
    }
}
