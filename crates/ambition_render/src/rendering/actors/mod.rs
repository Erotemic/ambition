//! Per-frame actor sprite and animation presentation.

use ambition_platformer2d_core as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::primitives::{
    feature_color, feature_z, switch_on_color, FeatureVisual, PlayerSpriteBaseline, PlayerVisual,
    PropVisual,
};
use ambition_persistence::settings::TextureResolutionScale;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d_shared_tangle::feature_kind::{BoundFeatureKind, FeatureVisualKind};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_sim_view::FeatureViewIndex;
use ambition_sprite_sheet::character::{
    build_character_presentation_with_render_size, feet_anchor_for_render_size, sprite_render_size,
    CharacterAnimator,
};
use ambition_sprite_sheet::game_assets::{self, EntitySprite, GameAssets};

/// The one answer to "what quad does this sheet draw with, and where is it anchored".
///
/// The actor road and the player road both use this rule. Both feed
/// `CharacterAnimator`, and `apply_character_frame` writes its basis back over
/// `custom_size` and `Anchor` every frame, so two different rules would fight.
///
/// Test the three cases in this order:
/// - An authored quad with an authored offset: the sheet publishes where its art goes
///   per pose. The quad is centred and the offset does the shift. A static feet anchor
///   on top would count the shift twice. That anchor also comes from the idle frame,
///   which is wrong for a body that changes silhouette.
/// - An authored quad with no offset: render at the stored quad, so the sprite does not
///   grow once collision equals the body. Keep the feet anchor.
/// - No authored quad: derive both from the collision box.
pub fn character_render_basis(
    spec: &ambition_sprite_sheet::character::CharacterSheetSpec,
    collision: BVec2,
    authored_render: Option<BVec2>,
    authored_offset: Option<BVec2>,
) -> (BVec2, Anchor) {
    match authored_render {
        Some(render) if authored_offset.is_some() => (render, Anchor::CENTER),
        Some(render) => (render, feet_anchor_for_render_size(spec, collision, render)),
        None => {
            let render = sprite_render_size(spec, collision);
            (render, feet_anchor_for_render_size(spec, collision, render))
        }
    }
}

/// Build a textured player's presentation from its collision box.
///
/// Returns the sprite and the `PlayerSpriteBaseline` measured from the same box.
/// `sync_visuals` scales the art by `base_size / standing_collision`, so a baseline
/// from a different box would scale every frame. The caller names the box once, so
/// the two cannot disagree.
pub fn player_presentation_for_collision(
    asset: &ambition_sprite_sheet::character::CharacterSpriteAsset,
    collision: BVec2,
    // An argument, not a lookup: each caller must state whether the body has a
    // sheet-authored quad. A clone with no pose yet passes `None`. The quad and
    // offset are both Some or both None (see `pose_view`'s `sheet_authored_body`).
    authored_render: Option<BVec2>,
    authored_offset: Option<BVec2>,
) -> (Sprite, Anchor, CharacterAnimator, PlayerSpriteBaseline) {
    let (render, anchor) =
        character_render_basis(&asset.spec, collision, authored_render, authored_offset);
    let (sprite, anchor, animator) =
        build_character_presentation_with_render_size(asset, render, anchor);
    (
        sprite,
        anchor,
        animator,
        PlayerSpriteBaseline {
            standing_render: render,
            standing_collision: collision,
        },
    )
}

/// Whether a texture handle is ready for presentation.
///
/// Asset-server handles use load state so readiness is independent of CPU
/// residency. Directly inserted/procedural handles have no load state, so their
/// presence in `Assets<Image>` is the readiness signal.
pub(crate) fn texture_is_ready(
    asset_server: &AssetServer,
    images: &Assets<Image>,
    handle: &Handle<Image>,
) -> bool {
    match asset_server.get_load_state(handle.id()) {
        Some(_) => asset_server.is_loaded_with_dependencies(handle),
        None => images.contains(handle),
    }
}

mod animation;
mod boss;
mod overlays;

pub use animation::*;
pub use boss::*;
pub use overlays::*;

/// Ensure every simulation-owned player visual has a renderable sprite.
///
/// A player that carries the canonical [`WornCharacter`] identity is owned by
/// [`bind_worn_character_presentation`] (it installs the sheet or a fallback
/// rectangle). This system is the safety net for a bare `PlayerVisual` with NO
/// worn identity — a minimal test/demo shell — so `sync_visuals` always has a
/// `Sprite` to query. The `Without<WornCharacter>` filter (a spawn-time fact, no
/// same-frame race) keeps the two systems from both claiming one entity.
pub fn ensure_player_visual_sprite(
    mut commands: Commands,
    players: Query<
        Entity,
        (
            With<PlayerVisual>,
            Without<Sprite>,
            Without<ambition_characters::actor::WornCharacter>,
        ),
    >,
) {
    for entity in &players {
        // Session teardown may despawn this entity before queued commands apply.
        commands.entity(entity).try_insert(Sprite::from_color(
            Color::srgba(0.18, 0.55, 1.0, 1.0),
            BVec2::ONE,
        ));
    }
}

