//! Generic platformer room presentation.
//!
//! [`PlatformerPresentationPlugin`] installs the gameplay camera, room/parallax
//! visuals, sprite animation, and player-visual scheduling. Session room visuals
//! can also be installed independently by hosts with their own camera stack.
//! HUD, menus, audio, dev overlays, and game-specific presentation remain host
//! responsibilities. Missing art falls back to the renderer's ordinary block
//! representation.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionScopeId, SessionScopeSet, SessionSpawnScope,
};
use ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings;
use ambition_platformer2d_world::rooms::RoomSet;
use ambition_sprite_sheet::game_assets::GameAssets;

use crate::rendering::{
    spawn_parallax_layers, spawn_room_visuals, PlayerVisualSchedulePlugin,
    PresentationVisualAnimationPlugin,
};

/// System set for this plugin's one-shot host-resident `Startup` work, so a game
/// can order its own presentation setup against it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformerPresentationSetupSet;

#[derive(Resource, Default)]
struct PresentedSessionScope(Option<SessionScopeId>);

/// The parallax half of the presentation memo, separate because a backdrop can
/// become possible LATER than the room it sits behind. See
/// `sync_session_room_visuals`.
#[derive(Resource, Default)]
struct PresentedParallaxScope(Option<SessionScopeId>);

/// The provider-agnostic per-session room presentation: whenever a fresh session scope goes
/// live, spawn the active `RoomSet` room's parallax layers and static visuals exactly once,
/// owned by that scope.
///
/// `PlatformerPresentationPlugin` includes it; a host with its own camera and
/// presentation stack (the Ambition shell host) adds JUST this plugin.
pub struct SessionRoomVisualsPlugin;

impl Plugin for SessionRoomVisualsPlugin {
    fn build(&self, app: &mut App) {
        // Room visuals include world labels, so this composition also installs their layout pass.
        // `WorldLabelLayoutPlugin` is idempotent.
        app.add_plugins(crate::rendering::WorldLabelLayoutPlugin);
        // Room/parallax passes consume the resolved quality budget; install its idempotent owner.
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.init_resource::<PresentedSessionScope>();
        app.init_resource::<PresentedParallaxScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        app.add_systems(
            Update,
            sync_session_room_visuals.in_set(SessionScopeSet::Presentation),
        );
        // The composition that spawns blocks also applies authored per-block art overrides.
        app.add_systems(Update, crate::rendering::apply_block_art);
        // The layer-spawning composition owns active-room theme loading and the refresh that
        // materializes newly available/quality-changed parallax assets.
        app.add_systems(
            Update,
            (
                crate::rendering::ensure_active_room_parallax_theme,
                crate::rendering::refresh_parallax_layers_on_quality_change,
            )
                .chain()
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
        // Each live view owns its own parallax layers because placement depends on that view's
        // camera and viewport. Chain mirror -> sync so spawned/re-keyed copies flush before sync,
        // and run after refresh so quality-driven respawns have settled. No session-world guard is
        // needed; these systems operate only on the view/layer entities that exist.
        app.add_systems(
            Update,
            (
                crate::rendering::mirror_parallax_layers_per_view,
                crate::rendering::sync_parallax_layers,
            )
                .chain()
                .after(crate::rendering::camera_follow)
                .after(crate::rendering::refresh_parallax_layers_on_quality_change),
        );
    }
}

/// See the module docs. The generic platformer presentation: a camera, the room's
/// static visuals, and the sprite/animation chain.
pub struct PlatformerPresentationPlugin;

impl Plugin for PlatformerPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.add_systems(
            Startup,
            (spawn_main_camera, spawn_initial_room_visuals)
                .chain()
                .in_set(PlatformerPresentationSetupSet),
        );
        app.add_plugins(SessionRoomVisualsPlugin);
        // Room-transition requests rebuild visuals through the presentation animation plugin.
        app.add_plugins((
            PresentationVisualAnimationPlugin,
            PlayerVisualSchedulePlugin,
        ));
    }
}

