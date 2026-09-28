//! Mary-O's AI Slop — the plain stompable walker, authored as pure content.
//!
//! Mary-O has TWO enemies. Solid Snake is the shell enemy: a head-stomp makes it
//! withdraw into a kickable shell (see `snake.rs`). AI Slop is the SIMPLE one — the
//! throwaway mob a head-stomp just SQUASHES flat (similar to a Goomba). It renders
//! from the published `ai_slop` sheet.
//!
//! Unlike the snake, a squashed AI Slop does not become anything — it just dies. The
//! stomp reads the body's authored brain ([`is_ai_slop_brain`]), so it never touches
//! a snake (which owns its own shell rule); the two enemies never share a code path.
//!
//! Every type it names comes through the `ambition_platformer2d` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

use crate::stomp::{player_touch, PlayerTouch};

/// The catalog `display_name` an AI Slop renders from, and the name every AI Slop
/// spawn carries so its `ai_slop` sheet resolves.
pub const AI_SLOP_DISPLAY_NAME: &str = "AI Slop";

/// The roster brain key the AI Slop archetype is filed under, namespaced so it
/// never collides with a host provider's roster.
pub const AI_SLOP_BRAIN_KEY: &str = "mary_o_ai_slop";

/// The `ai_slop` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for an AI Slop.
pub const AI_SLOP_SHEET_TARGET: &str = "ai_slop";

/// Upward speed Mary-O gets off a squashed AI Slop — a lively hop, a touch under a
/// full jump so a stomp reads as a bounce, not a re-jump. Matches the snake's, so
/// bouncing off either enemy feels identical.
const BOUNCE_SPEED: f32 = 430.0;

// Demo-owned hostile archetype: ONE 1-HP `Wanderer` that walks forward and reverses
// at walls (`aggro_radius`/`attack_range` are ignored by that template). It carries
// no `melee`, so its only offense is the default-on body contact.
//
// The ROW is authored with no outer braces, so it can register on its own for a
// single-enemy test OR fold into the combined Mary-O roster fragment — one fragment
// per provider, since assembly rejects a second from the same provider.

// `AI_SLOP_TILE_COLUMNS`, `ai_slop_stair_steps()` and
// `stair_slop_spawn_positions()` are GONE. Where a slop stands is authored in
// the level; how big it is, is authored in its catalog row.

/// Half-extents of an AI Slop's idle body, in world units.
///
/// The same resolution construction performs (`posed_body_geometry` at the
/// scale its row names), so a level or a test that asks how big a slop is
/// compares against the body the cast builds, not a second derivation.
pub fn ai_slop_half_size() -> ae::Vec2 {
    let scale = crate::pack::PACK.posed_body_world_per_pixel(AI_SLOP_SHEET_TARGET);
    ambition_platformer2d::character_sprites::posed_body_geometry(
        AI_SLOP_SHEET_TARGET,
        ambition_platformer2d::sprite_sheet::character::CharacterAnim::Idle,
        scale,
    )
    .expect("the ai_slop sheet is baked")
    .collision
        * 0.5
}

/// Is this actor an AI Slop?
///
/// Both mobs made the same mistake twice, so the reasoning is written once.
///
/// The level authors them now, and the engine's own authored-enemy construction builds them —
/// this crate no longer stages a second copy.
pub fn is_ai_slop_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == AI_SLOP_BRAIN_KEY)
}

/// The head-stomp. A player on an AI Slop's head bounces up and squashes it —
/// the classic contact stomp, NOT the engine's attack-hitbox pogo. "On its head" is
/// the shared [`crate::stomp::PlayerTouch::Top`] rule, so this and the snake's shell
/// can never disagree about the same contact — and a player standing STILL on a mob
/// is stomping it, not being hurt by it.
///
/// Ordered BEFORE the shared body-contact-damage pass so a stomp never also hurts
/// the stomper: on a squash the mob's health is zeroed THIS frame (a component
/// write, immediately visible), so the contact pass sees a not-alive attacker and
/// skips it; the body is then despawned. A SIDE touch (no head overlap) is left
/// untouched here and lands as normal contact damage on Mary-O.
///
/// Why this despawns directly instead of routing through the shared actor-death
/// path (`HitEvent` → drops/score/debris): that path is DEFERRED — a hit emitted
/// here is consumed a stage later, so the mob would still be alive-and-hostile when
/// the contact pass runs THIS frame and would hurt the stomper. And it has no score
/// value and no drop table, so there is nothing for the shared path to carry. The
/// one thing a silent despawn would drop is the visible pop, so we emit a dust
/// [`ambition_platformer2d::vfx::VfxMessage::Burst`] at the corpse through the engine's own vfx
/// seam — a squash reads as a squash without adopting a wrong-ordered pipeline.
pub fn bounce_squash_ai_slop(
    mut commands: Commands,
    mut vfx: MessageWriter<ambition_platformer2d::vfx::VfxMessage>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut players: Query<(Entity, &mut ae::BodyKinematics), With<PrimaryPlayer>>,
    // Which bodies are AI Slop is their authored brain, read here rather
    // than copied onto a marker after they are built.
    mut mobs: Query<
        (Entity, &ae::BodyKinematics, &mut BodyHealth, &ActorConfig),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    let Ok((player_entity, mut player)) = players.single_mut() else {
        return;
    };
    let (p, pvel) = (player.aabb(), player.vel);
    for (entity, mob_kin, mut health, config) in &mut mobs {
        if !is_ai_slop_brain(&config.brain) || !health.alive() {
            continue;
        }
        // The shared top/side rule: only a TOP contact squashes. A side (or an
        // underside) touch is left alone here and lands as normal contact damage.
        if player_touch(mob_kin.aabb(), p, pvel) != Some(PlayerTouch::Top) {
            continue;
        }
        ae::movement::set_jump_velocity(&mut player.vel, ae::DEFAULT_GRAVITY_DIR, BOUNCE_SPEED);
        // The squash pops a low, tan dust burst through the engine's shared particle
        // seam, so the mob leaves a mark instead of blinking out.
        vfx.write(ambition_platformer2d::vfx::VfxMessage::Burst {
            pos: mob_kin.pos,
            count: 12,
            speed: 130.0,
            color: [0.80, 0.68, 0.48, 1.0],
            kind: ambition_platformer2d::vfx::ParticleKind::Dust,
        });
        // ...and the stomp thuds on the shared `Pogo` cue (the "you bounced off
        // something" verb a head-stomp is), voiced by the provider's own spec.
        //
        // H2: the STOMPER's, like every other pogo. The mob is what got bounced off,
        // not what made the sound.
        sfx.write_for(
            player_entity,
            ambition_platformer2d::sfx::SfxMessage::Pogo { pos: mob_kin.pos },
        );
        // Neutralize before the contact pass runs THIS frame, then remove the body.
        health.health.current = 0;
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests;
