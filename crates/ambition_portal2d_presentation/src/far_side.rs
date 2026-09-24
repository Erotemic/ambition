//! Draw a far-side body as the part of it the pane does not cover.
//!
//! A pane draws at [`crate::PORTAL_WINDOW_Z`] (`9.5`) and an actor at
//! `WORLD_Z_DUMMY + 1.0` or `WORLD_Z_PLAYER`. Every actor therefore wins the
//! depth test against every pane, and a body behind an aperture shows through
//! the captured image that should hide it.
//!
//! The fix is structural, not a z ordering. [`crate::uncovered_remainder`]
//! returns the part the pane does not hide, and only those pieces are drawn.
//! A higher window z inverts the bug onto near-side bodies. One actor z cannot
//! serve two panes that disagree in the same frame.
//!
//! [`PortalViewer`] is a resource, so there is exactly one viewpoint.
//! Split-screen would need per-view pieces on per-view render layers.
//!
//! The pieces carry no `RenderLayers` because actor sprites do not. The
//! per-view isolation pass writes layers only onto `PresentedForView` things
//! (labels, backdrop panels, plates). The transit pieces make the same
//! assumption.
//!
//! Only one covering pane is subtracted. Two apertures would need more than
//! the three clip half-planes that [`PortalClipMaterial`] carries. In the
//! shipped worlds two panes are never close enough to both cover one body
//! (see `scripts/portal_pane_separation.py`). When more than one pane covers a
//! body, the first by [`ambition_portal2d::stable_portal_order`] is used, so the
//! result does not depend on query order.

use ambition_platformer2d_core::Vec2;
use ambition_portal2d::PlacedPortal;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial, CLIP_PLANE_OFF,
};
use crate::{PortalCompositingCandidate, PortalViewer, PortalWorldFrame};
use ambition_sprite_fx::DeclaredFrame;

/// One drawn fragment of a far-side body. Rebuilt every frame from the source
/// sprite, so it can never drift from what the sprite currently looks like.
#[derive(Component)]
pub struct PortalFarSidePiece;

/// This system withdrew this body's whole-sprite draw and will give it back.
///
/// Visibility is not this system's fact to own. A body can be hidden for other
/// reasons (death, culling, a cutscene, an editor toggle). The marker records
/// what this system did, so it reverses only that. A body it never hid is
/// never touched.
#[derive(Component)]
pub struct PortalFarSideHidden;

