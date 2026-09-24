//! Publish the drawables a portal pane may have to composite against.
//!
//! The portal presentation crate cannot see ordinary actors. Its body seams
//! are `PortalSceneBody` (one entity, decomposed at the seam) and
//! `PortalAffordanceBody` (whoever operates the portals). An NPC behind an
//! aperture is neither, but this crate draws it at `WORLD_Z_DUMMY + 1.0`,
//! above every pane, so a far-side actor would show through a seamless
//! window.
//!
//! So the host publishes the fact, like every other seam that crate exposes.
//! That crate does not reach into this one, and this one does not learn what
//! a pane is.

use bevy::prelude::*;

/// Publish each drawn actor sprite as a compositing candidate, in engine
/// coordinates.
///
/// Use drawn bounds, not the collision box. `Sprite::custom_size` is what
/// paints; the collision box is often smaller, and the overhang is what shows
/// through the window.
///
/// Sprites with no `custom_size` are skipped. A texture-sized sprite's bounds
/// are unknown here without the atlas, and a guessed rectangle would make the
/// result untrustworthy.
pub fn publish_portal_compositing_candidates(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    // Both `FeatureVisual` and `PlayerVisual`: the exploration player is
    // spawned with `PlayerVisual` (`session/setup.rs`), not `FeatureVisual`.
    //
    // Read the local `Transform`, not `GlobalTransform`. Propagation runs in
    // `PostUpdate`, so during `Update` a `GlobalTransform` is last frame's
    // pose, while `actors::sync_visuals` has already written this frame's
    // `Transform`. Mixing them makes the compositor subtract a region the body
    // already left.
    //
    // This is sound only because these sprites are unparented (top-level
    // world-space entities), so local is world. `Without<ChildOf>` enforces
    // it: a parented drawable would need the propagated pose.
    drawables: Query<
        (
            Entity,
            &Sprite,
            &Transform,
            Option<&bevy::sprite::Anchor>,
            &Visibility,
            Option<&ambition_portal2d_presentation::PortalSourceHidden>,
        ),
        (
            Or<(
                With<crate::rendering::primitives::FeatureVisual>,
                With<ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual>,
                // A body's other representations draw too. While a player is
                // morphed, its `PlayerVisual` sprite is hidden and the ball is the
                // player: a separate root at `WORLD_Z_PLAYER + 0.05` (20.05), above
                // the portal band (at or below `WORLD_Z_DUMMY`, 10).
                // `PresentationOf` says whose body such a drawable draws.
                With<ambition_platformer2d_shared_tangle::lifecycle::PresentationOf>,
            )>,
            Without<bevy::prelude::ChildOf>,
        ),
    >,
    // Non-sprite drawables. A body-owned `Mesh2d` that declares what it
    // paints (`DeclaredFrame`) is published from that declaration and its
    // transform, so the compositor can clip it like a sprite (for example
    // the hit-flash silhouette). `Without<Sprite>` keeps the two queries
    // disjoint, so each drawable is published once; a sprite that also
    // declares goes through the sprite arm.
    declared: Query<
        (
            Entity,
            &ambition_sprite_fx::DeclaredFrame,
            &Transform,
            &Visibility,
            Option<&ambition_portal2d_presentation::PortalSourceHidden>,
        ),
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::PresentationOf>,
            Without<Sprite>,
            Without<bevy::prelude::ChildOf>,
        ),
    >,
) {
    // `SessionWorldRef` is a `Single`, so this system does not run without a
    // session world. There is then no frame to publish engine positions in.
    let size = world.0.size;
    for (entity, frame, transform, visibility, portal_hid_it) in &declared {
        if matches!(visibility, Visibility::Hidden) && portal_hid_it.is_none() {
            continue;
        }
        commands
            .entity(entity)
            .insert(candidate_for(size, transform, frame.anchor, frame.size));
    }
    for (entity, sprite, transform, anchor, visibility, portal_hid_it) in &drawables {
        // A drawable that nothing draws is not a candidate. For example, a
        // morphed player's hidden base sprite must not produce far-side pieces
        // while the ball draws.
        //
        // Except when the portal hides it: a far-side body is `Hidden` because
        // clipped pieces replace it. Dropping its candidate would make the
        // compositor show the body again, then hide it next frame, and flicker.
        // The question is whether something other than the portal hides it.
        if matches!(visibility, Visibility::Hidden) && portal_hid_it.is_none() {
            continue;
        }
        let Some(drawn) = sprite.custom_size else {
            continue;
        };
        commands.entity(entity).insert(candidate_for(
            size,
            transform,
            anchor.map_or(Vec2::ZERO, |a| a.0),
            drawn,
        ));
    }
}

