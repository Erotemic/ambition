//! A live bomb on the stage: the ruleset half of `smash.drop_bomb`.
//!
//! ⭐⭐ JON'S DESIGN, 2026-08-27: *"The projectile polygon should poop a bomb
//! onto the stage, they should be able to pick it up and throw it. The bomb
//! should detonate in 4 seconds or if it hits something with enough velocity,
//! whichever comes first."*
//!
//! ⭐⭐ IT IS A GROUND ITEM, AND THAT IS WHY THE SECOND SENTENCE IS FREE. Picking
//! things up and throwing them is machinery this engine already has —
//! `GroundItem`, `ItemCustody`, `pickup_held_item_system`,
//! `throw_held_item_system`, all installed by `ItemPickupSimulationPlugin` in
//! every game including this one. A bomb modelled as a summoned BODY (the road
//! the shark takes) would have needed every one of those written again in order
//! to be a thing you can hold.
//!
//! ⭐ AND THE IMPACT RULE IS THE ONE THE ITEM PHYSICS ALREADY PUBLISHES.
//! `ground_item_physics` stamps `SettledItem` on the tick an item is blocked by
//! geometry — that IS "it hit something" — so the only thing this file has to
//! remember is HOW FAST it was going one tick before, because the same function
//! zeroes the velocity as it settles.
//!
//! ⭐ **ANY DRIVEN BODY CAN TAKE AND THROW ONE, NOT ONLY THE CONTROLLED SUBJECT.**
//! `pickup_held_item_system` and `throw_held_item_system` both take
//! `driven: DrivenBodies` — the union of `ControlledSubject` and every
//! `DrivingParticipant` — and `ItemCustody::Held { holder: Entity }` is keyed by
//! the holding body. ⇒ A CPU seat is eligible on both roads.
//!
//! ⚠ **Eligible is not the same as observed.** Whether the fighter brain ever
//! REQUESTS a pickup is a separate question and is unmeasured; the systems permit
//! it, which is all this states.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_bomb::{DropBombParams, DROP_BOMB};
use ambition_platformer2d::engine_core as ae;

/// A ground item with a fuse.
///
/// ⛔ ROLLBACK STATE. Every field of it counts down or remembers, and a rewind
/// that put the bomb back on the stage without putting its fuse back would give
/// the resimulated timeline a different explosion from the confirmed one.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct LiveBomb {
    /// Seconds left before it goes off by itself.
    pub fuse_s: f32,
    /// Damage at the centre.
    pub damage: i32,
    /// How far the blast reaches.
    pub blast_radius: f32,
    /// At or above this speed, contact IS the detonation.
    pub impact_speed: f32,
}

// ⛔⛔ NO OWNER HANDLE, AND THAT IS A DESIGN RATHER THAN AN OMISSION. A live
// bomb CHANGES HANDS — that is the whole move — so "whose bomb is this" has no
// stable answer, and the blast is attributed to the BOMB. It also keeps an
// `Entity` out of rollback state, which would otherwise need remapping and a
// probe through the target's stable sim identity for a fact nothing reads.

/// Recognise the authored bomb drop and put the object on the stage.
pub fn translate_bomb_drops(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<&ae::BodyKinematics>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != DROP_BOMB {
            continue;
        }
        let params = match params.hydrate::<DropBombParams>() {
            Ok(params) => params,
            Err(err) => {
                warn!("drop-bomb params did not hydrate: {err}");
                continue;
            }
        };
        let Some(held) = ambition_platformer2d::characters::brain::held_item_by_id(&params.item_id)
        else {
            // A warning and not a refusal is wrong HERE, and that is the
            // difference between this and a brandish: an unregistered id means
            // the object cannot be picked up, and a bomb nobody can pick up is
            // half a move pretending to be a whole one.
            error!(
                "move drops `{}`, which is not a registered held item — nobody \
                 could pick this bomb up, so it is not dropped at all",
                params.item_id
            );
            continue;
        };
        let Ok(kin) = bodies.get(message.actor) else {
            continue;
        };
        // Body-local, mirrored by facing, like every other authored offset.
        let at = kin.pos + ae::Vec2::new(params.offset.0 * kin.facing.signum(), params.offset.1);
        let half = ae::Vec2::new(params.half_extents.0, params.half_extents.1);
        info!(
            target: "ambition::moves",
            "bomb dropped: owner={:?} item=`{}` at {at:?} fuse={}s",
            message.actor, params.item_id, params.fuse_s,
        );
        commands.spawn((
            Name::new(format!("Live bomb: {}", params.item_id)),
            ambition_platformer2d::item::GroundItem {
                spec: held,
                pos: at,
                vel: ae::Vec2::ZERO,
                half_extent: half,
            },
            LiveBomb {
                fuse_s: params.fuse_s,
                damage: params.damage,
                blast_radius: params.blast_radius,
                impact_speed: params.impact_speed,
            },
        ));
    }
}

