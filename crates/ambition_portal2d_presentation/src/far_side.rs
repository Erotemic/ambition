//! Draw a far-side body as the part of it the pane does NOT cover.
//!
//! ⛔⛔ THE BUG THIS EXISTS FOR. A pane draws at [`crate::PORTAL_WINDOW_Z`]
//! (`9.5`) and an actor at `WORLD_Z_DUMMY + 1.0` or `WORLD_Z_PLAYER`, so every
//! actor wins the depth test against every pane and a body standing BEHIND an
//! aperture punches through the captured image that should hide it.
//!
//! ⭐ THE REPAIR IS STRUCTURAL, NOT AN ORDERING. The covered region is never
//! handed to the renderer: [`crate::uncovered_remainder`] returns the part the
//! pane does not hide, and only those pieces are drawn. There is no z to get
//! wrong because the pixels that could be wrong are in no piece. Both cheap
//! fixes were ruled out for reasons that outlive them — raising the window z
//! above the player inverts the bug onto near-side bodies, and an actor's single
//! z cannot serve two panes that disagree about it in the same frame.
//!
//! ⚠ ONE EYE, BY CONSTRUCTION. [`PortalViewer`] is a RESOURCE, so there is
//! exactly one viewpoint and no arbitration to do. ⇒ A split-screen session
//! would need per-view pieces on per-view render layers, because one body is
//! near for one player and far for the other -- structurally the two-pane
//! problem again -- but that is not a case this seam can even express today.
//!
//! ⚠ The pieces carry no `RenderLayers` because actor sprites do not: the
//! per-view isolation pass writes layers onto things keyed by `PresentedForView`
//! -- labels, backdrop panels, plates -- and a body sprite is none of those. The
//! transit pieces make the same assumption, so this matches the shipped road
//! rather than adding one.
//!
//! ⚠ ONE COVERING PANE. Subtracting two apertures from one body would exceed the
//! three clip half-planes [`PortalClipMaterial`] carries. MEASURED 2026-09-05
//! (`scripts/portal_pane_separation.py`): the closest two panes a body could be
//! far of BOTH are 163.2px apart against a body about 32px wide, so the shipped
//! worlds cannot reach that case. When more than one pane covers a body the
//! FIRST by [`ambition_portal2d::stable_portal_order`] is subtracted — chosen so
//! the result is deterministic rather than query-order dependent.

use ambition_platformer2d_core::Vec2;
use ambition_portal2d::PlacedPortal;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial, CLIP_PLANE_OFF,
};
use crate::{PortalCompositingCandidate, PortalViewer, PortalWorldFrame};

/// One drawn fragment of a far-side body. Rebuilt every frame from the source
/// sprite, so it can never drift from what the sprite currently looks like.
#[derive(Component)]
pub struct PortalFarSidePiece;

