//! Per-frame Bevy systems that mirror engine actor state into Bevy
//! sprites + animations. Covers the player, enemies, and bosses
//! along with the upgrade-to-spritesheet pass that converts the
//! initial colored rectangles into authored character sprites once
//! the asset is loaded.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{
    feature_color, feature_z, switch_on_color, FeatureVisual, PlayerSpriteBaseline, PlayerVisual,
    PropVisual, SceneEntities,
};
use crate::assets::game_assets::{self, EntitySprite, GameAssets};
use crate::boss_encounter::sprites::{self, BossAnimState, BossAnimator};
use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::features::{
    ActorRuntime, BossFeature, BreakableFeature, ChestFeature, FeatureId, FeatureViewIndex,
    FeatureVisualKind, Opened,
};
use crate::presentation::character_sprites::{
    build_character_sprite, feet_anchor_for, CharacterAnimator,
};

pub fn sync_visuals(
    world: Res<crate::GameWorld>,
    entities: Res<SceneEntities>,
    assets: Option<Res<GameAssets>>,
    feature_views: Res<FeatureViewIndex>,
    mut player_query: Query<
        (
            &mut Transform,
            &mut Sprite,
            Option<&PlayerSpriteBaseline>,
            &crate::player::PlayerBody,
            &crate::player::PlayerCombatState,
        ),
        With<PlayerVisual>,
    >,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
    ecs_chest_states: Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakable_states: Query<(&FeatureId, &BreakableFeature)>,
) {
    if let Ok((mut transform, mut sprite, baseline, body, player_combat)) =
        player_query.get_mut(entities.player)
    {
        transform.translation = world_to_bevy(&world.0, body.pos, WORLD_Z_PLAYER);
        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Colored-rectangle fallback only — stretch to the collision-box
            // size and tint by flash. Textured sprites (atlas OR plain image)
            // keep their authored size and are tinted in the animation system.
            sprite.custom_size = Some(BVec2::new(body.size.x, body.size.y));
            let alpha = if player_combat.flash_timer > 0.0 {
                0.72
            } else {
                1.0
            };
            sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
        } else if let Some(baseline) = baseline {
            // HACK(crouch-sprite-row): when the player crouches (or
            // morphs / crawls / slides), the engine shrinks the AABB
            // and slides `pos.y` down to keep feet planted. The
            // textured sprite was sized for the standing pose, so
            // without compensation it floats below the floor by half
            // the height delta. Re-scale the sprite's vertical extent
            // by the same ratio the collision shrunk; the normalized
            // sprite anchor preserves foot alignment automatically.
            // Phase 1 also lets the development menu swap standing body
            // profiles live. Scale the placeholder art against the recorded
            // startup collision so body-profile experiments remain visual.
            // Replace with authored body-profile rows once the generator emits
            // them — see PlayerSpriteBaseline doc.
            let base_y = body.base_size.y.max(1.0);
            let stance_ratio_y = (body.size.y / base_y).clamp(0.1, 1.0);
            let scale_x = body.base_size.x / baseline.standing_collision.x.max(1.0);
            let scale_y = body.base_size.y / baseline.standing_collision.y.max(1.0);
            sprite.custom_size = Some(BVec2::new(
                baseline.standing_render.x * scale_x,
                baseline.standing_render.y * scale_y * stance_ratio_y,
            ));
        }
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut feature_query {
        let Some(view) = feature_views.get(&visual.id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = world_to_bevy(&world.0, view.pos, feature_z(view.kind));

        // State-aware sprite swap for breakables and chests. Pickups are
        // chosen at spawn time and never change kind. Enemies are animated
        // through the character spritesheet path.
        if let Some(assets) = assets.as_deref() {
            if let Some(target_key) = state_aware_entity_sprite(
                &visual.id,
                view.kind,
                &ecs_chest_states,
                &ecs_breakable_states,
            ) {
                if let Some(handle) = assets.entities.get(target_key) {
                    if sprite.image != *handle {
                        sprite.image = handle.clone();
                    }
                }
            }
        }

        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Bare colored rectangle (no entity sprite available, no atlas).
            sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            sprite.color = if matches!(view.kind, FeatureVisualKind::Switch) && view.switch_on {
                switch_on_color()
            } else {
                feature_color(view.kind, view.flash)
            };
        } else if sprite.texture_atlas.is_none() {
            // Textured single-image entity sprite. Keep author size; tint
            // for hit-flash, otherwise white.
            sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            sprite.color = if view.flash {
                Color::srgba(1.0, 0.55, 0.55, 1.0)
            } else {
                Color::WHITE
            };
        }
        *visibility = if view.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn state_aware_entity_sprite(
    id: &str,
    kind: FeatureVisualKind,
    ecs_chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Breakable => crate::features::ecs_breakable_state(id, ecs_breakables)
            .map(game_assets::breakable_state_sprite),
        FeatureVisualKind::Chest => {
            crate::features::ecs_chest_opened(id, ecs_chests).map(game_assets::chest_state_sprite)
        }
        _ => None,
    }
}

/// Marker recording which `FeatureVisualKind` the current sprite +
/// `CharacterAnimator` were bound for. The upgrade systems read this
/// to detect mid-life kind changes — e.g. when a peaceful NPC turns
/// hostile and `apply_save` migrates the runtime entry from `npcs`
/// to `enemies`. Without this marker, the existing
/// `Without<CharacterAnimator>` filter hid the entity from the enemy
/// upgrade pass and the kernel guide stayed visually a kernel guide
/// after the third strike.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundFeatureKind(pub FeatureVisualKind);

/// Bind enemy/sandbag visuals to the appropriate character sheet
/// once the asset is available — and re-bind when an existing visual
/// changes kind (e.g. NPC → Enemy on hostility flip).
pub fn upgrade_enemy_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    feature_views: Res<FeatureViewIndex>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
    ecs_actors: Query<(&FeatureId, &ActorRuntime)>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        if !matches!(
            view.kind,
            FeatureVisualKind::Enemy | FeatureVisualKind::Sandbag
        ) {
            continue;
        }
        // Already bound to the correct kind — nothing to do this frame.
        if matches!(bound, Some(BoundFeatureKind(k)) if *k == view.kind) {
            continue;
        }
        // Sprite-override path: an enemy that was spawned by migrating
        // a hostile NPC carries the original LDtk display name so the
        // renderer can keep that NPC's sheet (with its authored slash
        // / hit rows). Only the Kernel Guide migration leaves the
        // override blank, so kernel→goblin keeps its dedicated visual
        // gag while every other faction NPC stays themselves when
        // hostile.
        //
        // Fallback for direct EnemySpawn entities (no NPC migration
        // history): try the enemy's display name against the same
        // NPC sprite registry. "Framebreaker" + "Nazi Salvage Guard"
        // resolve to fascist_enforcer_spritesheet this way without
        // authors having to duplicate the registry entry on an
        // enemy-side table.
        let character_asset =
            match crate::features::ecs_enemy_sprite_override(&visual.id, &ecs_actors) {
                Some(name) => assets
                    .characters
                    .npc_asset_for_name(name)
                    .or_else(|| {
                        crate::features::ecs_enemy_name(&visual.id, &ecs_actors)
                            .and_then(|n| assets.characters.npc_asset_for_name(n))
                    })
                    .or_else(|| assets.characters.enemy_asset(view.kind)),
                None => crate::features::ecs_enemy_name(&visual.id, &ecs_actors)
                    .and_then(|n| assets.characters.npc_asset_for_name(n))
                    .or_else(|| assets.characters.enemy_asset(view.kind)),
            };
        let Some(character_asset) = character_asset else {
            continue;
        };
        // Android loads assets out of the APK asynchronously, and missing or
        // platform-rejected images still have a Handle. Do not replace the
        // colored fallback with an atlas sprite until the texture is actually
        // present in Assets<Image>; otherwise a failed or delayed load renders
        // the NPC/enemy invisible.
        if images.get(&character_asset.texture).is_none() {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let sprite = build_character_sprite(character_asset, collision);
        commands.entity(entity).insert((
            sprite,
            feet_anchor_for(&character_asset.spec, collision),
            CharacterAnimator::new(&character_asset.spec),
            BoundFeatureKind(view.kind),
        ));
    }
}

