//! Sandbox-side body-mode driver (crouch + morph ball + collision-safe
//! stand-up).
//!
//! Listens to the deadzoned `axis_y` from `ControlFrame` and the
//! double-tap-down gesture (`fast_fall_pressed`) and asks the engine
//! to flip `Player::body_mode` between `Standing`, `Crouching`, and
//! `MorphBall`. `try_change_body_mode` does the per-frame
//! collision-safe resize: if a low ceiling would clip the larger body
//! the helper rejects the transition and the player stays in the
//! smaller stance. Auto-detected `PlayerModeChanged` trace events
//! fire from the trace recorder diffing `player.body_mode` between
//! snapshots, so this driver does not push events itself.
//!
//! Input model:
//! - Standing + Down held + grounded → Crouching.
//! - Standing/Crouching + double-tap Down + grounded → MorphBall.
//! - MorphBall + Jump pressed → try Standing (gated). If a low
//!   ceiling blocks the standing body, the morph ball stays curled.
//! - Crouching + Down released → Standing (gated).
//! - Mid-action mechanics (dash, blink-aim, wall-cling/climb, swim)
//!   own the player shape; the driver no-ops while any of them are
//!   active.
//!
//! Runs in the progression chain after `sandbox_update` for the same
//! reason `ledge_grab` and `swim` do: it mutates `runtime.player`
//! outside the dense `movement.rs` simulator. The size/pos delta is
//! constrained to the body-mode swap (no horizontal repositioning),
//! so the next simulator tick treats it as a clean smaller AABB and
//! collision repair runs as usual against any new geometry. The
//! engine still gates `fast_fall_pressed` on `!on_ground`, so using
//! the same gesture for grounded morph and airborne fast-fall has
//! no input crosstalk.

use ambition_engine as ae;
use bevy::prelude::*;

/// Threshold on `axis_y` for treating Down as "held" for crouch.
/// Mirrors the threshold used by ledge-grab drop and the engine's
/// drop-through gesture so the player feel stays consistent.
const CROUCH_AXIS_Y_THRESHOLD: f32 = 0.4;

