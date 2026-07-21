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

use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Procedural sphere texture. Built once at startup, shown on a sibling
/// of the player while `Player::body_mode == MorphBall`.
#[derive(Resource, Clone, Default)]
pub struct MorphBallSprite {
    pub handle: Handle<Image>,
}

/// Marker on the morph-ball sibling sprite. The sprite is hidden by
/// default and mirrored to the player's position by
/// `sync_morph_ball_visual` when active.
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
/// handle. The sibling visual is spawned by
/// `spawn_morph_ball_visual` once the sprite handle is ready.
pub fn build_morph_ball_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_morph_ball_image());
    commands.insert_resource(MorphBallSprite { handle });
}

/// Spawn the morph-ball sibling sprite tied to the primary player body.
/// Runs each frame but inserts only when the `MorphBallVisual` query is
/// empty — equivalent to a one-shot "after the ball visual exists"
/// guard that handles the visible-binary boot order without needing a
/// dedicated state.
pub fn spawn_morph_ball_visual(
    mut commands: Commands,
    sprite: Option<Res<MorphBallSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    existing: Query<(), With<MorphBallVisual>>,
) {
    if !existing.is_empty() {
        return;
    }
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
                ambition_engine_core::config::WORLD_Z_PLAYER + 0.05,
            ),
            Visibility::Hidden,
            MorphBallVisual,
            Name::new("Morph Ball Visual"),
        ),
    );
}

/// Toggle the morph-ball visual on / off based on `Player::body_mode`,
/// mirror its position to the player, and scale it to the morph-ball
/// AABB. Hides the regular player sprite while the ball is active so
/// the standing-rig animation doesn't show through.
pub fn sync_morph_ball_visual(
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    // Sim-built pose read-model (E4): body-mode + geometry facts, no live
    // cluster reads.
    player_q: Query<
        (
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
        ),
        ambition_platformer_primitives::markers::PrimaryPlayerOnly,
    >,
    // The primary player's visual is discovered by marker, not a process-global
    // handle: `PrimaryPlayer` names the home avatar, `PlayerVisual` its sprite.
    mut player_query: Query<
        &mut Visibility,
        (
            With<ambition_platformer_primitives::lifecycle::PlayerVisual>,
            With<ambition_platformer_primitives::markers::PrimaryPlayer>,
            Without<MorphBallVisual>,
        ),
    >,
    mut ball_query: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<MorphBallVisual>>,
) {
    let Ok((mut transform, mut sprite, mut ball_visibility)) = ball_query.single_mut() else {
        return;
    };
    let Ok((pose, presented)) = player_q.single() else {
        return;
    };
    let in_morph = pose.morph_ball;
    if in_morph {
        transform.translation = ambition_engine_core::config::world_to_bevy(
            &world.0,
            // The sphere IS the body while morphed, so it draws where the body
            // is presented — not where its last tick left it.
            ambition_sim_view::presented_pose::draw_pos(pose, presented),
            ambition_engine_core::config::WORLD_Z_PLAYER + 0.05,
        );
        // Slightly larger than the AABB so the soft anti-aliased rim
        // reads as the ball's outline rather than as background.
        let render = bevy::math::Vec2::new(pose.size.x * 1.10, pose.size.y * 1.10);
        sprite.custom_size = Some(render);
        *ball_visibility = Visibility::Visible;
        if let Ok(mut player_vis) = player_query.single_mut() {
            *player_vis = Visibility::Hidden;
        }
    } else {
        *ball_visibility = Visibility::Hidden;
        if let Ok(mut player_vis) = player_query.single_mut() {
            // Inherited visibility lets the parent / overlay control
            // hiding (death overlay, room transition fade); we only
            // override to Visible when leaving morph ball, then drop
            // back to Inherited so we don't fight other systems.
            if matches!(*player_vis, Visibility::Hidden) {
                *player_vis = Visibility::Inherited;
            }
        }
    }
}