/// Replace the static `EntitySprite::NpcTerminal` placeholder with a
/// faction-specific spritesheet once the asset is loaded. Today the
/// dispatch is keyed off the NPC's authored name (see
/// `CharacterSpriteAssets::npc_asset_for_name`); when LDtk grows a
/// `category` field on `NpcSpawn`, switch this to lookup-by-category
/// so the dispatch survives display-name edits.
///
/// NPCs without a registered sprite (the common case for the existing
/// hub guides etc.) keep the default terminal placeholder — symmetric
/// with `enemy_asset` returning `None` for non-enemy kinds.
pub fn upgrade_npc_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    feature_views: Res<FeatureViewIndex>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
    ecs_actors: Query<(&FeatureId, &ActorRuntime)>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Npc) {
            continue;
        }
        if matches!(bound, Some(BoundFeatureKind(k)) if *k == view.kind) {
            continue;
        }
        let Some(name) = crate::features::ecs_npc_name(&visual.id, &ecs_actors) else {
            continue;
        };
        let Some(character_asset) = assets.characters.npc_asset_for_name(name) else {
            continue;
        };
        // Keep the visible terminal/rectangle fallback until the PNG has
        // actually loaded. This is especially important on Android, where the
        // asset exists inside the APK but individual textures can still fail
        // or arrive later.
        if images.get(&character_asset.texture).is_none() {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let sprite = build_character_sprite(character_asset, collision);
        commands.entity(entity).insert((
            sprite,
            feet_anchor_for(&character_asset.spec, collision),
            CharacterAnimator::new(&character_asset.spec),
            BoundFeatureKind(view.kind),
        ));
    }
}