/// Spawn the single-view gameplay camera and full-screen front-UI camera.
///
/// Gameplay may occupy a viewport, while UI targets the full display. Multi-view compositions own
/// their gameplay rigs and use this path only for the unambiguous full-screen UI camera.
fn spawn_main_camera(
    mut commands: Commands,
    // Bind camera→view at spawn so view ownership is composition state, not a per-frame guess.
    views: Query<Entity, With<ambition_sim_view::LocalView>>,
) {
    let layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER);
    // This plugin owns exactly one gameplay rig. Spawn it only when `ViewsOnHand` can identify one
    // view; multi-view callers use `ambition_sim_view::compose_local_views` and must not receive an
    // extra unbound full-screen gameplay camera.
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter());
    match on_hand.presented_by(None) {
        Some(view) => {
            let camera = commands
                .spawn((
                    Camera2d,
                    MainCamera,
                    layers,
                    ambition_sim_view::PresentsView(view),
                    Name::new("Main Camera"),
                ))
                .id();
            // The shared publisher rejects a second main-camera record rather than choosing a winner.
            ambition_platformer2d_shared_tangle::camera_layers::publish_main_camera(
                &mut commands,
                camera,
            );
        }
        None => bevy::log::info_once!(
            "several local views exist, so the shared presentation plugin spawned no              gameplay camera: the composition that asked for those views owns their rigs              (`ambition_sim_view::compose_local_views`)."
        ),
    }

    commands.spawn((
        Camera2d,
        Camera {
            order: 9,
            clear_color: bevy::camera::ClearColorConfig::None,
            ..default()
        },
        ambition_platformer2d_shared_tangle::camera_layers::FrontHudCamera,
        bevy::ui::IsDefaultUiCamera,
        bevy::camera::visibility::RenderLayers::layer(
            ambition_platformer2d_shared_tangle::camera_layers::FRONT_HUD_LAYER,
        ),
        Name::new("Front HUD Camera"),
    ));
}

/// Spawn the active room once for legacy hosts that do not install the
/// gameplay-session lifecycle. Shell hosts wait for a real session activation.
fn spawn_initial_room_visuals(
    mut commands: Commands,
    room_set: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    physics_settings: Res<PhysicsSandboxSettings>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    if active_session.is_some() {
        return;
    }
    // No world installed (a minimal test app) → nothing to draw, and that is not
    // an error: the same shape every optional-resource system in the engine uses.
    let Some(room_set) = room_set else {
        return;
    };
    let spec = room_set.active_spec();
    spawn_parallax_layers(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        spec,
        *physics_settings,
        assets.as_deref(),
    );
}

