//! Mary-O's AI Slop — the plain stompable walker, authored as pure content.
//!
//! Mary-O has TWO enemies. Solid Snake is the shell enemy: a head-stomp makes it
//! withdraw into a kickable shell (see `snake.rs`). AI Slop is the SIMPLE one — the
//! throwaway mob a head-stomp just SQUASHES flat (similar to a Goomba). It renders
//! from the published `ai_slop` sheet.
//!
//! Unlike the snake, a squashed AI Slop does not become anything — it just dies. The
//! [`AiSlop`] marker keeps this stomp from ever touching a snake (which owns its own
//! shell rule); the two enemies never share a code path.
//!
//! Every type it names comes through the `ambition_platformer2d` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::components::CenteredAabb;
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

/// How close an observer must be for an AI Slop to keep thinking, in world units.
///
/// DERIVED from the view presets, not chosen by feel. A wake radius below
/// the half-width of the view a player selected pops an actor into frame already
/// moving. The five presets are 640/800/960/1120/1600 world units wide, so the
/// half-widths are 320/400/480/560/800. `Cinematic`'s 560 is the widest that
/// is a PLAY choice; 720 clears it by 160 — five tiles at this level's `T` — so a
/// slop has settled onto its column well before it is visible.
///
/// `Debug`'s 1600 is deliberately not covered. A developer at twice the
/// gameplay view is looking for exactly this kind of thing, and sizing content for
/// that zoom would leave the policy culling nothing.
///
/// The near pair is awake; everything from the third column out sleeps until she comes for it.
pub const AI_SLOP_WAKE_RADIUS: f32 = 720.0;

/// Upward speed Mary-O gets off a squashed AI Slop — a lively hop, a touch under a
/// full jump so a stomp reads as a bounce, not a re-jump. Matches the snake's, so
/// bouncing off either enemy feels identical.
const BOUNCE_SPEED: f32 = 430.0;

/// Marks a body as an AI Slop, so the stomp rule finds its own and never squashes a
/// snake. Inserted by [`tag_mary_o_ai_slop`] off the authored [`FeatureName`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AiSlop;

/// Demo-owned hostile archetype: ONE 1-HP `Wanderer` that walks forward and reverses
/// at walls (`aggro_radius`/`attack_range` are ignored by that template). It carries
/// no `melee`, so its only offense is the default-on body contact.
///
/// The ROW is authored with no outer braces, so it can register on its own for a
/// single-enemy test OR fold into the combined Mary-O roster fragment — one fragment
/// per provider, since assembly rejects a second from the same provider.

/// Ensure the `ai_slop` sheet is drawable, keyed by BOTH its catalog id and its
/// display name, so the enemy render's `npc_asset_for_name` finds it instead of
/// falling back to the generic goblin sheet.
///
/// Identical strategy to the snake's `register_solid_snake_sheet`: the catalog
/// defers a non-eager character's sheet to a room-staging barrier in the app host
/// that a standalone demo does not reliably drive, so the demo OWNS its enemy sheet
/// — a per-frame insert-if-missing load through the target loader (which bypasses
/// the lean sandbox catalog), self-healing across a `GameAssets` rebuild and a
/// no-op headless / `--no-assets`.
pub fn register_ai_slop_sheet(
    game_assets: Option<ResMut<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>>,
    config: Option<Res<ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig>>,
    asset_server: Option<Res<AssetServer>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
) {
    let (Some(mut game_assets), Some(config), Some(asset_server), Some(mut layouts)) =
        (game_assets, config, asset_server, layouts)
    else {
        return;
    };
    if config.no_assets || game_assets.characters.sheet(AI_SLOP_DISPLAY_NAME).is_some() {
        return;
    }
    if let Some(asset) =
        ambition_platformer2d::actors::character_sprites::load_prop_sheet_for_target(
            &asset_server,
            &mut layouts,
            &config.sprite_folder,
            AI_SLOP_SHEET_TARGET,
            &ambition_platformer2d::sprite_sheet::character::SheetTuning::new(1.0, 0),
        )
    {
        // Double-keyed exactly like the eager loader: the render resolves an actor
        // by its display name, and other seams by the catalog id.
        game_assets
            .characters
            .publish_under(AI_SLOP_SHEET_TARGET, asset.clone());
        game_assets
            .characters
            .publish_under(AI_SLOP_DISPLAY_NAME, asset);
    }
}