/// Drive the player sprite's animation state, atlas index, and facing flip.
/// Runs every frame; no-op on color-rectangle fallbacks (no `CharacterAnimator`).
pub fn animate_player(
    world_time: Res<crate::WorldTime>,
    primary_attack: Query<&crate::player::ActivePlayerAttack, crate::player::PrimaryPlayerOnly>,
    entities: Res<SceneEntities>,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &crate::player::PlayerBody,
            &crate::player::PlayerCombatState,
            &crate::player::PlayerMovementAuthority,
            &crate::player::PlayerAnimState,
            &crate::player::PlayerBlinkCameraState,
            Option<&crate::time::time_control::ProperTimeScale>,
        ),
        With<PlayerVisual>,
    >,
) {
    let Ok((
        mut sprite,
        mut animator,
        player_body,
        player_combat,
        authority,
        anim_state,
        blink_cam,
        scale,
    )) = query.get_mut(entities.player)
    else {
        return;
    };
    let attack_state = primary_attack.iter().next().and_then(|a| a.0.as_ref());
    let anim = crate::presentation::character_sprites::pick_player_anim(
        anim_state,
        player_combat,
        blink_cam,
        attack_state,
        &authority.player,
    );
    animator.request(anim);
    // ADR 0011 — `entity_dt` collapses to `sim_dt` when no
    // ProperTimeScale is set (SP default), so bullet-time /
    // hitstop / pause still slow the animation in lockstep. Step 4
    // wires the player ProperTimeScale path so future MP regimes
    // can boost the player's cognitive rate without slowing the
    // world for other observers.
    let index = animator.tick(world_time.entity_dt(
        crate::time::time_control::ProperTimeScale::or_default(scale),
    ));
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    }
    sprite.flip_x = player_body.facing < 0.0;
    // Keep the textured sprite at full opacity by default, with a subtle
    // red tint when invulnerable / hit so the existing flash signal still
    // reads. Tints multiply the texture color, so values below 1.0 darken
    // the channel.
    let flash_timer = player_combat.flash_timer;
    sprite.color = if flash_timer > 0.0 {
        Color::srgba(1.0, 0.55, 0.55, 1.0)
    } else {
        Color::WHITE
    };
}