/// Hide each far-covered body and redraw the part the pane leaves visible.
///
/// Rebuilt wholesale each frame rather than diffed: the source sprite's frame,
/// flip and pose all change under the animator, and a cached piece is a second
/// copy of facts that already have an owner.
pub fn composite_far_side_bodies(
    mut commands: Commands,
    frame: Res<PortalWorldFrame>,
    stale: Query<Entity, With<PortalFarSidePiece>>,
    hidden: Query<Entity, With<PortalFarSideHidden>>,
    portals: Query<&PlacedPortal>,
    viewer: Option<Res<PortalViewer>>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    clip_materials: Option<ResMut<Assets<PortalClipMaterial>>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
    mut candidates: Query<(
        Entity,
        &PortalCompositingCandidate,
        // A `Sprite` is read for its frame; a non-sprite drawable declares its
        // frame. Both reach the same piece builder, so a `Mesh2d` overlay
        // composites like a sprite.
        Option<&Sprite>,
        Option<&DeclaredFrame>,
        Option<&Anchor>,
        // The local `Transform`, like the candidate publisher. `GlobalTransform`
        // updates only in `PostUpdate`, so pieces would use last frame's pose
        // and lag behind a moving body.
        &Transform,
        Option<&ambition_portal2d::PortalTransit>,
    )>,
) {
    for entity in &stale {
        commands.entity(entity).despawn();
    }

    // Near and far are relative to a viewpoint. Without one, every body
    // draws as it did before.
    let Some(viewer) = viewer.filter(|v| v.present) else {
        restore_hidden(&mut commands, &hidden, &mut candidates);
        return;
    };
    let (Some(images), Some(layouts), Some(mut meshes), Some(mut materials)) =
        (images, layouts, meshes, clip_materials)
    else {
        // Headless, or a host that never registered the material: the
        // pre-compositing behaviour is kept rather than a body vanishing.
        restore_hidden(&mut commands, &hidden, &mut candidates);
        return;
    };

    let mut panes: Vec<PlacedPortal> = portals.iter().cloned().collect();
    panes.sort_by(ambition_portal2d::stable_portal_order);

    let mesh = unit_mesh
        .get_or_insert_with(|| meshes.add(Rectangle::default()))
        .clone();

    for (entity, candidate, sprite, declared, anchor, transform, transit) in &mut candidates {
        let min = candidate.drawn_centre - candidate.drawn_half;
        let max = candidate.drawn_centre + candidate.drawn_half;

        // The first covering pane in the stable order (see the module note).
        // Pass the transit flag: a straddling body is already drawn as two
        // slices by `sync_portal_body_pieces`. `PaneRelation::Transiting` keeps
        // this system from adding a third copy. Neither system writes
        // `Visibility`; `source_visibility::resolve_portal_source_visibility`
        // is the one writer.
        let cover = panes.iter().find(|pane| {
            matches!(
                crate::pane_relation(pane, viewer.eye, min, max, transit.is_some()),
                crate::PaneRelation::FarCovered
            )
        });
        let Some(pane) = cover else {
            give_back(&mut commands, entity, &hidden);
            continue;
        };
        let Some(look) = piece_look(sprite, declared, anchor, &layouts, &images) else {
            // No loaded texture to rebuild from: leaving the whole sprite drawn
            // is the old bug, but blanking the body is a worse one.
            give_back(&mut commands, entity, &hidden);
            continue;
        };

        let (cover_min, cover_max) = crate::pane_cover_rect(pane);
        let pieces = crate::uncovered_remainder(min, max, cover_min, cover_max);

        // The pieces are this body now. A fully covered body has an empty
        // remainder, so nothing is spawned. This states a reason;
        // `resolve_portal_source_visibility` owns `Visibility`.
        commands.entity(entity).insert(PortalFarSideHidden);

        let control = Vec4::new(
            if look.flip_x { 1.0 } else { 0.0 },
            0.0,
            if look.silhouette { 1.0 } else { 0.0 },
            0.0,
        );
        // The pose the candidate was classified from, not a second reading of it.
        let base = *transform;

        for piece in pieces.iter() {
            let edges = crate::piece_clip_edges(&piece, min, max);
            let mut planes = edges
                .iter()
                .flatten()
                .map(|(point, normal)| clip_plane_render(&frame, *point, *normal));
            let (clip0, clip1, clip2) = (
                planes.next().unwrap_or(CLIP_PLANE_OFF),
                planes.next().unwrap_or(CLIP_PLANE_OFF),
                planes.next().unwrap_or(CLIP_PLANE_OFF),
            );
            debug_assert!(
                planes.next().is_none(),
                "a piece needed a fourth clip plane; the material carries three"
            );
            commands.spawn((
                PortalFarSidePiece,
                Mesh2d(mesh.clone()),
                MeshMaterial2d(materials.add(PortalClipMaterial {
                    uv_rect: look.uv_rect,
                    control,
                    tint: look.tint,
                    clip0,
                    clip1,
                    clip2,
                    color_texture: look.image.clone(),
                })),
                clip_piece_transform(&base, look.anchor, look.size),
                Name::new("Portal far-side piece"),
            ));
        }
    }
}

/// Everything a piece needs to look like the drawable it replaces.
struct PieceLook {
    uv_rect: Vec4,
    size: Vec2,
    anchor: Vec2,
    image: Handle<Image>,
    flip_x: bool,
    tint: Vec4,
    silhouette: bool,
}

