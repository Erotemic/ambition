//! Boss sprite upgrade + animation (the boss spritesheet resolver, per-boss
//! animation, and the DATA-driven multi-layer boss render). Split out of the
//! actors renderer god-module; `use super::*` reaches the shared sprite-build
//! helpers + the marker components / z-constants.
//!
//! **Multi-layer bosses (fable review C7).** A boss whose art ships two sheets
//! keyed `{boss_key}_body` + `{boss_key}_hands` renders split across two layers
//! (body BEHIND one-way platforms, overlay in FRONT of the player), driven purely
//! by that asset CONVENTION — no per-boss code path. GNU-ton is the first such
//! boss (`gnu_ton_body` / `gnu_ton_hands`); any future giant gets the same look by
//! shipping the two sheets, editing no engine code.

use super::*;

/// Marks a multi-layer boss's BODY entity (the layer that sits behind one-way
/// platforms). The marker drives a follow-up system that overrides the entity's
/// z-translation so the body silhouette sits behind platforms, letting the player
/// read jump targets through a giant boss. Generic across bosses — GNU-ton is the
/// first, but the split-layer render is asset-convention-driven, not per-boss.
#[derive(Component)]
pub struct BossBodyLayer;

/// Marks the overlay child entity spawned alongside a multi-layer boss (GNU-ton's
/// hands are the first instance). A sync system mirrors the parent boss's atlas
/// index, page, + tint onto this child each frame, so both layers stay in lockstep
/// without needing a second `BossAnimator`. Carries the overlay sheet's per-page
/// handles so the child follows the parent onto the same page of a split sheet (the
/// body and overlay sheets are emitted in lockstep, so a shared page index applies).
#[derive(Component)]
pub struct BossOverlayLayer {
    pub pages: Vec<sprites::BossSpritePage>,
}

/// World-space z for a split-layer boss's BODY silhouette — between block tiles
/// (`WORLD_Z_BLOCK + 0.5 = 0.5`) and one-way platforms
/// (`WORLD_Z_BLOCK + 4.0 = 4.0`) so the body sits behind platforms but
/// in front of the wall tiles. Generic default; a per-boss override is a
/// parameterizable detail (bulk-review).
pub const BOSS_SPLIT_BODY_Z: f32 = 2.0;

/// World-space z for a split-layer boss's OVERLAY — just in front of the
/// player (`WORLD_Z_PLAYER = 20.0`) so the slamming layer reads as a
/// foreground threat the player navigates around.
pub const BOSS_SPLIT_OVERLAY_Z: f32 = 20.5;

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
        // Multi-layer boss render (fable review C7): a boss whose art ships
        // `{boss_key}_body` + `{boss_key}_hands` sheets renders split across two
        // layers — driven by the asset CONVENTION, not a per-boss string match. Any
        // boss gets the giant-behind-platforms look by shipping the two sheets. If
        // either layer is missing, fall back to the single-sheet path.
        let split_layers = match (
            assets.boss_sprite(&format!("{boss_key}_body")),
            assets.boss_sprite(&format!("{boss_key}_hands")),
        ) {
            (Some(body), Some(hands))
                if images.get(&body.pages[0].texture).is_some()
                    && images.get(&hands.pages[0].texture).is_some() =>
            {
                Some((body, hands))
            }
            _ => None,
        };
        // Dedicated sheets are keyed by `boss_key` in the asset registry, so the
        // former per-boss if-else chain collapses to one lookup + the generic
        // fallback. A split-layer boss's body sheet (above) takes precedence.
        let dedicated = assets.boss_sprite(&boss_key);
        // Warn once for any boss without its own sheet (it renders with the
        // generic gradient-sentinel body) — the same signal the per-boss chain
        // gave, so a boss that should have art isn't silently shipped generic.
        if split_layers.is_none()
            && dedicated.is_none()
            && warned_generic_bosses.insert(boss_key.clone())
        {
            bevy::log::warn!(
                target: "ambition::sprites",
                "boss '{boss_key}' has no dedicated spritesheet wired — rendering with the \
                 generic boss body. If it should have its own sprite, wire a BossSheetSpec + \
                 a boss_sprites entry (keyed by boss_key) + its loader (see \
                 flying_spaghetti_monster_boss).",
            );
        }
        let boss_asset = if let Some((body, _hands)) = split_layers {
            body
        } else if let Some(asset) = dedicated.or(assets.boss.as_ref()) {
            asset
        } else {
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
        let mut entity_commands = commands.entity(entity);
        // `with_render_basis` lets a trimmed (alpha-packed) boss sheet recompute
        // per-frame size/anchor in `animate_bosses`; untrimmed sheets ignore it.
        entity_commands.insert((
            sprite,
            anchor,
            BossAnimator::new(boss_asset).with_render_basis(render_size, anchor.0),
        ));
        if let Some((_body, hands)) = split_layers {
            // Spawn the overlay as a Bevy child so it inherits the parent's
            // translation. The child's local z offset puts the overlay well in
            // front of platforms (and slightly in front of the player) so incoming
            // slams read as foreground danger.
            entity_commands.insert(BossBodyLayer);
            let mut hands_sprite = Sprite::from_atlas_image(
                hands.texture(),
                bevy::image::TextureAtlas {
                    layout: hands.layout(),
                    index: hands.flat_index(sprites::BossAnim::Rest, 0),
                },
            );
            hands_sprite.custom_size = Some(render_size);
            let hands_pages = hands.pages.clone();
            entity_commands.with_children(|parent| {
                parent.spawn((
                    hands_sprite,
                    anchor,
                    BossOverlayLayer { pages: hands_pages },
                    // Local z offset relative to the parent body. The parent's
                    // absolute z is forced to `BOSS_SPLIT_BODY_Z` by
                    // `apply_boss_split_body_z` each frame, so this offset lands the
                    // child at `BOSS_SPLIT_OVERLAY_Z` in world space.
                    Transform::from_xyz(0.0, 0.0, BOSS_SPLIT_OVERLAY_Z - BOSS_SPLIT_BODY_Z),
                ));
            });
        }
    }
}

