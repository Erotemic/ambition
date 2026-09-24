//! The remote mine: a stage object that answers to one fighter and nobody else.
//!
//! No object behaviour lives here. `GroundItem` owns sitting on the stage,
//! falling, pickup and throw; `ItemWorldPos` owns where it is, whoever has it;
//! `DamageBoxEffect` owns the blast. This module adds one clock and one
//! decision.
//!
//! The owner is a seat, not an `Entity`: `MatchSeat` is rollback-registered
//! (`actor.match_seat`) and needs no entity remapping. So a mine needs a seated
//! placer, and outside a match there are none; see
//! `place_or_detonate_authored_mines`.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_mine::{PlaceMineParams, PLACE_MINE};
use ambition_platformer2d::engine_core as ae;

/// How hard the mine throws what it catches, as the feel multiplier
/// `DamageBoxEffect::knockback` is. Under the bomb's 2.0: a mine is armed in
/// advance and triggers on someone else's mistake.
const BLAST_FEEL_SCALE: f32 = 1.7;

/// A mine somebody placed, and the two facts that are not the object's own.
///
/// Rollback state: a restore without the arming clock could answer a press
/// the confirmed timeline ignored.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlacedMine {
    /// Which seat may set this off. Not an `Entity`; see the module note.
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
/// The clock only, as in `live_bomb_probe`: damage, radius and seat are
/// constants or already restored.
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
            // A carried mine still arms, like a carried bomb still burns.
            // Taking somebody's mine does not make it yours.
            mine.arm_s = (mine.arm_s - dt).max(0.0);
        }
    }
}

/// Place a mine, or set off the one already out. One press, two outcomes.
///
/// One system for both: one press is one decision. Two systems reading the same
/// press could leave a fighter with two mines or none.
///
/// One mine per seat, which makes the arming delay a brake: a press while your
/// mine is arming does nothing to it (the move plays and the recovery is
/// spent). A press with no mine out places one; a press with an armed mine out
/// sets it off, wherever it is and whoever holds it.
pub fn place_or_detonate_authored_mines(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    placers: Query<(
        &ae::BodyKinematics,
        &ambition_platformer2d::actor::MatchSeat,
    )>,
    mines: Query<(
        Entity,
        &PlacedMine,
        &ambition_platformer2d::item::GroundItem,
        &ambition_platformer2d::item::ItemCustody,
    )>,
    where_it_is: ambition_platformer2d::item::ItemWorldPos,
    // The running match, so what this spawns dies with it (see
    // `crate::match_scope`).
    active_match: Option<Res<ambition_platformer2d::versus_match::ActiveMatch>>,
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
        // No seat, no mine, and an error: a mine nobody can detonate would be
        // permanent furniture.
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
        if let Some((entity, mine, item, custody)) = mines
            .iter()
            .find(|(_, mine, _, _)| mine.owner_seat == owner_seat)
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
                // The mine owns the effect, as the bomb owns its blast.
                owner: entity,
                effect: ambition_platformer2d::vfx::Effect::DamageBox(
                    ambition_platformer2d::vfx::DamageBoxEffect {
                        center: at,
                        // `Environment`, not `Neutral` (`melee_source` excludes
                        // Neutral from the body path). It hurts the owner too:
                        // they choose the instant, so standing next to their own
                        // mine must not be free.
                        faction: ambition_platformer2d::vfx::HitSide::Environment,
                        half_extent: ae::Vec2::splat(mine.blast_radius),
                        damage: mine.damage,
                        // A feel multiplier, not a launch speed (see
                        // `spawn_damage_box`: values such as 1.0 or 1.6).
                        knockback: BLAST_FEEL_SCALE,
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
        let spawned = commands
            .spawn((
                Name::new(format!("Placed mine: {}", params.item_id)),
                ambition_platformer2d::item::GroundItem::at_rest(
                    held,
                    at,
                    ae::Vec2::new(params.half_extents.0, params.half_extents.1),
                ),
                PlacedMine {
                    owner_seat,
                    arm_s: params.arm_s,
                    damage: params.damage,
                    blast_radius: params.blast_radius,
                },
            ))
            .id();
        // The match owns this object's end; see `crate::match_scope`.
        crate::match_scope::stamp(&mut commands, spawned, active_match.as_deref());
    }
}

#[cfg(test)]
mod tests;
