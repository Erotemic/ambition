//! The delayed mark: a clock riding on the body that was hit.
//!
//! No combat behaviour lives here. `OnHitEffectMessage` owns who was struck by
//! what; `BodyKinematics` owns where that body is; `DamageBoxEffect` owns the
//! blast. This module adds one clock and one decision: when the clock runs out,
//! ask for a blast where the marked body is.
//!
//! The mark rides the body: a mine asks the opponent to avoid a spot; a mark
//! travels with them, so the pressure is on the clock. A second mark refreshes
//! and does not stack, so the player can still read "how long have I got".
//!
//! Three questions stay separate: who is credited (`hitbox.owner`: kills,
//! grudges, staleness, rage), where the blast is (the marked body's position
//! when the clock runs out), and whom it may hurt (`HitSide::Environment`:
//! everybody, the planter included). The credit rides the mark as the
//! attacker's seat, as for the mine and the bolt, and is resolved to a body
//! when the blast appears. The marked body is never the credited owner.
//!
//! The mark lives as long as the stock it was put on. It is retired from any
//! body that is `OutOfPlay`, so it cannot go off through the death interlude
//! on the next stock (as `spend_fighter_stocks` resets the meter).

use bevy::prelude::*;

use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::entity_catalog::smash_mark::{MarkBodyParams, MARK_BODY};
use ambition_platformer2d::combat::death_rules::OutOfPlay;

/// A live mark on one body.
///
/// Rollback-registered: it decides whether a hitbox appears, as for
/// `PlacedMine`. The probe is the clock, which a desync disagrees on first.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BodyMark {
    /// Seconds remaining before this mark goes off.
    pub fuse_s: f32,
    /// The fuse as authored, so a reader can say how far along the clock is.
    /// Constant for the mark's life; the telegraph divides by it.
    pub fuse_total_s: f32,
    pub damage: i32,
    pub blast_radius: f32,
    pub knockback: f32,
    /// The seat credited with the detonation: whoever landed the marking strike,
    /// or whoever refreshed it last.
    ///
    /// A seat, not an `Entity`, as for the mine and bolt owners: `MatchSeat` is
    /// rollback-registered and names a fighter durably.
    ///
    /// The last striker owns the blast: a refresh is a new mark, so the
    /// pressure and the credit are the refresher's. Otherwise a fighter could
    /// farm a kill off a teammate's follow-ups.
    pub attacker_seat: usize,
}

/// How long a credit stand-in outlives the detonation it was spawned for.
///
/// The blast's own lifetime is 0.08s; this is comfortably past it, so every
/// contact the blast makes can still resolve the attacker to a seat. Ticked on
/// the sim clock by the same system that spawns it.
pub const SEAT_CREDIT_STAND_IN_S: f32 = 0.25;

/// A seat's credit, standing in for a body that has left the match.
///
/// Rollback state, like the mark that spawned it: a rewind across the
/// detonation must restore it, or the blast credits nobody.
///
/// The entity is what matters: attribution runs on `Entity`, so the blast needs
/// a valid non-victim owner with no `MatchSeat`. This component tracks its
/// lifetime.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SeatCreditStandIn {
    pub remaining_s: f32,
}

/// Value projection for the rollback checksum: the clock.
pub fn seat_credit_stand_in_probe(stand_in: &SeatCreditStandIn) -> u64 {
    (stand_in.remaining_s * 1000.0).round().max(0.0) as u64
}

impl BodyMark {
    /// How much of the authored fuse is still to run, `1.0` fresh down to `0.0`.
    pub fn remaining_fraction(&self) -> f32 {
        if self.fuse_total_s <= 0.0 {
            return 0.0;
        }
        (self.fuse_s / self.fuse_total_s).clamp(0.0, 1.0)
    }
}

/// Value projection for the rollback checksum: the clock, in milliseconds, and
/// the seat that will be credited.
///
/// Not a presence probe: a mark whose clock was lost would detonate on
/// different frames on two peers. The seat is included so peers cannot credit
/// different fighters.
pub fn body_mark_probe(mark: &BodyMark) -> u64 {
    let clock = (mark.fuse_s * 1000.0).round().max(0.0) as u64;
    clock ^ ((mark.attacker_seat as u64) << 48)
}

/// Attach or refresh a mark on every body struck by a move that authors one.
///
/// No seat, no mark, and an error, not silence, as for the mine: a mark with
/// no nameable attacker would credit somebody else. Outside a match there are
/// no seats and no marks.
pub fn apply_authored_body_marks(
    mut commands: Commands,
    mut hits: MessageReader<ambition_platformer2d::combat::on_hit::OnHitEffectMessage>,
    seats: Query<&MatchSeat>,
) {
    for hit in hits.read() {
        if hit.effect.key != MARK_BODY {
            continue;
        }
        let Ok(params) = hit.effect.params.hydrate::<MarkBodyParams>() else {
            // An unhydratable payload is an authoring error: the key matched,
            // so somebody meant this. Loud, like the mine's.
            warn!(
                target: "ambition::moves",
                "a `{MARK_BODY}` payload did not hydrate; the mark was not applied",
            );
            continue;
        };
        let Ok(seat) = seats.get(hit.owner) else {
            error!(
                target: "ambition::moves",
                "a mark was landed by {:?}, which has no MatchSeat — nobody could \
                 be credited for its blast, so it is not applied at all",
                hit.owner
            );
            continue;
        };
        if let Ok(mut victim) = commands.get_entity(hit.victim) {
            victim.insert(BodyMark {
                fuse_s: params.fuse_s,
                fuse_total_s: params.fuse_s,
                damage: params.damage,
                blast_radius: params.blast_radius,
                knockback: params.knockback,
                attacker_seat: seat.0,
            });
        }
    }
}

