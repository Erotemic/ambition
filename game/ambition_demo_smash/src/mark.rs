//! The delayed mark: a clock riding on the body that was hit.
//!
//! ⭐⭐ NOTHING HERE IS COMBAT BEHAVIOUR, which is the campaign's thesis for the
//! sixth time. `OnHitEffectMessage` owns "who was struck by what"; `BodyKinematics`
//! owns where that body is; `DamageBoxEffect` owns the blast. This module
//! contributes ONE CLOCK and ONE DECISION — when the clock runs out, ask for a
//! blast where the marked body currently is. That is the whole of it.
//!
//! ⛔ THE MARK RIDES THE BODY. A mine asks the opponent to avoid a SPOT; a mark
//! travels with them, so running away does not help and the pressure is on the
//! clock. That is the only reason this is a different technique rather than a
//! re-tuned mine.
//!
//! ⚠ AND A SECOND MARK REFRESHES RATHER THAN STACKS, which the authored params
//! say too: stacking turns one read ("how long have I got") into arithmetic the
//! player cannot do mid-match, and the read is what the move sells.
//!
//! ⛔⛔ THREE AUTHORITIES, KEPT APART, and the first version collapsed them. A
//! blast answers three different questions and the resolver keeps three fields
//! for them: WHO IS CREDITED (`hitbox.owner` — kills, grudges, staleness, rage),
//! WHERE IT IS (the marked body's position at the moment the clock runs out) and
//! WHO IT MAY HURT (`HitSide::Environment` — everybody, the planter included).
//! Crediting the marked body made the victim their own attacker: a bystander
//! KO'd by the blast was credited to the fighter who was marked, not the one
//! who marked them. The credit now rides the mark as the attacker's SEAT, the
//! way the mine and the bolt carry theirs, and is resolved to a body when the
//! blast materialises. A GPT review found it 2026-09-07.
//!
//! ⛔⛔ AND THE MARK LIVES EXACTLY AS LONG AS THE STOCK IT WAS PUT ON. It is
//! rollback state on the persistent fighter entity, and nothing retired it when
//! a stock was spent — so with a 1.4s fuse and a 1.0s death interlude, a mark
//! kept ticking through the interlude and went off on the victim's NEXT stock.
//! A fighter coming back comes back FRESH (`spend_fighter_stocks` resets the
//! meter for the same reason); a note left on the last life does not survive to
//! the next. Retired from any body that is `OutOfPlay`, on whichever road put it
//! there. Same review.

use bevy::prelude::*;

use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::entity_catalog::smash_mark::{MarkBodyParams, MARK_BODY};
use ambition_platformer2d::combat::death_rules::OutOfPlay;

/// A live mark on one body.
///
/// ⛔ ROLLBACK-REGISTERED, and it must be: this is sim state that decides whether
/// a hitbox appears, so a rewind that lost it would drop a detonation the peer
/// still expects — `PlacedMine` carries the same registration for the same
/// reason. The probe is the clock, because the clock is what a desync would
/// disagree about first.
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
    /// or whoever REFRESHED it last.
    ///
    /// ⛔ A SEAT, NOT AN `Entity`, for the reason the mine's and the bolt's owners
    /// are seats: `MatchSeat` is rollback-registered and needs no entity remap,
    /// and `match_participants`' own doc says the seat is how this codebase names
    /// a fighter durably.
    ///
    /// ⭐ THE LAST STRIKER OWNS THE BLAST, stated rather than inherited. A refresh
    /// is a NEW mark whose clock happens to replace an old one — the pressure the
    /// victim is now under is the refresher's, so the credit is too. The other
    /// reading (the first marker keeps the credit through refreshes) would let a
    /// fighter farm a kill off a teammate's follow-ups.
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
/// ⛔ ROLLBACK STATE, like the mark that spawned it: a rewind across the
/// detonation must put the stand-in back or the resimulated blast credits
/// nobody.
///
/// ⚠ The ENTITY is what matters, not a label on it. A `SeatCredit(seat)` rode
/// beside this until v178 and was removed for having no reader: attribution runs
/// on `Entity` everywhere, so what the blast needs is a valid non-victim owner
/// that carries no `MatchSeat`. Both of those are properties of the entity
/// itself. This component is the ruleset's lifetime bookkeeping for it.
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
/// ⛔ NOT A PRESENCE PROBE. A mark whose PRESENCE survived a rewind while its
/// clock did not would detonate on a different frame on the two peers, which is
/// exactly the desync a presence-only probe cannot see. The seat is folded in
/// because two peers agreeing on WHEN and disagreeing on WHO is a kill credited
/// to different fighters on the two screens.
pub fn body_mark_probe(mark: &BodyMark) -> u64 {
    let clock = (mark.fuse_s * 1000.0).round().max(0.0) as u64;
    clock ^ ((mark.attacker_seat as u64) << 48)
}

