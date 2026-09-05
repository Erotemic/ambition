//! The remote mine: a stage object that answers to one fighter and nobody else.
//!
//! ⭐⭐ NOTHING HERE IS OBJECT BEHAVIOUR. `GroundItem` owns sitting on the
//! stage, falling, being picked up and being thrown; `ItemWorldPos` owns "where
//! is this thing, whoever has it"; `DamageBoxEffect` owns the blast. This module
//! contributes one clock and one decision. That is the whole of it, and it is
//! the campaign's thesis again: the authorities were already here.
//!
//! ⛔⛔ THE OWNER IS A SEAT, NOT AN `Entity`. `MatchSeat` is rollback-registered
//! (`actor.match_seat`), survives a rewind unchanged, and needs no entity
//! remapping — and `match_participants`' own doc says the seat is how this
//! codebase names a fighter durably, because "no resource stores live entity
//! handles for the cast". An `Entity` here would have cost a `MapEntities` impl
//! to answer a question a `usize` comparison answers.
//!
//! ⚠ THE CONSEQUENCE IS THAT A MINE NEEDS A SEATED PLACER, and outside a match
//! there are no seats. That is stated rather than papered over: see
//! `place_or_detonate_authored_mines`.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_mine::{PlaceMineParams, PLACE_MINE};
use ambition_platformer2d::engine_core as ae;

/// A mine somebody placed, and the two facts that are not the object's own.
///
/// ⛔ ROLLBACK STATE. The arming clock outlives the tick that made it, so a
/// rewind that put the mine back without putting its clock back would give the
/// resimulated timeline a mine that answers a press the confirmed timeline
/// ignored — which is a blast on one peer and not the other.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlacedMine {
    /// Which seat may set this off. ⛔ Not an `Entity`: see the module note.
    pub owner_seat: usize,
    /// Seconds until it will answer its owner. Inert until this reaches zero.
    pub arm_s: f32,
    /// Damage at the centre of the blast.
    pub damage: i32,
    /// How far the blast reaches.
    pub blast_radius: f32,
}

impl PlacedMine {
    /// Will this mine answer a press right now?
    pub fn armed(&self) -> bool {
        self.arm_s <= 0.0
    }
}

/// Checksum probe: the clock is the part a peer can disagree about.
///
/// ⛔ THE CLOCK AND NOT THE NUMBERS BESIDE IT, for the reason `live_bomb_probe`
/// gives: damage, radius and seat are constants copied off the move or off a
/// component rollback already restores, and hashing them would make every mine
/// of the same kind indistinguishable in the probe while the one field that
/// actually moves went unwatched.
pub fn placed_mine_probe(mine: &PlacedMine) -> u64 {
    mine.arm_s.to_bits() as u64
}

/// Spend the arming delay.
pub fn arm_placed_mines(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut mines: Query<&mut PlacedMine>,
) {
    let dt = time.sim_dt();
    for mut mine in &mut mines {
        if mine.arm_s > 0.0 {
            // ⭐ A CARRIED MINE STILL ARMS, like a carried bomb still burns.
            // Custody is not consulted here at all: taking somebody's mine does
            // not make it yours, and it does not stop it becoming live.
            mine.arm_s = (mine.arm_s - dt).max(0.0);
        }
    }
}

