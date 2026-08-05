//! Mary-O's AI Slop — the plain stompable walker, authored as pure content.
//!
//! Mary-O has TWO enemies. Solid Snake is the shell enemy: a head-stomp makes it
//! withdraw into a kickable shell (see `snake.rs`). AI Slop is the SIMPLE one — the
//! throwaway mob a head-stomp just SQUASHES flat (similar to a Goomba). It renders
//! from the published `ai_slop` sheet.
//!
//! With **zero engine edits**:
//! - **Body + walk + contact damage** come from a demo-owned roster archetype
//!   (`mary_o_ai_slop`, a 1-HP `Wanderer` that paces and reverses at walls). Its
//!   `body_contact_damage` defaults true, so a side touch hurts Mary-O through the
//!   ONE shared body-contact pass — no bespoke code.
//! - **The sprite** is the `ai_slop` sheet, resolved by a demo catalog row of the
//!   same name so it renders standalone AND hosted.
//! - **The stomp** is a demo RULE, not the engine's attack-hitbox pogo (Jon:
//!   Mary-O does not pogo; she *bounces* on enemies to squash them). A player
//!   descending onto its head bounces up, and the mob dies on the spot.
//!
//! Unlike the snake, a squashed AI Slop does not become anything — it just dies. The
//! [`AiSlop`] marker keeps this stomp from ever touching a snake (which owns its own
//! shell rule); the two enemies never share a code path.
//!
//! Every type it names comes through the `ambition_platformer2d` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition_platformer2d::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::actors::combat::components::ActorFaction;
use ambition_platformer2d::actors::features::{SpawnActorKind, SpawnActorRequest};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

use crate::stomp::{player_touch, PlayerTouch};
use crate::{LEVEL_1_1_ROOM_ID, T};

/// The catalog `display_name` an AI Slop renders from, and the name every AI Slop
/// spawn carries so its `ai_slop` sheet resolves.
pub const AI_SLOP_DISPLAY_NAME: &str = "AI Slop";

/// The roster brain key the AI Slop archetype is filed under, namespaced so it
/// never collides with a host provider's roster.
pub const AI_SLOP_BRAIN_KEY: &str = "mary_o_ai_slop";

/// The `ai_slop` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for an AI Slop.
pub const AI_SLOP_SHEET_TARGET: &str = "ai_slop";

/// **How close an observer must be for an AI Slop to keep thinking**, in world
/// units. Jon's report is the reason this number exists: *"ai slop will just walk
/// off the edge of the level before she even gets to that part of the level"*.
///
/// ⭐ **DERIVED from the view presets, not chosen by feel.** A wake radius below
/// the half-width of the view a player selected pops an actor into frame already
/// moving. The five presets are 640/800/960/1120/1600 world units wide, so the
/// half-widths are 320/400/480/**560**/800. `Cinematic`'s 560 is the widest that
/// is a PLAY choice; 720 clears it by 160 — five tiles at this level's `T` — so a
/// slop has settled onto its column well before it is visible.
///
/// ⚠ **`Debug`'s 1600 is deliberately not covered.** A developer at twice the
/// gameplay view is looking for exactly this kind of thing, and sizing content for
/// that zoom would leave the policy culling nothing.
///
/// ⭐ and it culls the reported case: 1-1's slop sit at x = 416/768/1696/2240 plus
/// one per stair step near the flagpole, in a level 104 columns / 3328px wide,
/// with the player spawning at `2 * T` = 64. The near pair is awake; everything
/// from the third column out sleeps until she comes for it.
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
pub(crate) const AI_SLOP_ROSTER_ROWS: &str = r#"
    "mary_o_ai_slop": (
        max_health: 1,
        run_speed: 42.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Wanderer,
        move_style: Walk,
        respawn: OnRoomReenter,
    ),
"#;

/// Register the demo's AI Slop roster fragment. Shares the Mary-O provider id so its
/// brain key namespaces under this experience.
pub fn register_ai_slop_roster(app: &mut App) {
    use ambition_platformer2d::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(
            crate::provider::MARY_O_EXPERIENCE,
            None::<String>,
            &format!("{{{AI_SLOP_ROSTER_ROWS}}}"),
        )
        .expect("Mary-O AI Slop roster fragment should be valid"),
    );
}

/// **Ensure the `ai_slop` sheet is drawable**, keyed by BOTH its catalog id and its
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

/// Tile x-columns (level grid) each AI Slop paces near — interleaved with the snake
/// columns on the same ground segments (`[0,20) [22,42) [45,60) [65,104)` after the
/// level was lengthened for the vault), clear of the pits, the spawn, the surface
/// exit pipe (column 45), and the stairs/flagpole.
const AI_SLOP_TILE_COLUMNS: &[f32] = &[13.0, 24.0, 53.0, 70.0];

/// **One AI Slop on top of every stair step**, asked of the level rather than
/// copied from it.
///
/// Jon asked for these to test the quasar: "put some AI slop on the top of each
/// stairs so it goes down them and gets stuck in the bottom." They walk off
/// their step, tumble down, and collect in the trench between the two halves — a
/// crowd to run through while invincible, which is the only way to see that
/// invincibility is doing anything.
///
/// ⚠ this used to be a hand-copied `(column, height)` table duplicating the
/// arithmetic in `level_1_1`. Widening the trench moved the far half four
/// columns right and would have left four of these floating in mid-air over
/// where the steps used to be — a level edit silently breaking an enemy list in
/// another file is exactly the drift the ground list above is written to avoid.
fn ai_slop_stair_steps() -> Vec<(f32, f32)> {
    crate::stair_steps()
}

