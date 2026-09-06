//! Publish the drawables a portal pane may have to composite against.
//!
//! ⛔⛔ THE PORTAL PRESENTATION CRATE CANNOT SEE ORDINARY ACTORS. Its body seams
//! are `PortalSceneBody` (ONE entity — whose sprite is decomposed at the seam)
//! and `PortalAffordanceBody` (whoever operates the portals). An NPC standing
//! behind an aperture is neither, so that crate had no way to know it exists —
//! while THIS crate drew it at `WORLD_Z_DUMMY + 1.0`, above every pane. A
//! far-side actor punching through a seamless window (Jon, 2026-09-05) is
//! invisible from the side that would have to fix it.
//!
//! ⭐ THE HOST PUBLISHES THE FACT, which is the same shape as every other seam
//! that crate exposes: it does not reach into this one, and this one does not
//! learn what a pane is.

use bevy::prelude::*;

/// Publish each drawn actor sprite as a compositing candidate, in ENGINE
/// coordinates.
///
/// ⚠ DRAWN BOUNDS, NOT THE COLLISION BOX. `Sprite::custom_size` is what actually
/// paints; the collision footprint is routinely smaller, and the difference IS
/// the overhang that punches through the window. Publishing the box would build
/// a report that misses the finding it exists for.
///
/// ⚠ SPRITES WITH NO `custom_size` ARE SKIPPED rather than guessed at. A sprite
/// sized by its texture has bounds this system cannot know without the atlas,
/// and inventing one would put a confident wrong rectangle into a diagnostic
/// whose whole job is to be trusted.
pub fn publish_portal_compositing_candidates(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    // ⛔⛔ THE PLAYER IS NOT A `FeatureVisual`. This query was `With<FeatureVisual>`
    // alone, and the exploration player is spawned with `PlayerVisual`
    // (`session/setup.rs`), so a far-side PLAYER never became a candidate and
    // kept punching through the pane -- the exact case the compositor exists for,
    // excluded at the door. Found by a GPT review 2026-09-05.
    //
    // ⚠ The crate-level test that claimed this case CONSTRUCTS its own candidate,
    // so it proved the compositor is z-independent and could never witness a
    // population it was never given.
    // ⛔⛔ THE LOCAL `Transform`, NOT `GlobalTransform`, AND THAT IS THE WHOLE
    // POINT. Transform propagation runs in `PostUpdate`, so a `GlobalTransform`
    // read during `Update` still describes the PREVIOUS frame -- while
    // `actors::sync_visuals` has already written this frame's local `Transform`
    // earlier in this same run. Publishing a candidate that mixed this frame's
    // `Sprite` with last frame's pose made the compositor subtract a region the
    // body had already left, so pieces lagged and re-revealed pixels the pane
    // should hide. Found by a GPT review 2026-09-05.
    //
    // ⚠ THE SUBSTITUTION IS ONLY SOUND BECAUSE THESE SPRITES ARE UNPARENTED --
    // actor and feature visuals are spawned as top-level world-space entities,
    // so local IS world. `Without<ChildOf>` states that as a requirement rather
    // than an assumption: a parented drawable would need the propagated pose and
    // must not be silently published with a local one.
    drawables: Query<
        (Entity, &Sprite, &Transform, Option<&bevy::sprite::Anchor>),
        (
            Or<(
                With<crate::rendering::primitives::FeatureVisual>,
                With<ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual>,
                // ⛔⛔ A BODY'S OTHER REPRESENTATIONS DRAW TOO, and the base
                // markers do not find them. While a player is MORPHED its
                // `PlayerVisual` sprite is hidden and the ball IS the player --
                // a separate root at `WORLD_Z_PLAYER + 0.05` = 20.05, far above
                // the portal band pinned at or below `WORLD_Z_DUMMY` = 10. It
                // therefore drew straight over a pane while the hidden base
                // sprite was the only thing this publisher could see.
                // ⇒ `PresentationOf` is how such a drawable says whose body it
                // draws, so asking for it here is asking the question the
                // compositor could not previously form. Raised by a GPT review
                // 2026-09-06.
                With<ambition_platformer2d_shared_tangle::lifecycle::PresentationOf>,
            )>,
            Without<bevy::prelude::ChildOf>,
        ),
    >,
) {
    // ⚠ `SessionWorldRef` is a `Single`, so this system simply does not run
    // without a session world -- which is the honest behaviour: there is no
    // coordinate frame to publish engine positions in.
    let size = world.0.size;
    for (entity, sprite, transform, anchor) in &drawables {
        let Some(drawn) = sprite.custom_size else {
            continue;
        };
        // ⛔⛔ A SPRITE PIVOTS ON ITS ANCHOR; A QUAD IS CENTRE-ORIGIN. Character
        // sprites are FEET-anchored (`feet_anchor_for_render_size`), so the
        // drawn rectangle's centre is nowhere near the transform translation --
        // it is most of a body-height above it. Publishing the translation as
        // the centre handed the compositor a rectangle offset by that much, and
        // it then subtracted the wrong region.
        //
        // ⭐ Derived by the SAME helper the compositor uses to place its pieces,
        // rather than a second copy of the rule here. If these two ever disagreed
        // about where a sprite is, the subtracted region and the drawn region
        // would differ -- which is the whole defect, one layer down.
        let posed = ambition_portal2d_presentation::clip_piece_transform(
            transform,
            anchor.map_or(Vec2::ZERO, |a| a.0),
            drawn,
        );
        let bevy_centre = posed.translation.truncate();
        // ⭐ The ONE definition of the y-flip, called rather than repeated.
        let centre =
            ambition_platformer2d_core::config::bevy_size_to_world(size, bevy_centre);
        commands
            .entity(entity)
            .insert(ambition_portal2d_presentation::PortalCompositingCandidate {
                drawn_centre: centre,
                // ⚠ A y-flip moves a CENTRE, never a size -- but SCALE does
                // change a size, and `clip_piece_transform` folds the sprite
                // scale into the quad it poses, so the half-extent reads it back
                // from there rather than from `custom_size` alone.
                //
                // ⛔ AND ROTATION CHANGES IT TOO. The candidate is a world-space
                // AABB; a ROTATED non-square sprite does not occupy its
                // unrotated rectangle, so publishing the scaled half-extents
                // under-reported the region for any rolled body (`ActorRoll` --
                // an aerial/gravity roll is ordinary, not exotic) and the pane
                // then failed to subtract the corners that actually overhang it.
                drawn_half: rotated_half_extent(
                    posed.scale.truncate().abs() * 0.5,
                    posed.rotation,
                ),
            });
    }
}