/// Place a mine, or set off the one already out. One press, two outcomes.
///
/// ⛔⛔ ONE SYSTEM FOR BOTH, because they are ONE DECISION with one input — the
/// same reasoning `burn_fuses_and_answer_impacts` gives for keeping the fuse and
/// the impact together. Two systems, one placing and one detonating, would both
/// read the same press and the ordering between them would decide whether a
/// fighter ends the frame with two mines or none.
///
/// ⭐ THE RULE IS ONE MINE PER SEAT, and it is what makes the arming delay a
/// brake rather than a decoration. A press while your mine is still arming does
/// nothing to the mine: the move plays, the recovery is spent, and that is the
/// price of mashing. A press with no mine out places one; a press with an armed
/// mine out sets it off from wherever it is and whoever is holding it.
pub fn place_or_detonate_authored_mines(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    placers: Query<(&ae::BodyKinematics, &ambition_platformer2d::actor::MatchSeat)>,
    mines: Query<(
        Entity,
        &PlacedMine,
        &ambition_platformer2d::item::GroundItem,
        &ambition_platformer2d::item::ItemCustody,
    )>,
    where_it_is: ambition_platformer2d::item::ItemWorldPos,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != PLACE_MINE {
            continue;
        }
        let params = match params.hydrate::<PlaceMineParams>() {
            Ok(params) => params,
            Err(err) => {
                warn!("place-mine params did not hydrate: {err}");
                continue;
            }
        };
        // ⛔ NO SEAT, NO MINE, and an error rather than a silent placement. A
        // mine whose owner cannot be named is one nobody can ever detonate, so
        // placing it would leave permanent furniture on the stage — a worse
        // outcome than the move doing nothing and saying why.
        let Ok((kin, seat)) = placers.get(message.actor) else {
            error!(
                "a mine was placed by {:?}, which has no MatchSeat — nobody \
                 could ever set it off, so it is not placed at all",
                message.actor
            );
            continue;
        };
        let owner_seat = seat.0;

        // Do I already have one out?
        if let Some((entity, mine, item, custody)) =
            mines.iter().find(|(_, mine, _, _)| mine.owner_seat == owner_seat)
        {
            if !mine.armed() {
                info!(
                    target: "ambition::moves",
                    "mine press ignored: seat {owner_seat}'s mine still arming ({:.2}s left)",
                    mine.arm_s,
                );
                continue;
            }
            let at = where_it_is.of(custody, item);
            info!(
                target: "ambition::moves",
                "mine detonated: seat={owner_seat} at {at:?} damage={}",
                mine.damage,
            );
            effects.write(ambition_platformer2d::vfx::EffectRequest {
                // The MINE is the owner of the effect, like the bomb's blast is
                // the bomb's: the object is what exploded.
                owner: entity,
                effect: ambition_platformer2d::vfx::Effect::DamageBox(
                    ambition_platformer2d::vfx::DamageBoxEffect {
                        center: at,
                        // ⛔ `Neutral`, the same ruling the bomb's blast carries,
                        // and it is load-bearing HERE in a way it is not there.
                        // The mine's owner chooses the instant, so a blast that
                        // could not hurt them would make standing next to your
                        // own mine free — and "get them to stand near it" is the
                        // entire move. A neutral blast is what makes the timing
                        // a decision instead of a formality.
                        faction: ambition_platformer2d::vfx::HitSide::Neutral,
                        half_extent: ae::Vec2::splat(mine.blast_radius),
                        damage: mine.damage,
                        knockback: mine.blast_radius * 2.4,
                        lifetime_s: 0.08,
                        name: Some("mine blast"),
                    },
                ),
            });
            commands.entity(entity).despawn();
            continue;
        }

        let Some(held) = ambition_platformer2d::characters::brain::held_item_by_id(&params.item_id)
        else {
            error!(
                "move places `{}`, which is not a registered held item — nobody \
                 could pick this mine up, so it is not placed at all",
                params.item_id
            );
            continue;
        };
        // Body-local, mirrored by facing, like every other authored offset.
        let at = kin.pos + ae::Vec2::new(params.offset.0 * kin.facing.signum(), params.offset.1);
        info!(
            target: "ambition::moves",
            "mine placed: seat={owner_seat} item=`{}` at {at:?} arm={}s",
            params.item_id, params.arm_s,
        );
        commands.spawn((
            Name::new(format!("Placed mine: {}", params.item_id)),
            ambition_platformer2d::item::GroundItem {
                spec: held,
                pos: at,
                vel: ae::Vec2::ZERO,
                half_extent: ae::Vec2::new(params.half_extents.0, params.half_extents.1),
            },
            PlacedMine {
                owner_seat,
                arm_s: params.arm_s,
                damage: params.damage,
                blast_radius: params.blast_radius,
            },
        ));
    }
}

#[cfg(test)]
mod tests;