/// Attach or refresh a mark on every body struck by a move that authors one.
///
/// ⛔ NO SEAT, NO MARK, and an error rather than a silent one — the mine's own
/// ruling. A mark whose attacker cannot be named would have to credit somebody
/// else for its blast, which is the defect this module was corrected for.
/// Outside a match there are no seats and no marks.
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
            // An unhydratable payload is an AUTHORING error, not a runtime one:
            // the key matched, so somebody meant this. Left loud rather than
            // silent for the same reason the mine's is.
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
/// ⛔⛔ RUNS BEFORE `apply_authored_body_marks`, AND THE FIRST VERSION RAN AFTER
/// IT. Apply-then-tick spent one `sim_dt` of every fresh mark on the tick it was
/// attached: a fuse of exactly one tick detonated on the tick it was applied,
/// and an N-tick fuse gave the victim N-1 ticks to act. The comment beside the
/// old order claimed the opposite. Tick-then-apply makes the tick of application
/// tick ZERO of the authored fuse — the same interval ownership the dilation and
/// the tether reel settled. A GPT review found it 2026-09-07.
///
/// ⭐ ONE SYSTEM DECIDES HOW A MARK ENDS — retired or detonated — for the reason
/// `expire_time_dilations` gives: two systems racing over one clock is how a
/// body ends up on somebody else's timeline. A body that is `OutOfPlay` has the
/// world's hands off it, so its mark is retired here without a blast, on the
/// tick after whichever road put it out.
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
    // The stand-ins from earlier detonations run out here, in the one system
    // that decides how a mark ends, so their clock and the mark's are one clock.
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
        // ⛔⛔ WHO IS CREDITED IS NOT WHO WAS HIT. This passed the marked body as
        // the blast's owner, on the argument that "the thing that goes off is
        // what exploded" — which is a statement about GEOMETRY, and the owner
        // field is not geometry. Resolved by seat at the moment the blast
        // materialises, exactly as the bolt resolves its caster.
        //
        // ⛔⛔ AND THE CREDIT OUTLIVES THE BODY. The first version fell back
        // to the marked body when the attacker's seat had no live fighter -- a
        // fighter eliminated inside the 1.4s fuse in a three-way match -- which
        // re-created the defect one case over: a bystander KO'd by the blast was
        // credited to the victim. The seat is the credit; the body is an
        // OPTIONAL live source. With no body, a stand-in entity is the blast's
        // owner for its lifetime, carries no `MatchSeat` so the match still
        // counts two participants, and has no rage or staleness to read,
        // which is the honest value for a fighter who is out. Never the victim.
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
                    // ⛔ `Environment`, the ruling the mine had to be CORRECTED
                    // to: its comment records that `Neutral` reads as "hurts
                    // everybody" and the resolver does the opposite — `melee_source`
                    // excludes Neutral from the body path, so a Neutral blast
                    // damages nobody and only a test asking about the REQUEST
                    // would call that working.
                    // ⭐ AND IT IS WHAT LETS THE OWNER BE THE ATTACKER: the
                    // resolver's self-immunity rule is waived for `Environment`,
                    // so the planter standing next to their own note still
                    // catches it, which is the counterplay.
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
/// ⛔⛔ THE FUSE IS A DECISION AND THEREFORE MUST BE OBSERVABLE — and until
/// 2026-09-07 it was observable only to the test that said so. No overlay, HUD,
/// sprite or VFX consumed `BodyMark`: the victim got a hidden 1.4s timer and then
/// an explosion, which is a hit arriving late, not pressure. This is the sim's
/// half of the read: a row in the generic body-clock view, in the same sim-tail
/// phase every other read-model is rebuilt in. The render half draws it above
/// the body as a shrinking bar and knows nothing about marks.
///
/// Ordered after the view's own clearer, because the view is generic and this
/// is one contributor to it.
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
