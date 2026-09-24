// ---------------------------------------------------------------------------
// Morph ball sprite (procedural)
// ---------------------------------------------------------------------------
//
// The shipped player spritesheet has no `MorphBall` row, but we still want
// the morph ball to look distinct from a crouched robot mid-game. Generating
// a small RGBA circle at startup avoids a parallel "render Morph row" task
// in the gen2d toolchain and keeps the mechanic playable today. Future art
// can replace this with a real spritesheet row by setting the
// `MorphBallSprite` handle to a loaded asset and the same toggle logic
// applies.

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Procedural sphere texture. Built once at startup; every morphed body's ball
/// draws it.
#[derive(Resource, Clone, Default)]
pub struct MorphBallSprite {
    pub handle: Handle<Image>,
}

/// Marker on a morph-ball sprite. Each one draws exactly one body, named by
/// its `PresentationOf`, and follows that body's own pose.
#[derive(Component)]
pub struct MorphBallVisual;

const MORPH_BALL_TEXTURE_SIZE: u32 = 64;

/// Generate a 64x64 RGBA circle with a soft anti-aliased rim and a
/// top-left highlight so the ball reads as a sphere even at small render
/// sizes. Color matches the steel-blue palette of the player robot's
/// fallback rectangle (`Color::srgba(0.80, 0.95, 1.0, 1.0)`) so the
/// visual ties back to the standing body.
pub fn build_morph_ball_image() -> Image {
    let size = MORPH_BALL_TEXTURE_SIZE;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let cx = (size as f32 - 1.0) * 0.5;
    let cy = cx;
    let radius = size as f32 * 0.5;
    // Anti-alias band width (pixels): edge fades from 1.0 → 0.0 alpha
    // across this many pixels at the sphere boundary.
    let edge = 1.5_f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = ((radius - dist) / edge).clamp(0.0, 1.0);
            // Top-left highlight: dot product with (-0.7, -0.7) direction.
            let nx = if dist > 0.001 { dx / radius } else { 0.0 };
            let ny = if dist > 0.001 { dy / radius } else { 0.0 };
            let highlight_dot = (-nx * 0.7 - ny * 0.7).clamp(0.0, 1.0);
            let highlight = highlight_dot.powf(2.5) * 0.55;
            // Rim shading: darker near the edge for spherical depth.
            let rim_factor = (1.0 - (dist / radius).powf(3.0)).clamp(0.0, 1.0);
            let base = 0.35 + 0.40 * rim_factor;
            let value = (base + highlight).clamp(0.0, 1.0);
            // Steel-blue tint: r=0.80, g=0.95, b=1.0 multiplied by value.
            let r = (value * 0.80 * 255.0) as u8;
            let g = (value * 0.95 * 255.0) as u8;
            let b = (value * 1.00 * 255.0) as u8;
            let a = (alpha * 255.0) as u8;
            let i = ((y * size + x) * 4) as usize;
            data[i] = r;
            data[i + 1] = g;
            data[i + 2] = b;
            data[i + 3] = a;
        }
    }
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Startup system: build the procedural morph ball image and stash its
/// handle.
pub fn build_morph_ball_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_morph_ball_image());
    commands.insert_resource(MorphBallSprite { handle });
}

/// Give every morphed drawn body its own ball, the first frame it is morphed.
///
/// The ball names its body at spawn (`PresentationOf`), so what draws a morphed
/// body is answerable from the ball alone — portal composition asks exactly
/// that. Chained before [`sync_morph_ball_visual`], so a body is never hidden on
/// a frame its ball does not yet exist.
pub fn spawn_morph_ball_visual(
    mut commands: Commands,
    sprite: Option<Res<MorphBallSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    bodies: Query<
        (Entity, &ambition_sim_view::BodyPoseView),
        With<ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual>,
    >,
    balls: Query<
        &ambition_platformer2d_shared_tangle::lifecycle::PresentationOf,
        With<MorphBallVisual>,
    >,
) {
    let Some(sprite) = sprite else {
        return;
    };
    if sprite.handle == Handle::default() {
        return;
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (body, pose) in &bodies {
        if !pose.morph_ball || balls.iter().any(|owner| owner.0 == body) {
            continue;
        }
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite {
                    image: sprite.handle.clone(),
                    custom_size: Some(bevy::math::Vec2::new(16.0, 16.0)),
                    ..default()
                },
                Transform::from_xyz(
                    0.0,
                    0.0,
                    ambition_platformer2d_core::config::WORLD_Z_PLAYER + 0.05,
                ),
                Visibility::Hidden,
                MorphBallVisual,
                ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(body),
                Name::new("Morph Ball Visual"),
            ),
        );
    }
}

