//! Boss sprite binding and per-frame boss animation.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::rendering::primitives::{FeatureVisual, PlayerVisual};
use ambition_sprite_sheet::boss::{self as sprites, BossAnimState, BossAnimator};
use ambition_sprite_sheet::character::CharacterAnimator;
use ambition_sprite_sheet::game_assets::GameAssets;

/// Replace the static `boss_core.png` look on boss feature entities with the
/// animated boss spritesheet once the asset is available. Like
/// `upgrade_actor_sprites`, but uses `BossAnimator` instead of
/// `CharacterAnimator` because the boss generator emits its own row set.
pub fn upgrade_boss_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    // Readiness, not residency; see `super::texture_is_ready`.
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    // The boss's render `size` is on its `FeatureView`; its static identity
    // (name and behavior id, for the sheet lookup) is on `BossRenderIndex`.
    // Reading both by id binds a boss sheet without the live boss clusters.
    feature_views: Res<ambition_sim_view::FeatureViewIndex>,
    boss_render: Res<ambition_sim_view::BossRenderIndex>,
    new_bosses: Query<
        (Entity, &FeatureVisual),
        (Without<CharacterAnimator>, Without<BossAnimator>),
    >,
    // Boss keys already warned about (no dedicated sheet), so each warns
    // once.
    mut warned_generic_bosses: Local<std::collections::HashSet<String>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual) in &new_bosses {
        // The read model is the gate: an id with no boss identity (not a boss,
        // or not materialized yet) is skipped.
        let (Some(view), Some(boss_ident)) =
            (feature_views.get(&visual.id), boss_render.get(&visual.id))
        else {
            continue;
        };
        // Pick the per-boss sheet by behavior id. Unrecognized bosses use the
        // gradient-sentinel sheet. With no asset, skip; `sync_visuals` draws
        // the coloured rectangle.
        let boss_name = boss_ident.name.as_str();
        let boss_behavior_id = boss_ident.behavior_id.as_str();
        let _ = boss_name;
        let boss_key = boss_behavior_id.to_ascii_lowercase().replace('-', "_");
        // Dedicated sheets are keyed by `boss_key` in the asset registry: one
        // lookup plus the generic fallback.
        let dedicated = assets.boss_sprite(&boss_key);
        // Warn once for a boss without its own sheet (it uses the generic
        // gradient-sentinel body), so missing art is not shipped silently.
        if dedicated.is_none() && warned_generic_bosses.insert(boss_key.clone()) {
            bevy::log::warn!(
                target: "ambition_platformer2d::sprites",
                "boss '{boss_key}' has no dedicated spritesheet wired — rendering with the \
                 generic boss body. If it should have its own sprite, wire a BossSheetSpec + \
                 a boss_sprites entry (keyed by boss_key) + its loader (see \
                 flying_spaghetti_monster_boss).",
            );
        }
        let Some(boss_asset) = dedicated.or(assets.boss.as_ref()) else {
            continue;
        };
        if !super::texture_is_ready(&asset_server, &images, &boss_asset.pages[0].texture) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let render_size = boss_asset.spec.render_size(collision);
        let anchor = boss_asset.spec.collision_anchor(collision);
        let mut sprite = Sprite::from_atlas_image(
            boss_asset.texture(),
            bevy::image::TextureAtlas {
                layout: boss_asset.layout(),
                index: boss_asset.flat_index(sprites::BossAnim::Rest, 0),
            },
        );
        sprite.custom_size = Some(render_size);
        // `with_render_basis` lets a trimmed (alpha-packed) boss sheet compute
        // per-frame size and anchor in `animate_bosses`; untrimmed sheets ignore
        // it. `try_insert`: a boss can die on the frame its sheet finishes
        // loading, and `despawn_dead_dynamic_feature_visuals` then removes the
        // `FeatureVisual`. Covered by
        // `boss_pass::the_boss_sprite_upgrade_survives_its_target_being_retired`.
        commands.entity(entity).try_insert((
            sprite,
            anchor,
            BossAnimator::new(boss_asset).with_render_basis(render_size, anchor.0),
        ));
    }
}

