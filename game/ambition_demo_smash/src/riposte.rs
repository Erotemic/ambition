//! The answering cut: a parry's response that hits back.
//!
//! This module owns no damage. It spawns one ordinary body strike through
//! `strike::spawn_body_strike`, and the single hitbox authority resolves it.
//! This module decides where the cut goes: in front of the fighter who
//! answered, at the authored reach.
//!
//! It must be a body strike, not an `EffectRequest::DamageBox`.
//! `spawn_damage_box` anchors every box as `HitboxAnchor::World`, and the
//! resolver maps `(HitSide::Player, HitboxAnchor::World { .. }) => None`: that
//! box damages nobody. The `Environment` side reaches bodies from a world
//! anchor but has no self-exclusion, so it would cut the fighter who parried.
//! See `a_hazard_hits_bystander_and_owner_alike_where_a_neutral_box_hits_neither`.
//!
//! So the cut is spawned like a move's own volume: `HitSide::Player`, anchored
//! `FollowOwner`. The owner is excluded by identity, and `damage_lands_between`
//! still decides teams, factions and friendly fire. The tests assert damage,
//! not the request shape, for this reason.
//!
//! Rollback: this technique adds no state. The cut uses components that are
//! already registered: `Hitbox` (`component-clone-entity-ref`, with
//! `map_entities`), `HitboxHits` (`component-clone-entity-set`, mapped) and
//! `HitboxLifetime` (`component-clone-probed`). No schema bump is needed. The
//! stored owner is a rollback-managed fighter, so the existing entity mapping
//! keeps the cut on the same body after a restore.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_riposte::{RiposteStrikeParams, RIPOSTE_STRIKE};
use ambition_platformer2d::engine_core as ae;

/// Cut in front of whoever a riposte was answered on.
pub fn cut_where_a_riposte_answers(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<(
        &ae::BodyKinematics,
        &ambition_platformer2d::world::ResolvedMotionFrame,
    )>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != RIPOSTE_STRIKE {
            continue;
        }
        let params: RiposteStrikeParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("riposte-strike params did not hydrate: {err}");
                continue;
            }
        };
        // The authoring check runs here, where both facts are available.
        // `CounterParams` has no constructor to assert in, so a bad value
        // would otherwise reach the player as a cut that does nothing.
        let problems = params.problems();
        if !problems.is_empty() {
            error!(
                target: "ambition::moves",
                "a riposte answers with an unusable cut and is skipped: {}",
                problems.join("; "),
            );
            continue;
        }
        let Ok((kin, frame)) = bodies.get(message.actor) else {
            continue;
        };
        // Body-local: the cut follows the fighter for its whole life, and
        // `+x` is their forward, not the world's. `spawn_body_strike` rotates
        // it through the frame below; only a rotated-frame fixture can tell a
        // world vector from a body-local one.
        let local_offset = ae::Vec2::new(kin.facing.signum() * params.reach, 0.0);
        info!(
            target: "ambition::moves",
            "riposte: cutting {local_offset:?} from {:?} for {} damage",
            kin.pos, params.damage,
        );
        ambition_platformer2d::combat::strike::spawn_body_strike(
            &mut commands,
            message.actor,
            local_offset,
            kin.facing.signum(),
            frame.basis(),
            ae::Vec2::new(params.half_extents.0, params.half_extents.1),
            params.damage as i32,
            params.knockback,
            params.lifetime_s,
            // The author's sound for this cut. Resolved here because `SfxId`
            // is a runtime hash, and the authored surface stays a plain string
            // such as `"player.slash"`.
            params
                .hit_sfx
                .as_deref()
                .map(ambition_platformer2d::sfx::SfxId::new),
        );
    }
}

#[cfg(test)]
mod tests;