/// Materialize the active session's room presentation exactly once. The scope
/// is captured before any spawn request, so route retirement owns every static
/// visual and parallax entity created here.
fn sync_session_room_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    mut presented: ResMut<PresentedSessionScope>,
    mut parallax_presented: ResMut<PresentedParallaxScope>,
    room_set: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    physics_settings: Res<PhysicsSandboxSettings>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
) {
    let Some(active_session) = active_session else {
        return;
    };
    let current = active_session.current();
    let Some(scope) = current else {
        presented.0 = None;
        parallax_presented.0 = None;
        return;
    };
    if presented.0 == Some(scope) && parallax_presented.0 == Some(scope) {
        return;
    }
    let Some(room_set) = room_set else {
        // Keep the scope unpresented so a provider that publishes its world on a
        // later frame is retried rather than permanently skipped.
        return;
    };
    let spec = room_set.active_spec();

    // ⛔⛔ TWO MEMOS, BECAUSE THE ROOM AND ITS BACKDROP BECOME POSSIBLE AT
    // DIFFERENT TIMES. `spawn_parallax_layers` early-returns when `GameAssets`
    // has no layers for the room's theme, and `GameAssets` loads ONE theme at
    // startup — every other theme lazy-loads via
    // `ensure_active_room_parallax_theme`. A single memo forced a choice
    // between two wrong answers: set it first and a session that activated the
    // frame BEFORE its theme arrived got no parallax and was never retried
    // (one shot, and it missed); defer it and the whole room waited on a
    // backdrop.
    //
    // ⭐ THIS IS WHY AMBITION HAD A BACKDROP AND SMASH DID NOT, on the same
    // assets and the same theme (`Hub`). The sandbox draws through
    // `spawn_initial_room_visuals`, which has no memo and simply tries again
    // next frame; the session path is memoized and did not.
    //
    // ⛔⛔ AND DEFERRING BOTH WAS THE WORSE HALF, which is what this split
    // fixes: `!theme_loaded` used to `return` before `spawn_room_visuals`, so
    // at a tier that wants parallax a late theme withheld EVERY static visual
    // and every authored room entity — not merely the backdrop. Potato never
    // showed it because `parallax.enabled` is false there, so the gate never
    // engaged. ⇒ Room presentation may not depend on a backdrop becoming
    // resident. Parallax waits alone.
    let spawn_scope = SessionSpawnScope::scoped(scope);

    if presented.0 != Some(scope) {
        presented.0 = Some(scope);
        spawn_room_visuals(
            &mut commands,
            spawn_scope,
            spec,
            *physics_settings,
            assets.as_deref(),
        );
    }

    if parallax_presented.0 == Some(scope) {
        return;
    }
    let wants_parallax = quality
        .as_deref()
        .map(|q| q.budget.parallax.enabled)
        .unwrap_or(true);
    if wants_parallax {
        let theme = ambition_sprite_sheet::game_assets::ParallaxTheme::from_room_metadata(
            &spec.metadata,
        );
        let theme_loaded = assets.as_deref().is_some_and(|a| {
            ambition_sprite_sheet::game_assets::ParallaxLayerAsset::ALL
                .iter()
                .any(|layer| a.parallax_layers.get(theme, *layer).is_some())
        });
        if !theme_loaded {
            // Leave the PARALLAX memo unset so the next frame retries — and only
            // that one. The room is already on screen.
            return;
        }
    }

    // Settled either way: a tier that disables parallax, or a room whose theme
    // legitimately has no art, is finished rather than retried every frame.
    parallax_presented.0 = Some(scope);
    spawn_parallax_layers(
        &mut commands,
        spawn_scope,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
}


#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;

    /// One room asking for a parallax theme that no `GameAssets` provides —
    /// which is the state a session is in on the frame it activates, before
    /// `ensure_active_room_parallax_theme` has loaded the theme.
    fn room_set_wanting_a_theme() -> RoomSet {
        let mut room = ambition_platformer2d_world::rooms::RoomSpec::new(
            "late_theme_room",
            ambition_platformer2d_core::World::new(
                "late_theme_room",
                ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        room.metadata.visual_profile.parallax_theme = Some("a_theme_nobody_loaded".to_string());
        RoomSet::from_parts("late_theme_room", vec![room], Vec::new())
    }

    fn app_with_an_active_session() -> (App, SessionScopeId) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PresentedSessionScope>();
        app.init_resource::<PresentedParallaxScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        app.add_systems(Update, sync_session_room_visuals);

        let mut active = ActiveSessionScope::default();
        let scope = active.begin();
        app.insert_resource(active);
        app.world_mut()
            .spawn((SessionRoot(scope), room_set_wanting_a_theme()));
        (app, scope)
    }

    /// ⛔⛔ A LATE BACKDROP MUST NOT WITHHOLD THE ROOM.
    ///
    /// `!theme_loaded` used to `return` before `spawn_room_visuals`, so at any
    /// tier whose budget wants parallax — every tier above Potato — a room whose
    /// theme had not arrived yet presented NOTHING: no static visuals, no
    /// authored room entities, not merely no backdrop. Potato hid it because
    /// `parallax.enabled` is false there, so the gate never engaged and the one
    /// tier anybody measured headless looked correct.
    ///
    /// ⇒ Two memos. The room presents on the first frame it can; parallax keeps
    /// the retry that it needed, alone.
    #[test]
    fn the_room_presents_even_though_its_parallax_theme_has_not_arrived() {
        let (mut app, scope) = app_with_an_active_session();
        // No `GameAssets` at all, so no theme can be loaded — the strongest form
        // of the condition, and the one a fresh session actually starts in.
        app.update();

        assert_eq!(
            app.world().resource::<PresentedSessionScope>().0,
            Some(scope),
            "the room must be presented on the frame it activates, whatever the backdrop is doing",
        );
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            None,
            "and parallax must stay unsettled so a later theme is still retried",
        );
    }

    /// The retry the original single memo existed to provide, kept: parallax is
    /// unsettled while the theme is missing, on every frame, not just the first.
    #[test]
    fn parallax_keeps_retrying_while_the_theme_is_missing() {
        let (mut app, scope) = app_with_an_active_session();
        app.update();
        app.update();
        app.update();

        assert_eq!(app.world().resource::<PresentedSessionScope>().0, Some(scope));
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            None,
            "a missing theme must never settle the parallax memo",
        );
    }

    /// ⚠ NON-VACUITY, and the reason this test exists beside the two above: if
    /// the room memo could never settle, the first test would pass for the wrong
    /// reason. A tier that does not want parallax settles BOTH on frame one.
    #[test]
    fn a_tier_that_wants_no_parallax_settles_both_memos_at_once() {
        let (mut app, scope) = app_with_an_active_session();
        let mut quality = crate::quality::ResolvedVisualQuality::default();
        quality.budget.parallax.enabled = false;
        app.insert_resource(quality);
        app.update();

        assert_eq!(app.world().resource::<PresentedSessionScope>().0, Some(scope));
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            Some(scope),
            "nothing is coming, so parallax is finished rather than retried every frame",
        );
    }
}