/// The compositing candidate for a drawable posed by `transform`, pivoting on
/// `anchor`, painting a `drawn`-sized quad before scale.
///
/// A sprite pivots on its anchor; a quad is centre-origin. Character sprites
/// are feet-anchored (`feet_anchor_for_render_size`), so the drawn centre is
/// most of a body height above the translation.
///
/// Uses the same helper the compositor uses to place its pieces, so the
/// subtracted region and the drawn region always agree.
fn candidate_for(
    size: ambition_platformer2d_core::Vec2,
    transform: &Transform,
    anchor: Vec2,
    drawn: Vec2,
) -> ambition_portal2d_presentation::PortalCompositingCandidate {
    let posed = ambition_portal2d_presentation::clip_piece_transform(transform, anchor, drawn);
    let bevy_centre = posed.translation.truncate();
    // The single definition of the y-flip.
    let centre = ambition_platformer2d_core::config::bevy_size_to_world(size, bevy_centre);
    ambition_portal2d_presentation::PortalCompositingCandidate {
        drawn_centre: centre,
        // A y-flip moves a centre, not a size. Scale does change a size, and
        // `clip_piece_transform` folds the sprite scale into the posed quad,
        // so the half-extent is read from there.
        //
        // Rotation changes it too. The candidate is a world-space AABB, and a
        // rotated non-square sprite (for example `ActorRoll`) overhangs its
        // unrotated rectangle at the corners.
        drawn_half: rotated_half_extent(posed.scale.truncate().abs() * 0.5, posed.rotation),
    }
}

