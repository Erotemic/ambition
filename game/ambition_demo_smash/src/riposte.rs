//! The answering cut: a parry's response that hits back.
//!
//! ⭐⭐ THIS MODULE OWNS NO DAMAGE. It spawns one ordinary body strike through
//! `strike::spawn_body_strike`, and the single hitbox authority every swing in
//! the workspace goes through resolves it. What is decided here is WHERE the cut
//! goes: in front of the fighter who answered, at the reach their move authored.
//!
//! ⛔⛔ IT IS AN ORDINARY BODY STRIKE, AND THE FIRST DRAFT GOT THAT WRONG IN A
//! WAY NOTHING WOULD HAVE CAUGHT. It wrote an `EffectRequest::DamageBox`, the
//! request a mine's blast writes — and `spawn_damage_box` anchors every box as
//! `HitboxAnchor::World`, where the resolver's table reads
//! `(HitSide::Player, HitboxAnchor::World { .. }) => None`: a player-sided world
//! box takes no melee path and damages NOBODY. The only side that reaches
//! bodies from a world anchor is `Environment`, and `Environment` deliberately
//! consults no self-exclusion — *"your own bomb hurts you, and you still placed
//! it."* ⇒ Every spelling available through a `DamageBox` was wrong: one that
//! hurts no one, or one that cuts the fighter who parried.
//!
//! ⭐ SO THE CUT IS SPAWNED THE WAY A MOVE'S OWN VOLUME IS: `HitSide::Player`
//! anchored `FollowOwner`, through `strike::spawn_body_strike`. The owner is
//! excluded by identity, and `damage_lands_between` still decides teams,
//! factions and friendly fire — so a teams match does not have the answering
//! blade cutting an ally.
//!
//! ⚠ THE MISTAKE WAS INVISIBLE UNTIL A CITATION WAS READ. The `HitSide` comment
//! next door names the combat test that proves the hazard end
//! (`a_hazard_hits_bystander_and_owner_alike_where_a_neutral_box_hits_neither`),
//! and that test's own title is the correction. Its tests assert DAMAGE for
//! exactly this reason: a request-shaped assertion is a question about my own
//! authoring, not about the engine's answer to it.
//!
//! ⛔ ROLLBACK: THIS TECHNIQUE ADDS NO STATE, and that is worth stating rather
//! than leaving a reader to check. The cut is three components that are already
//! registered — `Hitbox` (`component-clone-entity-ref`, with `map_entities`
//! because it carries its owner), `HitboxHits` (`component-clone-entity-set`,
//! likewise mapped) and `HitboxLifetime` (`component-clone-probed`) — the exact
//! set `spawn_damage_box` has always spawned. ⇒ No new component, no probe, and
//! no schema bump: `GGRS_ROLLBACK_SCHEMA_VERSION` is untouched by this row.
//! ⚠ The owner it stores is a FIGHTER, which is itself rollback-managed, so the
//! entity mapping that already exists is what keeps the cut pointing at the
//! same body after a restore.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_riposte::{RiposteStrikeParams, RIPOSTE_STRIKE};
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
        // ⭐ THE AUTHORING CHECK RUNS HERE BECAUSE HERE IS WHERE BOTH FACTS ARE
        // IN HAND. A response's params are authored inside a `CounterParams`,
        // which has no constructor to assert in, so a bad value would otherwise
        // reach the player as a cut that does nothing.
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
        // ⭐ BODY-LOCAL, NOT WORLD. `FollowOwner` offsets are in the fighter's
        // own space, so the cut tracks them for its whole life instead of
        // hanging in the air where they were standing when they parried.
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
            frame.down(),
            ae::Vec2::new(params.half_extents.0, params.half_extents.1),
            params.damage as i32,
            params.knockback,
            params.lifetime_s,
            // ⭐ THE AUTHOR'S VOICE FOR THIS CUT. Resolved here rather than in
            // the params struct because `SfxId` is a runtime hash and the
            // authored surface stays a plain string — an author writes
            // `"player.slash"`, not an id.
            params
                .hit_sfx
                .as_deref()
                .map(ambition_platformer2d::sfx::SfxId::new),
        );
    }
}

#[cfg(test)]
mod tests;