pub fn update_body_mode(
    world: Res<crate::GameWorld>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    controls: Res<crate::input::ControlFrame>,
) {
    let player = &runtime.player;

    // Mid-action mechanics own the body shape — don't fight them.
    if player.dash_timer > 0.0 || player.blink_aiming {
        return;
    }
    // Wall / ledge state owns its own posture; reverting it via crouch
    // would break the ledge-grab anchor invariant.
    if player.wall_clinging || player.wall_climbing {
        return;
    }
    // In-water posture: leave water swim mechanics alone.
    if player.water_contact.is_some() {
        return;
    }

    let down_held = controls.axis_y > CROUCH_AXIS_Y_THRESHOLD;
    let on_ground = player.on_ground;
    let mode = player.body_mode;
    let solid = |b: &ae::Block| matches!(b.kind, ae::BlockKind::Solid);

    // Consume the double-tap-down edge regardless of branch so we
    // don't latch a stale signal across frames or gameplay states.
    let double_tap_down = std::mem::take(&mut runtime.double_tap_down_pending);

    // MorphBall has the smallest AABB. Exiting it means re-checking
    // overhead clearance; sourcing the exit input from `jump_pressed`
    // mirrors how a player would naturally try to "stand up" out of
    // the ball. Up-pressed (a tap, not held) is also accepted as the
    // unmorph gesture so keyboards that bind Up to a different
    // physical key can still escape the ball without committing to a
    // jump arc — useful for testing on layouts where Jump and Up
    // map to the same key.
    if mode == ae::BodyMode::MorphBall {
        if controls.jump_pressed || controls.up_pressed {
            let _ = ae::try_change_body_mode(
                &mut runtime.player,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
        }
        return;
    }

    // Double-tap-down on the ground from Standing or Crouching curls
    // into MorphBall. The signal is `runtime.double_tap_down_pending`,
    // routed through SandboxRuntime by `input_timer_phase` because
    // `sandbox_update` consumes its ControlFrame as a local copy that
    // doesn't reach the progression chain. The engine gates
    // fast_fall on `!on_ground` already, so the same gesture firing
    // morph-ball when grounded has no input crosstalk.
    if on_ground && double_tap_down {
        let _ = ae::try_change_body_mode(
            &mut runtime.player,
            ae::BodyMode::MorphBall,
            &world.0,
            solid,
        );
        return;
    }

    let target = if down_held && on_ground {
        ae::BodyMode::Crouching
    } else {
        ae::BodyMode::Standing
    };

    if mode == target {
        return;
    }

    // The engine helper does the resize-with-fit check; ignore the
    // boolean result — a blocked stand-up is the desired UX (player
    // stays crouched under the ceiling) and the auto-trace diff will
    // surface a successful transition.
    let _ = ae::try_change_body_mode(&mut runtime.player, target, &world.0, solid);
}

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

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
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
/// `spawn_morph_ball_visual` once `SceneEntities` is populated.
pub fn build_morph_ball_sprite(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let handle = images.add(build_morph_ball_image());
    commands.insert_resource(MorphBallSprite { handle });
}

/// Spawn the morph-ball sibling sprite tied to the live `SceneEntities`.
/// Runs each frame but inserts only when the `MorphBallVisual` query is
/// empty — equivalent to a one-shot "after SceneEntities is ready"
/// guard that handles the visible-binary boot order without needing a
/// dedicated state.
pub fn spawn_morph_ball_visual(
    mut commands: Commands,
    sprite: Option<Res<MorphBallSprite>>,
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
    commands.spawn((
        Sprite {
            image: sprite.handle.clone(),
            custom_size: Some(bevy::math::Vec2::new(16.0, 16.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, crate::config::WORLD_Z_PLAYER + 0.05),
        Visibility::Hidden,
        MorphBallVisual,
        Name::new("Morph Ball Visual"),
    ));
}

/// Toggle the morph-ball visual on / off based on `Player::body_mode`,
/// mirror its position to the player, and scale it to the morph-ball
/// AABB. Hides the regular player sprite while the ball is active so
/// the standing-rig animation doesn't show through.
pub fn sync_morph_ball_visual(
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    entities: Res<crate::rendering::SceneEntities>,
    mut player_query: Query<
        &mut Visibility,
        (
            With<crate::rendering::PlayerVisual>,
            Without<MorphBallVisual>,
        ),
    >,
    mut ball_query: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        With<MorphBallVisual>,
    >,
) {
    let Ok((mut transform, mut sprite, mut ball_visibility)) = ball_query.single_mut() else {
        return;
    };
    let in_morph = runtime.player.body_mode == ae::BodyMode::MorphBall;
    if in_morph {
        transform.translation = crate::config::world_to_bevy(
            &world.0,
            runtime.player.pos,
            crate::config::WORLD_Z_PLAYER + 0.05,
        );
        // Slightly larger than the AABB so the soft anti-aliased rim
        // reads as the ball's outline rather than as background.
        let render = bevy::math::Vec2::new(
            runtime.player.size.x * 1.10,
            runtime.player.size.y * 1.10,
        );
        sprite.custom_size = Some(render);
        *ball_visibility = Visibility::Visible;
        if let Ok(mut player_vis) = player_query.get_mut(entities.player) {
            *player_vis = Visibility::Hidden;
        }
    } else {
        *ball_visibility = Visibility::Hidden;
        if let Ok(mut player_vis) = player_query.get_mut(entities.player) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ControlFrame;
    use crate::{GameWorld, SandboxRuntime};
    use ambition_engine as ae;

    fn empty_world() -> ae::World {
        ae::World::new(
            "body_mode_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            Vec::new(),
        )
    }

    fn ceiling_world(ceiling_top: f32, ceiling_h: f32) -> ae::World {
        ae::World::new(
            "body_mode_ceiling",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            vec![ae::Block::solid(
                "ceiling",
                ae::Vec2::new(0.0, ceiling_top),
                ae::Vec2::new(2000.0, ceiling_h),
            )],
        )
    }

    fn body_app(world: ae::World) -> App {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(GameWorld(world.clone()));
        let runtime = SandboxRuntime::new(
            &world,
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        app.insert_resource(runtime);
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, update_body_mode);
        app
    }

    fn set_grounded_at(app: &mut App, pos: ae::Vec2) {
        let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
        runtime.player.pos = pos;
        runtime.player.vel = ae::Vec2::ZERO;
        runtime.player.on_ground = true;
        runtime.player.on_wall = false;
        runtime.player.wall_clinging = false;
        runtime.player.wall_climbing = false;
        runtime.player.dash_timer = 0.0;
        runtime.player.blink_aiming = false;
        runtime.player.water_contact = None;
    }

    fn set_axis_y(app: &mut App, axis_y: f32) {
        let mut controls = app.world_mut().resource_mut::<ControlFrame>();
        controls.axis_y = axis_y;
    }

    /// Mark the double-tap-down edge on `SandboxRuntime` exactly as
    /// `input_timer_phase` does in the live build. The driver
    /// consumes via `mem::take`, so the test only needs to arm it
    /// before the tick under test.
    fn arm_double_tap_down(app: &mut App) {
        let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
        runtime.double_tap_down_pending = true;
    }

    /// Holding Down on the ground transitions Standing → Crouching and
    /// shrinks `player.size.y`.
    #[test]
    fn down_held_grounded_enters_crouch() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        set_axis_y(&mut app, 1.0);

        app.update();

        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Crouching);
        assert!(runtime.player.size.y < runtime.player.base_size.y);
    }

    /// Releasing Down with overhead clearance returns to Standing.
    #[test]
    fn down_released_returns_to_standing_when_clear() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

        // Crouch first.
        set_axis_y(&mut app, 1.0);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::Crouching
        );

        // Release down.
        set_axis_y(&mut app, 0.0);
        app.update();

        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
        assert_eq!(runtime.player.size, runtime.player.base_size);
    }

    /// A low ceiling above the crouched body must reject the stand-up
    /// transition — the player stays crouched.
    #[test]
    fn stand_up_blocked_under_low_ceiling() {
        // Player at pos.y = 600, base_size.y = 46. AABB convention:
        // pos = center, +Y is downward, so:
        //   * Standing y range = [600 - 23, 600 + 23] = [577, 623].
        //   * Crouching size = 46 * 0.55 = 25.3, dy = (46-25.3)/2 = 10.35,
        //     so pos.y = 610.35 and crouched y range = [597.7, 623].
        //   * Stand-up restores pos.y = 600 and standing y range = [577, 623].
        //
        // Ceiling y range [560, 590]:
        //   * Crouched [597.7, 623] vs [560, 590]: 597.7 > 590 → no overlap.
        //   * Standing [577, 623] vs [560, 590]: 577 < 590 → overlap.
        // Initial standing also overlaps; the helper doesn't reject pre-
        // existing penetration — it only checks the *target* shape, so
        // the crouch transition still succeeds and the stand-up correctly
        // fails.
        let world = ceiling_world(560.0, 30.0);
        let mut app = body_app(world);
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 600.0));

        // Crouch.
        set_axis_y(&mut app, 1.0);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::Crouching
        );

        // Release down — stand-up should be blocked.
        set_axis_y(&mut app, 0.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Crouching);
        assert!(runtime.player.size.y < runtime.player.base_size.y);
    }

    /// In the air, holding Down does not crouch (crouch is grounded only).
    #[test]
    fn airborne_down_does_not_crouch() {
        let mut app = body_app(empty_world());
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.pos = ae::Vec2::new(200.0, 200.0);
            runtime.player.on_ground = false;
            runtime.player.vel = ae::Vec2::ZERO;
        }
        set_axis_y(&mut app, 1.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }

    /// Mid-dash holding Down does not crouch — dash owns the body shape.
    #[test]
    fn dash_active_blocks_crouch() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.dash_timer = 0.05;
        }
        set_axis_y(&mut app, 1.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }

    /// Double-tap-down on the ground from Standing curls into MorphBall.
    /// The signal is `runtime.double_tap_down_pending` (routed
    /// through SandboxRuntime by `input_timer_phase` because
    /// sandbox_update consumes a local copy of ControlFrame).
    #[test]
    fn double_tap_down_grounded_enters_morph_ball() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        arm_double_tap_down(&mut app);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::MorphBall);
        // MorphBall is smaller than Standing on both axes.
        assert!(runtime.player.size.x < runtime.player.base_size.x);
        assert!(runtime.player.size.y < runtime.player.base_size.y);
    }

    /// Crouching + double-tap-down also curls into MorphBall (reachable
    /// from either entry point). Mirrors the input model in the
    /// docstring.
    #[test]
    fn double_tap_down_from_crouch_enters_morph_ball() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

        // Crouch first.
        set_axis_y(&mut app, 1.0);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::Crouching
        );
        // Then double-tap-down.
        arm_double_tap_down(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::MorphBall
        );
    }

    /// Jump-pressed inside MorphBall unmorphs to Standing when there's
    /// overhead clearance.
    #[test]
    fn jump_press_in_morph_ball_unmorphs_to_standing() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

        // Drive into MorphBall via the gesture (covers the input path
        // and avoids juggling a second world reference inside a
        // resource borrow).
        arm_double_tap_down(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::MorphBall
        );

        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.jump_pressed = true;
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
        assert_eq!(runtime.player.size, runtime.player.base_size);
    }

    /// Jump-pressed inside MorphBall under a low ceiling stays curled —
    /// the standing AABB doesn't fit.
    #[test]
    fn jump_press_in_morph_ball_under_low_ceiling_stays_curled() {
        // Ceiling at y in [560, 590]: standing top 577 < 590 → blocks.
        // MorphBall body: base_size 28x46 → MorphBall is (28*0.55,
        // 28*0.55) = (15.4, 15.4). On the floor at pos.y = 600, the
        // morph ball center is at 600 + (46 - 15.4)/2 = 615.3, half
        // 7.7 → top 607.6, bottom 623.0. Crouched would be 597.7→623,
        // so the morph ball clears the ceiling at 590 by an even wider
        // margin. Standing has top 577 → blocked.
        let world = ceiling_world(560.0, 30.0);
        let mut app = body_app(world);
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 600.0));

        // Morph via gesture.
        arm_double_tap_down(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::MorphBall
        );

        // Try to unmorph.
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.jump_pressed = true;
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::MorphBall);
    }

    /// Airborne double-tap-down does NOT curl (morph is grounded only).
    #[test]
    fn airborne_double_tap_down_does_not_morph() {
        let mut app = body_app(empty_world());
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.pos = ae::Vec2::new(200.0, 200.0);
            runtime.player.on_ground = false;
        }
        arm_double_tap_down(&mut app);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }

    /// Repro for the ControlFrame-not-flowing-through-sandbox_update
    /// bug: setting `controls.fast_fall_pressed = true` directly on
    /// the resource (mimicking what `input_timer_phase` writes to its
    /// LOCAL controls copy) is NOT sufficient to enter MorphBall.
    /// The driver only reads `runtime.double_tap_down_pending`. This
    /// test pins the routing so a future refactor can't accidentally
    /// switch the body-mode driver back to reading ControlFrame and
    /// silently break the in-game gesture.
    #[test]
    fn morph_ball_does_not_fire_from_control_frame_alone() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = true;
        }
        // double_tap_down_pending is NOT armed.
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(
            runtime.player.body_mode,
            ae::BodyMode::Standing,
            "the body-mode driver must read runtime.double_tap_down_pending, \
             not controls.fast_fall_pressed (which sandbox_update consumes \
             on a local copy that doesn't reach later systems)"
        );
    }

    /// `SandboxRuntime::reset` (called by death/respawn) must restore
    /// the player to Standing with the canonical base size, even if
    /// the player was mid-Crouch or mid-MorphBall when they died.
    /// Otherwise a respawn could land in a smaller body and the engine
    /// would compute collision against the shrunk AABB until the next
    /// crouch input.
    #[test]
    fn reset_restores_standing_from_crouch() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        set_axis_y(&mut app, 1.0);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::Crouching
        );

        let world = empty_world();
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.reset(&world, ae::DEFAULT_TUNING);
        }
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
        assert_eq!(runtime.player.size, runtime.player.base_size);
    }

    #[test]
    fn reset_restores_standing_from_morph_ball() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        arm_double_tap_down(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::MorphBall
        );

        let world = empty_world();
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.reset(&world, ae::DEFAULT_TUNING);
        }
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
        assert_eq!(runtime.player.size, runtime.player.base_size);
    }

    /// Wall-cling state owns the player posture; do not crouch from it.
    #[test]
    fn wall_clinging_blocks_crouch() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.wall_clinging = true;
            runtime.player.on_wall = true;
            runtime.player.on_ground = false;
        }
        set_axis_y(&mut app, 1.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }
}