/// Override a split-layer boss parent entity's world z so the body
/// silhouette sits behind one-way platforms. `sync_visuals` resets
/// `translation.z` every frame from `feature_z(Boss) = 11.0`; this
/// system runs after it and rewrites just the z, leaving x/y alone.
pub fn apply_boss_split_body_z(mut query: Query<&mut Transform, With<BossBodyLayer>>) {
    for mut transform in &mut query {
        transform.translation.z = BOSS_SPLIT_BODY_Z;
    }
}

/// Mirror the parent boss's atlas index and color tint onto the overlay
/// child each frame. Both sheets share the same atlas layout
/// (same rows + frame counts) because the generator emits them in
/// lockstep, so the same flat index applies to both.
pub fn sync_boss_split_overlay(
    parents: Query<(&Sprite, &BossAnimator, &bevy::sprite::Anchor, &Children), With<BossBodyLayer>>,
    mut hands: Query<
        (&mut Sprite, &mut bevy::sprite::Anchor, &BossOverlayLayer),
        Without<BossBodyLayer>,
    >,
) {
    for (parent_sprite, animator, parent_anchor, children) in &parents {
        let Some(parent_atlas) = parent_sprite.texture_atlas.as_ref() else {
            continue;
        };
        let parent_index = parent_atlas.index;
        let parent_color = parent_sprite.color;
        // The body + hands sheets pack in lockstep (shared per-frame rect, page,
        // and alpha-trim), so EVERY per-frame render-basis value the parent
        // computes — page, flat index, trimmed `custom_size`, anchor, and facing
        // flip — applies verbatim to the hands overlay. Mirror them all so a
        // trimmed gnu_ton sheet keeps the hands aligned with the body.
        let parent_page = animator.current_page();
        let parent_size = parent_sprite.custom_size;
        let parent_flip = parent_sprite.flip_x;
        let parent_anchor_v = parent_anchor.0;
        for child in children.iter() {
            if let Ok((mut child_sprite, mut child_anchor, layer)) = hands.get_mut(child) {
                if let Some(page) = layer.pages.get(parent_page as usize) {
                    child_sprite.image = page.texture.clone();
                    if let Some(child_atlas) = child_sprite.texture_atlas.as_mut() {
                        child_atlas.layout = page.layout.clone();
                    }
                }
                if let Some(child_atlas) = child_sprite.texture_atlas.as_mut() {
                    child_atlas.index = parent_index;
                }
                child_sprite.color = parent_color;
                child_sprite.custom_size = parent_size;
                child_sprite.flip_x = parent_flip;
                child_anchor.0 = parent_anchor_v;
            }
        }
    }
}

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    // The boss frame read-model (E4 slice 7): facing flip + attack-telegraph
    // tint facts resolved sim-side into `BossFrameIndex`. The animation FRAME
    // is not derived here either (R1.3): the SIM owns it
    // (`drive_boss_animators` runs `request_for_phase` + `tick` and writes the
    // geometry sample), so this presentation system READS the already-driven
    // animator and only draws — no render→sim reads at all.
    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &BossAnimator,
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
    for (visual, mut sprite, animator, anchor) in &mut query {
        let Some(state): Option<BossAnimState> =
            boss_frames.get(&visual.id).map(|frame| frame.anim)
        else {
            continue;
        };
        // R1.3: the frame is driven SIM-side by `drive_boss_animators` (it ran
        // `request_for_phase` + `tick` and wrote the geometry sample this frame);
        // read the current flat index to draw, so the drawn pose and the strike
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