/// The look of a candidate, from whichever description it carries.
///
/// A declaration wins over a sprite: a drawable that declares says its
/// `Sprite` does not describe what it paints.
fn piece_look(
    sprite: Option<&Sprite>,
    declared: Option<&DeclaredFrame>,
    anchor: Option<&Anchor>,
    layouts: &Assets<TextureAtlasLayout>,
    images: &Assets<Image>,
) -> Option<PieceLook> {
    if let Some(declared) = declared {
        return Some(PieceLook {
            uv_rect: declared.uv_rect,
            size: declared.size,
            anchor: declared.anchor,
            image: declared.color_texture.clone(),
            flip_x: declared.flip_x,
            tint: declared.tint,
            silhouette: declared.silhouette,
        });
    }
    let sprite = sprite?;
    let basis = sprite_frame_basis(sprite, layouts, images)?;
    let c = sprite.color.to_linear();
    Some(PieceLook {
        uv_rect: basis.uv_rect,
        size: basis.size,
        anchor: anchor.map_or(Vec2::ZERO, |a| a.0),
        image: sprite.image.clone(),
        flip_x: sprite.flip_x,
        tint: Vec4::new(c.red, c.green, c.blue, c.alpha),
        silhouette: false,
    })
}

/// Give back only the bodies this system hid, on the roads where no
/// classification is possible.
fn restore_hidden(
    commands: &mut Commands,
    hidden: &Query<Entity, With<PortalFarSideHidden>>,
    candidates: &mut Query<(
        Entity,
        &PortalCompositingCandidate,
        Option<&Sprite>,
        Option<&DeclaredFrame>,
        Option<&Anchor>,
        // Same query as `composite_far_side_bodies`.
        &Transform,
        Option<&ambition_portal2d::PortalTransit>,
    )>,
) {
    for (entity, ..) in candidates.iter_mut() {
        give_back(commands, entity, hidden);
    }
}