/// The set [`animate_bosses`] runs in.
///
/// Camera shake and follow read the pose it resolves, so they run after it
/// (this frame's snapshot, not last frame's).
///
/// One member only. `manage_gradient_lane_visual` runs right after to read
/// the `BossAttackState` this system produces; including it would make the
/// camera wait on an unrelated hazard visual.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BossAnimation;

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    // The sim owns the frame (`drive_boss_animators`); this system mirrors
    // the published cursor into the draw-only animator and renders it.
    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
    // Read the cursor from `boss_frames` (by id), not a `BossAnimFrame`
    // component: this presentation entity is a `FeatureVisual` mirror, and
    // `BossAnimFrame` is on the separate sim boss entity.
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            // Immutable: the animator only addresses the texture; the cursor is
            // in `boss_frames`.
            &BossAnimator,
            Option<&mut bevy::sprite::Anchor>,
        ),
        Without<PlayerVisual>,
    >,
    // Localized gravity, so a boss under flipped or sideways gravity flips
    // like the player and enemies. It self-rights with `ActorRoll`, so its
    // facing must be gravity-aware or the 180° roll mirrors it backwards.
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
) {
    // ADR 0011: per-entity proper time. A boss with `ProperTimeScale > 1.0`
    // keeps animating while its `SimClock` request freezes the world
    // (ADR 0010, narrative authority).
    for (visual, mut sprite, animator, anchor) in &mut query {
        let Some(view) = boss_frames.get(&visual.id) else {
            continue;
        };
        let state: BossAnimState = view.anim;
        // Draw the sim-owned cursor from the read model. The render only
        // addresses the atlas cell for `(anim, frame)`, so the sprite and the
        // strike geometry share one sim frame.
        //
        // A hit reaction is presentation: the `Hit` row is drawn while the
        // flash runs, and the cursor (and its geometry) continues underneath, so
        // the attack resumes at the strike's frame.
        //
        // Then a PINNED row (content's `PinnedRow`: a scholar's tumble), when
        // this sheet has one of its names; else the cursor's slot row.
        let (row, frame) = match animator.spec.hit_reaction_frame(view.hit_flash_secs) {
            Some((anim, frame)) => (animator.spec.record_row(anim), frame),
            None => view
                .pinned
                .as_ref()
                .and_then(|pin| animator.pinned_cell(pin.rows.iter().map(String::as_str), pin.elapsed, pin.looping))
                .unwrap_or((animator.spec.record_row(view.cursor_anim), view.cursor_frame)),
        };
        let index = animator.flat_index_at(row, frame);
        // Split sheets: select the page for the active frame before setting
        // the page-local index. Single-page bosses skip this.
        if animator.is_paged() {
            let page = animator.page_at(row, frame);
            if let Some(pg) = animator.pages.get(page as usize) {
                sprite.image = pg.texture.clone();
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.layout = pg.layout.clone();
                }
            }
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        // Default art faces +x. A left-drawn sheet (the mockingbird) sets
        // `authored_faces_left`, which inverts the flip. The gravity term
        // matches the player and enemy path: zero under normal gravity, and
        // under a flip it cancels the `ActorRoll` 180° mirror.
        let flip = ambition_sprite_sheet::art_is_mirrored(
            animator.spec.authored_faces_left,
            state.facing,
            gravity.dir_at(state.pos),
        );
        sprite.flip_x = flip;
        // `render_of` is `None` for untrimmed sheets, which keep their spawn
        // size and anchor. The anchor x mirrors with the sprite flip.
        if let (Some((size, mut anchor_v)), Some(mut anchor)) =
            (animator.render_at(row, frame), anchor)
        {
            sprite.custom_size = Some(size);
            if flip {
                anchor_v.x = -anchor_v.x;
            }
            anchor.0 = anchor_v;
        }
        // Same split as `animate_characters`: hit feedback is the
        // `hit_flash` overlay; the warm attack tint stays on `sprite.color`
        // so the player can read the boss's swing telegraph.
        sprite.color = if state.attack_active || state.attack_windup {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}
