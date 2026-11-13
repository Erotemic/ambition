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

use ambition_characters::actor::BodyWallet;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_combat::components::FeatureId;
use ambition_combat::death_rules::{ActorDiedMessage, DeathFee, OutOfPlay};
use ambition_platformer2d_core::Vec2;
use ambition_platformer2d_items::pickups::{Collected, PickupFeature};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, RoomInstanceRoot, SessionSpawnScope,
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
    mut deaths: MessageReader<ActorDiedMessage>,
    fees: ambition_platformer2d_world::governing_rules::RulesOf<DeathFee>,
    specs: LiveRoomSpecs,
    players: Query<(&DrivingParticipant, Option<&PlayerSafetyState>), (With<PlayerEntity>, With<BodyWallet>)>,
    mut ledger: ResMut<DeathFees>,
) {
    for death in deaths.read() {
        let (Ok((driver, safety)), Some(fee), Some(room)) =
            (players.get(death.victim), fees.of(death.victim), specs.spec_of(death.victim))
        else {
            continue;
        };
        let participant = driver.0;
        ledger.owed.retain(|owed| owed.participant != participant);
        ledger.owed.push(OwedFee {
            participant,
            room: room.id.clone(),
            pos: safety.map_or(death.pos, |safety| safety.last_safe_pos),
            coins: fee.coins,
        });
    }
}

/// Take each owed fee once its body is back in play. The coins taken become
/// the participant's parcel, in place of any parcel it had.
pub fn take_death_fees(
    mut players: Query<(&DrivingParticipant, &mut BodyWallet, Has<OutOfPlay>), With<PlayerEntity>>,
    mut ledger: ResMut<DeathFees>,
) {
    if ledger.owed.is_empty() {
        return;
    }
    let owed = std::mem::take(&mut ledger.owed);
    for fee in owed {
        let body = players.iter_mut().find(|(driver, ..)| driver.0 == fee.participant);
        let Some((_, mut wallet, out_of_play)) = body else {
            // Its participant left the session: nobody owes it.
            continue;
        };
        if out_of_play {
            ledger.owed.push(fee);
            continue;
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
            });
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
) {
    let mut present = Vec::new();
    for (entity, id, collected) in &pickups {
        let Some(serial) = parcel_serial(id) else { continue };
        let owed = ledger.parcels.iter().any(|parcel| parcel.serial == serial);
        if collected {
            if owed {
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
        );
    }
}
