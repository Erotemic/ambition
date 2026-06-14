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
    ecs_bosses: Query<(&FeatureId, BossClusterRef, &crate::brain::BossAttackState)>,
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
                Some(crate::features::FeatureView {
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
        let boss_name = crate::features::ecs_boss_name(&visual.id, &ecs_bosses).unwrap_or("");
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
        } else if boss_key == "smirking_behemoth_boss" {
            let Some(asset) = assets
                .smirking_behemoth_boss
                .as_ref()
                .or(assets.boss.as_ref())
            else {
                continue;
            };
            asset
        } else if boss_key == "flying_spaghetti_monster_boss" {
            let Some(asset) = assets
                .flying_spaghetti_monster_boss
                .as_ref()
                .or(assets.boss.as_ref())
            else {
                continue;
            };
            asset
        } else if boss_key == "trex_boss" {
            let Some(asset) = assets.trex_boss.as_ref().or(assets.boss.as_ref()) else {
                continue;
            };
            asset
        } else {
            // No dedicated sheet wired for this boss — it renders with the
            // generic gradient-sentinel body. Surface it once (a boss with its
            // own art should have a branch above + a GameAssets field) instead
            // of silently shipping the wrong character, the bug that hid the
            // FSM / T-Rex behind the generic boss.
            if warned_generic_bosses.insert(boss_key.clone()) {
                bevy::log::warn!(
                    target: "ambition::sprites",
                    "boss '{boss_key}' has no dedicated spritesheet wired — rendering with the \
                     generic boss body. If it should have its own sprite, wire a BossSheetSpec + \
                     a GameAssets field + a resolver branch (see flying_spaghetti_monster_boss).",
                );
            }
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
    mut commands: Commands,
    world_time: Res<crate::WorldTime>,
    ecs_bosses: Query<(
        Entity,
        &FeatureId,
        BossClusterRef,
        &crate::brain::BossAttackState,
        &crate::brain::Brain,
    )>,
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
        let Some((boss_entity, state)): Option<(Entity, BossAnimState)> =
            crate::features::ecs_boss_anim_state_and_entity(&visual.id, &ecs_bosses)
        else {
            continue;
        };
        let anim = sprites::pick_boss_anim(state);
        let drive_phase = state.drive_phase();
        animator.request_for_phase(anim, drive_phase);
        let index = animator.tick(dt);
        let animation_sample = crate::features::ecs_boss_animation_frame_sample(
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
                .remove::<crate::features::BossAnimationFrameSample>();
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        // Default art faces +x (right). A sheet drawn facing left (the
        // mockingbird) sets `authored_faces_left`, which inverts the flip so
        // the boss faces the player instead of always facing away.
        sprite.flip_x = animator.spec.flip_x(state.facing);
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