/// The reusable selected-character presentation binder.
///
/// Observes the canonical simulation-owned [`WornCharacter`] identity on each
/// player body and installs the matching visual configuration — sprite sheet,
/// animation cursor ([`CharacterAnimator`]), feet [`Anchor`], the crouch-squash
/// [`PlayerSpriteBaseline`], and the [`PlayerSpriteCharacter`] marker recording
/// what is currently bound. It:
///
/// * binds when a player first appears (the marker is absent), and
/// * rebinds when the worn identity changes (marker id ≠ worn id), and replaces
///   the prior sheet-derived components instead of adding duplicates.
///
/// There is no per-character branch: every character resolves through the
/// `GameAssets` catalog. The shared animation plugin adds this system, so
/// `ambition_app` and standalone demos use the same path. With no `GameAssets` or
/// no sheet for the id, it installs the colored-rectangle fallback and still
/// marks the identity. [`ensure_player_visual_sprite`] only covers bare
/// `PlayerVisual`s with no identity.
pub fn bind_worn_character_presentation(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    players: Query<
        (
            Entity,
            &ambition_characters::actor::WornCharacter,
            Option<&PlayerSpriteCharacter>,
            Has<CharacterAnimator>,
            // This body's own standing size, from the read-model. See the seed
            // below.
            Option<&ambition_sim_view::BodyPoseView>,
        ),
        With<PlayerVisual>,
    >,
) {
    for (entity, worn, bound, has_sheet, base_size) in &players {
        // Resolve the sheet. No `GameAssets` and no sheet for the id both fall
        // through to the rectangle, so a worn player is always drawn.
        let asset = assets.as_ref().and_then(|a| a.characters.sheet(worn.id()));
        // Skip only when already correctly bound: same id, and a real sheet is
        // installed or none is available. A body on the fallback (no animator)
        // is tried again when its sheet appears, so a late asset is not lost.
        let already_bound = bound.map(|b| b.id.as_str()) == Some(worn.id());
        // The baseline goes stale after a form change, on purpose.
        // `sync_grown_form` swaps the identity in `FeatureInteraction` (9), and
        // `sync_sprite_posed_bodies` resizes the body in `WorldPrep` (2), so a
        // rebind on the swap frame records the old box. Re-seeding on a size
        // change does not help: every body that changes size with its form is
        // sheet-authored, and `sync_visuals` uses `authored_render` for it, not
        // the baseline. The bodies that read the baseline are the dev menu's
        // body-profile experiment, where a ratio other than one is the point.
        //
        // A sheet-backed presentation is final only when the pose exists.
        // `CharacterAnimator::render_basis` is set once, and the authored quad
        // and offset come from `BodyPoseView`. A bind before the pose would fix
        // the basis to a collision-derived guess, and nothing corrects it later,
        // because the worn identity does not change when the pose appears. So a
        // player with a sheet but no pose draws the provisional rectangle below.
        // It has no `CharacterAnimator`, so `has_sheet` is false and the next
        // pass binds for real.
        //
        // The pose geometry must also belong to the worn identity. On the tick a
        // form swap is decided, the pose names the new form but carries the old
        // box and offset (`ambition_sim_view::PoseGeometry`). A bound body keeps
        // its binding in that window; an unbound body draws the rectangle.
        let settled = base_size.filter(|pose| pose.geometry.is_settled());
        if base_size.is_some() && settled.is_none() && has_sheet {
            continue;
        }
        let sheet_bind = asset.zip(settled);
        if already_bound && (has_sheet || sheet_bind.is_none()) {
            continue;
        }
        if let Some((asset, pose)) = sheet_bind {
            // The baseline is the body's own standing size, not a constant.
            // `sync_visuals` scales the art by `base_size / standing_collision`
            // for the dev menu's body-profile experiment. A default-size seed
            // made that ratio differ from one for other bodies and stretched the
            // art. Each form has its own sheet, so growth swaps art and box size
            // and never scales the art. The dev experiment still works: it
            // changes `base_size` after the bind.
            let player_collision = pose.base_size;
            let (sprite, anchor, animator, baseline) = player_presentation_for_collision(
                asset,
                player_collision,
                pose.authored_render,
                pose.authored_offset,
            );
            let player_render = baseline.standing_render;
            // Log each bind: a visible resize has no other trace, and the line
            // names which bind site fired.
            eprintln!(
                "[sprite-bind] worn character '{}' collision={:.0}x{:.0} render={:.0}x{:.0} \
                 (seed: body baseline)",
                worn.id(),
                player_collision.x,
                player_collision.y,
                player_render.x,
                player_render.y,
            );
            // `try_insert`: the target is the player body, which session
            // teardown can despawn before these commands apply. Binding a sprite
            // to a body being destroyed has no meaning, so a silent failure is
            // correct. The alternative makes render own session lifecycle.
            commands.entity(entity).try_insert((
                sprite,
                anchor,
                animator,
                baseline,
                PlayerSpriteCharacter {
                    id: worn.id().to_string(),
                },
                // Without it the quality binder later in this same chain would see an unstamped
                // body and immediately rebuild what was just built.
                BoundSpriteQuality {
                    scale: asset.resolved_tier,
                },
            ));
        } else {
            // No sheet for this identity yet: draw the colored rectangle and
            // remove any sheet-derived components a prior identity installed.
            // This is also the provisional state of a body with a sheet but no
            // pose; the branch above upgrades it when the pose lands. With no
            // pose there is no measured size, so use the engine default.
            let player_collision = base_size.map(|pose| pose.base_size).unwrap_or(BVec2::new(
                ae::DEFAULT_PLAYER_BODY_WIDTH,
                ae::DEFAULT_PLAYER_BODY_HEIGHT,
            ));
            commands
                .entity(entity)
                // `try_*` for the same teardown reason as the bind above.
                .try_remove::<CharacterAnimator>()
                .try_remove::<bevy::sprite::Anchor>()
                .try_remove::<PlayerSpriteBaseline>()
                .try_remove::<BoundSpriteQuality>()
                .try_insert((
                    Sprite::from_color(Color::srgba(0.80, 0.95, 1.0, 1.0), player_collision),
                    PlayerSpriteCharacter {
                        id: worn.id().to_string(),
                    },
                ));
        }
    }
}