/// This body's whole-sprite draw was withdrawn BY THIS SYSTEM, and this system
/// is what will give it back.
///
/// ⛔⛔ VISIBILITY IS NOT THIS SYSTEM'S FACT TO OWN. A body can be hidden for
/// reasons that have nothing to do with portals -- death, culling, a cutscene,
/// an editor toggle -- and a compositor that wrote `Inherited` onto every
/// candidate each frame would silently overrule all of them. That is the same
/// defect this module exists to fix, one layer up: two authorities writing one
/// fact, with the last writer winning by accident of ordering.
///
/// ⇒ The marker records what THIS system did, so it can reverse exactly that and
/// nothing else. A body it never hid is never touched.
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
        &Sprite,
        Option<&Anchor>,
        &GlobalTransform,
        Option<&ambition_portal2d::PortalTransit>,
        &mut Visibility,
    )>,
) {
    for entity in &stale {
        commands.entity(entity).despawn();
    }

    // ⛔ Near and far are relative to a viewpoint. Without one there is no
    // honest classification, so every body draws exactly as it did before.
    //
    // ⭐ `Res`, because that is what the eye IS: the host writes it through
    // `ResMut<PortalViewer>` and every shipped reader takes `Res<PortalViewer>`.
    // The first version queried it as a component, which HAPPENS to work --
    // resources live on a singleton entity, so the query finds it -- and that is
    // exactly why it was worth changing: one fact read two ways, where the
    // second way is an accident of storage rather than a statement of intent.
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

    for (entity, candidate, sprite, anchor, transform, transit, mut visibility) in &mut candidates {
        let min = candidate.drawn_centre - candidate.drawn_half;
        let max = candidate.drawn_centre + candidate.drawn_half;

        // The first covering pane in the stable order; see the module note on
        // why one is enough for the shipped worlds and why it is not arbitrary.
        // ⛔⛔ THE TRANSIT FLAG IS NOT A FORMALITY, AND HARDCODING IT `false`
        // WAS A REAL DEFECT. A straddling body is already drawn as two clipped
        // slices by `sync_portal_body_pieces`, which also owns its `Visibility`.
        // Classifying it `FarCovered` gives it a THIRD copy and puts two systems
        // on one fact -- the exact shape `PortalFarSideHidden` exists to prevent
        // one level down. `PaneRelation::Transiting` is the vocabulary for
        // "another presentation owns this body"; it only works if it is asked.
        let cover = panes.iter().find(|pane| {
            matches!(
                crate::pane_relation(pane, viewer.eye, min, max, transit.is_some()),
                crate::PaneRelation::FarCovered
            )
        });
        let Some(pane) = cover else {
            give_back(&mut commands, entity, &hidden, &mut visibility);
            continue;
        };
        let Some(basis) = sprite_frame_basis(sprite, &layouts, &images) else {
            // No loaded texture to rebuild from: leaving the whole sprite drawn
            // is the old bug, but blanking the body is a worse one.
            give_back(&mut commands, entity, &hidden, &mut visibility);
            continue;
        };

        let (cover_min, cover_max) = crate::pane_cover_rect(pane);
        let pieces = crate::uncovered_remainder(min, max, cover_min, cover_max);

        // The pieces ARE this body now. When the pane covers it completely the
        // remainder is empty and nothing is spawned, which is the correct
        // picture rather than a special case.
        *visibility = Visibility::Hidden;
        commands.entity(entity).insert(PortalFarSideHidden);

        let tint = {
            let c = sprite.color.to_linear();
            Vec4::new(c.red, c.green, c.blue, c.alpha)
        };
        let control = Vec4::new(if sprite.flip_x { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0);
        let base = Transform {
            translation: transform.translation(),
            rotation: transform.rotation(),
            scale: transform.scale(),
        };
        let anchor_v = anchor.map_or(Vec2::ZERO, |a| a.0);

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
                    uv_rect: basis.uv_rect,
                    control,
                    tint,
                    clip0,
                    clip1,
                    clip2,
                    color_texture: sprite.image.clone(),
                })),
                clip_piece_transform(&base, anchor_v, basis.size),
                Name::new("Portal far-side piece"),
            ));
        }
    }
}

/// Give back only the bodies THIS system hid, on the roads where no
/// classification is possible -- so "we cannot tell" never means "the body
/// disappears", and never means "somebody else's hidden body reappears".
fn restore_hidden(
    commands: &mut Commands,
    hidden: &Query<Entity, With<PortalFarSideHidden>>,
    candidates: &mut Query<(
        Entity,
        &PortalCompositingCandidate,
        &Sprite,
        Option<&Anchor>,
        &GlobalTransform,
        Option<&ambition_portal2d::PortalTransit>,
        &mut Visibility,
    )>,
) {
    for (entity, .., mut visibility) in candidates.iter_mut() {
        give_back(commands, entity, hidden, &mut visibility);
    }
}