/// Draw each ball where its own body is presented while that body is morphed,
/// and hide each morphed body's standing sprite so the rig does not show
/// through. A ball whose body is gone is despawned.
///
/// ⛔ Every drawn body answers from its OWN `BodyPoseView`, never "the primary
/// player": a match or a possession has bodies that are not the session's home
/// avatar, and a hide keyed on one singleton skipped them in silence.
pub fn sync_morph_ball_visual(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut bodies: Query<
        (
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
            &mut Visibility,
        ),
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual>,
            Without<MorphBallVisual>,
        ),
    >,
    mut balls: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::PresentationOf,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        ),
        With<MorphBallVisual>,
    >,
) {
    for (ball, owner, mut transform, mut sprite, mut ball_visibility) in &mut balls {
        let Ok((pose, presented, _)) = bodies.get(owner.0) else {
            commands.entity(ball).despawn();
            continue;
        };
        if pose.morph_ball {
            transform.translation = ambition_platformer2d_core::config::world_to_bevy(
                &world.0,
                // The sphere IS the body while morphed, so it draws where the
                // body is presented — not where its last tick left it.
                ambition_sim_view::presented_pose::draw_pos(pose, presented),
                ambition_platformer2d_core::config::WORLD_Z_PLAYER + 0.05,
            );
            // Slightly larger than the AABB so the soft anti-aliased rim
            // reads as the ball's outline rather than as background.
            sprite.custom_size = Some(bevy::math::Vec2::new(pose.size.x * 1.10, pose.size.y * 1.10));
            *ball_visibility = Visibility::Visible;
        } else {
            *ball_visibility = Visibility::Hidden;
        }
    }
    for (pose, _, mut body_visibility) in &mut bodies {
        if pose.morph_ball {
            if *body_visibility != Visibility::Hidden {
                *body_visibility = Visibility::Hidden;
            }
        } else if matches!(*body_visibility, Visibility::Hidden) {
            // Back to `Inherited`, never a hard `Visible`, so the death overlay
            // and the room-transition fade keep their authority over the body.
            *body_visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::{PlayerVisual, PresentationOf};
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
    use ambition_sim_view::BodyPoseView;

    fn pose(morph: bool) -> BodyPoseView {
        BodyPoseView {
            morph_ball: morph,
            size: ambition_platformer2d_core::Vec2::new(24.0, 24.0),
            ..Default::default()
        }
    }

    /// Drawn bodies as a match spawns them — the first is the session's own
    /// avatar, the rest are fighters with no `PrimaryPlayer` — run through the
    /// shipped spawn → sync chain.
    fn rig(bodies: &[bool]) -> (App, Vec<Entity>) {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "t",
                ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                ambition_platformer2d_core::Vec2::ZERO,
                Vec::new(),
            )),
        );
        let mut ids = Vec::new();
        for (i, morph) in bodies.iter().enumerate() {
            let mut body = app.world_mut().spawn((
                PlayerVisual,
                PlayerEntity,
                pose(*morph),
                Visibility::Inherited,
                Transform::default(),
            ));
            if i == 0 {
                body.insert(PrimaryPlayer);
            }
            ids.push(body.id());
        }
        app.add_systems(Startup, build_morph_ball_sprite);
        app.add_systems(Update, (spawn_morph_ball_visual, sync_morph_ball_visual).chain());
        (app, ids)
    }

    fn vis(app: &App, e: Entity) -> Visibility {
        *app.world().get::<Visibility>(e).unwrap()
    }

    /// The ball drawing `body`, if it has one.
    fn ball_of(app: &mut App, body: Entity) -> Option<Entity> {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &PresentationOf), With<MorphBallVisual>>();
        q.iter(app.world())
            .find(|(_, owner)| owner.0 == body)
            .map(|(ball, _)| ball)
    }

    fn set_morph(app: &mut App, body: Entity, morph: bool) {
        app.world_mut().get_mut::<BodyPoseView>(body).unwrap().morph_ball = morph;
    }

    /// In morph the ball shows and the body's sprite is hidden, on the same
    /// frame — otherwise the standing rig draws through the ball, or the body
    /// is invisible for a frame.
    #[test]
    fn entering_morph_hides_the_body_sprite_and_shows_its_ball() {
        let (mut app, bodies) = rig(&[true]);
        app.update();
        let ball = ball_of(&mut app, bodies[0]).expect("the morphed body has a ball");
        assert_eq!(vis(&app, ball), Visibility::Visible, "the ball draws");
        assert_eq!(vis(&app, bodies[0]), Visibility::Hidden, "the rig does not draw through it");
    }

    /// ⛔⛔ A MORPHED FIGHTER THAT IS NOT THE PRIMARY PLAYER GETS ITS OWN BALL.
    /// The ball was one session singleton that followed the primary player while
    /// the hide was per body, so any other morphed body was hidden and drawn by
    /// nothing at all.
    #[test]
    fn a_morphed_body_that_is_not_the_primary_player_is_drawn_by_its_own_ball() {
        let (mut app, bodies) = rig(&[false, true]);
        app.update();
        assert_eq!(vis(&app, bodies[1]), Visibility::Hidden);
        let ball = ball_of(&mut app, bodies[1])
            .expect("the morphed fighter is hidden and nothing draws it");
        assert_eq!(vis(&app, ball), Visibility::Visible);
        assert!(ball_of(&mut app, bodies[0]).is_none(), "a body that never morphed gets no ball");
        assert_eq!(vis(&app, bodies[0]), Visibility::Inherited, "and keeps drawing");
    }

    /// Two morphed bodies are two balls, each naming its own body — which is
    /// what portal composition asks (`PresentationOf`).
    #[test]
    fn two_morphed_bodies_are_two_balls() {
        let (mut app, bodies) = rig(&[true, true]);
        app.update();
        let a = ball_of(&mut app, bodies[0]).expect("first ball");
        let b = ball_of(&mut app, bodies[1]).expect("second ball");
        assert_ne!(a, b);
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<MorphBallVisual>>();
        assert_eq!(q.iter(app.world()).count(), 2, "a ball is spawned once per body, not per frame");
    }

    /// Leaving morph restores the body to `Inherited` — never a hard `Visible`,
    /// so the death overlay and the room-transition fade keep their authority.
    #[test]
    fn leaving_morph_returns_the_body_to_inherited_not_visible() {
        let (mut app, bodies) = rig(&[true]);
        app.update();
        let ball = ball_of(&mut app, bodies[0]).unwrap();
        set_morph(&mut app, bodies[0], false);
        app.update();
        assert_eq!(vis(&app, ball), Visibility::Hidden);
        assert_eq!(vis(&app, bodies[0]), Visibility::Inherited);
    }

    /// A ball whose body is gone goes with it.
    #[test]
    fn a_ball_leaves_with_its_body() {
        let (mut app, bodies) = rig(&[true]);
        app.update();
        let ball = ball_of(&mut app, bodies[0]).unwrap();
        app.world_mut().despawn(bodies[0]);
        app.update();
        assert!(app.world().get_entity(ball).is_err(), "an orphaned ball kept drawing");
    }

    /// A body that never morphs is never touched and gets no ball.
    #[test]
    fn a_body_that_is_not_in_morph_is_left_alone() {
        let (mut app, bodies) = rig(&[false]);
        app.update();
        assert!(ball_of(&mut app, bodies[0]).is_none());
        assert_eq!(vis(&app, bodies[0]), Visibility::Inherited);
    }
}