/// Drive enemy AND NPC sprite animation, atlas index, and facing flip.
///
/// Enemies and NPCs both render through `CharacterAnimator`; their
/// per-frame state is owned by separate runtime lists, but a feature
/// id only ever appears in one of them at a time. We try the enemy
/// lookup first (most entities in the room) and fall through to the
/// NPC lookup, so a stationary General sheet ticks its 8 idle frames
/// once the animator is attached.
///
/// One system instead of two avoids the borrow conflict on the
/// shared `(&mut Sprite, &mut CharacterAnimator)` query.
pub fn animate_characters(
    world_time: Res<crate::WorldTime>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut CharacterAnimator,
            Option<&crate::time::time_control::ProperTimeScale>,
        ),
        (
            Without<PlayerVisual>,
            Without<crate::rooms::PortalSprite>,
            Without<PropVisual>,
        ),
    >,
    ecs_actors: Query<(&FeatureId, &ActorRuntime)>,
) {
    // ADR 0011 — per-entity proper time. SP today: no entity carries
    // ProperTimeScale, so `entity_dt` collapses to `sim_dt` and
    // every actor ticks at the world rate. The seam matters once a
    // boss freezes the world but leaves the player un-frozen, or
    // future MP boosts one player's proper time.
    for (visual, mut sprite, mut animator, scale) in &mut query {
        let dt = world_time.entity_dt(crate::time::time_control::ProperTimeScale::or_default(
            scale,
        ));
        let (anim, facing, hit_flash, attacking) = if let Some(state) =
            crate::features::ecs_enemy_anim_state(&visual.id, &ecs_actors)
        {
            (
                crate::presentation::character_sprites::pick_enemy_anim(state),
                state.facing,
                state.hit_flash,
                state.attack_active || state.attack_windup,
            )
        } else if let Some(state) = crate::features::ecs_npc_anim_state(&visual.id, &ecs_actors) {
            (
                crate::presentation::character_sprites::pick_npc_anim(state),
                state.facing,
                state.hit_flash,
                false,
            )
        } else {
            continue;
        };
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.flip_x = facing < 0.0;
        sprite.color = if hit_flash {
            Color::srgba(1.0, 0.55, 0.55, 1.0)
        } else if attacking {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}

/// Prop kinds whose authored "Idle" row depicts motion (e.g. rolling
/// wheels). These props stay pinned at frame 0 in [`animate_props`]
/// until a `PropMotionState` component lands to gate their tick by
/// real motion. Add a kind here when its sprite's idle frame reads
/// as "this prop is moving" — the cart is the v1 case.
pub const PROP_KINDS_STATIC_UNTIL_MOVING: &[&str] = &["intro_cart"];

/// Tick the idle animation row for every `PropVisual` sprite that
/// owns a `CharacterAnimator`. Props have no ECS actor entity, so
/// the regular `animate_characters` lookup would skip them — without
/// this system the sprite stays pinned to frame 0 forever.
///
/// Filtered with `Without<crate::rooms::PortalSprite>` so the gate
/// ring + gate portal stay owned by the portal-presentation systems
/// (which drive the animator from `PortalPhase` instead of a flat
/// Idle row tick).
///
/// Motion-gated props: a kind listed in [`PROP_KINDS_STATIC_UNTIL_MOVING`]
/// stays pinned at frame 0. The intro cart's authored "idle" row is a
/// wheel-rolling cycle that reads as "the cart is moving"; without a
/// real motion source today (no scripted push), looping it makes the
/// cart look like it's drifting in place. Until a `PropMotionState`
/// component lands, hold these kinds at rest.
pub fn animate_props(
    world_time: Res<crate::WorldTime>,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &PropVisual,
            Option<&crate::time::time_control::ProperTimeScale>,
        ),
        Without<crate::rooms::PortalSprite>,
    >,
) {
    // ADR 0011 — per-entity proper time. Props that need to keep
    // ticking when the world freezes (a clock prop in a frozen
    // boss arena, say) get a non-1.0 ProperTimeScale.
    for (mut sprite, mut animator, prop, scale) in &mut query {
        if PROP_KINDS_STATIC_UNTIL_MOVING.contains(&prop.kind.as_str()) {
            // Force-rest at frame 0 of the Idle row. `request` selects
            // the row; ticking with dt=0 holds the row's current frame
            // and matches the asset's first frame on entry.
            animator.request(crate::presentation::character_sprites::CharacterAnim::Idle);
            let index = animator.tick(0.0);
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
            continue;
        }
        let dt = world_time.entity_dt(crate::time::time_control::ProperTimeScale::or_default(
            scale,
        ));
        animator.request(crate::presentation::character_sprites::CharacterAnim::Idle);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
    }
}

/// Marks a GNU-ton boss entity whose render is split across two layers
/// (body behind platforms, hands in front). The marker drives a follow-up
/// system that overrides the entity's z-translation so the body silhouette
/// sits behind one-way platforms, letting the player read jump targets
/// through the giant.
#[derive(Component)]
pub struct GnuTonBodyLayer;

/// Marks the hands overlay child entity spawned alongside a gnu_ton boss.
/// A sync system mirrors the parent boss's atlas index + tint onto this
/// child each frame, so both layers stay in lockstep without needing a
/// second `BossAnimator`.
#[derive(Component)]
pub struct GnuTonHandsLayer;

/// World-space z for the GNU-ton body silhouette — between block tiles
/// (`WORLD_Z_BLOCK + 0.5 = 0.5`) and one-way platforms
/// (`WORLD_Z_BLOCK + 4.0 = 4.0`) so the body sits behind platforms but
/// in front of the wall tiles.
pub const GNU_TON_BODY_Z: f32 = 2.0;

/// World-space z for the GNU-ton hands overlay — just in front of the
/// player (`WORLD_Z_PLAYER = 20.0`) so the slamming hands read as a
/// foreground threat the player navigates around.
pub const GNU_TON_HANDS_Z: f32 = 20.5;

/// Replace the static `boss_core.png` look on boss feature entities with
/// the animated boss spritesheet once the asset is available. Symmetric
/// with `upgrade_enemy_sprites` but uses `BossAnimator` instead of
/// `CharacterAnimator` because the boss generator emits its own row set.
pub fn upgrade_boss_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    ecs_bosses: Query<(&FeatureId, &BossFeature)>,
    new_bosses: Query<
        (Entity, &FeatureVisual),
        (Without<CharacterAnimator>, Without<BossAnimator>),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual) in &new_bosses {
        let Some(view) = ecs_bosses.iter().find_map(|(feature_id, boss)| {
            if feature_id.as_str() != visual.id.as_str() {
                return None;
            }
            let boss = &boss.boss;
            Some(crate::features::FeatureView {
                pos: boss.pos,
                size: boss.render_size(),
                kind: FeatureVisualKind::Boss,
                visible: boss.alive,
                flash: boss.hit_flash > 0.0
                    || boss.attack_windup_timer > 0.0
                    || boss.attack_timer > 0.0,
                switch_on: false,
            })
        }) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Boss) {
            continue;
        }
        // Pick the per-boss sheet by authored name. Each boss has its
        // own spritesheet from a dedicated Python generator; unrecognized
        // bosses fall back to the gradient-sentinel sheet.
        // If no asset is available we skip — the colored rectangle
        // fallback in `sync_visuals` continues to render.
        let boss_name = crate::features::ecs_boss_name(&visual.id, &ecs_bosses).unwrap_or("");
        let is_gnu_ton = boss_name.eq_ignore_ascii_case("gnu_ton")
            || boss_name.eq_ignore_ascii_case("gnu-ton")
            || boss_name.to_lowercase().starts_with("gnu_ton")
            || boss_name.to_lowercase().starts_with("gnu-ton");
        // GNU-ton gets a split body + hands render. If either layered
        // sheet is missing, fall back to the legacy single-sheet path.
        let split_layers = if is_gnu_ton {
            match (assets.gnu_ton_body.as_ref(), assets.gnu_ton_hands.as_ref()) {
                (Some(body), Some(hands))
                    if images.get(&body.texture).is_some()
                        && images.get(&hands.texture).is_some() =>
                {
                    Some((body, hands))
                }
                _ => None,
            }
        } else {
            None
        };
        let boss_asset = if let Some((body, _hands)) = split_layers {
            body
        } else if boss_name.eq_ignore_ascii_case("mockingbird") {
            let Some(asset) = assets.mockingbird.as_ref().or(assets.boss.as_ref()) else {
                continue;
            };
            asset
        } else if is_gnu_ton {
            let Some(asset) = assets.gnu_ton.as_ref().or(assets.boss.as_ref()) else {
                continue;
            };
            asset
        } else {
            let Some(asset) = assets.boss.as_ref() else {
                continue;
            };
            asset
        };
        if images.get(&boss_asset.texture).is_none() {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let render_size = boss_asset.spec.render_size(collision);
        let anchor = boss_asset.spec.collision_anchor(collision);
        let mut sprite = Sprite::from_atlas_image(
            boss_asset.texture.clone(),
            bevy::image::TextureAtlas {
                layout: boss_asset.layout.clone(),
                index: boss_asset.spec.flat_index(sprites::BossAnim::Rest, 0),
            },
        );
        sprite.custom_size = Some(render_size);
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((sprite, anchor, BossAnimator::new(boss_asset.spec)));
        if let Some((_body, hands)) = split_layers {
            // Spawn the hands overlay as a Bevy child so it inherits the
            // parent's translation. The child's local z offset puts the
            // hands well in front of platforms (and slightly in front of
            // the player) so incoming slams read as foreground danger.
            entity_commands.insert(GnuTonBodyLayer);
            let mut hands_sprite = Sprite::from_atlas_image(
                hands.texture.clone(),
                bevy::image::TextureAtlas {
                    layout: hands.layout.clone(),
                    index: hands.spec.flat_index(sprites::BossAnim::Rest, 0),
                },
            );
            hands_sprite.custom_size = Some(render_size);
            entity_commands.with_children(|parent| {
                parent.spawn((
                    hands_sprite,
                    anchor,
                    GnuTonHandsLayer,
                    // Local z offset relative to the parent body. The
                    // parent's absolute z is forced to `GNU_TON_BODY_Z` by
                    // `apply_gnu_ton_body_z` each frame, so this offset
                    // lands the child at `GNU_TON_HANDS_Z` in world space.
                    Transform::from_xyz(0.0, 0.0, GNU_TON_HANDS_Z - GNU_TON_BODY_Z),
                ));
            });
        }
    }
}