/// Burn the fuse, remember the speed, and go off for whichever reason arrives
/// first.
///
/// ⛔ ONE SYSTEM FOR BOTH REASONS, because they are one decision with two
/// inputs and the bomb may only explode once. Two systems racing to despawn the
/// same entity is the shape that produces a double blast on the tick a thrown
/// bomb's fuse also runs out.
pub fn burn_fuses_and_answer_impacts(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut commands: Commands,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    mut bombs: Query<(
        Entity,
        &mut LiveBomb,
        &ambition_platformer2d::item::GroundItem,
        &ambition_platformer2d::item::ItemCustody,
        Option<&ambition_platformer2d::item::SettledItem>,
        // ⭐ THE OTHER HARD CONTACT, and it is the same question. A bomb that
        // reaches a fighter at speed has hit something with enough velocity —
        // Jon's rule names no surface — and reading only the world settle is
        // what made "impact detonation" mean "touched a block".
        Option<&ambition_platformer2d::item::ItemStruckBody>,
    )>,
    // ⭐⭐ WHERE THE BOMB IS, WHOEVER HAS IT. `GroundItem::pos` is the WORLD's
    // copy and the world stops updating it the moment somebody picks the bomb
    // up — so a carried bomb blew up at the spot it was collected from, however
    // far its holder had run. See `ItemWorldPos`; the semantic is generic
    // because "where is this item" is not a bomb question.
    where_it_is: ambition_platformer2d::item::ItemWorldPos,
) {
    let dt = time.sim_dt();
    for (entity, mut bomb, item, custody, settled, struck_body) in &mut bombs {
        let at = where_it_is.of(custody, item);
        // ⛔ A CARRIED BOMB STILL BURNS. That is the whole tension of holding
        // one, and it is why the fuse is ticked before the custody check rather
        // than after it.
        bomb.fuse_s -= dt;
        // ⭐ IMPACT: geometry stopped it, and it was travelling hard enough for
        // that to count. The SPEED IS THE SETTLE'S OWN, published by
        // `ground_item_physics` on the tick it zeroed the velocity — this used
        // to remember last tick's speed instead, which is a different number in
        // the two cases that matter. A bomb thrown hard at a near wall collides
        // on its FIRST free tick, when the remembered speed is still the zero it
        // had in a hand; and a falling bomb can cross the threshold on the
        // gravity of the very tick it lands.
        // ⛔ ONE THRESHOLD, TWO SURFACES. The bomb sets the bar for "hard" and
        // the collision authority says what was reached; a second number for
        // bodies would be a second policy nobody asked for.
        let hardest = settled
            .map(|settled| settled.impact_speed)
            .into_iter()
            .chain(struck_body.map(|hit| hit.impact_speed))
            .fold(0.0_f32, f32::max);
        let struck_hard = custody.in_world() && hardest >= bomb.impact_speed;
        if bomb.fuse_s > 0.0 && !struck_hard {
            continue;
        }
        info!(
            target: "ambition::moves",
            "bomb detonates: entity={entity:?} reason={} at {:?}",
            if struck_hard { "impact" } else { "fuse" },
            at,
        );
        effects.write(ambition_platformer2d::vfx::EffectRequest {
            // The BOMB is the owner: see the note on `LiveBomb`.
            owner: entity,
            effect: ambition_platformer2d::vfx::Effect::DamageBox(
                ambition_platformer2d::vfx::DamageBoxEffect {
                    center: at,
                    // ⛔ `Neutral`, NOT the thrower's side. `damage_lands` is
                    // true for `Foe | Neutral`, so a neutral blast hurts
                    // everybody standing in it — including whoever threw it,
                    // which is what makes a live bomb a thing you respect
                    // rather than a free projectile.
                    // ⛔⛔ `Environment`, NOT `Neutral`. This read `Neutral` with a comment
                            // saying Neutral hurts everybody; the resolver says the exact opposite
                            // — `melee_source` excludes it from the body path and its terminal arm
                            // is empty, with the contract that Neutral never spawns a damaging
                            // hitbox. ⇒ This blast damaged NOBODY, and the test only asked whether
                            // the effect request existed.
                            faction: ambition_platformer2d::vfx::HitSide::Environment,
                    half_extent: ae::Vec2::splat(bomb.blast_radius),
                    damage: bomb.damage,
                    knockback: bomb.blast_radius * 2.4,
                    lifetime_s: 0.08,
                    name: Some("bomb blast"),
                },
            ),
        });
        commands.entity(entity).despawn();
    }
}

/// The localizer's window on a bomb: the two fields that move.
///
/// ⛔ THE FUSE, not the authored numbers beside it. Damage, radius and threshold
/// are constants copied off the move and cannot diverge; hashing them would make
/// every bomb of the same kind indistinguishable in the probe while the one field
/// that actually differs went unwatched.
///
/// ⚠ IT USED TO HASH A REMEMBERED SPEED TOO, and that field is gone: the impact
/// speed is published by the settle itself (`SettledItem::impact_speed`) rather
/// than carried here, so the bomb has exactly one moving field left.
pub fn live_bomb_probe(bomb: &LiveBomb) -> u64 {
    bomb.fuse_s.to_bits() as u64
}

#[cfg(test)]
mod tests;