/// Tick every live mark, retire the ones whose body has left play, and
/// detonate the ones that reach zero.
///
/// Runs before `apply_authored_body_marks`, so the tick that applies a mark
/// spends none of its fuse (as for the dilation and the tether reel). The
/// other order gives an N-tick fuse N-1 ticks.
///
/// One system decides how a mark ends, retired or detonated, as in
/// `expire_time_dilations`. A body that is `OutOfPlay` has its mark retired
/// here with no blast.
pub fn detonate_body_marks(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    mut marked: Query<(
        Entity,
        &mut BodyMark,
        &ambition_platformer2d::engine_core::BodyKinematics,
        Has<OutOfPlay>,
    )>,
    seats: Query<(Entity, &MatchSeat)>,
    mut stand_ins: Query<(Entity, &mut SeatCreditStandIn)>,
    active_match: Option<Res<ambition_platformer2d::versus_match::ActiveMatch>>,
) {
    let dt = time.sim_dt();
    // Stand-ins from earlier detonations expire here, in the system that ends
    // marks, so both use one clock.
    for (stand_in, mut credit) in &mut stand_ins {
        credit.remaining_s -= dt;
        if credit.remaining_s <= 0.0 {
            commands.entity(stand_in).despawn();
        }
    }
    for (entity, mut mark, kin, out_of_play) in &mut marked {
        if out_of_play {
            info!(
                target: "ambition::moves",
                "mark retired: {entity:?} left play with {:.2}s on the clock",
                mark.fuse_s,
            );
            commands.entity(entity).remove::<BodyMark>();
            continue;
        }
        mark.fuse_s -= dt;
        if mark.fuse_s > 0.0 {
            continue;
        }
        // Who is credited is not who was hit. Resolved by seat when the blast
        // appears, as the bolt resolves its caster.
        //
        // If the attacker's seat has no live fighter (eliminated during the
        // fuse), a stand-in entity owns the blast for its lifetime. It carries
        // no `MatchSeat`, so the match still counts its real participants, and
        // it has no rage or staleness. Never the victim.
        let owner = seats
            .iter()
            .find(|(_, seat)| seat.0 == mark.attacker_seat)
            .map(|(attacker, _)| attacker)
            .unwrap_or_else(|| {
                info!(
                    target: "ambition::moves",
                    "mark detonation: seat {} has left the match; its credit stands in",
                    mark.attacker_seat,
                );
                let stand_in = commands
                    .spawn((
                        SeatCreditStandIn {
                            remaining_s: SEAT_CREDIT_STAND_IN_S,
                        },
                        Name::new("Seat credit stand-in"),
                    ))
                    .id();
                crate::match_scope::stamp(&mut commands, stand_in, active_match.as_deref());
                stand_in
            });
        effects.write(ambition_platformer2d::vfx::EffectRequest {
            owner,
            effect: ambition_platformer2d::vfx::Effect::DamageBox(
                ambition_platformer2d::vfx::DamageBoxEffect {
                    center: kin.pos,
                    // `Environment`, not `Neutral`: `melee_source` excludes
                    // Neutral from the body path, so a Neutral blast damages
                    // nobody. `Environment` also waives self-immunity, so the
                    // planter next to their own mark is caught (the counterplay).
                    faction: ambition_platformer2d::vfx::HitSide::Environment,
                    half_extent: ambition_platformer2d::engine_core::Vec2::splat(mark.blast_radius),
                    damage: mark.damage,
                    knockback: mark.knockback,
                    lifetime_s: 0.08,
                    name: Some("mark detonation"),
                },
            ),
        });
        commands.entity(entity).remove::<BodyMark>();
    }
}

/// Publish every live mark as a clock the player can read.
///
/// The fuse is a decision, so the player must see it. This is the sim half: a
/// row in the generic body-clock view, rebuilt in the sim-tail phase. The
/// render half draws a shrinking bar above the body and knows nothing about
/// marks. Ordered after the view's own clearer.
pub fn publish_mark_clocks(
    mut view: ResMut<ambition_platformer2d::sim_view::BodyClocksView>,
    marked: Query<(
        Entity,
        &BodyMark,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>,
) {
    for (body, mark, kin) in &marked {
        view.0.push(ambition_platformer2d::sim_view::BodyClockFact {
            body,
            pos: kin.pos,
            half_height: kin.size.y * 0.5,
            remaining_fraction: mark.remaining_fraction(),
        });
    }
}

#[cfg(test)]
mod tests;