/// Restore the standing-frame sprite center for a compact native pose while
/// keeping the body's feet planted along its actual gravity axis.
///
/// The simulation moves the compact AABB center *down* by `dy` along gravity.
/// A native compact animation still renders in the full standing frame, so
/// presentation reverses that displacement. This must be vector-based: rooms
/// may use horizontal or diagonal gravity, not only screen-down/up gravity.
fn native_compact_render_pos(pos: ae::Vec2, gravity_dir: ae::Vec2, dy: f32) -> ae::Vec2 {
    let down = gravity_dir.normalize_or(ae::Vec2::new(0.0, 1.0));
    pos - down * dy
}

pub fn sync_visuals(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    primary_player: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    assets: Option<Res<GameAssets>>,
    feature_views: Res<FeatureViewIndex>,
    // The sim-built pose read-model (E4): render never reads the live `Body*`
    // clusters. Positions use the frame clock, which the camera also uses, so a
    // moving body does not shudder against the world.
    presented_features: Res<ambition_sim_view::PresentedFeaturePoses>,
    mut player_query: Query<
        (
            &mut Transform,
            &mut Sprite,
            Option<&PlayerSpriteBaseline>,
            Option<&CharacterAnimator>,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
            // Re-anchored per frame for a sheet-authored body: the anchor that
            // plants its feet depends on the current sheet.
            Option<&mut Anchor>,
        ),
        With<PlayerVisual>,
    >,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
    mut warned_unsized_player: Local<bool>,
    // `Option<Option<_>>`: the outer `None` means "never observed", which
    // differs from an observed `custom_size: None`. The first observation is
    // not a change.
    mut last_player_render_size: Local<Option<Option<BVec2>>>,
    // Entity `Transform::scale` and camera orthographic scale also multiply the
    // quad, and a transient in either looks like a sprite resize. Log them too.
    camera_view: ambition_sim_view::PresentedViewState,
    mut last_player_draw_scale: Local<Option<(BVec2, f32)>>,
) {
    let player = (primary_player.iter().count() == 1)
        .then(|| primary_player.iter().next())
        .flatten();
    if let Some(player) = player {
        if let Ok((mut transform, mut sprite, baseline, animator, pose, presented, anchor)) =
            player_query.get_mut(player)
        {
            let draw_pos = ambition_sim_view::presented_pose::draw_pos(pose, presented);
            transform.translation = world_to_bevy(&world.0, draw_pos, WORLD_Z_PLAYER);
            // Aerial roll (portal somersault / future gravity-room orientation).
            transform.rotation = Quat::from_rotation_z(pose.roll_angle);
            if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
                // Colored-rectangle fallback only — stretch to the collision-box
                // size and tint by flash. Textured sprites (atlas OR plain image)
                // keep their authored size and are tinted in the animation system.
                sprite.custom_size = Some(BVec2::new(pose.size.x, pose.size.y));
                let alpha = if pose.hit_flash_secs > 0.0 { 0.72 } else { 1.0 };
                sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
            } else if let Some(authored) = pose.authored_render {
                // The sheet authored this body's geometry: the quad is the frame
                // at the authored scale. The baseline branch below reconciles two
                // independent sizes by a ratio. Here box and quad come from one
                // number, so there is no ratio.
                sprite.custom_size = Some(BVec2::new(authored.x, authored.y));
                // Placement comes from the same publisher.
                // `sync_sprite_posed_bodies` computes the offset that puts the art
                // on the box, and the actor path reads it too. With the offset the
                // anchor is CENTER. Bevy y runs up and sheet y runs down, so y is
                // negated.
                //
                // Two mechanisms own this placement. The offset is computed for
                // the logical frame. For a body with an animator, `animate_player`
                // runs later in this stage and replaces `custom_size` and `Anchor`
                // from the animator's basis, but keeps this translation. They agree
                // only if that basis comes from the same authored quad with CENTER,
                // which `character_render_basis` provides. Do not suppress the
                // offset for animated bodies: that routes them to the baseline
                // branch, which has its own anchor rule. The error shows most where
                // a sheet's body sits far from its frame centre (Mary-O; see
                // `scripts/measure_sheet_body_offsets.py`).
                if let Some(offset) = pose.authored_offset {
                    transform.translation.x += offset.x;
                    transform.translation.y -= offset.y;
                    if let Some(mut anchor) = anchor {
                        if *anchor != Anchor::CENTER {
                            *anchor = Anchor::CENTER;
                        }
                    }
                } else if let (Some(animator), Some(mut anchor)) = (animator, anchor) {
                    // No published offset (a sheet that authors no body): the
                    // scale-invariant feet anchor is still the right answer.
                    let next = feet_anchor_for_render_size(
                        &animator.spec,
                        BVec2::new(pose.size.x, pose.size.y),
                        BVec2::new(authored.x, authored.y),
                    );
                    if *anchor != next {
                        *anchor = next;
                    }
                }
            } else if let Some(baseline) = baseline {
                // Body-profile experiment scale (live standing-profile swaps in
                // the development menu): render against the recorded startup
                // collision.
                let scale_x = pose.base_size.x / baseline.standing_collision.x.max(1.0);
                let scale_y = pose.base_size.y / baseline.standing_collision.y.max(1.0);
                if animator.is_some_and(|a| a.spec.maps(pose.anim)) {
                    // The sim lowered `pos` to the compact AABB's center to keep the feet
                    // planted — reverse exactly that shift so the standing-frame render puts
                    // its feet back on the same ground line.
                    sprite.custom_size = Some(BVec2::new(
                        baseline.standing_render.x * scale_x,
                        baseline.standing_render.y * scale_y,
                    ));
                    let dy = (pose.base_size.y - pose.size.y) * 0.5;
                    if dy > f32::EPSILON {
                        // Feet sit on the +gravity face (world +y is down under
                        // normal gravity); the standing center is `dy` opposite
                        // gravity from the compact center.
                        let render_pos = native_compact_render_pos(draw_pos, pose.gravity_dir, dy);
                        transform.translation = world_to_bevy(&world.0, render_pos, WORLD_Z_PLAYER);
                    }
                } else {
                    // HACK(crouch-sprite-row): when the player crouches (or
                    // morphs / crawls / slides) on a sheet WITHOUT a row for the
                    // pose, the fallback shows standing art while the engine
                    // shrinks the AABB and slides `pos.y` down to keep feet
                    // planted. Re-scale the sprite's vertical extent by the same
                    // ratio the collision shrunk; the normalized sprite anchor
                    // preserves foot alignment automatically. Retires per-row as
                    // generators emit real compact rows (the branch above) — see
                    // PlayerSpriteBaseline doc.
                    let base_y = pose.base_size.y.max(1.0);
                    let stance_ratio_y = (pose.size.y / base_y).clamp(0.1, 1.0);
                    sprite.custom_size = Some(BVec2::new(
                        baseline.standing_render.x * scale_x,
                        baseline.standing_render.y * scale_y * stance_ratio_y,
                    ));
                }
            } else if !*warned_unsized_player {
                // Every bind site inserts sprite and baseline together, so this
                // should not happen. Warn once instead of drawing the wrong size
                // silently.
                *warned_unsized_player = true;
                bevy::log::warn!(
                    target: "ambition_platformer2d::sprites",
                    "player sprite is textured but has no PlayerSpriteBaseline; \
                     custom_size is unset, so it renders at the atlas frame's native \
                     pixel size until a baseline arrives",
                );
            }

            // The bind sites log what they seeded; this logs what is drawn. A
            // size can change without a rebind, and a rebind can keep the size.
            // `None` means nothing set a size, so the quad draws at the atlas
            // frame's native pixel size.
            let describe = |size: Option<BVec2>| match size {
                Some(size) => format!("{:.0}x{:.0}", size.x, size.y),
                None => "NONE (draws at native frame size)".to_string(),
            };
            match *last_player_render_size {
                // First sighting: report the state, not a transition.
                None => {
                    *last_player_render_size = Some(sprite.custom_size);
                    eprintln!(
                        "[sprite-size] player first observed at {}",
                        describe(sprite.custom_size)
                    );
                }
                Some(previous) => {
                    // Sub-pixel drift is stance scaling doing its job, not a
                    // resize worth a line; crouching would otherwise emit one
                    // per frame.
                    let changed = match (previous, sprite.custom_size) {
                        (Some(before), Some(after)) => before.distance(after) > 0.5,
                        (before, after) => before.is_some() != after.is_some(),
                    };
                    if changed {
                        *last_player_render_size = Some(sprite.custom_size);
                        eprintln!(
                            "[sprite-size] player render size {} -> {}",
                            describe(previous),
                            describe(sprite.custom_size),
                        );
                    }
                }
            }

            // The two factors `custom_size` does not capture. A smaller
            // orthographic scale draws every quad bigger, so a camera that eases
            // out looks like a sprite that shrinks.
            let entity_scale = BVec2::new(transform.scale.x, transform.scale.y);
            let camera_scale = camera_view
                .get()
                .map(|view| view.orthographic_scale)
                .unwrap_or(1.0);
            let moved = match *last_player_draw_scale {
                None => {
                    eprintln!(
                        "[sprite-size] player draw scale first observed: \
                         entity={:.3}x{:.3} camera_ortho={camera_scale:.4}",
                        entity_scale.x, entity_scale.y,
                    );
                    false
                }
                Some((previous_entity, previous_camera)) => {
                    previous_entity.distance(entity_scale) > 1.0e-3
                        || (previous_camera - camera_scale).abs() > 1.0e-4
                }
            };
            if moved {
                if let Some((previous_entity, previous_camera)) = *last_player_draw_scale {
                    eprintln!(
                        "[sprite-size] player draw scale {:.3}x{:.3} @ortho \
                         {previous_camera:.4} -> {:.3}x{:.3} @ortho {camera_scale:.4}",
                        previous_entity.x, previous_entity.y, entity_scale.x, entity_scale.y,
                    );
                }
            }
            if moved || last_player_draw_scale.is_none() {
                *last_player_draw_scale = Some((entity_scale, camera_scale));
            }
        }
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut feature_query {
        let Some(view) = feature_views.get(&visual.id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        // Moving features use the frame clock, like the player. The quad centre
        // is the body centre plus the sheet's per-pose art offset, if any.
        let draw_pos = presented_features.presented(&visual.id, view.pos)
            + view.sprite_offset.unwrap_or(ae::Vec2::ZERO);
        transform.translation = world_to_bevy(&world.0, draw_pos, feature_z(view.kind));
        // Surface-walking enemies (PuppySlug) rotate the sprite so
        // its authored "up" axis aligns with the surface normal —
        // the slug crawls along walls / ceilings with its body
        // visibly clinging to them. All other actors stay axis-
        // aligned (rotation_rad = 0).
        transform.rotation = Quat::from_rotation_z(view.rotation_rad);

        // State-aware sprite swap for breakables and chests. Pickups are
        // chosen at spawn time and never change kind. Enemies are animated
        // through the character spritesheet path.
        if let Some(assets) = assets.as_deref() {
            if let Some(target_key) = state_aware_entity_sprite(view) {
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
                feature_color(view.kind, view.fighting, view.flash)
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

fn state_aware_entity_sprite(view: &ambition_sim_view::FeatureView) -> Option<EntitySprite> {
    match view.kind {
        FeatureVisualKind::Breakable => view
            .breakable_state
            .map(game_assets::breakable_state_sprite),
        FeatureVisualKind::Chest => Some(game_assets::chest_state_sprite(view.chest_opened)),
        // Switch shows its on/off button sprite (armed = on, disabled = off)
        // instead of a flat colored block (#57).
        FeatureVisualKind::Switch => Some(if view.switch_on {
            EntitySprite::SwitchArmed
        } else {
            EntitySprite::SwitchDisabled
        }),
        _ => None,
    }
}

/// Which quality tier the presentation on this entity was built from.
///
/// Spawned entities keep their cached image/atlas handles until a render system
/// overwrites them, so this is the only record of which art generation a body
/// shows. Binders compare it with the realization, not the setting: stamping
/// from the setting marks a body converged while it still draws old pixels.
///
/// It stores the resolved tier, not
/// [`requested_tier`](ambition_sprite_sheet::character::CharacterSpriteAsset::requested_tier).
/// A sheet with no baked variant answers `Half` with full-resolution bytes.
/// Keyed on the request, a rebind to identical pixels would look necessary. The
/// request belongs to the loader.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundSpriteQuality {
    pub scale: TextureResolutionScale,
}

/// Render-owned record of which catalog character id the controlled-body sprite
/// was bound from at presentation startup. The app writes it while crossing the
/// sim/render seam; quality reloads then preserve the same sheet without render
/// depending on the actor-side starting-character resource.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct PlayerSpriteCharacter {
    pub id: String,
}

// Every binder here stamps the resident realization (`asset.resolved_tier`).
// This file has no way to read the requested setting, so a new binder cannot
// stamp it by mistake. The requested tier lives in settings and the loader.

/// Which sprite upgrader owns this body.
///
/// A boss is also an actor, so its id is in `ActorRenderIndex` and
/// `BossRenderIndex`. `upgrade_boss_sprites` filters `Without<CharacterAnimator>`,
/// so if the actor path binds first, the boss sheet is never bound. System order
/// cannot fix this, and a `Without<BossAnimator>` filter cannot either: the boss
/// upgrader can skip frames while its image loads. The boss index claims the id,
/// so the boss path owns it.
pub fn actor_sprite_path_owns(id: &str, boss_render: &ambition_sim_view::BossRenderIndex) -> bool {
    boss_render.get(id).is_none()
}

/// Bind an actor's visual to its character sheet when the asset is available,
/// and rebind when its collision footprint or quality tier changes. One system
/// serves every actor (enemy, NPC, sandbag). Resolution: the actor's art
/// identity (a fighting-flipped NPC keeps its own sheet), then its display
/// name, against the shared character registry. An actor with no sheet draws
/// the placeholder rectangle.
pub fn upgrade_actor_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    // Readiness, not residency — see `texture_is_ready`.
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    feature_views: Res<FeatureViewIndex>,
    features: Query<(
        Entity,
        &FeatureVisual,
        Option<&BoundFeatureKind>,
        Option<&BoundSpriteQuality>,
    )>,
    // Materialized actor identity read-model (name / sprite-override / sandbag /
    // authored render size) — the renderer binds a sprite from this snapshot
    // WITHOUT borrowing gameplay_core's live actor clusters. Built by
    // `rebuild_actor_render_index` in the sim's `FeatureViewSync` set.
    actor_render: Res<ambition_sim_view::ActorRenderIndex>,
    // A boss is also an actor. Read this index only to yield; see
    // `actor_sprite_path_owns`.
    boss_render: Res<ambition_sim_view::BossRenderIndex>,
    // Names we've already warned about resolving no sprite, so the warning fires
    // once per offending name instead of every frame the actor is unbound.
    mut warned_sprite_names: Local<std::collections::HashSet<String>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound, bound_quality) in &features {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Actor) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        // Bound to the correct kind and collision footprint. The size check
        // catches rare runtime size changes; shark riders keep one scale across
        // mount and dismount. This is only half of "nothing to do": the quality
        // half needs the realization, so it is checked below.
        let kind_bound = bound.is_some_and(|b| b.matches(view.kind, view.size));
        // Identity decides which upgrader owns a body, not system order.
        if !actor_sprite_path_owns(&visual.id, &boss_render) {
            continue;
        }
        // Absent: the read-model has not seen this actor yet. Skip a frame; the
        // next rebuild fills it in.
        let Some(actor) = actor_render.get(&visual.id) else {
            continue;
        };
        // Do not bind from a body that is not whole yet (the same rule as
        // `bind_worn_character_presentation`). Until its prepared body is
        // granted, the body has its seed box and quad, and the binding builds
        // its render basis once from that geometry.
        if !actor.geometry.is_settled() {
            continue;
        }
        // Resolution order: the actor's art identity, then its display name. The
        // name stays so a direct `EnemySpawn` with no id still resolves a sheet.
        let art_identity = actor.sprite_character_id.as_deref();
        let actor_name = Some(actor.name.as_str());
        // Wait for a declared art identity's sheet; do not substitute. Sheets
        // load on demand, and the binding is keyed on kind and size, which the
        // late art does not change, so a substitute would stay. Only an
        // identity that no content declares falls back to the name.
        let own = art_identity.map(|n| assets.characters.sheet_state(n));
        if own
            .as_ref()
            .is_some_and(|state| state.declared_character_id().is_some())
        {
            continue;
        }
        let named = match own {
            Some(ambition_sprite_sheet::character::CharacterSheetState::Ready(asset)) => Some(asset),
            _ => actor_name.and_then(|n| assets.characters.sheet(n)),
        };
        let Some(character_asset) = named else {
            // An actor with no resolvable sheet draws the marked placeholder,
            // and the warning names the id, so missing art stays visible.
            if kind_bound {
                continue;
            }
            if let Some(missed) = actor_name {
                if warned_sprite_names.insert(missed.to_string()) {
                    // Name what the table knows, so a typo and an undecoded
                    // sheet read differently.
                    let diagnosis = match assets.characters.sheet_state(missed) {
                        ambition_sprite_sheet::character::CharacterSheetState::Declared {
                            character_id,
                        } => {
                            // `Declared` has two meanings: never realized, or
                            // retired by a quality change. They look the same;
                            // only `retired_tier` tells them apart.
                            match assets.characters.retired_tier(missed) {
                                Some(tier) => format!(
                                    "declared as '{character_id}' and RETIRED from {tier:?} — it \
                                     was decoded and then dropped by a quality transition, so this \
                                     is a re-realization that has not happened yet, not art \
                                     nobody asked for"
                                ),
                                None => format!(
                                    "declared as '{character_id}' but never materialized — no \
                                     realization of it has ever been resident, so nothing has \
                                     decoded its sheet"
                                ),
                            }
                        }
                        _ => "no loaded content declares this name — check for a typo or a \
                              decorated display name (\"Puppy Slug (ally)\"), or publish its art"
                            .to_string(),
                    };
                    bevy::log::warn!(
                        target: "ambition_platformer2d::sprites",
                        "actor '{missed}' resolved no sprite and is drawing the placeholder \
                         rectangle: {diagnosis}",
                    );
                }
            }
            continue;
        };
        // The other half of "nothing to do": this body's presentation was built
        // from a realization at the tier the table still holds.
        if kind_bound && bound_quality.is_some_and(|q| q.scale == character_asset.resolved_tier) {
            continue;
        }
        // Android loads assets out of the APK asynchronously, and missing or
        // platform-rejected images still have a Handle. Do not replace the
        // colored fallback with an atlas sprite until the texture is actually
        // present in Assets<Image>; otherwise a failed or delayed load renders
        // the NPC/enemy invisible.
        if !texture_is_ready(&asset_server, &images, &character_asset.texture) {
            // Diagnostic: log the asset path of each texture that blocks a bind.
            // Compare it with the handles the room barrier waited on. If they
            // differ, the barrier must wait on the textures the resident sheets
            // reference. Logged once per path, and only under
            // `AMBITION_PROFILE_CENSUS`, because this arm runs every frame for
            // every unbound body.
            //
            // Read the gate from the environment through a `OnceLock`, not from
            // the `RuntimeCensus` resource. A new system parameter makes a
            // missing resource a Bevy schedule panic in every composition that
            // does not provide it. An instrument must not change the signature
            // of the system it measures.
            {
                use std::collections::BTreeSet;
                use std::sync::{Mutex, OnceLock};
                static ON: OnceLock<bool> = OnceLock::new();
                static SEEN: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
                // Use the const, not a literal, so a rename in
                // `ambition_dev_tools` does not silently disable this.
                let on = *ON.get_or_init(|| {
                    std::env::var(ambition_dev_tools::runtime_census::CENSUS_ENV)
                        .map(|v| !v.is_empty() && v != "0")
                        .unwrap_or(false)
                });
                if on {
                    let path = asset_server
                        .get_path(character_asset.texture.id())
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "<no path: handle is not from a path>".to_owned());
                    let fresh = SEEN
                        .get_or_init(|| Mutex::new(BTreeSet::new()))
                        .lock()
                        .map(|mut seen| seen.insert(path.clone()))
                        .unwrap_or(false);
                    if fresh {
                        bevy::log::warn!(
                            target: "ambition_platformer2d::render",
                            "[texture-not-ready] {path} load_state={:?} — a body is \
                             unbound because THIS texture is not loaded with its \
                             dependencies, after the room barrier reported ready",
                            asset_server.get_load_state(character_asset.texture.id()),
                        );
                    }
                }
            }
            continue;
        }
        // Honor a shared sprite-metadata render size (e.g. a hostile-flipped
        // body-metrics NPC): render at the stored quad, NOT collision*scale,
        // so the sprite doesn't balloon once collision already equals the body.
        let (render_size, anchor) = character_render_basis(
            &character_asset.spec,
            collision,
            actor.render_size.map(|r| BVec2::new(r.x, r.y)),
            view.sprite_offset.map(|o| BVec2::new(o.x, o.y)),
        );
        let (sprite, anchor, animator) =
            build_character_presentation_with_render_size(character_asset, render_size, anchor);
        // The feet anchor plants the sprite's authored feet on the gravity-side
        // edge of the collision box. It rotates with the sprite, so a
        // surface-walker on a wall plants its contact edge (see
        // `update_enemy_actors`). The constructor seeds the full render basis
        // and applies frame-zero trim before the entity is drawable.
        // `try_insert`: `despawn_dead_dynamic_feature_visuals` removes these
        // entities when a feature's view goes away, which can happen on the
        // frame its sheet finishes decoding.
        commands.entity(entity).try_insert((
            sprite,
            anchor,
            animator,
            BoundFeatureKind::new(view.kind, collision),
            BoundSpriteQuality {
                scale: character_asset.resolved_tier,
            },
        ));
    }
}