/// Override the gnu_ton boss parent entity's world z so the body
/// silhouette sits behind one-way platforms. `sync_visuals` resets
/// `translation.z` every frame from `feature_z(Boss) = 11.0`; this
/// system runs after it and rewrites just the z, leaving x/y alone.
pub fn apply_gnu_ton_body_z(mut query: Query<&mut Transform, With<GnuTonBodyLayer>>) {
    for mut transform in &mut query {
        transform.translation.z = GNU_TON_BODY_Z;
    }
}

/// Mirror the parent boss's atlas index and color tint onto the hands
/// overlay child each frame. Both sheets share the same atlas layout
/// (same rows + frame counts) because the generator emits them in
/// lockstep, so the same flat index applies to both.
pub fn sync_gnu_ton_hands(
    parents: Query<(&Sprite, &Children), With<GnuTonBodyLayer>>,
    mut hands: Query<&mut Sprite, (With<GnuTonHandsLayer>, Without<GnuTonBodyLayer>)>,
) {
    for (parent_sprite, children) in &parents {
        let Some(parent_atlas) = parent_sprite.texture_atlas.as_ref() else {
            continue;
        };
        let parent_index = parent_atlas.index;
        let parent_color = parent_sprite.color;
        for child in children.iter() {
            if let Ok(mut child_sprite) = hands.get_mut(child) {
                if let Some(child_atlas) = child_sprite.texture_atlas.as_mut() {
                    child_atlas.index = parent_index;
                }
                child_sprite.color = parent_color;
            }
        }
    }
}

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    world_time: Res<crate::WorldTime>,
    ecs_bosses: Query<(&FeatureId, &BossFeature)>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut BossAnimator,
            Option<&crate::time::time_control::ProperTimeScale>,
        ),
        Without<PlayerVisual>,
    >,
) {
    // ADR 0011 — per-entity proper time. The "boss got root on the
    // simulator" pattern (ADR 0010 §Narrative authority) plays out
    // here: a boss with ProperTimeScale > 1.0 keeps tickling its
    // own animation while the world is frozen by its SimClock
    // request.
    for (visual, mut sprite, mut animator, scale) in &mut query {
        let dt = world_time.entity_dt(crate::time::time_control::ProperTimeScale::or_default(
            scale,
        ));
        let Some(state): Option<BossAnimState> =
            crate::features::ecs_boss_anim_state(&visual.id, &ecs_bosses)
        else {
            continue;
        };
        let anim = sprites::pick_boss_anim(state);
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.color = if state.hit_flash {
            Color::srgba(1.0, 0.55, 0.55, 1.0)
        } else if state.attack_active || state.attack_windup {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}

/// When `DeveloperTools::hide_sprites` is enabled, force every `Sprite`-bearing
/// entity to `Hidden` so only gizmo hitbox outlines remain visible. When the
/// flag flips off, restore every sprite to `Inherited` *exactly once* on the
/// falling edge — we deliberately do NOT keep stomping `Inherited` every
/// frame because that wipes out legitimate `Visibility::Hidden` writes from
/// upstream systems (collected pickups, idle morph-ball sphere, player while
/// in morph-ball mode, etc.) and makes them flicker back to visible.
/// UI uses `Node`/`ImageNode`, not `Sprite`, so HUD/menus are unaffected.
pub fn apply_hide_sprites_override(
    developer_tools: Res<crate::dev::dev_tools::DeveloperTools>,
    mut prev_active: Local<bool>,
    mut sprites: Query<&mut Visibility, With<Sprite>>,
) {
    let active = effective_hide_sprites(&developer_tools);
    if active {
        for mut vis in sprites.iter_mut() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
    } else if *prev_active {
        for mut vis in sprites.iter_mut() {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
        }
    }
    *prev_active = active;
}

fn effective_hide_sprites(developer_tools: &crate::dev::dev_tools::DeveloperTools) -> bool {
    // Placeholder art is a visible debug-art mode. If an old persisted or
    // inspector-mutated state leaves both booleans true, keep placeholders
    // visible instead of letting hide mode erase them.
    developer_tools.hide_sprites && !developer_tools.placeholder_sprites
}

/// Cached pre-placeholder sprite state so toggling `placeholder_sprites`
/// off can restore the textured rendering. Stored per-entity the first
/// time we collapse the sprite to a colored rectangle.
#[derive(Component, Clone)]
pub struct SpriteOriginalState {
    pub image: Handle<Image>,
    pub atlas: Option<bevy::image::TextureAtlas>,
    pub color: Color,
    pub custom_size: Option<BVec2>,
    pub image_mode: bevy::sprite::SpriteImageMode,
}

/// When `DeveloperTools::placeholder_sprites` is enabled, replace every
/// textured sprite with a colored rectangle of the collision/debug size —
/// the "placeholder art era" look. When the flag flips back off, restore
/// the original texture, atlas, tint, sizing, and image mode.
///
/// The placeholder color is derived from a per-entity discriminator
/// (`FeatureVisual` / `PlayerVisual` / boss / projectile markers) so
/// similar entities visually group. Anything without a known marker
/// falls back to the existing sprite color (kept as-is).
pub fn apply_placeholder_sprites_override(
    mut commands: Commands,
    developer_tools: Res<crate::dev::dev_tools::DeveloperTools>,
    feature_views: Res<FeatureViewIndex>,
    mut sprites: Query<(
        Entity,
        &mut Sprite,
        Option<&SpriteOriginalState>,
        Option<&FeatureVisual>,
        Option<&PlayerVisual>,
        Option<&crate::player::PlayerBody>,
        Option<&crate::projectile::PlayerProjectileVisual>,
        Option<&crate::enemy_projectile::EnemyProjectileVisual>,
    )>,
) {
    if developer_tools.placeholder_sprites {
        for (entity, mut sprite, original, feature, player, player_body, p_proj, e_proj) in
            &mut sprites
        {
            // Record original state once so we can restore on toggle-off.
            if original.is_none() {
                commands.entity(entity).insert(SpriteOriginalState {
                    image: sprite.image.clone(),
                    atlas: sprite.texture_atlas.clone(),
                    color: sprite.color,
                    custom_size: sprite.custom_size,
                    image_mode: sprite.image_mode.clone(),
                });
            }
            let feature_view = feature.and_then(|fv| feature_views.get(&fv.id));
            let placeholder_color = pick_placeholder_color(
                feature_view.map(|v| v.kind),
                player.is_some(),
                p_proj.is_some(),
                e_proj.is_some(),
            );
            // Drop the texture and atlas so the sprite renders as a flat
            // rectangle. Size feature placeholders to their gameplay AABB
            // rather than their authored render bounds so placeholder mode
            // doubles as a collision-readability mode.
            if sprite.image != Handle::default() {
                sprite.image = Handle::default();
            }
            if sprite.texture_atlas.is_some() {
                sprite.texture_atlas = None;
            }
            sprite.image_mode = bevy::sprite::SpriteImageMode::Auto;
            if let Some(view) = feature_view {
                sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            } else if let Some(body) = player_body {
                sprite.custom_size = Some(BVec2::new(body.size.x, body.size.y));
            }
            sprite.color = placeholder_color;
        }
    } else {
        // Restore any cached originals.
        for (entity, mut sprite, original, _, _, _, _, _) in &mut sprites {
            if let Some(orig) = original {
                if sprite.image != orig.image {
                    sprite.image = orig.image.clone();
                }
                if sprite.texture_atlas != orig.atlas {
                    sprite.texture_atlas = orig.atlas.clone();
                }
                sprite.color = orig.color;
                sprite.custom_size = orig.custom_size;
                sprite.image_mode = orig.image_mode.clone();
                commands.entity(entity).remove::<SpriteOriginalState>();
            }
        }
    }
}

fn pick_placeholder_color(
    feature_kind: Option<FeatureVisualKind>,
    is_player: bool,
    is_player_projectile: bool,
    is_enemy_projectile: bool,
) -> Color {
    if is_player {
        return Color::srgba(0.55, 0.85, 1.00, 1.0);
    }
    if is_player_projectile {
        return Color::srgba(1.00, 0.74, 0.30, 1.0);
    }
    if is_enemy_projectile {
        return Color::srgba(1.00, 0.32, 0.32, 1.0);
    }
    match feature_kind {
        Some(kind) => feature_color(kind, false),
        None => Color::srgba(0.70, 0.70, 0.72, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::effective_hide_sprites;
    use crate::dev::dev_tools::{DebugArtMode, DeveloperTools};

    #[test]
    fn placeholder_art_wins_over_stale_hide_flag() {
        let mut tools = DeveloperTools::default();
        tools.apply_debug_art_mode(DebugArtMode::Hidden);
        assert!(effective_hide_sprites(&tools));

        tools.hide_sprites = true;
        tools.placeholder_sprites = true;
        assert!(!effective_hide_sprites(&tools));
    }
}