/// Half-extent of an AI Slop, shared by both spawn lists.
pub(crate) const AI_SLOP_HALF: f32 = 14.0;

/// Where each stair slop is dropped, for a level whose player spawns at
/// `player_spawn`. Exposed so the level's own oracle can check they land on real
/// steps rather than over where the steps used to be.
pub(crate) fn stair_slop_spawn_positions(player_spawn: ae::Vec2) -> Vec<ae::Vec2> {
    let ground_top = player_spawn.y + ae::movement::default_player_body_size().y * 0.5;
    ai_slop_stair_steps()
        .into_iter()
        .map(|(col, tiles)| ae::Vec2::new(col * T, ground_top - tiles * T - AI_SLOP_HALF - 4.0))
        .collect()
}

/// The AI Slop spawn requests for level 1-1: the ground patrol, dropped at the
/// player's standing height so gravity settles each onto its column, plus one
/// per stair step, dropped just above the step it belongs to.
fn ai_slop_spawn_requests(player_spawn: ae::Vec2) -> Vec<SpawnActorRequest> {
    let half = ae::Vec2::new(AI_SLOP_HALF, AI_SLOP_HALF);
    let request = |id: String, pos: ae::Vec2| SpawnActorRequest {
        id,
        name: AI_SLOP_DISPLAY_NAME.to_string(),
        pos,
        half_size: half,
        faction: ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: CharacterBrain::Custom(AI_SLOP_BRAIN_KEY.to_string()),
        },
    };
    // The ground patrol rides her spawn height directly and settles under
    // gravity; the stair drop needs the surface line, which
    // `stair_slop_spawn_positions` derives from the same spawn.
    AI_SLOP_TILE_COLUMNS
        .iter()
        .enumerate()
        .map(|(i, col)| {
            request(
                ai_slop_id(i),
                ae::Vec2::new(col * T, player_spawn.y),
            )
        })
        .chain(
            // The SAME positions the level's oracle checks — asking a second
            // copy of this arithmetic would make that check agree with itself
            // and with nothing that ships.
            stair_slop_spawn_positions(player_spawn)
                .into_iter()
                .enumerate()
                .map(|(i, pos)| request(format!("mary_o_ai_slop_stair_{i}"), pos)),
        )
        .collect()
}

/// Register the walkers as level 1-1's content staging: whenever the level's
/// contents are staged (initial load, every cyclic replay — the walkers
/// `respawn: OnRoomReenter` — and a snapshot restore), the walkers stage with them.
pub fn register_ai_slop_content_staging(
    registry: &mut ambition_platformer2d::actors::features::RoomContentStagingRegistry,
) {
    registry
        .register(
            LEVEL_1_1_ROOM_ID,
            "ambition_demo_mary_o",
            "ai_slop",
            "ai-slop-staging.v1",
            |spec| ai_slop_spawn_requests(spec.world.spawn),
        )
        .expect("AI Slop staging registration is unique");
}

/// Does this authored id belong to an AI Slop?
///
/// ⛔ **identity is the `FeatureId`, never the `FeatureName`.** The tag pass used
/// to match `name.0 == AI_SLOP_DISPLAY_NAME` — and `FeatureName`'s own doc says what it is:
/// *"human-facing authored name for debug overlays / inspectors."* Renaming the
/// character in the catalog would have silently stopped every stomp, because a
/// presentation string was carrying gameplay identity. `FeatureId` is the stable
/// one, and the spawn already builds it as `mary_o_ai_slop_<n>` — the archetype key IS
/// its prefix. (GPT 5.6's Mary-O spec: *"Do not use a human-readable display
/// name as gameplay identity."*)
/// Mint the authored id for one AI Slop. **Every path that stages one must use
/// this** — [`is_ai_slop_id`] is what decides it is slop at all, and one staged
/// under another id is an enemy nothing can stomp.
pub fn ai_slop_id(suffix: impl std::fmt::Display) -> String {
    format!("{AI_SLOP_BRAIN_KEY}_{suffix}")
}

pub fn is_ai_slop_id(id: &str) -> bool {
    id == AI_SLOP_BRAIN_KEY || id.starts_with(&format!("{AI_SLOP_BRAIN_KEY}_"))
}

/// Tag freshly staged AI Slop with the [`AiSlop`] marker, so the stomp rule finds
/// its own.
pub fn tag_mary_o_ai_slop(
    mut commands: Commands,
    fresh: Query<
        (
            Entity,
            &ambition_platformer2d::actors::features::FeatureId,
        ),
        Without<AiSlop>,
    >,
) {
    for (entity, id) in &fresh {
        if is_ai_slop_id(id.as_str()) {
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

/// **The head-stomp.** A player on an AI Slop's head bounces up and squashes it —
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
/// **Why this despawns directly instead of routing through the shared actor-death
/// path** (`HitEvent` → drops/score/debris): that path is DEFERRED — a hit emitted
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