/// Keep the controlled body drawn from the realization the table holds.
///
/// A deferred sheet finishing its decode and a quality change that retires a
/// realization are the same event here: this body shows a sheet that is not
/// current. Only presentation components change; no room entities are
/// despawned. The animator is rebuilt from the new asset instead of carrying an
/// atlas cursor across a different layout. The tier comparison is true while
/// the body is stale and false once it is not.
pub fn refresh_player_sprites_for_resident_quality(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    // Readiness, not residency — see `texture_is_ready`.
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    players: Query<
        (
            Entity,
            &ambition_sim_view::BodyPoseView,
            Option<&BoundSpriteQuality>,
            Option<&PlayerSpriteCharacter>,
        ),
        With<PlayerVisual>,
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, pose, bound_quality, character) in &players {
        // Rebind the sheet of whichever character the sprite was bound from. A
        // body no binder stamped has no sheet to refresh: naming one for it
        // (this once defaulted to `player_robot_v3`) invents an identity.
        let Some(start_id) = character.map(|c| c.id.as_str()) else {
            continue;
        };
        let Some(asset) = assets.characters.sheet(start_id) else {
            continue;
        };
        // A pose mid-swap carries the previous identity's geometry; the worn
        // binder rebinds once it settles, and this pass must not finalize first.
        if !pose.geometry.is_settled() {
            continue;
        }
        // Cheapest first: a body already built from this realization's tier is
        // current, and that is almost every body on almost every frame.
        if bound_quality.is_some_and(|q| q.scale == asset.resolved_tier) {
            continue;
        }
        if !texture_is_ready(&asset_server, &images, &asset.texture) {
            continue;
        }
        let collision = BVec2::new(pose.base_size.x, pose.base_size.y);
        let (sprite, anchor, animator, baseline) = player_presentation_for_collision(
            asset,
            collision,
            pose.authored_render,
            pose.authored_offset,
        );
        let render = baseline.standing_render;
        // Counterpart of the log in `bind_worn_character_presentation`. This
        // one fires when the resident realization changes.
        eprintln!(
            "[sprite-bind] rebind character '{}' collision={:.0}x{:.0} render={:.0}x{:.0} \
             tier={:?} (seed: live pose, trigger: resident realization moved)",
            start_id, collision.x, collision.y, render.x, render.y, asset.resolved_tier,
        );
        // `try_insert`: a provider switch in the same frame as a quality change
        // can despawn this visual's session scope.
        commands.entity(entity).try_insert((
            sprite,
            anchor,
            animator,
            baseline,
            BoundSpriteQuality {
                scale: asset.resolved_tier,
            },
        ));
    }
}