/// World-space AABB half-extent of a rectangle rotated about its own centre.
///
/// ⭐ THE STANDARD ABSOLUTE-ROTATION FORM, not a corner sweep: for a rotation of
/// θ the extents are `|cos|*hx + |sin|*hy` and `|sin|*hx + |cos|*hy`. It is
/// exact for every angle, and reduces to the identity at θ = 0 so an unrotated
/// sprite publishes precisely what it did before this existed.
///
/// ⚠ Only the Z rotation is meaningful here — these are 2D world sprites, and a
/// quad rolled out of the XY plane is not something this projection can describe
/// honestly.
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

    /// ⛔⛔ A MORPHED PLAYER'S BALL IS THE PLAYER, and it was invisible to this
    /// publisher.
    ///
    /// While morphed the base `PlayerVisual` sprite is HIDDEN and
    /// `MorphBallVisual` draws instead — a separate root at
    /// `WORLD_Z_PLAYER + 0.05` = 20.05, far above the portal band this crate
    /// pins at or below `WORLD_Z_DUMMY` = 10. The publisher's population was
    /// `FeatureVisual OR PlayerVisual`, so the only thing it could see was the
    /// hidden sprite, and the ball drew straight over any pane it stood behind.
    ///
    /// ⭐ IT IS NOT A PORTAL SPECIAL CASE FOR BALLS. The ball says whose body it
    /// draws (`PresentationOf`), and this publisher asks that question — so the
    /// next overlay that declares an owner is composited without touching this
    /// file. Raised by a GPT review 2026-09-06.
    #[test]
    fn a_drawable_that_names_its_body_is_published_even_without_the_base_markers() {
        use ambition_platformer2d_shared_tangle::lifecycle::PresentationOf;

        let mut app = app();
        // The body itself, hidden as morphing leaves it.
        let body = app
            .world_mut()
            .spawn((PlayerVisual, sprite(Vec2::new(24.0, 24.0)), Transform::default()))
            .id();
        // Its OTHER representation: no `PlayerVisual`, no `FeatureVisual`, and
        // the only thing actually drawing.
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

    /// ⛔⛔ THE POSE MUST BE THIS FRAME'S, and the two components are made to
    /// DISAGREE here on purpose.
    ///
    /// In production they always disagree at this moment: transform propagation
    /// runs in `PostUpdate`, so during `Update` a `GlobalTransform` still holds
    /// the previous frame's pose while `sync_visuals` has already written the
    /// current one to the local `Transform`. Every earlier bridge test seeded
    /// the two IDENTICALLY, which removed exactly the failure from the fixture --
    /// so the publisher could read the stale one and stay green.
    ///
    /// ⚠ The gap is deliberately large (200 world units) so the assertion cannot
    /// be satisfied by rounding: a stale read lands at the OLD centre, and there
    /// is no interpretation under which that is "close enough".
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

    /// A rotated NON-SQUARE sprite does not occupy its unrotated rectangle.
    ///
    /// ⭐ Non-square is what makes the assertion falsifiable: a square's AABB is
    /// rotation-invariant at 90°, so a square fixture would pass with the
    /// rotation term deleted. 40x10 rotated a quarter turn is 10x40, and the
    /// half-extents must swap.
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

    /// The unrotated case is untouched by the rotation term — stated because a
    /// geometry change that quietly moved every ordinary candidate would be a
    /// far worse bug than the one it fixed.
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

    /// ⚠ The BOUNDS tests use a `FeatureVisual`, which was published before the
    /// population was widened. Otherwise reverting the population fix would
    /// redden them too, and a poison that fails everything proves nothing about
    /// which claim it broke.
    fn feature() -> crate::rendering::primitives::FeatureVisual {
        crate::rendering::primitives::FeatureVisual {
            id: "bounds probe".to_string(),
        }
    }

    /// ⛔⛔ THE PLAYER IS NOT A `FeatureVisual`, AND THE QUERY ONLY ASKED FOR
    /// THAT. A far-side PLAYER therefore never became a candidate and kept
    /// punching through the pane — the exact case the compositor exists for,
    /// excluded at the door. Found by review, 2026-09-05.
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

    /// ⛔⛔ A SPRITE PIVOTS ON ITS ANCHOR; A QUAD IS CENTRE-ORIGIN. Character
    /// sprites are FEET-anchored, so publishing the transform translation as the
    /// drawn centre put the rectangle most of a body-height below the art, and
    /// the compositor then subtracted the wrong region.
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
        // ⚠ The half-extent must NOT move with the anchor: an anchor relocates a
        // rectangle, it never resizes one.
        assert_eq!(centred.drawn_half, footed.drawn_half);
    }

    /// ⚠ Scale is part of "what is drawn" too, and reading `custom_size` alone
    /// misses it — a scaled sprite would be subtracted at its unscaled size.
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