/// Reverse this system's own withdrawal, and only that.
fn give_back(
    commands: &mut Commands,
    entity: Entity,
    hidden: &Query<Entity, With<PortalFarSideHidden>>,
) {
    if hidden.get(entity).is_ok() {
        // Withdraw the reason only. The transit splitter may still need the
        // body hidden.
        commands.entity(entity).remove::<PortalFarSideHidden>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal2d::{PortalChannel, PortalChannelColor};

    const WORLD: Vec2 = Vec2::new(1000.0, 600.0);
    /// A wall pane facing LEFT (-x), so "in front" is the low-x side.
    fn pane() -> PlacedPortal {
        PlacedPortal::fixed(
            PortalChannel::Authored(PortalChannelColor::Purple),
            Vec2::new(500.0, 300.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(9.0, 46.0),
        )
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.insert_resource(PortalWorldFrame { size: WORLD });
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<TextureAtlasLayout>::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<PortalClipMaterial>::default());
        // The resolver is part of the unit under test: it is the only writer
        // of `Visibility`. `.chain()` supplies the `ApplyDeferred` that makes
        // the reason visible in the same frame.
        app.add_systems(
            Update,
            (
                composite_far_side_bodies,
                crate::source_visibility::resolve_portal_source_visibility,
            )
                .chain(),
        );
        app
    }

    fn loaded_sprite(app: &mut App) -> Sprite {
        let mut image = Image::default();
        image.texture_descriptor.size.width = 48;
        image.texture_descriptor.size.height = 48;
        let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
        let mut sprite = Sprite::from_image(handle);
        sprite.custom_size = Some(Vec2::new(48.0, 48.0));
        sprite
    }

    /// The handoff frame: far-covered on frame N, transiting on N+1, with both
    /// production systems in one schedule. The whole sprite must not come back on
    /// top of its own transit slices. The body moves between frames, as an actor
    /// walking through the pane does.
    #[test]
    fn a_far_covered_body_that_enters_transit_stays_hidden() {
        let mut app = test_app();
        // Both reason-staters run ahead of the one writer.
        app.add_systems(
            bevy::prelude::Update,
            crate::visuals::sync_portal_body_pieces
                .before(crate::source_visibility::resolve_portal_source_visibility),
        );
        // The pane, and the partner the transit decomposition needs.
        app.world_mut().spawn(pane());
        app.world_mut().spawn(PlacedPortal::fixed(
            PortalChannel::Authored(PortalChannelColor::Yellow),
            Vec2::new(532.0, 300.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(9.0, 46.0),
        ));
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));

        // Frame N: far-covered at the aperture, not yet transiting.
        let at_pane = Vec2::new(505.0, 300.0);
        let body = spawn_candidate(&mut app, at_pane, Vec2::new(24.0, 24.0));
        app.world_mut().entity_mut(body).insert((
            crate::PortalSceneBody,
            crate::PortalBodyView {
                // The transit splitter reads this: centred in the aperture, so the
                // decomposition has two charts.
                pos: Vec2::new(498.0, 300.0),
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
        ));
        app.update();
        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "a far-covered body is drawn by its uncovered pieces, not whole"
        );
        assert!(
            app.world().get::<PortalFarSideHidden>(body).is_some(),
            "and the far-side compositor owns that hide"
        );

        // Frame N+1: `PortalTransit` arrives. That is the handoff; it changes
        // the far-side classification from `FarCovered` to `Transiting`.
        app.world_mut()
            .entity_mut(body)
            .insert(ambition_portal2d::PortalTransit {
                straddling: pane().channel,
                crossed: false,
            });
        app.update();

        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "⛔ the whole sprite must NOT come back on the handoff frame: the \
             transit pieces are the body, and the far-side compositor withdrawing \
             its own reason must not overrule the reason that replaced it"
        );
    }

    /// `eye` is in front of the pane (low x), so a body at high x is far.
    /// Inserted as a resource, as the host publishes it.
    fn spawn_viewer(app: &mut App, eye: Vec2) {
        app.insert_resource(PortalViewer {
            present: true,
            eye,
            ..default()
        });
    }

    fn spawn_candidate(app: &mut App, centre: Vec2, half: Vec2) -> Entity {
        // The generic actor band, `WORLD_Z_DUMMY + 1.0`.
        spawn_candidate_at_z(app, centre, half, 11.0)
    }

    /// The pieces must follow this frame's pose. The fixture makes the local
    /// `Transform` and the `GlobalTransform` disagree, as they do during
    /// `Update` in production (propagation runs in `PostUpdate`).
    #[test]
    fn a_moving_body_is_recomposed_from_the_pose_it_has_now() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        let before = piece_translations(&mut app);
        assert!(
            !before.is_empty(),
            "premise: the body composites into at least one piece"
        );

        // It moves. Local transform is this frame's; the propagated one is not.
        let moved = Vec2::new(505.0, 320.0);
        let frame = PortalWorldFrame { size: WORLD };
        {
            let mut entity = app.world_mut().entity_mut(body);
            entity.get_mut::<PortalCompositingCandidate>().unwrap().drawn_centre = moved;
            *entity.get_mut::<Transform>().unwrap() =
                Transform::from_translation(frame.to_render(moved, 11.0));
            // `GlobalTransform` stays at the old pose, as `PostUpdate` would leave it.
        }
        app.update();

        let after = piece_translations(&mut app);
        assert!(!after.is_empty(), "the body still composites after moving");
        // Check against the body's current pose, not only "pieces moved": a
        // constant offset would pass that. A piece is a slice of the body, so
        // it must sit within the body's drawn extent this frame.
        let now = app
            .world()
            .get::<Transform>(body)
            .expect("the body has a pose")
            .translation;
        assert!(
            after
                .iter()
                .any(|(x, y)| (*x - now.x as i32).abs() <= 48
                    && (*y - now.y as i32).abs() <= 48),
            "no piece sits near the body's CURRENT pose ({:.0}, {:.0}); pieces at \
             {after:?} (previous frame: {before:?}). Classification used this \
             frame's transform and the pieces used another.",
            now.x,
            now.y
        );
    }

    /// Where every far-side piece currently sits, so a move can be seen.
    fn piece_translations(app: &mut App) -> Vec<(i32, i32)> {
        app.world_mut()
            .query_filtered::<&Transform, With<PortalFarSidePiece>>()
            .iter(app.world())
            .map(|t| (t.translation.x as i32, t.translation.y as i32))
            .collect()
    }

    fn spawn_candidate_at_z(app: &mut App, centre: Vec2, half: Vec2, z: f32) -> Entity {
        let sprite = loaded_sprite(app);
        let frame = PortalWorldFrame { size: WORLD };
        let translation = frame.to_render(centre, z);
        app.world_mut()
            .spawn((
                PortalCompositingCandidate {
                    drawn_centre: centre,
                    drawn_half: half,
                },
                sprite,
                Transform::from_translation(translation),
                GlobalTransform::from(Transform::from_translation(translation)),
                Visibility::Inherited,
            ))
            .id()
    }

    fn pieces(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<PortalFarSidePiece>>()
            .iter(app.world())
            .count()
    }

    fn visibility(app: &App, entity: Entity) -> Visibility {
        *app.world().get::<Visibility>(entity).expect("visibility")
    }

    /// A body behind the aperture is redrawn as the part the pane leaves
    /// visible, and its whole-sprite draw is withdrawn. The covered pixels are
    /// not submitted, so no z can bring them back.
    #[test]
    fn a_far_side_body_is_redrawn_as_the_uncovered_part_only() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "the whole-sprite draw must be withdrawn; the pieces are the body now"
        );
        let n = pieces(&mut app);
        assert!(
            (1..=4).contains(&n),
            "expected between one and four uncovered pieces, got {n}"
        );
    }

    /// The near side is already correct with a single z. The repair must not
    /// change it (raising `PORTAL_WINDOW_Z` would).
    #[test]
    fn a_near_side_body_is_left_exactly_as_it_was() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(495.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(visibility(&app, body), Visibility::Inherited);
        assert_eq!(pieces(&mut app), 0, "a near-side body owes no pieces");
    }

    #[test]
    fn a_body_nowhere_near_a_pane_is_left_exactly_as_it_was() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(900.0, 100.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(visibility(&app, body), Visibility::Inherited);
        assert_eq!(pieces(&mut app), 0);
    }

    /// With no viewer, the old picture is kept. A hidden sprite with no pieces
    /// would be a body that vanished.
    #[test]
    fn with_no_viewer_every_body_still_draws_whole() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(visibility(&app, body), Visibility::Inherited);
        assert_eq!(pieces(&mut app), 0);
    }

    /// The pieces are rebuilt each frame, so they must not accumulate.
    #[test]
    fn the_pieces_do_not_accumulate_across_frames() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        let first = pieces(&mut app);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(first, pieces(&mut app), "pieces accumulated across frames");
    }

    /// The player band (`WORLD_Z_PLAYER`, 20) and the actor band (11) are both
    /// above `PORTAL_WINDOW_Z` (9.5). The repair never reads z; it subtracts
    /// rects. The same body at two z values must give identical output.
    #[test]
    fn the_player_band_and_the_actor_band_composite_identically() {
        let mut counts = Vec::new();
        for z in [11.0_f32, 20.0] {
            let mut app = test_app();
            app.world_mut().spawn(pane());
            spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
            let far =
                spawn_candidate_at_z(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0), z);
            let near =
                spawn_candidate_at_z(&mut app, Vec2::new(470.0, 300.0), Vec2::new(4.0, 4.0), z);
            app.update();
            assert_eq!(
                visibility(&app, far),
                Visibility::Hidden,
                "far-side body at z={z} was not composited"
            );
            assert_eq!(
                visibility(&app, near),
                Visibility::Inherited,
                "near-side body at z={z} must be left alone"
            );
            counts.push(pieces(&mut app));
        }
        assert_eq!(
            counts[0], counts[1],
            "the two z bands produced different pieces; this repair must not read z"
        );
        assert!(counts[0] > 0, "no pieces drawn at either band");
    }

    /// `present: false` means there is no eye this frame, and `eye` is
    /// meaningless. Nothing is composited.
    #[test]
    fn an_absent_eye_composites_nothing() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        app.insert_resource(PortalViewer {
            present: false,
            eye: Vec2::new(400.0, 300.0),
            ..default()
        });
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(visibility(&app, body), Visibility::Inherited);
        assert_eq!(pieces(&mut app), 0);
    }

    /// A transiting body belongs to `sync_portal_body_pieces`, which draws it
    /// as two clipped slices. Compositing it too would add a third copy. The
    /// `Visibility` write belongs to [`crate::source_visibility`].
    #[test]
    fn a_transiting_body_is_left_to_the_split_presentation() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        // The same far-side position that IS composited without the marker.
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.world_mut()
            .entity_mut(body)
            .insert(ambition_portal2d::PortalTransit {
                straddling: PortalChannel::Authored(PortalChannelColor::Purple),
                crossed: false,
            });
        app.update();
        assert_eq!(
            visibility(&app, body),
            Visibility::Inherited,
            "the transit presentation owns this body's visibility"
        );
        assert_eq!(
            pieces(&mut app),
            0,
            "a transiting body must not gain a third copy"
        );
    }

    /// A body hidden for reasons unrelated to portals (death, culling, a
    /// cutscene) must stay hidden.
    #[test]
    fn a_body_hidden_by_someone_else_is_not_given_back() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        // Near-side and disjoint: the compositor has no business with either,
        // and both take the give-back road every frame.
        let near = spawn_candidate(&mut app, Vec2::new(495.0, 300.0), Vec2::new(24.0, 24.0));
        let away = spawn_candidate(&mut app, Vec2::new(900.0, 100.0), Vec2::new(24.0, 24.0));
        for entity in [near, away] {
            *app.world_mut().get_mut::<Visibility>(entity).expect("visibility") =
                Visibility::Hidden;
        }
        for _ in 0..3 {
            app.update();
        }
        for entity in [near, away] {
            assert_eq!(
                visibility(&app, entity),
                Visibility::Hidden,
                "another system's hidden body was resurrected by the compositor"
            );
        }
    }

    /// A body that walks from far to near gets its whole sprite back.
    #[test]
    fn a_body_that_moves_to_the_near_side_is_restored() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(visibility(&app, body), Visibility::Hidden);

        let mut candidate = app
            .world_mut()
            .get_mut::<PortalCompositingCandidate>(body)
            .expect("candidate");
        candidate.drawn_centre = Vec2::new(495.0, 300.0);
        // Production always has a visibility owner for the body (`sync_visuals`
        // or morph-ball sync), and the portal resolver runs after it. Releasing
        // the portal claim means "no opinion", so the owner's value stands.
        // This fixture plays that owner.
        app.world_mut().entity_mut(body).insert(Visibility::Inherited);
        app.update();
        assert_eq!(
            visibility(&app, body),
            Visibility::Inherited,
            "crossing to the near side must give the whole sprite back"
        );
        assert_eq!(pieces(&mut app), 0);
    }

    /// A far-side body's other drawables must follow its hide.
    ///
    /// The hit-flash silhouette is a separate root mesh that mirrors the base
    /// sprite. `sync_hit_flash_overlays` runs before portal presentation, so it
    /// sees a visible source on the frame the portal hides the body. Ordering
    /// cannot fix this: the portal publisher runs after
    /// `animate_feature_sprites`, which is after the hit-flash mirror, so a later
    /// mirror is a cycle. The resolver hides the dependants in the same pass,
    /// through `PresentationOf`.
    #[test]
    fn a_drawable_that_names_a_hidden_body_is_hidden_with_it() {
        use ambition_platformer2d_shared_tangle::lifecycle::PresentationOf;

        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        // Its silhouette: a separate root that mirrors it, visible as the
        // hit-flash overlay is spawned.
        let silhouette = app
            .world_mut()
            .spawn((Visibility::Visible, PresentationOf(body)))
            .id();

        app.update();

        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "premise: the body itself is far-side and hidden"
        );
        assert_eq!(
            visibility(&app, silhouette),
            Visibility::Hidden,
            "the body is drawn as clipped pieces but its silhouette still draws \
             whole, so a far-side character shows its outline over the pane"
        );

        // The departure, with nothing writing the silhouette. The hit-flash
        // overlay is spawned `Visible` and its update path never writes
        // visibility again. If the resolver does not restore what it took, the
        // silhouette stays hidden for the rest of the session.
        {
            let mut entity = app.world_mut().entity_mut(body);
            entity.get_mut::<PortalCompositingCandidate>().unwrap().drawn_centre =
                Vec2::new(495.0, 300.0);
        }
        app.update();
        assert_ne!(
            visibility(&app, silhouette),
            Visibility::Hidden,
            "the owner returned to the near side and the portal never released its \
             hide, so this drawable is latched hidden for the rest of the session"
        );
    }

    /// A dependant the compositor can see answers for itself. Body ownership
    /// is not compositing geometry authority.
    ///
    /// An unparented sprite that names a body (a tether line, a flyline) is a
    /// compositing candidate, so its own bounds decide whether the pane hides
    /// it. The silhouette in the test above is not a sprite; there the scalar
    /// hide is the only tool, and it must still be claimed and released.
    #[test]
    fn a_sprite_dependant_disjoint_from_the_pane_is_not_hidden_by_its_owner() {
        use ambition_platformer2d_shared_tangle::lifecycle::PresentationOf;

        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        // An unparented sprite drawable naming that body, far from the pane.
        let line = app
            .world_mut()
            .spawn((
                Visibility::Visible,
                Sprite::default(),
                Transform::from_xyz(120.0, 300.0, 0.0),
                PresentationOf(body),
            ))
            .id();

        app.update();

        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "premise: the OWNER is far-side and hidden, or this proves nothing"
        );
        assert_ne!(
            visibility(&app, line),
            Visibility::Hidden,
            "a sprite dependant 380px from the pane was hidden because the body it \
             names overlaps one — body ownership became compositing authority"
        );
    }

    /// A loaded 48x48 image, the way `loaded_sprite` makes one, for a drawable
    /// that is not a sprite.
    fn loaded_image(app: &mut App) -> Handle<Image> {
        let mut image = Image::default();
        image.texture_descriptor.size.width = 48;
        image.texture_descriptor.size.height = 48;
        app.world_mut().resource_mut::<Assets<Image>>().add(image)
    }

    /// A `Mesh2d`-shaped drawable: no `Sprite`, a declared frame, a unit quad
    /// scaled to its drawn size -- the hit-flash overlay's exact shape.
    fn spawn_declared(app: &mut App, centre: Vec2, half: Vec2, alpha: f32) -> Entity {
        let image = loaded_image(app);
        let frame = PortalWorldFrame { size: WORLD };
        let mut transform = Transform::from_translation(frame.to_render(centre, 11.5));
        transform.scale = (half * 2.0).extend(1.0);
        app.world_mut()
            .spawn((
                PortalCompositingCandidate {
                    drawn_centre: centre,
                    drawn_half: half,
                },
                DeclaredFrame {
                    color_texture: image,
                    uv_rect: Vec4::new(0.0, 0.0, 1.0, 1.0),
                    flip_x: false,
                    tint: Vec4::new(1.0, 1.0, 1.0, alpha),
                    silhouette: true,
                    size: Vec2::ONE,
                    anchor: Vec2::ZERO,
                },
                transform,
                GlobalTransform::from(transform),
                Visibility::Inherited,
            ))
            .id()
    }

    /// A `Mesh2d` overlay that declares what it paints composites like a
    /// sprite: hidden whole, redrawn as the uncovered pieces, and the pieces
    /// keep its silhouette look, not the sprite's sampled colour.
    #[test]
    fn a_declared_non_sprite_drawable_is_redrawn_as_silhouette_pieces() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let overlay = spawn_declared(
            &mut app,
            Vec2::new(505.0, 300.0),
            Vec2::new(24.0, 24.0),
            0.8,
        );
        app.update();
        assert_eq!(
            visibility(&app, overlay),
            Visibility::Hidden,
            "a far-covered declared drawable must be withdrawn whole; its pieces \
             are it now"
        );
        let n = pieces(&mut app);
        assert!((1..=4).contains(&n), "expected uncovered pieces, got {n}");
        // The pieces look like the overlay. A piece that sampled the texture
        // colour would paint the character's art in place of its flash.
        let looks: Vec<(f32, f32)> = {
            let world = app.world_mut();
            let mut q = world
                .query_filtered::<&MeshMaterial2d<PortalClipMaterial>, With<PortalFarSidePiece>>();
            let handles: Vec<_> = q.iter(world).map(|m| m.0.clone()).collect();
            let materials = world.resource::<Assets<PortalClipMaterial>>();
            handles
                .iter()
                .map(|h| {
                    let m = materials.get(h).expect("piece material");
                    (m.control.z, m.tint.w)
                })
                .collect()
        };
        assert!(
            looks
                .iter()
                .all(|(silhouette, alpha)| *silhouette > 0.5 && (*alpha - 0.8).abs() < 1e-6),
            "the pieces of a silhouette overlay are not silhouettes at its \
             intensity: {looks:?}"
        );
    }

    /// A declared dependant answers for itself, as a sprite dependant does. A
    /// silhouette far from the pane is not hidden because its body is behind
    /// one.
    #[test]
    fn a_declared_dependant_disjoint_from_the_pane_is_not_hidden_by_its_owner() {
        use ambition_platformer2d_shared_tangle::lifecycle::PresentationOf;

        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        // Its silhouette, far from the pane, declaring what it paints.
        let silhouette = spawn_declared(
            &mut app,
            Vec2::new(120.0, 300.0),
            Vec2::new(24.0, 24.0),
            1.0,
        );
        app.world_mut()
            .entity_mut(silhouette)
            .insert(PresentationOf(body));

        app.update();

        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "premise: the OWNER is far-side and hidden, or this proves nothing"
        );
        assert_ne!(
            visibility(&app, silhouette),
            Visibility::Hidden,
            "a declared drawable 380px from the pane was hidden because the body \
             it names overlaps one -- the scalar fallback is still deciding for a \
             drawable the compositor can classify"
        );
    }

    /// Releasing the portal's claim must not show a body that another owner
    /// still hides.
    ///
    /// Frame N: the body is far-side, so the portal hides it and records the
    /// claim. Frame N+1: it crosses to the near side while another owner
    /// (morph-ball or submerged presentation) still hides it. The release must
    /// not write `Visibility::Inherited`.
    #[test]
    fn releasing_the_portal_hide_leaves_another_owners_hide_alone() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        spawn_viewer(&mut app, Vec2::new(400.0, 300.0));
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "premise: the portal owns this hide on the first frame"
        );

        // It crosses to the near side, and something else wants it hidden.
        {
            let mut entity = app.world_mut().entity_mut(body);
            entity.get_mut::<PortalCompositingCandidate>().unwrap().drawn_centre =
                Vec2::new(495.0, 300.0);
            *entity.get_mut::<Visibility>().unwrap() = Visibility::Hidden;
        }
        app.update();

        assert_eq!(
            visibility(&app, body),
            Visibility::Hidden,
            "the portal released its own hide and overwrote another owner's -- a \
             morphed or submerged body would pop back into view the frame it \
             stopped being far-side"
        );

        // The claim itself must also be released; the visibility check above
        // passes either way because of the other owner's hide.
        //
        // A stale `PortalSourceHidden` tells the render publisher that the body
        // is portal-hidden and may still be composited
        // (`portal_hid_it.is_none()`). It also makes this resolver hide the
        // body's `PresentationOf` dependants.
        assert!(
            app.world().get::<crate::source_visibility::PortalSourceHidden>(body).is_none(),
            "the portal's reason ended, so its CLAIM must end too -- a stale claim \
             tells the publisher this body is portal-hidden forever"
        );
    }
}