/// Rebind animated prop sprites in place after a quality-profile reload. Props
/// are room-scoped presentation entities, but they are not actor simulation
/// entities, so keeping this as a component overwrite avoids the v4-v6 class of
/// bugs where a visual refresh accidentally accumulated/despawned active room
/// content.
pub fn refresh_prop_sprites_on_game_assets_change(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    // Readiness, not residency — see `texture_is_ready`.
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    props: Query<(Entity, &PropVisual, Option<&BoundSpriteQuality>)>,
) {
    let Some(assets) = assets else {
        return;
    };
    // Stamp the resident realization's tier, not the requested setting, like
    // the actor path. The next frame then matches and the loop settles. There
    // is no `assets.is_changed()` early-out: images decode asynchronously, so
    // the frame `GameAssets` changes is not the frame the texture is ready. The
    // tier comparison is the convergence check, and it is cheap.
    for (entity, prop, bound_quality) in &props {
        let Some(asset) = assets.characters.prop_asset_for_kind(&prop.kind) else {
            continue;
        };
        if bound_quality.is_some_and(|q| q.scale == asset.resolved_tier) {
            continue;
        }
        if !texture_is_ready(&asset_server, &images, &asset.texture) {
            continue;
        }
        let bundle =
            crate::rendering::world::prop_sprite_bundle(prop.draw, prop.flip_y, asset, prop.size);
        commands.entity(entity).insert((
            bundle,
            BoundSpriteQuality {
                scale: asset.resolved_tier,
            },
        ));
    }
}