/// **What these tests prove, and what they do not.**
///
/// tracks.md's bug queue carries *"Morph ball still draws the robot"*. These
/// three tests run `sync_morph_ball_visual` against the rig it actually sees — a
/// `PlayerEntity + PrimaryPlayer + PlayerVisual` body carrying a `BodyPoseView`,
/// one `MorphBallVisual` sibling — and
/// the system is **correct**: it shows the ball, hides the body, and restores
/// `Inherited` (never a hard `Visible`) on exit.
///
/// So the reported bug is NOT in this system, and the search moves on: a child or
/// overlay entity carrying an explicit `Visibility::Visible` (which Bevy does not
/// hide with its parent), a second entity drawing the body's sprite, or a
/// last-write-wins ordering with a system that re-shows it. This file no longer
/// needs re-litigating.
///
/// The DESIGN defect is separate and still owed (E3, `mode→sprite-state row`):
/// a modal body morph should select an animation row on the body's own sheet, not
/// hide the body and draw a bespoke sibling sprite. That is what "generalize modal
/// body morphs" means, and it deletes this whole file.
#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer_primitives::lifecycle::PlayerVisual;
    use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
    use ambition_sim_view::BodyPoseView;

    fn pose(morph: bool) -> BodyPoseView {
        BodyPoseView {
            morph_ball: morph,
            size: ambition_engine_core::Vec2::new(24.0, 24.0),
            ..Default::default()
        }
    }

    /// The rig `sync_morph_ball_visual` actually runs against: a player body that
    /// carries `PlayerVisual` + `PrimaryPlayer` + a `BodyPoseView`, one
    /// `MorphBallVisual` sibling.
    fn rig(morph: bool) -> (App, Entity, Entity) {
        let mut app = App::new();
        ambition_platformer_primitives::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_engine_core::RoomGeometry(ambition_engine_core::World::new(
                "t",
                ambition_engine_core::Vec2::new(640.0, 480.0),
                ambition_engine_core::Vec2::ZERO,
                Vec::new(),
            )),
        );
        let player = app
            .world_mut()
            .spawn((
                PlayerVisual,
                PlayerEntity,
                PrimaryPlayer,
                pose(morph),
                Visibility::Inherited,
                Transform::default(),
            ))
            .id();
        let ball = app
            .world_mut()
            .spawn((
                MorphBallVisual,
                Sprite::default(),
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();
        app.add_systems(Update, sync_morph_ball_visual);
        (app, player, ball)
    }

    fn vis(app: &App, e: Entity) -> Visibility {
        *app.world().get::<Visibility>(e).unwrap()
    }

    /// **The reported bug: "morph ball still draws the robot".** In morph the ball
    /// shows and the body's sprite must be hidden — otherwise the standing rig
    /// draws through the ball.
    #[test]
    fn entering_morph_hides_the_body_sprite_and_shows_the_ball() {
        let (mut app, player, ball) = rig(true);
        app.update();
        assert_eq!(vis(&app, ball), Visibility::Visible, "the ball draws");
        assert_eq!(
            vis(&app, player),
            Visibility::Hidden,
            "the standing rig must not draw through the ball"
        );
    }

    /// Leaving morph restores the body to `Inherited` — never a hard `Visible`,
    /// so the death overlay and the room-transition fade keep their authority.
    #[test]
    fn leaving_morph_returns_the_body_to_inherited_not_visible() {
        let (mut app, player, ball) = rig(true);
        app.update();
        assert_eq!(vis(&app, player), Visibility::Hidden);

        app.world_mut()
            .get_mut::<BodyPoseView>(player)
            .unwrap()
            .morph_ball = false;
        app.update();
        assert_eq!(vis(&app, ball), Visibility::Hidden);
        assert_eq!(
            vis(&app, player),
            Visibility::Inherited,
            "not `Visible`: the overlay/fade systems must still be able to hide it"
        );
    }

    /// A body that never morphs is never touched.
    #[test]
    fn a_body_that_is_not_in_morph_is_left_alone() {
        let (mut app, player, ball) = rig(false);
        app.update();
        assert_eq!(vis(&app, ball), Visibility::Hidden);
        assert_eq!(vis(&app, player), Visibility::Inherited);
    }
}
