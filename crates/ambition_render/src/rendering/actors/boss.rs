//! GNU-ton was its only instance, and the ADR-0020 mount/rider split superseded it: the giant
//! is a real mount ACTOR whose hands are real limb bodies the rider boss's strikes drive.
//! Render-only layers can't be hit, possessed, or killed; limbs can.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::rendering::primitives::{FeatureVisual, PlayerVisual};
use ambition_sprite_sheet::boss::{self as sprites, BossAnimState, BossAnimator};
use ambition_sprite_sheet::character::CharacterAnimator;
use ambition_sprite_sheet::game_assets::GameAssets;

/// Replace the static `boss_core.png` look on boss feature entities with
/// the animated boss spritesheet once the asset is available. Symmetric
/// with `upgrade_actor_sprites` but uses `BossAnimator` instead of
/// `CharacterAnimator` because the boss generator emits its own row set.
pub fn upgrade_boss_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    // Readiness, not residency — see `super::texture_is_ready`.
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    // The boss's geometry (its render `size`) rides its `FeatureView`; its static
    // identity (name + behavior id, for the sheet lookup) rides `BossRenderIndex`.
    // Reading both by id lets this system bind a boss sheet WITHOUT borrowing the
    // live boss clusters — the boss render becomes a read-model consumer.
    feature_views: Res<ambition_sim_view::FeatureViewIndex>,
    boss_render: Res<ambition_sim_view::BossRenderIndex>,
    new_bosses: Query<
        (Entity, &FeatureVisual),
        (Without<CharacterAnimator>, Without<BossAnimator>),
    >,
    // Boss keys we've already warned about resolving no dedicated sheet, so the
    // warning fires once per boss instead of every time one spawns.
    mut warned_generic_bosses: Local<std::collections::HashSet<String>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual) in &new_bosses {
        // The read-model IS the gate: a non-boss (or not-yet-materialized) id has
        // no boss identity and is skipped — its geometry view alone isn't enough.
        let (Some(view), Some(boss_ident)) =
            (feature_views.get(&visual.id), boss_render.get(&visual.id))
        else {
            continue;
        };
        // Pick the per-boss sheet by authored name / behavior id. Each boss has
        // its own spritesheet from a dedicated Python generator; unrecognized
        // bosses fall back to the gradient-sentinel sheet. If no asset is
        // available we skip — the colored rectangle in `sync_visuals` renders.
        let boss_name = boss_ident.name.as_str();
        let boss_behavior_id = boss_ident.behavior_id.as_str();
        let _ = boss_name;
        let boss_key = boss_behavior_id.to_ascii_lowercase().replace('-', "_");
        // Dedicated sheets are keyed by `boss_key` in the asset registry, so the
        // former per-boss if-else chain collapses to one lookup + the generic
        // fallback.
        let dedicated = assets.boss_sprite(&boss_key);
        // Warn once for any boss without its own sheet (it renders with the
        // generic gradient-sentinel body) — the same signal the per-boss chain
        // gave, so a boss that should have art isn't silently shipped generic.
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
        // `with_render_basis` lets a trimmed (alpha-packed) boss sheet recompute
        // per-frame size/anchor in `animate_bosses`; untrimmed sheets ignore it.
        // `try_insert`: REPRODUCED, not reasoned. The boss visual is
        // a `FeatureVisual`, and `despawn_dead_dynamic_feature_visuals` retires
        // exactly those when a feature's view disappears — a boss dying on the
        // frame its sheet finishes loading is the ordinary way to hit it.
        // `boss_pass::the_boss_sprite_upgrade_survives_its_target_being_retired`
        // panics against the plain `insert`.
        commands.entity(entity).try_insert((
            sprite,
            anchor,
            BossAnimator::new(boss_asset).with_render_basis(render_size, anchor.0),
        ));
    }
}