/// The publisher and the compositor, in ONE app.
///
/// ⛔⛔ EVERY TEST BEFORE THIS ONE EXERCISED EXACTLY ONE SIDE. The presentation
/// crate's arms build their own `PortalCompositingCandidate`; the arms above
/// check what this bridge publishes. Three production defects lived in the gap
/// between them and all of them were green on both sides: the query excluded
/// `PlayerVisual`, the bounds ignored the anchor, and publication had no
/// ordering edge, so the compositor could read a candidate that was not there
/// yet.
///
/// ⚠ Still not the assembled host — this wires the two real systems rather than
/// the real plugins, so it cannot see a registration that is missing entirely.
/// It CAN see the three defects above, which is what the gap actually contained.
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
            // Well in FRONT of the pane, so a body at high x is FAR.
            eye: ambition_platformer2d_core::Vec2::new(400.0, 300.0),
            ..default()
        });
        app.world_mut().spawn(pane());
        // ⭐ THE ORDER UNDER TEST: publish, then composite, then RESOLVE, in one
        // frame. The compositor states a reason to hide the source; since
        // 2026-09-05 `resolve_portal_source_visibility` is the only writer of
        // `Visibility`, so the chain has to reach it for this assertion to be
        // about the production picture rather than an intermediate one.
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

    /// ⛔⛔ A STATIONARY FAR-SIDE BODY MUST STAY HIDDEN ON EVERY FRAME, not only
    /// the one it was classified on.
    ///
    /// Found by a GPT review 2026-09-05, and the crate's own tests could not see
    /// it: they step ONE frame, and the resolver's first version wrote `Hidden`
    /// only when it inserted its marker. That is correct only if nothing else
    /// writes the component — and `actors::sync_visuals` writes
    /// `*visibility = if view.visible { Visible } else { Hidden }`
    /// UNCONDITIONALLY for every `FeatureVisual`, every frame, before portal
    /// presentation runs.
    /// ⇒ Frame N: hidden and correct. Frame N+1: `sync_visuals` restores
    /// `Visible`, the resolver sees its own marker and writes nothing, and the
    /// whole sprite punches through the pane again — the reported bug, one frame
    /// later.
    ///
    /// ⭐ THIS FIXTURE COMPOSES THE REAL WRITER, which is the only reason it can
    /// witness the defect. A portal-only harness cannot: the thing that undoes
    /// the hide is not a portal system.
    #[test]
    fn a_far_side_body_stays_hidden_while_another_writer_keeps_showing_it() {
        let mut app = app();
        let body = far_side_player(&mut app);

        // The other visibility owner, running BEFORE portal presentation exactly
        // as the host schedules it.
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

    /// ⭐ ONE builder for both arms: the two differ ONLY by which marker they
    /// carry, which is exactly the fact under test. A second fixture would let
    /// them drift in position or size and quietly stop comparing like with like.
    fn far_side_body(app: &mut App, marker: impl Bundle) -> Entity {
        let mut image = Image::default();
        image.texture_descriptor.size.width = 48;
        image.texture_descriptor.size.height = 48;
        let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
        let mut sprite = Sprite::from_image(handle);
        sprite.custom_size = Some(Vec2::new(48.0, 48.0));
        // Engine (505, 300) is BEHIND the pane; convert to the render frame the
        // way the shipped code does rather than hand-placing it.
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

    /// ⭐⭐ JON'S REPORTED CASE, END TO END: a far-side NPC. The screenshot was a
    /// Perfect Cellular Automaton punching through a seamless window, and an NPC
    /// is a `FeatureVisual` — the population the bridge always had. The player
    /// arm below covers the half that was EXCLUDED; this one covers the half the
    /// bug was actually reported about, so a regression in either is visible
    /// separately.
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

    /// ⛔⛔ THE WHOLE POINT: a far-side PLAYER, published by the real bridge and
    /// composited by the real compositor, IN ONE FRAME. With the shipped
    /// `With<FeatureVisual>` filter this body was never a candidate, so it kept
    /// its sprite and punched through the pane.
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