/// World-space AABB half-extent of a rectangle rotated about its own centre.
///
/// The standard absolute-rotation form: for rotation θ the extents are
/// `|cos|*hx + |sin|*hy` and `|sin|*hx + |cos|*hy`. Exact at every angle, and
/// the identity at θ = 0.
///
/// Only the Z rotation is meaningful: these are 2D world sprites.
fn rotated_half_extent(half: Vec2, rotation: Quat) -> Vec2 {
    let (axis_z, angle) = {
        let (axis, angle) = rotation.to_axis_angle();
        (axis.z, angle)
    };
    let theta = angle * if axis_z < 0.0 { -1.0 } else { 1.0 };
    let (sin, cos) = theta.sin_cos();
    let (sin, cos) = (sin.abs(), cos.abs());
    Vec2::new(
        cos * half.x + sin * half.y,
        sin * half.x + cos * half.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
    use ambition_portal2d_presentation::PortalCompositingCandidate;
    use bevy::sprite::Anchor;

    const WORLD: ambition_platformer2d_core::Vec2 =
        ambition_platformer2d_core::Vec2::new(1000.0, 600.0);

    fn app() -> App {
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "portal compositing bridge",
                WORLD,
                ambition_platformer2d_core::Vec2::new(WORLD.x * 0.5, WORLD.y * 0.5),
                Vec::new(),
            )),
        );
        app.add_systems(Update, publish_portal_compositing_candidates);
        app
    }

    fn sprite(size: Vec2) -> Sprite {
        let mut sprite = Sprite::default();
        sprite.custom_size = Some(size);
        sprite
    }

    /// The hidden base sprite of a morphed player must not be composited.
    ///
    /// While morphed, `sync_morph_ball_visual` hides the base `PlayerVisual` and
    /// the ball draws instead. The hidden sprite must not produce far-side pieces.
    #[test]
    fn a_drawable_hidden_by_someone_else_is_not_composited() {
        let mut app = app();
        let hidden = app
            .world_mut()
            .spawn((
                PlayerVisual,
                sprite(Vec2::new(24.0, 24.0)),
                Transform::from_translation(Vec3::new(300.0, 300.0, 20.0)),
                Visibility::Hidden,
            ))
            .id();
        app.update();

        assert!(
            candidate(&app, hidden).is_none(),
            "a sprite nobody is drawing became a compositing candidate, so the \
             pane clips a representation that is not on screen"
        );
    }

    /// The other half, and why the rule is not "skip hidden".
    ///
    /// A far-side body is `Hidden` because the compositor replaced it with
    /// clipped pieces. Dropping its candidate would make it flicker. The question
    /// is whether something other than the portal hides it.
    #[test]
    fn a_body_the_portal_itself_hid_keeps_publishing() {
        use ambition_portal2d_presentation::PortalSourceHidden;

        let mut app = app();
        let composited = app
            .world_mut()
            .spawn((
                PlayerVisual,
                sprite(Vec2::new(24.0, 24.0)),
                Transform::from_translation(Vec3::new(300.0, 300.0, 20.0)),
                Visibility::Hidden,
                PortalSourceHidden,
            ))
            .id();
        app.update();

        assert!(
            candidate(&app, composited).is_some(),
            "the compositor's own hidden source stopped being a candidate, so its \
             pieces would vanish and the body would flicker back"
        );
    }

    /// A morphed player's ball is the player, so it must be published.
    ///
    /// While morphed, the base sprite is hidden and `MorphBallVisual` draws at
    /// `WORLD_Z_PLAYER + 0.05` (20.05), above the portal band (at or below
    /// `WORLD_Z_DUMMY`, 10). The ball declares its body with `PresentationOf`, and
    /// the publisher reads that, so any overlay that declares an owner is
    /// composited without changes here.
    #[test]
    fn a_drawable_that_names_its_body_is_published_even_without_the_base_markers() {
        use ambition_platformer2d_shared_tangle::lifecycle::PresentationOf;

        let mut app = app();
        // The body itself, hidden as morphing leaves it.
        let body = app
            .world_mut()
            .spawn((PlayerVisual, sprite(Vec2::new(24.0, 24.0)), Transform::default()))
            .id();
        // Its other representation: no `PlayerVisual`, no `FeatureVisual`, and
        // the only thing drawing.
        let ball = app
            .world_mut()
            .spawn((
                sprite(Vec2::new(16.0, 16.0)),
                Transform::from_translation(Vec3::new(300.0, 300.0, 20.05)),
                GlobalTransform::from(Transform::from_translation(Vec3::new(
                    300.0, 300.0, 20.05,
                ))),
                PresentationOf(body),
            ))
            .id();
        app.update();

        assert!(
            candidate(&app, ball).is_some(),
            "the drawable that IS the player while morphed is not a compositing \
             candidate, so a pane cannot clip it and it draws over the aperture \
             the body is standing behind"
        );
    }

    /// A non-sprite drawable that declares its frame is published from the
    /// declaration. A unit quad scaled to 48x48 (the hit-flash overlay's shape)
    /// publishes a 24x24 half-extent, so the compositor can classify it.
    #[test]
    fn a_declared_non_sprite_drawable_is_published_from_its_declaration() {
        use ambition_platformer2d_shared_tangle::lifecycle::PresentationOf;

        let mut app = app();
        let body = app.world_mut().spawn(PlayerVisual).id();
        let mut transform = Transform::from_translation(Vec3::new(300.0, 300.0, 21.5));
        transform.scale = Vec3::new(48.0, 48.0, 1.0);
        let overlay = app
            .world_mut()
            .spawn((
                ambition_sprite_fx::DeclaredFrame {
                    color_texture: Handle::default(),
                    uv_rect: Vec4::new(0.0, 0.0, 1.0, 1.0),
                    flip_x: false,
                    tint: Vec4::new(1.0, 1.0, 1.0, 1.0),
                    silhouette: true,
                    size: Vec2::ONE,
                    anchor: Vec2::ZERO,
                },
                transform,
                GlobalTransform::from(transform),
                Visibility::Visible,
                PresentationOf(body),
            ))
            .id();
        app.update();
        let published = candidate(&app, overlay).expect(
            "a declared non-sprite drawable is not a compositing candidate, so a \
             pane can only hide it whole",
        );
        assert_eq!(published.drawn_half, Vec2::new(24.0, 24.0));
    }

    /// The pose must be this frame's. The two components disagree here on purpose.
    ///
    /// In production they disagree at this point: propagation runs in
    /// `PostUpdate`, so during `Update` a `GlobalTransform` is last frame's pose
    /// and `sync_visuals` has written the current `Transform`. A fixture that
    /// seeds both the same would hide the failure.
    ///
    /// The gap is large (200 world units), so rounding cannot satisfy the
    /// assertion.
    #[test]
    fn the_candidate_uses_this_frames_transform_not_last_frames_global() {
        let mut app = app();
        let stale = Vec3::new(100.0, 300.0, 11.0);
        let current = Vec3::new(300.0, 300.0, 11.0);
        let entity = app
            .world_mut()
            .spawn((
                PlayerVisual,
                sprite(Vec2::new(40.0, 40.0)),
                Transform::from_translation(current),
                // Last frame's propagated pose, as `PostUpdate` left it.
                GlobalTransform::from(Transform::from_translation(stale)),
            ))
            .id();
        app.update();

        let published = candidate(&app, entity).expect("a player visual is a candidate");
        let expected = ambition_platformer2d_core::config::bevy_size_to_world(
            WORLD,
            current.truncate(),
        );
        assert!(
            (published.drawn_centre.x - expected.x).abs() < 0.001,
            "the candidate must follow the pose written THIS frame: published \
             {:.1} against the current {:.1} (the stale global would give a \
             different x entirely)",
            published.drawn_centre.x,
            expected.x
        );
    }

    /// A rotated non-square sprite does not occupy its unrotated rectangle.
    ///
    /// Non-square makes the test falsifiable: a square's AABB is unchanged by a
    /// 90° turn. 40x10 rotated a quarter turn is 10x40, so the half-extents must
    /// swap.
    #[test]
    fn a_rotated_non_square_sprite_publishes_its_rotated_bounds() {
        let mut app = app();
        let entity = app
            .world_mut()
            .spawn((
                crate::rendering::primitives::FeatureVisual {
                    id: "rolled_npc".to_string(),
                },
                sprite(Vec2::new(40.0, 10.0)),
                Transform::from_translation(Vec3::new(300.0, 300.0, 11.0))
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                GlobalTransform::from(Transform::from_translation(Vec3::new(
                    300.0, 300.0, 11.0,
                ))),
            ))
            .id();
        app.update();

        let published = candidate(&app, entity).expect("a feature visual is a candidate");
        assert!(
            (published.drawn_half.x - 5.0).abs() < 0.01
                && (published.drawn_half.y - 20.0).abs() < 0.01,
            "a quarter-turned 40x10 sprite occupies 10x40, so its half-extents \
             are (5, 20); published ({:.2}, {:.2})",
            published.drawn_half.x,
            published.drawn_half.y
        );
    }

    /// The rotation term leaves the unrotated case unchanged.
    #[test]
    fn an_unrotated_sprite_publishes_exactly_its_half_extents() {
        assert_eq!(
            rotated_half_extent(Vec2::new(20.0, 5.0), Quat::IDENTITY),
            Vec2::new(20.0, 5.0)
        );
    }

    fn candidate(app: &App, entity: Entity) -> Option<PortalCompositingCandidate> {
        app.world().get::<PortalCompositingCandidate>(entity).copied()
    }

    /// The bounds tests use a `FeatureVisual`, which the original query already
    /// covered. So reverting the `PlayerVisual` fix fails only its own test.
    fn feature() -> crate::rendering::primitives::FeatureVisual {
        crate::rendering::primitives::FeatureVisual {
            id: "bounds probe".to_string(),
        }
    }

    /// A far-side player must become a candidate. A player is a `PlayerVisual`,
    /// not a `FeatureVisual`.
    #[test]
    fn a_player_visual_is_published_as_a_candidate() {
        let mut app = app();
        let player = app
            .world_mut()
            .spawn((
                PlayerVisual,
                sprite(Vec2::new(24.0, 48.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 20.0)),
                GlobalTransform::from(Transform::from_translation(Vec3::new(0.0, 0.0, 20.0))),
            ))
            .id();
        app.update();
        assert!(
            candidate(&app, player).is_some(),
            "a PlayerVisual overlapping a pane must be composited like any actor"
        );
    }

    /// A sprite pivots on its anchor; a quad is centre-origin. A feet-anchored
    /// sprite must report the centre of its art, not its pivot.
    #[test]
    fn a_feet_anchored_sprite_reports_the_centre_of_its_art_not_its_pivot() {
        let mut env = app();
        let size = Vec2::new(24.0, 48.0);
        let at = Vec3::new(0.0, 0.0, 20.0);
        let centred = env
            .world_mut()
            .spawn((
                feature(),
                sprite(size),
                Transform::from_translation(at),
                GlobalTransform::from(Transform::from_translation(at)),
                Anchor::CENTER,
            ))
            .id();
        let footed = env
            .world_mut()
            .spawn((
                feature(),
                sprite(size),
                Transform::from_translation(at),
                GlobalTransform::from(Transform::from_translation(at)),
                Anchor::BOTTOM_CENTER,
            ))
            .id();
        env.update();

        let centred = candidate(&env, centred).expect("centred candidate");
        let footed = candidate(&env, footed).expect("feet-anchored candidate");
        assert!(
            (centred.drawn_centre.y - footed.drawn_centre.y).abs() > size.y * 0.4,
            "two sprites at the SAME translation with different anchors reported \
             the same centre ({:?} vs {:?}); the anchor is not being read",
            centred.drawn_centre,
            footed.drawn_centre
        );
        // The half-extent must not change with the anchor: an anchor moves a
        // rectangle, it does not resize it.
        assert_eq!(centred.drawn_half, footed.drawn_half);
    }

    /// Scale is part of what is drawn. Reading `custom_size` alone would subtract
    /// a scaled sprite at its unscaled size.
    #[test]
    fn a_scaled_sprite_reports_its_scaled_extent() {
        let mut env = app();
        let mut at = Transform::from_translation(Vec3::new(0.0, 0.0, 20.0));
        at.scale = Vec3::new(2.0, 3.0, 1.0);
        let entity = env
            .world_mut()
            .spawn((
                feature(),
                sprite(Vec2::new(10.0, 10.0)),
                at,
                GlobalTransform::from(at),
            ))
            .id();
        env.update();
        let published = candidate(&env, entity).expect("candidate");
        assert_eq!(published.drawn_half, Vec2::new(10.0, 15.0));
    }
}

