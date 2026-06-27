//! Boss sprite upgrade + animation (GNU-ton's body/hands layering, the boss
//! spritesheet resolver, per-boss animation). Split out of the actors renderer
//! god-module; `use super::*` reaches the shared sprite-build helpers + the
//! marker components / z-constants.

use super::*;

/// Marks a GNU-ton boss entity whose render is split across two layers
/// (body behind platforms, hands in front). The marker drives a follow-up
/// system that overrides the entity's z-translation so the body silhouette
/// sits behind one-way platforms, letting the player read jump targets
/// through the giant.
#[derive(Component)]
pub struct GnuTonBodyLayer;

/// Marks the hands overlay child entity spawned alongside a gnu_ton boss.
/// A sync system mirrors the parent boss's atlas index, page, + tint onto
/// this child each frame, so both layers stay in lockstep without needing a
/// second `BossAnimator`. Carries the hands sheet's per-page handles so the
/// child can follow the parent onto the same page of a split sheet (the body
/// and hands sheets are emitted in lockstep, so a shared page index applies).
#[derive(Component)]
pub struct GnuTonHandsLayer {
    pub pages: Vec<sprites::BossSpritePage>,
}

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
    ecs_bosses: Query<(
        &FeatureId,
        BossClusterRef,
        &ambition_characters::brain::BossAttackState,
    )>,
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
        let Some(view) = ecs_bosses
            .iter()
            .find_map(|(feature_id, item, attack_state)| {
                if feature_id.as_str() != visual.id.as_str() {
                    return None;
                }
                let boss = item.as_boss_ref();
                // `flash` reads `BossAttackState` instead of the deleted
                // `attack_timer` / `attack_windup_timer` mirror fields.
                Some(ambition_gameplay_core::features::FeatureView {
                    pos: boss.kin.pos,
                    size: boss.render_size(),
                    kind: FeatureVisualKind::Boss,
                    visible: boss.status.alive,
                    flash: boss.status.hit_flash > 0.0
                        || attack_state.telegraph_profile.is_some()
                        || attack_state.active_profile.is_some(),
                    switch_on: false,
                    rotation_rad: 0.0,
                })
            })
        else {
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
        let boss_name =
            ambition_gameplay_core::features::ecs_boss_name(&visual.id, &ecs_bosses).unwrap_or("");
        let boss_behavior_id = ecs_bosses
            .iter()
            .find_map(|(feature_id, item, _)| {
                (feature_id.as_str() == visual.id.as_str())
                    .then_some(item.config.behavior.id.as_str())
            })
            .unwrap_or(boss_name);
        let boss_key = boss_behavior_id.to_ascii_lowercase().replace('-', "_");
        let is_gnu_ton = boss_key == "gnu_ton"
            || boss_name.eq_ignore_ascii_case("gnu_ton")
            || boss_name.eq_ignore_ascii_case("gnu-ton")
            || boss_name.to_lowercase().starts_with("gnu_ton")
            || boss_name.to_lowercase().starts_with("gnu-ton");
        // GNU-ton gets a split body + hands render. If either layered
        // sheet is missing, fall back to the legacy single-sheet path.
        let split_layers = if is_gnu_ton {
            match (
                assets.boss_sprite("gnu_ton_body"),
                assets.boss_sprite("gnu_ton_hands"),
            ) {
                (Some(body), Some(hands))
                    if images.get(&body.pages[0].texture).is_some()
                        && images.get(&hands.pages[0].texture).is_some() =>
                {
                    Some((body, hands))
                }
                _ => None,
            }
        } else {
            None
        };
        // Dedicated sheets are keyed by `boss_key` in the asset registry, so the
        // former per-boss if-else chain collapses to one lookup + the generic
        // fallback. GNU-ton's split-layer body is the only special case above.
        let dedicated = assets.boss_sprite(&boss_key);
        // Warn once for any boss without its own sheet (it renders with the
        // generic gradient-sentinel body) — the same signal the per-boss chain
        // gave, so a boss that should have art isn't silently shipped generic.
        if !is_gnu_ton && dedicated.is_none() && warned_generic_bosses.insert(boss_key.clone()) {
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
            // Spawn the hands overlay as a Bevy child so it inherits the
            // parent's translation. The child's local z offset puts the
            // hands well in front of platforms (and slightly in front of
            // the player) so incoming slams read as foreground danger.
            entity_commands.insert(GnuTonBodyLayer);
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
                    GnuTonHandsLayer { pages: hands_pages },
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
    parents: Query<(&Sprite, &BossAnimator, &Children), With<GnuTonBodyLayer>>,
    mut hands: Query<(&mut Sprite, &GnuTonHandsLayer), Without<GnuTonBodyLayer>>,
) {
    for (parent_sprite, animator, children) in &parents {
        let Some(parent_atlas) = parent_sprite.texture_atlas.as_ref() else {
            continue;
        };
        let parent_index = parent_atlas.index;
        let parent_color = parent_sprite.color;
        // The body + hands sheets are emitted in lockstep, so the parent's
        // page-local flat index addresses the hands layout of the SAME page.
        let parent_page = animator.current_page();
        for child in children.iter() {
            if let Ok((mut child_sprite, layer)) = hands.get_mut(child) {
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
            }
        }
    }
}

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    mut commands: Commands,
    world_time: Res<ambition_gameplay_core::WorldTime>,
    ecs_bosses: Query<(
        Entity,
        &FeatureId,
        BossClusterRef,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut BossAnimator,
            Option<&mut bevy::sprite::Anchor>,
            Option<&ambition_gameplay_core::time::time_control::ProperTimeScale>,
        ),
        Without<PlayerVisual>,
    >,
    // Localized gravity, so a boss under flipped / sideways gravity flips the
    // same way the player and enemies do (it self-rights via `ActorRoll`, so its
    // facing must be gravity-aware too or the 180° roll mirrors it backwards).
    gravity: ambition_gameplay_core::physics::GravityCtx,
) {
    // ADR 0011 — per-entity proper time. The "boss got root on the
    // simulator" pattern (ADR 0010 §Narrative authority) plays out
    // here: a boss with ProperTimeScale > 1.0 keeps tickling its
    // own animation while the world is frozen by its SimClock
    // request.
    for (visual, mut sprite, mut animator, anchor, scale) in &mut query {
        let dt = world_time.entity_dt(
            ambition_gameplay_core::time::time_control::ProperTimeScale::or_default(scale),
        );
        let Some((boss_entity, state)): Option<(Entity, BossAnimState)> =
            ambition_gameplay_core::features::ecs_boss_anim_state_and_entity(
                &visual.id,
                &ecs_bosses,
            )
        else {
            continue;
        };
        let anim = sprites::pick_boss_anim(state);
        let drive_phase = state.drive_phase();
        animator.request_for_phase(anim, drive_phase);
        let index = animator.tick(dt);
        let animation_sample = ambition_gameplay_core::features::ecs_boss_animation_frame_sample(
            &visual.id,
            &ecs_bosses,
            anim,
            animator.frame,
        );
        if let Some((sample_entity, sample)) = animation_sample {
            commands.entity(sample_entity).insert(sample);
        } else {
            commands
                .entity(boss_entity)
                .remove::<ambition_gameplay_core::features::BossAnimationFrameSample>();
        }
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
        let flip = ambition_gameplay_core::physics::gravity_aware_flip_x(
            state.facing,
            gravity.dir_at(state.pos),
        ) ^ animator.spec.authored_faces_left;
        sprite.flip_x = flip;
        // Alpha-trimmed (atlas-packed) boss sheets: re-derive per-frame size +
        // anchor so the logical frame stays fixed. `current_render` is `None`
        // for untrimmed sheets, so those keep their spawn-time size/anchor. The
        // anchor x mirrors with the same facing flip applied to the sprite.
        if let (Some((size, mut anchor_v)), Some(mut anchor)) = (animator.current_render(), anchor) {
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