/// The set [`animate_bosses`] runs in.
///
/// Camera shake and follow read the pose it resolves, so they run after it —
/// "this frame's resolved snapshot, not last frame's", per the host's own note.
///
/// ONE member, and the neighbour is the reason: `manage_gradient_lane_visual`
/// is chained immediately after specifically so it can READ the move-derived
/// `BossAttackState` this system produces. Including it would make the camera
/// wait on a hazard visual it has nothing to do with.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BossAnimation;

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    // The frame is not derived here: the sim owns it (`drive_boss_animators`), and this system
    // mirrors the published cursor into the draw-only animator and renders it.
    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
    // The draw cursor is READ from `boss_frames` (the by-id read-model), NOT from a
    // `&BossAnimFrame` component: this presentation entity is a `FeatureVisual`
    // mirror that never carries the sim's `BossAnimFrame` (that lives on the
    // separate sim boss entity). Querying the component here matched ZERO bosses
    // and froze every boss on frame 0 — the read-model is the boundary.
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            // Immutable: the animator is a stateless texture addresser now — the
            // draw cursor lives in `boss_frames`, not on this component.
            &BossAnimator,
            Option<&mut bevy::sprite::Anchor>,
        ),
        Without<PlayerVisual>,
    >,
    // Localized gravity, so a boss under flipped / sideways gravity flips the
    // same way the player and enemies do (it self-rights via `ActorRoll`, so its
    // facing must be gravity-aware too or the 180° roll mirrors it backwards).
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
) {
    // ADR 0011 — per-entity proper time. The "boss got root on the
    // simulator" pattern (ADR 0010 §Narrative authority) plays out
    // here: a boss with ProperTimeScale > 1.0 keeps tickling its
    // own animation while the world is frozen by its SimClock
    // request.
    for (visual, mut sprite, animator, anchor) in &mut query {
        let Some(view) = boss_frames.get(&visual.id) else {
            continue;
        };
        let state: BossAnimState = view.anim;
        // Draw the SIM-owned cursor published in the read-model. `drive_boss_animators`
        // advanced it this tick; the render only addresses the atlas cell for that
        // `(anim, frame)`, so the drawn sprite and the strike geometry share the ONE
        // sim frame.
        let (cursor_anim, cursor_frame) = (view.cursor_anim, view.cursor_frame);
        let index = animator.flat_index(cursor_anim, cursor_frame);
        // Split sheets: select the page image the active frame draws from
        // before setting the (page-local) index. Single-page bosses skip this.
        if animator.is_paged() {
            let page = animator.page_of(cursor_anim, cursor_frame);
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
        // Default art faces +x (right). A sheet drawn facing left (the
        // mockingbird) sets `authored_faces_left`, which inverts the flip so
        // the boss faces the player instead of always facing away. The
        // gravity-aware flip matches the player / enemy path: under normal
        // gravity it reduces to `spec.flip_x(facing)` (the gravity term is 0), and
        // under a flip it cancels the `ActorRoll` 180° mirror so the boss keeps
        // facing the player.
        let flip = ambition_sprite_sheet::art_is_mirrored(
            animator.spec.authored_faces_left,
            state.facing,
            gravity.dir_at(state.pos),
        );
        sprite.flip_x = flip;
        // `render_of` is `None` for untrimmed sheets, so those keep their spawn-time
        // size/anchor. The anchor x mirrors with the same facing flip applied to the sprite.
        if let (Some((size, mut anchor_v)), Some(mut anchor)) =
            (animator.render_of(cursor_anim, cursor_frame), anchor)
        {
            sprite.custom_size = Some(size);
            if flip {
                anchor_v.x = -anchor_v.x;
            }
            anchor.0 = anchor_v;
        }
        // Same split as `animate_characters`: hit feedback rides on
        // the white-silhouette `hit_flash` overlay; the warm
        // attack tint stays on `sprite.color` so the player can
        // read the boss's incoming swing telegraph.
        sprite.color = if state.attack_active || state.attack_windup {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}
