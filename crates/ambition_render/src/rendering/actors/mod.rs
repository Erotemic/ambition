//! Per-frame actor sprite and animation presentation.

use ambition_platformer2d_core as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::primitives::{
    FeatureVisual, PlayerSpriteBaseline, PlayerVisual, PropVisual, feature_color, feature_z,
    switch_on_color,
};
use ambition_persistence::settings::TextureResolutionScale;
use ambition_platformer2d_core::config::{WORLD_Z_PLAYER, world_to_bevy};
use ambition_platformer2d_shared_tangle::feature_kind::{BoundFeatureKind, FeatureVisualKind};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_sim_view::FeatureViewIndex;
use ambition_sprite_sheet::character::{
    CharacterAnimator, build_character_presentation_with_render_size,
    feet_anchor_for_render_size, sprite_render_size,
};
use ambition_sprite_sheet::game_assets::{self, EntitySprite, GameAssets};

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
/// * rebinds when the worn identity changes (marker id ≠ worn id), REPLACING the
///   prior sheet-derived components rather than layering duplicates.
///
/// There is no per-character branch — every character resolves through the
/// same `GameAssets` catalog lookup, so a new character needs zero code here.
/// Owned by `ambition_render` (the lowest reusable presentation crate) and added
/// by the shared animation plugin, so `ambition_app` AND standalone demos consume
/// the identical path; neither binds the player sprite itself. With no `GameAssets`
/// (a demo shell that ships no art) OR an id with no sheet, it installs the
/// colored-rectangle fallback and still marks the identity — this system OWNS
/// every `WornCharacter` player's presentation, so [`ensure_player_visual_sprite`]
/// only backstops bare `PlayerVisual`s that carry no identity at all.
pub fn bind_worn_character_presentation(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    players: Query<
        (
            Entity,
            &ambition_characters::actor::WornCharacter,
            Option<&PlayerSpriteCharacter>,
            Has<CharacterAnimator>,
            // This body's OWN standing size, from the READ-MODEL. See the seed
            // below.
            //
            // `BodyPoseView:base_size` is the same number where the component exists, and where
            // it does NOT the view falls back to the body's CURRENT `size` while the old code
            // fell back to the engine's default player size.
            Option<&ambition_sim_view::BodyPoseView>,
        ),
        With<PlayerVisual>,
    >,
) {
    for (entity, worn, bound, has_sheet, base_size) in &players {
        // Seed the baseline from the BODY, not from a constant.
        //
        // `PlayerSpriteBaseline::standing_collision` is the reference the render
        // scales the art against (`base_size / standing_collision` in
        // `sync_visuals`), and that ratio exists for ONE reason: the dev menu's
        // live body-profile experiment. Seeding it with the default player size
        // meant the ratio was also non-1 for any body that is simply not the
        // default size — so Mary-O growing to her tall collider stretched the
        // tall sheet's art by 1.5 (`render size 70x84 -> 70x125`) instead of
        // just drawing the tall art at the tall size.
        //
        // Her forms have their own SHEETS. Growing should swap which art is
        // drawn and how big her box is; it should never scale the art. Binding
        // against the body's real baseline makes the ratio 1 for every form,
        // and leaves the dev experiment working — that changes `base_size`
        // after the bind, which is exactly the deviation the scale is for.
        let player_collision = base_size.map(|pose| pose.base_size).unwrap_or(BVec2::new(
            ae::DEFAULT_PLAYER_BODY_WIDTH,
            ae::DEFAULT_PLAYER_BODY_HEIGHT,
        ));
        // Resolve the sheet — absent `GameAssets` (art-free demo) and an id with no
        // sheet both fall through to the rectangle, so a worn player is ALWAYS drawn.
        let asset = assets.as_ref().and_then(|a| a.characters.sheet(worn.id()));
        // Skip only when already CORRECTLY bound: same id AND either a real sheet is
        // installed or none is available to upgrade to. A body sitting on a fallback
        // (marker matches but no animator) is re-attempted once its sheet appears, so
        // an asset that loads AFTER the first bind is not lost.
        let already_bound = bound.map(|b| b.id.as_str()) == Some(worn.id());
        if already_bound && (has_sheet || asset.is_none()) {
            continue;
        }
        if let Some(asset) = asset {
            let player_render = sprite_render_size(&asset.spec, player_collision);
            let anchor = feet_anchor_for_render_size(&asset.spec, player_collision, player_render);
            let (sprite, anchor, animator) =
                build_character_presentation_with_render_size(asset, player_render, anchor);
            // A visible sprite RESIZE mid-launch has no other trace: nothing
            // else records that the quad changed size, or which of the two
            // bind sites seeded it. Both seed `standing_collision` differently
            // (this one from the default body constant, the assets-changed
            // rebind from the live pose) and the render size is not linear in
            // collision, so knowing WHICH bound is the difference between
            // diagnosing this and guessing.
            eprintln!(
                "[sprite-bind] worn character '{}' collision={:.0}x{:.0} render={:.0}x{:.0} \
                 (seed: body baseline)",
                worn.id(),
                player_collision.x,
                player_collision.y,
                player_render.x,
                player_render.y,
            );
            // `try_insert`, not `insert`: this binder is deferred (Commands) and
            // its target is the PLAYER BODY, which session teardown despawns. A
            // provider switch therefore has one frame where the body is going
            // away and this pass is still decorating it, and whether the insert
            // or the despawn flushes first is decided by system ordering rather
            // than by anything either system knows.
            //
            // Failing silently is CORRECT here, not a papering-over: binding a
            // sprite onto a body that is being destroyed has no meaning, and the
            // alternative — ordering presentation around teardown — makes the
            // render layer responsible for session lifecycle.
            commands.entity(entity).try_insert((
                sprite,
                anchor,
                animator,
                PlayerSpriteBaseline {
                    standing_render: player_render,
                    standing_collision: player_collision,
                },
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
            // No sheet for this identity: draw the colored-rectangle fallback and
            // strip any sheet-derived presentation a PRIOR identity installed, so a
            // rebind never leaves a stale animator/anchor/baseline behind.
            commands
                .entity(entity)
                // Same reasoning as the bind above: the whole chain targets a
                // body that may be mid-teardown, and a `remove` on a despawned
                // entity fails exactly like an `insert` does.
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
    // The sim-built pose read-model (E4): position / roll / stance / flash
    // facts resolved in `FeatureViewSync`; render never touches the live
    // `Body*` clusters.
    // Frame-clock positions for everything the sim publishes per tick. The
    // camera frames these same values; sampling a different clock here is what
    // made a moving body shudder against a stable world.
    presented_features: Res<ambition_sim_view::PresentedFeaturePoses>,
    mut player_query: Query<
        (
            &mut Transform,
            &mut Sprite,
            Option<&PlayerSpriteBaseline>,
            Option<&CharacterAnimator>,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
            // Re-anchored per frame for a sheet-authored body: its quad is the
            // whole frame at an authored scale, so the anchor that plants its
            // feet is a fact about the SHEET and moves when the sheet does.
            Option<&mut Anchor>,
        ),
        With<PlayerVisual>,
    >,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
    mut warned_unsized_player: Local<bool>,
    // Option<Option<_>>, and the nesting is the point: the OUTER None means
    // "never observed", which is not the same fact as an observed
    // `custom_size: None`. Collapsing them made the first observation of a
    // perfectly correct sprite report a NONE -> 75x75 transition that never
    // happened -- the player entity does not exist until its room loads, so
    // the first observation is not a change.
    mut last_player_render_size: Local<Option<Option<BVec2>>>,
    // The other two multipliers between `custom_size` and what a player SEES.
    //
    // So the sprite's own size was never wrong, and the instrument's silence RULED OUT the two
    // hypotheses it was built for rather than confirming either.
    //
    // What is left is everything else on the path to pixels: the entity's own
    // `Transform:scale`, and the camera's orthographic scale. Both multiply the same quad, and
    // a transient in either reads exactly like a sprite resize. Watching the drawn size instead
    // of one of its factors is the difference between an instrument that can only confirm a
    // guess and one that can localise. Was `Res<CameraViewState>`, a process-global that with
    // two views could not say whose framing this is.
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
                // The SHEET authored this body's geometry, so there is
                // nothing here to compute: the quad is the frame at the authored
                // scale, produced beside the collision box from that one number.
                //
                // This branch exists because the one below cannot express it. `standing_render *
                // (base_size / standing_collision)` is a guess about the art CORRECTED by how far
                // the box has drifted from a baseline — two independent quantities reconciled by a
                // ratio. Here the box and the quad are two readings of one number, so there is no
                // ratio and nothing to double-count.
                sprite.custom_size = Some(BVec2::new(authored.x, authored.y));
                // and the PLACEMENT comes from the same publisher as the
                // size, rather than being re-derived here. A sheet frame is not
                // its character: the art sits somewhere inside the frame, usually
                // off-centre, so a quad centred on the body draws the character
                // wherever the padding happens to put it.
                // `sync_sprite_posed_bodies` computes the offset that puts the
                // ART on the BOX, and the actor path has always read it — this
                // branch instead recomputed a feet anchor from
                // `feet_anchor_norm`, which is a SECOND derivation of one fact
                // and disagreed with the first (for v3, ~1 px vertically and
                // ~2.5 px horizontally, because his `feet_pixel.y` is 157 against
                // a box bottom of 158 and his authored box is centred on 114.5
                // against a frame centre of 112).
                //
                // the anchor becomes CENTER and the offset moves the quad,
                // which is what the actor path does. Sheet pixel space and world
                // space share +y down, but Bevy's UI/render y runs UP — hence the
                // negated y, the same conversion `sync_sprite_posed_bodies`
                // documents at its own seam.
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
                // should be unreachable. Say so out loud rather than silently
                // rendering the wrong size: if the line never appears, the
                // launch-time resize is the two bind sites disagreeing instead,
                // and that is worth knowing just as much.
                *warned_unsized_player = true;
                bevy::log::warn!(
                    target: "ambition_platformer2d::sprites",
                    "player sprite is textured but has no PlayerSpriteBaseline; \
                     custom_size is unset, so it renders at the atlas frame's native \
                     pixel size until a baseline arrives",
                );
            }

            // The bind sites report what they SEEDED; this reports what is
            // actually drawn. A visible mid-launch resize is a change here,
            // and the two need not agree: a size can change without a rebind
            // (pose/stance scaling) and a rebind can leave the size identical.
            // `None` is its own event — it means nothing assigned a size and
            // the quad falls back to the atlas frame's native pixel size.
            let describe = |size: Option<BVec2>| match size {
                Some(size) => format!("{:.0}x{:.0}", size.x, size.y),
                None => "NONE (draws at native frame size)".to_string(),
            };
            match *last_player_render_size {
                // First sighting of this player. Report the state, not a
                // transition, and say which it is -- an opening NONE is a
                // genuine finding, an opening 75x75 is a healthy sprite.
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

            // The two factors `custom_size` does NOT capture.
            //
            // Camera scale is the divisor: a smaller orthographic scale shows
            // less world in the same viewport, which draws every quad bigger. So
            // a camera that opens zoomed-in and eases out presents as "the
            // character flashed large and then shrank" while every sprite size in
            // the game is constant.
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
        // Patrolling enemies and moving props step on the tick clock exactly as
        // the player body does, so they get the same frame-clock treatment.
        // The QUAD's centre: the body's centre, plus whatever the sheet says
        // about where this pose's art sits inside the frame. Absent for every
        // feature that publishes no placement, so this is the identity
        // everywhere it always was.
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

/// Which quality tier the presentation currently on this entity was built from.
///
/// Already-spawned entities keep their cached image/atlas handles until a render
/// system overwrites those components, so this is the only record of which
/// generation of the art a body is actually SHOWING.
///
/// Stamping from the setting marks a body converged while it is still drawing old pixels, and
/// then it is never revisited. Comparing against the realization asks the only question with an
/// answer: *is this body drawn from the sheet the table currently holds?*
///
/// RESOLVED, not
/// [`requested_tier`](ambition_sprite_sheet::character::CharacterSpriteAsset::requested_tier),
/// and the question decides it. This component is a statement about PIXELS —
/// which generation of the art is on screen — so it must move exactly when the
/// pixels do. A sheet with no baked variant answers `Half` with full-resolution
/// bytes; keyed on the request, a rebind to byte-identical pixels would look
/// necessary. The request is the convergence key and it belongs to the loader,
/// not to a presentation binder.
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

// Every other binding site here asks the resident realization (`asset.resolved_tier`) instead.
//
//  removing it makes the rule structural rather than remembered: there is no
// longer a way to reach for the requested setting from this file, so the next
// binder cannot repeat the mistake by picking the convenient helper. The
// requested tier still exists where it belongs — in the settings and in the
// loader that resolves it — it just is not something a PRESENTATION binder may
// stamp as fact.

/// Bind an actor's visual to its character sheet once the asset is available —
/// and re-bind when its collision footprint or the quality scale changes. ONE
/// system for EVERY actor (enemy, NPC, sandbag): the enemy/NPC split was never a
/// render type, so it collapsed with `FeatureVisualKind`. Resolution is
/// name-first — an authored sprite-override label (a fighting-flipped NPC keeps
/// its own sheet), then the actor's own display name, against the shared
/// character registry — then a STATE-keyed fallback: a sandbag renders the
/// sandbag sheet, a fighting actor the generic enemy sheet, and a peaceful
/// un-registered actor keeps its terminal-rectangle placeholder.
/// Which sprite upgrader owns this body.
///
/// A boss is also an actor — post-unification there is one body vocabulary — so a boss's id is
/// in `ActorRenderIndex` *and* `BossRenderIndex`. `upgrade_boss_sprites` is filtered
/// `Without<CharacterAnimator>`, so it then skipped that boss forever and its dedicated sheet
/// was never bound. Every boss in the game drew a generic body.
///
/// System ORDER cannot fix that (swapping them just moves the overwrite), and a
/// `Without<BossAnimator>` filter cannot either (the boss upgrader legitimately
/// skips a frame while its image loads, and the actor path would claim it in the
/// gap). The read-model is the answer: the boss index claims the id, so the boss
/// path owns it.
pub fn actor_sprite_path_owns(id: &str, boss_render: &ambition_sim_view::BossRenderIndex) -> bool {
    boss_render.get(id).is_none()
}

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
    // A boss is ALSO an actor (post-unification), so its id appears in BOTH render
    // read-models. This one is read to YIELD, never to bind — see
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
        // Bound to the correct kind and collision footprint. The collision-size
        // check is still useful for rare intentional runtime size changes, but
        // shark riders should normally keep the same visual/collision scale
        // across mount and dismount.
        //
        // this is only HALF of "nothing to do": the quality half cannot be answered until the
        // realization is in hand, so the early-out moves below the lookup.
        let kind_bound = bound.is_some_and(|b| b.matches(view.kind, view.size));
        // IDENTITY decides which upgrader owns a body, not which one ran first.
        if !actor_sprite_path_owns(&visual.id, &boss_render) {
            continue;
        }
        // Read the actor's materialized identity snapshot. Absent  the read-model
        // hasn't caught this actor yet (it just spawned); skip a frame — the next
        // rebuild fills it in, exactly like the `feature_views` miss above.
        let Some(actor) = actor_render.get(&visual.id) else {
            continue;
        };
        // Resolution order, shared by every actor: an authored sprite-override
        // label (a fighting-flipped NPC keeps its own sheet — the Kernel Guide
        // migration is the one that leaves it blank so kernel→goblin keeps its
        // visual gag), then the actor's ART IDENTITY, then its display name.
        //
        // the display name stays LAST rather than being deleted. A direct
        // `EnemySpawn` with no id still resolves by name — intro raiders pick up
        // their sheet without a duplicate enemy-side registry entry.
        let override_name = actor.sprite_override_name.as_deref();
        let art_identity = actor.sprite_character_id.as_deref();
        let actor_name = Some(actor.name.as_str());
        let named = override_name
            .and_then(|n| assets.characters.sheet(n))
            .or_else(|| art_identity.and_then(|n| assets.characters.sheet(n)))
            .or_else(|| actor_name.and_then(|n| assets.characters.sheet(n)));
        let Some(character_asset) = named else {
            // An actor whose own sheet does not resolve draws the marked placeholder rectangle,
            // everywhere, and the binding report names the id.
            //
            // That made missing art invisible — a body with no sprite of its own looked like a
            // deliberate goblin, so nobody ever went and drew it. Ambition's own enemies visibly
            // regress until each gets art, which is the point: it turns silent debt into visible
            // work.
            if kind_bound {
                continue;
            }
            if let Some(missed) = override_name.or(actor_name) {
                if warned_sprite_names.insert(missed.to_string()) {
                    // Name what the table actually knows, so a TYPO and an
                    // undecoded sheet stop reading as the same problem.
                    let diagnosis = match assets.characters.sheet_state(missed) {
                        ambition_sprite_sheet::character::CharacterSheetState::Declared {
                            character_id,
                        } => {
                            // ⛔⛔ `Declared` MEANS TWO DIFFERENT THINGS and this
                            // line used to assert one of them. Its own type doc
                            // says so — "either it never has, or its realization
                            // was retired by a quality change" — and retiring
                            // leaves the declaration standing, so the two states
                            // are identical to look at. The old text read
                            // "nothing demanded it, so the engine never decoded
                            // its sheet", which for a retired sheet is false
                            // twice over: it was demanded, and it WAS decoded.
                            // That warning fired 111 times on one Hall reveal, so
                            // it was the main evidence about a cause it had
                            // guessed. `retired_tier` is the only thing that can
                            // tell them apart.
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
            // ⛔⛔ THE HALL'S ULTRA BURST IS 129 BODIES STOPPING HERE. The cover
            // lifts and then, ~370 ms later, all 129 report the unclaimed-body
            // placeholder in ONE frame — so the barrier released on bodies whose
            // textures cannot bind. `assets.characters.sheet(..)` resolves for
            // every one of them ("resolved no sprite" prints zero times), which
            // leaves this arm: the sheet's texture is not
            // `is_loaded_with_dependencies` even though the barrier's own
            // manifest was satisfied.
            //
            // ⇒ WHAT THIS PRINTS IS THE SET DIFFERENCE: the asset PATH the
            // resident sheet actually references, against the handles the
            // barrier waited on. If they differ (per-character sheet vs pack
            // page) the fix is that the barrier's readiness set must be the
            // textures the RESIDENT sheets reference, not the manifest's
            // pre-resolution list.
            //
            // Once per PATH, behind `AMBITION_PROFILE_CENSUS`: this arm runs on
            // every frame of the ramp for every unbound body, and unthrottled it
            // is tens of thousands of lines that push the reveal itself out of
            // the log.
            //
            // ⚠ The gate is read from the environment through a `OnceLock`, NOT
            // from the `RuntimeCensus` resource, deliberately: adding a system
            // parameter to a shipped system is what turns a missing resource
            // into a Bevy 0.19 SCHEDULE PANIC in every composition that does not
            // provide it, which cost this repo 37 failures in one union run.
            // An instrument may not change the signature of the system it
            // instruments.
            {
                use std::collections::BTreeSet;
                use std::sync::{Mutex, OnceLock};
                static ON: OnceLock<bool> = OnceLock::new();
                static SEEN: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
                // The CONST, not the literal: the name lives in
                // `ambition_dev_tools` (already a dependency) and a literal here
                // would keep compiling after that name changed, leaving an
                // instrument that is simply never on.
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
        let render_size = actor.render_size.map(|r| BVec2::new(r.x, r.y));
        let (render_size, anchor) = match render_size {
            // This body publishes where its quad goes, per pose (the sheet's
            // per-animation body rectangle). That placement already puts the
            // art's feet on the box's gravity face for the pose being shown, so
            // the quad is CENTRED and `sync_visuals` does the shifting. Stacking
            // the sheet's one static feet anchor on top would double-count it —
            // and that anchor is derived from the idle frame, which is precisely
            // the wrong answer for a body that changes silhouette.
            Some(render_size) if view.sprite_offset.is_some() => (render_size, Anchor::CENTER),
            Some(render_size) => (
                render_size,
                feet_anchor_for_render_size(&character_asset.spec, collision, render_size),
            ),
            None => {
                let render_size = sprite_render_size(&character_asset.spec, collision);
                (
                    render_size,
                    feet_anchor_for_render_size(&character_asset.spec, collision, render_size),
                )
            }
        };
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(
            character_asset,
            render_size,
            anchor,
        );
        // The feet anchor plants the sprite's authored feet (`feet_anchor_y` from
        // sprite metadata) on the gravity-side edge of the collision box. It is a
        // 1-D anchor that rotates WITH the sprite, so for a surface-walker clung to
        // a wall it correctly plants the contact edge once the collision box itself
        // is oriented (see `update_enemy_actors`). No per-family special-casing.
        // The constructor seeds the full logical render basis and applies frame-zero
        // trim before this entity becomes drawable; later animation ticks reuse it.
        // `try_insert`: REPRODUCED, and the same shape as the boss
        // twin — these are `FeatureVisual` entities, which
        // `despawn_dead_dynamic_feature_visuals` retires the moment a feature's
        // view disappears. An actor dying on the frame its sheet finishes
        // decoding is the ordinary way to hit it.
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
/// Deferred sheets finishing their decode, and a confirmed quality change
/// retiring a realization for one at another tier, are the same event seen from
/// here: the sheet behind this character is not the sheet this body is showing.
/// Intentionally component-local — no room entities are despawned and the
/// gameplay/body components are untouched. The animator is rebuilt from the new
/// asset, restoring the spawn-time animation invariants rather than trying to
/// carry an atlas cursor across a different texture and layout.
///
/// The condition that remains is a comparison against the realization's own tier, which is true for
/// as long as the body is stale and stops being true the moment it is not.
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
        // Rebind the sheet of whichever character the sprite was originally
        // bound from. If an old test fixture lacks the marker, fall back to the
        // content default id used by the base sandbox catalog.
        let start_id = character
            .map(|c| c.id.as_str())
            .unwrap_or("player_robot_v3");
        let Some(asset) = assets.characters.sheet(start_id) else {
            continue;
        };
        // Cheapest first: a body already built from this realization's tier is
        // current, and that is almost every body on almost every frame.
        if bound_quality.is_some_and(|q| q.scale == asset.resolved_tier) {
            continue;
        }
        if !texture_is_ready(&asset_server, &images, &asset.texture) {
            continue;
        }
        let collision = BVec2::new(pose.base_size.x, pose.base_size.y);
        let render = sprite_render_size(&asset.spec, collision);
        // The counterpart line to the one in `bind_worn_character_presentation`.
        // This one fires when the RESIDENT realization moved — a deferred sheet
        // landing, or a quality transition — so a size that differs from the
        // earlier bind is the visible mid-launch pop, timestamped.
        eprintln!(
            "[sprite-bind] rebind character '{}' collision={:.0}x{:.0} render={:.0}x{:.0} \
             tier={:?} (seed: live pose, trigger: resident realization moved)",
            start_id, collision.x, collision.y, render.x, render.y, asset.resolved_tier,
        );
        // `try_insert`: REPRODUCED. Same `PlayerVisual` target as the
        // bare-player safety net, reached on a very different frame — a
        // confirmed quality-profile switch rebuilds `GameAssets`, and a provider
        // switch in the same frame despawns the session scope this visual
        // belongs to.
        let anchor = feet_anchor_for_render_size(&asset.spec, collision, render);
        let (sprite, anchor, animator) =
            build_character_presentation_with_render_size(asset, render, anchor);
        commands.entity(entity).try_insert((
            sprite,
            anchor,
            animator,
            PlayerSpriteBaseline {
                standing_render: render,
                standing_collision: collision,
            },
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
    // THE STAMP IS THE RESIDENT REALIZATION'S TIER, NEVER THE REQUESTED
    // SETTING.
    //
    // The actor path forty lines up (`BoundSpriteQuality { scale: asset.resolved_tier }`)
    // always did this correctly; the two disagreed inside one file. Asking the
    // asset is also what makes the comparison self-limiting: once stamped from
    // `asset.resolved_tier`, the next frame matches and the loop settles.
    //
    // The difference is that a stale prop is now VISIBLE to whoever fixes that, instead of claiming
    // to be up to date.
    //
    // the `assets.is_changed()` early-out is gone with it, for the reason the
    // actor path dropped its own: images decode asynchronously, so the frame
    // `GameAssets` changes is not the frame the texture is usable. The
    // tier comparison below is the convergence check and it is cheap.
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