/// Reverse this system's own withdrawal, and only that.
fn give_back(
    commands: &mut Commands,
    entity: Entity,
    hidden: &Query<Entity, With<PortalFarSideHidden>>,
    visibility: &mut Visibility,
) {
    if hidden.get(entity).is_ok() {
        *visibility = Visibility::Inherited;
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
        app.add_systems(Update, composite_far_side_bodies);
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

    /// `eye` sits well in FRONT of the pane (low x), so a body at high x is far.
    ///
    /// ⭐ INSERTED AS A RESOURCE, WHICH IS HOW THE HOST PUBLISHES IT. The first
    /// version SPAWNED it as a component; that passes -- resources live on a
    /// singleton entity, so a component query finds them -- and it meant every
    /// test built its subject a way the game never does.
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

    /// ⭐⭐ JON'S CASE. A body BEHIND the aperture is redrawn as the part the
    /// pane leaves visible, and its whole-sprite draw is withdrawn — so the
    /// covered pixels are not submitted at all and no z can bring them back.
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

    /// The near side is the half a single z already gets right, and the repair
    /// must not touch it -- that is the inverse bug Jon named when he ruled out
    /// raising `PORTAL_WINDOW_Z`.
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

    /// ⛔⛔ "WE CANNOT TELL" MUST NOT MEAN "THE BODY DISAPPEARS". Near and far
    /// are relative to a viewpoint; with no viewer the old picture is kept,
    /// because a hidden sprite with no pieces is a body that vanished.
    #[test]
    fn with_no_viewer_every_body_still_draws_whole() {
        let mut app = test_app();
        app.world_mut().spawn(pane());
        let body = spawn_candidate(&mut app, Vec2::new(505.0, 300.0), Vec2::new(24.0, 24.0));
        app.update();
        assert_eq!(visibility(&app, body), Visibility::Inherited);
        assert_eq!(pieces(&mut app), 0);
    }

    /// The pieces are rebuilt wholesale each frame, so they must not accumulate
    /// -- a leak here is invisible on frame one and a slideshow by frame 600.
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

    /// ⭐⭐ THE PLAYER BAND AND THE ACTOR BAND GET THE SAME ANSWER, WHICH IS THE
    /// WHOLE POINT. `WORLD_Z_PLAYER` is 20 and a generic actor sits at 11; both
    /// are above `PORTAL_WINDOW_Z` (9.5), which is why a single z could not
    /// serve them and why Jon asked for the pair twice.
    ///
    /// ⇒ This repair never READS z. It subtracts the pane's rect from the
    /// drawable's, so the answer is geometry and the band is irrelevant --
    /// asserted by giving the same body two very different z values and
    /// demanding identical output. A version that reached for z to decide
    /// anything would fail here.
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

    /// ⛔ `present: false` MEANS THERE IS NO EYE THIS FRAME, and the `eye` field
    /// is then meaningless. Compositing on it would classify near and far from a
    /// stale or default position -- a body hidden against a viewpoint nobody
    /// has. The flag exists to be asked.
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

    /// ⛔⛔ A TRANSITING BODY IS ANOTHER PRESENTATION'S. `sync_portal_body_pieces`
    /// already draws it as two clipped slices and owns its `Visibility`;
    /// compositing it too would give it a THIRD copy and put two systems on one
    /// fact. Jon named this case explicitly.
    ///
    /// ⚠ The first version hardcoded `transiting: false` into the
    /// classification, so this body was `FarCovered` and got the third copy.
    /// `PaneRelation::Transiting` existed the whole time -- a vocabulary is only
    /// worth having if it is asked.
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

    /// ⛔⛔ VISIBILITY BELONGS TO WHOEVER SET IT. A body hidden for reasons that
    /// are nothing to do with portals -- death, culling, a cutscene -- must stay
    /// hidden. An earlier draft wrote `Inherited` onto every candidate each
    /// frame and would have silently resurrected all of them, which is this
    /// module's own bug one layer up: two authorities writing one fact, last
    /// writer wins by accident of ordering.
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

    /// ⭐ A body that walks from far to near gets its whole sprite BACK. Without
    /// this the repair trades a punch-through for a permanently invisible actor,
    /// which is the louder bug.
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
        app.update();
        assert_eq!(
            visibility(&app, body),
            Visibility::Inherited,
            "crossing to the near side must give the whole sprite back"
        );
        assert_eq!(pieces(&mut app), 0);
    }
}