// `AI_SLOP_TILE_COLUMNS`, `ai_slop_stair_steps()` and
// `stair_slop_spawn_positions()` are GONE. Where a slop stands is authored.
//
// its SIZE stays here, and the difference is the rule: how big a slop is, is a
// fact about the character; where it stands is a fact about the level.

/// How WIDE an AI Slop is, in world units. The one authored number; its
/// height follows from the art.
///
/// WIDTH is the anchor because width is what the level sees. A slop
/// patrols a corridor; how much of it it occupies is the fact a room is authored
/// against, and 28 is what it occupied before — so this change costs no level a
/// re-author. The height stops being a claim and becomes a measurement.
///
/// this function's own doc already said the snake was *"one step further along
/// — its size comes from its sheet"*. This is the slop taking that step.
pub const AI_SLOP_BODY_WIDTH: f32 = 28.0;

/// World units per sheet pixel, derived so the body is [`AI_SLOP_BODY_WIDTH`]
/// wide and the art's own aspect decides the rest.
///
/// derived per call rather than pinned, exactly like
/// `mary_o_world_per_pixel`: the sheets are regenerated regularly and every
/// regeneration re-measures the alpha bbox, so a scale pinned to today's pixel
/// count silently resizes the creature the first time a crop moves by a pixel.
pub fn ai_slop_half_size() -> ae::Vec2 {
    let fallback = ae::Vec2::splat(AI_SLOP_BODY_WIDTH * 0.5);
    let Some(sheet) = ambition_platformer2d::character_sprites::posed_body_geometry(
        AI_SLOP_SHEET_TARGET,
        ambition_platformer2d::sprite_sheet::character::CharacterAnim::Idle,
        1.0,
    ) else {
        return fallback;
    };
    if sheet.collision.x <= 0.0 || sheet.collision.y <= 0.0 {
        return fallback;
    }
    let world_per_pixel = AI_SLOP_BODY_WIDTH / sheet.collision.x;
    sheet.collision * world_per_pixel * 0.5
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

/// Tag freshly staged AI Slop with the [`AiSlop`] marker, so the stomp rule finds
/// its own, and hold it to the size its sheet describes.
///
/// an enemy's authored rectangle says WHERE, not HOW BIG. The engine
/// spawns an authored body at the rect the level draws, which is right for
/// geometry and wrong for a character: how big a slop is, is a fact about the
/// slop. The snake makes the same statement one step further along — its size
/// comes from its sheet, and `SpritePosedBody` overwrites the authored rect
/// within a few frames whatever the level says. Doing it here too means the two
/// mobs answer "how big" the same way, and an author moving or resizing a
/// placement in LDtk gets a predictable answer instead of two different ones.
pub fn tag_mary_o_ai_slop(
    mut commands: Commands,
    mut fresh: Query<
        (
            Entity,
            &ActorConfig,
            &mut CenteredAabb,
            &mut ae::BodyKinematics,
        ),
        Without<AiSlop>,
    >,
) {
    for (entity, config, mut body, mut kin) in &mut fresh {
        if is_ai_slop_brain(&config.brain) {
            let half = ai_slop_half_size();
            // WRITE THE AUTHORITY, NOT ONLY THE MIRROR. `CenteredAabb` is DERIVED from
            // `BodyKinematics.size` — `reset_to_spawn` and the mount seam both do `aabb.half_size =
            // kin.size * 0.5` — so writing the box alone reached the slop for two ticks and was
            // then overwritten by the size the spawn gave it.
            //
            // `kin.size` is a FULL size and `half_size` is half of it; the
            // authored width (`AI_SLOP_BODY_WIDTH`) is the FULL width, which is
            // why `ai_slop_half_size()` is doubled here and not there.
            kin.size = half * 2.0;
            body.half_size = half;
            // The dormancy policy rides the same tag pass rather than the spawn
            // request, because `SpawnActorRequest` is the ENGINE's vocabulary for
            // what an actor IS and dormancy is a per-character decision the
            // content makes about it. Declaring it here also means a slop staged
            // by any future path picks it up for free.
            commands.entity(entity).try_insert((
                AiSlop,
                ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy::
                    AwakeNearObservers { radius: AI_SLOP_WAKE_RADIUS },
            ));
        }
    }
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
    mut mobs: Query<
        (Entity, &ae::BodyKinematics, &mut BodyHealth),
        (With<AiSlop>, Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    let Ok((player_entity, mut player)) = players.single_mut() else {
        return;
    };
    let (p, pvel) = (player.aabb(), player.vel);
    for (entity, mob_kin, mut health) in &mut mobs {
        if !health.alive() {
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