#[cfg(test)]
mod quality_convergence_tests;
#[cfg(test)]
mod worn_binder_tests;

#[cfg(test)]
mod compact_pose_tests {
    use super::native_compact_render_pos;
    use ambition_platformer2d_core as ae;

    fn assert_vec2_close(actual: ae::Vec2, expected: ae::Vec2) {
        assert!(
            (actual - expected).length() < 1.0e-5,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn native_compact_pose_reverses_the_shift_along_gravity() {
        let pos = ae::Vec2::new(10.0, 20.0);
        let dy = 6.0;
        assert_vec2_close(
            native_compact_render_pos(pos, ae::Vec2::new(0.0, 1.0), dy),
            ae::Vec2::new(10.0, 14.0),
        );
        assert_vec2_close(
            native_compact_render_pos(pos, ae::Vec2::new(0.0, -1.0), dy),
            ae::Vec2::new(10.0, 26.0),
        );
        assert_vec2_close(
            native_compact_render_pos(pos, ae::Vec2::new(1.0, 0.0), dy),
            ae::Vec2::new(4.0, 20.0),
        );
        assert_vec2_close(
            native_compact_render_pos(pos, ae::Vec2::new(-1.0, 0.0), dy),
            ae::Vec2::new(16.0, 20.0),
        );
        let diagonal = ae::Vec2::new(1.0, 1.0).normalize();
        assert_vec2_close(
            native_compact_render_pos(pos, diagonal, dy),
            pos - diagonal * dy,
        );
    }

    #[test]
    fn native_compact_pose_uses_screen_down_for_a_zero_gravity_vector() {
        assert_vec2_close(
            native_compact_render_pos(ae::Vec2::new(3.0, 9.0), ae::Vec2::ZERO, 2.0),
            ae::Vec2::new(3.0, 7.0),
        );
    }
}

#[cfg(test)]
mod render_basis_tests {
    use super::{character_render_basis, feet_anchor_for_render_size, sprite_render_size};
    use bevy::math::Vec2 as BVec2;
    use bevy::sprite::Anchor;

    fn spec() -> ambition_sprite_sheet::character::sheets::CharacterSheetSpec {
        ambition_sprite_sheet::character::sheets::try_load_spec_for_target(
            "robot",
            &Default::default(),
        )
        .expect("the robot sheet record resolves a spec")
    }

    /// A sheet that publishes an authored quad and an authored offset draws at
    /// that quad with a CENTER anchor, because the offset moves the art onto
    /// the body.
    #[test]
    fn an_authored_quad_with_an_offset_is_centred() {
        let spec = spec();
        let collision = BVec2::new(28.0, 40.0);
        let authored = BVec2::new(60.95238, 73.14286);
        let (render, anchor) = character_render_basis(
            &spec,
            collision,
            Some(authored),
            Some(BVec2::new(0.76, -19.81)),
        );
        assert_eq!(
            render, authored,
            "an authored quad is drawn at its authored size"
        );
        assert_eq!(
            anchor,
            Anchor::CENTER,
            "the offset carries the body, so the quad centres"
        );
    }

    /// The same authored quad without an offset keeps the feet anchor, so the
    /// offset selects the arm, not the quad. Both tests together catch a change
    /// that merges the arms.
    #[test]
    fn an_authored_quad_without_an_offset_keeps_its_feet_anchor() {
        let spec = spec();
        let collision = BVec2::new(28.0, 40.0);
        let authored = BVec2::new(60.95238, 73.14286);
        let (render, anchor) = character_render_basis(&spec, collision, Some(authored), None);
        assert_eq!(render, authored);
        assert_eq!(
            anchor,
            feet_anchor_for_render_size(&spec, collision, authored)
        );
        assert_ne!(
            anchor,
            Anchor::CENTER,
            "this arm must NOT be the centred one"
        );
    }

    /// No authored quad: both numbers come from the collision box, which is what every
    /// sheet that authors no body has always done.
    #[test]
    fn no_authored_quad_derives_both_from_the_collision_box() {
        let spec = spec();
        let collision = BVec2::new(28.0, 40.0);
        let (render, anchor) = character_render_basis(&spec, collision, None, None);
        let expected = sprite_render_size(&spec, collision);
        assert_eq!(render, expected);
        assert_eq!(
            anchor,
            feet_anchor_for_render_size(&spec, collision, expected)
        );
    }
}