/// The publisher and the compositor, in one app.
///
/// The unit tests above check one side each: the presentation crate's tests
/// build their own `PortalCompositingCandidate`, and the tests above check what
/// this bridge publishes. These tests run both real systems together, so they
/// catch a missing population, wrong bounds, or a missing ordering edge.
///
/// This wires the two systems, not the real plugins, so it cannot see a
/// registration that is missing entirely.
#[cfg(test)]
mod bridge_meets_compositor_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
    use ambition_portal2d_presentation::{PlacedPortal, PortalChannel, PortalChannelColor};
    use ambition_portal2d_presentation::{
        composite_far_side_bodies, PortalViewer, PortalWorldFrame,
    };
    use bevy::sprite::Anchor;

    const WORLD: ambition_platformer2d_core::Vec2 =
        ambition_platformer2d_core::Vec2::new(1000.0, 600.0);

    /// A wall pane facing -x, so "in front" is the low-x side.
    fn pane() -> PlacedPortal {
        PlacedPortal::fixed(
            PortalChannel::Authored(PortalChannelColor::Purple),
            ambition_platformer2d_core::Vec2::new(500.0, 300.0),
            ambition_platformer2d_core::Vec2::new(-1.0, 0.0),
            ambition_platformer2d_core::Vec2::new(9.0, 46.0),
        )
    }

    fn app() -> App {
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "bridge meets compositor",
                WORLD,
                ambition_platformer2d_core::Vec2::new(WORLD.x * 0.5, WORLD.y * 0.5),
                Vec::new(),
            )),
        );
        app.insert_resource(PortalWorldFrame { size: WORLD });
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<TextureAtlasLayout>::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(
            Assets::<ambition_portal2d_presentation::PortalClipMaterial>::default(),
        );
        app.insert_resource(PortalViewer {
            present: true,
            // Well in front of the pane, so a body at high x is far-side.
            eye: ambition_platformer2d_core::Vec2::new(400.0, 300.0),
            ..default()
        });
        app.world_mut().spawn(pane());
        // The order under test: publish, composite, then resolve, in one frame.
        // `resolve_portal_source_visibility` is the only writer of `Visibility`,
        // so the chain must include it for the assertion to match production.
        app.add_systems(
            Update,
            (
                publish_portal_compositing_candidates,
                composite_far_side_bodies,
                ambition_portal2d_presentation::resolve_portal_source_visibility,
            )
                .chain(),
        );
        app
    }

    /// A stationary far-side body must stay hidden on every frame, not only the
    /// frame it is classified on.
    ///
    /// `actors::sync_visuals` writes `Visible`/`Hidden` for every `FeatureVisual`
    /// every frame, before portal presentation. A resolver that writes `Hidden`
    /// only when it inserts its marker is correct on frame N and wrong on N+1.
    ///
    /// This fixture includes that other writer; a portal-only harness cannot see
    /// the failure.
    #[test]
    fn a_far_side_body_stays_hidden_while_another_writer_keeps_showing_it() {
        let mut app = app();
        let body = far_side_player(&mut app);

        // The other visibility owner, running before portal presentation, as
        // the host schedules it.
        fn keep_showing_it(mut bodies: Query<&mut Visibility, With<PlayerVisual>>) {
            for mut visibility in &mut bodies {
                *visibility = Visibility::Visible;
            }
        }
        app.add_systems(
            Update,
            keep_showing_it.before(publish_portal_compositing_candidates),
        );

        for frame in 1..=3 {
            app.update();
            assert_eq!(
                *app.world().get::<Visibility>(body).expect("visibility"),
                Visibility::Hidden,
                "frame {frame}: the far-side body is drawn whole again. Its hide \
                 reason still stands, so the resolver must reassert `Hidden` \
                 after every other writer, not only on the frame it first hid it."
            );
        }
    }

    fn far_side_player(app: &mut App) -> Entity {
        far_side_body(app, PlayerVisual)
    }

    /// One builder for both arms: they differ only by marker, the fact under
    /// test. Separate fixtures could drift in position or size.
    fn far_side_body(app: &mut App, marker: impl Bundle) -> Entity {
        let mut image = Image::default();
        image.texture_descriptor.size.width = 48;
        image.texture_descriptor.size.height = 48;
        let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
        let mut sprite = Sprite::from_image(handle);
        sprite.custom_size = Some(Vec2::new(48.0, 48.0));
        // Engine (505, 300) is behind the pane; convert to the render frame like
        // the shipped code does.
        let frame = PortalWorldFrame { size: WORLD };
        let at = frame.to_render(
            ambition_platformer2d_core::Vec2::new(505.0, 300.0),
            20.0,
        );
        app.world_mut()
            .spawn((
                marker,
                sprite,
                Transform::from_translation(at),
                GlobalTransform::from(Transform::from_translation(at)),
                Anchor::CENTER,
                Visibility::Inherited,
            ))
            .id()
    }

    /// A far-side NPC, end to end. An NPC is a `FeatureVisual`. The player arm
    /// below covers `PlayerVisual`, so a regression in either shows separately.
    #[test]
    fn a_far_side_npc_is_composited_in_the_same_frame_it_is_published() {
        let mut app = app();
        let npc = far_side_body(
            &mut app,
            crate::rendering::primitives::FeatureVisual {
                id: "perfect cellular automaton".to_string(),
            },
        );
        app.update();
        assert!(
            app.world()
                .get::<ambition_portal2d_presentation::PortalCompositingCandidate>(npc)
                .is_some(),
            "the bridge did not publish the NPC"
        );
        assert_eq!(
            *app.world().get::<Visibility>(npc).expect("visibility"),
            Visibility::Hidden,
            "the NPC's whole-sprite draw was not withdrawn, so it still punches \
             through the pane — the reported bug"
        );
    }

    /// The real hit-flash overlay, attached by its own system, is composited like
    /// the sprite it mirrors. The overlay is a `Mesh2d` root that declares its
    /// frame. Here `attach_hit_flash_overlays` spawns it beside a far-side
    /// player, and the chain publishes it, composites it into its own pieces, and
    /// hides the whole mesh, like the sprite.
    #[test]
    fn the_real_hit_flash_overlay_of_a_far_side_body_is_composited_not_hidden_whole() {
        use ambition_portal2d_presentation::{PortalDependantHidden, PortalFarSideHidden};

        let mut app = app();
        app.insert_resource(Assets::<crate::rendering::hit_flash::HitFlashMaterial>::default());
        app.add_systems(
            Update,
            crate::rendering::hit_flash::attach_hit_flash_overlays
                .before(publish_portal_compositing_candidates),
        );
        let body = far_side_player(&mut app);
        // The attach runs before the publisher with a sync point between, so the
        // overlay is published and composited on its spawn frame. Pieces are
        // told apart by look, because both frames include the overlay's pieces.
        app.update();
        let _ = body;

        let overlay = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<crate::rendering::hit_flash::HitFlashOverlay>>();
            q.single(world).expect("the real attach spawned exactly one overlay")
        };
        assert!(
            app.world()
                .get::<ambition_portal2d_presentation::PortalCompositingCandidate>(overlay)
                .is_some(),
            "the real overlay was never published as a candidate"
        );
        assert!(
            app.world().get::<PortalFarSideHidden>(overlay).is_some()
                && app.world().get::<PortalDependantHidden>(overlay).is_none(),
            "the overlay was hidden as a DEPENDANT of its body (the scalar \
             fallback) rather than composited as a candidate in its own right"
        );
        assert_eq!(
            *app.world().get::<Visibility>(overlay).expect("visibility"),
            Visibility::Hidden,
            "the whole silhouette mesh still draws over the pane"
        );
        let (sprite_pieces, silhouette_pieces) = pieces_by_look(&mut app);
        assert!(
            sprite_pieces >= 1,
            "premise: the body's own sprite composites into pieces"
        );
        assert!(
            silhouette_pieces >= 1,
            "the overlay was hidden but nothing redraws its uncovered part as a \
             silhouette: {sprite_pieces} sprite pieces, {silhouette_pieces} \
             silhouette pieces"
        );
    }

    fn candidate(
        app: &App,
        entity: Entity,
    ) -> Option<ambition_portal2d_presentation::PortalCompositingCandidate> {
        app.world()
            .get::<ambition_portal2d_presentation::PortalCompositingCandidate>(entity)
            .copied()
    }

    /// Far-side pieces, split by what they paint: `(sampled sprite, silhouette)`.
    fn pieces_by_look(app: &mut App) -> (usize, usize) {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &MeshMaterial2d<ambition_portal2d_presentation::PortalClipMaterial>,
            With<ambition_portal2d_presentation::PortalFarSidePiece>,
        >();
        let handles: Vec<_> = q.iter(world).map(|m| m.0.clone()).collect();
        let materials = world.resource::<Assets<ambition_portal2d_presentation::PortalClipMaterial>>();
        handles.iter().fold((0, 0), |(sprite, silhouette), h| {
            let m = materials.get(h).expect("piece material");
            if m.control.z > 0.5 {
                (sprite, silhouette + 1)
            } else {
                (sprite + 1, silhouette)
            }
        })
    }

    /// The bridge app plus the real body-owned drawable writers, wired like
    /// production: writers in `BodyOwnedDrawableSync`, the publisher after that
    /// set. The set edge is the command flush, so a drawable spawned inside the
    /// set is a candidate on its first frame.
    fn app_with_body_drawables() -> App {
        use crate::rendering::BodyOwnedDrawableSync;
        let mut app = app();
        // What `ImagePlugin` inserts in a real app: the 1x1 white image under the
        // default handle, which a colour sprite samples. Without it the
        // compositor cannot rebuild the bar and gives it back whole.
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(&Handle::default(), Image::default())
            .expect("the default image handle is insertable");
        app.init_resource::<ambition_sim_view::BodyClocksView>();
        app.init_resource::<ambition_time::SimTick>();
        app.init_resource::<ambition_sim_view::FeatureViewIndex>();
        app.init_resource::<ambition_sim_view::ActorAnimIndex>();
        app.init_resource::<
            ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveDefensePresentationPolicy,
        >();
        app.insert_resource(Assets::<crate::rendering::hit_flash::HitFlashMaterial>::default());
        app.add_systems(
            Update,
            (
                crate::rendering::hit_flash::attach_hit_flash_overlays,
                crate::rendering::hit_flash::sync_hit_flash_overlays,
                crate::rendering::body_clock::sync_body_clock_visuals,
            )
                .chain()
                .in_set(BodyOwnedDrawableSync),
        );
        app.configure_sets(
            Update,
            BodyOwnedDrawableSync.before(publish_portal_compositing_candidates),
        );
        app
    }

    /// The clock bar is classified on the frame it appears, at its current
    /// position. `sync_body_clock_visuals` spawns and moves the bar through
    /// commands and a transform write, so the publisher needs the set edge. The
    /// bar straddles the pane edge, so "composited" means hidden and redrawn.
    #[test]
    fn a_clock_bar_is_composited_on_its_first_frame_and_follows_its_body() {
        use crate::rendering::body_clock::BodyClockVisual;
        use ambition_sim_view::{BodyClockFact, BodyClocksView};

        let mut app = app_with_body_drawables();
        let body = far_side_player(&mut app);
        // A full clock on a body whose head is just under the pane's top edge.
        // The bar (28x4) straddles the pane's x extent, so part of it is covered.
        let fact = |remaining_fraction: f32, x: f32| BodyClockFact {
            body,
            pos: ambition_platformer2d_core::Vec2::new(x, 300.0),
            half_height: 16.0,
            remaining_fraction,
        };
        app.world_mut().resource_mut::<BodyClocksView>().0 = vec![fact(1.0, 505.0)];
        app.update();

        let bar = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<BodyClockVisual>>();
            q.single(world).expect("one clock, one bar")
        };
        let first = candidate(&app, bar).expect(
            "the bar was drawn this frame and never became a compositing \
             candidate: a pane cannot clip it on the frame it appears",
        );
        assert_eq!(
            *app.world().get::<Visibility>(bar).expect("visibility"),
            Visibility::Hidden,
            "the bar overlaps a far-side pane and still draws whole"
        );

        // The clock runs down and the body moves clear of the pane.
        app.world_mut().resource_mut::<BodyClocksView>().0 = vec![fact(0.5, 540.0)];
        app.update();
        let moved = candidate(&app, bar).expect("still a candidate");
        assert!(
            moved.drawn_centre.x > first.drawn_centre.x + 20.0,
            "the candidate was published from LAST frame's bar ({:.0}) rather than \
             this frame's ({:.0})",
            first.drawn_centre.x,
            moved.drawn_centre.x
        );
        assert!(
            moved.drawn_half.x < first.drawn_half.x,
            "the bar shrank and the candidate did not: {:?} -> {:?}",
            first.drawn_half,
            moved.drawn_half
        );
        assert_ne!(
            *app.world().get::<Visibility>(bar).expect("visibility"),
            Visibility::Hidden,
            "the bar moved clear of the pane and is still hidden"
        );
    }

    /// No missing frame on the way back. Frame N: the flashing body is far-side
    /// and its silhouette is hidden with the portal's marker. Frame N+1: the body
    /// crosses to the near side. The resolver drops its claim without writing a
    /// value, so the overlay's own owner must have written `Visible` earlier that
    /// frame, even while last frame's marker is still present.
    #[test]
    fn a_flashing_silhouette_is_back_the_frame_its_body_returns_to_the_near_side() {
        use crate::rendering::hit_flash::HitFlashOverlay;

        let mut app = app_with_body_drawables();
        let body = far_side_player(&mut app);
        // The flash is active: the pose row the overlay reads has a timer.
        app.world_mut()
            .entity_mut(body)
            .insert(ambition_sim_view::BodyPoseView {
                hit_flash_secs: 0.5,
                ..Default::default()
            });
        app.update();
        app.update();
        let overlay = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<HitFlashOverlay>>();
            q.single(world).expect("one overlay")
        };
        assert_eq!(
            *app.world().get::<Visibility>(overlay).expect("visibility"),
            Visibility::Hidden,
            "premise: the far-side silhouette is composited (hidden whole)"
        );
        assert!(
            app.world()
                .get::<ambition_portal2d_presentation::PortalSourceHidden>(overlay)
                .is_some(),
            "premise: the portal holds the claim"
        );

        // The body crosses to the near side: engine x 480 is in front of the pane.
        let frame = PortalWorldFrame { size: WORLD };
        let near = frame.to_render(ambition_platformer2d_core::Vec2::new(480.0, 300.0), 20.0);
        *app.world_mut().get_mut::<Transform>(body).expect("pose") =
            Transform::from_translation(near);
        app.update();
        assert_ne!(
            *app.world().get::<Visibility>(overlay).expect("visibility"),
            Visibility::Hidden,
            "the body is near-side, the portal has no reason, and the silhouette \
             is still hidden: one missing frame of flash on every crossing"
        );
    }

    /// A far-side player, published by the real bridge and composited by the
    /// real compositor, in one frame.
    #[test]
    fn a_far_side_player_is_composited_in_the_same_frame_it_is_published() {
        let mut app = app();
        let player = far_side_player(&mut app);
        app.update();
        assert!(
            app.world()
                .get::<ambition_portal2d_presentation::PortalCompositingCandidate>(player)
                .is_some(),
            "the bridge did not publish the player"
        );
        assert_eq!(
            *app.world().get::<Visibility>(player).expect("visibility"),
            Visibility::Hidden,
            "published but not composited: the two systems did not meet this frame"
        );
    }
}
