//! Boss sprite upgrade + animation (the boss spritesheet resolver and per-boss
//! animation). Split out of the actors renderer god-module; `use super::*`
//! reaches the shared sprite-build helpers + the marker components / z-constants.
//!
//! **A two-part boss is two linked actors, not two render layers.** The old
//! split-layer render (fable review C7) drew a giant's body behind one-way
//! platforms and its hands in front of the player, from a `{boss_key}_body` +
//! `{boss_key}_hands` sheet convention. GNU-ton was its only instance, and the
//! ADR-0020 mount/rider split superseded it: the giant is a real mount ACTOR
//! whose hands are real limb bodies the rider boss's strikes drive. Render-only
//! layers can't be hit, possessed, or killed; limbs can. Deleted in the E6
//! teardown (`refactor-chain.md` R2).

use super::*;

/// Replace the static `boss_core.png` look on boss feature entities with
/// the animated boss spritesheet once the asset is available. Symmetric
/// with `upgrade_actor_sprites` but uses `BossAnimator` instead of
/// `CharacterAnimator` because the boss generator emits its own row set.
pub fn upgrade_boss_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
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
                target: "ambition::sprites",
                "boss '{boss_key}' has no dedicated spritesheet wired — rendering with the \
                 generic boss body. If it should have its own sprite, wire a BossSheetSpec + \
                 a boss_sprites entry (keyed by boss_key) + its loader (see \
                 flying_spaghetti_monster_boss).",
            );
        }
        let Some(boss_asset) = dedicated.or(assets.boss.as_ref()) else {
            continue;
        };
        if images.get(&boss_asset.pages[0].texture).is_none() {
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
        commands.entity(entity).insert((
            sprite,
            anchor,
            BossAnimator::new(boss_asset).with_render_basis(render_size, anchor.0),
        ));
    }
}

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    // The boss frame read-model (E4 slice 7): facing flip + attack-telegraph
    // tint facts resolved sim-side into `BossFrameIndex`. The animation frame is
    // not derived here either: the SIM owns `BossAnimFrame`, so this presentation
    // system mirrors that cursor into the draw-only animator and renders it.
    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &BossAnimFrame,
            &mut BossAnimator,
            Option<&mut bevy::sprite::Anchor>,
        ),
        Without<PlayerVisual>,
    >,
    // Localized gravity, so a boss under flipped / sideways gravity flips the
    // same way the player and enemies do (it self-rights via `ActorRoll`, so its
    // facing must be gravity-aware too or the 180° roll mirrors it backwards).
    gravity: ambition_platformer_primitives::gravity::GravityCtx,
) {
    // ADR 0011 — per-entity proper time. The "boss got root on the
    // simulator" pattern (ADR 0010 §Narrative authority) plays out
    // here: a boss with ProperTimeScale > 1.0 keeps tickling its
    // own animation while the world is frozen by its SimClock
    // request.
    for (visual, mut sprite, frame, mut animator, anchor) in &mut query {
        let Some(state): Option<BossAnimState> =
            boss_frames.get(&visual.id).map(|frame| frame.anim)
        else {
            continue;
        };
        animator.mirror_frame(frame);
        // Read the current flat index to draw, so the drawn pose and the strike
        // geometry share the ONE sim-owned frame.
        let index = animator.current_flat_index();
        // Split sheets: select the page image the active frame draws from
        // before setting the (page-local) index. Single-page bosses skip this.
        if animator.is_paged() {
            let page = animator.current_page();
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
        let flip = ambition_platformer_primitives::gravity::gravity_aware_flip_x(
            state.facing,
            gravity.dir_at(state.pos),
        ) ^ animator.spec.authored_faces_left;
        sprite.flip_x = flip;
        // Alpha-trimmed (atlas-packed) boss sheets: re-derive per-frame size +
        // anchor so the logical frame stays fixed. `current_render` is `None`
        // for untrimmed sheets, so those keep their spawn-time size/anchor. The
        // anchor x mirrors with the same facing flip applied to the sprite.
        if let (Some((size, mut anchor_v)), Some(mut anchor)) = (animator.current_render(), anchor)
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
